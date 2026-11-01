//! The settings window contents: every page, section and row, in display order.
//!
//! The layout follows the Linux user interface (`src/ui/window.blp`): the same pages, the same rows in the same order,
//! the same option labels. Rows name the VCP code they drive; [`crate::monitor`] decides where that code lives.

use crate::monitor;

/// What a row looks like and which value it takes.
pub enum Kind {
    /// Read-only text: the monitor title and subtitle on the overview page, the model and usage time on the
    /// monitor page, and the status line of the window.
    Text(DynText),
    /// Trackbar with a value label. `factor` multiplies the position (OSD transparency: 20 per step).
    Slider { max: i32, factor: u16 },
    /// Check box that writes `on` or `off`.
    Switch { on: u16, off: u16 },
    /// Drop-down list of option values.
    Choice { values: &'static [u16], options: &'static [&'static str] },
    /// Colour buttons, one per value.
    Swatches { values: &'static [u16], options: &'static [&'static str], colors: &'static [u32] },
    /// Button that runs an action.
    Action(Action),
    /// Picture mode list, writes 0x22 with the Default or Custom value of the chosen mode.
    PictureMode,
    /// Default / Custom variant of the active picture mode, writes 0x22.
    Variant,
    /// Collapsible section title (the label of the row); following rows belong to it.
    Section,
    /// Plain (bold) group title (the label of the row).
    Header,
    /// System display mode of the overview page: resolution, refresh rate, scaling.
    SystemResolution,
    SystemRefresh,
    SystemScaling,
    /// Installed firmware package: an application setting, not a monitor code.
    FirmwarePackage,
}

/// Text rows whose content is set at run time.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DynText {
    MonitorTitle,
    MonitorSubtitle,
    Model,
    Usage,
}

/// Buttons that are not a plain value write.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Action {
    PowerOff,
    ResetColors,
    ResetBrightnessContrast,
    ResetFactory,
    PipSwap,
    PipReset,
}

pub struct Row {
    /// Polish source text, translated before it is shown.
    pub label: &'static str,
    pub kind: Kind,
    /// Setting written by this row; also the key of the value shown.
    pub code: Option<u8>,
    /// Explanation shown as a tooltip (the Linux rows keep it behind an info button or as a subtitle).
    pub tooltip: Option<&'static str>,
    /// Tray-shortcut id: the row gets a star to add it to the tray menu.
    pub favorite: Option<&'static str>,
    /// Inside a collapsible section or below a switch: drawn indented.
    pub sub: bool,
}

const fn row(label: &'static str, kind: Kind, code: Option<u8>) -> Row {
    Row { label, kind, code, tooltip: None, favorite: None, sub: false }
}

const fn slider(label: &'static str, code: u8, max: i32) -> Row {
    row(label, Kind::Slider { max, factor: 1 }, Some(code))
}

const fn slider_factor(label: &'static str, code: u8, max: i32, factor: u16) -> Row {
    row(label, Kind::Slider { max, factor }, Some(code))
}

const fn switch(label: &'static str, code: u8, on: u16, off: u16) -> Row {
    row(label, Kind::Switch { on, off }, Some(code))
}

const fn choice(label: &'static str, code: u8, values: &'static [u16], options: &'static [&'static str]) -> Row {
    row(label, Kind::Choice { values, options }, Some(code))
}

const fn header(title: &'static str) -> Row {
    row(title, Kind::Header, None)
}

const fn section(title: &'static str) -> Row {
    row(title, Kind::Section, None)
}

const fn text(label: &'static str, which: DynText) -> Row {
    row(label, Kind::Text(which), None)
}

const fn tooltip(mut row: Row, text: &'static str) -> Row {
    row.tooltip = Some(text);
    row
}

const fn star(mut row: Row, id: &'static str) -> Row {
    row.favorite = Some(id);
    row
}

const fn sub(mut row: Row) -> Row {
    row.sub = true;
    row
}

// ───────────────────────────── option labels ─────────────────────────────



