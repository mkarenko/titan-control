
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AppLang {
    EN,
    PL,
    DE,
    ES,
    FR,
    UA,
}

pub fn detect_system_lang() -> AppLang {
    let lang = std::env::var("LANG").unwrap_or_default();
    if lang.starts_with("pl") {
        AppLang::PL
    } else if lang.starts_with("de") {
        AppLang::DE
    } else if lang.starts_with("es") {
        AppLang::ES
    } else if lang.starts_with("fr") {
        AppLang::FR
    } else if lang.starts_with("uk") {
        AppLang::UA
    } else {
        AppLang::EN
    }
}

pub fn lang_index(lang: &AppLang) -> u32 {
    match lang {
        AppLang::EN => 0,
        AppLang::PL => 1,
        AppLang::DE => 2,
        AppLang::ES => 3,
        AppLang::FR => 4,
        AppLang::UA => 5,
    }
}

pub fn lang_from_index(idx: u32) -> AppLang {
    match idx {
        1 => AppLang::PL,
        2 => AppLang::DE,
        3 => AppLang::ES,
        4 => AppLang::FR,
        5 => AppLang::UA,
        _ => AppLang::EN,
    }
}

pub fn tr(lang: &AppLang, key: &str) -> String {
    match lang {
        AppLang::PL => tr_pl(key),
        _ => tr_en(key),
    }
}

fn tr_en(key: &str) -> String {
    match key {
        // Power
        "power_off" => "Turn off",

        // Reset
        "reset_btn" => "Reset",
        "cancel_btn" => "Cancel",
        "confirm_reset_colors_title" => "Reset colors?",
        "confirm_reset_colors_body" => "The colors return to their factory values (color temperature: Warm).",
        "confirm_reset_settings_title" => "Reset brightness and contrast?",
        "confirm_reset_settings_body" => "Brightness and contrast return to their factory values (90 / 50).",
        "confirm_reset_factory_title" => "Restore factory settings?",
        "confirm_reset_factory_body" => "All monitor settings return to their factory values.",
        "confirm_power_off_title" => "Turn the monitor off?",
        "confirm_power_off_body" => "Only the monitor's power button turns it on again.",

        // Profiles
        "brightness" => "Brightness",
        "contrast" => "Contrast",
        "sharpness" => "Sharpness",
        "shadow_balance" => "Shadow Balance",
        "cr_enhance" => "CR Enhance",
        "color_enhance" => "Color Enhance",
        "super_res" => "Super Res",
        "dcr" => "DCR",
        "low_blue_light" => "Low Blue Light",
        "color_temp_profile" => "Color temperature",
        "rgb_gain" => "RGB",
        "red" => "Red",
        "green" => "Green",
        "blue" => "Blue",
        "axis_red" => "Red",
        "axis_green" => "Green",
        "axis_blue" => "Blue",
        "axis_cyan" => "Cyan",
        "axis_magenta" => "Magenta",
        "axis_yellow" => "Yellow",
        "hdr" => "HDR",
        "gamma" => "Gamma",
        "night_vision" => "Night Vision",
        "dynamic_od" => "Dynamic OD",
        "hue" => "Hue",
        "saturation" => "Saturation",

        // Gaming - Game Aid
        "screen_size" => "Screen size (DualMode)",
        "fps_counter" => "FPS/Hz counter",
        "crosshair" => "Crosshair",
        "stopwatch" => "Stopwatch",
        "game_time" => "Game time",
        "magnifier" => "Magnifier",
        "hawkeye" => "Hawkeye Vision",

        // Gaming - Picture Enhance
        "adaptive_sync" => "Adaptive-Sync",
        "local_dimming" => "Local dimming",
        "dyds" => "DyDs",
        "halo_control" => "Halo Control",

        // Splash / Tray
        "splash_searching" => "Searching for monitor...",
        "splash_detecting_monitors" => "Detecting monitors...",
        "splash_searching_monitors" => "Searching for monitors...",
        "splash_loading_cached" => "Loading cached settings...",
        "splash_reading_profile" => "Reading the picture profile...",
        "splash_monitor_not_found" => "Monitor not found",
        "tray_show" => "Show",
        "tray_hide" => "Hide",
        "tray_quit" => "Quit",
        "tray_sliders" => "Sliders",
        "tray_options" => "Options",
        "tray_toggles" => "Toggles",

        _ => key,
    }
    .into()
}

