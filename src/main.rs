#[macro_use]
mod log;
mod app_settings;
mod i18n;
mod monitor;
mod tray;
mod ui;

use adw::prelude::*;
use app_settings::{AppSettings, FirmwarePackage, StepModifier, TrayIconStyle};
use gtk4::{gio, glib};
use i18n::{AppLang, tr};
use libadwaita as adw;
use monitor::WorkerCmd;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
use ui::Ui;
use ui::bindings::{self, Bindings, Control};
use ui::translate::tr_pl;

fn main() {
    log::init();
    // GTK's own texts (search fields, empty lists) follow the language chosen in the app, not only the system one.
    if let Some(index) = app_settings::load().language {
        let language = if i18n::lang_from_index(index) == AppLang::PL { "pl_PL:pl" } else { "en_US:en" };
        // SAFETY: nothing else runs yet (no other threads exist before GTK starts).
        unsafe { std::env::set_var("LANGUAGE", language) };
    }
    glib::set_application_name(app_settings::APP_DISPLAY_NAME);
    // The UI check must not activate a running instance (single-instance app over D-Bus).
    let flags = if std::env::var_os("TITAN_CHECK_UI").is_some() {
        gio::ApplicationFlags::NON_UNIQUE
    } else {
        gio::ApplicationFlags::empty()
    };
    let app = adw::Application::builder().application_id(app_settings::APP_ID).flags(flags).build();
    app.connect_activate(build_application);
    app.run();
}

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

/// Monitor HDR is mirrored to the system HDR once the value has been stable this long (startup reads and profile
/// changes can pass through other values first).
const HDR_SYNC_DELAY: Duration = Duration::from_millis(600);

/// Position of 21:9 in the `ratio` combo.
const RATIO_21_9_INDEX: u32 = 3;

/// OSD languages in the order of the `osd_language` combo (MCCS language codes).
const OSD_LANGUAGE_VALUES: [u16; 22] = [
    0x0D, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0C, 0x01, 0x0E, 0x0F, 0x14, 0x16, 0x17, 0x19, 0x1E,
    0x23, 0x24, 0x25,
];

/// Rows whose explanation is shown behind an info button instead of a subtitle.
const INFO_ROWS: &[&str] = &[
    "row_hdr",
    "magnifier",
    "eyeshield",
    "dcr",
    "row_halo_control",
    "rgb_expander",
    "saturation_expander",
    "ratio",
    "row_usb_hub",
    "row_pip_position",
    "row_pip_size",
    "audio_source",
    "led_mode",
    "led_sides",
    "osd_lock",
    "button_lock",
    "usb_power_sleep",
    "firmware_package",
];

/// What a tray shortcut acts on.
#[derive(Clone, Copy)]
enum Shortcut {
    /// Slider popup on the adjustment with this id.
    Slider(&'static str),
    /// Toggle a switch / an expander of this setting.
    Toggle(u8),
    /// Next option of the toggle group of this setting.
    Choice(u8),
}

/// Tray shortcuts: id (stored in the config), row with the star, action.
const SHORTCUTS: &[(&str, &str, Shortcut)] = &[
    ("brightness", "row_brightness", Shortcut::Slider("adj_brightness")),
    ("contrast", "row_contrast", Shortcut::Slider("adj_contrast")),
    ("sharpness", "row_sharpness", Shortcut::Slider("adj_sharpness")),
    ("low_blue_light", "row_low_blue_light", Shortcut::Slider("adj_lbl")),
    ("dcr", "dcr", Shortcut::Toggle(monitor::VCP_DCR)),
    ("hdr", "row_hdr", Shortcut::Choice(monitor::VCP_HDR)),
    ("local_dimming", "row_local_dimming", Shortcut::Choice(monitor::VCP_LOCAL_DIMMING)),
    ("color_enhance", "row_color_enhance", Shortcut::Slider("adj_color_enhance")),
    ("cr_enhance", "row_cr_enhance", Shortcut::Slider("adj_cr_enhance")),
    ("shadow_balance", "row_shadow_balance", Shortcut::Slider("adj_shadow_balance")),
    ("super_res", "row_super_res", Shortcut::Slider("adj_super_res")),
    ("halo_control", "row_halo_control", Shortcut::Slider("adj_halo")),
    ("color_temp_profile", "row_color_temp", Shortcut::Choice(monitor::VCP_COLOR_TEMP)),
    ("color_temp_red", "row_color_temp_red", Shortcut::Slider("adj_red")),
    ("color_temp_green", "row_color_temp_green", Shortcut::Slider("adj_green")),
    ("color_temp_blue", "row_color_temp_blue", Shortcut::Slider("adj_blue")),
    ("gamma", "row_gamma", Shortcut::Choice(monitor::VCP_GAMMA)),
    ("hue_red", "row_hue_red", Shortcut::Slider("adj_hue_red")),
    ("hue_green", "row_hue_green", Shortcut::Slider("adj_hue_green")),
    ("hue_blue", "row_hue_blue", Shortcut::Slider("adj_hue_blue")),
    ("hue_cyan", "row_hue_cyan", Shortcut::Slider("adj_hue_cyan")),
    ("hue_magenta", "row_hue_magenta", Shortcut::Slider("adj_hue_magenta")),
    ("hue_yellow", "row_hue_yellow", Shortcut::Slider("adj_hue_yellow")),
    ("saturation_red", "row_saturation_red", Shortcut::Slider("adj_sat_red")),
    ("saturation_green", "row_saturation_green", Shortcut::Slider("adj_sat_green")),
    ("saturation_blue", "row_saturation_blue", Shortcut::Slider("adj_sat_blue")),
    ("saturation_cyan", "row_saturation_cyan", Shortcut::Slider("adj_sat_cyan")),
    ("saturation_magenta", "row_saturation_magenta", Shortcut::Slider("adj_sat_magenta")),
    ("saturation_yellow", "row_saturation_yellow", Shortcut::Slider("adj_sat_yellow")),
    ("adaptive_sync", "adaptive_sync", Shortcut::Toggle(monitor::VCP_ADAPTIVE_SYNC)),
    ("dynamic_od", "row_dynamic_od", Shortcut::Choice(monitor::VCP_DYNAMIC_OD)),
    ("dyds", "row_dyds", Shortcut::Choice(monitor::VCP_DYDS)),
    ("night_vision", "row_night_vision", Shortcut::Choice(monitor::VCP_NIGHT_VISION)),
    ("screen_size", "row_screen_size", Shortcut::Choice(monitor::VCP_SCREEN_SIZE)),
    ("fps_counter", "fps_counter", Shortcut::Toggle(monitor::VCP_FPS_COUNTER)),
    ("crosshair", "crosshair", Shortcut::Toggle(monitor::VCP_CROSSHAIR)),
    ("stopwatch", "stopwatch", Shortcut::Toggle(monitor::VCP_STOPWATCH)),
    ("game_time", "game_time", Shortcut::Toggle(monitor::VCP_GAME_TIME)),
    ("magnifier", "magnifier", Shortcut::Toggle(monitor::VCP_MAGNIFIER)),
    ("hawkeye", "hawkeye", Shortcut::Toggle(monitor::VCP_HAWKEYE)),
];

struct App {
    ui: Ui,
    bindings: Rc<Bindings>,
    cfg: RefCell<AppSettings>,
    lang: Cell<AppLang>,
    /// Picture modes shown in the combos (indices into `monitor::PICTURE_MODE_NAMES`).
    picture_order: RefCell<Vec<u16>>,
    system_display: RefCell<Option<monitor::SystemDisplayState>>,
    connector: RefCell<String>,
    info: RefCell<Option<monitor::MonitorInfo>>,
    tray_favorites: tray::TrayFavoritesState,
    tray_shortcuts: tray::TrayShortcutsState,
    tray_active_mode: tray::TrayActiveModeState,
    tray_choices: tray::TrayChoiceStateHandle,
    tray_window_visible: tray::TrayWindowVisibleState,
    stars: RefCell<HashMap<&'static str, gtk4::ToggleButton>>,
    picture_star: RefCell<Option<gtk4::ToggleButton>>,
    tray: RefCell<Option<tray::TrayControl>>,
    /// Explanations behind info buttons: (button, label, Polish source text).
    info_texts: Vec<(gtk4::MenuButton, gtk4::Label, String)>,
    /// HDR of the monitor's output in the system (`None` = unknown or not available).
    system_hdr: Cell<Option<bool>>,
    /// Pending system HDR change (a newer change cancels an older one).
    hdr_sync: Cell<u32>,
    /// Color Enhance level at the last update, and the saturation the user had set while it was 0.
    color_enhance_level: Cell<u16>,
    /// A profile change is in progress (see `start_profile_load`) and a counter for its safety timeout.
    profile_loading: Cell<bool>,
    profile_load_generation: Cell<u32>,
    user_saturation: RefCell<Option<[u16; 6]>>,
}

impl App {
    fn save_cfg(&self, change: impl FnOnce(&mut AppSettings)) {
        let mut cfg = self.cfg.borrow().clone();
        change(&mut cfg);
        let _ = app_settings::save(&cfg);
        *self.cfg.borrow_mut() = cfg;
    }

