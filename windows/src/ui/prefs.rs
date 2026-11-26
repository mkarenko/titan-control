//! The preferences and "About" dialogs.
//!
//! Both are modal, created when they are opened and destroyed when they are closed. Every change in the preferences
//! is applied at once, like in the Linux dialog.

use std::{ffi::c_void, ptr};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{DeleteObject, InvalidateRect, SetBkColor, SetTextColor};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::{
    BST_CHECKED, TTF_IDISHWND, TTF_SUBCLASS, TTM_ADDTOOLW, TTM_SETMAXTIPWIDTH, TTS_ALWAYSTIP, TTS_NOPREFIX,
    TTTOOLINFOW,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, SetFocus};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Static control styles and combo box styles (`WinUser.h`).
#[allow(non_upper_case_globals)]
const SS_LEFT: u32 = 0x0000;
#[allow(non_upper_case_globals)]
const SS_NOTIFY: u32 = 0x0100;
#[allow(non_upper_case_globals)]
const BS_PUSHBUTTON: u32 = 0x0000;
#[allow(non_upper_case_globals)]
const BS_AUTOCHECKBOX: u32 = 0x0003;
#[allow(non_upper_case_globals)]
const BS_DEFPUSHBUTTON: u32 = 0x0001;
#[allow(non_upper_case_globals)]
const CBS_DROPDOWNLIST: u32 = 0x0003;
#[allow(non_upper_case_globals)]
const CBS_HASSTRINGS: u32 = 0x0200;

use crate::{
    app::App,
    i18n::{lang_from_index, lang_index, AppLang},
    settings::{self, FirmwarePackage, StepModifier, TrayIconStyle},
    win,
};

const ID_LANGUAGE: u16 = 200;
const ID_THEME: u16 = 201;
const ID_TRAY_ICON: u16 = 202;
const ID_AUTOSTART: u16 = 203;
const ID_MINIMIZED: u16 = 204;
const ID_MODIFIER: u16 = 205;
const ID_FIRMWARE: u16 = 206;
const ID_CLOSE: u16 = 210;
const ID_WEBSITE: u16 = 220;
const ID_ISSUES: u16 = 221;
const ID_COFFEE: u16 = 222;
const ID_ABOUT_LOG: u16 = 223;
const ID_PICKER_FILTER: u16 = 230;
const ID_PICKER_LIST: u16 = 231;
const ID_PICKER_ADD: u16 = 232;
const ID_PROFILE_NAME_EDIT: u16 = 240;
const ID_PROFILE_NAME_OK: u16 = 241;

const MARGIN: i32 = 16;
const LABEL_WIDTH: i32 = 250;
const CONTROL_WIDTH: i32 = 190;
const ROW_HEIGHT: i32 = 32;

/// Opens the preferences as a modal dialog.
pub fn open(app: &mut App) {
    run_modal(
        app,
        "TitanPreferences",
        "Preferencje",
        520,
        |app, hwnd, dpi| build_preferences(app, hwnd, dpi),
        preferences_proc,
    );
    rebuild_main_window(app);
}

/// Opens the "About" dialog.
pub fn about(app: &mut App) {
    run_modal(app, "TitanAbout", "O programie", 420, |app, hwnd, dpi| build_about(app, hwnd, dpi), about_proc);
    rebuild_main_window(app);
}

/// Opens the modal application locator dialog for Auto HDR.
pub fn auto_hdr_picker(app: &mut App) {
    PICKER_ALL.with(|cell| *cell.borrow_mut() = crate::auto_hdr::discover_installed_apps());
    PICKER_FILTERED.with(|cell| cell.borrow_mut().clear());
    run_modal(
        app,
        "TitanAutoHdrPicker",
        "Wybierz aplikacje",
        470,
        |app, hwnd, dpi| build_picker(app, hwnd, dpi),
        picker_proc,
    );
    PICKER_ALL.with(|cell| cell.borrow_mut().clear());
    PICKER_FILTERED.with(|cell| cell.borrow_mut().clear());
    rebuild_main_window(app);
}

/// Opens a compact modal prompt to name and save a new monitor profile from current settings.
pub fn new_profile_dialog(app: &mut App) {
    run_modal(
        app,
        "TitanNewProfile",
        "Nowy profil",
        165,
        |app, hwnd, dpi| build_new_profile(app, hwnd, dpi),
        new_profile_proc,
    );
    rebuild_main_window(app);
}

