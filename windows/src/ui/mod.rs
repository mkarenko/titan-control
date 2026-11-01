//! The settings window.
//!
//! The window is built from the declarative pages in [`rows`], created when the user opens it and destroyed when it is
//! closed, so none of its memory is kept while the application sits in the tray. Controls are plain Win32 children
//! (track bars, check boxes, combo boxes, buttons): no GUI toolkit and no web view.

use std::{collections::HashMap, ffi::c_void, ptr};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CreateFontW, CreatePen, CreateSolidBrush, DeleteObject, FillRect, GetStockObject, InvalidateRect, MoveToEx,
    Rectangle, SelectObject, SetBkColor, SetBkMode, SetTextColor, UpdateWindow, HBRUSH, HFONT, NULL_BRUSH,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::{
    InitCommonControlsEx, SetScrollInfo, SetScrollPos, BST_CHECKED, BST_UNCHECKED, DRAWITEMSTRUCT, ICC_BAR_CLASSES, ICC_STANDARD_CLASSES,
    INITCOMMONCONTROLSEX, ODT_BUTTON, TBS_HORZ, TBS_NOTICKS, TBM_SETPAGESIZE, TBM_SETPOS, TBM_SETRANGE, TTS_ALWAYSTIP, TTS_NOPREFIX,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Static control styles and the track bar read message (`WinUser.h`), kept here because the bindings split them
/// over modules that would otherwise have to be imported name by name.
#[allow(non_upper_case_globals)]
const SS_LEFT: u32 = 0x0000;
#[allow(non_upper_case_globals)]
const SS_ENDELLIPSIS: u32 = 0x4000;
#[allow(non_upper_case_globals)]
const SS_ETCHEDHORZ: u32 = 0x0010;
#[allow(non_upper_case_globals)]
const BS_PUSHBUTTON: u32 = 0x0000;
#[allow(non_upper_case_globals)]
const BS_AUTOCHECKBOX: u32 = 0x0003;
#[allow(non_upper_case_globals)]
const BS_OWNERDRAW: u32 = 0x000B;
#[allow(non_upper_case_globals)]
const LBS_NOTIFY: u32 = 0x0001;
#[allow(non_upper_case_globals)]
const LBS_NOINTEGRALHEIGHT: u32 = 0x0100;
#[allow(non_upper_case_globals)]
const CBS_DROPDOWNLIST: u32 = 0x0003;
#[allow(non_upper_case_globals)]
const CBS_HASSTRINGS: u32 = 0x0200;
#[allow(non_upper_case_globals)]
const TBM_GETPOS: u32 = 0x0400;
#[allow(non_upper_case_globals)]
const BS_PUSHLIKE: u32 = 0x1000;

use crate::{
    app::{App, SystemKind},
    i18n::{tr, usage_minutes, AppLang},
    monitor,
    settings::FirmwarePackage,
    win,
};

use rows::{Action, DynText, Kind};

pub mod prefs;
pub mod rows;

// ───────────────────────────── metrics at 96 DPI ─────────────────────────────

const MARGIN: i32 = 14;
const LABEL_WIDTH: i32 = 250;
const ROW_HEIGHT: i32 = 28;
const HEADER_HEIGHT: i32 = 32;
const SECTION_HEIGHT: i32 = 32;
const SLIDER_WIDTH: i32 = 240;
const VALUE_WIDTH: i32 = 44;
const COMBO_WIDTH: i32 = 250;
const SHORT_COMBO_WIDTH: i32 = 160;
const BUTTON_WIDTH: i32 = 220;
const STAR_SIZE: i32 = 18;
const SWATCH_SIZE: i32 = 26;
const SWATCH_GAP: i32 = 4;
const INDENT: i32 = 18;
const NAV_WIDTH: i32 = 170;
const TOOLBAR_HEIGHT: i32 = 42;
const STATUS_HEIGHT: i32 = 26;

const ID_REFRESH: u16 = 100;
const ID_PREFERENCES: u16 = 101;
const ID_ABOUT: u16 = 102;
const ID_LOG: u16 = 103;
const ID_NAV: u16 = 104;

pub struct Palette {
    pub background: u32,
    pub text: u32,
    pub line: u32,
}

pub const LIGHT: Palette = Palette { background: 0x00F3_F3F3, text: 0x0020_2020, line: 0x00C0_C0C0 };
pub const DARK: Palette = Palette { background: 0x0020_2020, text: 0x00F0_F0F0, line: 0x0050_5050 };

// ───────────────────────────── model ─────────────────────────────

enum KindUi {
    Slider { track: HWND, value: HWND, factor: u16, max: i32 },
    Switch { button: HWND, on: u16, off: u16 },
    Choice { combo: HWND, values: &'static [u16] },
    Swatches { buttons: Vec<HWND>, values: &'static [u16], colors: &'static [u32], selected: usize },
    Action { button: HWND, action: Action },
    PictureMode { combo: HWND },
    Variant { combo: HWND },
    Firmware { combo: HWND },
    System { combo: HWND, which: SystemKind },
    Text { label: HWND, which: DynText },
    Section { button: HWND },
}

struct Control {
    kind: KindUi,
    code: Option<u8>,
}

struct Element {
    window: HWND,
    x: i32,
    dy: i32,
    width: i32,
    height: i32,
}

struct RowUi {
    page: usize,
    height: i32,
    /// Section that hides the row when it is collapsed.
    section: Option<usize>,
    elements: Vec<Element>,
    star: Option<HWND>,
}

struct PageUi {
    window: HWND,
    scroll: i32,
    content_height: i32,
}

struct SectionUi {
    button: HWND,
    collapsed: bool,
    title: String,
}

pub struct Window {
    pub hwnd: HWND,
    font: HFONT,
    bold: HFONT,
    brush: HBRUSH,
    tooltip: HWND,
    palette: Palette,
    status: HWND,
    nav: HWND,
    pages: Vec<PageUi>,
    rows: Vec<RowUi>,
    controls: Vec<Control>,
    sections: Vec<SectionUi>,
    /// Swatch buttons: window → (control, option).
    swatches: HashMap<isize, (usize, usize)>,
    /// Star buttons: window → favourite id.
    stars: HashMap<isize, &'static str>,
    active: usize,
    applying: bool,
    firmware: usize,
    dpi: u32,
}

impl Window {
    pub fn is_open(&self) -> bool {
        !self.hwnd.is_null()
    }

    pub fn palette(&self) -> &Palette {
        &self.palette
    }
}

// ───────────────────────────── opening ─────────────────────────────

/// Opens the settings window, or brings it to the front when it is already open.
pub fn open(app: &mut App) {
    if let Some(window) = app.window.as_ref()
        && window.is_open()
    {
        unsafe {
            ShowWindow(window.hwnd, SW_RESTORE);
            SetForegroundWindow(window.hwnd);
        }
        return;
    }
    let window = create_shell(app);
    if !window.is_open() {
        return;
    }
    app.window = Some(window);
    rebuild(app);
    let _ = app.worker.send(monitor::WorkerCmd::CheckOsdChanges);
    let _ = app.worker.send(monitor::WorkerCmd::ReadUsageTime);
}

/// Closes the window and frees what belongs to it.
pub fn close(app: &mut App) {
    if let Some(mut window) = app.window.take() {
        release(&mut window);
        if !window.hwnd.is_null() {
            unsafe {
                DestroyWindow(window.hwnd);
            }
        }
        // Back to the tray: give the memory of the window back to the system.
        crate::app::trim_working_set();
    }
}

pub fn is_open(app: &App) -> bool {
    app.window.as_ref().is_some_and(Window::is_open)
}

/// Creates the window itself, without its contents.
fn create_shell(app: &mut App) -> Window {
    let instance = unsafe { GetModuleHandleW(ptr::null()) };
    register_classes(instance);
    let dpi = win::system_dpi();
    crate::log::write(format_args!(
        "[UI] creating the window at {dpi} DPI: {}x{}",
        win::scaled(1060, dpi),
        win::scaled(800, dpi)
    ));
    let dark = app.dark();
    let palette = if dark { DARK } else { LIGHT };
    let font = make_font(dpi, false);
    let bold = make_font(dpi, true);
    let brush = unsafe { CreateSolidBrush(palette.background) };
    let title = win::wide(&app.tr("Titan Control"));

    let hwnd = unsafe {
        CreateWindowExW(
            0,
            win::wide("TitanControlSettings").as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
            0,
            0,
            win::scaled(1060, dpi),
            win::scaled(800, dpi),
            ptr::null_mut(),
            ptr::null_mut(),
            instance,
            ptr::null_mut(),
        )
    };

    let window = Window {
        hwnd,
        font,
        bold,
        brush,
        tooltip: ptr::null_mut(),
        palette,
        status: ptr::null_mut(),
        nav: ptr::null_mut(),
        pages: Vec::new(),
        rows: Vec::new(),
        controls: Vec::new(),
        sections: Vec::new(),
        swatches: HashMap::new(),
        stars: HashMap::new(),
        active: 0,
        applying: false,
        firmware: FirmwarePackage::ALL
            .iter()
            .position(|package| *package == app.settings.firmware_package)
            .unwrap_or(0),
        dpi,
    };
    if hwnd.is_null() {
        crate::log::write(format_args!("[UI] cannot create the settings window: {}", win::last_error()));
        return window;
    }
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, app as *mut App as isize);
        // The DPI of the monitor the window landed on decides the layout, not the DPI of the primary display.
        let dpi = win::dpi_of(hwnd);
        let mut window = window;
        if dpi != window.dpi {
            rescale(&mut window, dpi);
        }
        win::set_dark_frame(hwnd, dark);
        SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, app.icon_large as isize);
        SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, app.icon_small as isize);
        win::center(hwnd, win::scaled(1060, dpi), win::scaled(800, dpi));
        let mut rect: RECT = std::mem::zeroed();
        GetWindowRect(hwnd, &mut rect);
        crate::log::write(format_args!(
            "[UI] window at {},{} {}x{} (dpi {dpi})",
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top
        ));
        return window;
    }
}