    fn tx(&self) -> &mpsc::Sender<WorkerCmd> {
        self.bindings.tx()
    }

    fn lang(&self) -> AppLang {
        self.lang.get()
    }

    fn update_info_texts(&self) {
        for (button, label, source) in &self.info_texts {
            let text = tr_pl(&self.lang(), source);
            label.set_label(&text);
            button.set_tooltip_text(Some(&text));
        }
    }

    /// Republishes the tray menu after a change it shows.
    fn refresh_tray(&self) {
        if let Some(tray) = self.tray.borrow().as_ref() {
            tray.refresh();
        }
    }
}

fn register_app_icons() {
    if let Some(display) = gtk4::gdk::Display::default() {
        let icon_theme = gtk4::IconTheme::for_display(&display);
        for icon_path in app_settings::icon_search_paths() {
            icon_theme.add_search_path(&icon_path);
        }
    }
    gtk4::Window::set_default_icon_name(app_settings::APP_ICON_NAME);
}

fn format_usage_minutes(usage_mins: u16) -> String {
    format!("{} h {} min", usage_mins / 60, usage_mins % 60)
}

fn translate_startup_message(lang: &AppLang, message: &str) -> String {
    match message {
        "Searching for monitor..." => tr(lang, "splash_searching"),
        "Detecting monitors..." => tr(lang, "splash_detecting_monitors"),
        "Searching for monitors..." => tr(lang, "splash_searching_monitors"),
        "Loading cached settings..." => tr(lang, "splash_loading_cached"),
        "Reading picture profile..." => tr(lang, "splash_reading_profile"),
        "Monitor not found" => tr(lang, "splash_monitor_not_found"),
        _ => message.to_string(),
    }
}

fn build_application(app: &adw::Application) {
    log::versions();
    register_app_icons();
    let _ = app_settings::ensure_desktop_entry();

    let cfg = app_settings::load();
    let lang = resolve_lang(cfg.language);
    apply_theme(cfg.theme);
    let ui = Ui::new(app);

    // Layout check without a monitor: `TITAN_CHECK_UI=1 titan_control` (`TITAN_CHECK_LANG=en|pl` picks the language).
    if std::env::var_os("TITAN_CHECK_UI").is_some() {
        let lang = match std::env::var("TITAN_CHECK_LANG").as_deref() {
            Ok("en") => AppLang::EN,
            Ok("pl") => AppLang::PL,
            _ => lang,
        };
        for (button, label, source) in ui.subtitles_to_info(INFO_ROWS) {
            let text = tr_pl(&lang, &source);
            label.set_label(&text);
            button.set_tooltip_text(Some(&text));
        }
        ui.translator.apply(&lang);
        translate_labels(&ui, &lang);
        for &(_, row, _) in SHORTCUTS {
            ui.add_favorite_star(row, false);
        }
        ui.align_rows();
        ui.show_favorite_stars(std::env::var_os("TITAN_CHECK_STARS").is_some());
        ui.show_wallpaper(monitor::query_system_display_state(None).ok().and_then(|state| monitor::system_wallpaper(&state)));
        println!("UI OK: {} sliders", ui.scales.len());
        match std::env::var("TITAN_CHECK_UI_SHOTS") {
            Ok(dir) => save_page_screenshots(app.clone(), ui, std::path::PathBuf::from(dir)),
            Err(_) => app.quit(),
        }
        return;
    }

    let info_texts = ui.subtitles_to_info(INFO_ROWS);
    let (worker_tx, worker_rx) = mpsc::channel::<WorkerCmd>();
    let (ui_tx, ui_rx) = async_channel::unbounded::<monitor::UiCmd>();
    let splash = build_splash(app, &lang, !cfg.start_minimized);
    let _ = worker_tx.send(WorkerCmd::DydsWithAdaptiveSync(cfg.firmware_package.dyds_with_adaptive_sync()));
    let bindings = Bindings::new(worker_tx, ui.scales.clone());

    let app_state = Rc::new(App {
        tray_favorites: Arc::new(Mutex::new(favorite_profile_entries(&cfg))),
        tray_shortcuts: Arc::new(Mutex::new(favorite_tray_shortcut_entries(&cfg, &lang))),
        tray_active_mode: Arc::new(Mutex::new(None)),
        tray_choices: Arc::new(Mutex::new(tray::TrayChoiceState::default())),
        tray_window_visible: Arc::new(Mutex::new(false)),
        ui,
        bindings,
        cfg: RefCell::new(cfg),
        lang: Cell::new(lang),
        picture_order: RefCell::default(),
        system_display: RefCell::default(),
        connector: RefCell::default(),
        info: RefCell::default(),
        stars: RefCell::default(),
        picture_star: RefCell::default(),
        tray: RefCell::default(),
        info_texts,
        system_hdr: Cell::new(None),
        hdr_sync: Cell::new(0),
        color_enhance_level: Cell::new(0),
        profile_loading: Cell::new(false),
        profile_load_generation: Cell::new(0),
        user_saturation: RefCell::new(None),
    });
    let a = &app_state;

    a.ui.translator.apply(&lang);
    translate_labels(&a.ui, &lang);
    a.update_info_texts();
    rebuild_menu(a);
    setup_actions(app, a);
    bind_settings(a);
    setup_picture_mode(a);
    setup_color_temp_rgb(a);
    setup_actions_rows(a);
    setup_system_display(a);
    setup_preferences(a);
    setup_info(a);
    setup_favorites(a);
    a.ui.align_rows();
    {
        let app_state = a.clone();
        a.bindings.on_change(move || sync_state(&app_state));
    }
    sync_state(a);

    monitor::start_worker(worker_rx, ui_tx);
    receive_monitor_messages(a, ui_rx, splash, app.clone());

    {
        let tx = a.tx().clone();
        glib::timeout_add_local(Duration::from_secs(180), move || {
            let _ = tx.send(WorkerCmd::ReadUsageTime);
            glib::ControlFlow::Continue
        });
    }
    {
        let a2 = a.clone();
        a.ui.window.connect_visible_notify(move |window| {
            if let Ok(mut state) = a2.tray_window_visible.lock() {
                *state = window.is_visible();
            }
            a2.refresh_tray();
        });
    }
    // Closing hides the window to the tray.
    a.ui.window.connect_close_request(|window| {
        window.set_visible(false);
        glib::Propagation::Stop
    });
    setup_tray(app, a);
}

/// Development aid: renders every page of the window and the preferences dialog into `dir` as PNG, then quits.
fn save_page_screenshots(app: adw::Application, ui: Ui, dir: std::path::PathBuf) {
    let _ = std::fs::create_dir_all(&dir);
    let size = |name: &str, default: i32| std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default);
    ui.window.set_default_size(size("TITAN_CHECK_UI_WIDTH", 1020), size("TITAN_CHECK_UI_HEIGHT", 900));
    ui.window.present();
    let stack: gtk4::Stack = ui.get("stack");
    let mut shots: Vec<String> = (0..stack.pages().n_items())
        .filter_map(|index| stack.pages().item(index).and_downcast::<gtk4::StackPage>())
        .filter_map(|page| page.name().map(|name| name.to_string()))
        .collect();
    shots.push("preferences".into());
    let ui = Rc::new(ui);
    let step = Rc::new(Cell::new(0usize));
    glib::timeout_add_local(Duration::from_millis(700), move || {
        let index = step.get();
        if index > 0 {
            let window = &ui.window;
            let paintable = gtk4::WidgetPaintable::new(Some(window));
            let snapshot = gtk4::Snapshot::new();
            paintable.snapshot(&snapshot, window.width() as f64, window.height() as f64);
            if let (Some(node), Some(renderer)) = (snapshot.to_node(), window.renderer()) {
                let path = dir.join(format!("{index:02}-{}.png", shots[index - 1]));
                let _ = renderer.render_texture(node, None).save_to_png(&path);
                println!("saved {}", path.display());
            }
        }
        match shots.get(index).map(String::as_str) {
            None => {
                app.quit();
                return glib::ControlFlow::Break;
            }
            Some("preferences") => ui.prefs.present(Some(&ui.window)),
            Some(page) => stack.set_visible_child_name(page),
        }
        step.set(index + 1);
        glib::ControlFlow::Continue
    });
}

/// Plain labels are not handled by the translator: the status under the monitor picture until it is connected.
fn translate_labels(ui: &Ui, lang: &AppLang) {
    ui.get::<gtk4::Label>("ov_monitor_subtitle").set_label(&tr_pl(lang, "Łączenie z monitorem…"));
}

