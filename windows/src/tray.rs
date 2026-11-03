//! The notification area icon: created with `Shell_NotifyIconW`, its menu built when it is opened.

use std::{ffi::c_void, ptr};

use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_INFO, NIIF_WARNING, NIM_ADD, NIM_DELETE,
    NIM_MODIFY, NIM_SETVERSION, NOTIFYICON_VERSION_4, NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::{
    app::{self, App, Shortcut},
    i18n::tr,
    monitor,
    settings::TrayIconStyle,
    win,
    WM_TRAY,
};

/// Icon files generated from `assets/titan_control.svg` by the build script.
const ICON_LIGHT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon-light.ico"));
const ICON_DARK: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon-dark.ico"));

const ICON_ID: u32 = 1;

enum Command {
    ShowWindow,
    Preferences,
    Quit,
    FavoriteMode(u16),
    SliderValue(&'static str, u16),
    ChoiceValue(&'static str, usize),
    ToggleShortcut(&'static str),
}

/// Icon data for the chosen style.
fn icon_bytes(style: TrayIconStyle, _app: &App) -> &'static [u8] {
    match style {
        TrayIconStyle::Light => ICON_LIGHT,
        TrayIconStyle::Dark => ICON_DARK,
        TrayIconStyle::Theme => {
            if win::system_taskbar_prefers_dark() {
                ICON_LIGHT
            } else {
                ICON_DARK
            }
        }
    }
}

fn icon_handle(app: &App) -> *mut c_void {
    let size = unsafe { GetSystemMetrics(SM_CXSMICON) } as u32;
    win::icon_from_ico(icon_bytes(app.settings.tray_icon_style, app), size).unwrap_or(ptr::null_mut())
}

fn data(app: &App) -> NOTIFYICONDATAW {
    let mut data: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = app.host;
    data.uID = ICON_ID;
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    data.uCallbackMessage = WM_TRAY;
    data.hIcon = icon_handle(app) as _;
    let tip = win::wide(&app.tr("Titan Control"));
    let length = (tip.len() - 1).min(data.szTip.len() - 1);
    data.szTip[..length].copy_from_slice(&tip[..length]);
    data
}

/// Adds the icon to the notification area.
pub fn create(app: &mut App) {
    let mut data = data(app);
    let added = unsafe { Shell_NotifyIconW(NIM_ADD, &data) };
    if added == 0 {
        crate::log::write(format_args!("[TRAY] Shell_NotifyIcon(NIM_ADD) failed: {}", win::last_error()));
        return;
    }
    app.tray_added = true;
    // Balloon notifications need the newer structure version.
    data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
    unsafe {
        Shell_NotifyIconW(NIM_SETVERSION, &data);
        win::destroy_icon(data.hIcon as *mut c_void);
    }
}

/// Re-applies the icon after a change of the style or of the theme.
pub fn refresh(app: &mut App) {
    let installed = app.tray_added;
    if !installed {
        create(app);
        return;
    }
    let data = data(app);
    unsafe {
        Shell_NotifyIconW(NIM_MODIFY, &data);
        win::destroy_icon(data.hIcon as *mut c_void);
    }
}

pub fn remove(app: &mut App) {
    if app.tray_added {
        let mut data: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
        data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        data.hWnd = app.host;
        data.uID = ICON_ID;
        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &data);
        }
        app.tray_added = false;
    }
}

/// Shows a notification next to the icon (used for errors the user has to see while the window is closed).
pub fn notify(app: &App, title: &str, text: &str, warning: bool) {
    if !app.tray_added {
        return;
    }
    let mut data: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = app.host;
    data.uID = ICON_ID;
    data.uFlags = NIF_INFO;
    data.dwInfoFlags = if warning { NIIF_WARNING } else { NIIF_INFO };
    let text = win::wide(text);
    let length = (text.len() - 1).min(data.szInfo.len() - 1);
    data.szInfo[..length].copy_from_slice(&text[..length]);
    let title = win::wide(title);
    let length = (title.len() - 1).min(data.szInfoTitle.len() - 1);
    data.szInfoTitle[..length].copy_from_slice(&title[..length]);
    unsafe {
        Shell_NotifyIconW(NIM_MODIFY, &data);
    }
}