/// Builds the settings window again after a language or theme change.
fn rebuild_main_window(app: &mut App) {
    if !app.needs_rebuild {
        return;
    }
    app.needs_rebuild = false;
    if crate::ui::is_open(app) {
        crate::ui::rebuild_preserving_view(app);
    } else {
        crate::ui::close(app);
    }
}

type Builder = fn(&mut App, HWND, u32);
type Proc = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

fn run_modal(app: &mut App, class: &str, title: &str, height: i32, build: Builder, proc_: Proc) {
    let owner = app.dialog_owner();
    let instance = unsafe { GetModuleHandleW(ptr::null()) };
    let class_name = win::wide(class);
    unsafe {
        let mut window_class: WNDCLASSEXW = std::mem::zeroed();
        window_class.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
        window_class.lpfnWndProc = Some(proc_);
        window_class.hInstance = instance;
        window_class.hCursor = LoadCursorW(ptr::null_mut(), IDC_ARROW);
        window_class.lpszClassName = class_name.as_ptr();
        RegisterClassExW(&window_class);
    }

    let dpi = win::system_dpi();
    let width = win::scaled(580, dpi);
    let dialog_height = win::scaled(height, dpi);
    let (screen_width, screen_height) = win::screen_size();
    let title = win::wide(&app.tr(title));
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE | WS_CLIPCHILDREN,
            (screen_width - width).max(0) / 2,
            (screen_height - dialog_height).max(0) / 2,
            width,
            dialog_height,
            owner,
            ptr::null_mut(),
            instance,
            ptr::null_mut(),
        )
    };
    if hwnd.is_null() {
        crate::log::write(format_args!("[UI] cannot create the dialog: {}", win::last_error()));
        return;
    }
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, app as *mut App as isize);
        win::set_dark_frame(hwnd, app.dark_caption());
        EnableWindow(owner, 0);
        SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, app.icon_small as isize);
    }
    build(app, hwnd, dpi);
    theme_dialog_children(hwnd, app.dark());
    unsafe {
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
    }

    // Modal loop: every window of the thread keeps receiving its messages.
    let mut message: MSG = unsafe { std::mem::zeroed() };
    loop {
        let result = unsafe { GetMessageW(&mut message, ptr::null_mut(), 0, 0) };
        if result <= 0 {
            if result == 0 {
                // WM_QUIT: keep it for the main loop.
                unsafe {
                    PostQuitMessage(message.wParam as i32);
                }
            }
            break;
        }
        if unsafe { IsDialogMessageW(hwnd, &message) } == 0 {
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        if unsafe { IsWindow(hwnd) } == 0 {
            break;
        }
    }
    unsafe {
        EnableWindow(owner, 1);
        SetForegroundWindow(owner);
    }
}

