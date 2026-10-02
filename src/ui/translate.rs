//! Runtime translation of the Blueprint UI.
//!
//! The Blueprint file (window.blp) is written in Polish. At startup every translatable text of the widget tree
//! (row titles/subtitles, group titles, toggle labels, tooltips, combo items...) is recorded together with its
//! Polish source; `Translator::apply` then switches between Polish and English. Languages without their own
//! dictionary use English. Texts set later by the code (e.g. the monitor name) are left alone.

use crate::i18n::AppLang;
use crate::ui::bindings;
use adw::prelude::*;
use gtk4::{gio, glib};
use libadwaita as adw;
use std::cell::RefCell;

/// Polish source → English.
const PL_EN: &[(&str, &str)] = &[
    ("Menu główne", "Main menu"),
    ("Odśwież informacje i ustawienia z monitora", "Read the monitor info and settings again"),
    ("Przegląd", "Overview"),
    ("Łączenie z monitorem…", "Connecting to the monitor…"),
    ("Tryb obrazu", "Picture mode"),
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
    ("W KDE Plasma (Wayland) włącza i wyłącza też HDR w systemie. Z firmware 2025-09-20 przy HDR włączonym w monitorze i w systemie zablokowane są: tryb obrazu, jasność, kontrast, DCR, Low Blue Light, Color Enhance, CR Enhance, Shadow Balance, Night Vision, DyDs, wyłączenie lokalnego przyciemniania, zakres RGB i lupa", "On KDE Plasma (Wayland) it also turns HDR on and off in the system. With firmware 2025-09-20, while HDR is on in both the monitor and the system, these are locked: picture mode, brightness, contrast, DCR, Low Blue Light, Color Enhance, CR Enhance, Shadow Balance, Night Vision, DyDs, turning local dimming off, RGB range and the magnifier"),
    ("Niedostępna przy włączonym Adaptive-Sync", "Not available while Adaptive-Sync is on"),
    ("Eyeshield Reminder. Przez DDC można je tylko włączyć (co 30 min) lub wyłączyć; inny odstęp ustawisz w OSD", "Eyeshield Reminder. Over DDC it can only be turned on (every 30 min) or off; set another interval in the OSD"),
    ("Podświetlenie", "Backlight"),
    ("Odcień (Hue)", "Hue"),
    ("Cyjan (C)", "Cyan (C)"),
    ("Magenta (M)", "Magenta (M)"),
    ("Żółty (Y)", "Yellow (Y)"),
    ("Nasycenie (Saturation)", "Saturation"),
    ("Przy włączonym Color Enhance nasycenie ustawia on sam: suwaki pokazują wartości dla jego poziomu i są zablokowane", "While Color Enhance is on it sets the saturation itself: the sliders show the values for its level and are locked"),
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
    ("Tryb wyświetlania", "Display mode"),
    ("Rozdzielczość", "Resolution"),
    ("Odświeżanie", "Refresh rate"),
    ("Skalowanie", "Scaling"),
    ("Obraz monitora", "Monitor picture"),
    ("Proporcje obrazu", "Aspect ratio"),
    ("Skalowanie sygnału wejściowego przez monitor. Proporcja 21:9 wymaga szerokiego trybu ekranu (np. 3K Wide), którego monitor nie przyjmuje przez DDC", "How the monitor scales the input signal. 21:9 needs a wide screen mode (e.g. 3K Wide), which the monitor does not accept over DDC"),
    ("Panoramiczny", "Wide screen"),
    ("Wejścia i dźwięk", "Inputs and audio"),
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
    ("Oświetlenie", "Lighting"),
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
];

/// Translation of a Polish source text, or `None` when it is not in the dictionary.
fn english(source: &str) -> Option<&'static str> {
    PL_EN.iter().find(|(pl, _)| *pl == source).map(|(_, en)| *en)
}

/// Translates a Polish source text for `lang` (texts outside the dictionary are returned unchanged).
pub fn tr_pl(lang: &AppLang, source: &str) -> String {
    match lang {
        AppLang::PL => source.to_string(),
        _ => english(source).map(str::to_string).unwrap_or_else(|| source.to_string()),
    }
}

