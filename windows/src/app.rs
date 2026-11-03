//! Application state and the rules between settings.

use std::{collections::HashMap, ffi::c_void, sync::mpsc, time::Instant};

use windows_sys::Win32::Foundation::{HWND, WPARAM};
use windows_sys::Win32::Graphics::Gdi::HFONT;

use crate::{
    i18n::{self, AppLang},
    monitor,
    monitor::{MonitorInfo, UiCmd, WorkerCmd},
    settings::{self, AppSettings, FirmwarePackage, TrayIconStyle},
    ui,
};

/// How long a profile change may keep the profile rows locked (the monitor may never answer).
const PROFILE_LOAD_TIMEOUT: u32 = 25_000;
/// Timer ids of the host window.
pub const TIMER_PROFILE: usize = 1;
pub const TIMER_OUTPUT_RANGE: usize = 2;
/// One-shot timer that trims the working set once the startup burst is over.
pub const TIMER_TRIM: usize = 3;
/// Polls the modifier key state while the settings window is open so favorite stars appear on hold.
pub const TIMER_FAV_KEY: usize = 4;
/// Delay before the working set is trimmed after the start (the startup scan and the first profile read are done).
const TRIM_DELAY: u32 = 30_000;

/// Settings that belong to the picture profile table: they can only be changed in a Custom profile.
const PROFILE_LOCKED: &[u8] = &[
    monitor::VCP_BRIGHTNESS,
    monitor::VCP_CONTRAST,
    monitor::VCP_SHARPNESS,
    monitor::VCP_COLOR_ENHANCE,
    monitor::VCP_CR_ENHANCE,
    monitor::VCP_SHADOW_BALANCE,
    monitor::VCP_SUPER_RES,
    monitor::VCP_LOW_BLUE,
    monitor::VCP_COLOR_TEMP,
    monitor::VCP_GAMMA,
    monitor::VCP_HUE_RED,
    monitor::VCP_HUE_GREEN,
    monitor::VCP_HUE_BLUE,
    monitor::VCP_HUE_CYAN,
    monitor::VCP_HUE_MAGENTA,
    monitor::VCP_HUE_YELLOW,
    monitor::VCP_SATURATION_RED,
    monitor::VCP_SATURATION_GREEN,
    monitor::VCP_SATURATION_BLUE,
    monitor::VCP_SATURATION_CYAN,
    monitor::VCP_SATURATION_MAGENTA,
    monitor::VCP_SATURATION_YELLOW,
    monitor::VCP_NIGHT_VISION,
    monitor::VCP_DYNAMIC_OD,
    monitor::VCP_LOCAL_DIMMING,
    monitor::VCP_HALO_CONTROL,
    monitor::VCP_DYDS,
];

