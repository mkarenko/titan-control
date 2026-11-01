//! Titan Control for Windows: native monitor control over DDC/CI with a tray icon and a settings window that is
//! created only while it is open.
//!
//! The program has one UI thread and one worker thread. The worker talks to the monitor and posts its results to the
//! host window; nothing polls the monitor unless the user asks for it.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[macro_use]
mod log;

mod app;
mod cache_store;
mod ddc;
mod i18n;
mod monitor;
mod settings;
mod tray;
mod ui;
mod win;

use std::{ffi::c_void, ptr, sync::mpsc};

use windows_sys::Win32::Foundation::{GetLastError, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{CreateFontW, DeleteObject, HFONT};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::SetProcessDpiAwarenessContext;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN,
};
use windows_sys::Win32::UI::Controls::{InitCommonControlsEx, ICC_BAR_CLASSES, ICC_STANDARD_CLASSES, INITCOMMONCONTROLSEX};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use app::App;
use i18n::AppLang;
use settings::StepModifier;

/// Worker notifications: the UI thread reads the channel when this arrives.
pub const WM_UI_COMMAND: u32 = WM_APP + 1;
/// Message of the tray icon.
pub const WM_TRAY: u32 = WM_APP + 2;
/// A second instance asks the first one to show its window.
pub const WM_SHOW_WINDOW: u32 = WM_APP + 3;
/// A second instance started with `--quit` asks the first one to stop.
pub const WM_QUIT_APP: u32 = WM_APP + 4;

const HOST_CLASS: &str = "TitanControlHost";
const MUTEX_NAME: &str = "Local\\TitanControl-single-instance";
const HOTKEY_FIRST: i32 = 100;
/// `DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2` (a pseudo handle, `-4`).
const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;

const ICON_THEME: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon-theme.ico"));

fn main() {
    // Per-monitor v2 awareness: the layout is computed in real pixels, so the window stays sharp at every scale.
    // The manifest asks for the same; setting it here keeps it true even when the manifest is not applied.
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 as *mut c_void);
    }
    log::init();

    // One process only: a second start brings the running one to the front.
    if !single_instance() {
        return;
    }
    unsafe {
        let mut controls: INITCOMMONCONTROLSEX = std::mem::zeroed();
        controls.dwSize = std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32;
        controls.dwICC = ICC_BAR_CLASSES | ICC_STANDARD_CLASSES;
        InitCommonControlsEx(&controls);
    }

    let settings = settings::load();
    let lang = resolve_lang(settings.language);
    let (worker_tx, worker_rx) = mpsc::channel::<monitor::WorkerCmd>();
    let (ui_tx, ui_rx) = mpsc::channel::<monitor::UiCmd>();
    let host = create_host();
    if host.is_null() {
        log!("[APP] cannot create the host window: {}", win::last_error());
        return;
    }

    let dpi = win::system_dpi();
    let app = App {
        worker: worker_tx,
        notifications: ui_rx,
        state: Default::default(),
        info: None,
        window: None,
        connected: false,
        profile_loading: false,
        profile_started: None,
        color_enhance_level: 0,
        user_saturation: None,
        pending_output_range: None,
        modes: Vec::new(),
        current_mode: None,
        connector: String::new(),
        icon_small: win::icon_for_dpi(ICON_THEME, host, 16),
        icon_large: win::icon_for_dpi(ICON_THEME, host, 32),
        font: ui_font(dpi, false),
        bold: ui_font(dpi, true),
        host,
        tray_added: false,
        start_minimized: false,
        needs_rebuild: false,
        quit: false,
        lang,
        settings,
    };
    let app = Box::into_raw(Box::new(app));
    unsafe {
        SetWindowLongPtrW(host, GWLP_USERDATA, app as isize);
    }
    let app = unsafe { &mut *app };
    // Only the explicit dark theme switches uxtheme to dark; with `System` the standard controls stay as Windows draws them.
    win::set_process_dark_mode(app.dark());
    app.sync_startup_state();

    monitor::start_worker(worker_rx, monitor::UiSink::new(ui_tx, host as isize), |monitor| {
        ddc::MsiDdc::open(monitor).ok()
    });
    tray::create(app);
    register_hotkeys(app);
    app::sync_autostart(app);
    app::schedule_trim(app);
    // `--minimized` starts in the tray; the autostart entry passes it.
    app.start_minimized = app.settings.start_minimized || std::env::args().any(|argument| argument == "--minimized");
    if !app.start_minimized {
        ui::open(app);
    }

    let mut message: MSG = unsafe { std::mem::zeroed() };
    loop {
        let result = unsafe { GetMessageW(&mut message, ptr::null_mut(), 0, 0) };
        if result <= 0 {
            break;
        }
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        if app.quit {
            break;
        }
    }

    log!("[APP] exiting");
    tray::remove(app);
    unregister_hotkeys(app);
    win::destroy_icon(app.icon_small);
    win::destroy_icon(app.icon_large);
    unsafe {
        DeleteObject(app.font as *mut c_void);
        DeleteObject(app.bold as *mut c_void);
    }
}

/// Creates the named mutex; `false` when another instance already runs (it was told to show its window or to quit).
fn single_instance() -> bool {
    use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
    let name = win::wide(MUTEX_NAME);
    // A second start with `--quit` stops the running instance instead of showing it: handy for scripts and updates.
    let quit = std::env::args().any(|argument| argument == "--quit");
    unsafe {
        let handle = CreateMutexW(ptr::null_mut(), 0, name.as_ptr());
        if handle.is_null() {
            return true;
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let class = win::wide(HOST_CLASS);
            let title = win::wide(settings::APP_DISPLAY_NAME);
            let existing = FindWindowW(class.as_ptr(), title.as_ptr());
            if !existing.is_null() {
                let message = if quit { WM_QUIT_APP } else { WM_SHOW_WINDOW };
                PostMessageW(existing, message, 0, 0);
            }
            return false;
        }
        true
    }
}