const HDR_OPTIONS: [&str; 4] = ["Off", "Auto", "Game", "Movie"];
const GAMMA_OPTIONS: [&str; 6] = ["1.8", "2.0", "2.2", "2.4", "2.6", "S Curve"];
const NIGHT_VISION_OPTIONS: [&str; 5] = ["Off", "Lvl 1", "Lvl 2", "Auto 1", "Auto 2"];
const DYNAMIC_OD_OPTIONS: [&str; 5] = ["Off", "Lvl 1", "Lvl 2", "Lvl 3", "Top Speed"];
const LOCAL_DIMMING_OPTIONS: [&str; 5] = ["Off", "Low", "Smooth", "Medium", "High"];
const DYDS_OPTIONS: [&str; 7] = ["Off", "Low", "Med", "High", "ULL-1", "ULL-2", "ULL-3"];
const COLOR_TEMP_OPTIONS: [&str; 6] = ["Warm", "Natural", "Cold", "User 1", "User 2", "User 3"];
const SCREEN_SIZE_OPTIONS: [&str; 2] = ["Wide", "25\""];
/// Corners in the order the UI shows them: top left, top right, bottom left, bottom right.
const CORNER_OPTIONS: [&str; 4] = ["\u{2196}", "\u{2197}", "\u{2199}", "\u{2198}"];
const CENTER_OPTIONS: [&str; 5] = ["\u{2196}", "\u{2197}", "\u{2022}", "\u{2199}", "\u{2198}"];
const SHAPE_OPTIONS: [&str; 6] = ["1", "2", "3", "4", "5", "6"];
const TIME_OPTIONS: [&str; 4] = ["15", "30", "45", "60"];
const ZOOM_OPTIONS: [&str; 3] = ["\u{00d7}1.5", "\u{00d7}2", "\u{00d7}4"];
const SMALL_MEDIUM_LARGE: [&str; 3] = ["Mały", "Średni", "Duży"];
const LEVEL_OPTIONS: [&str; 5] = ["1", "2", "3", "4", "5"];
const USB_HUB_OPTIONS: [&str; 2] = ["USB-C", "USB-B"];
const INPUT_OPTIONS: [&str; 5] = ["Auto", "DisplayPort", "USB-C", "HDMI 1", "HDMI 2"];
const OUTPUT_RANGE_OPTIONS: [&str; 3] = ["Auto", "Ograniczony", "Pełny"];
const PIP_MODE_OPTIONS: [&str; 5] = ["Wyłączony", "PIP (obraz w obrazie)", "PBP 1:1", "PBP 2:1", "PBP 1:2"];
const PIP_SOURCE_OPTIONS: [&str; 4] = ["DisplayPort", "USB-C", "HDMI 1", "HDMI 2"];
const AUDIO_SOURCE_OPTIONS: [&str; 4] = ["Auto", "USB-C", "HDMI 1", "HDMI 2"];
const PIP_SIZE_OPTIONS: [&str; 3] = ["Mały", "Średni", "Duży"];
const RATIO_OPTIONS: [&str; 5] = ["Panoramiczny", "4:3", "1:1", "21:9", "Auto"];
const LED_MODE_OPTIONS: [&str; 7] =
    ["Normal", "Breathe", "Flicker", "Plain Water", "Star", "Colorful Pearls", "Colorful Water"];
const LED_STRENGTH_OPTIONS: [&str; 3] = ["Mocna", "Standard", "Delikatna"];
const LED_SIDE_OPTIONS: [&str; 6] = ["Czerwony", "Zielony", "Niebieski", "Żółty", "Fioletowy", "Cyjan"];
const POWER_LED_OPTIONS: [&str; 4] = ["Off", "Lvl 1", "Lvl 2", "Lvl 3"];
const POWER_SAVING_OPTIONS: [&str; 3] = ["Off", "Lvl 1", "Lvl 2"];
const CROSSHAIR_COLOR_OPTIONS: [&str; 8] = ["Czerwony", "Żółty", "Zielony", "Cyjan", "Niebieski", "Fioletowy", "Biały", "Auto"];
const LED_COLOR_OPTIONS: [&str; 7] =
    ["Czerwony", "Zielony", "Niebieski", "Żółty", "Fioletowy", "Cyjan", "Kolorowy (tylko Breathe i Plain Water)"];