/// What a tray shortcut acts on.
#[derive(Clone, Copy)]
pub enum Shortcut {
    /// Slider value offered as a list of ready values (the tray menu cannot hold a slider).
    Slider(&'static str),
    /// Toggle a switch.
    Toggle(u8),
    /// One option of a choice.
    Choice(u8),
}

/// Tray shortcuts: id stored in the settings, action. Mirrors the Linux list.
pub const SHORTCUTS: &[(&str, Shortcut)] = &[
    ("brightness", Shortcut::Slider("brightness")),
    ("contrast", Shortcut::Slider("contrast")),
    ("sharpness", Shortcut::Slider("sharpness")),
    ("low_blue_light", Shortcut::Slider("low_blue_light")),
    ("dcr", Shortcut::Toggle(monitor::VCP_DCR)),
    ("hdr", Shortcut::Choice(monitor::VCP_HDR)),
    ("local_dimming", Shortcut::Choice(monitor::VCP_LOCAL_DIMMING)),
    ("color_enhance", Shortcut::Slider("color_enhance")),
    ("cr_enhance", Shortcut::Slider("cr_enhance")),
    ("shadow_balance", Shortcut::Slider("shadow_balance")),
    ("super_res", Shortcut::Slider("super_res")),
    ("halo_control", Shortcut::Slider("halo_control")),
    ("color_temp_profile", Shortcut::Choice(monitor::VCP_COLOR_TEMP)),
    ("color_temp_red", Shortcut::Slider("color_temp_red")),
    ("color_temp_green", Shortcut::Slider("color_temp_green")),
    ("color_temp_blue", Shortcut::Slider("color_temp_blue")),
    ("gamma", Shortcut::Choice(monitor::VCP_GAMMA)),
    ("hue_red", Shortcut::Slider("hue_red")),
    ("hue_green", Shortcut::Slider("hue_green")),
    ("hue_blue", Shortcut::Slider("hue_blue")),
    ("hue_cyan", Shortcut::Slider("hue_cyan")),
    ("hue_magenta", Shortcut::Slider("hue_magenta")),
    ("hue_yellow", Shortcut::Slider("hue_yellow")),
    ("saturation_red", Shortcut::Slider("saturation_red")),
    ("saturation_green", Shortcut::Slider("saturation_green")),
    ("saturation_blue", Shortcut::Slider("saturation_blue")),
    ("saturation_cyan", Shortcut::Slider("saturation_cyan")),
    ("saturation_magenta", Shortcut::Slider("saturation_magenta")),
    ("saturation_yellow", Shortcut::Slider("saturation_yellow")),
    ("adaptive_sync", Shortcut::Toggle(monitor::VCP_ADAPTIVE_SYNC)),
    ("dynamic_od", Shortcut::Choice(monitor::VCP_DYNAMIC_OD)),
    ("dyds", Shortcut::Choice(monitor::VCP_DYDS)),
    ("night_vision", Shortcut::Choice(monitor::VCP_NIGHT_VISION)),
    ("screen_size", Shortcut::Choice(monitor::VCP_SCREEN_SIZE)),
    ("fps_counter", Shortcut::Toggle(monitor::VCP_FPS_COUNTER)),
    ("crosshair", Shortcut::Toggle(monitor::VCP_CROSSHAIR)),
    ("stopwatch", Shortcut::Toggle(monitor::VCP_STOPWATCH)),
    ("game_time", Shortcut::Toggle(monitor::VCP_GAME_TIME)),
    ("magnifier", Shortcut::Toggle(monitor::VCP_MAGNIFIER)),
    ("hawkeye", Shortcut::Toggle(monitor::VCP_HAWKEYE)),
];

pub fn shortcut(id: &str) -> Option<&'static Shortcut> {
    SHORTCUTS.iter().find(|(known, _)| *known == id).map(|(_, action)| action)
}

/// Largest value and step of the values offered for a slider in the tray menu.
pub fn slider_range(id: &str) -> (u16, u16) {
    match id {
        "sharpness" | "cr_enhance" | "super_res" => (5, 1),
        "low_blue_light" => (4, 1),
        "color_enhance" => (10, 1),
        _ => (100, 10),
    }
}

/// Option labels of a choice shortcut.
pub fn choice_options(id: &str) -> Option<(&'static [&'static str], &'static [u16])> {
    Some(match id {
        "color_temp_profile" => (&["Warm", "Natural", "Cold", "User 1", "User 2", "User 3"], &monitor::COLOR_TEMP_VALUES),
        "screen_size" => (&["Wide", "25\""], &[0, 1]),
        "night_vision" => (&["Off", "Lvl 1", "Lvl 2", "Auto 1", "Auto 2"], &monitor::NIGHT_VISION_VALUES),
        "hdr" => (&["Off", "Auto", "Game", "Movie"], &monitor::HDR_VALUES),
        "gamma" => (&["1.8", "2.0", "2.2", "2.4", "2.6", "S Curve"], &monitor::GAMMA_VALUES),
        "local_dimming" => (&["Off", "Low", "Smooth", "Medium", "High"], &monitor::LOCAL_DIMMING_VALUES),
        "dynamic_od" => (&["Off", "Lvl 1", "Lvl 2", "Lvl 3", "Top Speed"], &monitor::DYNAMIC_OD_VALUES),
        "dyds" => (&["Off", "Low", "Med", "High", "ULL-1", "ULL-2", "ULL-3"], &monitor::DYDS_VALUES),
        _ => return None,
    })
}