/// Handles the callback of the icon.
///
/// With `NOTIFYICON_VERSION_4` the shell puts the event in the low word of `lParam` (the high word is the icon id) and
/// reports a left click as `NIN_SELECT` and a right click as `WM_CONTEXTMENU`. Note that `WM_LBUTTONUP` and
/// `WM_RBUTTONUP` are also sent right before `NIN_SELECT` and `WM_CONTEXTMENU`, so they must not be handled separately.
pub fn on_callback(app: &mut App, _x: i32, _y: i32, lparam: isize) {
    let event = (lparam as usize) & 0xFFFF;
    match event as u32 {
        NIN_SELECT | NIN_KEYSELECT => toggle_window(app),
        WM_CONTEXTMENU => show_menu(app),
        _ => {}
    }
}

/// `NIN_SELECT` and `NIN_KEYSELECT`: the select notifications of version 4 (`WM_USER + 0` / `WM_USER + 1`).
const NIN_SELECT: u32 = 0x0400;
const NIN_KEYSELECT: u32 = 0x0401;

/// Shows or hides the settings window.
pub fn toggle_window(app: &mut App) {
    if crate::ui::is_open(app) {
        crate::ui::close(app);
    } else {
        crate::ui::open(app);
    }
}

fn show_menu(app: &mut App) {
    let mut commands: Vec<Command> = Vec::new();
    let menu = unsafe { CreatePopupMenu() };
    if menu.is_null() {
        return;
    }
    let mut next_id = 1u16;
    let mut add = |app: &App, commands: &mut Vec<Command>, menu: HMENU, text: &str, command: Command, flags: u32| {
        let label = win::wide(text);
        unsafe {
            AppendMenuW(menu, flags, next_id as usize, label.as_ptr());
        }
        next_id += 1;
        commands.push(command);
        let _ = app;
    };

    // Favourite picture modes.
    let favorites = app.settings.favorite_picture_mode_set();
    let active = app.get(monitor::VCP_MODE).and_then(monitor::picture_mode_from_vcp).map(|(mode, _)| mode as u16);
    for mode in &favorites {
        if let Some(name) = monitor::PICTURE_MODE_NAMES.get(*mode as usize) {
            let checked = active == Some(*mode);
            add(
                app,
                &mut commands,
                menu,
                name,
                Command::FavoriteMode(*mode),
                MF_STRING | if checked { MF_CHECKED } else { 0 },
            );
        }
    }

    // Shortcuts, grouped like the Linux menu.
    let shortcuts = app.settings.favorite_tray_control_set();
    let kinds = ["Suwaki", "Opcje", "Przełączniki"];
    let mut separator_added = false;
    for title in kinds {
        let selected: Vec<&String> = shortcuts
            .iter()
            .filter(|id| match title {
                "Suwaki" => matches!(app::shortcut(id), Some(Shortcut::Slider(_))),
                "Opcje" => matches!(app::shortcut(id), Some(Shortcut::Choice(_))),
                _ => matches!(app::shortcut(id), Some(Shortcut::Toggle(_))),
            })
            .collect();
        if selected.is_empty() {
            continue;
        }
        if !favorites.is_empty() && !separator_added {
            unsafe {
                AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());
            }
            separator_added = true;
        }
        let submenu = unsafe { CreatePopupMenu() };
        for id in selected {
            // Only the ids in the shortcut table are stored, so the label and the actions use the static id.
            let Some(id): Option<&'static str> =
                app::SHORTCUTS.iter().find(|(known, _)| *known == id.as_str()).map(|(known, _)| *known)
            else {
                continue;
            };
            let label = shortcut_label(app, id);
            let action = app::shortcut(id).copied();
            match action {
                Some(Shortcut::Slider(key)) => {
                    let values = unsafe { CreatePopupMenu() };
                    let (max, step) = app::slider_range(id);
                    let current = slider_vcp_code(key).and_then(|c| app.get(c));
                    for value in (0..=max).step_by(step.max(1) as usize) {
                        let checked = current == Some(value);
                        let text = win::wide(&value.to_string());
                        unsafe {
                            AppendMenuW(
                                values,
                                MF_STRING | if checked { MF_CHECKED } else { 0 },
                                next_id as usize,
                                text.as_ptr(),
                            );
                        }
                        next_id += 1;
                        commands.push(Command::SliderValue(id, value));
                    }
                    let text = win::wide(&label);
                    unsafe {
                        AppendMenuW(submenu, MF_STRING | MF_POPUP, values as usize, text.as_ptr());
                    }
                }
                Some(Shortcut::Choice(code)) => {
                    if let Some((options, values)) = app::choice_options(id) {
                        let values_menu = unsafe { CreatePopupMenu() };
                        let current = app.get(code);
                        for (index, option) in options.iter().enumerate() {
                            let checked = current == values.get(index).copied();
                            let text = win::wide(&tr(&app.lang, option));
                            unsafe {
                                AppendMenuW(
                                    values_menu,
                                    MF_STRING | if checked { MF_CHECKED } else { 0 },
                                    next_id as usize,
                                    text.as_ptr(),
                                );
                            }
                            next_id += 1;
                            commands.push(Command::ChoiceValue(id, index));
                        }
                        let text = win::wide(&label);
                        unsafe {
                            AppendMenuW(submenu, MF_STRING | MF_POPUP, values_menu as usize, text.as_ptr());
                        }
                    } else {
                        let text = win::wide(&label);
                        unsafe {
                            AppendMenuW(submenu, MF_STRING, next_id as usize, text.as_ptr());
                        }
                        next_id += 1;
                        commands.push(Command::ToggleShortcut(id));
                    }
                }
                Some(Shortcut::Toggle(code)) => {
                    let checked = app.get(code) == Some(1);
                    let text = win::wide(&label);
                    unsafe {
                        AppendMenuW(
                            submenu,
                            MF_STRING | if checked { MF_CHECKED } else { 0 },
                            next_id as usize,
                            text.as_ptr(),
                        );
                    }
                    next_id += 1;
                    commands.push(Command::ToggleShortcut(id));
                }
                None => {}
            }
        }
        let text = win::wide(&app.tr(title));
        unsafe {
            AppendMenuW(menu, MF_STRING | MF_POPUP, submenu as usize, text.as_ptr());
        }
    }

    unsafe {
        if !commands.is_empty() {
            AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());
        }
        let show = if crate::ui::is_open(app) { "Ukryj" } else { "Pokaż" };
        let text = win::wide(&app.tr(show));
        AppendMenuW(menu, MF_STRING, next_id as usize, text.as_ptr());
    }
    next_id += 1;
    commands.push(Command::ShowWindow);
    unsafe {
        let text = win::wide(&app.tr("Preferencje"));
        AppendMenuW(menu, MF_STRING, next_id as usize, text.as_ptr());
    }
    next_id += 1;
    commands.push(Command::Preferences);
    unsafe {
        AppendMenuW(menu, MF_SEPARATOR, 0, ptr::null());
        let text = win::wide(&app.tr("Wyjdź"));
        AppendMenuW(menu, MF_STRING, next_id as usize, text.as_ptr());
    }
    commands.push(Command::Quit);

    let mut point: POINT = unsafe { std::mem::zeroed() };
    unsafe {
        GetCursorPos(&mut point);
        // The menu needs the foreground: otherwise it stays on screen when the user clicks somewhere else.
        SetForegroundWindow(app.host);
        let choice = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            0,
            app.host,
            ptr::null(),
        );
        // The notification-area documentation asks for this empty message so the menu really goes away.
        PostMessageW(app.host, WM_NULL, 0, 0);
        DestroyMenu(menu);
        if choice > 0 {
            if let Some(command) = commands.get(choice as usize - 1) {
                run(app, command);
            }
        }
    }
}