fn rebuild(app: &mut App) {
    let Some(mut window) = app.window.take() else {
        return;
    };
    build(app, &mut window);
    app.window = Some(window);
    apply_state(app);
    refresh_availability(app);
    if let Some(window) = app.window.take() {
        unsafe {
            ShowWindow(window.hwnd, SW_SHOW);
            SetForegroundWindow(window.hwnd);
            UpdateWindow(window.hwnd);
        }
        app.window = Some(window);
    }
}

fn make_font(dpi: u32, bold: bool) -> HFONT {
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

fn register_classes(instance: *mut c_void) {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| unsafe {
        let mut controls: INITCOMMONCONTROLSEX = std::mem::zeroed();
        controls.dwSize = std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32;
        controls.dwICC = ICC_BAR_CLASSES | ICC_STANDARD_CLASSES;
        InitCommonControlsEx(&controls);

        let classes: [(&str, WNDPROC); 2] =
            [("TitanControlSettings", Some(settings_proc)), ("TitanPage", Some(page_proc))];
        for (name, proc_) in classes {
            let class_name = win::wide(name);
            let mut class: WNDCLASSEXW = std::mem::zeroed();
            class.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
            class.style = CS_HREDRAW | CS_VREDRAW;
            class.lpfnWndProc = proc_;
            class.hInstance = instance;
            class.hCursor = LoadCursorW(ptr::null_mut(), IDC_ARROW);
            class.hbrBackground = ptr::null_mut();
            class.lpszClassName = class_name.as_ptr();
            if RegisterClassExW(&class) == 0 {
                crate::log::write(format_args!("[UI] RegisterClassExW({name}): {}", win::last_error()));
            }
        }
    });
}