/// Overview page lists: which system control a combo belongs to.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SystemKind {
    Resolution,
    Refresh,
    Scaling,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
    pub refresh: u32,
}

pub struct App {
    pub settings: AppSettings,
    pub lang: AppLang,
    pub worker: mpsc::Sender<WorkerCmd>,
    pub notifications: mpsc::Receiver<UiCmd>,
    pub state: HashMap<u8, u16>,
    pub info: Option<MonitorInfo>,
    pub window: Option<ui::Window>,
    /// Monitor found and DDC/CI ready.
    pub connected: bool,
    /// A profile change is in progress: the profile rows stay locked until its settings are read.
    pub profile_loading: bool,
    pub profile_started: Option<Instant>,
    /// Color Enhance level at the last update and the saturation the user had set while it was 0.
    pub color_enhance_level: u16,
    pub user_saturation: Option<[u16; 6]>,
    pub pending_output_range: Option<u16>,
    pub modes: Vec<DisplayMode>,
    pub current_mode: Option<DisplayMode>,
    pub connector: String,
    pub icon_small: *mut c_void,
    pub icon_large: *mut c_void,
    pub font: HFONT,
    pub bold: HFONT,
    /// Hidden window that owns the tray icon, the timers and the worker notifications.
    pub host: HWND,
    pub tray_added: bool,
    /// The application was started in the tray (`--minimized` or the preference): the window only opens on request.
    pub start_minimized: bool,
    /// A language or theme change needs the settings window to be built again.
    pub needs_rebuild: bool,
    pub quit: bool,
}

impl App {
    pub fn lang(&self) -> AppLang {
        self.lang
    }

    pub fn tr(&self, text: &str) -> String {
        i18n::tr(&self.lang, text)
    }

    /// Whether the application should use the dark theme (`System` follows the Windows app theme).
    pub fn dark(&self) -> bool {
        match self.settings.theme {
            1 => false,
            2 => true,
            _ => crate::win::system_prefers_dark(),
        }
    }

    /// Whether the caption bar should be drawn dark.
    pub fn dark_caption(&self) -> bool {
        self.dark()
    }

    pub fn tray_icon_style(&self) -> TrayIconStyle {
        self.settings.tray_icon_style
    }

    pub fn firmware(&self) -> FirmwarePackage {
        self.settings.firmware_package
    }

    pub fn save_settings(&mut self) {
        if let Err(error) = settings::save(&self.settings) {
            log!("[APP] cannot save the settings: {error}");
        }
    }

    pub fn dialog_owner(&self) -> HWND {
        self.window.as_ref().map(|window| window.hwnd).unwrap_or(self.host)
    }

    pub fn get(&self, code: u8) -> Option<u16> {
        self.state.get(&code).copied()
    }

    /// Sends a setting to the monitor, with the adjustments some settings need.
    pub fn set(&mut self, code: u8, value: u16) {
        match code {
            monitor::VCP_OUTPUT_RANGE => {
                self.local_change(code, value);
                // The monitor needs Auto before it takes another range.
                self.send(monitor::VCP_OUTPUT_RANGE, 0);
                if value != 0 {
                    self.pending_output_range = Some(value);
                    unsafe {
                        windows_sys::Win32::UI::WindowsAndMessaging::SetTimer(
                            self.host,
                            TIMER_OUTPUT_RANGE,
                            250,
                            None,
                        );
                    }
                }
                return;
            }
            monitor::VCP_DYDS => {
                // DyDs (except Off) and DCR exclude each other: switch DCR off first.
                if value != monitor::DYDS_VALUES[0] && self.get(monitor::VCP_DCR) == Some(1) {
                    self.send(monitor::VCP_DCR, 0);
                    self.local_change(monitor::VCP_DCR, 0);
                }
            }
            monitor::VCP_RED | monitor::VCP_GREEN | monitor::VCP_BLUE => {
                // RGB gains drive the User 1-3 set of the active color temperature.
                let channel = match code {
                    monitor::VCP_RED => 0,
                    monitor::VCP_GREEN => 1,
                    _ => 2,
                };
                let Some(temp) = self.get(monitor::VCP_COLOR_TEMP) else {
                    return;
                };
                let Some(codes) = monitor::color_temp_rgb_codes(temp) else {
                    return;
                };
                let target = [codes.0, codes.1, codes.2][channel];
                self.local_change(code, value);
                self.local_change(target, value);
                self.send(target, value);
                return;
            }
            monitor::VCP_MODE => self.start_profile_load(),
            _ => {}
        }
        self.local_change(code, value);
        self.send(code, value);
    }

