use std::cell::RefCell;
use std::rc::Rc;

pub type LangUpdaters = Rc<RefCell<Vec<Box<dyn Fn(&AppLang)>>>>;

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
        // Tabs
        "tab_main" => "Display",
        "tab_profiles" => "Profiles",
        "tab_gaming" => "Gaming",
        "tab_info" => "Info",

        // Audio
        "audio_group" => "Audio",
        "volume" => "Volume",
        "mute" => "Mute",

        // Device
        "device_group" => "Device",
        "model" => "Model",
        "refresh_rate" => "Refresh Rate",
        "controller" => "Controller",
        "firmware" => "Firmware",
        "usage_time" => "Usage Time",

        // Power
        "power_group" => "Power",
        "power_off" => "Turn off display",
        "power_button" => "Power button",
        "power_saving" => "Power Saving",
        "power_led" => "LED Indicator",

        // I/O
        "io_group" => "I/O & Signal",
        "input_source" => "Source",
        "output_range" => "Output Range",
        "quick_boot" => "Quick Boot",

        // OSD
        "osd_group" => "OSD Settings",
        "osd_lang" => "Language",
        "osd_time" => "Display Time",
        "osd_h_pos" => "H Position",
        "osd_v_pos" => "V Position",
        "osd_trans" => "Transparency",

        // Reset
        "reset_group" => "Reset",
        "reset_factory" => "Factory Settings",
        "reset_br_con" => "Brightness & Contrast",
        "reset_colors" => "RGB Colors",
        "reset_btn" => "Reset",

        // Profiles
        "display_mode" => "Display Mode",
        "active_mode" => "Active mode",
        "manual_settings" => "Manual Settings",
        "custom_config" => "Custom Configuration",
        "brightness" => "Brightness",
        "contrast" => "Contrast",
        "sharpness" => "Sharpness",
        "shadow_balance" => "Shadow Balance",
        "cr_enhance" => "CR Enhance",
        "color_enhance" => "Color Enhance",
        "super_res" => "Super Resolution",
        "low_blue_light" => "Low Blue Light",
        "color_temp" => "Color Temperature",
        "color_temp_profile" => "Profile",
        "red" => "Red",
        "green" => "Green",
        "blue" => "Blue",
        "lists_group" => "Selection Lists",
        "hdr" => "HDR",
        "gamma" => "Gamma",
        "night_vision" => "Night Vision",
        "dynamic_od" => "Dynamic OD",
        "hue" => "Hue",
        "saturation" => "Color Saturation",

        // Gaming - Game Aid
        "game_aid" => "Game Aid",
        "screen_size" => "Screen Size (DualMode)",
        "fps_counter" => "FPS/Hz Counter",
        "crosshair" => "Crosshair",
        "stopwatch" => "Stopwatch",
        "game_time" => "Game Time",
        "magnifier" => "Magnifier",
        "alignment_aid" => "Alignment",
        "hawkeye" => "Hawkeye Vision",
        "position" => "Position",
        "shape" => "Shape",
        "color" => "Color",
        "time_min" => "Time (min)",
        "zoom" => "Zoom",
        "size" => "Size",
        "level" => "Level",

        // Gaming - Picture Enhance
        "pic_enhance" => "Picture Enhance",
        "adaptive_sync" => "Adaptive-Sync",
        "game_rush" => "Game Rush",
        "local_dimming" => "Local Dimming",
        "dyds" => "DyDs",
        "shadow_enhance" => "Shadow Enhance",
        "super_resolution" => "Super Resolution",
        "halo_control" => "Halo Control",

        // Info / App settings
        "app_name" => "Titan Control",
        "app_desc" => "Monitor control application",
        "app_settings" => "Application Settings",
        "app_lang" => "Language",
        "app_theme" => "Application Theme",
        "theme_system" => "System theme",
        "theme_light" => "Light theme",
        "theme_dark" => "Dark theme",
        "auto_start" => "Start with system",
        "start_minimized" => "Start minimized",
        "about_app" => "About",
        "source_code" => "Source Code",
        "source_code_sub" => "GitHub Repository",
        "open_btn" => "Open",
        "report_bug" => "Report a Problem",
        "report_bug_sub" => "Found a bug or have a suggestion?",
        "report_btn" => "Report",
        "support_author" => "Support the Author",
        "support_author_sub" => "Help develop the project",
        "buy_coffee" => "Buy a coffee",
        "version" => "Version",

        // Splash / Tray
        "splash_searching" => "Searching for monitor...",
        "tray_show" => "Show",
        "tray_quit" => "Quit",

        // Misc
        "usage_fmt" => "{h} h {m} min",
        "hz_fmt" => "{hz} Hz",

        _ => key,
    }
    .into()
}