fn build(app: &mut App, window: &mut Window) {
    let hwnd = window.hwnd;
    let lang = app.lang;
    unsafe {
        for (id, text) in [
            (ID_REFRESH, "Odśwież"),
            (ID_PREFERENCES, "Preferencje"),
            (ID_ABOUT, "O programie"),
            (ID_LOG, "Pokaż plik logu"),
        ] {
            let button = CreateWindowExW(
                0,
                win::wide("BUTTON").as_ptr(),
                win::wide(&tr(&lang, text)).as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON,
                0,
                0,
                10,
                10,
                hwnd,
                id as usize as *mut c_void,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            );
            SendMessageW(button, WM_SETFONT, window.font as usize, 1);
        }
        window.status = CreateWindowExW(
            0,
            win::wide("STATIC").as_ptr(),
            win::wide(&tr(&lang, "Łączenie z monitorem…")).as_ptr(),
            WS_CHILD | WS_VISIBLE | SS_LEFT | SS_ENDELLIPSIS,
            0,
            0,
            10,
            10,
            hwnd,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
        );
        SendMessageW(window.status, WM_SETFONT, window.font as usize, 1);

        window.nav = CreateWindowExW(
            0,
            win::wide("LISTBOX").as_ptr(),
            ptr::null(),
            WS_CHILD | WS_VISIBLE | WS_VSCROLL | LBS_NOTIFY | LBS_NOINTEGRALHEIGHT,
            0,
            0,
            10,
            10,
            hwnd,
            ID_NAV as usize as *mut c_void,
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(window.nav, WM_SETFONT, window.font as usize, 1);
        if window.palette.background == DARK.background {
            win::set_control_theme(window.nav, true);
        }

        window.tooltip = CreateWindowExW(
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
        SendMessageW(window.tooltip, WM_SETFONT, window.font as usize, 1);
    }

    let pages = rows::pages();
    for (index, page) in pages.iter().enumerate() {
        unsafe {
            let label = win::wide(&tr(&lang, page.title));
            SendMessageW(window.nav, LB_ADDSTRING, 0, label.as_ptr() as isize);
            let page_window = CreateWindowExW(
                0,
                win::wide("TitanPage").as_ptr(),
                ptr::null(),
                WS_CHILD | WS_VSCROLL | WS_CLIPCHILDREN,
                0,
                0,
                10,
                10,
                hwnd,
                index as usize as *mut c_void,
                GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            );
            window.pages.push(PageUi { window: page_window, scroll: 0, content_height: 0 });
        }
        build_page(app, window, index, page);
    }
    unsafe {
        SendMessageW(window.nav, LB_SETCURSEL, 0, 0);
    }
    layout(window, app);
    // Layout check: `TITAN_UI_PAGE=3` opens the window on another page (used for screenshots).
    let page = std::env::var("TITAN_UI_PAGE").ok().and_then(|index| index.parse::<usize>().ok()).unwrap_or(0);
    let page = page.min(window.pages.len().saturating_sub(1));
    show_page(window, page);
    unsafe {
        SendMessageW(window.nav, LB_SETCURSEL, page, 0);
    }
    theme_children(window);
}

/// Hands the explorer dark theme to the window and to every control, which is what makes the classic controls
/// follow a dark window.
fn theme_children(window: &Window) {
    let dark = window.palette.background == DARK.background;
    unsafe {
        win::set_control_theme(window.hwnd, dark);
        for id in [ID_REFRESH, ID_PREFERENCES, ID_ABOUT, ID_LOG] {
            win::set_control_theme(GetDlgItem(window.hwnd, id as i32), dark);
        }
        win::set_control_theme(window.nav, dark);
        win::set_control_theme(window.status, dark);
        for page in &window.pages {
            win::set_control_theme(page.window, dark);
            let mut child = GetWindow(page.window, GW_CHILD);
            while !child.is_null() {
                win::set_control_theme(child, dark);
                child = GetWindow(child, GW_HWNDNEXT);
            }
        }
    }
}

fn build_page(app: &mut App, window: &mut Window, page: usize, spec: &rows::Page) {
    let lang = app.lang;
    let dpi = window.dpi;
    let scale = |value: i32| win::scaled(value, dpi);
    let page_window = window.pages[page].window;
    let label_width = scale(LABEL_WIDTH);
    let control_x = scale(MARGIN) + label_width + scale(20);
    let star_size = scale(STAR_SIZE);
    let slider_width = scale(SLIDER_WIDTH);
    let value_width = scale(VALUE_WIDTH);
    let combo_width = scale(COMBO_WIDTH);
    let short_combo = scale(SHORT_COMBO_WIDTH);
    let button_width = scale(BUTTON_WIDTH);
    let swatch = scale(SWATCH_SIZE);
    let swatch_gap = scale(SWATCH_GAP);
    let indent = scale(INDENT);
    let mut section: Option<usize> = None;

    for spec_row in &spec.rows {
        let mut elements: Vec<Element> = Vec::new();
        let row_label = tr(&lang, spec_row.label);
        let sub_offset = if spec_row.sub { indent } else { 0 };

        match &spec_row.kind {
            Kind::Header => {
                let label = create_label(window, page_window, &row_label, window.bold);
                elements.push(Element {
                    window: label,
                    x: scale(MARGIN),
                    dy: scale(7),
                    width: label_width + scale(30),
                    height: scale(20),
                });
                let line = create_separator(window, page_window);
                elements.push(Element {
                    window: line,
                    x: scale(MARGIN),
                    dy: scale(HEADER_HEIGHT) - scale(8),
                    width: scale(430),
                    height: 2,
                });
                window.rows.push(RowUi { page, height: scale(HEADER_HEIGHT), section: None, elements, star: None });
                continue;
            }
            Kind::Section => {
                let button = create_section_button(window, page_window, &row_label, window.bold);
                elements.push(Element {
                    window: button,
                    x: scale(MARGIN),
                    dy: scale(4),
                    width: label_width + scale(80),
                    height: scale(24),
                });
                window.sections.push(SectionUi {
                    button,
                    collapsed: false,
                    title: row_label.clone(),
                });
                section = Some(window.sections.len() - 1);
                window.controls.push(Control { kind: KindUi::Section { button }, code: None });
                window.rows.push(RowUi { page, height: scale(SECTION_HEIGHT), section: None, elements, star: None });
                continue;
            }
            _ => {}
        }

        let headline = matches!(
            spec_row.kind,
            Kind::Text(DynText::MonitorTitle) | Kind::Text(DynText::MonitorSubtitle)
        );
        if !headline {
            let label = create_label(window, page_window, &row_label, window.font);
            elements.push(Element {
                window: label,
                x: scale(MARGIN) + sub_offset,
                dy: scale(5),
                width: label_width - sub_offset,
                height: scale(20),
            });
            if let Some(tip) = spec_row.tooltip {
                add_tooltip(window.tooltip, label, &tr(&lang, tip));
            }
        }

        // Star of a favourite row.
        let mut star = None;
        if let Some(id) = spec_row.favorite {
            let button = create_button(window, page_window, "\u{2606}", window.font);
            star = Some(button);
            window.stars.insert(button as isize, id);
            elements.push(Element {
                window: button,
                x: scale(MARGIN) + label_width + scale(2),
                dy: scale(5),
                width: star_size,
                height: star_size,
            });
        }

        let kind = match &spec_row.kind {
            Kind::Text(which) => {
                let value = create_label(window, page_window, "—", window.font);
                elements.push(Element {
                    window: value,
                    x: control_x,
                    dy: scale(5),
                    width: scale(340),
                    height: scale(20),
                });
                KindUi::Text { label: value, which: *which }
            }
            Kind::Slider { max, factor } => {
                let track = create_trackbar(window, page_window, *max);
                elements.push(Element {
                    window: track,
                    x: control_x,
                    dy: scale(3),
                    width: slider_width,
                    height: scale(22),
                });
                let value = create_label(window, page_window, "0", window.font);
                elements.push(Element {
                    window: value,
                    x: control_x + slider_width + scale(8),
                    dy: scale(5),
                    width: value_width,
                    height: scale(20),
                });
                KindUi::Slider { track, value, factor: *factor, max: *max }
            }
            Kind::Switch { on, off } => {
                let button = create_checkbox(window, page_window);
                elements.push(Element {
                    window: button,
                    x: control_x,
                    dy: scale(4),
                    width: scale(22),
                    height: scale(20),
                });
                KindUi::Switch { button, on: *on, off: *off }
            }
            Kind::Choice { values, options } => {
                let combo = create_combo(window, page_window);
                for option in options.iter() {
                    add_combo_string(combo, &tr(&lang, option));
                }
                elements.push(Element {
                    window: combo,
                    x: control_x,
                    dy: scale(2),
                    width: combo_width,
                    height: scale(200),
                });
                KindUi::Choice { combo, values }
            }
            Kind::Swatches { values, options, colors } => {
                let mut buttons = Vec::new();
                for (index, (option, color)) in options.iter().zip(colors.iter()).enumerate() {
                    let button = create_swatch(window, page_window);
                    let _ = color;
                    elements.push(Element {
                        window: button,
                        x: control_x + (index as i32) * (swatch + swatch_gap),
                        dy: scale(2),
                        width: swatch,
                        height: scale(24),
                    });
                    add_tooltip(window.tooltip, button, &tr(&lang, option));
                    buttons.push(button);
                }
                let control_index = window.controls.len();
                for (index, button) in buttons.iter().enumerate() {
                    window.swatches.insert(*button as isize, (control_index, index));
                }
                KindUi::Swatches { buttons, values, colors, selected: usize::MAX }
            }
            Kind::Action(action) => {
                let button = create_button(window, page_window, &row_label, window.font);
                elements.push(Element {
                    window: button,
                    x: control_x,
                    dy: scale(2),
                    width: button_width,
                    height: scale(24),
                });
                KindUi::Action { button, action: *action }
            }
            Kind::PictureMode => {
                let combo = create_combo(window, page_window);
                for name in monitor::PICTURE_MODE_NAMES {
                    add_combo_string(combo, name);
                }
                elements.push(Element {
                    window: combo,
                    x: control_x,
                    dy: scale(2),
                    width: combo_width,
                    height: scale(200),
                });
                KindUi::PictureMode { combo }
            }
            Kind::Variant => {
                let combo = create_combo(window, page_window);
                for option in ["Domyślne", "Własne"] {
                    add_combo_string(combo, &tr(&lang, option));
                }
                elements.push(Element {
                    window: combo,
                    x: control_x,
                    dy: scale(2),
                    width: short_combo,
                    height: scale(200),
                });
                KindUi::Variant { combo }
            }
            Kind::FirmwarePackage => {
                let combo = create_combo(window, page_window);
                for label in ["2024-12-11", "2025-09-20"] {
                    add_combo_string(combo, label);
                }
                elements.push(Element {
                    window: combo,
                    x: control_x,
                    dy: scale(2),
                    width: short_combo,
                    height: scale(200),
                });
                KindUi::Firmware { combo }
            }
            Kind::SystemResolution | Kind::SystemRefresh | Kind::SystemScaling => {
                let which = match spec_row.kind {
                    Kind::SystemResolution => SystemKind::Resolution,
                    Kind::SystemRefresh => SystemKind::Refresh,
                    _ => SystemKind::Scaling,
                };
                let combo = create_combo(window, page_window);
                elements.push(Element {
                    window: combo,
                    x: control_x,
                    dy: scale(2),
                    width: combo_width,
                    height: scale(200),
                });
                KindUi::System { combo, which }
            }
            Kind::Header | Kind::Section => unreachable!(),
        };
        window.controls.push(Control { kind, code: spec_row.code });
        window.rows.push(RowUi { page, height: scale(ROW_HEIGHT), section, elements, star });
    }
}

// ───────────────────────────── widget factories ─────────────────────────────

fn create_label(window: &Window, parent: HWND, text: &str, font: HFONT) -> HWND {
    unsafe {
        let label = CreateWindowExW(
            0,
            win::wide("STATIC").as_ptr(),
            win::wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | SS_LEFT | SS_ENDELLIPSIS,
            0,
            0,
            10,
            10,
            parent,
            ptr::null_mut(),
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(label, WM_SETFONT, font as usize, 1);
        let _ = window;
        label
    }
}

fn create_separator(window: &Window, parent: HWND) -> HWND {
    unsafe {
        let line = CreateWindowExW(
            0,
            win::wide("STATIC").as_ptr(),
            ptr::null(),
            WS_CHILD | WS_VISIBLE | SS_ETCHEDHORZ,
            0,
            0,
            10,
            2,
            parent,
            ptr::null_mut(),
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        let _ = window;
        line
    }
}

fn create_button(window: &Window, parent: HWND, text: &str, font: HFONT) -> HWND {
    unsafe {
        let button = CreateWindowExW(
            0,
            win::wide("BUTTON").as_ptr(),
            win::wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON,
            0,
            0,
            10,
            10,
            parent,
            ptr::null_mut(),
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(button, WM_SETFONT, font as usize, 1);
        let _ = window;
        button
    }
}

fn create_checkbox(window: &Window, parent: HWND) -> HWND {
    unsafe {
        let button = CreateWindowExW(
            0,
            win::wide("BUTTON").as_ptr(),
            ptr::null(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_AUTOCHECKBOX,
            0,
            0,
            10,
            10,
            parent,
            ptr::null_mut(),
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(button, WM_SETFONT, window.font as usize, 1);
        button
    }
}

fn create_swatch(window: &Window, parent: HWND) -> HWND {
    unsafe {
        let button = CreateWindowExW(
            0,
            win::wide("BUTTON").as_ptr(),
            ptr::null(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW,
            0,
            0,
            10,
            10,
            parent,
            ptr::null_mut(),
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(button, WM_SETFONT, window.font as usize, 1);
        button
    }
}

fn create_section_button(window: &Window, parent: HWND, text: &str, font: HFONT) -> HWND {
    unsafe {
        let button = CreateWindowExW(
            0,
            win::wide("BUTTON").as_ptr(),
            win::wide(&format!("\u{25be}   {text}")).as_ptr(),
            // A push-like check box folds the section like the expander rows of the Linux window.
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_AUTOCHECKBOX | BS_PUSHLIKE,
            0,
            0,
            10,
            10,
            parent,
            ptr::null_mut(),
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(button, WM_SETFONT, font as usize, 1);
        SendMessageW(button, BM_SETCHECK, BST_CHECKED as usize, 0);
        let _ = window;
        button
    }
}

fn create_trackbar(window: &Window, parent: HWND, max: i32) -> HWND {
    unsafe {
        let track = CreateWindowExW(
            0,
            win::wide("msctls_trackbar32").as_ptr(),
            ptr::null(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | TBS_HORZ | TBS_NOTICKS,
            0,
            0,
            10,
            10,
            parent,
            ptr::null_mut(),
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(track, WM_SETFONT, window.font as usize, 1);
        SendMessageW(track, TBM_SETRANGE, 1, ((max as isize) << 16) | 0);
        SendMessageW(track, TBM_SETPAGESIZE, 0, 10);
        track
    }
}

fn create_combo(window: &Window, parent: HWND) -> HWND {
    unsafe {
        let combo = CreateWindowExW(
            0,
            win::wide("COMBOBOX").as_ptr(),
            ptr::null(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST | CBS_HASSTRINGS,
            0,
            0,
            10,
            220,
            parent,
            ptr::null_mut(),
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(combo, WM_SETFONT, window.font as usize, 1);
        if window.palette.background == DARK.background {
            win::set_control_theme(combo, true);
        }
        combo
    }
}

fn add_combo_string(combo: HWND, text: &str) {
    let text = win::wide(text);
    unsafe {
        SendMessageW(combo, CB_ADDSTRING, 0, text.as_ptr() as isize);
    }
}

fn add_tooltip(tooltip: HWND, target: HWND, text: &str) {
    if tooltip.is_null() || target.is_null() {
        return;
    }
    use windows_sys::Win32::UI::Controls::{TTF_IDISHWND, TTF_SUBCLASS, TTM_ADDTOOLW, TTTOOLINFOW};
    let text: Box<Vec<u16>> = Box::new(win::wide(text));
    let mut info: TTTOOLINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<TTTOOLINFOW>() as u32;
    info.uFlags = TTF_IDISHWND | TTF_SUBCLASS;
    info.hwnd = target;
    info.uId = target as usize;
    info.lpszText = Box::into_raw(text) as *mut u16;
    unsafe {
        SendMessageW(tooltip, TTM_ADDTOOLW, 0, &info as *const TTTOOLINFOW as isize);
    }
}

// ───────────────────────────── layout ─────────────────────────────

fn layout(window: &mut Window, app: &mut App) {
    let hwnd = window.hwnd;
    let dpi = window.dpi;
    let mut client: RECT = unsafe { std::mem::zeroed() };
    unsafe {
        GetClientRect(hwnd, &mut client);
    }
    let width = client.right;
    let height = client.bottom;
    let toolbar = win::scaled(TOOLBAR_HEIGHT, dpi);
    let status = win::scaled(STATUS_HEIGHT, dpi);
    let nav = win::scaled(NAV_WIDTH, dpi);
    unsafe {
        let mut x = win::scaled(MARGIN, dpi);
        for id in [ID_REFRESH, ID_PREFERENCES, ID_ABOUT, ID_LOG] {
            let button = GetDlgItem(hwnd, id as i32);
            let button_width = win::scaled(120, dpi);
            MoveWindow(button, x, win::scaled(8, dpi), button_width, win::scaled(26, dpi), 1);
            x += button_width + win::scaled(6, dpi);
        }
        MoveWindow(
            window.status,
            win::scaled(MARGIN, dpi),
            (height - status + win::scaled(4, dpi)).max(0),
            (width - win::scaled(2 * MARGIN, dpi)).max(0),
            win::scaled(18, dpi),
            1,
        );
        MoveWindow(window.nav, 0, toolbar, nav, (height - toolbar - status).max(0), 1);
        for page in &window.pages {
            MoveWindow(page.window, nav, toolbar, (width - nav).max(0), (height - toolbar - status).max(0), 1);
        }
    }
    reflow(window, app);
}

fn reflow(window: &mut Window, _app: &mut App) {
    let dpi = window.dpi;
    let margin = win::scaled(MARGIN, dpi);
    let mut client: RECT = unsafe { std::mem::zeroed() };

    for page_index in 0..window.pages.len() {
        let page_window = window.pages[page_index].window;
        unsafe {
            GetClientRect(page_window, &mut client);
        }
        let viewport = client.bottom.max(0);
        let width = client.right.max(0);
        let mut y = margin;
        let scroll = window.pages[page_index].scroll;
        let sections = &window.sections;
        let rows = &window.rows;

        for row in rows.iter().filter(|row| row.page == page_index) {
            let hidden = row.section.is_some_and(|section| sections.get(section).is_some_and(|s| s.collapsed));
            for element in &row.elements {
                if hidden {
                    unsafe {
                        ShowWindow(element.window, SW_HIDE);
                    }
                    continue;
                }
                let mut element_width = element.width;
                if element.x + element_width > width - margin {
                    element_width = (width - margin - element.x).max(win::scaled(40, dpi));
                }
                unsafe {
                    SetWindowPos(
                        element.window,
                        ptr::null_mut(),
                        element.x,
                        y + element.dy - scroll,
                        element_width,
                        element.height,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                    ShowWindow(element.window, SW_SHOW);
                }
            }
            if let Some(star) = row.star {
                unsafe {
                    ShowWindow(star, if hidden { SW_HIDE } else { SW_SHOW });
                }
            }
            if !hidden {
                y += row.height;
            }
        }
        let content = y + margin;
        window.pages[page_index].content_height = content;
        let max_scroll = (content - viewport).max(0);
        let scroll = window.pages[page_index].scroll.clamp(0, max_scroll);
        window.pages[page_index].scroll = scroll;
        unsafe {
            let mut info: SCROLLINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<SCROLLINFO>() as u32;
            info.fMask = SIF_RANGE | SIF_PAGE | SIF_POS;
            info.nMin = 0;
            info.nMax = content;
            info.nPage = viewport as u32;
            info.nPos = scroll;
            SetScrollInfo(page_window, SB_VERT, &info, 1);
        }
    }
}

fn show_page(window: &mut Window, index: usize) {
    for (position, page) in window.pages.iter().enumerate() {
        unsafe {
            ShowWindow(page.window, if position == index { SW_SHOW } else { SW_HIDE });
        }
    }
    window.active = index;
}

fn scroll_page(window: &mut Window, page_window: HWND, request: i32) {
    let Some(index) = window.pages.iter().position(|page| page.window == page_window) else {
        return;
    };
    let mut client: RECT = unsafe { std::mem::zeroed() };
    unsafe {
        GetClientRect(page_window, &mut client);
    }
    let max = (window.pages[index].content_height - client.bottom).max(0);
    window.pages[index].scroll = request.clamp(0, max);
    unsafe {
        SetScrollPos(page_window, SB_VERT, window.pages[index].scroll, 1);
    }
}

// ───────────────────────────── state ─────────────────────────────

pub fn apply_state(app: &mut App) {
    let Some(mut window) = app.window.take() else {
        return;
    };
    if !window.is_open() {
        app.window = Some(window);
        return;
    }
    window.applying = true;
    let values = app.state.clone();
    let info = app.info.clone();
    let language = app.lang;
    let firmware = window.firmware;
    let favorites: Vec<(&'static str, bool)> = window
        .stars
        .values()
        .map(|id| (*id, app.settings.is_favorite_tray_control(id)))
        .collect();

    for control in &mut window.controls {
        match &mut control.kind {
            KindUi::Slider { track, value, factor, max } => {
                let Some(current) = control.code.and_then(|code| values.get(&code).copied()) else {
                    continue;
                };
                let position = (current / (*factor).max(1)).min(*max as u16) as isize;
                unsafe {
                    SendMessageW(*track, TBM_SETPOS, 1, position);
                    SetWindowTextW(*value, win::wide(&current.to_string()).as_ptr());
                }
            }
            KindUi::Switch { button, on, .. } => {
                let Some(current) = control.code.and_then(|code| values.get(&code).copied()) else {
                    continue;
                };
                let state = if current == *on { BST_CHECKED } else { BST_UNCHECKED };
                unsafe {
                    SendMessageW(*button, BM_SETCHECK, state as usize, 0);
                }
            }
            KindUi::Choice { combo, values: options } => {
                let Some(current) = control.code.and_then(|code| values.get(&code).copied()) else {
                    continue;
                };
                if let Some(index) = options.iter().position(|candidate| *candidate == current) {
                    unsafe {
                        SendMessageW(*combo, CB_SETCURSEL, index, 0);
                    }
                }
            }
            KindUi::Swatches { buttons, values: options, selected, .. } => {
                let Some(current) = control.code.and_then(|code| values.get(&code).copied()) else {
                    continue;
                };
                if let Some(index) = options.iter().position(|candidate| *candidate == current) {
                    *selected = index;
                    for button in buttons.iter() {
                        unsafe {
                            InvalidateRect(*button, ptr::null(), 0);
                        }
                    }
                }
            }
            KindUi::PictureMode { combo } => {
                if let Some(current) = values.get(&monitor::VCP_MODE).copied()
                    && let Some((mode, _)) = monitor::picture_mode_from_vcp(current)
                {
                    unsafe {
                        SendMessageW(*combo, CB_SETCURSEL, mode, 0);
                    }
                }
            }
            KindUi::Variant { combo } => {
                if let Some(current) = values.get(&monitor::VCP_MODE).copied()
                    && let Some((_, custom)) = monitor::picture_mode_from_vcp(current)
                {
                    unsafe {
                        SendMessageW(*combo, CB_SETCURSEL, custom as usize, 0);
                    }
                }
            }
            KindUi::Firmware { combo } => unsafe {
                SendMessageW(*combo, CB_SETCURSEL, firmware, 0);
            },
            KindUi::Text { label, which } => {
                let text = match which {
                    DynText::MonitorTitle => info.as_ref().map(|info| info.name.clone()).unwrap_or_default(),
                    DynText::MonitorSubtitle => info
                        .as_ref()
                        .map(|info| {
                            format!(
                                "{} \u{00b7} {} Hz \u{00b7} {}",
                                info.resolution.replace('x', "\u{00d7}"),
                                info.hz,
                                usage_minutes(info.usage_mins)
                            )
                        })
                        .unwrap_or_default(),
                    DynText::Model => info.as_ref().map(|info| info.name.clone()).unwrap_or_default(),
                    DynText::Usage => info
                        .as_ref()
                        .map(|info| usage_minutes(info.usage_mins))
                        .unwrap_or_default(),
                };
                unsafe {
                    SetWindowTextW(*label, win::wide(&text).as_ptr());
                }
            }
            KindUi::Section { .. } | KindUi::Action { .. } | KindUi::System { .. } => {}
        }
    }
    for (id, active) in favorites {
        if let Some(button) = window
            .stars
            .iter()
            .find(|(_, favourite)| **favourite == id)
            .map(|(button, _)| *button as HWND)
        {
            unsafe {
                SetWindowTextW(button, win::wide(if active { "\u{2605}" } else { "\u{2606}" }).as_ptr());
            }
        }
    }
    apply_system_combos(&mut window, app);
    update_status(&mut window, app, &language);
    window.applying = false;
    app.window = Some(window);
}

fn update_status(window: &mut Window, app: &App, lang: &AppLang) {
    let text = match app.info.as_ref() {
        Some(info) => format!(
            "{} \u{00b7} {} \u{00b7} {} Hz",
            info.name,
            info.resolution.replace('x', "\u{00d7}"),
            info.hz
        ),
        None => tr(lang, "Szukam monitora..."),
    };
    unsafe {
        SetWindowTextW(window.status, win::wide(&text).as_ptr());
    }
}

fn apply_system_combos(window: &mut Window, app: &App) {
    for control in &window.controls {
        let KindUi::System { combo, which } = &control.kind else {
            continue;
        };
        unsafe {
            SendMessageW(*combo, CB_RESETCONTENT, 0, 0);
        }
        match which {
            SystemKind::Resolution => {
                let mut seen: Vec<(u32, u32)> = Vec::new();
                for mode in &app.modes {
                    let pair = (mode.width, mode.height);
                    if seen.contains(&pair) {
                        continue;
                    }
                    add_combo_string(*combo, &format!("{} \u{00d7} {}", mode.width, mode.height));
                    seen.push(pair);
                }
                if let Some(current) = app.current_mode.as_ref()
                    && let Some(index) = seen.iter().position(|pair| *pair == (current.width, current.height))
                {
                    unsafe {
                        SendMessageW(*combo, CB_SETCURSEL, index, 0);
                    }
                }
            }
            SystemKind::Refresh => {
                let mut rates: Vec<u32> = Vec::new();
                if let Some(current) = app.current_mode.as_ref() {
                    for mode in &app.modes {
                        if mode.width == current.width && mode.height == current.height && !rates.contains(&mode.refresh)
                        {
                            rates.push(mode.refresh);
                        }
                    }
                }
                rates.sort_unstable();
                for rate in &rates {
                    add_combo_string(*combo, &format!("{rate} Hz"));
                }
                if let Some(current) = app.current_mode.as_ref()
                    && let Some(index) = rates.iter().position(|rate| *rate == current.refresh)
                {
                    unsafe {
                        SendMessageW(*combo, CB_SETCURSEL, index, 0);
                    }
                }
            }
            SystemKind::Scaling => {
                let scale = win::system_dpi() * 100 / 96;
                add_combo_string(*combo, &format!("{scale}%"));
                unsafe {
                    SendMessageW(*combo, CB_SETCURSEL, 0, 0);
                }
            }
        }
        unsafe {
            EnableWindow(*combo, if matches!(which, SystemKind::Scaling) { 0 } else { 1 });
        }
    }
}

/// Re-evaluates which controls the current monitor state allows.
pub fn refresh_availability(app: &mut App) {
    let Some(window) = app.window.take() else {
        return;
    };
    if !window.is_open() {
        app.window = Some(window);
        return;
    }
    let allowed = app.availability();
    let custom = app.get(monitor::VCP_MODE).is_none_or(|mode| monitor::is_custom_profile(mode as u8));
    let color_temp_user = app.get(monitor::VCP_COLOR_TEMP).is_some_and(monitor::is_user_color_temp);
    let connected = app.connected;
    let loading = app.profile_loading;

    for control in &window.controls {
        let enabled = match &control.kind {
            KindUi::Action { .. } => connected,
            KindUi::PictureMode { .. } | KindUi::Variant { .. } => connected && !loading,
            KindUi::Firmware { .. } => true,
            KindUi::System { which, .. } => !matches!(which, SystemKind::Scaling),
            KindUi::Section { .. } => true,
            _ => {
                let Some(code) = control.code else { continue };
                let mut ok = connected && allowed.get(&code).copied().unwrap_or(true);
                if matches!(code, monitor::VCP_RED | monitor::VCP_GREEN | monitor::VCP_BLUE) {
                    ok &= custom && color_temp_user && !loading;
                }
                ok
            }
        };
        for hwnd in control_windows(&control.kind) {
            unsafe {
                EnableWindow(hwnd, if enabled { 1 } else { 0 });
            }
        }
    }
    app.window = Some(window);
}

fn control_windows(kind: &KindUi) -> Vec<HWND> {
    match kind {
        KindUi::Slider { track, .. } => vec![*track],
        KindUi::Switch { button, .. } => vec![*button],
        KindUi::Choice { combo, .. } => vec![*combo],
        KindUi::Swatches { buttons, .. } => buttons.clone(),
        KindUi::Action { button, .. } => vec![*button],
        KindUi::PictureMode { combo } | KindUi::Variant { combo } | KindUi::Firmware { combo } => vec![*combo],
        KindUi::System { combo, .. } => vec![*combo],
        KindUi::Text { .. } | KindUi::Section { .. } => Vec::new(),
    }
}

// ───────────────────────────── input ─────────────────────────────

/// Handles a notification of one control.
pub fn handle_command(app: &mut App, item: HWND, notification: u32) {
    if app.window.is_none() || !is_open(app) {
        return;
    }
    // Toolbar buttons and the page list belong to the settings window itself.
    if let Some(mut window) = app.window.take() {
        if item == window.nav {
            if notification == LBN_SELCHANGE {
                let index = unsafe { SendMessageW(window.nav, LB_GETCURSEL, 0, 0) } as usize;
                if index < window.pages.len() {
                    show_page(&mut window, index);
                    reflow(&mut window, app);
                }
            }
            app.window = Some(window);
            return;
        }
        let id = unsafe { GetDlgCtrlID(item) } as u16;
        match id {
            ID_REFRESH => {
                app.window = Some(window);
                app.start_profile_load();
                let _ = app.worker.send(monitor::WorkerCmd::RefreshAll);
                return;
            }
            ID_PREFERENCES => {
                app.window = Some(window);
                prefs::open(app);
                return;
            }
            ID_ABOUT => {
                app.window = Some(window);
                prefs::about(app);
                return;
            }
            ID_LOG => {
                app.window = Some(window);
                crate::settings::open_log_folder();
                return;
            }
            _ => {}
        }

        // Stars.
        if let Some(id) = window.stars.get(&(item as isize)).copied() {
            let active = app.settings.is_favorite_tray_control(id);
            if !active && app.settings.favorite_tray_control_set().len() >= 5 {
                app.window = Some(window);
                return;
            }
            app.settings.favorite_tray_controls.retain(|favorite| favorite != id);
            if !active {
                app.settings.favorite_tray_controls.push(id.to_string());
            }
            app.save_settings();
            unsafe {
                SetWindowTextW(item, win::wide(if active { "\u{2606}" } else { "\u{2605}" }).as_ptr());
            }
            app.window = Some(window);
            crate::tray::refresh(app);
            return;
        }

        // Swatches: the clicked button names the value.
        if let Some((control, option)) = window.swatches.get(&(item as isize)).copied() {
            let value = match &window.controls[control].kind {
                KindUi::Swatches { values, .. } => values.get(option).copied(),
                _ => None,
            };
            let code = window.controls[control].code;
            if let (Some(value), Some(code)) = (value, code) {
                if let KindUi::Swatches { selected, buttons, .. } = &mut window.controls[control].kind {
                    *selected = option;
                    for button in buttons.iter() {
                        unsafe {
                            InvalidateRect(*button, ptr::null(), 0);
                        }
                    }
                }
                app.window = Some(window);
                app.set(code, value);
                apply_state(app);
                return;
            }
            app.window = Some(window);
            return;
        }

        let Some(index) = window
            .controls
            .iter()
            .position(|control| control_windows(&control.kind).contains(&item))
        else {
            app.window = Some(window);
            return;
        };

        // Collapsible sections are handled here: they only change the layout.
        if let KindUi::Section { button } = window.controls[index].kind {
            let collapsed = unsafe { SendMessageW(button, BM_GETCHECK, 0, 0) } != BST_CHECKED as isize;
            if let Some(section) = window.sections.iter().position(|section| section.button == button) {
                window.sections[section].collapsed = collapsed;
                let title = window.sections[section].title.clone();
                unsafe {
                    SetWindowTextW(
                        button,
                        win::wide(&format!("{}   {title}", if collapsed { '\u{25b8}' } else { '\u{25be}' }))
                            .as_ptr(),
                    );
                }
            }
            reflow(&mut window, app);
            app.window = Some(window);
            return;
        }

        let action = read_control(&window, index, notification);
        app.window = Some(window);
        match action {
            Some(ControlChange::Value(code, value)) => {
                app.set(code, value);
                app.show_saturation();
                apply_state(app);
                refresh_availability(app);
            }
            Some(ControlChange::Action(action)) => run_action(app, action),
            Some(ControlChange::System(which, selection)) => change_system_mode(app, which, selection),
            Some(ControlChange::Firmware(selection)) => change_firmware(app, selection),
            Some(ControlChange::Profile(custom, mode)) => {
                if let Some(value) = monitor::picture_mode_vcp_value(mode, custom) {
                    app.start_profile_load();
                    app.set(monitor::VCP_MODE, value);
                }
            }
            None => {}
        }
        return;
    }
}

enum ControlChange {
    Value(u8, u16),
    Action(Action),
    System(SystemKind, usize),
    Firmware(usize),
    Profile(bool, usize),
}

fn read_control(window: &Window, index: usize, notification: u32) -> Option<ControlChange> {
    let control = &window.controls[index];
    let code = control.code;
    match &control.kind {
        KindUi::Slider { track, factor, max, .. } => {
            let position = unsafe { SendMessageW(*track, TBM_GETPOS, 0, 0) } as i32;
            let value = (position.clamp(0, *max) as u16) * (*factor).max(1);
            code.map(|code| ControlChange::Value(code, value))
        }
        KindUi::Switch { button, on, off } => {
            let checked = unsafe { SendMessageW(*button, BM_GETCHECK, 0, 0) } == BST_CHECKED as isize;
            code.map(|code| ControlChange::Value(code, if checked { *on } else { *off }))
        }
        KindUi::Choice { combo, values } => {
            if notification != CBN_SELCHANGE as u32 {
                return None;
            }
            let selection = unsafe { SendMessageW(*combo, CB_GETCURSEL, 0, 0) } as usize;
            let value = values.get(selection).copied()?;
            code.map(|code| ControlChange::Value(code, value))
        }
        KindUi::Action { action, .. } => Some(ControlChange::Action(*action)),
        KindUi::PictureMode { combo } => {
            if notification != CBN_SELCHANGE as u32 {
                return None;
            }
            let mode = unsafe { SendMessageW(*combo, CB_GETCURSEL, 0, 0) } as usize;
            let custom = window
                .controls
                .iter()
                .find_map(|control| match &control.kind {
                    KindUi::Variant { combo } => {
                        let selection = unsafe { SendMessageW(*combo, CB_GETCURSEL, 0, 0) };
                        Some(selection as usize == 1)
                    }
                    _ => None,
                })
                .unwrap_or(false);
            Some(ControlChange::Profile(custom, mode))
        }
        KindUi::Variant { combo } => {
            if notification != CBN_SELCHANGE as u32 {
                return None;
            }
            let custom = unsafe { SendMessageW(*combo, CB_GETCURSEL, 0, 0) } as usize == 1;
            let mode = window.controls.iter().find_map(|control| match &control.kind {
                KindUi::PictureMode { combo } => {
                    let selection = unsafe { SendMessageW(*combo, CB_GETCURSEL, 0, 0) };
                    Some(selection as usize)
                }
                _ => None,
            })?;
            Some(ControlChange::Profile(custom, mode))
        }
        KindUi::Firmware { combo } => {
            if notification != CBN_SELCHANGE as u32 {
                return None;
            }
            let selection = unsafe { SendMessageW(*combo, CB_GETCURSEL, 0, 0) } as usize;
            Some(ControlChange::Firmware(selection))
        }
        KindUi::System { combo, which } => {
            if notification != CBN_SELCHANGE as u32 {
                return None;
            }
            let selection = unsafe { SendMessageW(*combo, CB_GETCURSEL, 0, 0) } as usize;
            Some(ControlChange::System(*which, selection))
        }
        KindUi::Swatches { .. } | KindUi::Text { .. } | KindUi::Section { .. } => None,
    }
}

fn run_action(app: &mut App, action: Action) {
    let (title, body, code, value) = match action {
        Action::PowerOff => (
            "Wyłączyć monitor?",
            "Włączysz go ponownie tylko przyciskiem zasilania na monitorze.",
            monitor::VCP_DPMS,
            monitor::values::POWER_OFF,
        ),
        Action::ResetColors => (
            "Zresetować kolory?",
            "Kolory wrócą do wartości fabrycznych (temperatura barwowa: Warm).",
            monitor::VCP_RESET_COLOR,
            1,
        ),
        Action::ResetBrightnessContrast => (
            "Zresetować jasność i kontrast?",
            "Jasność i kontrast wrócą do wartości fabrycznych (90 / 50).",
            monitor::VCP_RESET_BC,
            1,
        ),
        Action::ResetFactory => (
            "Przywrócić ustawienia fabryczne?",
            "Wszystkie ustawienia monitora wrócą do wartości fabrycznych.",
            monitor::VCP_RESET_FACTORY,
            1,
        ),
        Action::PipSwap => {
            app.set(monitor::VCP_PIP_SWAP, 1);
            return;
        }
        Action::PipReset => {
            app.set(monitor::VCP_PIP_RESET, 1);
            return;
        }
    };
    if confirm_dialog(app, title, body) {
        app.set(code, value);
    }
}

fn confirm_dialog(app: &App, title: &str, body: &str) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MB_DEFBUTTON2, MB_ICONQUESTION, MB_YESNO};
    let message = format!("{}\n\n{}", app.tr(title), app.tr(body));
    unsafe {
        MessageBoxW(
            app.dialog_owner(),
            win::wide(&message).as_ptr(),
            win::wide(&app.tr("Titan Control")).as_ptr(),
            MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2,
        ) == IDYES
    }
}

pub fn message_box(app: &App, title: &str, text: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONWARNING, MB_OK};
    unsafe {
        MessageBoxW(
            app.dialog_owner(),
            win::wide(text).as_ptr(),
            win::wide(&app.tr(title)).as_ptr(),
            MB_OK | MB_ICONWARNING,
        );
    }
}

fn change_system_mode(app: &mut App, which: SystemKind, selection: usize) {
    match which {
        SystemKind::Resolution => {
            let mut pairs: Vec<(u32, u32)> = Vec::new();
            for mode in &app.modes {
                let pair = (mode.width, mode.height);
                if !pairs.contains(&pair) {
                    pairs.push(pair);
                }
            }
            if let Some(&(width, height)) = pairs.get(selection)
                && let Some(mode) = app
                    .modes
                    .iter()
                    .find(|mode| mode.width == width && mode.height == height)
                    .cloned()
            {
                crate::app::apply_display_mode(app, mode);
            }
        }
        SystemKind::Refresh => {
            let Some(current) = app.current_mode.clone() else {
                return;
            };
            let mut rates: Vec<u32> = app
                .modes
                .iter()
                .filter(|mode| mode.width == current.width && mode.height == current.height)
                .map(|mode| mode.refresh)
                .collect();
            rates.sort_unstable();
            rates.dedup();
            if let Some(rate) = rates.get(selection) {
                let mut mode = current;
                mode.refresh = *rate;
                crate::app::apply_display_mode(app, mode);
            }
        }
        SystemKind::Scaling => {}
    }
}

fn change_firmware(app: &mut App, selection: usize) {
    let Some(&package) = FirmwarePackage::ALL.get(selection) else {
        return;
    };
    app.settings.firmware_package = package;
    app.save_settings();
    let _ = app.worker.send(monitor::WorkerCmd::DydsWithAdaptiveSync(package.dyds_with_adaptive_sync()));
    if let Some(mut window) = app.window.take() {
        window.firmware = selection;
        app.window = Some(window);
    }
    show_firmware(app);
    refresh_availability(app);
}

fn show_firmware(app: &mut App) {
    let Some(window) = app.window.take() else {
        return;
    };
    for control in &window.controls {
        if let KindUi::Firmware { combo } = &control.kind {
            unsafe {
                SendMessageW(*combo, CB_SETCURSEL, window.firmware, 0);
            }
        }
    }
    app.window = Some(window);
}

// ───────────────────────────── drawing ─────────────────────────────

fn draw_swatch(window: &Window, item: &DRAWITEMSTRUCT) {
    if item.CtlType != ODT_BUTTON {
        return;
    }
    let Some((control, option)) = window.swatches.get(&(item.hwndItem as isize)).copied() else {
        return;
    };
    let KindUi::Swatches { colors, selected, .. } = &window.controls[control].kind else {
        return;
    };
    let Some(&color) = colors.get(option) else {
        return;
    };
    unsafe {
        let background = CreateSolidBrush(color);
        FillRect(item.hDC, &item.rcItem, background);
        DeleteObject(background as _);

        let active = *selected == option;
        let pen_color = if active { 0x0020_7AFF } else { window.palette.line };
        let pen = CreatePen(0, if active { win::scaled(3, 0) } else { 1 }, pen_color);
        let previous_pen = SelectObject(item.hDC, pen as _);
        let previous_brush = SelectObject(item.hDC, GetStockObject(NULL_BRUSH) as _);
        Rectangle(
            item.hDC,
            item.rcItem.left + 1,
            item.rcItem.top + 1,
            item.rcItem.right - 1,
            item.rcItem.bottom - 1,
        );
        SelectObject(item.hDC, previous_pen);
        SelectObject(item.hDC, previous_brush);
        DeleteObject(pen as _);
    }
}

// ───────────────────────────── window procedures ─────────────────────────────

pub unsafe extern "system" fn settings_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut App };
    if raw.is_null() {
        return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
    }
    let app = unsafe { &mut *raw };
    match message {
        WM_SIZE => {
            if let Some(mut window) = app.window.take() {
                if window.is_open() {
                    layout(&mut window, app);
                }
                app.window = Some(window);
            }
            0
        }
        WM_DPICHANGED => {
            let dpi = win::dpi_of(hwnd);
            let suggested = unsafe { &*(lparam as *const RECT) };
            crate::log::write(format_args!(
                "[UI] WM_DPICHANGED wparam {} dpi {dpi} suggested {},{} {}x{}",
                wparam,
                suggested.left,
                suggested.top,
                suggested.right - suggested.left,
                suggested.bottom - suggested.top
            ));
            let width = (suggested.right - suggested.left).max(win::scaled(980, dpi));
            let height = (suggested.bottom - suggested.top).max(win::scaled(560, dpi));
            unsafe {
                SetWindowPos(
                    hwnd,
                    ptr::null_mut(),
                    suggested.left,
                    suggested.top,
                    width,
                    height,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
            if let Some(mut window) = app.window.take() {
                if window.is_open() && dpi != 0 && dpi != window.dpi {
                    rescale(&mut window, dpi);
                    layout(&mut window, app);
                }
                app.window = Some(window);
            }
            0
        }
        WM_GETMINMAXINFO => {
            let info = unsafe { &mut *(lparam as *mut MINMAXINFO) };
            let dpi = win::system_dpi();
            info.ptMinTrackSize.x = win::scaled(780, dpi);
            info.ptMinTrackSize.y = win::scaled(480, dpi);
            0
        }
        WM_COMMAND => {
            handle_command(app, lparam as HWND, ((wparam >> 16) & 0xFFFF) as u32);
            0
        }
        WM_HSCROLL => {
            let track = lparam as HWND;
            if !track.is_null() {
                handle_command(app, track, ((wparam as u32) & 0xFFFF).max(1));
            } else {
                crate::app::handle_page_scroll(app, hwnd, wparam);
            }
            0
        }
        WM_VSCROLL => {
            crate::app::handle_page_scroll(app, hwnd, wparam);
            0
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN | WM_CTLCOLORLISTBOX => match app.window.as_ref() {
            Some(window) => unsafe {
                SetTextColor(wparam as _, window.palette().text);
                SetBkColor(wparam as _, window.palette().background);
                SetBkMode(wparam as _, 1);
                window.brush as LRESULT
            },
            None => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
        },
        WM_ERASEBKGND => match app.window.as_ref() {
            Some(window) => unsafe { fill_background(hwnd, wparam as _, window.brush) },
            None => 1,
        },
        WM_CLOSE => {
            close(app);
            0
        }
        WM_TIMER => {
            crate::app::handle_timer(app, wparam as usize);
            0
        }
        crate::WM_UI_COMMAND => {
            crate::app::drain_messages(app);
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

pub unsafe extern "system" fn page_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let raw = unsafe { app_of(hwnd) };
    if raw.is_null() {
        return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
    }
    let app = unsafe { &mut *raw };
    match message {
        WM_COMMAND => {
            handle_command(app, lparam as HWND, ((wparam >> 16) & 0xFFFF) as u32);
            0
        }
        WM_HSCROLL => {
            let track = lparam as HWND;
            if !track.is_null() {
                handle_command(app, track, ((wparam as u32) & 0xFFFF).max(1));
            } else {
                crate::app::handle_page_scroll(app, hwnd, wparam);
            }
            0
        }
        WM_VSCROLL => {
            crate::app::handle_page_scroll(app, hwnd, wparam);
            0
        }
        WM_MOUSEWHEEL => {
            let delta = ((wparam >> 16) & 0xFFFF) as u16 as i16 as i32;
            crate::app::handle_wheel(app, hwnd, delta);
            0
        }
        WM_DRAWITEM => {
            if let Some(window) = app.window.as_ref() {
                let item = unsafe { &*(lparam as *const DRAWITEMSTRUCT) };
                draw_swatch(window, item);
            }
            1
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN | WM_CTLCOLORLISTBOX => match app.window.as_ref() {
            Some(window) => unsafe {
                SetTextColor(wparam as _, window.palette().text);
                SetBkColor(wparam as _, window.palette().background);
                SetBkMode(wparam as _, 1);
                window.brush as LRESULT
            },
            None => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
        },
        WM_ERASEBKGND => match app.window.as_ref() {
            Some(window) => unsafe { fill_background(hwnd, wparam as _, window.brush) },
            None => 1,
        },
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

unsafe fn app_of(window: HWND) -> *mut App {
    let root = unsafe { GetAncestor(window, GA_ROOT) };
    if root.is_null() {
        return ptr::null_mut();
    }
    unsafe { GetWindowLongPtrW(root, GWLP_USERDATA) as *mut App }
}

unsafe fn fill_background(hwnd: HWND, dc: *mut c_void, brush: HBRUSH) -> LRESULT {
    let mut rect: RECT = unsafe { std::mem::zeroed() };
    unsafe {
        GetClientRect(hwnd, &mut rect);
        FillRect(dc, &rect, brush);
    }
    1
}

/// Rebuilds the fonts for a new monitor DPI and hands them to every control of the window.
fn rescale(window: &mut Window, dpi: u32) {
    unsafe {
        if !window.font.is_null() {
            DeleteObject(window.font as _);
        }
        if !window.bold.is_null() {
            DeleteObject(window.bold as _);
        }
    }
    window.font = make_font(dpi, false);
    window.bold = make_font(dpi, true);
    window.dpi = dpi;
    unsafe {
        for row in &window.rows {
            for element in &row.elements {
                let font = if row.height > win::scaled(ROW_HEIGHT, dpi) { window.bold } else { window.font };
                SendMessageW(element.window, WM_SETFONT, font as usize, 1);
            }
            if let Some(star) = row.star {
                SendMessageW(star, WM_SETFONT, window.font as usize, 1);
            }
        }
        SendMessageW(window.nav, WM_SETFONT, window.font as usize, 1);
        SendMessageW(window.status, WM_SETFONT, window.font as usize, 1);
        for id in [ID_REFRESH, ID_PREFERENCES, ID_ABOUT, ID_LOG] {
            let button = GetDlgItem(window.hwnd, id as i32);
            SendMessageW(button, WM_SETFONT, window.font as usize, 1);
        }
    }
}


/// Scrolls a page by `request` (absolute position) and redraws it.
pub fn set_scroll(app: &mut App, page_window: HWND, request: i32) {
    if let Some(mut window) = app.window.take() {
        if window.is_open() {
            scroll_page(&mut window, page_window, request);
            reflow(&mut window, app);
        }
        app.window = Some(window);
    }
}

/// Current scroll position of a page.
pub fn scroll_of(app: &App, page_window: HWND) -> i32 {
    app.window
        .as_ref()
        .and_then(|window| window.pages.iter().find(|page| page.window == page_window).map(|page| page.scroll))
        .unwrap_or(0)
}

/// Destroys everything the window allocated outside of its child windows.
fn release(window: &mut Window) {
    unsafe {
        if !window.font.is_null() {
            DeleteObject(window.font as _);
            window.font = ptr::null_mut();
        }
        if !window.bold.is_null() {
            DeleteObject(window.bold as _);
            window.bold = ptr::null_mut();
        }
        if !window.brush.is_null() {
            DeleteObject(window.brush as _);
            window.brush = ptr::null_mut();
        }
    }
    let _ = (MoveToEx, NULL_BRUSH);
}