    /// Sends a write to the worker; the value counts as set until the monitor corrects it.
    pub fn send(&mut self, code: u8, value: u16) {
        let _ = self.worker.send(WorkerCmd::Set(code, value));
    }

    /// Updates the shown value without telling the monitor.
    pub fn local_change(&mut self, code: u8, value: u16) {
        self.state.insert(code, value);
        if let Some(cache_code) = monitor::cache_rgb_key_for_vcp_code(code) {
            self.state.insert(cache_code, value);
        }
        if code == monitor::VCP_COLOR_TEMP {
            self.apply_color_temp_rgb();
        }
    }

    pub fn start_profile_load(&mut self) {
        self.profile_loading = true;
        self.profile_started = Some(Instant::now());
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::SetTimer(self.host, TIMER_PROFILE, PROFILE_LOAD_TIMEOUT, None);
        }
        ui::refresh_availability(self);
    }

    pub fn finish_profile_load(&mut self) {
        if self.profile_loading {
            self.profile_loading = false;
            self.profile_started = None;
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::KillTimer(self.host, TIMER_PROFILE);
            }
            ui::refresh_availability(self);
        }
    }

    /// Shows the RGB gains of the active color temperature in the three sliders.
    pub fn apply_color_temp_rgb(&mut self) {
        let Some(temp) = self.get(monitor::VCP_COLOR_TEMP) else {
            return;
        };
        let values = match monitor::builtin_color_temp_rgb(temp) {
            Some((red, green, blue)) => Some([red, green, blue]),
            None => monitor::color_temp_rgb_codes(temp)
                .and_then(|codes| Some([self.get(codes.0)?, self.get(codes.1)?, self.get(codes.2)?])),
        };
        if let Some([red, green, blue]) = values {
            self.state.insert(monitor::VCP_RED, red);
            self.state.insert(monitor::VCP_GREEN, green);
            self.state.insert(monitor::VCP_BLUE, blue);
        }
    }

    /// With Color Enhance on, the monitor sets the six saturation axes itself; going back to 0 restores what the
    /// user had set before.
    pub fn show_saturation(&mut self) {
        let level = self.get(monitor::VCP_COLOR_ENHANCE).unwrap_or(0);
        let was_on = std::mem::replace(&mut self.color_enhance_level, level) > 0;
        if let Some(forced) = monitor::color_enhance_saturation(level) {
            for (index, code) in monitor::SATURATION_CODES.into_iter().enumerate() {
                self.local_change(code, forced[index]);
            }
            return;
        }
        if was_on {
            if let Some(saved) = self.user_saturation {
                for (index, code) in monitor::SATURATION_CODES.into_iter().enumerate() {
                    if self.get(code) != Some(saved[index]) {
                        self.local_change(code, saved[index]);
                        let _ = self.worker.send(WorkerCmd::Set(code, saved[index]));
                    }
                }
            }
            return;
        }
        let now: [Option<u16>; 6] = std::array::from_fn(|index| self.get(monitor::SATURATION_CODES[index]));
        if let [Some(red), Some(green), Some(blue), Some(cyan), Some(magenta), Some(yellow)] = now {
            self.user_saturation = Some([red, green, blue, cyan, magenta, yellow]);
        }
    }

    /// Applies values read from the monitor.
    pub fn apply_values(&mut self, values: HashMap<u8, u16>) {
        self.state.extend(values.iter().map(|(&code, &value)| (code, value)));
        if values.contains_key(&monitor::VCP_COLOR_TEMP) {
            self.apply_color_temp_rgb();
        }
        if values.contains_key(&monitor::VCP_COLOR_ENHANCE) {
            self.show_saturation();
        }
        ui::apply_state(self);
        ui::refresh_availability(self);
    }

    /// Whether a control may be used with the current monitor state.
    pub fn availability(&self) -> HashMap<u8, bool> {
        use crate::monitor::*;
        let get = |code: u8| self.state.get(&code).copied();
        let mut allowed: HashMap<u8, bool> = HashMap::new();
        let mut allow = |code: u8, ok: bool| {
            *allowed.entry(code).or_insert(true) &= ok;
        };

        // Picture profile: the table settings can only be changed in a Custom profile.
        let custom = get(VCP_MODE).is_none_or(|mode| is_custom_profile(mode as u8));
        let loading = self.profile_loading;
        for &code in PROFILE_LOCKED {
            allow(code, custom && !loading);
        }

        // Game modes and overlays.
        let wide = get(VCP_SCREEN_SIZE) != Some(1);
        let adaptive = get(VCP_ADAPTIVE_SYNC) == Some(1);
        let dyds_ull = get(VCP_DYDS).is_some_and(|value| value >= DYDS_VALUES[4]);
        let local_dimming = get(VCP_LOCAL_DIMMING).is_some_and(|value| value > LOCAL_DIMMING_VALUES[0]);
        let dcr = get(VCP_DCR) == Some(1);
        let magnifier = get(VCP_MAGNIFIER) == Some(1);
        let hawkeye = get(VCP_HAWKEYE) == Some(1);
        let dyds_with_vrr = self.settings.firmware_package.dyds_with_adaptive_sync();
        allow(VCP_ADAPTIVE_SYNC, wide);
        allow(VCP_DYDS, wide && (dyds_with_vrr || !adaptive));
        allow(VCP_DYNAMIC_OD, wide);
        allow(VCP_MAGNIFIER, wide && !adaptive && !hawkeye);
        allow(VCP_HAWKEYE, wide && !magnifier);
        allow(VCP_HALO_CONTROL, wide && local_dimming && !dyds_ull && !dcr);
        allow(VCP_LOCAL_DIMMING, !dyds_ull && !dcr);
        allow(VCP_DCR, !local_dimming);
        for code in [VCP_BRIGHTNESS, VCP_CONTRAST, VCP_SHADOW_BALANCE] {
            allow(code, !dcr);
        }
        let color_enhance = get(VCP_COLOR_ENHANCE).is_some_and(|value| value > 0);
        for code in SATURATION_CODES {
            allow(code, !color_enhance);
        }

        // PIP/PBP: the monitor switches some features off; position and size only in PIP.
        let pip = get(VCP_PIP_MODE).unwrap_or(values::PIP_OFF);
        for &code in PIP_DISABLED_CODES {
            allow(code, pip == values::PIP_OFF);
        }
        allow(VCP_PIP_SOURCE, pip != values::PIP_OFF);
        allow(VCP_AUDIO_SOURCE, pip != values::PIP_OFF);
        allow(VCP_PIP_SWAP, pip != values::PIP_OFF);
        allow(VCP_PIP_RESET, pip != values::PIP_OFF);
        allow(VCP_PIP_POSITION, pip == 1);
        allow(VCP_PIP_SIZE, pip == 1);

        // LED effects: only with the LED on; some modes lock colors.
        let led_on = get(VCP_REAR_LED) != Some(values::REAR_LED_OFF);
        for code in [VCP_LED_MODE, VCP_LED_COLOR, VCP_LED_STRENGTH, VCP_LED_FRONT_COLOR, VCP_LED_REAR_COLOR] {
            allow(code, led_on && is_setting_available(&self.state, code));
        }

        // Firmware 2025-09-20: with HDR on the monitor locks these settings.
        let hdr_locked = self.hdr_locked();
        if hdr_locked {
            for code in [
                VCP_BRIGHTNESS,
                VCP_CONTRAST,
                VCP_DCR,
                VCP_LOW_BLUE,
                VCP_COLOR_ENHANCE,
                VCP_CR_ENHANCE,
                VCP_SHADOW_BALANCE,
                VCP_OUTPUT_RANGE,
            ] {
                allow(code, false);
            }
            for code in [VCP_NIGHT_VISION, VCP_DYDS] {
                allow(code, adaptive);
            }
            allow(VCP_MAGNIFIER, false);
        }

        // Gaming overlay sub-rows require their parent overlay toggle to be active and unlocked.
        let overlay_active = |parent: u8| -> bool {
            get(parent) == Some(1) && allowed.get(&parent).copied().unwrap_or(true)
        };
        let fps_on = overlay_active(VCP_FPS_COUNTER);
        let crosshair_on = overlay_active(VCP_CROSSHAIR);
        let stopwatch_on = overlay_active(VCP_STOPWATCH);
        let game_time_on = overlay_active(VCP_GAME_TIME);
        let magnifier_on = overlay_active(VCP_MAGNIFIER);
        let hawkeye_on = overlay_active(VCP_HAWKEYE);

        let mut allow = |code: u8, ok: bool| {
            *allowed.entry(code).or_insert(true) &= ok;
        };
        allow(VCP_FPS_POS, fps_on);
        allow(VCP_CROSSHAIR_SHAPE, crosshair_on);
        allow(VCP_CROSSHAIR_COLOR, crosshair_on);
        allow(VCP_STOPWATCH_TIME, stopwatch_on);
        allow(VCP_STOPWATCH_POS, stopwatch_on);
        allow(VCP_GAME_TIME_VAL, game_time_on);
        allow(VCP_GAME_TIME_POS, game_time_on);
        allow(VCP_MAGNIFIER_ZOOM, magnifier_on);
        allow(VCP_MAGNIFIER_SIZE, magnifier_on);
        allow(VCP_MAGNIFIER_POS, magnifier_on);
        allow(VCP_HAWKEYE_SIZE, hawkeye_on);
        allow(VCP_HAWKEYE_POS, hawkeye_on);
        allow(VCP_HAWKEYE_LEVEL, hawkeye_on);

        allowed
    }

    pub fn hdr_locked(&self) -> bool {
        self.settings.firmware_package.dyds_with_adaptive_sync()
            && self.get(monitor::VCP_HDR).is_some_and(|value| value != monitor::HDR_VALUES[0])
    }
}