/// App language: the saved choice, else the system language; only Polish and English are translated.
fn resolve_lang(saved: Option<u32>) -> AppLang {
    let lang = saved.map(i18n::lang_from_index).unwrap_or_else(i18n::detect_system_lang);
    if lang == AppLang::PL { AppLang::PL } else { AppLang::EN }
}

fn apply_theme(theme: u32) {
    adw::StyleManager::default().set_color_scheme(match theme {
        1 => adw::ColorScheme::ForceLight,
        2 => adw::ColorScheme::ForceDark,
        _ => adw::ColorScheme::Default,
    });
}

/// Startup window while the monitor is searched for: logo, name, spinner and the current step.
fn build_splash(app: &adw::Application, lang: &AppLang, show: bool) -> (adw::Window, gtk4::Label) {
    let logo = match app_settings::resolve_logo_path() {
        Some(path) => gtk4::Image::from_file(path),
        None => gtk4::Image::from_icon_name(app_settings::APP_ICON_NAME),
    };
    logo.set_pixel_size(112);
    let label = gtk4::Label::builder()
        .label(tr(lang, "splash_searching"))
        .wrap(true)
        .justify(gtk4::Justification::Center)
        .css_classes(["dim-label"])
        .build();
    let status = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(10)
        .halign(gtk4::Align::Center)
        .build();
    status.append(&adw::Spinner::builder().width_request(18).height_request(18).build());
    status.append(&label);
    let content = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .valign(gtk4::Align::Center)
        .margin_top(36)
        .margin_bottom(36)
        .margin_start(32)
        .margin_end(32)
        .build();
    content.append(&logo);
    content.append(
        &gtk4::Label::builder()
            .label(app_settings::APP_DISPLAY_NAME)
            .margin_top(12)
            .margin_bottom(18)
            .css_classes(["title-1"])
            .build(),
    );
    content.append(&status);
    let handle = gtk4::WindowHandle::builder().child(&content).build();
    let splash = adw::Window::builder()
        .title(app_settings::APP_DISPLAY_NAME)
        .default_width(380)
        .resizable(false)
        .content(&handle)
        .application(app)
        .build();
    if show {
        let splash = splash.clone();
        glib::timeout_add_local_once(Duration::from_millis(350), move || {
            if splash.application().is_some() && !splash.is_visible() && splash.content().is_some() {
                splash.present();
            }
        });
    }
    (splash, label)
}

fn rebuild_menu(a: &Rc<App>) {
    let lang = a.lang();
    let menu = gio::Menu::new();
    menu.append(Some(&tr_pl(&lang, "Preferencje")), Some("app.preferences"));
    menu.append(Some(&tr_pl(&lang, "Pokaż plik logu")), Some("app.logs"));
    menu.append(Some(&tr_pl(&lang, "O programie")), Some("app.about"));
    a.ui.get::<gtk4::MenuButton>("menu_button").set_menu_model(Some(&menu));
}

fn setup_actions(app: &adw::Application, a: &Rc<App>) {
    let preferences = gio::SimpleAction::new("preferences", None);
    {
        let a = a.clone();
        preferences.connect_activate(move |_, _| a.ui.prefs.present(Some(&a.ui.window)));
    }
    app.add_action(&preferences);

    // The log file is what a bug report needs: show it in the file manager.
    let logs = gio::SimpleAction::new("logs", None);
    {
        let a = a.clone();
        logs.connect_activate(move |_, _| {
            let file = gio::File::for_path(log::file_path());
            gtk4::FileLauncher::new(Some(&file)).open_containing_folder(
                Some(&a.ui.window),
                gio::Cancellable::NONE,
                |_| {},
            );
        });
    }
    app.add_action(&logs);

    let about = gio::SimpleAction::new("about", None);
    {
        let a = a.clone();
        about.connect_activate(move |_, _| show_about(&a));
    }
    app.add_action(&about);

    // Refresh: monitor info, system display mode, wallpaper and every setting.
    let a2 = a.clone();
    a.ui.get::<gtk4::Button>("refresh_button").connect_clicked(move |_| {
        start_profile_load(&a2);
        let _ = a2.tx().send(WorkerCmd::RefreshAll);
        refresh_system_display(&a2);
    });
}

/// About dialog: name, version and the links right away (no "Details" page).
fn show_about(a: &Rc<App>) {
    let lang = a.lang();
    let logo = match app_settings::resolve_logo_path() {
        Some(path) => gtk4::Image::from_file(path),
        None => gtk4::Image::from_icon_name(app_settings::APP_ICON_NAME),
    };
    logo.set_pixel_size(96);
    let content = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(12)
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .build();
    content.append(&logo);
    content.append(&gtk4::Label::builder().label(app_settings::APP_DISPLAY_NAME).css_classes(["title-1"]).build());
    content.append(&gtk4::Label::builder().label(app_settings::APP_VERSION).css_classes(["dim-label"]).build());
    let links = adw::PreferencesGroup::builder().margin_top(12).build();
    for (icon, title, url) in [
        ("web-browser-symbolic", tr_pl(&lang, "Strona programu"), "https://github.com/mkarenko/titan-control"),
        ("titan-bug-symbolic", tr_pl(&lang, "Zgłoś błąd"), "https://github.com/mkarenko/titan-control/issues/new"),
        ("titan-coffee-symbolic", "Buy me a coffee".to_string(), "https://buymeacoffee.com/mkarenko"),
    ] {
        let row = adw::ActionRow::builder().title(title).activatable(true).build();
        row.add_suffix(&gtk4::Image::from_icon_name(icon));
        let window = a.ui.window.clone();
        row.connect_activated(move |_| {
            gtk4::UriLauncher::new(url).launch(Some(&window), gio::Cancellable::NONE, |_| {});
        });
        links.add(&row);
    }
    content.append(&links);
    let view = adw::ToolbarView::new();
    view.add_top_bar(&adw::HeaderBar::new());
    view.set_content(Some(&content));
    let dialog = adw::Dialog::builder()
        .title(tr_pl(&lang, "O programie"))
        .content_width(400)
        .child(&view)
        .build();
    dialog.present(Some(&a.ui.window));
}