enum Target {
    /// A string property of a widget or object.
    Property(glib::Object, &'static str),
    /// An item of a combo/drop-down model; `owner` keeps its selection.
    ListItem { list: gtk4::StringList, position: u32, owner: glib::Object },
}

struct Entry {
    target: Target,
    source: String,
    /// What was set last; a different current value means the code changed it, so it is left alone.
    last: RefCell<String>,
}

#[derive(Default)]
pub struct Translator {
    entries: Vec<Entry>,
}

impl Translator {
    /// Records the translatable texts of the given trees (call before the code sets dynamic texts).
    pub fn collect(roots: &[gtk4::Widget], stacks: &[gtk4::Stack]) -> Self {
        let mut translator = Self::default();
        for root in roots {
            translator.visit(root);
        }
        for stack in stacks {
            let pages = stack.pages();
            for index in 0..pages.n_items() {
                if let Some(page) = pages.item(index).and_downcast::<gtk4::StackPage>() {
                    translator.property(page.upcast_ref(), "title");
                }
            }
        }
        translator
    }

    fn property(&mut self, object: &glib::Object, name: &'static str) {
        if object.find_property(name).is_none() {
            return;
        }
        let Some(value) = object.property::<Option<String>>(name) else { return };
        if english(&value).is_some() {
            self.entries.push(Entry {
                target: Target::Property(object.clone(), name),
                last: RefCell::new(value.clone()),
                source: value,
            });
        }
    }

    fn list(&mut self, owner: &glib::Object, model: Option<gio::ListModel>) {
        let Some(list) = model.and_downcast::<gtk4::StringList>() else { return };
        for position in 0..list.n_items() {
            let Some(value) = list.string(position).map(|value| value.to_string()) else { continue };
            if english(&value).is_some() {
                self.entries.push(Entry {
                    target: Target::ListItem { list: list.clone(), position, owner: owner.clone() },
                    last: RefCell::new(value.clone()),
                    source: value,
                });
            }
        }
    }

    fn visit(&mut self, widget: &gtk4::Widget) {
        let object = widget.upcast_ref::<glib::Object>();
        if widget.is::<adw::PreferencesRow>() {
            self.property(object, "title");
        }
        if widget.is::<adw::ActionRow>() || widget.is::<adw::ExpanderRow>() {
            self.property(object, "subtitle");
        }
        if let Some(combo) = widget.downcast_ref::<adw::ComboRow>() {
            self.list(object, combo.model());
        }
        if let Some(drop_down) = widget.downcast_ref::<gtk4::DropDown>() {
            self.list(object, drop_down.model());
        }
        if widget.is::<adw::PreferencesGroup>() {
            self.property(object, "title");
            self.property(object, "description");
        }
        if widget.is::<adw::PreferencesPage>()
            || widget.is::<adw::NavigationPage>()
            || widget.is::<adw::PreferencesDialog>()
            || widget.is::<adw::WindowTitle>()
        {
            self.property(object, "title");
        }
        if widget.is::<gtk4::Button>() && !widget.is::<adw::ButtonRow>() {
            self.property(object, "label");
        }
        self.property(object, "tooltip-text");
        if let Some(group) = widget.downcast_ref::<adw::ToggleGroup>() {
            for index in 0..group.n_toggles() {
                if let Some(toggle) = group.toggle(index) {
                    self.property(toggle.upcast_ref(), "label");
                    self.property(toggle.upcast_ref(), "tooltip");
                }
            }
        }

        let mut child = widget.first_child();
        while let Some(current) = child {
            self.visit(&current);
            child = current.next_sibling();
        }
    }

    /// Shows every recorded text in `lang`.
    pub fn apply(&self, lang: &AppLang) {
        bindings::suppress(|| {
            for entry in &self.entries {
                let text = tr_pl(lang, &entry.source);
                match &entry.target {
                    Target::Property(object, name) => {
                        let current = object.property::<Option<String>>(name).unwrap_or_default();
                        if current != *entry.last.borrow() {
                            continue;
                        }
                        object.set_property(name, &text);
                    }
                    Target::ListItem { list, position, owner } => {
                        if list.string(*position).map(|value| value.to_string()).as_deref()
                            != Some(entry.last.borrow().as_str())
                        {
                            continue;
                        }
                        let selected = owner.property::<u32>("selected");
                        list.splice(*position, 1, &[text.as_str()]);
                        owner.set_property("selected", selected);
                    }
                }
                *entry.last.borrow_mut() = text;
            }
        });
    }
}