// ───────────────────────────── worker messages ─────────────────────────────

/// Handles the worker notifications that arrived while the UI was busy.
pub fn drain_messages(app: &mut App) {
    loop {
        let message = match app.notifications.try_recv() {
            Ok(message) => message,
            Err(mpsc::TryRecvError::Empty) => break,
            Err(mpsc::TryRecvError::Disconnected) => break,
        };
        match message {
            UiCmd::MonitorFound(info) => {
                app.connected = true;
                app.connector = info.connector.clone();
                refresh_modes(app);
                app.apply_values(info.settings.clone());
                app.info = Some(info);
                ui::apply_state(app);
                crate::tray::refresh(app);
                // A start in the tray stays in the tray: the monitor being found does not open the window.
                if !app.start_minimized {
                    ui::open(app);
                }
            }
            UiCmd::Settings(values) => app.apply_values(values),
            UiCmd::Corrected(values) => {
                // The monitor keeps another value than the one written: show what it reports.
                app.state.extend(values);
                app.show_saturation();
                ui::apply_state(app);
                ui::refresh_availability(app);
                crate::tray::refresh(app);
            }
            UiCmd::ProfileLoaded => app.finish_profile_load(),
            UiCmd::UsageTimeUpdated(minutes) => {
                if let Some(info) = app.info.as_mut() {
                    info.usage_mins = minutes;
                }
                ui::apply_state(app);
            }
            UiCmd::InfoRefreshed { hz, usage_mins } => {
                if let Some(info) = app.info.as_mut() {
                    if let Some(hz) = hz {
                        info.hz = hz;
                    }
                    if let Some(minutes) = usage_mins {
                        info.usage_mins = minutes;
                    }
                }
                ui::apply_state(app);
            }
            UiCmd::Progress(text) => {
                log!("[APP] {text}");
                ui::set_status_progress(app, &text);
            }
            UiCmd::Error(error) => {
                log!("[APP] {error}");
                if error == "Monitor not found" {
                    crate::notify(app, "Nie znaleziono monitora", "Podłącz monitor P275MV PLUS i uruchom program ponownie.");
                    app.quit = true;
                }
            }
        }
    }
}