/// Connects every simple control with its VCP code.
fn bind_settings(a: &Rc<App>) {
    use monitor::*;
    let ui = &a.ui;
    let b = &a.bindings;
    let adj = |id: &str| -> gtk4::Adjustment { ui.get(id) };
    let toggles = |id: &str, values: &[u16]| Control::Toggles { group: ui.get(id), values: values.to_vec() };
    let combo = |id: &str, values: &[u16]| Control::Combo { row: ui.get(id), values: values.to_vec() };
    let switch = |id: &str, on: u16, off: u16| Control::Switch { row: ui.get(id), on, off };
    let expander = |id: &str| Control::Expander { row: ui.get(id) };
    let slider = |id: &str| Control::Adjustment { adj: adj(id), factor: 1 };

    // Sliders (shared adjustments show the setting on several pages).
    for (code, id) in [
        (VCP_BRIGHTNESS, "adj_brightness"),
        (VCP_CONTRAST, "adj_contrast"),
        (VCP_VOLUME, "adj_volume"),
        (VCP_LOW_BLUE, "adj_lbl"),
        (VCP_SHARPNESS, "adj_sharpness"),
        (VCP_COLOR_ENHANCE, "adj_color_enhance"),
        (VCP_CR_ENHANCE, "adj_cr_enhance"),
        (VCP_SHADOW_BALANCE, "adj_shadow_balance"),
        (VCP_SUPER_RES, "adj_super_res"),
        (VCP_HALO_CONTROL, "adj_halo"),
        (VCP_HUE_RED, "adj_hue_red"),
        (VCP_HUE_GREEN, "adj_hue_green"),
        (VCP_HUE_BLUE, "adj_hue_blue"),
        (VCP_HUE_CYAN, "adj_hue_cyan"),
        (VCP_HUE_MAGENTA, "adj_hue_magenta"),
        (VCP_HUE_YELLOW, "adj_hue_yellow"),
        (VCP_SATURATION_RED, "adj_sat_red"),
        (VCP_SATURATION_GREEN, "adj_sat_green"),
        (VCP_SATURATION_BLUE, "adj_sat_blue"),
        (VCP_SATURATION_CYAN, "adj_sat_cyan"),
        (VCP_SATURATION_MAGENTA, "adj_sat_magenta"),
        (VCP_SATURATION_YELLOW, "adj_sat_yellow"),
        (VCP_OSD_TIME, "adj_osd_time"),
        (VCP_OSD_H_POS, "adj_osd_h"),
        (VCP_OSD_V_POS, "adj_osd_v"),
    ] {
        b.bind(code, slider(id));
    }
    // OSD transparency: 6 OSD levels = DDC 0-100 in steps of 20.
    b.bind(VCP_OSD_TRANS, Control::Adjustment { adj: adj("adj_osd_trans"), factor: 20 });

    // Switches (on value, off value).
    b.bind(VCP_ADAPTIVE_SYNC, switch("adaptive_sync", 1, 0));
    b.bind(VCP_REAR_LED, switch("led", 0, 1));
    b.bind(VCP_OSD_LOCK, switch("osd_lock", 1, 0));
    b.bind(VCP_BUTTON_LOCK, switch("button_lock", values::BUTTONS_LOCKED, values::BUTTONS_UNLOCKED));
    b.bind(VCP_DCR, switch("dcr", 1, 0));
    // Mute: speaker button on the volume row (speaker / crossed-out speaker).
    let mute: gtk4::ToggleButton = ui.get("mute");
    mute.connect_toggled(|button| {
        button.set_icon_name(if button.is_active() { "audio-volume-muted-symbolic" } else { "audio-volume-high-symbolic" });
    });
    b.bind(VCP_MUTE, Control::Toggle { button: mute, on: 1, off: 0 });
    b.bind(VCP_USB_POWER_SLEEP, switch("usb_power_sleep", 1, 0));
    b.bind(VCP_QUICK_BOOT, switch("quick_boot", 1, 0));
    // Eyeshield reminder: over DDC only on (1 = 30 min) and off; the OSD can set 2-8.
    b.bind(VCP_EYESHIELD_REMINDER, switch("eyeshield", 1, 0));

    // Toggle groups (values in the order of the toggles).
    b.bind(VCP_HDR, toggles("hdr", &HDR_VALUES));
    b.bind(VCP_LOCAL_DIMMING, toggles("local_dimming", &LOCAL_DIMMING_VALUES));
    b.bind(VCP_COLOR_TEMP, toggles("color_temp", &COLOR_TEMP_VALUES));
    b.bind(VCP_GAMMA, toggles("gamma", &GAMMA_VALUES));
    b.bind(VCP_DYNAMIC_OD, toggles("dynamic_od", &DYNAMIC_OD_VALUES));
    b.bind(VCP_DYDS, toggles("dyds", &DYDS_VALUES));
    b.bind(VCP_NIGHT_VISION, toggles("night_vision", &NIGHT_VISION_VALUES));
    b.bind(VCP_SCREEN_SIZE, toggles("screen_size", &[0, 1]));
    // Positions in the UI: top-left, top-right, (center,) bottom-left, bottom-right.
    // Monitor values: 0 top-right, 1 top-left, 2 bottom-right, 3 bottom-left.
    let corners = [1, 0, 3, 2];
    b.bind(VCP_FPS_POS, toggles("fps_pos", &corners));
    b.bind(VCP_STOPWATCH_POS, toggles("stopwatch_pos", &corners));
    b.bind(VCP_GAME_TIME_POS, toggles("game_time_pos", &corners));
    // Magnifier: 1 top-right, 2 top-left, 5 center, 3 bottom-right, 4 bottom-left.
    b.bind(VCP_MAGNIFIER_POS, toggles("magnifier_pos", &[2, 1, 5, 4, 3]));
    // HawkEye: 0 top-right, 1 top-left, 2 center, 3 bottom-right, 4 bottom-left.
    b.bind(VCP_HAWKEYE_POS, toggles("hawkeye_pos", &[1, 0, 2, 4, 3]));
    b.bind(VCP_CROSSHAIR_SHAPE, toggles("crosshair_shape", &[1, 2, 3, 4, 5, 6]));
    b.bind(VCP_STOPWATCH_TIME, toggles("stopwatch_time", &[1, 2, 3, 4]));
    b.bind(VCP_GAME_TIME_VAL, toggles("game_time_val", &[1, 2, 3, 4]));
    b.bind(VCP_MAGNIFIER_ZOOM, toggles("magnifier_zoom", &[0, 1, 2]));
    b.bind(VCP_MAGNIFIER_SIZE, toggles("magnifier_size", &[1, 2, 3]));
    b.bind(VCP_HAWKEYE_SIZE, toggles("hawkeye_size", &[0, 1, 2]));
    b.bind(VCP_HAWKEYE_LEVEL, toggles("hawkeye_level", &[0, 1, 2, 3, 4]));
    b.bind(VCP_USB_HUB_SOURCE, toggles("usb_hub", &values::USB_HUB_SOURCE));
    b.bind(VCP_PIP_POSITION, toggles("pip_position", &corners));
    b.bind(VCP_PIP_SIZE, toggles("pip_size", &values::PIP_SIZE));
    b.bind(VCP_LED_STRENGTH, toggles("led_strength", &values::LED_STRENGTH));
    b.bind(VCP_POWER_LED, toggles("power_led", &values::POWER_LED));
    b.bind(VCP_POWER_SAVING, toggles("power_saving", &[0, 1, 2]));

    // Combos.
    let inputs = [0, 1, 3, 5, 6]; // auto, DP, USB-C, HDMI-1, HDMI-2 (0x57)
    b.bind(VCP_INPUT_SELECT, combo("input_select", &inputs));
    b.bind(VCP_OUTPUT_RANGE, combo("output_range", &OUTPUT_RANGE_VALUES));
    b.bind(VCP_RATIO, combo("ratio", &values::RATIO));
    // 21:9 needs a wider Full Game screen mode (3K Wide and similar) than Wide / 25", which are the only ones the
    // monitor accepts over DDC: the option is shown greyed out.
    {
        let factory = gtk4::SignalListItemFactory::new();
        factory.connect_setup(|_, item| {
            let Some(item) = item.downcast_ref::<gtk4::ListItem>() else { return };
            item.set_child(Some(&gtk4::Label::builder().xalign(0.0).build()));
        });
        factory.connect_bind(|_, item| {
            let Some(item) = item.downcast_ref::<gtk4::ListItem>() else { return };
            let Some(label) = item.child().and_downcast::<gtk4::Label>() else { return };
            let text = item.item().and_downcast::<gtk4::StringObject>().map(|text| text.string());
            label.set_label(text.as_deref().unwrap_or_default());
            let available = item.position() != RATIO_21_9_INDEX;
            label.set_sensitive(available);
            item.set_selectable(available);
            item.set_activatable(available);
        });
        ui.get::<adw::ComboRow>("ratio").set_list_factory(Some(&factory));
    }
    b.bind(VCP_PIP_MODE, combo("pip_mode", &values::PIP_MODE));
    b.bind(VCP_PIP_SOURCE, combo("pip_source", &values::PIP_SOURCE));
    b.bind(VCP_AUDIO_SOURCE, combo("audio_source", &values::AUDIO_SOURCE));
    b.bind(VCP_LED_MODE, combo("led_mode", &values::LED_MODE));
    b.bind(VCP_LED_FRONT_COLOR, combo("led_front", &values::LED_SIDE_COLOR));
    b.bind(VCP_LED_REAR_COLOR, combo("led_rear", &values::LED_SIDE_COLOR));
    b.bind(VCP_OSD_LANG, combo("osd_language", &crate::OSD_LANGUAGE_VALUES));

    // Expanders with an enable switch.
    for (code, id) in [
        (VCP_FPS_COUNTER, "fps_counter"),
        (VCP_CROSSHAIR, "crosshair"),
        (VCP_STOPWATCH, "stopwatch"),
        (VCP_GAME_TIME, "game_time"),
        (VCP_MAGNIFIER, "magnifier"),
        (VCP_HAWKEYE, "hawkeye"),
    ] {
        b.bind(code, expander(id));
    }

    // Color swatches.
    let buttons = |prefix: &str, count: usize| -> Vec<gtk4::ToggleButton> {
        (0..count).map(|index| ui.get(&format!("{prefix}{index}"))).collect()
    };
    b.bind(VCP_CROSSHAIR_COLOR, Control::Buttons { buttons: buttons("cross_c", 8), values: (0..8).collect() });
    let led_buttons: Vec<gtk4::ToggleButton> = (1..=7).map(|index| ui.get(&format!("led_c{index}"))).collect();
    b.bind(VCP_LED_COLOR, Control::Buttons { buttons: led_buttons, values: values::LED_COLOR.to_vec() });

    // Output Range: the monitor needs Auto (0) before another range.
    {
        let tx = a.tx().clone();
        b.set_sender(VCP_OUTPUT_RANGE, move |value| {
            let _ = tx.send(WorkerCmd::Set(VCP_OUTPUT_RANGE, 0));
            if value != 0 {
                let tx = tx.clone();
                glib::timeout_add_local_once(Duration::from_millis(250), move || {
                    let _ = tx.send(WorkerCmd::Set(VCP_OUTPUT_RANGE, value));
                });
            }
        });
    }
    // DyDs (except Off) and DCR exclude each other: switch DCR off first.
    {
        let bindings = Rc::downgrade(&a.bindings);
        let tx = a.tx().clone();
        b.set_sender(VCP_DYDS, move |value| {
            if let Some(bindings) = bindings.upgrade()
                && value != DYDS_VALUES[0]
                && bindings.get(VCP_DCR) == Some(1)
            {
                let _ = tx.send(WorkerCmd::Set(VCP_DCR, 0));
                bindings.apply(&HashMap::from([(VCP_DCR, 0)]), false);
            }
            let _ = tx.send(WorkerCmd::Set(VCP_DYDS, value));
        });
    }
}

