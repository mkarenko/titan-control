//! The settings window.
//!
//! The window is built from the declarative pages in [`rows`], created when the user opens it and destroyed when it is
//! closed, so none of its memory is kept while the application sits in the tray. Controls are Win32 children rendered
//! with a modern card-based layout, owner-drawn pill switches, segmented toggle groups, and double-buffered pages.

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    ffi::c_void,
    ptr,
    time::Instant,
};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreatePen, CreateSolidBrush,
    DeleteDC, DeleteObject, DrawTextW, Ellipse, EndPaint, FillRect, GetStockObject, IntersectClipRect, InvalidateRect,
    LineTo, MoveToEx, RoundRect, SelectClipRgn, SelectObject, SetBkColor, SetBkMode, SetTextColor, UpdateWindow,
    DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, HBITMAP, HBRUSH, HDC, HFONT, HPEN,
    NULL_BRUSH, OPAQUE, PAINTSTRUCT, PS_SOLID, SRCCOPY, TRANSPARENT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::{
    DRAWITEMSTRUCT, ICC_BAR_CLASSES, ICC_STANDARD_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx,
    MEASUREITEMSTRUCT, ODS_DISABLED, ODS_SELECTED, ODT_BUTTON, ODT_LISTBOX, SetScrollInfo, TBM_SETPAGESIZE,
    TBM_SETPOS, TBM_SETRANGE, TBS_HORZ, TBS_NOTICKS, TTM_SETMAXTIPWIDTH, TTS_ALWAYSTIP, TTS_NOPREFIX,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetAsyncKeyState, IsWindowEnabled, SetFocus, VK_CONTROL, VK_ESCAPE, VK_LWIN, VK_MENU, VK_RWIN,
    VK_SHIFT,
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
#[allow(non_upper_case_globals)]
const ES_LEFT: u32 = 0x0000;
#[allow(non_upper_case_globals)]
const ES_AUTOHSCROLL: u32 = 0x0080;
#[allow(non_upper_case_globals)]
const EN_SETFOCUS: u32 = 0x0100;
#[allow(non_upper_case_globals)]
const EN_KILLFOCUS: u32 = 0x0200;
#[allow(non_upper_case_globals)]
const EN_CHANGE: u32 = 0x0300;
#[allow(non_upper_case_globals)]
const EM_SETCUEBANNER: u32 = 0x1501;

use crate::{
    app::{App, SystemKind, TIMER_FAV_KEY},
    i18n::{AppLang, tr, usage_minutes},
    monitor,
    settings::StepModifier,
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
const ID_SEARCH: u16 = 105;
const ID_SEARCH_CLEAR: u16 = 106;

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
    /// Parent non-sub row above a `sub(...)` row so search keeps context together.
    parent_row: Option<usize>,
    /// Lowercase searchable text (label, parent label, section, header, page, options, tooltip).
    search_text: String,
    visible_in_search: bool,
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

/// Reusable GDI objects and page off-screen backbuffer so painting never allocates per frame.
struct DrawCache {
    brushes: HashMap<u32, HBRUSH>,
    pens: HashMap<(u32, i32), HPEN>,
    back_dc: HDC,
    back_bmp: HBITMAP,
    stock_bmp: *mut c_void,
    back_w: i32,
    back_h: i32,
}

impl DrawCache {
    fn new() -> Self {
        Self {
            brushes: HashMap::new(),
            pens: HashMap::new(),
            back_dc: ptr::null_mut(),
            back_bmp: ptr::null_mut(),
            stock_bmp: ptr::null_mut(),
            back_w: 0,
            back_h: 0,
        }
    }

    unsafe fn brush(&mut self, color: u32) -> HBRUSH {
        *self
            .brushes
            .entry(color)
            .or_insert_with(|| unsafe { CreateSolidBrush(color) })
    }

    unsafe fn pen(&mut self, color: u32, width: i32) -> HPEN {
        *self
            .pens
            .entry((color, width))
            .or_insert_with(|| unsafe { CreatePen(PS_SOLID, width, color) })
    }

    unsafe fn backbuffer(&mut self, hdc: HDC, w: i32, h: i32) -> (HDC, bool) {
        unsafe {
            if self.back_dc.is_null() {
                self.back_dc = CreateCompatibleDC(hdc);
            }
            if !self.back_dc.is_null() && (self.back_bmp.is_null() || w > self.back_w || h > self.back_h) {
                let alloc_w = w.max(self.back_w);
                let alloc_h = h.max(self.back_h);
                let new_bmp = CreateCompatibleBitmap(hdc, alloc_w, alloc_h);
                if !new_bmp.is_null() {
                    let prev = SelectObject(self.back_dc, new_bmp as _);
                    if self.stock_bmp.is_null() {
                        self.stock_bmp = prev;
                    }
                    if !self.back_bmp.is_null() {
                        DeleteObject(self.back_bmp as _);
                    }
                    self.back_bmp = new_bmp;
                    self.back_w = alloc_w;
                    self.back_h = alloc_h;
                }
            }
            (self.back_dc, !self.back_dc.is_null() && !self.back_bmp.is_null())
        }
    }

    unsafe fn release(&mut self) {
        unsafe {
            for (_, b) in self.brushes.drain() {
                if !b.is_null() {
                    DeleteObject(b as _);
                }
            }
            for (_, p) in self.pens.drain() {
                if !p.is_null() {
                    DeleteObject(p as _);
                }
            }
            if !self.back_dc.is_null() {
                if !self.stock_bmp.is_null() {
                    SelectObject(self.back_dc, self.stock_bmp);
                    self.stock_bmp = ptr::null_mut();
                }
                DeleteDC(self.back_dc);
                self.back_dc = ptr::null_mut();
            }
            if !self.back_bmp.is_null() {
                DeleteObject(self.back_bmp as _);
                self.back_bmp = ptr::null_mut();
            }
            self.back_w = 0;
            self.back_h = 0;
        }
    }
}

pub struct Window {
    pub hwnd: HWND,
    font: HFONT,
    bold: HFONT,
    title_font: HFONT,
    brush_bg: HBRUSH,
    brush_card: HBRUSH,
    brush_sidebar: HBRUSH,
    draw: DrawCache,
    tooltip: HWND,
    palette: Palette,
    status: HWND,
    nav: HWND,
    nav_labels: Vec<String>,
    pages: Vec<PageUi>,
    search_box: HWND,
    search_clear: HWND,
    search_page: PageUi,
    search_query: String,
    search_focused: bool,
    empty_search_text: String,
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
    /// Header and hero `STATIC` HWNDs that sit directly on the page background.
    page_bg_labels: HashSet<isize>,
    /// Cached label strings (`status` and `KindUi::Text`) to skip unchanged `SetWindowTextW` calls.
    text_cache: HashMap<isize, String>,
    /// Cached slider display values keyed by trackbar HWND to skip redundant label updates.
    slider_cache: HashMap<isize, u16>,
    /// Cached `(modes_len, current_mode, dpi)` for skipping redundant system combo rebuilds.
    last_system_key: Option<(usize, Option<(u32, u32, u32)>, u32)>,
    fav_key_held: bool,
    active: usize,
    applying: bool,
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

    fn in_search(&self) -> bool {
        !self.search_query.is_empty()
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
        app.active_page = window.active;
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
    let in_search = window.in_search();
    let active_page = window.active;
    for row in &window.rows {
        let matches_view = if in_search { row.visible_in_search } else { row.page == active_page };
        if !matches_view {
            continue;
        }
        let Some(star_hwnd) = row.star else {
            continue;
        };
        let hidden_by_section = !in_search
            && row
                .section
                .is_some_and(|sec| window.sections.get(sec).is_some_and(|s| s.collapsed));
        if hidden_by_section {
            continue;
        }
        let is_fav = window.stars.get(&(star_hwnd as isize)).is_some_and(|(_, fav)| *fav);
        set_visible(star_hwnd, held || is_fav);
    }
}

/// Shows or hides a control based on its own `WS_VISIBLE` style bit (unlike `IsWindowVisible`,
/// which returns false whenever the parent page or settings window is currently hidden).
fn set_visible(hwnd: HWND, visible: bool) {
    unsafe {
        let style_visible = (GetWindowLongW(hwnd, GWL_STYLE) as u32 & WS_VISIBLE) != 0;
        if style_visible != visible {
            ShowWindow(hwnd, if visible { SW_SHOW } else { SW_HIDE });
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
        draw: DrawCache::new(),
        tooltip: ptr::null_mut(),
        palette,
        status: ptr::null_mut(),
        nav: ptr::null_mut(),
        nav_labels: Vec::new(),
        pages: Vec::new(),
        search_box: ptr::null_mut(),
        search_clear: ptr::null_mut(),
        search_page: PageUi {
            window: ptr::null_mut(),
            scroll: 0,
            content_height: 0,
            cards: Vec::new(),
        },
        search_query: String::new(),
        search_focused: false,
        empty_search_text: app.tr("Brak pasujących ustawień"),
        rows: Vec::new(),
        controls: Vec::new(),
        sections: Vec::new(),
        swatches: HashMap::new(),
        toggles: HashMap::new(),
        buttons: HashMap::new(),
        stars: HashMap::new(),
        page_bg_labels: HashSet::new(),
        text_cache: HashMap::new(),
        slider_cache: HashMap::new(),
        last_system_key: None,
        fav_key_held: false,
        active: 0,
        applying: false,
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

        let classes: [(&str, WNDPROC, u32); 2] = [
            ("TitanControlSettings", Some(settings_proc), CS_HREDRAW | CS_VREDRAW),
            ("TitanPage", Some(page_proc), 0),
        ];
        for (name, proc_, style) in classes {
            let class_name = win::wide(name);
            let mut class: WNDCLASSEXW = std::mem::zeroed();
            class.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
            class.style = style;
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

        window.search_box = CreateWindowExW(
            0,
            win::wide("EDIT").as_ptr(),
            ptr::null(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_LEFT | ES_AUTOHSCROLL,
            0,
            0,
            10,
            10,
            hwnd,
            ID_SEARCH as usize as *mut c_void,
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(window.search_box, WM_SETFONT, window.font as usize, 1);
        let hint = win::wide(&tr(&lang, "Szukaj..."));
        SendMessageW(window.search_box, EM_SETCUEBANNER, 1, hint.as_ptr() as isize);
        SetWindowSubclass(window.search_box, search_edit_proc, 1, 0);

        window.search_clear = CreateWindowExW(
            0,
            win::wide("BUTTON").as_ptr(),
            win::wide("\u{00d7}").as_ptr(),
            WS_CHILD | BS_OWNERDRAW,
            0,
            0,
            10,
            10,
            hwnd,
            ID_SEARCH_CLEAR as usize as *mut c_void,
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );
        SendMessageW(window.search_clear, WM_SETFONT, window.font as usize, 1);

        window.search_page.window = CreateWindowExW(
            0,
            win::wide("TitanPage").as_ptr(),
            ptr::null(),
            WS_CHILD | WS_VSCROLL | WS_CLIPCHILDREN,
            0,
            0,
            10,
            10,
            hwnd,
            usize::MAX as *mut c_void,
            GetModuleHandleW(ptr::null()),
            ptr::null_mut(),
        );

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
        SendMessageW(window.tooltip, TTM_SETMAXTIPWIDTH, 0, win::scaled(360, dpi) as isize);
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

    for row in &window.rows {
        if row.is_header || row.is_hero {
            for element in &row.elements {
                window.page_bg_labels.insert(element.window as isize);
            }
        }
    }

    let page = std::env::var("TITAN_UI_PAGE")
        .ok()
        .and_then(|index| index.parse::<usize>().ok())
        .unwrap_or(app.active_page)
        .min(window.pages.len().saturating_sub(1));
    window.active = page;
    app.active_page = page;
    unsafe {
        SendMessageW(window.nav, LB_SETCURSEL, page, 0);
    }
    layout(window, app);
    show_page(window, page);
    theme_children(window);
}

/// Applies modern Explorer / DarkMode_Explorer themes to child controls that render via UxTheme.
fn theme_children(window: &Window) {
    let dark = window.palette.background == DARK.background;
    unsafe {
        win::set_control_theme(window.nav, dark);
        win::set_control_theme(window.search_box, dark);
        win::set_control_theme(window.tooltip, dark);
        win::set_control_theme(window.search_page.window, dark);
        for page in &window.pages {
            win::set_control_theme(page.window, dark);
            let mut child = GetWindow(page.window, GW_CHILD);
            while !child.is_null() {
                let mut class_buf = [0u16; 32];
                let len = GetClassNameW(child, class_buf.as_mut_ptr(), class_buf.len() as i32) as usize;
                let class_name = String::from_utf16_lossy(&class_buf[..len]);
                // Labels (`STATIC`) and owner-drawn `BUTTON`s are painted in `WM_CTLCOLOR*`/`WM_DRAWITEM`.
                if class_name.eq_ignore_ascii_case("ComboBox") {
                    win::set_combo_theme(child, dark);
                } else if class_name.eq_ignore_ascii_case("msctls_trackbar32") {
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
    let button_width = scale(BUTTON_WIDTH);
    let swatch = scale(SWATCH_SIZE);
    let swatch_gap = scale(SWATCH_GAP);
    let indent = scale(INDENT);
    let mut section: Option<usize> = None;
    let page_title = tr(&lang, spec.title).to_lowercase();
    let mut current_header = String::new();
    let mut current_section = String::new();
    let mut last_parent: Option<(usize, String)> = None;

    for spec_row in &spec.rows {
        let mut elements: Vec<Element> = Vec::new();
        let row_label = tr(&lang, spec_row.label);
        let sub_offset = if spec_row.sub { indent } else { 0 };

        match &spec_row.kind {
            Kind::Header => {
                section = None;
                current_header = row_label.to_lowercase();
                current_section.clear();
                last_parent = None;
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
                    parent_row: None,
                    search_text: String::new(),
                    visible_in_search: false,
                    elements,
                    star: None,
                });
                continue;
            }
            Kind::Section => {
                current_section = row_label.to_lowercase();
                last_parent = None;
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
                    parent_row: None,
                    search_text: current_section.clone(),
                    visible_in_search: false,
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
                parent_row: None,
                search_text: String::new(),
                visible_in_search: false,
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
            set_visible(button, false);
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
        let parent_row = if spec_row.sub {
            last_parent.as_ref().map(|(idx, _)| *idx)
        } else {
            None
        };
        let parent_label = if spec_row.sub {
            last_parent.as_ref().map(|(_, title)| title.as_str()).unwrap_or("")
        } else {
            ""
        };
        let mut search_text = format!(
            "{row_label} {parent_label} {current_section} {current_header} {page_title}"
        );
        if let Some(tip) = spec_row.tooltip {
            search_text.push(' ');
            search_text.push_str(&tr(&lang, tip));
        }
        match &spec_row.kind {
            Kind::ToggleGroup { options, .. } | Kind::Choice { options, .. } | Kind::Swatches { options, .. } => {
                for option in options.iter() {
                    search_text.push(' ');
                    search_text.push_str(&tr(&lang, option));
                }
            }
            Kind::PictureMode => {
                for name in monitor::PICTURE_MODE_NAMES {
                    search_text.push(' ');
                    search_text.push_str(name);
                }
            }
            _ => {}
        }
        let aliases = rows::search_aliases(spec_row);
        if !aliases.is_empty() {
            search_text.push(' ');
            search_text.push_str(aliases);
        }
        let search_text = search_text.to_lowercase();
        let row_index = window.rows.len();
        if !spec_row.sub {
            last_parent = Some((row_index, row_label));
        }
        window.controls.push(Control { kind, code: spec_row.code });
        window.rows.push(RowUi {
            page,
            height: scale(ROW_HEIGHT),
            is_header: false,
            is_hero: false,
            section,
            parent_row,
            search_text,
            visible_in_search: false,
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

/// Subclass procedure for the sidebar search box: `Escape` clears the filter and moves focus to the nav list.
unsafe extern "system" fn search_edit_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if msg == WM_KEYDOWN && wparam == VK_ESCAPE as usize {
        unsafe {
            let root = GetParent(hwnd);
            if !root.is_null() {
                let clear = GetDlgItem(root, ID_SEARCH_CLEAR as i32);
                SendMessageW(root, WM_COMMAND, ID_SEARCH_CLEAR as usize, clear as isize);
                let nav = GetDlgItem(root, ID_NAV as i32);
                if !nav.is_null() {
                    SetFocus(nav);
                }
            }
        }
        return 0;
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

fn search_pill_rect(dpi: u32) -> RECT {
    let nav = win::scaled(NAV_WIDTH, dpi);
    let pad = win::scaled(10, dpi);
    RECT {
        left: pad,
        top: pad,
        right: nav - pad,
        bottom: win::scaled(38, dpi),
    }
}

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
    let pill = search_pill_rect(dpi);
    let clear_size = win::scaled(20, dpi);
    let edit_left = pill.left + win::scaled(8, dpi);
    let edit_right = pill.right - clear_size - win::scaled(6, dpi);
    unsafe {
        let mut x = nav + win::scaled(MARGIN, dpi);
        for id in [ID_REFRESH, ID_PREFERENCES, ID_ABOUT, ID_LOG] {
            let button = GetDlgItem(hwnd, id as i32);
            let button_width = win::scaled(132, dpi);
            MoveWindow(button, x, win::scaled(10, dpi), button_width, win::scaled(28, dpi), 1);
            x += button_width + win::scaled(8, dpi);
        }
        MoveWindow(
            window.search_box,
            edit_left,
            pill.top + win::scaled(5, dpi),
            (edit_right - edit_left).max(10),
            win::scaled(18, dpi),
            1,
        );
        MoveWindow(
            window.search_clear,
            pill.right - clear_size - win::scaled(3, dpi),
            pill.top + win::scaled(4, dpi),
            clear_size,
            clear_size,
            1,
        );
        MoveWindow(
            window.status,
            win::scaled(16, dpi),
            (height - status + win::scaled(6, dpi)).max(0),
            (width - win::scaled(32, dpi)).max(0),
            win::scaled(20, dpi),
            1,
        );
        MoveWindow(window.nav, 0, toolbar, nav, (height - toolbar - status).max(0), 1);
        MoveWindow(
            window.search_page.window,
            nav,
            toolbar,
            (width - nav).max(0),
            (height - toolbar - status).max(0),
            1,
        );
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

/// Keeps each row's Win32 child controls parented to the currently visible container (`search_page` or `pages[row.page]`).
fn sync_row_parents(window: &Window) {
    let in_search = window.in_search();
    for row in &window.rows {
        let target = if in_search && row.visible_in_search {
            window.search_page.window
        } else {
            window.pages[row.page].window
        };
        if target.is_null() {
            continue;
        }
        for element in &row.elements {
            unsafe {
                if GetParent(element.window) != target {
                    SetParent(element.window, target);
                }
            }
        }
    }
}

/// Positions the controls of the active view (`pages[active]` or `search_page`) and computes card rectangles for `WM_PAINT`.
fn reflow(window: &mut Window, _app: &mut App) {
    let in_search = window.in_search();
    let page_index = window.active;
    if !in_search && page_index >= window.pages.len() {
        return;
    }
    sync_row_parents(window);
    let dpi = window.dpi;
    let margin = win::scaled(MARGIN, dpi);
    let page_window = if in_search {
        window.search_page.window
    } else {
        window.pages[page_index].window
    };
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

    for row in &window.rows {
        let in_view = if in_search {
            row.visible_in_search
        } else {
            row.page == page_index
        };
        if !in_view {
            continue;
        }
        let hidden = !in_search
            && row
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
    let page_ui = if in_search {
        &mut window.search_page
    } else {
        &mut window.pages[page_index]
    };
    page_ui.content_height = content;
    page_ui.cards = cards;
    let max_scroll = (content - viewport).max(0);
    let scroll = page_ui.scroll.clamp(0, max_scroll);
    page_ui.scroll = scroll;

    // Second pass: batch-position child HWNDs on the active view using BeginDeferWindowPos.
    let mut y = margin / 2;
    let elem_count: usize = window
        .rows
        .iter()
        .filter(|row| if in_search { row.visible_in_search } else { row.page == page_index })
        .map(|row| row.elements.len())
        .sum();
    let hdwp = unsafe { BeginDeferWindowPos(elem_count.max(1) as i32) };
    let mut current_hdwp = hdwp;

    // Calculate right-shift offset so right-aligned controls sit cleanly near the right edge of the card.
    let right_pad = win::scaled(CARD_PAD_X, dpi);
    let target_control_right = card_right - right_pad;

    for row in &window.rows {
        let in_view = if in_search {
            row.visible_in_search
        } else {
            row.page == page_index
        };
        if !in_view {
            continue;
        }
        let hidden = !in_search
            && row
                .section
                .is_some_and(|sec| window.sections.get(sec).is_some_and(|s| s.collapsed));
        if hidden {
            for element in &row.elements {
                set_visible(element.window, false);
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
            let (elem_x, elem_w, should_show) = if Some(element.window) == row.star {
                let is_fav = window.stars.get(&(element.window as isize)).is_some_and(|(_, fav)| *fav);
                (element.x, element.width, window.fav_key_held || is_fav)
            } else if row.is_hero {
                let hero_w = (card_right - margin).max(win::scaled(280, dpi));
                (margin, hero_w, true)
            } else if element.right_aligned {
                (element.x + shift_x, element.width, true)
            } else {
                (element.x, element.width, true)
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
            }
            set_visible(element.window, should_show);
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
    let in_search = window.in_search();
    set_visible(window.search_page.window, in_search);
    for (position, page) in window.pages.iter().enumerate() {
        set_visible(page.window, !in_search && position == index);
    }
}

fn read_search_box(hwnd: HWND) -> String {
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return String::new();
    }
    let mut buf = vec![0u16; len as usize + 1];
    let copied = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) } as usize;
    String::from_utf16_lossy(&buf[..copied])
}

/// Evaluates the sidebar search query across all pages and switches the active view accordingly.
fn update_search_filter(window: &mut Window, app: &mut App, raw_query: &str) {
    let normalized = raw_query.trim().to_lowercase();
    set_visible(window.search_clear, !raw_query.is_empty());
    if normalized == window.search_query {
        return;
    }
    window.search_query = normalized;
    window.search_page.scroll = 0;

    if window.search_query.is_empty() {
        for row in &mut window.rows {
            row.visible_in_search = false;
        }
        unsafe {
            SendMessageW(window.nav, LB_SETCURSEL, window.active, 0);
            InvalidateRect(window.nav, ptr::null(), 0);
        }
        let active = window.active;
        reflow(window, app);
        show_page(window, active);
        return;
    }

    let tokens: Vec<&str> = window.search_query.split_whitespace().collect();
    for row in &mut window.rows {
        row.visible_in_search = false;
    }

    let mut matched_sections: Vec<usize> = Vec::new();
    let mut matched_parents: Vec<usize> = Vec::new();
    for i in 0..window.rows.len() {
        let is_section = window
            .rows[i]
            .elements
            .first()
            .is_some_and(|e| window.sections.iter().any(|s| s.button == e.window));
        if window.rows[i].is_header || window.rows[i].is_hero || is_section {
            continue;
        }
        if tokens.iter().all(|t| window.rows[i].search_text.contains(t)) {
            window.rows[i].visible_in_search = true;
            if let Some(parent_idx) = window.rows[i].parent_row {
                matched_parents.push(parent_idx);
            }
            if let Some(sec_idx) = window.rows[i].section {
                matched_sections.push(sec_idx);
            }
        }
    }
    for parent_idx in matched_parents {
        window.rows[parent_idx].visible_in_search = true;
        if let Some(sec_idx) = window.rows[parent_idx].section {
            matched_sections.push(sec_idx);
        }
    }
    for sec_idx in matched_sections {
        if let Some(sec_btn) = window.sections.get(sec_idx).map(|s| s.button)
            && let Some(sec_row) = window
                .rows
                .iter_mut()
                .find(|r| r.elements.first().is_some_and(|e| e.window == sec_btn))
        {
            sec_row.visible_in_search = true;
        }
    }

    // Include each card header above at least one matched setting row.
    let mut current_header: Option<usize> = None;
    let mut header_page: usize = usize::MAX;
    for i in 0..window.rows.len() {
        if window.rows[i].page != header_page {
            current_header = None;
            header_page = window.rows[i].page;
        }
        if window.rows[i].is_header {
            current_header = Some(i);
        } else if window.rows[i].visible_in_search
            && let Some(h) = current_header
        {
            window.rows[h].visible_in_search = true;
        }
    }

    unsafe {
        SendMessageW(window.nav, LB_SETCURSEL, usize::MAX, 0);
        InvalidateRect(window.nav, ptr::null(), 0);
    }
    let active = window.active;
    reflow(window, app);
    show_page(window, active);
}

fn clear_search(window: &mut Window, app: &mut App) {
    if window.search_query.is_empty() && read_search_box(window.search_box).is_empty() {
        return;
    }
    let was_applying = window.applying;
    window.applying = true;
    unsafe {
        SetWindowTextW(window.search_box, win::wide("").as_ptr());
    }
    window.applying = was_applying;
    update_search_filter(window, app, "");
}

/// Scrolls the active view (`pages[active]` or `search_page`) using `ScrollWindowEx`.
fn scroll_page_fast(window: &mut Window, page_window: HWND, request: i32) {
    let mut client: RECT = unsafe { std::mem::zeroed() };
    unsafe {
        GetClientRect(page_window, &mut client);
    }
    let viewport = client.bottom.max(0);
    let page_ui = if window.search_page.window == page_window {
        &mut window.search_page
    } else if let Some(page) = window.pages.iter_mut().find(|page| page.window == page_window) {
        page
    } else {
        return;
    };
    let content = page_ui.content_height;
    let max = (content - viewport).max(0);
    let new_scroll = request.clamp(0, max);
    let old_scroll = page_ui.scroll;
    let dy = old_scroll - new_scroll;
    if dy == 0 {
        return;
    }
    page_ui.scroll = new_scroll;
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
                    if SendMessageW(*track, TBM_GETPOS, 0, 0) != position {
                        SendMessageW(*track, TBM_SETPOS, 1, position);
                    }
                }
                let track_key = *track as isize;
                if window.slider_cache.get(&track_key).copied() != Some(current) {
                    window.slider_cache.insert(track_key, current);
                    unsafe {
                        SetWindowTextW(*value, win::wide(&current.to_string()).as_ptr());
                    }
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
                        if SendMessageW(*combo, CB_GETCURSEL, 0, 0) != index as isize {
                            SendMessageW(*combo, CB_SETCURSEL, index, 0);
                        }
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
            KindUi::PictureMode { combo } => {
                if let Some(current) = values.get(&monitor::VCP_MODE).copied()
                    && let Some((mode, _)) = monitor::picture_mode_from_vcp(current)
                {
                    unsafe {
                        if SendMessageW(*combo, CB_GETCURSEL, 0, 0) != mode as isize {
                            SendMessageW(*combo, CB_SETCURSEL, mode, 0);
                        }
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
                set_text_cached(&mut window.text_cache, *label, &text);
            }
            KindUi::Section { .. } | KindUi::Action { .. } | KindUi::System { .. } => {}
        }
    }

    // Update favorite stars only when state or visibility actually changed.
    let active_page = window.active;
    for (button_key, (id, active_flag)) in window.stars.iter_mut() {
        let is_fav = if *id == "picture_mode" {
            active_mode_idx.is_some_and(|mode| app.settings.is_favorite_picture_mode(mode))
        } else {
            app.settings.is_favorite_tray_control(id)
        };
        if *active_flag != is_fav {
            *active_flag = is_fav;
            let button = *button_key as HWND;
            let glyph = if is_fav { "\u{2605}" } else { "\u{2606}" };
            window.buttons.insert(*button_key, glyph.to_string());
            unsafe {
                SetWindowTextW(button, win::wide(glyph).as_ptr());
                InvalidateRect(button, ptr::null(), 0);
            }
        }
    }
    let in_search = window.in_search();
    for row in &window.rows {
        let in_view = if in_search {
            row.visible_in_search
        } else {
            row.page == active_page
        };
        if !in_view {
            continue;
        }
        if let Some(star_hwnd) = row.star {
            let hidden = !in_search
                && row
                    .section
                    .is_some_and(|sec| window.sections.get(sec).is_some_and(|s| s.collapsed));
            let is_fav = window.stars.get(&(star_hwnd as isize)).is_some_and(|(_, fav)| *fav);
            let should_show = !hidden && (window.fav_key_held || is_fav);
            set_visible(star_hwnd, should_show);
        }
    }

    apply_system_combos(&mut window, app);
    update_status(&mut window, app, &language);
    window.applying = false;
    app.window = Some(window);
}

fn set_text_cached(cache: &mut HashMap<isize, String>, hwnd: HWND, text: &str) {
    let key = hwnd as isize;
    if cache.get(&key).is_some_and(|prev| prev == text) {
        return;
    }
    cache.insert(key, text.to_owned());
    unsafe {
        SetWindowTextW(hwnd, win::wide(text).as_ptr());
    }
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
    set_text_cached(&mut window.text_cache, window.status, &text);
}

fn apply_system_combos(window: &mut Window, app: &App) {
    let dpi = if window.hwnd.is_null() {
        win::system_dpi()
    } else {
        win::dpi_of(window.hwnd)
    };
    let key = (
        app.modes.len(),
        app.current_mode.as_ref().map(|m| (m.width, m.height, m.refresh)),
        dpi,
    );
    if window.last_system_key == Some(key) {
        return;
    }
    window.last_system_key = Some(key);

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
                set_enabled_if_changed(*hwnd, btn_ok);
            }
            continue;
        }

        for hwnd in control_windows(&control.kind) {
            set_enabled_if_changed(hwnd, enabled);
        }
    }
}

fn set_enabled_if_changed(hwnd: HWND, enabled: bool) {
    unsafe {
        if (IsWindowEnabled(hwnd) != 0) != enabled {
            EnableWindow(hwnd, i32::from(enabled));
            InvalidateRect(hwnd, ptr::null(), 0);
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
        KindUi::PictureMode { combo } => vec![*combo],
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
        if item == window.search_box {
            if notification == EN_CHANGE {
                let query = read_search_box(window.search_box);
                update_search_filter(&mut window, app, &query);
            } else if notification == EN_SETFOCUS || notification == EN_KILLFOCUS {
                window.search_focused = notification == EN_SETFOCUS;
                let pill = search_pill_rect(window.dpi);
                unsafe {
                    InvalidateRect(window.hwnd, &pill, 0);
                }
            }
            app.window = Some(window);
            return;
        }
        if item == window.search_clear {
            clear_search(&mut window, app);
            app.window = Some(window);
            return;
        }
        if item == window.nav {
            if notification == LBN_SELCHANGE {
                let index = unsafe { SendMessageW(window.nav, LB_GETCURSEL, 0, 0) } as usize;
                if index < window.pages.len() {
                    window.active = index;
                    app.active_page = index;
                    if window.in_search() {
                        clear_search(&mut window, app);
                    } else {
                        reflow(&mut window, app);
                        show_page(&mut window, index);
                    }
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
                                usize::try_from(unsafe { SendMessageW(*combo, CB_GETCURSEL, 0, 0) }).ok()
                            }
                            _ => None,
                        })
                        .or_else(|| {
                            app.get(monitor::VCP_MODE)
                                .and_then(monitor::picture_mode_from_vcp)
                                .map(|(mode, _)| mode)
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

// ───────────────────────────── owner-drawn controls & page painting ─────────────────────────────

fn draw_owner_item(window: &mut Window, item: &DRAWITEMSTRUCT) {
    if item.CtlType == ODT_LISTBOX {
        draw_nav_item(window, item);
        return;
    }
    if item.CtlType != ODT_BUTTON {
        return;
    }
    let hwnd = item.hwndItem;
    let key = hwnd as isize;

    if hwnd == window.search_clear {
        draw_search_clear_button(window, item);
        return;
    }

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
    enum Found {
        Switch(bool),
        Section,
    }
    let mut found = None;
    for control in &window.controls {
        match &control.kind {
            KindUi::Switch { button, checked, .. } if *button == hwnd => {
                found = Some(Found::Switch(*checked));
                break;
            }
            KindUi::Section { button } if *button == hwnd => {
                found = Some(Found::Section);
                break;
            }
            _ => {}
        }
    }
    match found {
        Some(Found::Switch(checked)) => {
            draw_switch(window, item, checked);
            return;
        }
        Some(Found::Section) => {
            draw_section_header(window, item, hwnd);
            return;
        }
        None => {}
    }
    // 4. Star button?
    if let Some(&(_, is_fav)) = window.stars.get(&key) {
        draw_star_button(window, item, is_fav);
        return;
    }
    // 5. Standard rounded button (Toolbar / Action).
    draw_push_button(window, item, key);
}

fn draw_nav_item(window: &mut Window, item: &DRAWITEMSTRUCT) {
    let index = item.itemID as usize;
    let selected = (item.itemState & ODS_SELECTED) != 0;
    let dpi = window.dpi;
    let scale = |v: i32| win::scaled(v, dpi);
    let palette = window.palette;
    unsafe {
        FillRect(item.hDC, &item.rcItem, window.brush_sidebar);
        if selected {
            let pill_brush = window.draw.brush(palette.card);
            let pill_pen = window.draw.pen(palette.line, 1);
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

            // Accent indicator bar on the left edge of the active nav item.
            let bar_brush = window.draw.brush(palette.control_active);
            let bar_rect = RECT {
                left: item.rcItem.left + scale(10),
                top: item.rcItem.top + scale(10),
                right: item.rcItem.left + scale(14),
                bottom: item.rcItem.bottom - scale(10),
            };
            FillRect(item.hDC, &bar_rect, bar_brush);
        }

        if let Some(label) = window.nav_labels.get(index) {
            let mut text_rect = item.rcItem;
            text_rect.left += scale(22);
            text_rect.right -= scale(10);
            SetBkMode(item.hDC, TRANSPARENT as i32);
            SetTextColor(item.hDC, palette.text);
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

fn draw_switch(window: &mut Window, item: &DRAWITEMSTRUCT, checked: bool) {
    let disabled = (item.itemState & ODS_DISABLED) != 0;
    let dpi = window.dpi;
    let scale = |v: i32| win::scaled(v, dpi);
    let palette = window.palette;
    unsafe {
        FillRect(item.hDC, &item.rcItem, window.brush_card);
        let track_color = if checked {
            if disabled { palette.line } else { palette.control_active }
        } else {
            palette.control_bg
        };
        let border_color = if checked && !disabled {
            palette.control_active
        } else {
            palette.line
        };
        let track_brush = window.draw.brush(track_color);
        let track_pen = window.draw.pen(border_color, 1);
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
        let thumb_brush = window.draw.brush(thumb_color);
        let thumb_pen = window.draw.pen(thumb_color, 1);
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
    }
}

fn draw_toggle_pill(window: &mut Window, item: &DRAWITEMSTRUCT, control_idx: usize, option_idx: usize) {
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
    let palette = window.palette;
    unsafe {
        FillRect(item.hDC, &item.rcItem, window.brush_card);
        let fill_color = if active {
            if disabled { palette.line } else { palette.control_active }
        } else {
            palette.control_bg
        };
        let pen_color = if active && !disabled {
            palette.control_active
        } else {
            palette.line
        };
        let brush = window.draw.brush(fill_color);
        let pen = window.draw.pen(pen_color, 1);
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

        let fg_color = if active && !disabled {
            0x00FF_FFFF
        } else if disabled {
            palette.muted
        } else {
            palette.text
        };

        if is_crosshair {
            draw_crosshair_icon(window, item.hDC, &item.rcItem, option_idx, fg_color, dpi);
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
fn draw_crosshair_icon(window: &mut Window, hdc: HDC, rc: &RECT, shape_idx: usize, color: u32, dpi: u32) {
    let cx = (rc.left + rc.right) / 2;
    let cy = (rc.top + rc.bottom) / 2;
    let s = win::scaled(8, dpi);
    let gap = win::scaled(3, dpi);
    let pen_w = win::scaled(2, dpi).max(1);
    unsafe {
        let pen = window.draw.pen(color, pen_w);
        let brush = window.draw.brush(color);
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

fn draw_search_clear_button(window: &Window, item: &DRAWITEMSTRUCT) {
    let pressed = (item.itemState & ODS_SELECTED) != 0;
    let palette = window.palette;
    unsafe {
        FillRect(item.hDC, &item.rcItem, window.brush_card);
        let mut rc = item.rcItem;
        SetBkMode(item.hDC, TRANSPARENT as i32);
        SetTextColor(item.hDC, if pressed { palette.text } else { palette.muted });
        let old_font = SelectObject(item.hDC, window.bold as _);
        let mut wide = win::wide("\u{00d7}");
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

fn draw_push_button(window: &mut Window, item: &DRAWITEMSTRUCT, key: isize) {
    let disabled = (item.itemState & ODS_DISABLED) != 0;
    let pressed = (item.itemState & ODS_SELECTED) != 0;
    let is_toolbar = unsafe { GetParent(item.hwndItem) } == window.hwnd;
    let dpi = window.dpi;
    let r = win::scaled(6, dpi);
    let palette = window.palette;
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
            palette.line
        } else if is_toolbar {
            palette.card
        } else {
            palette.control_bg
        };
        let brush = window.draw.brush(fill_color);
        let pen = window.draw.pen(palette.line, 1);
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

        if let Some(label) = window.buttons.get(&key) {
            let mut rc = item.rcItem;
            SetBkMode(item.hDC, TRANSPARENT as i32);
            SetTextColor(
                item.hDC,
                if disabled { palette.muted } else { palette.text },
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

fn draw_swatch(window: &mut Window, item: &DRAWITEMSTRUCT) {
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
    let palette = window.palette;
    unsafe {
        FillRect(item.hDC, &item.rcItem, window.brush_card);
        let fill = window.draw.brush(bgr);
        let border_w = if active { win::scaled(3, dpi).max(2) } else { 1 };
        let pen_color = if active {
            palette.control_active
        } else {
            palette.line
        };
        let pen = window.draw.pen(pen_color, border_w);
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
            let inner_pen = window.draw.pen(0x00FF_FFFF, 1);
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
        } else if disabled {
            // Diagonal slash on disabled swatch (e.g. Colorful in unsupported LED mode).
            let slash_pen = window.draw.pen(palette.card, 1);
            SelectObject(item.hDC, slash_pen as _);
            MoveToEx(item.hDC, item.rcItem.left + 4, item.rcItem.bottom - 4, ptr::null_mut());
            LineTo(item.hDC, item.rcItem.right - 4, item.rcItem.top + 4);
        }
        SelectObject(item.hDC, old_pen);
        SelectObject(item.hDC, old_brush);
    }
}

/// Double-buffered paint of a `TitanPage` window clipped to `ps.rcPaint` with a reusable backbuffer.
fn paint_page(window: &mut Window, page_window: HWND) {
    let is_search_page = window.search_page.window == page_window;
    let page_idx = if is_search_page {
        None
    } else {
        let Some(idx) = window.pages.iter().position(|p| p.window == page_window) else {
            return;
        };
        Some(idx)
    };
    let is_overview = page_idx == Some(0);
    let dpi = window.dpi;
    let scale = |v: i32| win::scaled(v, dpi);
    let margin = scale(MARGIN);
    let scroll = match page_idx {
        Some(idx) => window.pages[idx].scroll,
        None => window.search_page.scroll,
    };
    let palette = window.palette;

    unsafe {
        let mut ps: PAINTSTRUCT = std::mem::zeroed();
        let hdc = BeginPaint(page_window, &mut ps);
        let mut client: RECT = std::mem::zeroed();
        GetClientRect(page_window, &mut client);
        let w = client.right.max(1);
        let h = client.bottom.max(1);
        let dirty = RECT {
            left: ps.rcPaint.left.clamp(0, w),
            top: ps.rcPaint.top.clamp(0, h),
            right: ps.rcPaint.right.clamp(0, w),
            bottom: ps.rcPaint.bottom.clamp(0, h),
        };
        if dirty.right <= dirty.left || dirty.bottom <= dirty.top {
            EndPaint(page_window, &ps);
            return;
        }

        let (mem_dc, ok) = window.draw.backbuffer(hdc, w, h);
        if !ok {
            EndPaint(page_window, &ps);
            return;
        }

        IntersectClipRect(mem_dc, dirty.left, dirty.top, dirty.right, dirty.bottom);
        FillRect(mem_dc, &dirty, window.brush_bg);

        // Draw Overview monitor illustration on page 0.
        if is_overview {
            draw_overview_monitor(window, mem_dc, w, scroll, &dirty);
        }

        let card_left = margin;
        let card_right = (w - margin).max(card_left + scale(320));
        let radius = scale(10);

        let cards = match page_idx {
            Some(idx) => &window.pages[idx].cards,
            None => &window.search_page.cards,
        };

        if is_search_page && cards.is_empty() && !window.search_query.is_empty() {
            let mut rc = RECT {
                left: margin,
                top: scale(48),
                right: w - margin,
                bottom: scale(96),
            };
            SetBkMode(mem_dc, TRANSPARENT as i32);
            SetTextColor(mem_dc, palette.muted);
            let old_font = SelectObject(mem_dc, window.font as _);
            let mut wide = win::wide(&window.empty_search_text);
            DrawTextW(
                mem_dc,
                wide.as_mut_ptr(),
                (wide.len().saturating_sub(1)) as i32,
                &mut rc,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
            SelectObject(mem_dc, old_font);
        } else {
            let card_pen = window.draw.pen(palette.line, 1);
            let old_pen = SelectObject(mem_dc, card_pen as _);
            let old_brush = SelectObject(mem_dc, window.brush_card as _);

            let pad_x = scale(CARD_PAD_X);
            for card in cards {
                let top = card.top - scroll;
                let bottom = card.bottom - scroll;
                if bottom < dirty.top || top > dirty.bottom {
                    continue;
                }
                RoundRect(mem_dc, card_left, top, card_right, bottom, radius, radius);
                for i in 0..card.divider_count {
                    let div_y = card.dividers[i] - scroll;
                    if div_y >= dirty.top - 1 && div_y <= dirty.bottom + 1 {
                        MoveToEx(mem_dc, card_left + pad_x, div_y, ptr::null_mut());
                        LineTo(mem_dc, card_right - pad_x, div_y);
                    }
                }
            }

            SelectObject(mem_dc, old_pen);
            SelectObject(mem_dc, old_brush);
        }
        SelectClipRgn(mem_dc, ptr::null_mut());

        BitBlt(
            hdc,
            dirty.left,
            dirty.top,
            dirty.right - dirty.left,
            dirty.bottom - dirty.top,
            mem_dc,
            dirty.left,
            dirty.top,
            SRCCOPY,
        );
        EndPaint(page_window, &ps);
    }
}

/// Draws a sleek vector illustration of the P275MV PLUS monitor centered at the top of the Overview page.
fn draw_overview_monitor(window: &mut Window, hdc: HDC, page_width: i32, scroll: i32, dirty: &RECT) {
    let dpi = window.dpi;
    let scale = |v: i32| win::scaled(v, dpi);
    let mon_w = scale(210);
    let mon_h = scale(122);
    let x = (page_width - mon_w) / 2;
    let y = scale(18) - scroll;
    let bottom = y + mon_h + scale(24);
    if bottom < dirty.top || y > dirty.bottom {
        return;
    }
    let palette = window.palette;
    unsafe {
        // Stand neck and base.
        let stand_brush = window.draw.brush(palette.muted);
        let stand_pen = window.draw.pen(palette.line, 1);
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
        let bezel_brush = window.draw.brush(0x0024_2220);
        let bezel_pen = window.draw.pen(palette.line, scale(2).max(1));
        SelectObject(hdc, bezel_pen as _);
        SelectObject(hdc, bezel_brush as _);
        RoundRect(hdc, x, y, x + mon_w, y + mon_h, scale(8), scale(8));

        // Inner display panel.
        let pad = scale(6);
        let screen_brush = window.draw.brush(palette.control_active);
        let screen_pen = window.draw.pen(palette.control_active, 1);
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
        let accent_brush = window.draw.brush(0x0058_3218);
        let accent_rect = RECT {
            left: x + pad,
            top: y + mon_h / 2,
            right: x + mon_w - pad,
            bottom: y + mon_h - pad - scale(4),
        };
        FillRect(hdc, &accent_rect, accent_brush);

        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_brush);
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
            if let Some(window) = app.window.as_mut() {
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
            if let Some(page_hwnd) = app.window.as_ref().and_then(|w| {
                if w.in_search() {
                    Some(w.search_page.window)
                } else {
                    w.pages.get(w.active).map(|p| p.window)
                }
            }) {
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
        WM_CTLCOLOREDIT => {
            let palette = app
                .window
                .as_ref()
                .map(|w| w.palette)
                .unwrap_or_else(|| palette_for(app.settings.theme));
            let (brush_card, _) = cached_brushes(&palette);
            unsafe {
                SetTextColor(wparam as _, palette.text);
                SetBkColor(wparam as _, palette.card);
                SetBkMode(wparam as _, OPAQUE as i32);
                brush_card as LRESULT
            }
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
        WM_PAINT => {
            if let Some(window) = app.window.as_mut() {
                unsafe {
                    let mut ps: PAINTSTRUCT = std::mem::zeroed();
                    let hdc = BeginPaint(hwnd, &mut ps);
                    let pill = search_pill_rect(window.dpi);
                    let r = win::scaled(6, window.dpi);
                    let border = if window.search_focused {
                        window.palette.control_active
                    } else {
                        window.palette.line
                    };
                    let brush = window.draw.brush(window.palette.card);
                    let pen = window.draw.pen(border, 1);
                    let old_pen = SelectObject(hdc, pen as _);
                    let old_brush = SelectObject(hdc, brush as _);
                    RoundRect(hdc, pill.left, pill.top, pill.right, pill.bottom, r, r);
                    SelectObject(hdc, old_pen);
                    SelectObject(hdc, old_brush);
                    EndPaint(hwnd, &ps);
                }
                return 0;
            }
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
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
            if let Some(window) = app.window.as_mut() {
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
                .is_some_and(|w| w.page_bg_labels.contains(&(child as isize)));
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
            if let Some(window) = app.window.as_mut() {
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
        SendMessageW(window.search_box, WM_SETFONT, window.font as usize, 1);
        SendMessageW(window.search_clear, WM_SETFONT, window.font as usize, 1);
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
        window.draw.release();
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