// ───────────────────────────── timers and scrolling ─────────────────────────────

pub fn handle_timer(app: &mut App, timer: usize) {
    match timer {
        TIMER_PROFILE => {
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::KillTimer(app.host, TIMER_PROFILE);
            }
            app.finish_profile_load();
        }
        TIMER_OUTPUT_RANGE => {
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::KillTimer(app.host, TIMER_OUTPUT_RANGE);
            }
            if let Some(value) = app.pending_output_range.take() {
                app.send(monitor::VCP_OUTPUT_RANGE, value);
            }
        }
        TIMER_TRIM => {
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::KillTimer(app.host, TIMER_TRIM);
            }
            if !ui::is_open(app) {
                trim_working_set();
            }
        }
        TIMER_FAV_KEY => {
            ui::poll_favorites_key(app);
        }
        _ => {}
    }
}

/// Hands the pages of the process back to the system: the startup burst is over, and a tray application only touches
/// them again when the user opens the window. The pages are faulted back in on demand, so this costs nothing at idle
/// and keeps the idle working set small.
pub fn trim_working_set() {
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, SetProcessWorkingSetSize};
    log!("[APP] trimming the working set");
    unsafe {
        // (-1, -1) asks Windows to remove as many pages as it can.
        SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX);
    }
}

/// Starts the one-shot working-set trim of the startup burst.
pub fn schedule_trim(app: &App) {
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::SetTimer(app.host, TIMER_TRIM, TRIM_DELAY, None);
    }
}