const CROSSHAIR_COLORS: [u32; 8] = [
    0xD32F2F, 0xF2C200, 0x388E3C, 0x00ACC1, 0x1E88E5, 0x8E24AA, 0xFFFFFF, 0x808080,
];
const LED_COLORS: [u32; 7] = [0xD32F2F, 0x388E3C, 0x1E88E5, 0xF2C200, 0x8E24AA, 0x00ACC1, 0xFF7043];

/// OSD languages in the order of the language list (MCCS language codes), see the Linux build.
const OSD_LANGUAGE_VALUES: [u16; 22] = [
    0x0D, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0C, 0x01, 0x0E, 0x0F, 0x14, 0x16, 0x17, 0x19,
    0x1E, 0x23, 0x24, 0x25,
];
const OSD_LANGUAGE_OPTIONS: [&str; 22] = [
    "Chiński (uproszczony)",
    "Angielski",
    "Francuski",
    "Niemiecki",
    "Włoski",
    "Japoński",
    "Koreański",
    "Portugalski",
    "Rosyjski",
    "Hiszpański",
    "Turecki",
    "Chiński (tradycyjny)",
    "Portugalski (Brazylia)",
    "Arabski",
    "Holenderski",
    "Fiński",
    "Grecki",
    "Hindi",
    "Polski",
    "Tajski",
    "Ukraiński",
    "Wietnamski",
];

/// Monitor values of the four corners in UI order (monitor: 0 top right, 1 top left, 2 bottom right, 3 bottom left).
const CORNER_VALUES: [u16; 4] = [1, 0, 3, 2];
/// Magnifier positions: 1 top right, 2 top left, 5 center, 3 bottom right, 4 bottom left.
const MAGNIFIER_POS_VALUES: [u16; 5] = [2, 1, 5, 4, 3];
/// HawkEye positions: 0 top right, 1 top left, 2 center, 3 bottom right, 4 bottom left.
const HAWKEYE_POS_VALUES: [u16; 5] = [1, 0, 2, 4, 3];

pub struct Page {
    pub title: &'static str,
    pub rows: Vec<Row>,
}

/// Builds the six pages of the settings window.
pub fn pages() -> Vec<Page> {
    vec![overview(), picture(), gaming(), inputs(), lighting(), monitor_page()]
}

fn overview() -> Page {
    Page {
        title: "Przegląd",
        rows: vec![
            text("", DynText::MonitorTitle),
            text("", DynText::MonitorSubtitle),
            header("Tryb wyświetlania"),
            row("Rozdzielczość", Kind::SystemResolution, None),
            row("Odświeżanie", Kind::SystemRefresh, None),
            row("Skalowanie", Kind::SystemScaling, None),
            header("Obraz monitora"),
            tooltip(
                choice(
                    "Proporcje obrazu",
                    monitor::VCP_RATIO,
                    &monitor::values::RATIO,
                    &RATIO_OPTIONS,
                ),
                "Skalowanie sygnału wejściowego przez monitor. Proporcja 21:9 wymaga szerokiego trybu ekranu (np. 3K Wide), którego monitor nie przyjmuje przez DDC",
            ),
        ],
    }
}