fn shortcut_label(app: &App, id: &str) -> String {
    let axis_name = |color: &str| match color {
        "red" => app.tr("Czerwony (R)"),
        "green" => app.tr("Zielony (G)"),
        "blue" => app.tr("Niebieski (B)"),
        "cyan" => app.tr("Cyjan (C)"),
        "magenta" => app.tr("Magenta (M)"),
        "yellow" => app.tr("Żółty (Y)"),
        _ => app.tr(color),
    };
    match id {
        "brightness" => app.tr("Jasność"),
        "contrast" => app.tr("Kontrast"),
        "sharpness" => app.tr("Ostrość"),
        "low_blue_light" => "Low Blue Light".to_string(),
        "dcr" => "DCR".to_string(),
        "hdr" => "HDR".to_string(),
        "local_dimming" => app.tr("Lokalne przyciemnianie"),
        "color_enhance" => "Color Enhance".to_string(),
        "cr_enhance" => "CR Enhance".to_string(),
        "shadow_balance" => "Shadow Balance".to_string(),
        "super_res" => "Super Res".to_string(),
        "halo_control" => "Halo Control".to_string(),
        "color_temp_profile" => app.tr("Temperatura barwowa"),
        "color_temp_red" => format!("RGB: {}", axis_name("red")),
        "color_temp_green" => format!("RGB: {}", axis_name("green")),
        "color_temp_blue" => format!("RGB: {}", axis_name("blue")),
        "gamma" => "Gamma".to_string(),
        "adaptive_sync" => "Adaptive-Sync".to_string(),
        "dynamic_od" => "Dynamic OD".to_string(),
        "dyds" => "DyDs".to_string(),
        "night_vision" => "Night Vision".to_string(),
        "screen_size" => app.tr("Rozmiar ekranu (DualMode)"),
        "fps_counter" => app.tr("Licznik FPS/Hz"),
        "crosshair" => app.tr("Celownik"),
        "stopwatch" => app.tr("Stoper"),
        "game_time" => app.tr("Czas gry"),
        "magnifier" => app.tr("Lupa"),
        "hawkeye" => "Hawkeye Vision".to_string(),
        _ => {
            if let Some(color) = id.strip_prefix("hue_") {
                return format!("{}: {}", app.tr("Odcień (Hue)"), axis_name(color));
            }
            if let Some(color) = id.strip_prefix("saturation_") {
                return format!("{}: {}", app.tr("Nasycenie (Saturation)"), axis_name(color));
            }
            app.tr(id)
        }
    }
}