/// Picture mode combo and the Default/Custom toggle, both written as VCP 0x22.
fn setup_picture_mode(a: &Rc<App>) {
    refresh_picture_models(a);
    {
        let combo: adw::ComboRow = a.ui.get("picture_mode");
        let a2 = a.clone();
        combo.connect_selected_notify(move |combo| {
            if bindings::suppressed() {
                return;
            }
            let Some(&mode) = a2.picture_order.borrow().get(combo.selected() as usize) else { return };
            let custom = a2.ui.get::<adw::ToggleGroup>("profile_variant").active() == 1;
            if let Some(value) = monitor::picture_mode_vcp_value(mode as usize, custom) {
                start_profile_load(&a2);
                a2.bindings.local_change(monitor::VCP_MODE, value);
            }
        });
    }
    let variant: adw::ToggleGroup = a.ui.get("profile_variant");
    let a2 = a.clone();
    variant.connect_active_notify(move |group| {
        if bindings::suppressed() {
            return;
        }
        let mode = a2
            .bindings
            .get(monitor::VCP_MODE)
            .and_then(monitor::picture_mode_from_vcp)
            .map(|(mode, _)| mode)
            .unwrap_or(0);
        if let Some(value) = monitor::picture_mode_vcp_value(mode, group.active() == 1) {
            start_profile_load(&a2);
            a2.bindings.local_change(monitor::VCP_MODE, value);
        }
    });
}

/// A profile change is being carried out: the profile controls stay locked until its settings are read (at most
/// 25 seconds, in case the monitor never answers).
fn start_profile_load(a: &Rc<App>) {
    a.profile_loading.set(true);
    let generation = a.profile_load_generation.get().wrapping_add(1);
    a.profile_load_generation.set(generation);
    let a2 = a.clone();
    glib::timeout_add_local_once(Duration::from_secs(25), move || {
        if a2.profile_load_generation.get() == generation {
            finish_profile_load(&a2);
        }
    });
    sync_state(a);
}

fn finish_profile_load(a: &Rc<App>) {
    if a.profile_loading.replace(false) {
        sync_state(a);
    }
}

/// Fills the picture mode combo.
fn refresh_picture_models(a: &Rc<App>) {
    let order: Vec<u16> = (0..monitor::PICTURE_MODE_NAMES.len() as u16).collect();
    let names: Vec<&str> = order.iter().map(|&mode| monitor::PICTURE_MODE_NAMES[mode as usize]).collect();
    *a.picture_order.borrow_mut() = order;
    bindings::suppress(|| {
        a.ui.get::<adw::ComboRow>("picture_mode").set_model(Some(&gtk4::StringList::new(&names)));
    });
    show_picture_mode(a);
}

/// Shows the active picture mode (from VCP 0x22) in the combos, the toggle, the star and the tray.
fn show_picture_mode(a: &Rc<App>) {
    let Some((mode, custom)) = a.bindings.get(monitor::VCP_MODE).and_then(monitor::picture_mode_from_vcp) else {
        return;
    };
    let index = a
        .picture_order
        .borrow()
        .iter()
        .position(|&candidate| candidate as usize == mode)
        .map(|index| index as u32)
        .unwrap_or(gtk4::INVALID_LIST_POSITION);
    bindings::suppress(|| {
        let combo: adw::ComboRow = a.ui.get("picture_mode");
        if combo.selected() != index {
            combo.set_selected(index);
        }
        let variant: adw::ToggleGroup = a.ui.get("profile_variant");
        variant.set_active(custom as u32);
        if let Some(star) = a.picture_star.borrow().as_ref() {
            star.set_active(a.cfg.borrow().is_favorite_picture_mode(mode as u16));
        }
    });
    let changed = a.tray_active_mode.lock().map(|mut active| active.replace(mode as u16) != Some(mode as u16));
    if changed.unwrap_or(false) {
        a.refresh_tray();
    }
}

/// RGB gains show and change the User 1-3 set of the selected color temperature.
fn setup_color_temp_rgb(a: &Rc<App>) {
    for (channel, id) in ["adj_red", "adj_green", "adj_blue"].into_iter().enumerate() {
        let adj: gtk4::Adjustment = a.ui.get(id);
        let a2 = a.clone();
        adj.connect_value_changed(move |adj| {
            if bindings::suppressed() {
                return;
            }
            let Some(temp) = a2.bindings.get(monitor::VCP_COLOR_TEMP) else { return };
            let Some(codes) = monitor::color_temp_rgb_codes(temp) else { return };
            let code = [codes.0, codes.1, codes.2][channel];
            a2.bindings.slider_change(code, adj.value().round() as u16);
        });
    }
}

/// With Color Enhance on, the monitor sets the six saturation axes itself (and locks them): the sliders show those
/// values. When it goes back to 0 the saturation the user had set before is restored (and sent to the monitor).
fn show_saturation(a: &Rc<App>) {
    let level = a.bindings.get(monitor::VCP_COLOR_ENHANCE).unwrap_or(0);
    let was_on = a.color_enhance_level.replace(level) > 0;
    let current = |index: usize| a.bindings.get(monitor::SATURATION_CODES[index]);
    let forced = monitor::color_enhance_saturation(level);
    let values: [Option<u16>; 6] = if let Some(forced) = forced {
        forced.map(Some)
    } else if was_on && let Some(saved) = *a.user_saturation.borrow() {
        for (index, code) in monitor::SATURATION_CODES.into_iter().enumerate() {
            if current(index) != Some(saved[index]) {
                a.bindings.local_change(code, saved[index]);
            }
        }
        saved.map(Some)
    } else {
        // Free sliders: remember what the user set, to restore it after Color Enhance.
        let now: [Option<u16>; 6] = std::array::from_fn(current);
        if let [Some(r), Some(g), Some(b), Some(c), Some(m), Some(y)] = now {
            *a.user_saturation.borrow_mut() = Some([r, g, b, c, m, y]);
        }
        now
    };
    // Free sliders are left alone while one is being dragged; locked ones always follow Color Enhance.
    if forced.is_none() && bindings::pointer_down() {
        return;
    }
    let ids = ["adj_sat_red", "adj_sat_green", "adj_sat_blue", "adj_sat_cyan", "adj_sat_magenta", "adj_sat_yellow"];
    bindings::suppress(|| {
        for (id, value) in ids.into_iter().zip(values) {
            if let Some(value) = value {
                a.ui.get::<gtk4::Adjustment>(id).set_value(value as f64);
            }
        }
    });
}

fn show_color_temp_rgb(a: &Rc<App>) {
    if bindings::pointer_down() {
        return;
    }
    let Some(temp) = a.bindings.get(monitor::VCP_COLOR_TEMP) else { return };
    let values = match monitor::builtin_color_temp_rgb(temp) {
        Some((r, g, b)) => Some([r, g, b]),
        None => monitor::color_temp_rgb_codes(temp).and_then(|(r, g, b)| {
            Some([a.bindings.get(r)?, a.bindings.get(g)?, a.bindings.get(b)?])
        }),
    };
    let Some(values) = values else { return };
    bindings::suppress(|| {
        for (id, value) in ["adj_red", "adj_green", "adj_blue"].into_iter().zip(values) {
            a.ui.get::<gtk4::Adjustment>(id).set_value(value as f64);
        }
    });
}

