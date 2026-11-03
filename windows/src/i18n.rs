//! Translations.
//!
//! The user interface sources are Polish, like the Linux application's Blueprint files: a text is looked up in the
//! Polish → English dictionary and returned unchanged for Polish. Languages without a dictionary use English, which
//! matches the Linux application (only English and Polish are translated there too).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppLang {
    EN,
    PL,
}

/// The language of Windows and of the current user.
pub fn detect_system_lang() -> AppLang {
    if system_is_polish() { AppLang::PL } else { AppLang::EN }
}

fn system_is_polish() -> bool {
    use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;
    // 0x0415 = Polish (Poland).
    let _ = GetUserDefaultUILanguage;
    let locale = std::env::var("LANG").unwrap_or_default();
    if locale.to_ascii_lowercase().starts_with("pl") {
        return true;
    }
    unsafe { primary_language() == 0x15 }
}

/// LANGID of the user interface language, read through `kernel32!GetUserDefaultUILanguage`.
unsafe fn primary_language() -> u16 {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetUserDefaultUILanguage() -> u16;
    }
    unsafe { GetUserDefaultUILanguage() & 0x03FF }
}

pub fn lang_index(lang: &AppLang) -> u32 {
    match lang {
        AppLang::EN => 0,
        AppLang::PL => 1,
    }
}

pub fn lang_from_index(index: u32) -> AppLang {
    match index {
        1 => AppLang::PL,
        _ => AppLang::EN,
    }
}

/// Translation of a Polish source text.
pub fn tr(lang: &AppLang, source: &str) -> String {
    match lang {
        AppLang::PL => source.to_string(),
        AppLang::EN => english(source).unwrap_or(source).to_string(),
    }
}

fn english(source: &str) -> Option<&'static str> {
    PL_EN.iter().find(|(pl, _)| *pl == source).map(|(_, en)| *en)
}