fn picture() -> Page {
    let rows = vec![
        header("Tryb obrazu"),
        star(row("Aktywny tryb", Kind::PictureMode, Some(monitor::VCP_MODE)), "picture_mode"),
        row("Ustawienia trybu", Kind::Variant, Some(monitor::VCP_MODE)),
        header("Podstawowe"),
        star(slider("Jasność", monitor::VCP_BRIGHTNESS, 100), "brightness"),
        star(slider("Kontrast", monitor::VCP_CONTRAST, 100), "contrast"),
        tooltip(star(switch("DCR", monitor::VCP_DCR, 1, 0), "dcr"), "Dynamiczny kontrast"),
        star(slider("Low Blue Light", monitor::VCP_LOW_BLUE, 4), "low_blue_light"),
        star(slider("Ostrość", monitor::VCP_SHARPNESS, 5), "sharpness"),
        star(
            choice("Gamma", monitor::VCP_GAMMA, &monitor::GAMMA_VALUES, &GAMMA_OPTIONS),
            "gamma",
        ),
        header("Efekty"),
        tooltip(
            star(
                choice("HDR", monitor::VCP_HDR, &monitor::HDR_VALUES, &HDR_OPTIONS),
                "hdr",
            ),
            "W KDE Plasma (Wayland) włącza i wyłącza też HDR w systemie. Z firmware 2025-09-20 przy HDR włączonym w monitorze i w systemie zablokowane są: tryb obrazu, jasność, kontrast, DCR, Low Blue Light, Color Enhance, CR Enhance, Shadow Balance, Night Vision, DyDs, wyłączenie lokalnego przyciemniania, zakres RGB i lupa",
        ),
        star(slider("Color Enhance", monitor::VCP_COLOR_ENHANCE, 10), "color_enhance"),
        star(slider("CR Enhance", monitor::VCP_CR_ENHANCE, 5), "cr_enhance"),
        star(slider("Shadow Balance", monitor::VCP_SHADOW_BALANCE, 100), "shadow_balance"),
        star(slider("Super Res", monitor::VCP_SUPER_RES, 5), "super_res"),
        star(
            choice(
                "Night Vision",
                monitor::VCP_NIGHT_VISION,
                &monitor::NIGHT_VISION_VALUES,
                &NIGHT_VISION_OPTIONS,
            ),
            "night_vision",
        ),
        star(
            choice(
                "Dynamic OD",
                monitor::VCP_DYNAMIC_OD,
                &monitor::DYNAMIC_OD_VALUES,
                &DYNAMIC_OD_OPTIONS,
            ),
            "dynamic_od",
        ),
        header("Podświetlenie"),
        star(
            choice(
                "Lokalne przyciemnianie",
                monitor::VCP_LOCAL_DIMMING,
                &monitor::LOCAL_DIMMING_VALUES,
                &LOCAL_DIMMING_OPTIONS,
            ),
            "local_dimming",
        ),
        tooltip(
            star(slider("Halo Control", monitor::VCP_HALO_CONTROL, 100), "halo_control"),
            "Tylko przy włączonym lokalnym przyciemnianiu",
        ),
        star(
            choice("DyDs", monitor::VCP_DYDS, &monitor::DYDS_VALUES, &DYDS_OPTIONS),
            "dyds",
        ),
        header("Kolor"),
        star(
            choice(
                "Temperatura barwowa",
                monitor::VCP_COLOR_TEMP,
                &monitor::COLOR_TEMP_VALUES,
                &COLOR_TEMP_OPTIONS,
            ),
            "color_temp_profile",
        ),
        section("RGB"),
        sub(star(slider("Czerwony (R)", monitor::VCP_RED, 100), "color_temp_red")),
        sub(star(slider("Zielony (G)", monitor::VCP_GREEN, 100), "color_temp_green")),
        sub(star(slider("Niebieski (B)", monitor::VCP_BLUE, 100), "color_temp_blue")),
        section("Odcień (Hue)"),
        sub(star(slider("Czerwony (R)", monitor::VCP_HUE_RED, 100), "hue_red")),
        sub(star(slider("Zielony (G)", monitor::VCP_HUE_GREEN, 100), "hue_green")),
        sub(star(slider("Niebieski (B)", monitor::VCP_HUE_BLUE, 100), "hue_blue")),
        sub(star(slider("Cyjan (C)", monitor::VCP_HUE_CYAN, 100), "hue_cyan")),
        sub(star(slider("Magenta (M)", monitor::VCP_HUE_MAGENTA, 100), "hue_magenta")),
        sub(star(slider("Żółty (Y)", monitor::VCP_HUE_YELLOW, 100), "hue_yellow")),
        section("Nasycenie (Saturation)"),
        sub(star(slider("Czerwony (R)", monitor::VCP_SATURATION_RED, 100), "saturation_red")),
        sub(star(slider("Zielony (G)", monitor::VCP_SATURATION_GREEN, 100), "saturation_green")),
        sub(star(slider("Niebieski (B)", monitor::VCP_SATURATION_BLUE, 100), "saturation_blue")),
        sub(star(slider("Cyjan (C)", monitor::VCP_SATURATION_CYAN, 100), "saturation_cyan")),
        sub(star(slider("Magenta (M)", monitor::VCP_SATURATION_MAGENTA, 100), "saturation_magenta")),
        sub(star(slider("Żółty (Y)", monitor::VCP_SATURATION_YELLOW, 100), "saturation_yellow")),
    ];
    Page { title: "Tryb obrazu", rows }
}

