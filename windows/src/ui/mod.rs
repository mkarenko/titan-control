//! The settings window.
//!
//! The window is built from the declarative pages in [`rows`], created when the user opens it and destroyed when it is
//! closed, so none of its memory is kept while the application sits in the tray. Controls are Win32 children rendered
//! with a modern card-based layout, owner-drawn pill switches, segmented toggle groups, and double-buffered pages.

use std::{cell::RefCell, collections::HashMap, ffi::c_void, ptr, time::Instant};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreatePen, CreateSolidBrush,
    DeleteDC, DeleteObject, DrawTextW, Ellipse, EndPaint, FillRect, GetStockObject, InvalidateRect, LineTo, MoveToEx,
    RoundRect, SelectObject, SetBkColor, SetBkMode, SetTextColor, UpdateWindow, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT,
    DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, HBRUSH, HDC, HFONT, NULL_BRUSH, PAINTSTRUCT, PS_SOLID, SRCCOPY,
    TRANSPARENT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::{
    DRAWITEMSTRUCT, ICC_BAR_CLASSES, ICC_STANDARD_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx,
    MEASUREITEMSTRUCT, ODS_DISABLED, ODS_SELECTED, ODT_BUTTON, ODT_LISTBOX, SetScrollInfo, TBM_SETPAGESIZE,
    TBM_SETPOS, TBM_SETRANGE, TBS_HORZ, TBS_NOTICKS, TTS_ALWAYSTIP, TTS_NOPREFIX,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[allow(non_upper_case_globals)]
const SS_LEFT: u32 = 0x0000;
#[allow(non_upper_case_globals)]
const SS_CENTER: u32 = 0x0001;
#[allow(non_upper_case_globals)]
const SS_NOTIFY: u32 = 0x0100;
#[allow(non_upper_case_globals)]
const SS_ENDELLIPSIS: u32 = 0x4000;
#[allow(non_upper_case_globals)]
const BS_OWNERDRAW: u32 = 0x000B;
#[allow(non_upper_case_globals)]
const LBS_NOTIFY: u32 = 0x0001;
#[allow(non_upper_case_globals)]
const LBS_OWNERDRAWFIXED: u32 = 0x0010;
#[allow(non_upper_case_globals)]
const LBS_HASSTRINGS: u32 = 0x0040;
#[allow(non_upper_case_globals)]
const LBS_NOINTEGRALHEIGHT: u32 = 0x0100;
#[allow(non_upper_case_globals)]
const CBS_DROPDOWNLIST: u32 = 0x0003;
#[allow(non_upper_case_globals)]
const CBS_HASSTRINGS: u32 = 0x0200;
#[allow(non_upper_case_globals)]
const TBM_GETPOS: u32 = 0x0400;
#[allow(non_upper_case_globals)]
const TB_THUMBTRACK: u32 = 5;

use crate::{
    app::{App, SystemKind, TIMER_FAV_KEY},
    i18n::{AppLang, tr, usage_minutes},
    monitor,
    settings::{FirmwarePackage, StepModifier},
    win,
};

use rows::{Action, DynText, Kind};

pub mod prefs;
pub mod rows;

// ───────────────────────────── metrics at 96 DPI ─────────────────────────────

const MARGIN: i32 = 20;
const CARD_PAD_X: i32 = 16;
const LABEL_WIDTH: i32 = 230;
const ROW_HEIGHT: i32 = 40;
const HEADER_HEIGHT: i32 = 38;
const SECTION_HEIGHT: i32 = 36;
const SLIDER_WIDTH: i32 = 220;
const VALUE_WIDTH: i32 = 46;
const COMBO_WIDTH: i32 = 240;
const SHORT_COMBO_WIDTH: i32 = 160;
const BUTTON_WIDTH: i32 = 220;
const SWITCH_WIDTH: i32 = 44;
const SWITCH_HEIGHT: i32 = 24;
const STAR_SIZE: i32 = 24;
const SWATCH_SIZE: i32 = 26;
const SWATCH_GAP: i32 = 6;
const INDENT: i32 = 18;
const NAV_WIDTH: i32 = 190;
const TOOLBAR_HEIGHT: i32 = 48;
const STATUS_HEIGHT: i32 = 30;
const HERO_IMAGE_HEIGHT: i32 = 156;

const ID_REFRESH: u16 = 100;
const ID_PREFERENCES: u16 = 101;
const ID_ABOUT: u16 = 102;
const ID_LOG: u16 = 103;
const ID_NAV: u16 = 104;

/// Minimum interval between DDC/CI writes while dragging a slider.
const SLIDER_DRAG_INTERVAL_MS: u128 = 250;

#[derive(Clone, Copy)]
pub struct Palette {
    pub background: u32,
    pub sidebar: u32,
    pub card: u32,
    pub control_bg: u32,
    pub control_active: u32,
    pub text: u32,
    pub muted: u32,
    pub line: u32,
}

/// Modern Light palette (BGR color values).
pub const LIGHT: Palette = Palette {
    background: 0x00F6_F4F3,
    sidebar: 0x00EC_EAE8,
    card: 0x00FF_FFFF,
    control_bg: 0x00F0_EDEC,
    control_active: 0x00D8_6B10,
    text: 0x001E_1C1A,
    muted: 0x0075_6E68,
    line: 0x00E4_E0DD,
};

/// Modern Dark palette (BGR color values).
pub const DARK: Palette = Palette {
    background: 0x001E_1C1B,
    sidebar: 0x0016_1514,
    card: 0x002B_2928,
    control_bg: 0x003A_3735,
    control_active: 0x00F0_8228,
    text: 0x00F4_F2F0,
    muted: 0x009E_9892,
    line: 0x003D_3A38,
};

/// Palette of a theme: `0` = system, `1` = light, `2` = dark.
pub fn palette_for(theme: u32) -> Palette {
    match theme {
        1 => LIGHT,
        2 => DARK,
        _ => {
            if win::system_prefers_dark() {
                DARK
            } else {
                LIGHT
            }
        }
    }
}

thread_local! {
    /// Cached GDI brushes for `WM_CTLCOLOR*` so synchronous messages during layout or state updates never fall
    /// through to `DefWindowProcW` (`#F0F0F0`).
    static PAINT_BRUSHES: RefCell<Option<(u32, HBRUSH, u32, HBRUSH)>> = const { RefCell::new(None) };
}

fn cached_brushes(palette: &Palette) -> (HBRUSH, HBRUSH) {
    PAINT_BRUSHES.with(|cell| {
        let mut entry = cell.borrow_mut();
        if let Some((c_card, b_card, c_side, b_side)) = *entry {
            if c_card == palette.card && c_side == palette.sidebar && !b_card.is_null() && !b_side.is_null() {
                return (b_card, b_side);
            }
            unsafe {
                DeleteObject(b_card as _);
                DeleteObject(b_side as _);
            }
        }
        let b_card = unsafe { CreateSolidBrush(palette.card) };
        let b_side = unsafe { CreateSolidBrush(palette.sidebar) };
        *entry = Some((palette.card, b_card, palette.sidebar, b_side));
        (b_card, b_side)
    })
}

// ───────────────────────────── model ─────────────────────────────

enum KindUi {
    Slider {
        track: HWND,
        value: HWND,
        factor: u16,
        min: i32,
        max: i32,
    },
    Switch {
        button: HWND,
        on: u16,
        off: u16,
        checked: bool,
    },
    ToggleGroup {
        buttons: Vec<HWND>,
        values: &'static [u16],
        selected: usize,
        is_crosshair: bool,
    },
    Choice {
        combo: HWND,
        values: &'static [u16],
    },
    Swatches {
        buttons: Vec<HWND>,
        values: &'static [u16],
        colors: &'static [u32],
        selected: usize,
    },
    Action {
        button: HWND,
        action: Action,
    },
    PictureMode {
        combo: HWND,
    },
    Variant {
        buttons: Vec<HWND>,
        selected: usize,
    },
    Firmware {
        combo: HWND,
    },
    System {
        combo: HWND,
        which: SystemKind,
    },
    Text {
        label: HWND,
        which: DynText,
    },
    Section {
        button: HWND,
    },
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
    right_aligned: bool,
}

struct RowUi {
    page: usize,
    height: i32,
    is_header: bool,
    is_hero: bool,
    /// Section that hides the row when it is collapsed.
    section: Option<usize>,
    elements: Vec<Element>,
    star: Option<HWND>,
}

#[derive(Clone, Copy)]
struct CardRect {
    top: i32,
    bottom: i32,
    dividers: [i32; 24],
    divider_count: usize,
}

struct PageUi {
    window: HWND,
    scroll: i32,
    content_height: i32,
    cards: Vec<CardRect>,
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
    title_font: HFONT,
    brush_bg: HBRUSH,
    brush_card: HBRUSH,
    brush_sidebar: HBRUSH,
    tooltip: HWND,
    palette: Palette,
    status: HWND,
    nav: HWND,
    nav_labels: Vec<String>,
    pages: Vec<PageUi>,
    rows: Vec<RowUi>,
    controls: Vec<Control>,
    sections: Vec<SectionUi>,
    /// Swatch buttons: window → (control, option).
    swatches: HashMap<isize, (usize, usize)>,
    /// Segmented toggle buttons: window → (control, option).
    toggles: HashMap<isize, (usize, usize)>,
    /// Owner-drawn push buttons (toolbar / action): window → label text.
    buttons: HashMap<isize, String>,
    /// Star buttons: window → (favourite id, is_active).
    stars: HashMap<isize, (&'static str, bool)>,
    fav_key_held: bool,
    active: usize,
    applying: bool,
    firmware: usize,
    dpi: u32,
    last_slider_send: Option<(u8, Instant)>,
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
    unsafe {
        SetTimer(app.host, TIMER_FAV_KEY, 100, None);
    }
    let _ = app.worker.send(monitor::WorkerCmd::CheckOsdChanges);
    let _ = app.worker.send(monitor::WorkerCmd::ReadUsageTime);
}

/// Closes the window and frees what belongs to it.
pub fn close(app: &mut App) {
    unsafe {
        KillTimer(app.host, TIMER_FAV_KEY);
    }
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

/// Rebuilds the open settings window while preserving the active page and scroll positions.
pub fn rebuild_preserving_view(app: &mut App) {
    let saved = app.window.as_ref().filter(|w| w.is_open()).map(|w| {
        let scrolls: Vec<i32> = w.pages.iter().map(|p| p.scroll).collect();
        (w.active, scrolls)
    });
    let Some((active_page, scrolls)) = saved else {
        return;
    };
    close(app);
    open(app);
    if let Some(mut window) = app.window.take() {
        if window.is_open() {
            for (page, scroll) in window.pages.iter_mut().zip(scrolls.into_iter()) {
                page.scroll = scroll;
            }
            if active_page < window.pages.len() && active_page != window.active {
                unsafe {
                    SendMessageW(window.nav, LB_SETCURSEL, active_page, 0);
                }
                show_page(&mut window, active_page);
            }
            reflow(&mut window, app);
        }
        app.window = Some(window);
    }
}

/// Live-updates the window when the system theme changes while `System` theme is selected.
pub fn refresh_theme(app: &mut App) {
    let target = palette_for(app.settings.theme);
    let Some(window) = app.window.as_ref() else {
        return;
    };
    if !window.is_open() || window.palette.background == target.background {
        return;
    }
    rebuild_preserving_view(app);
}

/// Updates the status bar with startup progress messages while reading VCP codes.
pub fn set_status_progress(app: &mut App, progress: &str) {
    if let Some(window) = app.window.as_ref()
        && window.is_open()
        && !window.status.is_null()
    {
        let text = app.tr(progress);
        unsafe {
            SetWindowTextW(window.status, win::wide(&text).as_ptr());
        }
    }
}

/// Polls whether the configured Favorites modifier key is held down and shows/hides unfavorited stars (`☆`).
pub fn poll_favorites_key(app: &mut App) {
    let Some(window) = app.window.as_mut() else {
        return;
    };
    if !window.is_open() {
        return;
    }
    let held = unsafe {
        match app.settings.favorite_modifier {
            StepModifier::Ctrl => (GetAsyncKeyState(VK_CONTROL as i32) as u16 & 0x8000) != 0,
            StepModifier::Shift => (GetAsyncKeyState(VK_SHIFT as i32) as u16 & 0x8000) != 0,
            StepModifier::Alt => (GetAsyncKeyState(VK_MENU as i32) as u16 & 0x8000) != 0,
            StepModifier::Super => {
                ((GetAsyncKeyState(VK_LWIN as i32) | GetAsyncKeyState(VK_RWIN as i32)) as u16 & 0x8000) != 0
            }
        }
    };
    if held == window.fav_key_held {
        return;
    }
    window.fav_key_held = held;
    let active_page = window.active;
    for row in window.rows.iter().filter(|r| r.page == active_page) {
        let Some(star_hwnd) = row.star else {
            continue;
        };
        let hidden_by_section = row
            .section
            .is_some_and(|sec| window.sections.get(sec).is_some_and(|s| s.collapsed));
        if hidden_by_section {
            continue;
        }
        let is_fav = window.stars.get(&(star_hwnd as isize)).is_some_and(|(_, fav)| *fav);
        let should_show = held || is_fav;
        unsafe {
            ShowWindow(star_hwnd, if should_show { SW_SHOW } else { SW_HIDE });
        }
    }
}

/// Creates the window itself, without its contents.
fn create_shell(app: &mut App) -> Window {
    let instance = unsafe { GetModuleHandleW(ptr::null()) };
    register_classes(instance);
    let dpi = win::system_dpi();
    let palette = palette_for(app.settings.theme);
    let _ = cached_brushes(&palette);
    let font = make_font(dpi, 14, false);
    let bold = make_font(dpi, 14, true);
    let title_font = make_font(dpi, 18, true);
    let brush_bg = unsafe { CreateSolidBrush(palette.background) };
    let brush_card = unsafe { CreateSolidBrush(palette.card) };
    let brush_sidebar = unsafe { CreateSolidBrush(palette.sidebar) };
    let title = win::wide(&app.tr("Titan Control"));

    let hwnd = unsafe {
        CreateWindowExW(
            0,
            win::wide("TitanControlSettings").as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
            0,
            0,
            win::scaled(1040, dpi),
            win::scaled(780, dpi),
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
        title_font,
        brush_bg,
        brush_card,
        brush_sidebar,
        tooltip: ptr::null_mut(),
        palette,
        status: ptr::null_mut(),
        nav: ptr::null_mut(),
        nav_labels: Vec::new(),
        pages: Vec::new(),
        rows: Vec::new(),
        controls: Vec::new(),
        sections: Vec::new(),
        swatches: HashMap::new(),
        toggles: HashMap::new(),
        buttons: HashMap::new(),
        stars: HashMap::new(),
        fav_key_held: false,
        active: 0,
        applying: false,
        firmware: FirmwarePackage::ALL
            .iter()
            .position(|package| *package == app.settings.firmware_package)
            .unwrap_or(0),
        dpi,
        last_slider_send: None,
    };
    if hwnd.is_null() {
        crate::log::write(format_args!("[UI] cannot create the settings window: {}", win::last_error()));
        return window;
    }
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, app as *mut App as isize);
        let dpi = win::dpi_of(hwnd);
        let mut window = window;
        if dpi != window.dpi {
            rescale(&mut window, dpi);
        }
        win::set_dark_frame(hwnd, app.dark_caption());
        SetClassLongPtrW(hwnd, GCLP_HBRBACKGROUND, window.brush_sidebar as isize);
        SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, app.icon_large as isize);
        SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, app.icon_small as isize);
        win::center(hwnd, win::scaled(1040, dpi), win::scaled(780, dpi));
        window
    }
}