/// A page scroll bar or the mouse wheel moved.
pub fn handle_page_scroll(app: &mut App, page_window: HWND, request: WPARAM) {
    let code = (request as u32) & 0xFFFF;
    let current = ui::scroll_of(app, page_window);
    let dpi = crate::win::dpi_of(page_window);
    let step = crate::win::scaled(48, dpi);
    let next = match code {
        SOCB_LINEUP_CODE => current - step,
        SOCB_LINEDOWN_CODE => current + step,
        SOCB_PAGEUP_CODE => current - step * 6,
        SOCB_PAGEDOWN_CODE => current + step * 6,
        SOCB_THUMBPOSITION_CODE | SOCB_THUMBTRACK_CODE => {
            use windows_sys::Win32::UI::WindowsAndMessaging::{GetScrollInfo, SB_VERT, SCROLLINFO, SIF_TRACKPOS};
            let mut info: SCROLLINFO = unsafe { std::mem::zeroed() };
            info.cbSize = std::mem::size_of::<SCROLLINFO>() as u32;
            info.fMask = SIF_TRACKPOS;
            if unsafe { GetScrollInfo(page_window, SB_VERT, &mut info) } != 0 {
                info.nTrackPos
            } else {
                ((request as u32) >> 16) as i32
            }
        }
        SOCB_TOP_CODE => 0,
        SOCB_BOTTOM_CODE => i32::MAX / 2,
        _ => return,
    };
    ui::set_scroll(app, page_window, next);
}

/// Small helper constants so the scroll codes stay readable (they are the SB_* values of Win32).
const SOCB_LINEUP_CODE: u32 = 0;
const SOCB_LINEDOWN_CODE: u32 = 1;
const SOCB_PAGEUP_CODE: u32 = 2;
const SOCB_PAGEDOWN_CODE: u32 = 3;
const SOCB_THUMBPOSITION_CODE: u32 = 4;
const SOCB_THUMBTRACK_CODE: u32 = 5;
const SOCB_TOP_CODE: u32 = 6;
const SOCB_BOTTOM_CODE: u32 = 7;