fn tr_pl(key: &str) -> String {
    match key {
        // Power
        "power_off" => "Wyłącz",

        // Reset
        "reset_btn" => "Resetuj",
        "cancel_btn" => "Anuluj",
        "confirm_reset_colors_title" => "Zresetować kolory?",
        "confirm_reset_colors_body" => "Kolory wrócą do wartości fabrycznych (temperatura barwowa: Warm).",
        "confirm_reset_settings_title" => "Zresetować jasność i kontrast?",
        "confirm_reset_settings_body" => "Jasność i kontrast wrócą do wartości fabrycznych (90 / 50).",
        "confirm_reset_factory_title" => "Przywrócić ustawienia fabryczne?",
        "confirm_reset_factory_body" => "Wszystkie ustawienia monitora wrócą do wartości fabrycznych.",
        "confirm_power_off_title" => "Wyłączyć monitor?",
        "confirm_power_off_body" => "Włączysz go ponownie tylko przyciskiem zasilania na monitorze.",

        // Profiles
        "brightness" => "Jasność",
        "contrast" => "Kontrast",
        "sharpness" => "Ostrość",
        "shadow_balance" => "Shadow Balance",
        "cr_enhance" => "CR Enhance",
        "color_enhance" => "Color Enhance",
        "super_res" => "Super Res",
        "dcr" => "DCR",
        "low_blue_light" => "Low Blue Light",
        "color_temp_profile" => "Temperatura barwowa",
        "rgb_gain" => "RGB",
        "red" => "Czerwony (R)",
        "green" => "Zielony (G)",
        "blue" => "Niebieski (B)",
        "axis_red" => "Czerwony (R)",
        "axis_green" => "Zielony (G)",
        "axis_blue" => "Niebieski (B)",
        "axis_cyan" => "Cyjan (C)",
        "axis_magenta" => "Magenta (M)",
        "axis_yellow" => "Żółty (Y)",
        "hdr" => "HDR",
        "gamma" => "Gamma",
        "night_vision" => "Night Vision",
        "dynamic_od" => "Dynamic OD",
        "hue" => "Odcień",
        "saturation" => "Nasycenie",

        // Gaming - Game Aid
        "screen_size" => "Rozmiar ekranu (DualMode)",
        "fps_counter" => "Licznik FPS/Hz",
        "crosshair" => "Celownik",
        "stopwatch" => "Stoper",
        "game_time" => "Czas gry",
        "magnifier" => "Lupa",
        "hawkeye" => "Hawkeye Vision",

        // Gaming - Picture Enhance
        "adaptive_sync" => "Adaptive-Sync",
        "local_dimming" => "Lokalne przyciemnianie",
        "dyds" => "DyDs",
        "halo_control" => "Halo Control",

        // Splash / Tray
        "splash_searching" => "Szukam monitora...",
        "splash_detecting_monitors" => "Wykrywam monitory...",
        "splash_searching_monitors" => "Szukam monitorów...",
        "splash_loading_cached" => "Wczytuję zapisane ustawienia...",
        "splash_reading_profile" => "Odczytuję profil obrazu...",
        "splash_monitor_not_found" => "Nie znaleziono monitora",
        "tray_show" => "Pokaż",
        "tray_hide" => "Ukryj",
        "tray_quit" => "Wyjdź",
        "tray_sliders" => "Suwaki",
        "tray_options" => "Opcje",
        "tray_toggles" => "Przełączniki",

        _ => return tr_en(key),
    }
    .into()
}