fn gaming() -> Page {
    Page {
        title: "Gaming",
        rows: vec![
            header("Wydajność"),
            star(switch("Adaptive-Sync", monitor::VCP_ADAPTIVE_SYNC, 1, 0), "adaptive_sync"),
            star(
                choice(
                    "Rozmiar ekranu (DualMode)",
                    monitor::VCP_SCREEN_SIZE,
                    &[0, 1],
                    &SCREEN_SIZE_OPTIONS,
                ),
                "screen_size",
            ),
            header("Nakładki"),
            sub(star(switch("Licznik FPS/Hz", monitor::VCP_FPS_COUNTER, 1, 0), "fps_counter")),
            sub(choice("Pozycja", monitor::VCP_FPS_POS, &CORNER_VALUES, &CORNER_OPTIONS)),
            sub(star(switch("Celownik", monitor::VCP_CROSSHAIR, 1, 0), "crosshair")),
            sub(choice("Kształt", monitor::VCP_CROSSHAIR_SHAPE, &[1, 2, 3, 4, 5, 6], &SHAPE_OPTIONS)),
            sub(row(
                "Kolor",
                Kind::Swatches {
                    values: &[0, 1, 2, 3, 4, 5, 6, 7],
                    options: &CROSSHAIR_COLOR_OPTIONS,
                    colors: &CROSSHAIR_COLORS,
                },
                Some(monitor::VCP_CROSSHAIR_COLOR),
            )),
            sub(star(switch("Stoper", monitor::VCP_STOPWATCH, 1, 0), "stopwatch")),
            sub(choice("Czas (min)", monitor::VCP_STOPWATCH_TIME, &[1, 2, 3, 4], &TIME_OPTIONS)),
            sub(choice("Pozycja", monitor::VCP_STOPWATCH_POS, &CORNER_VALUES, &CORNER_OPTIONS)),
            sub(star(switch("Czas gry", monitor::VCP_GAME_TIME, 1, 0), "game_time")),
            sub(choice("Czas (min)", monitor::VCP_GAME_TIME_VAL, &[1, 2, 3, 4], &TIME_OPTIONS)),
            sub(choice("Pozycja", monitor::VCP_GAME_TIME_POS, &CORNER_VALUES, &CORNER_OPTIONS)),
            sub(tooltip(
                star(switch("Lupa", monitor::VCP_MAGNIFIER, 1, 0), "magnifier"),
                "Niedostępna przy włączonym Adaptive-Sync",
            )),
            sub(choice("Powiększenie", monitor::VCP_MAGNIFIER_ZOOM, &[0, 1, 2], &ZOOM_OPTIONS)),
            sub(choice("Rozmiar", monitor::VCP_MAGNIFIER_SIZE, &[1, 2, 3], &SMALL_MEDIUM_LARGE)),
            sub(choice("Pozycja", monitor::VCP_MAGNIFIER_POS, &MAGNIFIER_POS_VALUES, &CENTER_OPTIONS)),
            sub(star(switch("Hawkeye Vision", monitor::VCP_HAWKEYE, 1, 0), "hawkeye")),
            sub(choice("Rozmiar", monitor::VCP_HAWKEYE_SIZE, &[0, 1, 2], &SMALL_MEDIUM_LARGE)),
            sub(choice("Pozycja", monitor::VCP_HAWKEYE_POS, &HAWKEYE_POS_VALUES, &CENTER_OPTIONS)),
            sub(choice("Poziom", monitor::VCP_HAWKEYE_LEVEL, &[0, 1, 2, 3, 4], &LEVEL_OPTIONS)),
            tooltip(
                switch("Przypomnienie o przerwie", monitor::VCP_EYESHIELD_REMINDER, 1, 0),
                "Eyeshield Reminder. Przez DDC można je tylko włączyć (co 30 min) lub wyłączyć; inny odstęp ustawisz w OSD",
            ),
        ],
    }
}