/// Buttons that send an action: PIP, power, resets.
fn setup_actions_rows(a: &Rc<App>) {
    for (id, code) in [("pip_swap", monitor::VCP_PIP_SWAP), ("pip_reset", monitor::VCP_PIP_RESET)] {
        let tx = a.tx().clone();
        a.ui.get::<adw::ButtonRow>(id).connect_activated(move |_| {
            let _ = tx.send(WorkerCmd::Set(code, 1));
        });
    }
    let confirm = |id: &str, title: &'static str, body: &'static str, action: &'static str, code: u8, value: u16| {
        let a2 = a.clone();
        a.ui.get::<adw::ButtonRow>(id).connect_activated(move |_| {
            let lang = a2.lang();
            let dialog = adw::AlertDialog::new(Some(&tr(&lang, title)), Some(&tr(&lang, body)));
            dialog.add_responses(&[("cancel", &tr(&lang, "cancel_btn")), ("confirm", &tr(&lang, action))]);
            dialog.set_response_appearance("confirm", adw::ResponseAppearance::Destructive);
            dialog.set_default_response(Some("cancel"));
            dialog.set_close_response("cancel");
            let tx = a2.tx().clone();
            dialog.connect_response(Some("confirm"), move |_, _| {
                let _ = tx.send(WorkerCmd::Set(code, value));
            });
            dialog.present(Some(&a2.ui.window));
        });
    };
    confirm(
        "power_off",
        "confirm_power_off_title",
        "confirm_power_off_body",
        "power_off",
        monitor::VCP_DPMS,
        monitor::values::POWER_OFF,
    );
    confirm(
        "reset_colors",
        "confirm_reset_colors_title",
        "confirm_reset_colors_body",
        "reset_btn",
        monitor::VCP_RESET_COLOR,
        1,
    );
    confirm(
        "reset_brightness",
        "confirm_reset_settings_title",
        "confirm_reset_settings_body",
        "reset_btn",
        monitor::VCP_RESET_BC,
        1,
    );
    confirm(
        "reset_factory",
        "confirm_reset_factory_title",
        "confirm_reset_factory_body",
        "reset_btn",
        monitor::VCP_RESET_FACTORY,
        1,
    );
}

/// Updates everything that depends on several settings: availability of controls, picture mode, RGB, tray.
fn sync_state(a: &Rc<App>) {
    use monitor::*;
    let state = a.bindings.state();
    let get = |code: u8| state.get(&code).copied();
    let mut allowed: HashMap<u8, bool> = HashMap::new();
    let mut allow = |code: u8, ok: bool| {
        *allowed.entry(code).or_insert(true) &= ok;
    };

    // Picture profile: the table settings can only be changed in a Custom profile.
    let custom = get(VCP_MODE).is_none_or(|mode| is_custom_profile(mode as u8));
    let loading = a.profile_loading.get();
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
    // Firmware 2025-09-20 runs DyDs with Adaptive-Sync. HawkEye works with Adaptive-Sync, the magnifier does not.
    let dyds_with_vrr = a.cfg.borrow().firmware_package.dyds_with_adaptive_sync();
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
    allow(VCP_PIP_POSITION, pip == 1);
    allow(VCP_PIP_SIZE, pip == 1);
    a.ui.get::<adw::ButtonRow>("pip_swap").set_sensitive(pip != values::PIP_OFF);

    // LED effects: only with the LED on; some modes lock colors.
    let led_on = get(VCP_REAR_LED) != Some(values::REAR_LED_OFF);
    for code in [VCP_LED_MODE, VCP_LED_COLOR, VCP_LED_STRENGTH, VCP_LED_FRONT_COLOR, VCP_LED_REAR_COLOR] {
        allow(code, led_on && is_setting_available(&state, code));
    }
    a.ui.get::<adw::ExpanderRow>("led_sides").set_sensitive(led_on && is_setting_available(&state, VCP_LED_FRONT_COLOR));
    a.ui.get::<gtk4::ToggleButton>("led_c7").set_sensitive(led_mode_has_colorful(get(VCP_LED_MODE)));

    // Firmware 2025-09-20: with HDR on the monitor locks these settings; DyDs, Night Vision and the magnifier
    // only while Adaptive-Sync is off.
    let hdr_locked = a.cfg.borrow().firmware_package.dyds_with_adaptive_sync()
        && get(VCP_HDR).is_some_and(|value| value != HDR_VALUES[0]);
    if hdr_locked {
        for code in [
            VCP_BRIGHTNESS, VCP_CONTRAST, VCP_DCR, VCP_LOW_BLUE, VCP_COLOR_ENHANCE, VCP_CR_ENHANCE, VCP_SHADOW_BALANCE,
            VCP_OUTPUT_RANGE,
        ] {
            allow(code, false);
        }
        for code in [VCP_NIGHT_VISION, VCP_DYDS] {
            allow(code, adaptive);
        }
        allow(VCP_MAGNIFIER, false);
    }
    // Firmware 2025-09-20 with Adaptive-Sync on: DyDs offers only ULL 1-3 (the first four options are off).
    let ull_only = a.cfg.borrow().firmware_package.dyds_with_adaptive_sync() && adaptive;
    let dyds_group = a.ui.get::<adw::ToggleGroup>("dyds");
    for index in 0..4 {
        if let Some(toggle) = dyds_group.toggle(index) {
            toggle.set_enabled(!ull_only);
        }
    }
    // Local dimming stays adjustable between Low and High, but cannot be turned off.
    if let Some(toggle) = a.ui.get::<adw::ToggleGroup>("local_dimming").toggle(0) {
        toggle.set_enabled(!hdr_locked);
    }
    a.ui.get::<adw::ComboRow>("picture_mode").set_sensitive(!hdr_locked && !loading);
    a.ui.get::<gtk4::Widget>("row_profile_variant").set_sensitive(!hdr_locked && !loading);

    for (code, ok) in allowed {
        a.bindings.set_sensitive(code, ok);
    }

    // RGB gains: editable only for User 1-3 in a Custom profile.
    let user_temp = get(VCP_COLOR_TEMP).is_some_and(is_user_color_temp);
    for id in ["row_color_temp_red", "row_color_temp_green", "row_color_temp_blue"] {
        a.ui.get::<gtk4::Widget>(id).set_sensitive(custom && user_temp);
    }
    show_color_temp_rgb(a);
    show_saturation(a);

    show_picture_mode(a);
    sync_system_hdr(a);
    let hdr = get(VCP_HDR).and_then(|v| HDR_VALUES.iter().position(|&x| x == v)).unwrap_or(0);
    let gamma = get(VCP_GAMMA).and_then(|v| GAMMA_VALUES.iter().position(|&x| x == v)).unwrap_or(2);
    let changed = a.tray_choices.lock().map(|mut choices| {
        let before = (choices.hdr_selected, choices.gamma_selected, choices.hdr_enabled, choices.gamma_enabled);
        (choices.hdr_selected, choices.gamma_selected, choices.hdr_enabled, choices.gamma_enabled) =
            (hdr, gamma, true, custom);
        before != (hdr, gamma, true, custom)
    });
    if changed.unwrap_or(false) {
        a.refresh_tray();
    }
}

/// Mirrors the monitor HDR (any mode but Off) to the system HDR of its output, once the value is stable.
fn sync_system_hdr(a: &Rc<App>) {
    let Some(hdr) = a.bindings.get(monitor::VCP_HDR).map(|value| value != monitor::HDR_VALUES[0]) else { return };
    // Unknown system state (no HDR query result): still turn HDR on when the monitor has it on.
    if a.system_hdr.get().map_or(!hdr, |system| system == hdr) {
        return;
    }
    let generation = a.hdr_sync.get().wrapping_add(1);
    a.hdr_sync.set(generation);
    let a2 = a.clone();
    glib::timeout_add_local_once(HDR_SYNC_DELAY, move || {
        let current = a2.bindings.get(monitor::VCP_HDR).map(|value| value != monitor::HDR_VALUES[0]);
        let Some(output) = a2.system_display.borrow().as_ref().map(|state| state.output.clone()) else { return };
        if a2.hdr_sync.get() != generation || current != Some(hdr) || a2.system_hdr.get() == Some(hdr) {
            return;
        }
        a2.system_hdr.set(Some(hdr));
        sync_state(&a2);
        std::thread::spawn(move || {
            if let Err(error) = monitor::set_system_hdr(&output, hdr) {
                log!("[HDR] system HDR {}: {error}", if hdr { "on" } else { "off" });
            }
        });
    });
}

fn setup_system_display(a: &Rc<App>) {
    for id in ["sys_resolution", "sys_refresh", "sys_scaling"] {
        let a2 = a.clone();
        let id_owned = id.to_string();
        a.ui.get::<adw::ComboRow>(id).connect_selected_notify(move |combo| {
            if bindings::suppressed() || !combo.is_sensitive() {
                return;
            }
            let Some(state) = a2.system_display.borrow().clone() else { return };
            let resolution_index = a2.ui.get::<adw::ComboRow>("sys_resolution").selected() as usize;
            let Some(resolution) = state.available_resolutions.get(resolution_index).cloned() else { return };
            let rates = state.refresh_rates.get(&resolution).cloned().unwrap_or_default();
            let (refresh, scale) = match id_owned.as_str() {
                "sys_resolution" => (
                    rates
                        .iter()
                        .find(|rate| **rate == state.current_refresh)
                        .or_else(|| rates.first())
                        .cloned(),
                    Some(state.current_scale_percent),
                ),
                "sys_refresh" => (rates.get(combo.selected() as usize).cloned(), Some(state.current_scale_percent)),
                _ => (
                    Some(state.current_refresh.clone()),
                    state.scaling_options.get(combo.selected() as usize).copied(),
                ),
            };
            match monitor::set_system_display_mode(&state.output, &resolution, refresh.as_deref(), scale) {
                Ok(()) => {
                    let a3 = a2.clone();
                    glib::timeout_add_local_once(Duration::from_millis(350), move || refresh_system_display(&a3));
                }
                Err(error) => {
                    log!("[display] mode change failed: {error}");
                    let dialog = adw::AlertDialog::new(Some("Display mode change failed"), Some(&error.to_string()));
                    dialog.add_response("ok", "OK");
                    dialog.present(Some(&a2.ui.window));
                    refresh_system_display(&a2);
                }
            }
        });
    }
}