pub fn handle_wheel(app: &mut App, page_window: HWND, delta: i32) {
    if delta == 0 {
        return;
    }
    let current = ui::scroll_of(app, page_window);
    let dpi = crate::win::dpi_of(page_window);
    let base = crate::win::scaled(64, dpi);
    let mut step = -(delta * base / 120);
    if step == 0 {
        step = if delta > 0 { -16 } else { 16 };
    }
    ui::set_scroll(app, page_window, current + step);
}

// ───────────────────────────── system display ─────────────────────────────

/// Reads the modes the monitor is offered by the system.
pub fn refresh_modes(app: &mut App) {
    use windows_sys::Win32::Graphics::Gdi::{EnumDisplaySettingsW, DEVMODEW, ENUM_CURRENT_SETTINGS};

    let mut modes: Vec<DisplayMode> = Vec::new();
    let mut current = None;
    let name = crate::win::wide(&app.connector);
    unsafe {
        let mut devmode: DEVMODEW = std::mem::zeroed();
        devmode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
        let mut index = 0u32;
        while EnumDisplaySettingsW(name.as_ptr(), index, &mut devmode) != 0 {
            if devmode.dmBitsPerPel >= 32 && devmode.dmDisplayFrequency > 1 {
                modes.push(DisplayMode {
                    width: devmode.dmPelsWidth,
                    height: devmode.dmPelsHeight,
                    refresh: devmode.dmDisplayFrequency,
                });
            }
            index += 1;
            if index > 512 {
                break;
            }
        }
        let mut devmode: DEVMODEW = std::mem::zeroed();
        devmode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
        if EnumDisplaySettingsW(name.as_ptr(), ENUM_CURRENT_SETTINGS, &mut devmode) != 0 {
            current = Some(DisplayMode {
                width: devmode.dmPelsWidth,
                height: devmode.dmPelsHeight,
                refresh: devmode.dmDisplayFrequency,
            });
        }
    }
    modes.sort_by(|left, right| {
        (left.width, left.height, left.refresh).cmp(&(right.width, right.height, right.refresh))
    });
    modes.dedup();
    if let (Some(info), Some(cur)) = (app.info.as_mut(), current.as_ref()) {
        info.resolution = format!("{}x{}", cur.width, cur.height);
        info.hz = cur.refresh as u16;
    }
    app.modes = modes;
    app.current_mode = current;
}

/// Switches the display to another mode.
pub fn apply_display_mode(app: &mut App, mode: DisplayMode) {
    use windows_sys::Win32::Graphics::Gdi::{
        ChangeDisplaySettingsExW, EnumDisplaySettingsW, CDS_UPDATEREGISTRY, DEVMODEW, DISP_CHANGE_SUCCESSFUL,
        DM_DISPLAYFREQUENCY, DM_PELSHEIGHT, DM_PELSWIDTH, ENUM_CURRENT_SETTINGS,
    };
    let name = crate::win::wide(&app.connector);
    unsafe {
        let mut devmode: DEVMODEW = std::mem::zeroed();
        devmode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
        if EnumDisplaySettingsW(name.as_ptr(), ENUM_CURRENT_SETTINGS, &mut devmode) == 0 {
            return;
        }
        devmode.dmPelsWidth = mode.width;
        devmode.dmPelsHeight = mode.height;
        devmode.dmDisplayFrequency = mode.refresh;
        devmode.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY;
        let result = ChangeDisplaySettingsExW(
            name.as_ptr(),
            &devmode,
            std::ptr::null_mut(),
            CDS_UPDATEREGISTRY,
            std::ptr::null_mut(),
        );
        if result != DISP_CHANGE_SUCCESSFUL {
            log!("[DISPLAY] mode change failed: {result}");
            ui::message_box(app, "Nie udało się zmienić trybu wyświetlania", &format!("{result}"));
            return;
        }
    }
    log!(
        "[DISPLAY] mode {}x{} @ {} Hz",
        mode.width,
        mode.height,
        mode.refresh
    );
    refresh_modes(app);
    ui::apply_state(app);
}

/// Confirms that the settings file and the autostart entry agree with the settings.
pub fn sync_autostart(app: &mut App) {
    if let Err(error) = settings::sync_autostart(&app.settings) {
        log!("[APP] cannot update the autostart entry: {error}");
    }
}