fn tr_pl(key: &str) -> String {
    match key {
        // Tabs
        "tab_main" => "Monitor",
        "tab_profiles" => "Profile",
        "tab_gaming" => "Gaming",
        "tab_info" => "Info",

        // Audio
        "audio_group" => "Dźwięk",
        "volume" => "Głośność",
        "mute" => "Wyciszenie",

        // Device
        "device_group" => "Urządzenie",
        "model" => "Model",
        "refresh_rate" => "Odświeżanie",
        "controller" => "Kontroler",
        "firmware" => "Firmware",
        "usage_time" => "Czas pracy",

        // Power
        "power_group" => "Zasilanie",
        "power_off" => "Wyłącz ekran",
        "power_button" => "Przycisk zasilania",
        "power_saving" => "Oszczędzanie energii",
        "power_led" => "Dioda LED",

        // I/O
        "io_group" => "Wejścia i Sygnał",
        "input_source" => "Źródło",
        "output_range" => "Output Range",
        "quick_boot" => "Quick Boot",

        // OSD
        "osd_group" => "Ustawienia OSD",
        "osd_lang" => "Język",
        "osd_time" => "Czas wyświetlania",
        "osd_h_pos" => "Pozycja H",
        "osd_v_pos" => "Pozycja V",
        "osd_trans" => "Przezroczystość",

        // Reset
        "reset_group" => "Resetowanie",
        "reset_factory" => "Ustawienia fabryczne",
        "reset_br_con" => "Jasność i Kontrast",
        "reset_colors" => "Kolory RGB",
        "reset_btn" => "Resetuj",

        // Profiles
        "display_mode" => "Tryb Wyświetlania",
        "active_mode" => "Aktywny tryb",
        "manual_settings" => "Ustawienia Ręczne",
        "custom_config" => "Konfiguracja Custom",
        "brightness" => "Jasność",
        "contrast" => "Kontrast",
        "sharpness" => "Ostrość",
        "shadow_balance" => "Shadow Balance",
        "cr_enhance" => "CR Enhance",
        "color_enhance" => "Color Enhance",
        "super_res" => "Super Res",
        "low_blue_light" => "Low Blue Light",
        "color_temp" => "Temperatura Kolorów",
        "color_temp_profile" => "Profil",
        "red" => "Czerwony",
        "green" => "Zielony",
        "blue" => "Niebieski",
        "lists_group" => "Listy Wyboru",
        "hdr" => "HDR",
        "gamma" => "Gamma",
        "night_vision" => "Night Vision",
        "dynamic_od" => "Dynamic OD",
        "hue" => "Odcień (Hue)",
        "saturation" => "Nasycenie kolorów",

        // Gaming - Game Aid
        "game_aid" => "Wspomaganie gry",
        "screen_size" => "Rozmiar ekranu (DualMode)",
        "fps_counter" => "Licznik FPS/Hz",
        "crosshair" => "Celownik",
        "stopwatch" => "Stoper",
        "game_time" => "Czas gry",
        "magnifier" => "Lupa",
        "alignment_aid" => "Wyrównanie",
        "hawkeye" => "Hawkeye Vision",
        "position" => "Pozycja",
        "shape" => "Kształt",
        "color" => "Kolor",
        "time_min" => "Czas (min)",
        "zoom" => "Powiększenie",
        "size" => "Rozmiar",
        "level" => "Poziom",

        // Gaming - Picture Enhance
        "pic_enhance" => "Ulepszanie obrazu",
        "adaptive_sync" => "Adaptive-Sync",
        "game_rush" => "Game Rush",
        "local_dimming" => "Lokalne przyciemnianie",
        "dyds" => "DyDs",
        "shadow_enhance" => "Shadow Enhance",
        "super_resolution" => "Super Resolution",
        "halo_control" => "Halo Control",

        // Info / App settings
        "app_name" => "Titan Control",
        "app_desc" => "Aplikacja do sterowania monitorem",
        "app_settings" => "Ustawienia Aplikacji",
        "app_lang" => "Język aplikacji",
        "app_theme" => "Motyw aplikacji",
        "theme_system" => "Motyw systemowy",
        "theme_light" => "Jasny motyw",
        "theme_dark" => "Ciemny motyw",
        "auto_start" => "Uruchom z systemem",
        "start_minimized" => "Uruchom zminimalizowany",
        "about_app" => "O programie",
        "source_code" => "Kod źródłowy",
        "source_code_sub" => "GitHub Repository",
        "open_btn" => "Otwórz",
        "report_bug" => "Zgłoś problem",
        "report_bug_sub" => "Masz błąd lub propozycję?",
        "report_btn" => "Zgłoś",
        "support_author" => "Wesprzyj autora",
        "support_author_sub" => "Pomóż w rozwoju projektu",
        "buy_coffee" => "Kup kawę ☕",
        "version" => "Wersja",

        // Splash / Tray
        "splash_searching" => "Szukam monitora...",
        "tray_show" => "Pokaż",
        "tray_quit" => "Wyjdź",

        // Misc
        "usage_fmt" => "{h} h {m} min",
        "hz_fmt" => "{hz} Hz",

        _ => return tr_en(key),
    }
    .into()
}