fn refresh_system_display(a: &Rc<App>) {
    let connector = a.connector.borrow().clone();
    let combos: Vec<adw::ComboRow> =
        ["sys_resolution", "sys_refresh", "sys_scaling"].iter().map(|id| a.ui.get(id)).collect();
    let group: adw::PreferencesGroup = a.ui.get("sys_group");
    let mut wallpaper = None;
    let fill = |combo: &adw::ComboRow, items: &[String], selected: &str| {
        let refs: Vec<&str> = items.iter().map(String::as_str).collect();
        combo.set_model(Some(&gtk4::StringList::new(&refs)));
        combo.set_selected(items.iter().position(|item| item == selected).unwrap_or(0) as u32);
    };
    bindings::suppress(|| {
        match monitor::query_system_display_state((!connector.is_empty()).then_some(connector.as_str())) {
            Ok(state) => {
                group.set_description(None);
                fill(&combos[0], &state.available_resolutions, &state.current_resolution);
                let rates = state.refresh_rates.get(&state.current_resolution).cloned().unwrap_or_default();
                fill(&combos[1], &rates, &state.current_refresh);
                let scales: Vec<String> = state.scaling_options.iter().map(|value| format!("{value}%")).collect();
                fill(&combos[2], &scales, &format!("{}%", state.current_scale_percent));
                combos.iter().for_each(|combo| combo.set_sensitive(true));
                a.system_hdr.set(state.hdr);
                wallpaper = monitor::system_wallpaper(&state);
                *a.system_display.borrow_mut() = Some(state);
            }
            Err(error) => {
                group.set_description(Some(&error.to_string()));
                for combo in &combos {
                    fill(combo, &["—".to_string()], "—");
                    combo.set_sensitive(false);
                }
                a.system_hdr.set(None);
                *a.system_display.borrow_mut() = None;
            }
        }
    });
    a.ui.show_wallpaper(wallpaper);
    sync_state(a);
}

fn setup_preferences(a: &Rc<App>) {
    let cfg = a.cfg.borrow().clone();
    let ui = &a.ui;
    let language: adw::ComboRow = ui.get("pref_language");
    let theme: adw::ComboRow = ui.get("pref_theme");
    let tray_icon: adw::ComboRow = ui.get("pref_tray_icon");
    let autostart: adw::SwitchRow = ui.get("pref_autostart");
    let minimized: adw::SwitchRow = ui.get("pref_start_minimized");
    let modifier: adw::ComboRow = ui.get("pref_favorite_modifier");

    bindings::suppress(|| {
        // 0 = system, 1 = English, 2 = Polish.
        language.set_selected(match cfg.language {
            None => 0,
            Some(index) if i18n::lang_from_index(index) == AppLang::PL => 2,
            Some(_) => 1,
        });
        theme.set_selected(cfg.theme);
        tray_icon.set_selected(match cfg.tray_icon_style {
            TrayIconStyle::Theme => 0,
            TrayIconStyle::Light => 1,
            TrayIconStyle::Dark => 2,
        });
        autostart.set_active(cfg.auto_start);
        minimized.set_active(cfg.start_minimized);
        minimized.set_sensitive(cfg.auto_start);
        modifier.set_selected(StepModifier::ALL.iter().position(|m| *m == cfg.favorite_modifier).unwrap_or(0) as u32);
    });
    a.ui.set_favorite_modifier(cfg.favorite_modifier);

    {
        let a = a.clone();
        language.connect_selected_notify(move |combo| {
            if bindings::suppressed() {
                return;
            }
            let saved = match combo.selected() {
                1 => Some(i18n::lang_index(&AppLang::EN)),
                2 => Some(i18n::lang_index(&AppLang::PL)),
                _ => None,
            };
            let lang = resolve_lang(saved);
            a.lang.set(lang);
            a.save_cfg(|cfg| cfg.language = saved);
            a.update_info_texts();
            a.ui.translator.apply(&lang);
            rebuild_menu(&a);
            if let Ok(mut shortcuts) = a.tray_shortcuts.lock() {
                *shortcuts = favorite_tray_shortcut_entries(&a.cfg.borrow(), &lang);
            }
            a.refresh_tray();
        });
    }
    {
        let a = a.clone();
        theme.connect_selected_notify(move |combo| {
            if bindings::suppressed() {
                return;
            }
            apply_theme(combo.selected());
            a.save_cfg(|cfg| cfg.theme = combo.selected());
        });
    }
    {
        let a = a.clone();
        let minimized = minimized.clone();
        autostart.connect_active_notify(move |row| {
            minimized.set_sensitive(row.is_active());
            a.save_cfg(|cfg| cfg.auto_start = row.is_active());
            let _ = app_settings::sync_autostart(&a.cfg.borrow());
        });
    }
    {
        let a = a.clone();
        minimized.connect_active_notify(move |row| a.save_cfg(|cfg| cfg.start_minimized = row.is_active()));
    }
    {
        let a = a.clone();
        modifier.connect_selected_notify(move |combo| {
            let modifier = StepModifier::ALL.get(combo.selected() as usize).copied().unwrap_or_default();
            a.ui.set_favorite_modifier(modifier);
            a.save_cfg(|cfg| cfg.favorite_modifier = modifier);
        });
    }
}

/// Firmware package (chosen by the user; DDC reports 5.1.1 for both packages).
fn setup_info(a: &Rc<App>) {
    let package: adw::ComboRow = a.ui.get("firmware_package");
    {
        let cfg = a.cfg.borrow();
        bindings::suppress(|| {
            package.set_selected(FirmwarePackage::ALL.iter().position(|p| *p == cfg.firmware_package).unwrap_or(0) as u32);
        });
    }
    let a = a.clone();
    package.connect_selected_notify(move |combo| {
        if bindings::suppressed() {
            return;
        }
        let Some(&value) = FirmwarePackage::ALL.get(combo.selected() as usize) else { return };
        a.save_cfg(|cfg| cfg.firmware_package = value);
        let _ = a.tx().send(WorkerCmd::DydsWithAdaptiveSync(value.dyds_with_adaptive_sync()));
        sync_state(&a);
    });
}

fn show_monitor_info(a: &Rc<App>, info: &monitor::MonitorInfo) {
    let ui = &a.ui;
    let usage = format_usage_minutes(info.usage_mins);
    ui.get::<gtk4::Label>("ov_monitor_title").set_label(&info.name);
    ui.get::<gtk4::Label>("ov_monitor_subtitle").set_label(&format!(
        "{} · {} Hz · {}",
        info.resolution.replace('x', "×"),
        info.hz,
        usage
    ));
    ui.get::<adw::ActionRow>("info_model").set_subtitle(&info.name);
    ui.get::<adw::ActionRow>("info_usage").set_subtitle(&usage);
    let label = if info.connector.is_empty() {
        info.name.clone()
    } else {
        format!("{} ({})", info.name, info.connector.split_once('-').map(|(_, c)| c).unwrap_or(&info.connector))
    };
    // The app drives one monitor; the selector appears only when there is a choice.
    let select: gtk4::DropDown = ui.get("monitor_select");
    select.set_model(Some(&gtk4::StringList::new(&[label.as_str()])));
    select.set_visible(select.model().is_some_and(|model| model.n_items() > 1));
}