fn rebuild(app: &mut App) {
    let Some(mut window) = app.window.take() else {
        return;
    };
    build(app, &mut window);
    let hwnd = window.hwnd;
    // Restore `app.window` BEFORE calling `apply_state`, `refresh_availability`, `ShowWindow`, and `UpdateWindow`
    // so synchronous `WM_CTLCOLOR*`, `WM_DRAWITEM`, and `WM_PAINT` messages always see `app.window == Some(...)`.
    app.window = Some(window);
    apply_state(app);
    refresh_availability(app);
    unsafe {
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
        UpdateWindow(hwnd);
    }
}

fn make_font(dpi: u32, size_pt: i32, bold: bool) -> HFONT {
    let face = win::wide("Segoe UI");
    unsafe {
        CreateFontW(
            -win::scaled(size_pt, dpi),
            0,
            0,
            0,
            if bold { 600 } else { 400 },
            0,
            0,
            0,
            1,
            0,
            0,
            5, // CLEARTYPE_QUALITY
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
    let dpi = window.dpi;
    unsafe {
        for (id, text) in [
            (ID_REFRESH, "Odśwież"),
            (ID_PREFERENCES, "Preferencje"),
            (ID_ABOUT, "O programie"),
            (ID_LOG, "Pokaż plik logu"),
        ] {
            let label = tr(&lang, text);
            let button = CreateWindowExW(
                0,
                win::wide("BUTTON").as_ptr(),
                win::wide(&label).as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW,
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
            window.buttons.insert(button as isize, label);
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
            WS_CHILD | WS_VISIBLE | LBS_NOTIFY | LBS_NOINTEGRALHEIGHT | LBS_OWNERDRAWFIXED | LBS_HASSTRINGS,
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
        SendMessageW(window.nav, LB_SETITEMHEIGHT, 0, win::scaled(38, dpi) as isize);

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
        let nav_title = tr(&lang, page.title);
        window.nav_labels.push(nav_title.clone());
        unsafe {
            let label = win::wide(&nav_title);
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
            window.pages.push(PageUi {
                window: page_window,
                scroll: 0,
                content_height: 0,
                cards: Vec::new(),
            });
        }
        build_page(app, window, index, page);
    }

    let page = std::env::var("TITAN_UI_PAGE")
        .ok()
        .and_then(|index| index.parse::<usize>().ok())
        .unwrap_or(0)
        .min(window.pages.len().saturating_sub(1));
    window.active = page;
    unsafe {
        SendMessageW(window.nav, LB_SETCURSEL, page, 0);
    }
    show_page(window, page);
    layout(window, app);
    theme_children(window);
}

/// Applies modern Explorer / DarkMode_Explorer themes to child controls (never to the top-level window frame).
fn theme_children(window: &Window) {
    let dark = window.palette.background == DARK.background;
    unsafe {
        win::set_control_theme(window.nav, dark);
        win::set_control_theme(window.tooltip, dark);
        for page in &window.pages {
            win::set_control_theme(page.window, dark);
            let mut child = GetWindow(page.window, GW_CHILD);
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
}

fn build_page(app: &mut App, window: &mut Window, page: usize, spec: &rows::Page) {
    let lang = app.lang;
    let dpi = window.dpi;
    let scale = |value: i32| win::scaled(value, dpi);
    let page_window = window.pages[page].window;
    let label_x = scale(MARGIN + CARD_PAD_X);
    let label_width = scale(LABEL_WIDTH);
    let control_x = label_x + label_width + scale(28);
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
                section = None;
                let label = create_label(window, page_window, &row_label, window.bold, false);
                elements.push(Element {
                    window: label,
                    x: scale(MARGIN + 4),
                    dy: scale(12),
                    width: label_width + scale(120),
                    height: scale(22),
                    right_aligned: false,
                });
                window.rows.push(RowUi {
                    page,
                    height: scale(HEADER_HEIGHT),
                    is_header: true,
                    is_hero: false,
                    section: None,
                    elements,
                    star: None,
                });
                continue;
            }
            Kind::Section => {
                let button = create_owner_button(window, page_window, &row_label, window.bold);
                elements.push(Element {
                    window: button,
                    x: label_x,
                    dy: scale(6),
                    width: label_width + scale(160),
                    height: scale(26),
                    right_aligned: false,
                });
                window.sections.push(SectionUi {
                    button,
                    collapsed: false,
                    title: row_label.clone(),
                });
                section = Some(window.sections.len() - 1);
                window.controls.push(Control { kind: KindUi::Section { button }, code: None });
                window.rows.push(RowUi {
                    page,
                    height: scale(SECTION_HEIGHT),
                    is_header: false,
                    is_hero: false,
                    section: None,
                    elements,
                    star: None,
                });
                continue;
            }
            _ => {}
        }

        let is_hero_title = matches!(spec_row.kind, Kind::Text(DynText::MonitorTitle));
        let is_hero_sub = matches!(spec_row.kind, Kind::Text(DynText::MonitorSubtitle));
        if is_hero_title || is_hero_sub {
            let font = if is_hero_title { window.title_font } else { window.font };
            let value = create_centered_label(window, page_window, "—", font);
            let row_h = if is_hero_title {
                scale(HERO_IMAGE_HEIGHT + 30)
            } else {
                scale(28)
            };
            let dy = if is_hero_title { scale(HERO_IMAGE_HEIGHT + 4) } else { scale(2) };
            elements.push(Element {
                window: value,
                x: scale(MARGIN),
                dy,
                width: scale(520),
                height: scale(24),
                right_aligned: false,
            });
            let which = match spec_row.kind {
                Kind::Text(w) => w,
                _ => unreachable!(),
            };
            window.controls.push(Control {
                kind: KindUi::Text { label: value, which },
                code: None,
            });
            window.rows.push(RowUi {
                page,
                height: row_h,
                is_header: false,
                is_hero: true,
                section: None,
                elements,
                star: None,
            });
            continue;
        }

        let has_tip = spec_row.tooltip.is_some();
        let label_text = if has_tip {
            format!("{row_label}  \u{24d8}")
        } else {
            row_label.clone()
        };
        let label = create_label(window, page_window, &label_text, window.font, has_tip);
        elements.push(Element {
            window: label,
            x: label_x + sub_offset,
            dy: scale(10),
            width: label_width - sub_offset,
            height: scale(20),
            right_aligned: false,
        });
        if let Some(tip) = spec_row.tooltip {
            add_tooltip(window.tooltip, label, &tr(&lang, tip));
        }

        // Star of a favourite row (hidden unless already a favorite or the user holds the Favorites key).
        let mut star = None;
        if let Some(id) = spec_row.favorite {
            let button = create_owner_button(window, page_window, "\u{2606}", window.font);
            star = Some(button);
            window.stars.insert(button as isize, (id, false));
            elements.push(Element {
                window: button,
                x: label_x + label_width + scale(2),
                dy: scale(8),
                width: star_size,
                height: star_size,
                right_aligned: false,
            });
        }

        let kind = match &spec_row.kind {
            Kind::Text(which) => {
                let value = create_label(window, page_window, "—", window.font, false);
                elements.push(Element {
                    window: value,
                    x: control_x,
                    dy: scale(10),
                    width: scale(300),
                    height: scale(20),
                    right_aligned: true,
                });
                KindUi::Text { label: value, which: *which }
            }
            Kind::Slider { min, max, factor } => {
                let track = create_trackbar(window, page_window, *min, *max);
                elements.push(Element {
                    window: track,
                    x: control_x,
                    dy: scale(8),
                    width: slider_width,
                    height: scale(24),
                    right_aligned: true,
                });
                let value = create_label(window, page_window, "0", window.font, false);
                elements.push(Element {
                    window: value,
                    x: control_x + slider_width + scale(8),
                    dy: scale(10),
                    width: value_width,
                    height: scale(20),
                    right_aligned: true,
                });
                KindUi::Slider {
                    track,
                    value,
                    factor: *factor,
                    min: *min,
                    max: *max,
                }
            }
            Kind::Switch { on, off } => {
                let button = create_owner_button(window, page_window, "", window.font);
                elements.push(Element {
                    window: button,
                    x: control_x,
                    dy: scale(8),
                    width: scale(SWITCH_WIDTH),
                    height: scale(SWITCH_HEIGHT),
                    right_aligned: true,
                });
                KindUi::Switch {
                    button,
                    on: *on,
                    off: *off,
                    checked: false,
                }
            }
            Kind::ToggleGroup { values, options } => {
                let is_crosshair = spec_row.code == Some(monitor::VCP_CROSSHAIR_SHAPE);
                let count = options.len().max(1) as i32;
                let gap = scale(4);
                let mut buttons = Vec::new();
                let mut x_cursor = control_x;
                for option in options.iter() {
                    let translated = tr(&lang, option);
                    let char_count = translated.chars().count() as i32;
                    let pill_w = if is_crosshair {
                        scale(36)
                    } else if count >= 6 {
                        scale((char_count * 7 + 16).max(38))
                    } else {
                        scale((char_count * 7 + 22).max(46))
                    };
                    let button = create_owner_button(window, page_window, &translated, window.font);
                    elements.push(Element {
                        window: button,
                        x: x_cursor,
                        dy: scale(6),
                        width: pill_w,
                        height: scale(28),
                        right_aligned: true,
                    });
                    x_cursor += pill_w + gap;
                    buttons.push(button);
                }
                let control_index = window.controls.len();
                for (idx, button) in buttons.iter().enumerate() {
                    window.toggles.insert(*button as isize, (control_index, idx));
                }
                KindUi::ToggleGroup {
                    buttons,
                    values,
                    selected: usize::MAX,
                    is_crosshair,
                }
            }
            Kind::Choice { values, options } => {
                let combo = create_combo(window, page_window);
                for option in options.iter() {
                    add_combo_string(combo, &tr(&lang, option));
                }
                elements.push(Element {
                    window: combo,
                    x: control_x,
                    dy: scale(7),
                    width: combo_width,
                    height: scale(220),
                    right_aligned: true,
                });
                KindUi::Choice { combo, values }
            }
            Kind::Swatches { values, options, colors } => {
                let mut buttons = Vec::new();
                for (index, (option, _color)) in options.iter().zip(colors.iter()).enumerate() {
                    let button = create_owner_button(window, page_window, "", window.font);
                    elements.push(Element {
                        window: button,
                        x: control_x + (index as i32) * (swatch + swatch_gap),
                        dy: scale(7),
                        width: swatch,
                        height: swatch,
                        right_aligned: true,
                    });
                    add_tooltip(window.tooltip, button, &tr(&lang, option));
                    buttons.push(button);
                }
                let control_index = window.controls.len();
                for (index, button) in buttons.iter().enumerate() {
                    window.swatches.insert(*button as isize, (control_index, index));
                }
                KindUi::Swatches {
                    buttons,
                    values,
                    colors,
                    selected: usize::MAX,
                }
            }
            Kind::Action(action) => {
                let button = create_owner_button(window, page_window, &row_label, window.font);
                elements.push(Element {
                    window: button,
                    x: control_x,
                    dy: scale(6),
                    width: button_width,
                    height: scale(28),
                    right_aligned: true,
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
                    dy: scale(7),
                    width: combo_width,
                    height: scale(260),
                    right_aligned: true,
                });
                KindUi::PictureMode { combo }
            }
            Kind::Variant => {
                let mut buttons = Vec::new();
                let mut x_cursor = control_x;
                let pill_w = scale(96);
                let gap = scale(6);
                for option in ["Domyślne", "Własne"] {
                    let translated = tr(&lang, option);
                    let button = create_owner_button(window, page_window, &translated, window.font);
                    elements.push(Element {
                        window: button,
                        x: x_cursor,
                        dy: scale(6),
                        width: pill_w,
                        height: scale(28),
                        right_aligned: true,
                    });
                    x_cursor += pill_w + gap;
                    buttons.push(button);
                }
                let control_index = window.controls.len();
                for (idx, button) in buttons.iter().enumerate() {
                    window.toggles.insert(*button as isize, (control_index, idx));
                }
                KindUi::Variant {
                    buttons,
                    selected: 0,
                }
            }
            Kind::FirmwarePackage => {
                let combo = create_combo(window, page_window);
                for label in ["2024-12-11", "2025-09-20"] {
                    add_combo_string(combo, label);
                }
                elements.push(Element {
                    window: combo,
                    x: control_x,
                    dy: scale(7),
                    width: short_combo,
                    height: scale(200),
                    right_aligned: true,
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
                    dy: scale(7),
                    width: combo_width,
                    height: scale(220),
                    right_aligned: true,
                });
                KindUi::System { combo, which }
            }
            Kind::Header | Kind::Section => unreachable!(),
        };
        window.controls.push(Control { kind, code: spec_row.code });
        window.rows.push(RowUi {
            page,
            height: scale(ROW_HEIGHT),
            is_header: false,
            is_hero: false,
            section,
            elements,
            star,
        });
    }
}

// ───────────────────────────── widget factories & wheel subclass ─────────────────────────────

#[link(name = "comctl32")]
unsafe extern "system" {
    fn SetWindowSubclass(
        hwnd: HWND,
        pfn_subclass: unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM, usize, usize) -> LRESULT,
        u_id_subclass: usize,
        dw_ref_data: usize,
    ) -> i32;
    fn DefSubclassProc(hwnd: HWND, u_msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT;
}

/// Subclass procedure for trackbars and combo boxes so `WM_MOUSEWHEEL` scrolls the page instead of modifying VCP settings.
unsafe extern "system" fn wheel_forward_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if msg == WM_MOUSEWHEEL {
        let parent = unsafe { GetParent(hwnd) };
        if !parent.is_null() {
            return unsafe { SendMessageW(parent, WM_MOUSEWHEEL, wparam, lparam) };
        }
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

fn create_label(_window: &Window, parent: HWND, text: &str, font: HFONT, notify: bool) -> HWND {
    let style = WS_CHILD | WS_VISIBLE | SS_LEFT | SS_ENDELLIPSIS | if notify { SS_NOTIFY } else { 0 };
    unsafe {
        let label = CreateWindowExW(
            0,
            win::wide("STATIC").as_ptr(),
            win::wide(text).as_ptr(),
            style,
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
        label
    }
}

fn create_centered_label(_window: &Window, parent: HWND, text: &str, font: HFONT) -> HWND {
    unsafe {
        let label = CreateWindowExW(
            0,
            win::wide("STATIC").as_ptr(),
            win::wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | SS_CENTER | SS_ENDELLIPSIS,
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
        label
    }
}

fn create_owner_button(window: &mut Window, parent: HWND, text: &str, font: HFONT) -> HWND {
    unsafe {
        let button = CreateWindowExW(
            0,
            win::wide("BUTTON").as_ptr(),
            win::wide(text).as_ptr(),
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
        SendMessageW(button, WM_SETFONT, font as usize, 1);
        if !text.is_empty() {
            window.buttons.insert(button as isize, text.to_string());
        }
        button
    }
}

fn create_trackbar(window: &Window, parent: HWND, min: i32, max: i32) -> HWND {
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
        let range_lparam = ((max as isize) << 16) | ((min as isize) & 0xFFFF);
        SendMessageW(track, TBM_SETRANGE, 1, range_lparam);
        SendMessageW(track, TBM_SETPAGESIZE, 0, ((max - min) / 10).max(1) as isize);
        SetWindowSubclass(track, wheel_forward_proc, 1, 0);
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
            240,
            parent,
            ptr::null_mut(),
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(combo, WM_SETFONT, window.font as usize, 1);
        let dark = window.palette.background == DARK.background;
        win::set_combo_theme(combo, dark);
        SetWindowSubclass(combo, wheel_forward_proc, 1, 0);
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

// ───────────────────────────── layout & fast scrolling ─────────────────────────────

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
        let mut x = nav + win::scaled(MARGIN, dpi);
        for id in [ID_REFRESH, ID_PREFERENCES, ID_ABOUT, ID_LOG] {
            let button = GetDlgItem(hwnd, id as i32);
            let button_width = win::scaled(132, dpi);
            MoveWindow(button, x, win::scaled(10, dpi), button_width, win::scaled(28, dpi), 1);
            x += button_width + win::scaled(8, dpi);
        }
        MoveWindow(
            window.status,
            win::scaled(16, dpi),
            (height - status + win::scaled(6, dpi)).max(0),
            (width - win::scaled(32, dpi)).max(0),
            win::scaled(20, dpi),
            1,
        );
        MoveWindow(window.nav, 0, toolbar, nav, (height - toolbar - status).max(0), 1);
        for page in &window.pages {
            MoveWindow(
                page.window,
                nav,
                toolbar,
                (width - nav).max(0),
                (height - toolbar - status).max(0),
                1,
            );
        }
    }
    reflow(window, app);
}

/// Positions the controls of the active page using `DeferWindowPos` and computes the card rectangles for `WM_PAINT`.
fn reflow(window: &mut Window, _app: &mut App) {
    let page_index = window.active;
    if page_index >= window.pages.len() {
        return;
    }
    let dpi = window.dpi;
    let margin = win::scaled(MARGIN, dpi);
    let page_window = window.pages[page_index].window;
    let mut client: RECT = unsafe { std::mem::zeroed() };
    unsafe {
        GetClientRect(page_window, &mut client);
    }
    let viewport = client.bottom.max(0);
    let width = client.right.max(0);
    let card_right = (width - margin).max(margin + win::scaled(320, dpi));
    let base_control_x = win::scaled(MARGIN + CARD_PAD_X + LABEL_WIDTH + 28, dpi);

    // First pass: calculate total content height and card geometry in content-space coordinates.
    let mut y = margin / 2;
    let mut cards: Vec<CardRect> = Vec::new();
    let mut current_card: Option<CardRect> = None;

    for row in window.rows.iter().filter(|row| row.page == page_index) {
        let hidden = row
            .section
            .is_some_and(|sec| window.sections.get(sec).is_some_and(|s| s.collapsed));
        if hidden {
            continue;
        }
        if row.is_hero {
            y += row.height;
            continue;
        }
        if row.is_header {
            if let Some(card) = current_card.take() {
                cards.push(card);
            }
            y += win::scaled(6, dpi);
            let header_bottom = y + row.height;
            y = header_bottom;
            current_card = Some(CardRect {
                top: y,
                bottom: y,
                dividers: [0; 24],
                divider_count: 0,
            });
            continue;
        }
        if current_card.is_none() {
            current_card = Some(CardRect {
                top: y,
                bottom: y,
                dividers: [0; 24],
                divider_count: 0,
            });
        }
        if let Some(card) = current_card.as_mut() {
            if card.bottom > card.top && card.divider_count < card.dividers.len() {
                card.dividers[card.divider_count] = y;
                card.divider_count += 1;
            }
            card.bottom = y + row.height;
        }
        y += row.height;
    }
    if let Some(card) = current_card.take()
        && card.bottom > card.top
    {
        cards.push(card);
    }

    let content = y + margin;
    window.pages[page_index].content_height = content;
    window.pages[page_index].cards = cards;
    let max_scroll = (content - viewport).max(0);
    let scroll = window.pages[page_index].scroll.clamp(0, max_scroll);
    window.pages[page_index].scroll = scroll;

    // Second pass: batch-position child HWNDs on the active page using BeginDeferWindowPos.
    let mut y = margin / 2;
    let hdwp = unsafe { BeginDeferWindowPos(64) };
    let mut current_hdwp = hdwp;

    // Calculate right-shift offset so right-aligned controls sit cleanly near the right edge of the card.
    let right_pad = win::scaled(CARD_PAD_X, dpi);
    let target_control_right = card_right - right_pad;

    for row in window.rows.iter().filter(|row| row.page == page_index) {
        let hidden = row
            .section
            .is_some_and(|sec| window.sections.get(sec).is_some_and(|s| s.collapsed));
        if hidden {
            for element in &row.elements {
                unsafe {
                    if IsWindowVisible(element.window) != 0 {
                        ShowWindow(element.window, SW_HIDE);
                    }
                }
            }
            continue;
        }
        if row.is_header {
            y += win::scaled(6, dpi);
        }

        // Compute the rightmost extent of right-aligned elements in this row so the group aligns to `target_control_right`.
        let max_right = row
            .elements
            .iter()
            .filter(|e| e.right_aligned)
            .map(|e| e.x + e.width)
            .max()
            .unwrap_or(base_control_x);
        let shift_x = (target_control_right - max_right).max(0);

        for element in &row.elements {
            if Some(element.window) == row.star {
                let is_fav = window.stars.get(&(element.window as isize)).is_some_and(|(_, fav)| *fav);
                let show_star = window.fav_key_held || is_fav;
                unsafe {
                    SetWindowPos(
                        element.window,
                        ptr::null_mut(),
                        element.x,
                        y + element.dy - scroll,
                        element.width,
                        element.height,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                    ShowWindow(element.window, if show_star { SW_SHOW } else { SW_HIDE });
                }
                continue;
            }

            let (elem_x, elem_w) = if row.is_hero {
                let hero_w = (card_right - margin).max(win::scaled(280, dpi));
                (margin, hero_w)
            } else if element.right_aligned {
                (element.x + shift_x, element.width)
            } else {
                (element.x, element.width)
            };

            unsafe {
                if !current_hdwp.is_null() {
                    current_hdwp = DeferWindowPos(
                        current_hdwp,
                        element.window,
                        ptr::null_mut(),
                        elem_x,
                        y + element.dy - scroll,
                        elem_w,
                        element.height,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                } else {
                    SetWindowPos(
                        element.window,
                        ptr::null_mut(),
                        elem_x,
                        y + element.dy - scroll,
                        elem_w,
                        element.height,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
                if IsWindowVisible(element.window) == 0 {
                    ShowWindow(element.window, SW_SHOW);
                }
            }
        }
        y += row.height;
    }

    unsafe {
        if !current_hdwp.is_null() {
            EndDeferWindowPos(current_hdwp);
        }
        let mut info: SCROLLINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<SCROLLINFO>() as u32;
        info.fMask = SIF_RANGE | SIF_PAGE | SIF_POS;
        info.nMin = 0;
        info.nMax = content;
        info.nPage = viewport as u32;
        info.nPos = scroll;
        SetScrollInfo(page_window, SB_VERT, &info, 1);
        InvalidateRect(page_window, ptr::null(), 0);
    }
}

fn show_page(window: &mut Window, index: usize) {
    window.active = index;
    for (position, page) in window.pages.iter().enumerate() {
        unsafe {
            ShowWindow(page.window, if position == index { SW_SHOW } else { SW_HIDE });
        }
    }
}

/// Scrolls the active page using `ScrollWindowEx` without reflowing all 6 pages.
fn scroll_page_fast(window: &mut Window, page_window: HWND, request: i32) {
    let Some(index) = window.pages.iter().position(|page| page.window == page_window) else {
        return;
    };
    let mut client: RECT = unsafe { std::mem::zeroed() };
    unsafe {
        GetClientRect(page_window, &mut client);
    }
    let viewport = client.bottom.max(0);
    let content = window.pages[index].content_height;
    let max = (content - viewport).max(0);
    let new_scroll = request.clamp(0, max);
    let old_scroll = window.pages[index].scroll;
    let dy = old_scroll - new_scroll;
    if dy == 0 {
        return;
    }
    window.pages[index].scroll = new_scroll;
    unsafe {
        let mut info: SCROLLINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<SCROLLINFO>() as u32;
        info.fMask = SIF_POS;
        info.nPos = new_scroll;
        SetScrollInfo(page_window, SB_VERT, &info, 1);
        ScrollWindowEx(
            page_window,
            0,
            dy,
            ptr::null(),
            ptr::null(),
            ptr::null_mut(),
            ptr::null_mut(),
            SW_SCROLLCHILDREN | SW_INVALIDATE,
        );
        InvalidateRect(page_window, ptr::null(), 0);
        UpdateWindow(page_window);
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
    let _ = cached_brushes(&window.palette);
    let values = app.state.clone();
    let info = app.info.clone();
    let language = app.lang;
    let firmware = window.firmware;

    let active_mode_idx = values
        .get(&monitor::VCP_MODE)
        .copied()
        .and_then(monitor::picture_mode_from_vcp)
        .map(|(mode, _)| mode as u16);

    for control in &mut window.controls {
        match &mut control.kind {
            KindUi::Slider {
                track,
                value,
                factor,
                min,
                max,
            } => {
                let Some(current) = control.code.and_then(|code| values.get(&code).copied()) else {
                    continue;
                };
                let position = ((current / (*factor).max(1)) as i32).clamp(*min, *max) as isize;
                unsafe {
                    SendMessageW(*track, TBM_SETPOS, 1, position);
                    SetWindowTextW(*value, win::wide(&current.to_string()).as_ptr());
                }
            }
            KindUi::Switch {
                button,
                on,
                checked,
                ..
            } => {
                let Some(current) = control.code.and_then(|code| values.get(&code).copied()) else {
                    continue;
                };
                let new_checked = current == *on;
                if *checked != new_checked {
                    *checked = new_checked;
                    unsafe {
                        InvalidateRect(*button, ptr::null(), 0);
                    }
                }
            }
            KindUi::ToggleGroup {
                buttons,
                values: options,
                selected,
                ..
            } => {
                let Some(current) = control.code.and_then(|code| values.get(&code).copied()) else {
                    continue;
                };
                if let Some(index) = options.iter().position(|candidate| *candidate == current)
                    && *selected != index
                {
                    *selected = index;
                    for button in buttons.iter() {
                        unsafe {
                            InvalidateRect(*button, ptr::null(), 0);
                        }
                    }
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
            KindUi::Swatches {
                buttons,
                values: options,
                selected,
                ..
            } => {
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
            KindUi::Variant { buttons, selected } => {
                if let Some(current) = values.get(&monitor::VCP_MODE).copied()
                    && let Some((_, custom)) = monitor::picture_mode_from_vcp(current)
                {
                    let idx = custom as usize;
                    if *selected != idx {
                        *selected = idx;
                        for button in buttons.iter() {
                            unsafe {
                                InvalidateRect(*button, ptr::null(), 0);
                            }
                        }
                    }
                }
            }
            KindUi::Firmware { combo } => unsafe {
                SendMessageW(*combo, CB_SETCURSEL, firmware, 0);
            },
            KindUi::Text { label, which } => {
                let text = match which {
                    DynText::MonitorTitle => info
                        .as_ref()
                        .map(|info| info.name.clone())
                        .unwrap_or_else(|| "Titan Army P275MV PLUS".to_string()),
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
                        .unwrap_or_else(|| tr(&language, "Łączenie z monitorem…")),
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

    // Update favorite stars (both picture_mode and tray shortcut controls).
    let active_page = window.active;
    for (button_key, (id, active_flag)) in window.stars.iter_mut() {
        let is_fav = if *id == "picture_mode" {
            active_mode_idx.is_some_and(|mode| app.settings.is_favorite_picture_mode(mode))
        } else {
            app.settings.is_favorite_tray_control(id)
        };
        *active_flag = is_fav;
        let button = *button_key as HWND;
        let glyph = if is_fav { "\u{2605}" } else { "\u{2606}" };
        window.buttons.insert(*button_key, glyph.to_string());
        unsafe {
            SetWindowTextW(button, win::wide(glyph).as_ptr());
            InvalidateRect(button, ptr::null(), 0);
        }
    }
    for row in window.rows.iter().filter(|r| r.page == active_page) {
        if let Some(star_hwnd) = row.star {
            let hidden = row
                .section
                .is_some_and(|sec| window.sections.get(sec).is_some_and(|s| s.collapsed));
            let is_fav = window.stars.get(&(star_hwnd as isize)).is_some_and(|(_, fav)| *fav);
            unsafe {
                ShowWindow(
                    star_hwnd,
                    if !hidden && (window.fav_key_held || is_fav) {
                        SW_SHOW
                    } else {
                        SW_HIDE
                    },
                );
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
                let dpi = if window.hwnd.is_null() {
                    win::system_dpi()
                } else {
                    win::dpi_of(window.hwnd)
                };
                let scale = dpi * 100 / 96;
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
    let Some(window) = app.window.as_ref() else {
        return;
    };
    if !window.is_open() {
        return;
    }
    let _ = cached_brushes(&window.palette);
    let allowed = app.availability();
    let custom = app.get(monitor::VCP_MODE).is_none_or(|mode| monitor::is_custom_profile(mode as u8));
    let color_temp_user = app.get(monitor::VCP_COLOR_TEMP).is_some_and(monitor::is_user_color_temp);
    let connected = app.connected;
    let loading = app.profile_loading;
    let hdr_locked = app.hdr_locked();
    let led_has_colorful = monitor::led_mode_has_colorful(app.get(monitor::VCP_LED_MODE));

    for control in &window.controls {
        let enabled = match &control.kind {
            KindUi::Action { .. } => {
                let code_ok = control
                    .code
                    .map(|code| allowed.get(&code).copied().unwrap_or(true))
                    .unwrap_or(true);
                connected && code_ok
            }
            KindUi::PictureMode { .. } | KindUi::Variant { .. } => connected && !loading && !hdr_locked,
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

        if let KindUi::Swatches { buttons, values, .. } = &control.kind {
            for (idx, hwnd) in buttons.iter().enumerate() {
                let mut btn_ok = enabled;
                if control.code == Some(monitor::VCP_LED_COLOR)
                    && values.get(idx) == Some(&monitor::values::LED_COLOR_COLORFUL)
                {
                    btn_ok &= led_has_colorful;
                }
                unsafe {
                    EnableWindow(*hwnd, if btn_ok { 1 } else { 0 });
                    InvalidateRect(*hwnd, ptr::null(), 0);
                }
            }
            continue;
        }

        for hwnd in control_windows(&control.kind) {
            unsafe {
                EnableWindow(hwnd, if enabled { 1 } else { 0 });
                InvalidateRect(hwnd, ptr::null(), 0);
            }
        }
    }
}

fn control_windows(kind: &KindUi) -> Vec<HWND> {
    match kind {
        KindUi::Slider { track, .. } => vec![*track],
        KindUi::Switch { button, .. } => vec![*button],
        KindUi::ToggleGroup { buttons, .. } | KindUi::Variant { buttons, .. } => buttons.clone(),
        KindUi::Choice { combo, .. } => vec![*combo],
        KindUi::Swatches { buttons, .. } => buttons.clone(),
        KindUi::Action { button, .. } => vec![*button],
        KindUi::PictureMode { combo } | KindUi::Firmware { combo } => vec![*combo],
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
    if let Some(mut window) = app.window.take() {
        if window.applying {
            app.window = Some(window);
            return;
        }
        if item == window.nav {
            if notification == LBN_SELCHANGE {
                let index = unsafe { SendMessageW(window.nav, LB_GETCURSEL, 0, 0) } as usize;
                if index < window.pages.len() {
                    show_page(&mut window, index);
                    reflow(&mut window, app);
                    unsafe {
                        InvalidateRect(window.nav, ptr::null(), 0);
                    }
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

        // Favorite stars (handles both "picture_mode" and tray control shortcuts).
        if let Some((fav_id, _)) = window.stars.get(&(item as isize)).copied() {
            if fav_id == "picture_mode" {
                let current_mode = app
                    .get(monitor::VCP_MODE)
                    .and_then(monitor::picture_mode_from_vcp)
                    .map(|(mode, _)| mode as u16);
                let Some(mode) = current_mode else {
                    app.window = Some(window);
                    return;
                };
                let active = app.settings.is_favorite_picture_mode(mode);
                if !active && app.settings.favorite_picture_mode_set().len() >= 5 {
                    app.window = Some(window);
                    return;
                }
                app.settings.favorite_picture_modes.retain(|&m| m != mode);
                if !active {
                    app.settings.favorite_picture_modes.push(mode);
                }
                app.save_settings();
                let new_active = !active;
                window.stars.insert(item as isize, (fav_id, new_active));
                let glyph = if new_active { "\u{2605}" } else { "\u{2606}" };
                window.buttons.insert(item as isize, glyph.to_string());
                unsafe {
                    SetWindowTextW(item, win::wide(glyph).as_ptr());
                    InvalidateRect(item, ptr::null(), 0);
                }
                app.window = Some(window);
                crate::register_hotkeys(app);
                crate::tray::refresh(app);
                return;
            }

            let active = app.settings.is_favorite_tray_control(fav_id);
            if !active && app.settings.favorite_tray_control_set().len() >= 5 {
                app.window = Some(window);
                return;
            }
            app.settings.favorite_tray_controls.retain(|favorite| favorite != fav_id);
            if !active {
                app.settings.favorite_tray_controls.push(fav_id.to_string());
            }
            app.save_settings();
            let new_active = !active;
            window.stars.insert(item as isize, (fav_id, new_active));
            let glyph = if new_active { "\u{2605}" } else { "\u{2606}" };
            window.buttons.insert(item as isize, glyph.to_string());
            unsafe {
                SetWindowTextW(item, win::wide(glyph).as_ptr());
                InvalidateRect(item, ptr::null(), 0);
            }
            app.window = Some(window);
            crate::tray::refresh(app);
            return;
        }

        // Segmented toggle pills (`KindUi::ToggleGroup` and `KindUi::Variant`).
        if let Some((control_idx, option_idx)) = window.toggles.get(&(item as isize)).copied() {
            let code = window.controls[control_idx].code;
            match &mut window.controls[control_idx].kind {
                KindUi::ToggleGroup {
                    buttons,
                    values,
                    selected,
                    ..
                } => {
                    if let (Some(value), Some(code)) = (values.get(option_idx).copied(), code) {
                        *selected = option_idx;
                        for button in buttons.iter() {
                            unsafe {
                                InvalidateRect(*button, ptr::null(), 0);
                            }
                        }
                        app.window = Some(window);
                        app.set(code, value);
                        app.show_saturation();
                        apply_state(app);
                        refresh_availability(app);
                        return;
                    }
                }
                KindUi::Variant { buttons, selected } => {
                    *selected = option_idx;
                    for button in buttons.iter() {
                        unsafe {
                            InvalidateRect(*button, ptr::null(), 0);
                        }
                    }
                    let custom = option_idx == 1;
                    let mode = window
                        .controls
                        .iter()
                        .find_map(|c| match &c.kind {
                            KindUi::PictureMode { combo } => {
                                Some(unsafe { SendMessageW(*combo, CB_GETCURSEL, 0, 0) } as usize)
                            }
                            _ => None,
                        })
                        .unwrap_or(0);
                    app.window = Some(window);
                    if let Some(value) = monitor::picture_mode_vcp_value(mode, custom) {
                        app.start_profile_load();
                        app.set(monitor::VCP_MODE, value);
                        apply_state(app);
                        refresh_availability(app);
                    }
                    return;
                }
                _ => {}
            }
            app.window = Some(window);
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
                refresh_availability(app);
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

        // Collapsible sections.
        if let KindUi::Section { button } = window.controls[index].kind {
            if let Some(sec_idx) = window.sections.iter().position(|section| section.button == button) {
                window.sections[sec_idx].collapsed = !window.sections[sec_idx].collapsed;
                unsafe {
                    InvalidateRect(button, ptr::null(), 0);
                }
            }
            reflow(&mut window, app);
            app.window = Some(window);
            return;
        }

        // Owner-drawn switch toggle.
        if let KindUi::Switch {
            button,
            on,
            off,
            checked,
        } = &mut window.controls[index].kind
        {
            *checked = !*checked;
            let new_val = if *checked { *on } else { *off };
            let btn = *button;
            let code = window.controls[index].code;
            unsafe {
                InvalidateRect(btn, ptr::null(), 0);
            }
            app.window = Some(window);
            if let Some(code) = code {
                app.set(code, new_val);
                app.show_saturation();
                apply_state(app);
                refresh_availability(app);
            }
            return;
        }

        // Slider with drag throttling.
        if let KindUi::Slider {
            track,
            value: value_label,
            factor,
            min,
            max,
        } = &window.controls[index].kind
        {
            let position = unsafe { SendMessageW(*track, TBM_GETPOS, 0, 0) } as i32;
            let vcp_val = (position.clamp(*min, *max) as u16) * (*factor).max(1);
            unsafe {
                SetWindowTextW(*value_label, win::wide(&vcp_val.to_string()).as_ptr());
            }
            let code = window.controls[index].code;
            let is_drag = notification == TB_THUMBTRACK;
            let now = Instant::now();
            let should_send = if let Some(code) = code {
                if is_drag {
                    match window.last_slider_send {
                        Some((last_code, last_time))
                            if last_code == code && now.duration_since(last_time).as_millis() < SLIDER_DRAG_INTERVAL_MS =>
                        {
                            false
                        }
                        _ => {
                            window.last_slider_send = Some((code, now));
                            true
                        }
                    }
                } else {
                    window.last_slider_send = Some((code, now));
                    true
                }
            } else {
                false
            };
            app.window = Some(window);
            if let Some(code) = code {
                app.local_change(code, vcp_val);
                if should_send {
                    app.set(code, vcp_val);
                    app.show_saturation();
                    if !is_drag {
                        apply_state(app);
                        refresh_availability(app);
                    }
                }
            }
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
                    apply_state(app);
                    refresh_availability(app);
                }
            }
            None => {}
        }
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
                    KindUi::Variant { selected, .. } => Some(*selected == 1),
                    _ => None,
                })
                .unwrap_or(false);
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
        _ => None,
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
            if let Some(&(width, height)) = pairs.get(selection) {
                let target_hz = app.current_mode.as_ref().map(|m| m.refresh).unwrap_or(0);
                // Preserve the current refresh rate if available, otherwise pick the highest refresh rate for that resolution.
                let mode = app
                    .modes
                    .iter()
                    .find(|m| m.width == width && m.height == height && m.refresh == target_hz)
                    .or_else(|| {
                        app.modes
                            .iter()
                            .rev()
                            .find(|m| m.width == width && m.height == height)
                    })
                    .cloned();
                if let Some(mode) = mode {
                    crate::app::apply_display_mode(app, mode);
                }
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
    let _ = app.worker.send(monitor::WorkerCmd::DydsWithAdaptiveSync(
        package.dyds_with_adaptive_sync(),
    ));
    if let Some(window) = app.window.as_mut() {
        window.firmware = selection;
    }
    show_firmware(app);
    refresh_availability(app);
}

fn show_firmware(app: &mut App) {
    let Some(window) = app.window.as_ref() else {
        return;
    };
    for control in &window.controls {
        if let KindUi::Firmware { combo } = &control.kind {
            unsafe {
                SendMessageW(*combo, CB_SETCURSEL, window.firmware, 0);
            }
        }
    }
}

// ───────────────────────────── owner-drawn controls & page painting ─────────────────────────────

fn draw_owner_item(window: &Window, item: &DRAWITEMSTRUCT) {
    if item.CtlType == ODT_LISTBOX {
        draw_nav_item(window, item);
        return;
    }
    if item.CtlType != ODT_BUTTON {
        return;
    }
    let hwnd = item.hwndItem;
    let key = hwnd as isize;

    // 1. Color swatch?
    if window.swatches.contains_key(&key) {
        draw_swatch(window, item);
        return;
    }
    // 2. Segmented toggle pill (`KindUi::ToggleGroup` or `KindUi::Variant`)?
    if let Some(&(control_idx, option_idx)) = window.toggles.get(&key) {
        draw_toggle_pill(window, item, control_idx, option_idx);
        return;
    }
    // 3. Switch or Section button?
    for control in &window.controls {
        match &control.kind {
            KindUi::Switch { button, checked, .. } if *button == hwnd => {
                draw_switch(window, item, *checked);
                return;
            }
            KindUi::Section { button } if *button == hwnd => {
                draw_section_header(window, item, hwnd);
                return;
            }
            _ => {}
        }
    }
    // 4. Star button?
    if let Some(&(_, is_fav)) = window.stars.get(&key) {
        draw_star_button(window, item, is_fav);
        return;
    }
    // 5. Standard rounded button (Toolbar / Action).
    draw_push_button(window, item, key);
}

fn draw_nav_item(window: &Window, item: &DRAWITEMSTRUCT) {
    let index = item.itemID as usize;
    let selected = (item.itemState & ODS_SELECTED) != 0;
    let dpi = window.dpi;
    let scale = |v: i32| win::scaled(v, dpi);
    unsafe {
        FillRect(item.hDC, &item.rcItem, window.brush_sidebar);
        if selected {
            let pill_brush = CreateSolidBrush(window.palette.card);
            let pill_pen = CreatePen(PS_SOLID, 1, window.palette.line);
            let old_pen = SelectObject(item.hDC, pill_pen as _);
            let old_brush = SelectObject(item.hDC, pill_brush as _);
            let r = scale(8);
            RoundRect(
                item.hDC,
                item.rcItem.left + scale(8),
                item.rcItem.top + scale(3),
                item.rcItem.right - scale(8),
                item.rcItem.bottom - scale(3),
                r,
                r,
            );
            SelectObject(item.hDC, old_pen);
            SelectObject(item.hDC, old_brush);
            DeleteObject(pill_pen as _);
            DeleteObject(pill_brush as _);

            // Accent indicator bar on the left edge of the active nav item.
            let bar_brush = CreateSolidBrush(window.palette.control_active);
            let bar_rect = RECT {
                left: item.rcItem.left + scale(10),
                top: item.rcItem.top + scale(10),
                right: item.rcItem.left + scale(14),
                bottom: item.rcItem.bottom - scale(10),
            };
            FillRect(item.hDC, &bar_rect, bar_brush);
            DeleteObject(bar_brush as _);
        }

        if let Some(label) = window.nav_labels.get(index) {
            let mut text_rect = item.rcItem;
            text_rect.left += scale(22);
            text_rect.right -= scale(10);
            SetBkMode(item.hDC, TRANSPARENT as i32);
            SetTextColor(item.hDC, window.palette.text);
            let font = if selected { window.bold } else { window.font };
            let old_font = SelectObject(item.hDC, font as _);
            let mut wide = win::wide(label);
            DrawTextW(
                item.hDC,
                wide.as_mut_ptr(),
                (wide.len().saturating_sub(1)) as i32,
                &mut text_rect,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
            );
            SelectObject(item.hDC, old_font);
        }
    }
}

fn draw_switch(window: &Window, item: &DRAWITEMSTRUCT, checked: bool) {
    let disabled = (item.itemState & ODS_DISABLED) != 0;
    let dpi = window.dpi;
    let scale = |v: i32| win::scaled(v, dpi);
    unsafe {
        FillRect(item.hDC, &item.rcItem, window.brush_card);
        let track_color = if checked {
            if disabled {
                window.palette.line
            } else {
                window.palette.control_active
            }
        } else {
            window.palette.control_bg
        };
        let border_color = if checked && !disabled {
            window.palette.control_active
        } else {
            window.palette.line
        };
        let track_brush = CreateSolidBrush(track_color);
        let track_pen = CreatePen(PS_SOLID, 1, border_color);
        let old_pen = SelectObject(item.hDC, track_pen as _);
        let old_brush = SelectObject(item.hDC, track_brush as _);

        let w = item.rcItem.right - item.rcItem.left;
        let h = item.rcItem.bottom - item.rcItem.top;
        RoundRect(
            item.hDC,
            item.rcItem.left,
            item.rcItem.top,
            item.rcItem.right,
            item.rcItem.bottom,
            h,
            h,
        );

        let pad = scale(3);
        let thumb_d = (h - pad * 2).max(4);
        let thumb_x = if checked {
            item.rcItem.left + w - pad - thumb_d
        } else {
            item.rcItem.left + pad
        };
        let thumb_y = item.rcItem.top + pad;
        let thumb_color = if disabled { 0x00B0_B0B0 } else { 0x00FF_FFFF };
        let thumb_brush = CreateSolidBrush(thumb_color);
        let thumb_pen = CreatePen(PS_SOLID, 1, thumb_color);
        SelectObject(item.hDC, thumb_pen as _);
        SelectObject(item.hDC, thumb_brush as _);
        Ellipse(
            item.hDC,
            thumb_x,
            thumb_y,
            thumb_x + thumb_d,
            thumb_y + thumb_d,
        );

        SelectObject(item.hDC, old_pen);
        SelectObject(item.hDC, old_brush);
        DeleteObject(thumb_pen as _);
        DeleteObject(thumb_brush as _);
        DeleteObject(track_pen as _);
        DeleteObject(track_brush as _);
    }
}

fn draw_toggle_pill(window: &Window, item: &DRAWITEMSTRUCT, control_idx: usize, option_idx: usize) {
    let disabled = (item.itemState & ODS_DISABLED) != 0;
    let (active, is_crosshair) = match &window.controls[control_idx].kind {
        KindUi::ToggleGroup {
            selected,
            is_crosshair,
            ..
        } => (*selected == option_idx, *is_crosshair),
        KindUi::Variant { selected, .. } => (*selected == option_idx, false),
        _ => (false, false),
    };
    let dpi = window.dpi;
    let r = win::scaled(6, dpi);
    unsafe {
        FillRect(item.hDC, &item.rcItem, window.brush_card);
        let fill_color = if active {
            if disabled {
                window.palette.line
            } else {
                window.palette.control_active
            }
        } else {
            window.palette.control_bg
        };
        let pen_color = if active && !disabled {
            window.palette.control_active
        } else {
            window.palette.line
        };
        let brush = CreateSolidBrush(fill_color);
        let pen = CreatePen(PS_SOLID, 1, pen_color);
        let old_pen = SelectObject(item.hDC, pen as _);
        let old_brush = SelectObject(item.hDC, brush as _);
        RoundRect(
            item.hDC,
            item.rcItem.left,
            item.rcItem.top,
            item.rcItem.right,
            item.rcItem.bottom,
            r,
            r,
        );
        SelectObject(item.hDC, old_pen);
        SelectObject(item.hDC, old_brush);
        DeleteObject(pen as _);
        DeleteObject(brush as _);

        let fg_color = if active && !disabled {
            0x00FF_FFFF
        } else if disabled {
            window.palette.muted
        } else {
            window.palette.text
        };

        if is_crosshair {
            draw_crosshair_icon(item.hDC, &item.rcItem, option_idx, fg_color, dpi);
            return;
        }

        if let Some(label) = window.buttons.get(&(item.hwndItem as isize)) {
            let mut rc = item.rcItem;
            SetBkMode(item.hDC, TRANSPARENT as i32);
            SetTextColor(item.hDC, fg_color);
            let font = if active { window.bold } else { window.font };
            let old_font = SelectObject(item.hDC, font as _);
            let mut wide = win::wide(label);
            DrawTextW(
                item.hDC,
                wide.as_mut_ptr(),
                (wide.len().saturating_sub(1)) as i32,
                &mut rc,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
            SelectObject(item.hDC, old_font);
        }
    }
}

/// Draws the 6 crosshair shapes matching `assets/crosshairs/crosshair_1..6.svg`.
fn draw_crosshair_icon(hdc: HDC, rc: &RECT, shape_idx: usize, color: u32, dpi: u32) {
    let cx = (rc.left + rc.right) / 2;
    let cy = (rc.top + rc.bottom) / 2;
    let s = win::scaled(8, dpi);
    let gap = win::scaled(3, dpi);
    unsafe {
        let pen = CreatePen(PS_SOLID, win::scaled(2, dpi).max(1), color);
        let brush = CreateSolidBrush(color);
        let old_pen = SelectObject(hdc, pen as _);
        let old_brush = SelectObject(hdc, GetStockObject(NULL_BRUSH) as _);

        match shape_idx {
            0 => {
                // 1: Full cross + center dot
                MoveToEx(hdc, cx - s, cy, ptr::null_mut());
                LineTo(hdc, cx + s + 1, cy);
                MoveToEx(hdc, cx, cy - s, ptr::null_mut());
                LineTo(hdc, cx, cy + s + 1);
            }
            1 => {
                // 2: 4-way gapped cross + center dot
                MoveToEx(hdc, cx - s, cy, ptr::null_mut());
                LineTo(hdc, cx - gap, cy);
                MoveToEx(hdc, cx + gap, cy, ptr::null_mut());
                LineTo(hdc, cx + s + 1, cy);
                MoveToEx(hdc, cx, cy - s, ptr::null_mut());
                LineTo(hdc, cx, cy - gap);
                MoveToEx(hdc, cx, cy + gap, ptr::null_mut());
                LineTo(hdc, cx, cy + s + 1);
                SelectObject(hdc, brush as _);
                Ellipse(hdc, cx - 1, cy - 1, cx + 2, cy + 2);
            }
            2 => {
                // 3: Circle + cross
                let r = win::scaled(6, dpi);
                Ellipse(hdc, cx - r, cy - r, cx + r, cy + r);
                MoveToEx(hdc, cx - s, cy, ptr::null_mut());
                LineTo(hdc, cx + s + 1, cy);
                MoveToEx(hdc, cx, cy - s, ptr::null_mut());
                LineTo(hdc, cx, cy + s + 1);
            }
            3 => {
                // 4: T-crosshair / chevron
                MoveToEx(hdc, cx - s, cy, ptr::null_mut());
                LineTo(hdc, cx - gap, cy);
                MoveToEx(hdc, cx + gap, cy, ptr::null_mut());
                LineTo(hdc, cx + s + 1, cy);
                MoveToEx(hdc, cx, cy + gap, ptr::null_mut());
                LineTo(hdc, cx, cy + s + 1);
                MoveToEx(hdc, cx - gap, cy, ptr::null_mut());
                LineTo(hdc, cx, cy - gap);
                LineTo(hdc, cx + gap + 1, cy + 1);
            }
            4 => {
                // 5: Circle + center dot
                let r = win::scaled(6, dpi);
                Ellipse(hdc, cx - r, cy - r, cx + r, cy + r);
                SelectObject(hdc, brush as _);
                Ellipse(hdc, cx - 2, cy - 2, cx + 2, cy + 2);
            }
            _ => {
                // 6: Bold center dot with corner ticks
                SelectObject(hdc, brush as _);
                let r = win::scaled(3, dpi);
                Ellipse(hdc, cx - r, cy - r, cx + r, cy + r);
            }
        }

        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_brush);
        DeleteObject(pen as _);
        DeleteObject(brush as _);
    }
}

fn draw_section_header(window: &Window, item: &DRAWITEMSTRUCT, button: HWND) {
    let Some(section) = window.sections.iter().find(|s| s.button == button) else {
        return;
    };
    unsafe {
        FillRect(item.hDC, &item.rcItem, window.brush_card);
        let arrow = if section.collapsed { "\u{25b8}" } else { "\u{25be}" };
        let label = format!("{arrow}   {}", section.title);
        let mut rc = item.rcItem;
        SetBkMode(item.hDC, TRANSPARENT as i32);
        SetTextColor(item.hDC, window.palette.text);
        let old_font = SelectObject(item.hDC, window.bold as _);
        let mut wide = win::wide(&label);
        DrawTextW(
            item.hDC,
            wide.as_mut_ptr(),
            (wide.len().saturating_sub(1)) as i32,
            &mut rc,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
        SelectObject(item.hDC, old_font);
    }
}

fn draw_star_button(window: &Window, item: &DRAWITEMSTRUCT, is_fav: bool) {
    unsafe {
        FillRect(item.hDC, &item.rcItem, window.brush_card);
        let mut rc = item.rcItem;
        SetBkMode(item.hDC, TRANSPARENT as i32);
        let color = if is_fav {
            0x0014_A8F5 // Gold/Amber in BGR
        } else {
            window.palette.muted
        };
        SetTextColor(item.hDC, color);
        let old_font = SelectObject(item.hDC, window.bold as _);
        let glyph = if is_fav { "\u{2605}" } else { "\u{2606}" };
        let mut wide = win::wide(glyph);
        DrawTextW(
            item.hDC,
            wide.as_mut_ptr(),
            (wide.len().saturating_sub(1)) as i32,
            &mut rc,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
        SelectObject(item.hDC, old_font);
    }
}

fn draw_push_button(window: &Window, item: &DRAWITEMSTRUCT, key: isize) {
    let disabled = (item.itemState & ODS_DISABLED) != 0;
    let pressed = (item.itemState & ODS_SELECTED) != 0;
    let is_toolbar = unsafe { GetParent(item.hwndItem) } == window.hwnd;
    let dpi = window.dpi;
    let r = win::scaled(6, dpi);
    unsafe {
        FillRect(
            item.hDC,
            &item.rcItem,
            if is_toolbar {
                window.brush_sidebar
            } else {
                window.brush_card
            },
        );
        let fill_color = if pressed {
            window.palette.line
        } else if is_toolbar {
            window.palette.card
        } else {
            window.palette.control_bg
        };
        let brush = CreateSolidBrush(fill_color);
        let pen = CreatePen(PS_SOLID, 1, window.palette.line);
        let old_pen = SelectObject(item.hDC, pen as _);
        let old_brush = SelectObject(item.hDC, brush as _);
        RoundRect(
            item.hDC,
            item.rcItem.left,
            item.rcItem.top,
            item.rcItem.right,
            item.rcItem.bottom,
            r,
            r,
        );
        SelectObject(item.hDC, old_pen);
        SelectObject(item.hDC, old_brush);
        DeleteObject(pen as _);
        DeleteObject(brush as _);

        if let Some(label) = window.buttons.get(&key) {
            let mut rc = item.rcItem;
            SetBkMode(item.hDC, TRANSPARENT as i32);
            SetTextColor(
                item.hDC,
                if disabled {
                    window.palette.muted
                } else {
                    window.palette.text
                },
            );
            let old_font = SelectObject(item.hDC, window.font as _);
            let mut wide = win::wide(label);
            DrawTextW(
                item.hDC,
                wide.as_mut_ptr(),
                (wide.len().saturating_sub(1)) as i32,
                &mut rc,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
            );
            SelectObject(item.hDC, old_font);
        }
    }
}

fn draw_swatch(window: &Window, item: &DRAWITEMSTRUCT) {
    let Some((control, option)) = window.swatches.get(&(item.hwndItem as isize)).copied() else {
        return;
    };
    let KindUi::Swatches {
        colors, selected, ..
    } = &window.controls[control].kind
    else {
        return;
    };
    let Some(&rgb) = colors.get(option) else {
        return;
    };
    // Convert 0xRRGGBB to Win32 BGR 0x00BBGGRR.
    let bgr = ((rgb & 0xFF) << 16) | (rgb & 0xFF00) | ((rgb >> 16) & 0xFF);
    let disabled = (item.itemState & ODS_DISABLED) != 0;
    let active = *selected == option;
    let dpi = window.dpi;
    let r = win::scaled(6, dpi);
    unsafe {
        FillRect(item.hDC, &item.rcItem, window.brush_card);
        let fill = CreateSolidBrush(bgr);
        let border_w = if active {
            win::scaled(3, dpi).max(2)
        } else {
            1
        };
        let pen_color = if active {
            window.palette.control_active
        } else {
            window.palette.line
        };
        let pen = CreatePen(PS_SOLID, border_w, pen_color);
        let old_pen = SelectObject(item.hDC, pen as _);
        let old_brush = SelectObject(item.hDC, fill as _);
        RoundRect(
            item.hDC,
            item.rcItem.left + 1,
            item.rcItem.top + 1,
            item.rcItem.right - 1,
            item.rcItem.bottom - 1,
            r,
            r,
        );
        if active {
            // Draw inner contrasting ring so selection is clear on both white and dark swatches.
            let inner_pen = CreatePen(PS_SOLID, 1, 0x00FF_FFFF);
            SelectObject(item.hDC, inner_pen as _);
            SelectObject(item.hDC, GetStockObject(NULL_BRUSH) as _);
            let inset = border_w + 1;
            RoundRect(
                item.hDC,
                item.rcItem.left + inset,
                item.rcItem.top + inset,
                item.rcItem.right - inset,
                item.rcItem.bottom - inset,
                (r - 2).max(2),
                (r - 2).max(2),
            );
            DeleteObject(inner_pen as _);
        } else if disabled {
            // Diagonal slash on disabled swatch (e.g. Colorful in unsupported LED mode).
            let slash_pen = CreatePen(PS_SOLID, 1, window.palette.card);
            SelectObject(item.hDC, slash_pen as _);
            MoveToEx(item.hDC, item.rcItem.left + 4, item.rcItem.bottom - 4, ptr::null_mut());
            LineTo(item.hDC, item.rcItem.right - 4, item.rcItem.top + 4);
            DeleteObject(slash_pen as _);
        }
        SelectObject(item.hDC, old_pen);
        SelectObject(item.hDC, old_brush);
        DeleteObject(pen as _);
        DeleteObject(fill as _);
    }
}

/// Double-buffered paint of a `TitanPage` window: draws card containers, row dividers, and the Overview hero monitor.
fn paint_page(window: &Window, page_window: HWND) {
    let Some(page_ui) = window.pages.iter().find(|p| p.window == page_window) else {
        return;
    };
    let is_overview = window.pages.first().is_some_and(|p| p.window == page_window);
    let dpi = window.dpi;
    let scale = |v: i32| win::scaled(v, dpi);
    let margin = scale(MARGIN);
    let scroll = page_ui.scroll;

    unsafe {
        let mut ps: PAINTSTRUCT = std::mem::zeroed();
        let hdc = BeginPaint(page_window, &mut ps);
        let mut client: RECT = std::mem::zeroed();
        GetClientRect(page_window, &mut client);
        let w = client.right.max(1);
        let h = client.bottom.max(1);

        let mem_dc = CreateCompatibleDC(hdc);
        let mem_bmp = CreateCompatibleBitmap(hdc, w, h);
        let old_bmp = SelectObject(mem_dc, mem_bmp as _);

        FillRect(mem_dc, &client, window.brush_bg);

        // Draw Overview monitor illustration on page 0.
        if is_overview {
            draw_overview_monitor(window, mem_dc, w, scroll);
        }

        let card_left = margin;
        let card_right = (w - margin).max(card_left + scale(320));
        let radius = scale(10);

        let card_pen = CreatePen(PS_SOLID, 1, window.palette.line);
        let old_pen = SelectObject(mem_dc, card_pen as _);
        let old_brush = SelectObject(mem_dc, window.brush_card as _);

        for card in &page_ui.cards {
            let top = card.top - scroll;
            let bottom = card.bottom - scroll;
            if bottom < 0 || top > h {
                continue;
            }
            RoundRect(mem_dc, card_left, top, card_right, bottom, radius, radius);
            for i in 0..card.divider_count {
                let div_y = card.dividers[i] - scroll;
                if div_y >= 0 && div_y <= h {
                    MoveToEx(mem_dc, card_left + scale(CARD_PAD_X), div_y, ptr::null_mut());
                    LineTo(mem_dc, card_right - scale(CARD_PAD_X), div_y);
                }
            }
        }

        SelectObject(mem_dc, old_pen);
        SelectObject(mem_dc, old_brush);
        DeleteObject(card_pen as _);

        BitBlt(hdc, 0, 0, w, h, mem_dc, 0, 0, SRCCOPY);
        SelectObject(mem_dc, old_bmp);
        DeleteObject(mem_bmp as _);
        DeleteDC(mem_dc);
        EndPaint(page_window, &ps);
    }
}

/// Draws a sleek vector illustration of the P275MV PLUS monitor centered at the top of the Overview page.
fn draw_overview_monitor(window: &Window, hdc: HDC, page_width: i32, scroll: i32) {
    let dpi = window.dpi;
    let scale = |v: i32| win::scaled(v, dpi);
    let mon_w = scale(210);
    let mon_h = scale(122);
    let x = (page_width - mon_w) / 2;
    let y = scale(18) - scroll;
    if y + mon_h + scale(30) < 0 {
        return;
    }
    unsafe {
        // Stand neck and base.
        let stand_brush = CreateSolidBrush(window.palette.muted);
        let stand_pen = CreatePen(PS_SOLID, 1, window.palette.line);
        let old_pen = SelectObject(hdc, stand_pen as _);
        let old_brush = SelectObject(hdc, stand_brush as _);
        let cx = page_width / 2;
        RoundRect(
            hdc,
            cx - scale(10),
            y + mon_h - scale(2),
            cx + scale(10),
            y + mon_h + scale(20),
            scale(4),
            scale(4),
        );
        RoundRect(
            hdc,
            cx - scale(46),
            y + mon_h + scale(16),
            cx + scale(46),
            y + mon_h + scale(24),
            scale(4),
            scale(4),
        );

        // Outer monitor bezel.
        let bezel_brush = CreateSolidBrush(0x0024_2220);
        let bezel_pen = CreatePen(PS_SOLID, scale(2).max(1), window.palette.line);
        SelectObject(hdc, bezel_pen as _);
        SelectObject(hdc, bezel_brush as _);
        RoundRect(hdc, x, y, x + mon_w, y + mon_h, scale(8), scale(8));

        // Inner display panel.
        let pad = scale(6);
        let screen_brush = CreateSolidBrush(window.palette.control_active);
        let screen_pen = CreatePen(PS_SOLID, 1, window.palette.control_active);
        SelectObject(hdc, screen_pen as _);
        SelectObject(hdc, screen_brush as _);
        RoundRect(
            hdc,
            x + pad,
            y + pad,
            x + mon_w - pad,
            y + mon_h - pad - scale(4),
            scale(4),
            scale(4),
        );

        // Subtle wallpaper horizon accent inside the screen.
        let accent_brush = CreateSolidBrush(0x0058_3218);
        let accent_rect = RECT {
            left: x + pad,
            top: y + mon_h / 2,
            right: x + mon_w - pad,
            bottom: y + mon_h - pad - scale(4),
        };
        FillRect(hdc, &accent_rect, accent_brush);

        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_brush);
        DeleteObject(accent_brush as _);
        DeleteObject(screen_pen as _);
        DeleteObject(screen_brush as _);
        DeleteObject(bezel_pen as _);
        DeleteObject(bezel_brush as _);
        DeleteObject(stand_pen as _);
        DeleteObject(stand_brush as _);
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
            let width = (suggested.right - suggested.left).max(win::scaled(960, dpi));
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
            info.ptMinTrackSize.x = win::scaled(820, dpi);
            info.ptMinTrackSize.y = win::scaled(500, dpi);
            0
        }
        WM_MEASUREITEM => {
            let mis = unsafe { &mut *(lparam as *mut MEASUREITEMSTRUCT) };
            if mis.CtlType == ODT_LISTBOX {
                let dpi = win::dpi_of(hwnd);
                mis.itemHeight = win::scaled(38, dpi) as u32;
                return 1;
            }
            0
        }
        WM_DRAWITEM => {
            if let Some(window) = app.window.as_ref() {
                let item = unsafe { &*(lparam as *const DRAWITEMSTRUCT) };
                draw_owner_item(window, item);
            }
            1
        }
        WM_COMMAND => {
            handle_command(app, lparam as HWND, ((wparam >> 16) & 0xFFFF) as u32);
            0
        }
        WM_MOUSEWHEEL => {
            if let Some(page_hwnd) = app
                .window
                .as_ref()
                .and_then(|w| w.pages.get(w.active))
                .map(|p| p.window)
            {
                let delta = ((wparam >> 16) & 0xFFFF) as u16 as i16 as i32;
                crate::app::handle_wheel(app, page_hwnd, delta);
            }
            0
        }
        WM_HSCROLL => {
            let track = lparam as HWND;
            if !track.is_null() {
                handle_command(app, track, (wparam as u32) & 0xFFFF);
            } else {
                crate::app::handle_page_scroll(app, hwnd, wparam);
            }
            0
        }
        WM_VSCROLL => {
            crate::app::handle_page_scroll(app, hwnd, wparam);
            0
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN | WM_CTLCOLORLISTBOX => {
            let palette = app
                .window
                .as_ref()
                .map(|w| w.palette)
                .unwrap_or_else(|| palette_for(app.settings.theme));
            let (_, brush_sidebar) = cached_brushes(&palette);
            unsafe {
                SetTextColor(wparam as _, palette.text);
                SetBkColor(wparam as _, palette.sidebar);
                SetBkMode(wparam as _, TRANSPARENT as i32);
                brush_sidebar as LRESULT
            }
        }
        WM_ERASEBKGND => unsafe {
            let palette = app
                .window
                .as_ref()
                .map(|w| w.palette)
                .unwrap_or_else(|| palette_for(app.settings.theme));
            let (_, brush_sidebar) = cached_brushes(&palette);
            fill_background(hwnd, wparam as _, brush_sidebar)
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
                handle_command(app, track, (wparam as u32) & 0xFFFF);
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
                draw_owner_item(window, item);
            }
            1
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN | WM_CTLCOLORLISTBOX => {
            let palette = app
                .window
                .as_ref()
                .map(|w| w.palette)
                .unwrap_or_else(|| palette_for(app.settings.theme));
            let (brush_card, _) = cached_brushes(&palette);
            let child = lparam as HWND;
            // Header & Hero labels sit directly on the page background (`palette.background`);
            // rows inside cards sit on `palette.card`.
            let on_page_bg = app
                .window
                .as_ref()
                .is_some_and(|w| {
                    w.rows.iter().any(|r| {
                        (r.is_header || r.is_hero) && r.elements.iter().any(|e| e.window == child)
                    })
                });
            unsafe {
                SetTextColor(wparam as _, palette.text);
                if on_page_bg {
                    SetBkColor(wparam as _, palette.background);
                    SetBkMode(wparam as _, TRANSPARENT as i32);
                    if let Some(w) = app.window.as_ref() {
                        return w.brush_bg as LRESULT;
                    }
                }
                SetBkColor(wparam as _, palette.card);
                SetBkMode(wparam as _, TRANSPARENT as i32);
                brush_card as LRESULT
            }
        }
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            if let Some(window) = app.window.as_ref() {
                paint_page(window, hwnd);
                return 0;
            }
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
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
        if !window.title_font.is_null() {
            DeleteObject(window.title_font as _);
        }
    }
    window.font = make_font(dpi, 14, false);
    window.bold = make_font(dpi, 14, true);
    window.title_font = make_font(dpi, 18, true);
    window.dpi = dpi;
    unsafe {
        for row in &window.rows {
            for element in &row.elements {
                let font = if row.is_header { window.bold } else { window.font };
                SendMessageW(element.window, WM_SETFONT, font as usize, 1);
            }
        }
        SendMessageW(window.nav, WM_SETFONT, window.font as usize, 1);
        SendMessageW(window.nav, LB_SETITEMHEIGHT, 0, win::scaled(38, dpi) as isize);
        SendMessageW(window.status, WM_SETFONT, window.font as usize, 1);
        for id in [ID_REFRESH, ID_PREFERENCES, ID_ABOUT, ID_LOG] {
            let button = GetDlgItem(window.hwnd, id as i32);
            SendMessageW(button, WM_SETFONT, window.font as usize, 1);
        }
    }
}

/// Scrolls a page by `request` (absolute position) using `ScrollWindowEx` without taking `app.window`.
pub fn set_scroll(app: &mut App, page_window: HWND, request: i32) {
    if let Some(window) = app.window.as_mut()
        && window.is_open()
    {
        scroll_page_fast(window, page_window, request);
    }
}

/// Current scroll position of a page.
pub fn scroll_of(app: &App, page_window: HWND) -> i32 {
    app.window
        .as_ref()
        .and_then(|window| {
            window
                .pages
                .iter()
                .find(|page| page.window == page_window)
                .map(|page| page.scroll)
        })
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
        if !window.title_font.is_null() {
            DeleteObject(window.title_font as _);
            window.title_font = ptr::null_mut();
        }
        if !window.brush_bg.is_null() {
            DeleteObject(window.brush_bg as _);
            window.brush_bg = ptr::null_mut();
        }
        if !window.brush_card.is_null() {
            DeleteObject(window.brush_card as _);
            window.brush_card = ptr::null_mut();
        }
        if !window.brush_sidebar.is_null() {
            DeleteObject(window.brush_sidebar as _);
            window.brush_sidebar = ptr::null_mut();
        }
    }
    let _ = POINT { x: 0, y: 0 };
}