fn build_preferences(app: &mut App, hwnd: HWND, dpi: u32) {
    let lang = app.lang;
    let scale = |value: i32| win::scaled(value, dpi);
    let label_width = scale(LABEL_WIDTH);
    let control_x = scale(MARGIN) + label_width + scale(12);
    let combo_width = scale(CONTROL_WIDTH);
    let mut y = scale(MARGIN);

    let add_label = |text: &str, y: i32, notify: bool| unsafe {
        let style = WS_CHILD | WS_VISIBLE | SS_LEFT | if notify { SS_NOTIFY } else { 0 };
        let label = CreateWindowExW(
            0,
            win::wide("STATIC").as_ptr(),
            win::wide(text).as_ptr(),
            style,
            scale(MARGIN),
            y + scale(6),
            label_width,
            scale(20),
            hwnd,
            ptr::null_mut(),
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(label, WM_SETFONT, app.font as usize, 1);
        label
    };
    let tooltip = unsafe {
        let tip = CreateWindowExW(
            WS_EX_TOPMOST,
            win::wide("tooltips_class32").as_ptr(),
            ptr::null(),
            WS_POPUP | TTS_ALWAYSTIP | TTS_NOPREFIX,
            0,
            0,
            0,
            0,
            hwnd,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
        );
        SendMessageW(tip, WM_SETFONT, app.font as usize, 1);
        SendMessageW(tip, TTM_SETMAXTIPWIDTH, 0, scale(340) as isize);
        win::set_control_theme(tip, app.dark());
        tip
    };
    PREFS_TOOLTIP.with(|cell| *cell.borrow_mut() = tooltip);
    let add_combo = |options: &[&str], selected: usize, id: u16, y: i32| unsafe {
        let combo = CreateWindowExW(
            0,
            win::wide("COMBOBOX").as_ptr(),
            ptr::null(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST | CBS_HASSTRINGS,
            control_x,
            y,
            combo_width,
            scale(200),
            hwnd,
            id as usize as *mut c_void,
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(combo, WM_SETFONT, app.font as usize, 1);
        for option in options {
            let text = win::wide(option);
            SendMessageW(combo, CB_ADDSTRING, 0, text.as_ptr() as isize);
        }
        SendMessageW(combo, CB_SETCURSEL, selected, 0);
        combo
    };

    add_label(&app.tr("Język aplikacji"), y, false);
    let language = match app.settings.language {
        None => 0,
        Some(index) if lang_from_index(index) == AppLang::PL => 2,
        Some(_) => 1,
    };
    let language_options = ["Systemowy", "English", "Polski"];
    let translated: Vec<String> = language_options.iter().map(|option| app.tr(option)).collect();
    let translated: Vec<&str> = translated.iter().map(String::as_str).collect();
    add_combo(&translated, language, ID_LANGUAGE, y);
    y += scale(ROW_HEIGHT);

    add_label(&app.tr("Motyw"), y, false);
    let theme_options = ["Systemowy", "Jasny", "Ciemny"];
    let translated: Vec<String> = theme_options.iter().map(|option| app.tr(option)).collect();
    let translated: Vec<&str> = translated.iter().map(String::as_str).collect();
    add_combo(&translated, app.settings.theme as usize, ID_THEME, y);
    y += scale(ROW_HEIGHT);

    add_label(&app.tr("Ikona w zasobniku"), y, false);
    let tray_options = ["Kolory motywu", "Jasna", "Ciemna"];
    let translated: Vec<String> = tray_options.iter().map(|option| app.tr(option)).collect();
    let translated: Vec<&str> = translated.iter().map(String::as_str).collect();
    add_combo(&translated, app.settings.tray_icon_style as usize, ID_TRAY_ICON, y);
    y += scale(ROW_HEIGHT);

    add_label(&app.tr("Klawisz ulubionych"), y, false);
    let modifiers = ["Ctrl", "Shift", "Alt", "Super"];
    add_combo(
        &modifiers,
        StepModifier::ALL.iter().position(|modifier| *modifier == app.settings.favorite_modifier).unwrap_or(0),
        ID_MODIFIER,
        y,
    );
    y += scale(ROW_HEIGHT);

    let firmware_label_text = format!("{}  \u{24d8}", app.tr("Wgrany pakiet firmware"));
    let firmware_label = add_label(&firmware_label_text, y, true);
    add_dialog_tooltip(
        tooltip,
        firmware_label,
        &app.tr("Monitor nie zgłasza pakietu, wybierz ręcznie. Z pakietem 2025-09-20 DyDs działa razem z Adaptive-Sync"),
    );
    let selected = FirmwarePackage::ALL
        .iter()
        .position(|package| *package == app.settings.firmware_package)
        .unwrap_or(0);
    add_combo(&["2024-12-11", "2025-09-20"], selected, ID_FIRMWARE, y);
    y += scale(ROW_HEIGHT);

    let add_check = |text: &str, checked: bool, id: u16, y: i32| unsafe {
        let button = CreateWindowExW(
            0,
            win::wide("BUTTON").as_ptr(),
            win::wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_AUTOCHECKBOX,
            scale(MARGIN),
            y,
            scale(360),
            scale(22),
            hwnd,
            id as usize as *mut c_void,
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(button, WM_SETFONT, app.font as usize, 1);
        SendMessageW(button, BM_SETCHECK, if checked { BST_CHECKED as usize } else { 0 }, 0);
        button
    };
    y += scale(8);
    let autostart = crate::settings::autostart_enabled();
    add_check(&app.tr("Uruchamiaj z systemem"), autostart, ID_AUTOSTART, y);
    y += scale(ROW_HEIGHT);
    add_check(&app.tr("Uruchamiaj zminimalizowany"), app.settings.start_minimized, ID_MINIMIZED, y);

    // Close button.
    unsafe {
        let button = CreateWindowExW(
            0,
            win::wide("BUTTON").as_ptr(),
            win::wide(&app.tr("Zamknij")).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_DEFPUSHBUTTON,
            scale(MARGIN),
            y + scale(ROW_HEIGHT + 8),
            scale(110),
            scale(28),
            hwnd,
            ID_CLOSE as usize as *mut c_void,
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(button, WM_SETFONT, app.font as usize, 1);
    }
    let _ = lang;
}

fn build_about(app: &mut App, hwnd: HWND, dpi: u32) {
    let scale = |value: i32| win::scaled(value, dpi);
    unsafe {
        let title = CreateWindowExW(
            0,
            win::wide("STATIC").as_ptr(),
            win::wide("Titan Control").as_ptr(),
            WS_CHILD | WS_VISIBLE | SS_LEFT,
            scale(MARGIN),
            scale(MARGIN),
            scale(360),
            scale(28),
            hwnd,
            ptr::null_mut(),
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(title, WM_SETFONT, app.bold as usize, 1);
        let version = CreateWindowExW(
            0,
            win::wide("STATIC").as_ptr(),
            win::wide(settings::APP_VERSION).as_ptr(),
            WS_CHILD | WS_VISIBLE | SS_LEFT,
            scale(MARGIN),
            scale(MARGIN + 28),
            scale(360),
            scale(20),
            hwnd,
            ptr::null_mut(),
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(version, WM_SETFONT, app.font as usize, 1);

        let links = [
            (ID_WEBSITE, "Strona programu", "https://github.com/mkarenko/titan-control"),
            (ID_ISSUES, "Zgłoś błąd", "https://github.com/mkarenko/titan-control/issues/new"),
            (ID_ABOUT_LOG, "Pokaż plik logu", ""),
            (ID_COFFEE, "Buy me a coffee", "https://buymeacoffee.com/mkarenko"),
        ];
        let mut y = scale(MARGIN + 60);
        for (id, text, _) in links {
            let button = CreateWindowExW(
                0,
                win::wide("BUTTON").as_ptr(),
                win::wide(&app.tr(text)).as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON,
                scale(MARGIN),
                y,
                scale(220),
                scale(26),
                hwnd,
                id as usize as *mut c_void,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            );
            SendMessageW(button, WM_SETFONT, app.font as usize, 1);
            y += scale(32);
        }
        let close = CreateWindowExW(
            0,
            win::wide("BUTTON").as_ptr(),
            win::wide(&app.tr("Zamknij")).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_DEFPUSHBUTTON,
            scale(MARGIN),
            y + scale(8),
            scale(110),
            scale(28),
            hwnd,
            ID_CLOSE as usize as *mut c_void,
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(close, WM_SETFONT, app.font as usize, 1);
    }
}

fn handle_command(app: &mut App, hwnd: HWND, id: u16, notification: u32) {
    match id {
        ID_CLOSE => {
            if notification != BN_CLICKED {
                return;
            }
            close_modal(app, hwnd);
        }
        ID_LANGUAGE => {
            if notification != CBN_SELCHANGE as u32 {
                return;
            }
            let selection = unsafe { SendMessageW(GetDlgItem(hwnd, ID_LANGUAGE as i32), CB_GETCURSEL, 0, 0) } as usize;
            let saved = match selection {
                1 => Some(lang_index(&AppLang::EN)),
                2 => Some(lang_index(&AppLang::PL)),
                _ => None,
            };
            let new_lang = match saved {
                Some(index) => lang_from_index(index),
                None => crate::i18n::detect_system_lang(),
            };
            if app.settings.language == saved && app.lang == new_lang {
                return;
            }
            app.settings.language = saved;
            app.lang = new_lang;
            app.save_settings();
            // The settings window is built again (in the new language) once this dialog closes.
            app.needs_rebuild = true;
            crate::label_refresh(app);
        }
        ID_THEME => {
            if notification != CBN_SELCHANGE as u32 {
                return;
            }
            let selection = unsafe { SendMessageW(GetDlgItem(hwnd, ID_THEME as i32), CB_GETCURSEL, 0, 0) } as u32;
            if app.settings.theme == selection {
                return;
            }
            let palette_changed = crate::ui::palette_for(app.settings.theme).background
                != crate::ui::palette_for(selection).background;
            app.settings.theme = selection;
            app.save_settings();
            if palette_changed {
                app.needs_rebuild = true;
            }
            win::set_process_dark_mode(app.dark());
            theme_dialog_children(hwnd, app.dark());
            unsafe {
                win::set_dark_frame(hwnd, app.dark_caption());
                InvalidateRect(hwnd, ptr::null(), 1);
            }
        }
        ID_TRAY_ICON => {
            if notification != CBN_SELCHANGE as u32 {
                return;
            }
            let selection = unsafe { SendMessageW(GetDlgItem(hwnd, ID_TRAY_ICON as i32), CB_GETCURSEL, 0, 0) } as usize;
            app.settings.tray_icon_style = match selection {
                1 => TrayIconStyle::Light,
                2 => TrayIconStyle::Dark,
                _ => TrayIconStyle::Theme,
            };
            app.save_settings();
            crate::tray::refresh(app);
        }
        ID_MODIFIER => {
            if notification != CBN_SELCHANGE as u32 {
                return;
            }
            let selection =
                unsafe { SendMessageW(GetDlgItem(hwnd, ID_MODIFIER as i32), CB_GETCURSEL, 0, 0) } as usize;
            app.settings.favorite_modifier =
                StepModifier::ALL.get(selection).copied().unwrap_or_default();
            app.save_settings();
            crate::register_hotkeys(app);
        }
        ID_FIRMWARE => {
            if notification != CBN_SELCHANGE as u32 {
                return;
            }
            let selection =
                unsafe { SendMessageW(GetDlgItem(hwnd, ID_FIRMWARE as i32), CB_GETCURSEL, 0, 0) } as usize;
            if let Some(&package) = FirmwarePackage::ALL.get(selection) {
                app.settings.firmware_package = package;
                app.save_settings();
                let _ = app.worker.send(crate::monitor::WorkerCmd::DydsWithAdaptiveSync(
                    package.dyds_with_adaptive_sync(),
                ));
            }
        }
        ID_AUTOSTART => {
            if notification != BN_CLICKED {
                return;
            }
            let checked = unsafe { SendMessageW(GetDlgItem(hwnd, ID_AUTOSTART as i32), BM_GETCHECK, 0, 0) }
                == BST_CHECKED as isize;
            app.settings.auto_start = checked;
            app.save_settings();
            crate::app::sync_autostart(app);
        }
        ID_MINIMIZED => {
            if notification != BN_CLICKED {
                return;
            }
            let checked = unsafe { SendMessageW(GetDlgItem(hwnd, ID_MINIMIZED as i32), BM_GETCHECK, 0, 0) }
                == BST_CHECKED as isize;
            app.settings.start_minimized = checked;
            app.save_settings();
        }
        ID_WEBSITE if notification == BN_CLICKED => settings::open_url("https://github.com/mkarenko/titan-control"),
        ID_ISSUES if notification == BN_CLICKED => {
            settings::open_url("https://github.com/mkarenko/titan-control/issues/new")
        }
        ID_ABOUT_LOG if notification == BN_CLICKED => settings::open_log_folder(),
        ID_COFFEE if notification == BN_CLICKED => settings::open_url("https://buymeacoffee.com/mkarenko"),
        _ => {}
    }
}

fn close_modal(app: &App, hwnd: HWND) {
    PREFS_TOOLTIP.with(|cell| *cell.borrow_mut() = ptr::null_mut());
    let owner = app.dialog_owner();
    unsafe {
        // Re-enable the owner before destroying the modal popup so Windows hands focus directly back to it
        // without flashing an unrelated window behind the app.
        if !owner.is_null() {
            EnableWindow(owner, 1);
        }
        DestroyWindow(hwnd);
    }
}

unsafe extern "system" fn preferences_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { dialog_proc(hwnd, message, wparam, lparam) }
}

unsafe extern "system" fn about_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { dialog_proc(hwnd, message, wparam, lparam) }
}

unsafe extern "system" fn dialog_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut App };
    if raw.is_null() {
        return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
    }
    let app = unsafe { &mut *raw };
    let _ = &app;
    match message {
        WM_COMMAND => {
            handle_command(app, hwnd, (wparam & 0xFFFF) as u16, ((wparam >> 16) & 0xFFFF) as u32);
            0
        }
        WM_CLOSE => {
            close_modal(app, hwnd);
            0
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
            let palette = crate::ui::palette_for(app.settings.theme);
            let background = palette.card;
            let text = palette.text;
            unsafe {
                SetTextColor(wparam as _, text);
                SetBkColor(wparam as _, background);
                use windows_sys::Win32::Graphics::Gdi::CreateSolidBrush;
                let brush = DLG_BRUSH.with(|cell| {
                    let mut entry = cell.borrow_mut();
                    if let Some((cached_color, cached_brush)) = *entry {
                        if cached_color == background && !cached_brush.is_null() {
                            return cached_brush;
                        }
                        DeleteObject(cached_brush as _);
                    }
                    let new_brush = CreateSolidBrush(background);
                    *entry = Some((background, new_brush));
                    new_brush
                });
                brush as LRESULT
            }
        }
        WM_ERASEBKGND => {
            let palette = crate::ui::palette_for(app.settings.theme);
            let background = palette.card;
            unsafe {
                use windows_sys::Win32::Graphics::Gdi::{CreateSolidBrush, FillRect};
                let brush = CreateSolidBrush(background);
                let mut rect: RECT = std::mem::zeroed();
                GetClientRect(hwnd, &mut rect);
                FillRect(wparam as _, &rect, brush);
                DeleteObject(brush as _);
            }
            1
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn theme_dialog_children(hwnd: HWND, dark: bool) {
    PREFS_TOOLTIP.with(|cell| {
        let tip = *cell.borrow();
        if !tip.is_null() {
            win::set_control_theme(tip, dark);
        }
    });
    unsafe {
        let mut child = GetWindow(hwnd, GW_CHILD);
        while !child.is_null() {
            let mut class_buf = [0u16; 32];
            let len = GetClassNameW(child, class_buf.as_mut_ptr(), class_buf.len() as i32) as usize;
            let class_name = String::from_utf16_lossy(&class_buf[..len]);
            if class_name.eq_ignore_ascii_case("ComboBox") {
                win::set_combo_theme(child, dark);
            } else {
                win::set_control_theme(child, dark);
            }
            child = GetWindow(child, GW_HWNDNEXT);
        }
    }
}

fn add_dialog_tooltip(tooltip: HWND, target: HWND, text: &str) {
    if tooltip.is_null() || target.is_null() {
        return;
    }
    let mut wide_text = win::wide(text);
    let mut info: TTTOOLINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<TTTOOLINFOW>() as u32;
    info.uFlags = TTF_IDISHWND | TTF_SUBCLASS;
    info.hwnd = target;
    info.uId = target as usize;
    info.lpszText = wide_text.as_mut_ptr();
    unsafe {
        SendMessageW(tooltip, TTM_ADDTOOLW, 0, &info as *const TTTOOLINFOW as isize);
    }
}

thread_local! {
    /// Background brush of the dialogs (`(color, HBRUSH)`), updated automatically if the theme changes.
    static DLG_BRUSH: std::cell::RefCell<Option<(u32, windows_sys::Win32::Graphics::Gdi::HBRUSH)>> = const { std::cell::RefCell::new(None) };
    static PREFS_TOOLTIP: std::cell::RefCell<HWND> = const { std::cell::RefCell::new(ptr::null_mut()) };
    static PICKER_ALL: std::cell::RefCell<Vec<crate::auto_hdr::DiscoveredApp>> = const { std::cell::RefCell::new(Vec::new()) };
    static PICKER_FILTERED: std::cell::RefCell<Vec<usize>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn build_picker(app: &mut App, hwnd: HWND, dpi: u32) {
    let margin = win::scaled(MARGIN, dpi);
    let width = win::scaled(560, dpi) - margin * 2;
    let btn_w = win::scaled(110, dpi);
    let btn_h = win::scaled(30, dpi);
    let filter_h = win::scaled(28, dpi);
    let list_h = win::scaled(330, dpi);

    unsafe {
        let filter = CreateWindowExW(
            0x0000_0200, // WS_EX_CLIENTEDGE
            win::wide("EDIT").as_ptr(),
            ptr::null(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | 0x0080, // ES_AUTOHSCROLL
            margin,
            margin,
            width,
            filter_h,
            hwnd,
            ID_PICKER_FILTER as usize as *mut c_void,
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(filter, WM_SETFONT, app.font as usize, 1);
        let cue = win::wide(&app.tr("Filtruj listę..."));
        SendMessageW(filter, 0x1501 /* EM_SETCUEBANNER */, 1, cue.as_ptr() as isize);

        let list = CreateWindowExW(
            0x0000_0200,
            win::wide("LISTBOX").as_ptr(),
            ptr::null(),
            WS_CHILD
                | WS_VISIBLE
                | WS_TABSTOP
                | WS_VSCROLL
                | (LBS_NOTIFY | LBS_HASSTRINGS | LBS_NOINTEGRALHEIGHT) as u32,
            margin,
            margin + filter_h + win::scaled(10, dpi),
            width,
            list_h,
            hwnd,
            ID_PICKER_LIST as usize as *mut c_void,
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(list, WM_SETFONT, app.font as usize, 1);

        let bottom_y = margin + filter_h + win::scaled(10, dpi) + list_h + win::scaled(12, dpi);
        let add_btn = CreateWindowExW(
            0,
            win::wide("BUTTON").as_ptr(),
            win::wide(&app.tr("Dodaj")).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_DEFPUSHBUTTON,
            margin + width - btn_w * 2 - win::scaled(8, dpi),
            bottom_y,
            btn_w,
            btn_h,
            hwnd,
            ID_PICKER_ADD as usize as *mut c_void,
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(add_btn, WM_SETFONT, app.font as usize, 1);

        let close_btn = CreateWindowExW(
            0,
            win::wide("BUTTON").as_ptr(),
            win::wide(&app.tr("Zamknij")).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON,
            margin + width - btn_w,
            bottom_y,
            btn_w,
            btn_h,
            hwnd,
            ID_CLOSE as usize as *mut c_void,
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(close_btn, WM_SETFONT, app.font as usize, 1);
    }
    populate_picker_list(app, hwnd, "");
}

fn populate_picker_list(app: &App, hwnd: HWND, query: &str) {
    let list = unsafe { GetDlgItem(hwnd, ID_PICKER_LIST as i32) };
    if list.is_null() {
        return;
    }
    let needle = query.trim().to_lowercase();
    unsafe {
        SendMessageW(list, LB_RESETCONTENT, 0, 0);
    }
    PICKER_ALL.with(|all_cell| {
        PICKER_FILTERED.with(|flt_cell| {
            let all = all_cell.borrow();
            let mut filtered = flt_cell.borrow_mut();
            filtered.clear();
            for (idx, item) in all.iter().enumerate() {
                if !needle.is_empty() {
                    let hay = format!("{} {} {}", item.name, item.process, item.path).to_lowercase();
                    if !needle.split_whitespace().all(|tok| hay.contains(tok)) {
                        continue;
                    }
                }
                let already = app
                    .settings
                    .auto_profile_apps
                    .iter()
                    .any(|a| a.process.eq_ignore_ascii_case(&item.process));
                if already {
                    continue;
                }
                filtered.push(idx);
                let label = format!("{}  ({}.exe)", item.name, item.process);
                let wide = win::wide(&label);
                unsafe {
                    SendMessageW(list, LB_ADDSTRING, 0, wide.as_ptr() as isize);
                }
            }
            if filtered.is_empty() {
                let empty = win::wide(&app.tr("Brak wykrytych aplikacji"));
                unsafe {
                    SendMessageW(list, LB_ADDSTRING, 0, empty.as_ptr() as isize);
                }
            } else {
                unsafe {
                    SendMessageW(list, LB_SETCURSEL, 0, 0);
                }
            }
        });
    });
}

fn add_selected_picker_app(app: &mut App, hwnd: HWND) {
    let list = unsafe { GetDlgItem(hwnd, ID_PICKER_LIST as i32) };
    let sel = unsafe { SendMessageW(list, LB_GETCURSEL, 0, 0) };
    if sel < 0 {
        return;
    }
    let picked = PICKER_FILTERED.with(|flt_cell| {
        let flt = flt_cell.borrow();
        let all_idx = *flt.get(sel as usize)?;
        PICKER_ALL.with(|all_cell| all_cell.borrow().get(all_idx).cloned())
    });
    if let Some(entry) = picked
        && crate::app::add_auto_hdr_app(app, entry.name, entry.process, Some(entry.path))
    {
        crate::ui::refresh_auto_profiles_page(app);
        let filter_hwnd = unsafe { GetDlgItem(hwnd, ID_PICKER_FILTER as i32) };
        let len = unsafe { GetWindowTextLengthW(filter_hwnd) };
        let mut buf = vec![0u16; len.max(0) as usize + 1];
        let copied = unsafe { GetWindowTextW(filter_hwnd, buf.as_mut_ptr(), buf.len() as i32) } as usize;
        let q = String::from_utf16_lossy(&buf[..copied]);
        populate_picker_list(app, hwnd, &q);
    }
}

unsafe extern "system" fn picker_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut App };
    if raw.is_null() {
        return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
    }
    let app = unsafe { &mut *raw };
    match message {
        WM_COMMAND => {
            let item = (wparam & 0xFFFF) as u16;
            let notif = ((wparam >> 16) & 0xFFFF) as u32;
            match item {
                ID_PICKER_FILTER if notif == 0x0300 /* EN_CHANGE */ => {
                    let filter_hwnd = unsafe { GetDlgItem(hwnd, ID_PICKER_FILTER as i32) };
                    let len = unsafe { GetWindowTextLengthW(filter_hwnd) };
                    let mut buf = vec![0u16; len.max(0) as usize + 1];
                    let copied = unsafe { GetWindowTextW(filter_hwnd, buf.as_mut_ptr(), buf.len() as i32) } as usize;
                    let q = String::from_utf16_lossy(&buf[..copied]);
                    populate_picker_list(app, hwnd, &q);
                }
                ID_PICKER_LIST if notif == LBN_DBLCLK => add_selected_picker_app(app, hwnd),
                ID_PICKER_ADD if notif == BN_CLICKED => add_selected_picker_app(app, hwnd),
                ID_CLOSE if notif == BN_CLICKED => close_modal(app, hwnd),
                _ => {}
            }
            0
        }
        WM_CTLCOLORLISTBOX | WM_CTLCOLOREDIT => unsafe { dialog_proc(hwnd, WM_CTLCOLORSTATIC, wparam, lparam) },
        _ => unsafe { dialog_proc(hwnd, message, wparam, lparam) },
    }
}

fn build_new_profile(app: &App, hwnd: HWND, dpi: u32) {
    let margin = win::scaled(MARGIN, dpi);
    let width = win::scaled(560, dpi) - margin * 2;
    let btn_w = win::scaled(104, dpi);
    let btn_h = win::scaled(30, dpi);
    let label_h = win::scaled(20, dpi);
    let edit_h = win::scaled(28, dpi);

    unsafe {
        let label = CreateWindowExW(
            0,
            win::wide("STATIC").as_ptr(),
            win::wide(&app.tr("Nazwa profilu")).as_ptr(),
            WS_CHILD | WS_VISIBLE | SS_LEFT,
            margin,
            margin,
            width,
            label_h,
            hwnd,
            ptr::null_mut(),
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(label, WM_SETFONT, app.font as usize, 1);

        let edit = CreateWindowExW(
            0x0000_0200,
            win::wide("EDIT").as_ptr(),
            ptr::null(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | 0x0080,
            margin,
            margin + label_h + win::scaled(6, dpi),
            width,
            edit_h,
            hwnd,
            ID_PROFILE_NAME_EDIT as usize as *mut c_void,
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(edit, WM_SETFONT, app.font as usize, 1);
        SetFocus(edit);

        let btn_y = margin + label_h + edit_h + win::scaled(18, dpi);
        let ok_btn = CreateWindowExW(
            0,
            win::wide("BUTTON").as_ptr(),
            win::wide(&app.tr("Zapisz")).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_DEFPUSHBUTTON,
            margin + width - btn_w * 2 - win::scaled(8, dpi),
            btn_y,
            btn_w,
            btn_h,
            hwnd,
            ID_PROFILE_NAME_OK as usize as *mut c_void,
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(ok_btn, WM_SETFONT, app.font as usize, 1);

        let cancel_btn = CreateWindowExW(
            0,
            win::wide("BUTTON").as_ptr(),
            win::wide(&app.tr("Anuluj")).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON,
            margin + width - btn_w,
            btn_y,
            btn_w,
            btn_h,
            hwnd,
            ID_CLOSE as usize as *mut c_void,
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(cancel_btn, WM_SETFONT, app.font as usize, 1);
    }
}

unsafe extern "system" fn new_profile_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut App };
    if raw.is_null() {
        return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
    }
    let app = unsafe { &mut *raw };
    match message {
        WM_COMMAND => {
            let item = (wparam & 0xFFFF) as u16;
            let notif = ((wparam >> 16) & 0xFFFF) as u32;
            if (item == ID_PROFILE_NAME_OK || item == IDOK as u16) && notif == BN_CLICKED {
                let edit = unsafe { GetDlgItem(hwnd, ID_PROFILE_NAME_EDIT as i32) };
                let len = unsafe { GetWindowTextLengthW(edit) };
                let mut buf = vec![0u16; len.max(0) as usize + 1];
                let copied = unsafe { GetWindowTextW(edit, buf.as_mut_ptr(), buf.len() as i32) } as usize;
                let name = String::from_utf16_lossy(&buf[..copied]);
                if crate::app::create_profile(app, name) {
                    close_modal(app, hwnd);
                }
                return 0;
            }
            if (item == ID_CLOSE || item == IDCANCEL as u16) && notif == BN_CLICKED {
                close_modal(app, hwnd);
                return 0;
            }
            0
        }
        WM_CTLCOLOREDIT => unsafe { dialog_proc(hwnd, WM_CTLCOLORSTATIC, wparam, lparam) },
        _ => unsafe { dialog_proc(hwnd, message, wparam, lparam) },
    }
}