fn ui_font(dpi: u32, bold: bool) -> HFONT {
    let face = win::wide("Segoe UI");
    unsafe {
        CreateFontW(
            -win::scaled(14, dpi),
            0,
            0,
            0,
            if bold { 700 } else { 400 },
            0,
            0,
            0,
            1,
            0,
            0,
            0,
            0,
            face.as_ptr(),
        )
    }
}

fn create_host() -> HWND {
    let instance = unsafe { GetModuleHandleW(ptr::null()) };
    let class_name = win::wide(HOST_CLASS);
    let title = win::wide(settings::APP_DISPLAY_NAME);
    unsafe {
        let mut class: WNDCLASSEXW = std::mem::zeroed();
        class.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
        class.lpfnWndProc = Some(host_proc);
        class.hInstance = instance;
        class.lpszClassName = class_name.as_ptr();
        if RegisterClassExW(&class) == 0 && GetLastError() != 0 {
            log!("[APP] RegisterClassExW: {}", win::last_error());
        }
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class_name.as_ptr(),
            title.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            instance,
            ptr::null_mut(),
        )
    }
}

/// Registers the favorite hotkeys: `<favorites key>+Alt+1..5` select the favorite picture modes.
pub fn register_hotkeys(app: &mut App) {
    unregister_hotkeys(app);
    let modifier = match app.settings.favorite_modifier {
        StepModifier::Ctrl => MOD_CONTROL,
        StepModifier::Shift => MOD_SHIFT,
        StepModifier::Alt => MOD_CONTROL,
        StepModifier::Super => MOD_WIN,
    } | MOD_ALT;
    let favorites = app.settings.favorite_picture_mode_set();
    for (index, _mode) in favorites.iter().enumerate().take(5) {
        let key = '1' as u32 + index as u32;
        let registered = unsafe { RegisterHotKey(app.host, HOTKEY_FIRST + index as i32, modifier, key) };
        if registered == 0 {
            log!("[HOTKEY] {index}: {}", win::last_error());
        }
    }
}

fn unregister_hotkeys(app: &mut App) {
    for index in 0..5 {
        unsafe {
            UnregisterHotKey(app.host, HOTKEY_FIRST + index);
        }
    }
}

/// Language: the saved choice, else the Windows one; only English and Polish are translated.
fn resolve_lang(saved: Option<u32>) -> AppLang {
    let language = saved.map(i18n::lang_from_index).unwrap_or_else(i18n::detect_system_lang);
    if language == AppLang::PL { AppLang::PL } else { AppLang::EN }
}

/// Shows a notification next to the tray icon.
pub fn notify(app: &App, title: &str, text: &str) {
    tray::notify(app, &app.tr(title), &app.tr(text), true);
}

/// Re-reads the tray menu texts (after a language change).
pub fn label_refresh(app: &mut App) {
    tray::refresh(app);
}

unsafe extern "system" fn host_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut App };
    if raw.is_null() {
        return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
    }
    let app = unsafe { &mut *raw };
    match message {
        WM_TRAY => {
            // Version 4 packs the cursor position into `wParam` and the event into the low word of `lParam`.
            let x = (wparam as u32 & 0xFFFF) as u16 as i16 as i32;
            let y = ((wparam as u32) >> 16) as u16 as i16 as i32;
            tray::on_callback(app, x, y, lparam);
            0
        }
        WM_UI_COMMAND => {
            app::drain_messages(app);
            0
        }
        WM_SHOW_WINDOW => {
            ui::open(app);
            0
        }
        WM_QUIT_APP => {
            app.quit = true;
            ui::close(app);
            0
        }
        WM_HOTKEY => {
            {
                let index = (wparam as i32 - HOTKEY_FIRST).max(0) as usize;
                if let Some(mode) = app.settings.favorite_picture_mode_set().get(index).copied() {
                    let custom = app
                        .get(monitor::VCP_MODE)
                        .and_then(monitor::picture_mode_from_vcp)
                        .map(|(_, custom)| custom)
                        .unwrap_or(false);
                    if let Some(value) = monitor::picture_mode_vcp_value(mode as usize, custom) {
                        app.start_profile_load();
                        app.set(monitor::VCP_MODE, value);
                        ui::apply_state(app);
                    }
                }
            }
            0
        }
        WM_TIMER => {
            app::handle_timer(app, wparam as usize);
            0
        }
        WM_ENDSESSION | WM_QUERYENDSESSION => {
            unsafe {
                for index in 0..5 {
                    UnregisterHotKey(hwnd, HOTKEY_FIRST + index);
                }
            }
            if message == WM_ENDSESSION && wparam != 0 {
                return 0;
            }
            1
        }
        WM_DESTROY => {
            unsafe {
                PostQuitMessage(0);
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

impl App {
    /// Applies the loaded settings to the worker (firmware package) and to the theme.
    fn sync_startup_state(&mut self) {
        let _ = self.worker.send(monitor::WorkerCmd::DydsWithAdaptiveSync(
            self.settings.firmware_package.dyds_with_adaptive_sync(),
        ));
        let _ = self.dark();
    }
}