fn slider_vcp_code(key: &str) -> Option<u8> {
    match key {
        "brightness" => Some(monitor::VCP_BRIGHTNESS),
        "contrast" => Some(monitor::VCP_CONTRAST),
        "sharpness" => Some(monitor::VCP_SHARPNESS),
        "low_blue_light" => Some(monitor::VCP_LOW_BLUE),
        "color_enhance" => Some(monitor::VCP_COLOR_ENHANCE),
        "cr_enhance" => Some(monitor::VCP_CR_ENHANCE),
        "shadow_balance" => Some(monitor::VCP_SHADOW_BALANCE),
        "super_res" => Some(monitor::VCP_SUPER_RES),
        "halo_control" => Some(monitor::VCP_HALO_CONTROL),
        "color_temp_red" => Some(monitor::VCP_RED),
        "color_temp_green" => Some(monitor::VCP_GREEN),
        "color_temp_blue" => Some(monitor::VCP_BLUE),
        "hue_red" => Some(monitor::VCP_HUE_RED),
        "hue_green" => Some(monitor::VCP_HUE_GREEN),
        "hue_blue" => Some(monitor::VCP_HUE_BLUE),
        "hue_cyan" => Some(monitor::VCP_HUE_CYAN),
        "hue_magenta" => Some(monitor::VCP_HUE_MAGENTA),
        "hue_yellow" => Some(monitor::VCP_HUE_YELLOW),
        "saturation_red" => Some(monitor::VCP_SATURATION_RED),
        "saturation_green" => Some(monitor::VCP_SATURATION_GREEN),
        "saturation_blue" => Some(monitor::VCP_SATURATION_BLUE),
        "saturation_cyan" => Some(monitor::VCP_SATURATION_CYAN),
        "saturation_magenta" => Some(monitor::VCP_SATURATION_MAGENTA),
        "saturation_yellow" => Some(monitor::VCP_SATURATION_YELLOW),
        _ => None,
    }
}

fn run(app: &mut App, command: &Command) {
    match command {
        Command::ShowWindow => toggle_window(app),
        Command::Preferences => crate::ui::prefs::open(app),
        Command::Quit => {
            app.quit = true;
            crate::ui::close(app);
        }
        Command::FavoriteMode(mode) => {
            // The tray shows the mode with the variant it currently has.
            let value = match app.get(monitor::VCP_MODE).and_then(monitor::picture_mode_from_vcp) {
                Some((_, custom)) => monitor::picture_mode_vcp_value(*mode as usize, custom),
                None => monitor::picture_mode_vcp_value(*mode as usize, false),
            };
            if let Some(value) = value {
                app.start_profile_load();
                app.set(monitor::VCP_MODE, value);
                crate::ui::apply_state(app);
                refresh(app);
            }
        }
        Command::SliderValue(id, value) => {
            let code = match app::shortcut(id) {
                Some(Shortcut::Slider(key)) => slider_vcp_code(key),
                _ => None,
            };
            if let Some(code) = code {
                app.set(code, *value);
                crate::ui::apply_state(app);
                crate::ui::refresh_availability(app);
            }
        }
        Command::ChoiceValue(id, index) => {
            if let Some((_, values)) = app::choice_options(id)
                && let Some(Shortcut::Choice(code)) = app::shortcut(id)
                && let Some(value) = values.get(*index).copied()
            {
                app.set(*code, value);
                crate::ui::apply_state(app);
                crate::ui::refresh_availability(app);
            }
        }
        Command::ToggleShortcut(id) => {
            if let Some(Shortcut::Toggle(code)) = app::shortcut(id) {
                let on = app.get(*code).unwrap_or(0) == 0;
                app.set(*code, if on { 1 } else { 0 });
                crate::ui::apply_state(app);
                crate::ui::refresh_availability(app);
            }
        }
    }
}