fn receive_monitor_messages(
    a: &Rc<App>,
    ui_rx: async_channel::Receiver<monitor::UiCmd>,
    splash: (adw::Window, gtk4::Label),
    app: adw::Application,
) {
    let a = a.clone();
    glib::spawn_future_local(async move {
        let (splash, splash_label) = splash;
        while let Ok(message) = ui_rx.recv().await {
            match message {
                monitor::UiCmd::MonitorFound(info) => {
                    *a.connector.borrow_mut() = info.connector.clone();
                    show_monitor_info(&a, &info);
                    a.bindings.apply(&info.settings, true);
                    *a.info.borrow_mut() = Some(info);
                    refresh_system_display(&a);
                    splash.close();
                    if !a.cfg.borrow().start_minimized {
                        a.ui.window.present();
                    }
                }
                monitor::UiCmd::Settings(values) => a.bindings.apply(&values, true),
                monitor::UiCmd::ProfileLoaded => finish_profile_load(&a),
                monitor::UiCmd::Corrected(values) => a.bindings.apply(&values, false),
                monitor::UiCmd::Progress(message) => {
                    splash_label.set_label(&translate_startup_message(&a.lang(), &message));
                }
                monitor::UiCmd::Error(error) => {
                    splash_label.set_label(&translate_startup_message(&a.lang(), &error));
                    if !splash.is_visible() {
                        splash.present();
                    }
                    if error == "Monitor not found" {
                        let app = app.clone();
                        glib::timeout_add_local_once(Duration::from_secs(3), move || app.quit());
                    }
                }
                monitor::UiCmd::UsageTimeUpdated(usage_mins) => {
                    a.ui.get::<adw::ActionRow>("info_usage").set_subtitle(&format_usage_minutes(usage_mins));
                }
                monitor::UiCmd::InfoRefreshed { hz, usage_mins } => {
                    let mut info = a.info.borrow_mut();
                    let Some(info) = info.as_mut() else { continue };
                    if let Some(hz) = hz {
                        info.hz = hz;
                    }
                    if let Some(usage_mins) = usage_mins {
                        info.usage_mins = usage_mins;
                    }
                    if let Some(state) = a.system_display.borrow().as_ref() {
                        info.resolution = state.current_resolution.clone();
                    }
                    show_monitor_info(&a, info);
                }
            }
        }
    });
}

// ───────────────────────────── tray ─────────────────────────────

fn favorite_profile_entries(cfg: &AppSettings) -> Vec<tray::TrayFavoriteProfile> {
    cfg.favorite_picture_mode_set()
        .into_iter()
        .filter_map(|mode| {
            monitor::PICTURE_MODE_NAMES
                .get(mode as usize)
                .map(|name| tray::TrayFavoriteProfile { mode, label: (*name).to_string() })
        })
        .collect()
}

fn tray_shortcut_label(id: &str, lang: &AppLang) -> String {
    let axis = |prefix: &str, key: &str| format!("{}: {}", tr(lang, prefix), tr(lang, key));
    match id {
        "color_temp_red" => axis("rgb_gain", "red"),
        "color_temp_green" => axis("rgb_gain", "green"),
        "color_temp_blue" => axis("rgb_gain", "blue"),
        _ => {
            for prefix in ["hue", "saturation"] {
                if let Some(color) = id.strip_prefix(prefix).and_then(|rest| rest.strip_prefix('_')) {
                    return axis(prefix, &format!("axis_{color}"));
                }
            }
            tr(lang, id)
        }
    }
}

fn favorite_tray_shortcut_entries(cfg: &AppSettings, lang: &AppLang) -> Vec<tray::TrayShortcut> {
    cfg.favorite_tray_control_set()
        .into_iter()
        .filter(|id| SHORTCUTS.iter().any(|(known, _, _)| known == id))
        .map(|id| tray::TrayShortcut { label: tray_shortcut_label(&id, lang), id })
        .collect()
}

/// Stars: picture modes (always visible) and tray shortcuts (while the favorites key is held), up to 5 each.
fn setup_favorites(a: &Rc<App>) {
    let picture_star = gtk4::ToggleButton::builder()
        .icon_name("starred-symbolic")
        .valign(gtk4::Align::Center)
        .tooltip_text("Tray")
        .css_classes(["flat", "picture-star"])
        .build();
    a.ui.get::<adw::ComboRow>("picture_mode").add_suffix(&picture_star);
    {
        let a2 = a.clone();
        picture_star.connect_toggled(move |star| {
            if bindings::suppressed() {
                return;
            }
            let Some((mode, _)) = a2.bindings.get(monitor::VCP_MODE).and_then(monitor::picture_mode_from_vcp) else {
                return;
            };
            let mode = mode as u16;
            if star.is_active() && !a2.cfg.borrow().is_favorite_picture_mode(mode) && a2.cfg.borrow().favorite_picture_mode_set().len() >= 5 {
                bindings::suppress(|| star.set_active(false));
                return;
            }
            a2.save_cfg(|cfg| {
                cfg.favorite_picture_modes.retain(|&favorite| favorite != mode);
                if star.is_active() {
                    cfg.favorite_picture_modes.push(mode);
                }
            });
            if let Ok(mut favorites) = a2.tray_favorites.lock() {
                *favorites = favorite_profile_entries(&a2.cfg.borrow());
            }
            a2.refresh_tray();
        });
    }
    *a.picture_star.borrow_mut() = Some(picture_star);

    for &(id, row, _) in SHORTCUTS {
        let star = a.ui.add_favorite_star(row, a.cfg.borrow().is_favorite_tray_control(id));
        let a2 = a.clone();
        star.connect_toggled(move |star| {
            if bindings::suppressed() {
                return;
            }
            if star.is_active() && a2.cfg.borrow().favorite_tray_control_set().len() >= 5 {
                bindings::suppress(|| star.set_active(false));
                return;
            }
            a2.save_cfg(|cfg| {
                cfg.favorite_tray_controls.retain(|favorite| favorite != id);
                if star.is_active() {
                    cfg.favorite_tray_controls.push(id.to_string());
                }
            });
            if let Ok(mut shortcuts) = a2.tray_shortcuts.lock() {
                *shortcuts = favorite_tray_shortcut_entries(&a2.cfg.borrow(), &a2.lang());
            }
            a2.refresh_tray();
        });
        a.stars.borrow_mut().insert(id, star);
    }
}

fn run_shortcut(a: &Rc<App>, id: &str) {
    let Some(&(_, _, action)) = SHORTCUTS.iter().find(|(known, _, _)| *known == id) else { return };
    match action {
        // Sliders are offered as ready values in the tray menu (see `select_shortcut_option`).
        Shortcut::Slider(_) => {}
        Shortcut::Toggle(code) => {
            if !a.bindings.is_sensitive(code) {
                return;
            }
            match a.bindings.control(code) {
                Some(Control::Switch { row, .. }) => row.set_active(!row.is_active()),
                Some(Control::Expander { row }) => row.set_enable_expansion(!row.enables_expansion()),
                _ => {}
            }
        }
        Shortcut::Choice(code) => {
            if let Some(Control::Toggles { group, .. }) = a.bindings.control(code)
                && a.bindings.is_sensitive(code)
                && group.n_toggles() > 0
            {
                group.set_active((group.active().wrapping_add(1)) % group.n_toggles());
            }
        }
    }
}

fn select_shortcut_option(a: &Rc<App>, id: &str, index: usize) {
    let Some(&(_, row, action)) = SHORTCUTS.iter().find(|(known, _, _)| *known == id) else { return };
    // Sliders: the tray menu offers ready values; `index` is the value.
    if let Shortcut::Slider(adjustment) = action {
        if a.ui.get::<gtk4::Widget>(row).is_sensitive() {
            a.ui.get::<gtk4::Adjustment>(adjustment).set_value(index as f64);
        }
        return;
    }
    let Shortcut::Choice(code) = action else { return };
    if let Some(Control::Toggles { group, .. }) = a.bindings.control(code)
        && a.bindings.is_sensitive(code)
        && (index as u32) < group.n_toggles()
    {
        group.set_active(index as u32);
    }
}

fn setup_tray(app: &adw::Application, a: &Rc<App>) {
    let style = a.cfg.borrow().tray_icon_style;
    let select_favorite = {
        let a = a.clone();
        Rc::new(move |mode: u16| {
            if let Some(index) = a.picture_order.borrow().iter().position(|&candidate| candidate == mode) {
                a.ui.get::<adw::ComboRow>("picture_mode").set_selected(index as u32);
            }
        })
    };
    let shortcut = {
        let a = a.clone();
        Rc::new(move |id: &str| run_shortcut(&a, id))
    };
    let option = {
        let a = a.clone();
        Rc::new(move |id: &str, index: usize| select_shortcut_option(&a, id, index))
    };
    let control = tray::setup_tray(
        app,
        &a.ui.window,
        app.hold(),
        a.lang(),
        a.tray_favorites.clone(),
        a.tray_active_mode.clone(),
        a.tray_shortcuts.clone(),
        a.tray_window_visible.clone(),
        a.tray_choices.clone(),
        style,
        select_favorite,
        shortcut,
        option,
    );
    *a.tray.borrow_mut() = Some(control.clone());
    let a2 = a.clone();
    a.ui.get::<adw::ComboRow>("pref_tray_icon").connect_selected_notify(move |combo| {
        if bindings::suppressed() {
            return;
        }
        let style = match combo.selected() {
            1 => TrayIconStyle::Light,
            2 => TrayIconStyle::Dark,
            _ => TrayIconStyle::Theme,
        };
        a2.save_cfg(|cfg| cfg.tray_icon_style = style);
        control.set_style(style);
    });
}