fn inputs() -> Page {
    Page {
        title: "Wejścia i dźwięk",
        rows: vec![
            header("Sygnał"),
            choice(
                "Źródło sygnału",
                monitor::VCP_INPUT_SELECT,
                &monitor::values::INPUT_SELECT,
                &INPUT_OPTIONS,
            ),
            choice(
                "Zakres RGB (Output Range)",
                monitor::VCP_OUTPUT_RANGE,
                &monitor::OUTPUT_RANGE_VALUES,
                &OUTPUT_RANGE_OPTIONS,
            ),
            tooltip(
                choice(
                    "Hub USB",
                    monitor::VCP_USB_HUB_SOURCE,
                    &monitor::values::USB_HUB_SOURCE,
                    &USB_HUB_OPTIONS,
                ),
                "Który port upstream obsługuje porty USB monitora",
            ),
            header("Dźwięk"),
            slider("Głośność", monitor::VCP_VOLUME, 100),
            switch("Wycisz", monitor::VCP_MUTE, 1, 0),
            header("Obraz w obrazie"),
            tooltip(
                choice("Tryb", monitor::VCP_PIP_MODE, &monitor::values::PIP_MODE, &PIP_MODE_OPTIONS),
                "Przy włączonym PIP/PBP monitor wyłącza m.in. DCR, HDR, Adaptive-Sync, DyDs i nakładki gamingowe",
            ),
            choice(
                "Źródło podrzędne",
                monitor::VCP_PIP_SOURCE,
                &monitor::values::PIP_SOURCE,
                &PIP_SOURCE_OPTIONS,
            ),
            tooltip(
                choice(
                    "Źródło dźwięku",
                    monitor::VCP_AUDIO_SOURCE,
                    &monitor::values::AUDIO_SOURCE,
                    &AUDIO_SOURCE_OPTIONS,
                ),
                "Dźwięk przy włączonym PIP/PBP; dostępne tylko podłączone wejścia",
            ),
            tooltip(
                choice("Pozycja", monitor::VCP_PIP_POSITION, &CORNER_VALUES, &CORNER_OPTIONS),
                "Tylko w trybie PIP",
            ),
            tooltip(
                choice("Rozmiar", monitor::VCP_PIP_SIZE, &monitor::values::PIP_SIZE, &PIP_SIZE_OPTIONS),
                "Tylko w trybie PIP",
            ),
            row("Zamień obrazy", Kind::Action(Action::PipSwap), Some(monitor::VCP_PIP_SWAP)),
            row("Resetuj ustawienia PIP", Kind::Action(Action::PipReset), Some(monitor::VCP_PIP_RESET)),
        ],
    }
}