/// Polish source → English. The first part is the dictionary of the Linux user interface
/// (`src/ui/translate.rs`), the second part covers texts that only the Windows build shows.
const PL_EN: &[(&str, &str)] = &[
    // ── window, tabs ──
    ("Titan Control", "Titan Control"),
    ("Menu główne", "Main menu"),
    ("Odśwież informacje i ustawienia z monitora", "Read the monitor info and settings again"),
    ("Przegląd", "Overview"),
    ("Łączenie z monitorem…", "Connecting to the monitor…"),
    ("Tryb obrazu", "Picture mode"),
    ("Gaming", "Gaming"),
    ("Wejścia i dźwięk", "Inputs and audio"),
    ("Oświetlenie", "Lighting"),
    ("Monitor", "Monitor"),
    // ── picture mode ──
    ("Jasność", "Brightness"),
    ("Kontrast", "Contrast"),
    ("Głośność", "Volume"),
    ("Wycisz", "Mute"),
    ("Źródło sygnału", "Input source"),
    ("Oświetlenie LED", "LED lighting"),
    ("Blokada OSD", "OSD lock"),
    ("Aktywny tryb", "Active mode"),
    ("Ustawienia trybu", "Mode settings"),
    ("Domyślne", "Default"),
    ("Własne", "Custom"),
    ("Podstawowe", "Basic"),
    ("Ostrość", "Sharpness"),
    ("Dynamiczny kontrast", "Dynamic contrast"),
    ("Lokalne przyciemnianie", "Local dimming"),
    ("Tylko przy włączonym lokalnym przyciemnianiu", "Only with local dimming on"),
    ("Kolor", "Color"),
    ("Edytowalne tylko dla profili User 1–3", "Editable only for User 1–3"),
    ("Czerwony (R)", "Red (R)"),
    ("Zielony (G)", "Green (G)"),
    ("Niebieski (B)", "Blue (B)"),
    ("Kolor 6-osiowy", "6-axis color"),
    ("Temperatura barwowa", "Color temperature"),
    ("Efekty", "Effects"),
    (
        "W KDE Plasma (Wayland) włącza i wyłącza też HDR w systemie. Z firmware 2025-09-20 przy HDR włączonym w monitorze i w systemie zablokowane są: tryb obrazu, jasność, kontrast, DCR, Low Blue Light, Color Enhance, CR Enhance, Shadow Balance, Night Vision, DyDs, wyłączenie lokalnego przyciemniania, zakres RGB i lupa",
        "On KDE Plasma (Wayland) it also turns HDR on and off in the system. With firmware 2025-09-20, while HDR is on in both the monitor and the system, these are locked: picture mode, brightness, contrast, DCR, Low Blue Light, Color Enhance, CR Enhance, Shadow Balance, Night Vision, DyDs, turning local dimming off, RGB range and the magnifier",
    ),
    ("Niedostępna przy włączonym Adaptive-Sync", "Not available while Adaptive-Sync is on"),
    (
        "Eyeshield Reminder. Przez DDC można je tylko włączyć (co 30 min) lub wyłączyć; inny odstęp ustawisz w OSD",
        "Eyeshield Reminder. Over DDC it can only be turned on (every 30 min) or off; set another interval in the OSD",
    ),
    ("Podświetlenie", "Backlight"),
    ("Odcień (Hue)", "Hue"),
    ("Cyjan (C)", "Cyan (C)"),
    ("Magenta (M)", "Magenta (M)"),
    ("Żółty (Y)", "Yellow (Y)"),
    ("Nasycenie (Saturation)", "Saturation"),
    (
        "Przy włączonym Color Enhance nasycenie ustawia on sam: suwaki pokazują wartości dla jego poziomu i są zablokowane",
        "While Color Enhance is on it sets the saturation itself: the sliders show the values for its level and are locked",
    ),
    // ── gaming ──
    ("Wydajność", "Performance"),
    ("Rozmiar ekranu (DualMode)", "Screen size (DualMode)"),
    ("Nakładki", "Overlays"),
    ("Licznik FPS/Hz", "FPS/Hz counter"),
    ("Pozycja", "Position"),
    ("Prawy górny", "Top right"),
    ("Lewy górny", "Top left"),
    ("Prawy dolny", "Bottom right"),
    ("Lewy dolny", "Bottom left"),
    ("Celownik", "Crosshair"),
    ("Kształt", "Shape"),
    ("Czerwony", "Red"),
    ("Żółty", "Yellow"),
    ("Zielony", "Green"),
    ("Cyjan", "Cyan"),
    ("Niebieski", "Blue"),
    ("Fioletowy", "Purple"),
    ("Biały", "White"),
    ("Kolorowy (tylko Breathe i Plain Water)", "Colorful (Breathe and Plain Water only)"),
    ("Stoper", "Stopwatch"),
    ("Czas (min)", "Time (min)"),
    ("Czas gry", "Game time"),
    ("Lupa", "Magnifier"),
    ("Powiększenie", "Zoom"),
    ("Rozmiar", "Size"),
    ("Mały", "Small"),
    ("Średni", "Medium"),
    ("Duży", "Large"),
    ("Środek", "Center"),
    ("Poziom", "Level"),
    // ── overview ──
    ("Tryb wyświetlania", "Display mode"),
    ("Rozdzielczość", "Resolution"),
    ("Odświeżanie", "Refresh rate"),
    ("Skalowanie", "Scaling"),
    ("Obraz monitora", "Monitor picture"),
    ("Proporcje obrazu", "Aspect ratio"),
    (
        "Skalowanie sygnału wejściowego przez monitor. Proporcja 21:9 wymaga szerokiego trybu ekranu (np. 3K Wide), którego monitor nie przyjmuje przez DDC",
        "How the monitor scales the input signal. 21:9 needs a wide screen mode (e.g. 3K Wide), which the monitor does not accept over DDC",
    ),
    ("Panoramiczny", "Wide screen"),
    // ── monitor page ──
    ("Monitor nie zgłasza pakietu, wybierz ręcznie. Z pakietem 2025-09-20 DyDs działa razem z Adaptive-Sync", "The monitor does not report the package, choose it yourself. With the 2025-09-20 package DyDs works together with Adaptive-Sync"),
    ("O programie", "About"),
    ("Pokaż plik logu", "Show the log file"),
    ("Strona programu", "Website"),
    ("Zgłoś błąd", "Report a bug"),
    ("Sygnał", "Signal"),
    ("Zakres RGB (Output Range)", "RGB range (Output Range)"),
    ("Ograniczony", "Limited"),
    ("Pełny", "Full"),
    ("Hub USB", "USB hub"),
    ("Który port upstream obsługuje porty USB monitora", "Which upstream port serves the monitor's USB ports"),
    ("Obraz w obrazie", "Picture in picture"),
    (
        "Przy włączonym PIP/PBP monitor wyłącza m.in. DCR, HDR, Adaptive-Sync, DyDs i nakładki gamingowe",
        "With PIP/PBP on, the monitor disables DCR, HDR, Adaptive-Sync, DyDs, gaming overlays and more",
    ),
    ("Tryb", "Mode"),
    ("Wyłączony", "Off"),
    ("PIP (obraz w obrazie)", "PIP (picture in picture)"),
    ("Źródło podrzędne", "Secondary source"),
    ("Tylko w trybie PIP", "PIP mode only"),
    ("Zamień obrazy", "Swap pictures"),
    ("Resetuj ustawienia PIP", "Reset PIP settings"),
    ("Dźwięk", "Audio"),
    ("Źródło dźwięku", "Audio source"),
    ("Dźwięk przy włączonym PIP/PBP; dostępne tylko podłączone wejścia", "Sound while PIP/PBP is on; only connected inputs are available"),
    ("Podświetlenie LED", "LED backlight"),
    ("Efekt", "Effect"),
    ("Star – tylko kolor · Colorful Pearls – bez ustawień", "Star – color only · Colorful Pearls – no settings"),
    ("Intensywność", "Strength"),
    ("Mocna", "Highest"),
    ("Delikatna", "Soft"),
    ("Kolory przód / tył", "Front / rear colors"),
    ("Tylko dla efektu Colorful Water", "Colorful Water effect only"),
    ("Przód", "Front"),
    ("Tył", "Rear"),
    ("Dioda zasilania", "Power LED"),
    ("Jasność diody", "LED brightness"),
    ("Menu OSD", "OSD menu"),
    ("Blokuje przyciski menu na monitorze", "Locks the menu buttons on the monitor"),
    ("Blokada przycisków skrótów", "Shortcut button lock"),
    ("Blokuje przyciski 2 i 3 (Custom 1 / Custom 2)", "Locks buttons 2 and 3 (Custom 1 / Custom 2)"),
    ("Język", "Language"),
    ("Chiński (uproszczony)", "Chinese (simplified)"),
    ("Angielski", "English"),
    ("Francuski", "French"),
    ("Niemiecki", "German"),
    ("Włoski", "Italian"),
    ("Japoński", "Japanese"),
    ("Koreański", "Korean"),
    ("Portugalski", "Portuguese"),
    ("Rosyjski", "Russian"),
    ("Hiszpański", "Spanish"),
    ("Turecki", "Turkish"),
    ("Chiński (tradycyjny)", "Chinese (traditional)"),
    ("Portugalski (Brazylia)", "Portuguese (Brazil)"),
    ("Arabski", "Arabic"),
    ("Holenderski", "Dutch"),
    ("Fiński", "Finnish"),
    ("Grecki", "Greek"),
    ("Polski", "Polish"),
    ("Tajski", "Thai"),
    ("Ukraiński", "Ukrainian"),
    ("Wietnamski", "Vietnamese"),
    ("Czas wyświetlania (s)", "Display time (s)"),
    ("Położenie i przezroczystość", "Position and transparency"),
    ("Pozycja pozioma", "Horizontal position"),
    ("Pozycja pionowa", "Vertical position"),
    ("Przezroczystość", "Transparency"),
    ("Przypomnienie o przerwie", "Break reminder"),
    ("Zasilanie", "Power"),
    ("Oszczędzanie energii", "Power saving"),
    ("Zasilanie USB w uśpieniu", "USB power in sleep"),
    ("Porty USB monitora ładują, gdy monitor śpi", "The monitor's USB ports charge while it sleeps"),
    ("Wyłącz monitor", "Turn the monitor off"),
    ("Informacje", "Information"),
    ("Wgrany pakiet firmware", "Installed firmware package"),
    ("Czas pracy", "Usage time"),
    ("Przywracanie", "Restore"),
    ("Każda akcja wymaga potwierdzenia", "Every action asks for confirmation"),
    ("Resetuj kolory", "Reset colors"),
    ("Resetuj jasność i kontrast", "Reset brightness and contrast"),
    ("Przywróć ustawienia fabryczne", "Restore factory settings"),
    ("Wybór monitora", "Monitor"),
    ("Model", "Model"),
    // ── preferences ──
    ("Preferencje", "Preferences"),
    ("Ogólne", "General"),
    ("Wygląd", "Appearance"),
    ("Język aplikacji", "App language"),
    ("Motyw", "Theme"),
    ("Systemowy", "System"),
    ("Jasny", "Light"),
    ("Ciemny", "Dark"),
    ("Ikona w zasobniku", "Tray icon"),
    ("Kolory motywu", "Theme colors"),
    ("Jasna", "Light"),
    ("Ciemna", "Dark"),
    ("Uruchamianie", "Startup"),
    ("Uruchamiaj z systemem", "Start with the system"),
    ("Uruchamiaj zminimalizowany", "Start minimized"),
    ("Zasobnik", "Tray"),
    (
        "Przytrzymaj klawisz, aby pokazać gwiazdki przy ustawieniach i dodać je do menu w zasobniku (do 5)",
        "Hold the key to show stars next to settings and add them to the tray menu (up to 5)",
    ),
    ("Klawisz ulubionych", "Favorites key"),
    // ── texts only the Windows build shows ──
    ("Zastosuj", "Apply"),
    ("Zamknij", "Close"),
    ("Anuluj", "Cancel"),
    ("Resetuj", "Reset"),
    ("Wyłącz", "Turn off"),
    ("Wyłączyć monitor?", "Turn the monitor off?"),
    ("Włączysz go ponownie tylko przyciskiem zasilania na monitorze.", "Only the monitor's power button turns it on again."),
    ("Zresetować kolory?", "Reset colors?"),
    ("Kolory wrócą do wartości fabrycznych (temperatura barwowa: Warm).", "The colors return to their factory values (color temperature: Warm)."),
    ("Zresetować jasność i kontrast?", "Reset brightness and contrast?"),
    ("Jasność i kontrast wrócą do wartości fabrycznych (90 / 50).", "Brightness and contrast return to their factory values (90 / 50)."),
    ("Przywrócić ustawienia fabryczne?", "Restore factory settings?"),
    ("Wszystkie ustawienia monitora wrócą do wartości fabrycznych.", "All monitor settings return to their factory values."),
    ("Szukam monitora...", "Searching for monitor..."),
    ("Wykrywam monitory...", "Detecting monitors..."),
    ("Szukam monitorów...", "Searching for monitors..."),
    ("Wczytuję zapisane ustawienia...", "Loading cached settings..."),
    ("Odczytuję profil obrazu...", "Reading the picture profile..."),
    ("Nie znaleziono monitora", "Monitor not found"),
    ("Pokaż", "Show"),
    ("Ukryj", "Hide"),
    ("Wyjdź", "Quit"),
    ("Suwaki", "Sliders"),
    ("Opcje", "Options"),
    ("Przełączniki", "Toggles"),
    ("Nie udało się zmienić trybu wyświetlania", "Display mode change failed"),
    ("Skalowanie ustawia się w Windows (Ustawienia → System → Ekran); aplikacja tylko pokazuje wartość", "Scaling is set in Windows (Settings → System → Display); the application only shows the value"),
    ("Monitor gotowy", "Monitor ready"),
    ("Ustawienia monitora wczytane", "Monitor settings read"),
    ("Monitor nie przyjął wartości, ustawiono wartość zgłoszoną przez monitor", "The monitor did not accept the value; the value it reports was set"),
    ("Odśwież", "Refresh"),
    ("Zapisz log", "Save log"),
    ("Wersja", "Version"),
    ("Uruchom z Windows", "Start with Windows"),
    ("Sterowanie monitorem przez DDC/CI (dxva2). Menu programu:", "Monitor control over DDC/CI (dxva2). Application menu:"),
];

/// Formats usage time as `N h M min`, like the Linux application.
pub fn usage_minutes(minutes: u16) -> String {
    format!("{} h {} min", minutes / 60, minutes % 60)
}