fn lighting() -> Page {
    Page {
        title: "Oświetlenie",
        rows: vec![
            header("Podświetlenie LED"),
            switch("Oświetlenie LED", monitor::VCP_REAR_LED, 0, 1),
            tooltip(
                choice("Efekt", monitor::VCP_LED_MODE, &monitor::values::LED_MODE, &LED_MODE_OPTIONS),
                "Star – tylko kolor · Colorful Pearls – bez ustawień",
            ),
            row(
                "Kolor",
                Kind::Swatches {
                    values: &monitor::values::LED_COLOR,
                    options: &LED_COLOR_OPTIONS,
                    colors: &LED_COLORS,
                },
                Some(monitor::VCP_LED_COLOR),
            ),
            choice(
                "Intensywność",
                monitor::VCP_LED_STRENGTH,
                &monitor::values::LED_STRENGTH,
                &LED_STRENGTH_OPTIONS,
            ),
            section("Kolory przód / tył"),
            sub(
                tooltip(
                    choice(
                        "Przód",
                        monitor::VCP_LED_FRONT_COLOR,
                        &monitor::values::LED_SIDE_COLOR,
                        &LED_SIDE_OPTIONS,
                    ),
                    "Tylko dla efektu Colorful Water",
                ),
            ),
            sub(choice(
                "Tył",
                monitor::VCP_LED_REAR_COLOR,
                &monitor::values::LED_SIDE_COLOR,
                &LED_SIDE_OPTIONS,
            )),
            header("Dioda zasilania"),
            choice(
                "Jasność diody",
                monitor::VCP_POWER_LED,
                &monitor::values::POWER_LED,
                &POWER_LED_OPTIONS,
            ),
        ],
    }
}

fn monitor_page() -> Page {
    Page {
        title: "Monitor",
        rows: vec![
            header("Menu OSD"),
            tooltip(switch("Blokada OSD", monitor::VCP_OSD_LOCK, 1, 0), "Blokuje przyciski menu na monitorze"),
            tooltip(
                switch("Blokada przycisków skrótów", monitor::VCP_BUTTON_LOCK, 1, 2),
                "Blokuje przyciski 2 i 3 (Custom 1 / Custom 2)",
            ),
            choice(
                "Język",
                monitor::VCP_OSD_LANG,
                &OSD_LANGUAGE_VALUES,
                &OSD_LANGUAGE_OPTIONS,
            ),
            slider("Czas wyświetlania (s)", monitor::VCP_OSD_TIME, 60),
            section("Położenie i przezroczystość"),
            sub(slider("Pozycja pozioma", monitor::VCP_OSD_H_POS, 100)),
            sub(slider("Pozycja pionowa", monitor::VCP_OSD_V_POS, 100)),
            sub(slider_factor("Przezroczystość", monitor::VCP_OSD_TRANS, 100, 20)),
            header("Zasilanie"),
            choice(
                "Oszczędzanie energii",
                monitor::VCP_POWER_SAVING,
                &monitor::POWER_SAVING_VALUES,
                &POWER_SAVING_OPTIONS,
            ),
            tooltip(
                switch("Zasilanie USB w uśpieniu", monitor::VCP_USB_POWER_SLEEP, 1, 0),
                "Porty USB monitora ładują, gdy monitor śpi",
            ),
            switch("Quick Boot", monitor::VCP_QUICK_BOOT, 1, 0),
            row("Wyłącz monitor", Kind::Action(Action::PowerOff), None),
            header("Informacje"),
            text("Model", DynText::Model),
            tooltip(
                row("Wgrany pakiet firmware", Kind::FirmwarePackage, None),
                "Monitor nie zgłasza pakietu, wybierz ręcznie. Z pakietem 2025-09-20 DyDs działa razem z Adaptive-Sync",
            ),
            text("Czas pracy", DynText::Usage),
            header("Przywracanie"),
            row("Resetuj kolory", Kind::Action(Action::ResetColors), None),
            row("Resetuj jasność i kontrast", Kind::Action(Action::ResetBrightnessContrast), None),
            row("Przywróć ustawienia fabryczne", Kind::Action(Action::ResetFactory), None),
        ],
    }
}




