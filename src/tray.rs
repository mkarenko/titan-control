use crate::app_settings::TrayIconStyle;
use crate::i18n::{AppLang, tr};
use adw::prelude::*;
use gtk4::glib;
use ksni::blocking::TrayMethods;
use libadwaita as adw;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

enum TrayAction {
    Toggle,
    SelectFavorite(u16),
    SelectShortcut(String),
    SelectOption(String, usize),
    SetIconStyle(TrayIconStyle),
    Refresh,
    Quit,
}

/// Changes the tray while the app runs.
#[derive(Clone)]
pub struct TrayControl(async_channel::Sender<TrayAction>);

impl TrayControl {
    pub fn set_style(&self, style: TrayIconStyle) {
        let _ = self.0.send_blocking(TrayAction::SetIconStyle(style));
    }

    /// Tells the panel that the menu changed (show/hide label, favorites, active mode); the panel keeps the old
    /// menu until it is told so.
    pub fn refresh(&self) {
        let _ = self.0.try_send(TrayAction::Refresh);
    }
}

#[derive(Clone, Debug, Default)]
pub struct TrayFavoriteProfile {
    pub mode: u16,
    pub label: String,
}

pub type TrayFavoritesState = Arc<Mutex<Vec<TrayFavoriteProfile>>>;
pub type TrayActiveModeState = Arc<Mutex<Option<u16>>>;

#[derive(Clone, Debug, Default)]
pub struct TrayShortcut {
    pub id: String,
    pub label: String,
}

pub type TrayShortcutsState = Arc<Mutex<Vec<TrayShortcut>>>;
pub type TrayWindowVisibleState = Arc<Mutex<bool>>;

#[derive(Clone, Debug)]
pub struct TrayChoiceState {
    pub hdr_selected: usize,
    pub hdr_enabled: bool,
    pub gamma_selected: usize,
    pub gamma_enabled: bool,
}

impl Default for TrayChoiceState {
    fn default() -> Self {
        Self {
            hdr_selected: 0,
            hdr_enabled: true,
            gamma_selected: 2,
            gamma_enabled: true,
        }
    }
}

pub type TrayChoiceStateHandle = Arc<Mutex<TrayChoiceState>>;
type SelectFavoriteCallback = Rc<dyn Fn(u16)>;
type SelectShortcutCallback = Rc<dyn Fn(&str)>;
type SelectOptionCallback = Rc<dyn Fn(&str, usize)>;

struct TitanTray {
    tx: async_channel::Sender<TrayAction>,
    lang: AppLang,
    favorites: TrayFavoritesState,
    active_mode: TrayActiveModeState,
    shortcuts: TrayShortcutsState,
    window_visible: TrayWindowVisibleState,
    choices: TrayChoiceStateHandle,
    icon_style: TrayIconStyle,
}

/// Monochrome copies of the app icon: the theme-colored path (`ColorScheme-Text`) gets a fixed fill.
/// Returns the directory with the icon and its name, or `None` (then the theme-colored icon is used).
fn monochrome_icon(style: TrayIconStyle) -> Option<(String, String)> {
    let (suffix, color) = match style {
        TrayIconStyle::Theme => return None,
        TrayIconStyle::Light => ("tray-white", "#ffffff"),
        TrayIconStyle::Dark => ("tray-black", "#000000"),
    };
    let source = crate::app_settings::resolve_assets_dir()?
        .join(format!("{}.svg", crate::app_settings::APP_ICON_NAME));
    let svg = std::fs::read_to_string(source).ok()?;
    let svg = svg.replace(
        r#"fill="currentColor" class="ColorScheme-Text""#,
        &format!(r#"fill="{color}""#),
    );
    let dir = std::env::temp_dir().join("titan_control_tray_icons");
    std::fs::create_dir_all(&dir).ok()?;
    let name = format!("{}-{suffix}", crate::app_settings::APP_ICON_NAME);
    std::fs::write(dir.join(format!("{name}.svg")), svg).ok()?;
    Some((dir.to_string_lossy().into_owned(), name))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ShortcutKind {
    Slider,
    Choice,
    Toggle,
}

fn shortcut_kind(id: &str) -> ShortcutKind {
    match id {
        "brightness"
        | "contrast"
        | "sharpness"
        | "color_enhance"
        | "cr_enhance"
        | "shadow_balance"
        | "super_res"
        | "halo_control"
        | "low_blue_light"
        | "color_temp_red"
        | "color_temp_green"
        | "color_temp_blue"
        | "hue_red"
        | "hue_green"
        | "hue_blue"
        | "hue_cyan"
        | "hue_magenta"
        | "hue_yellow"
        | "saturation_red"
        | "saturation_green"
        | "saturation_blue"
        | "saturation_cyan"
        | "saturation_magenta"
        | "saturation_yellow"
        | "volume" => ShortcutKind::Slider,
        "color_temp_profile"
        | "source"
        | "input_source"
        | "output_range"
        | "power_saving"
        | "power_led"
        | "hdr"
        | "gamma"
        | "local_dimming"
        | "dynamic_od"
        | "dyds"
        | "screen_size"
        | "night_vision" => ShortcutKind::Choice,
        _ => ShortcutKind::Toggle,
    }
}

/// Largest value and step of the values offered for a slider in the tray menu.
fn slider_range(id: &str) -> (u16, u16) {
    match id {
        "sharpness" | "cr_enhance" | "super_res" => (5, 1),
        "low_blue_light" => (4, 1),
        "color_enhance" => (10, 1),
        _ => (100, 10),
    }
}

fn choices_for(id: &str) -> Option<&'static [&'static str]> {
    match id {
        "color_temp_profile" => Some(&["Warm", "Natural", "Cold", "User 1", "User 2", "User 3"]),
        "screen_size" => Some(&["Wide", "25\""]),
        "night_vision" => Some(&["Off", "Lvl 1", "Lvl 2", "Auto 1", "Auto 2"]),
        "hdr" => Some(&["Off", "Auto", "Game", "Movie"]),
        "gamma" => Some(&["1.8", "2.0", "2.2", "2.4", "2.6", "S Curve"]),
        "local_dimming" => Some(&["Off", "Low", "Smooth", "Medium", "High"]),
        "dynamic_od" => Some(&["Off", "Lvl 1", "Lvl 2", "Lvl 3", "Top Speed"]),
        "dyds" => Some(&["Off", "Low", "Med", "High", "ULL-1", "ULL-2", "ULL-3"]),
        "output_range" => Some(&["Auto", "Limited", "Full"]),
        "power_saving" => Some(&["Off", "Lvl 1", "Lvl 2"]),
        "power_led" => Some(&["Off", "Lvl 1", "Lvl 2", "Lvl 3"]),
        "input_source" | "source" => Some(&["DisplayPort", "USB-C", "HDMI 1", "HDMI 2"]),
        _ => None,
    }
}

fn selected_choice(id: &str, choices: &TrayChoiceState) -> Option<usize> {
    match id {
        "hdr" => Some(choices.hdr_selected),
        "gamma" => Some(choices.gamma_selected),
        _ => None,
    }
}

fn choice_enabled(id: &str, choices: &TrayChoiceState) -> bool {
    match id {
        "hdr" => choices.hdr_enabled,
        "gamma" => choices.gamma_enabled,
        _ => true,
    }
}

impl ksni::Tray for TitanTray {
    fn id(&self) -> String {
        crate::app_settings::APP_ID.into()
    }

    fn title(&self) -> String {
        crate::app_settings::APP_DISPLAY_NAME.into()
    }

    fn category(&self) -> ksni::Category {
        ksni::Category::Hardware
    }

    fn icon_name(&self) -> String {
        if let Some((_, name)) = monochrome_icon(self.icon_style) {
            return name;
        }
        crate::app_settings::APP_ICON_NAME.into()
    }

    fn icon_theme_path(&self) -> String {
        if let Some((dir, _)) = monochrome_icon(self.icon_style) {
            return dir;
        }
        if let Some(path) = crate::app_settings::resolve_assets_dir() {
            return path.to_string_lossy().into_owned();
        }
        String::new()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: crate::app_settings::APP_DISPLAY_NAME.into(),
            ..Default::default()
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.tx.send_blocking(TrayAction::Toggle);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        let mut items = Vec::new();

        let favorites = self
            .favorites
            .lock()
            .map(|favorites| favorites.clone())
            .unwrap_or_default();
        let active_mode = self.active_mode.lock().map(|mode| *mode).unwrap_or(None);
        let shortcuts = self
            .shortcuts
            .lock()
            .map(|shortcuts| shortcuts.clone())
            .unwrap_or_default();
        let window_visible = self.window_visible.lock().map(|state| *state).unwrap_or(false);
        let choices = self.choices.lock().map(|choices| choices.clone()).unwrap_or_default();

        for favorite in favorites {
            let checked = active_mode == Some(favorite.mode);
            items.push(
                CheckmarkItem {
                    label: favorite.label,
                    checked,
                    activate: Box::new(move |this: &mut Self| {
                        let _ = this.tx.send_blocking(TrayAction::SelectFavorite(favorite.mode));
                    }),
                    ..Default::default()
                }
                .into(),
            );
        }

        let slider_shortcuts: Vec<_> = shortcuts
            .iter()
            .filter(|shortcut| shortcut_kind(&shortcut.id) == ShortcutKind::Slider)
            .cloned()
            .collect();
        if !items.is_empty() && !slider_shortcuts.is_empty() {
            items.push(ksni::MenuItem::Separator);
        }
        if !slider_shortcuts.is_empty() {
            // The tray menu cannot hold a slider: each one is a submenu of ready values.
            let submenu = slider_shortcuts
                .into_iter()
                .map(|shortcut| {
                    let (max, step) = slider_range(&shortcut.id);
                    let values = (0..=max).step_by(step as usize).map(|value| {
                        let id = shortcut.id.clone();
                        StandardItem {
                            label: value.to_string(),
                            activate: Box::new(move |this: &mut Self| {
                                let _ = this.tx.send_blocking(TrayAction::SelectOption(id.clone(), value as usize));
                            }),
                            ..Default::default()
                        }
                        .into()
                    });
                    SubMenu {
                        label: format!("{}\u{a0}\u{a0}\u{a0}\u{a0}", shortcut.label),
                        submenu: values.collect(),
                        ..Default::default()
                    }
                    .into()
                })
                .collect();
            items.push(
                SubMenu {
                    label: tr(&self.lang, "tray_sliders"),
                    submenu,
                    ..Default::default()
                }
                .into(),
            );
        }

        let choice_shortcuts: Vec<_> = shortcuts
            .iter()
            .filter(|shortcut| shortcut_kind(&shortcut.id) == ShortcutKind::Choice)
            .cloned()
            .collect();
        if !items.is_empty() && !choice_shortcuts.is_empty() {
            items.push(ksni::MenuItem::Separator);
        }
        if !choice_shortcuts.is_empty() {
            let submenu = choice_shortcuts
                .into_iter()
                .map(|shortcut| {
                    if let Some(options) = choices_for(&shortcut.id) {
                        let selected = selected_choice(&shortcut.id, &choices);
                        let enabled = choice_enabled(&shortcut.id, &choices);
                        let option_items = options
                            .iter()
                            .enumerate()
                            .map(|(index, option)| {
                                let id = shortcut.id.clone();
                                CheckmarkItem {
                                    label: (*option).into(),
                                    enabled,
                                    checked: selected == Some(index),
                                    activate: Box::new(move |this: &mut Self| {
                                        let _ = this
                                            .tx
                                            .send_blocking(TrayAction::SelectOption(id.clone(), index));
                                    }),
                                    ..Default::default()
                                }
                                .into()
                            })
                            .collect();
                        SubMenu {
                            // Some desktops draw the submenu arrow right behind the longest label: trailing
                            // non-breaking spaces keep a gap.
                            label: format!("{}\u{a0}\u{a0}\u{a0}\u{a0}", shortcut.label),
                            enabled,
                            submenu: option_items,
                            ..Default::default()
                        }
                        .into()
                    } else {
                        StandardItem {
                            label: shortcut.label,
                            activate: Box::new(move |this: &mut Self| {
                                let _ = this
                                    .tx
                                    .send_blocking(TrayAction::SelectShortcut(shortcut.id.clone()));
                            }),
                            ..Default::default()
                        }
                        .into()
                    }
                })
                .collect();
            items.push(
                SubMenu {
                    label: tr(&self.lang, "tray_options"),
                    submenu,
                    ..Default::default()
                }
                .into(),
            );
        }

        let toggle_shortcuts: Vec<_> = shortcuts
            .iter()
            .filter(|shortcut| shortcut_kind(&shortcut.id) == ShortcutKind::Toggle)
            .cloned()
            .collect();
        if !items.is_empty() && !toggle_shortcuts.is_empty() {
            items.push(ksni::MenuItem::Separator);
        }
        if !toggle_shortcuts.is_empty() {
            let submenu = toggle_shortcuts
                .into_iter()
                .map(|shortcut| {
                    StandardItem {
                        label: shortcut.label,
                        activate: Box::new(move |this: &mut Self| {
                            let _ = this
                                .tx
                                .send_blocking(TrayAction::SelectShortcut(shortcut.id.clone()));
                        }),
                        ..Default::default()
                    }
                    .into()
                })
                .collect();
            items.push(
                SubMenu {
                    label: tr(&self.lang, "tray_toggles"),
                    submenu,
                    ..Default::default()
                }
                .into(),
            );
        }

        if !items.is_empty() {
            items.push(ksni::MenuItem::Separator);
        }

        items.extend([
            StandardItem {
                label: tr(
                    &self.lang,
                    if window_visible { "tray_hide" } else { "tray_show" },
                ),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send_blocking(TrayAction::Toggle);
                }),
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            StandardItem {
                label: tr(&self.lang, "tray_quit"),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send_blocking(TrayAction::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]);

        items
    }
}

#[allow(clippy::too_many_arguments)]
pub fn setup_tray(
    app: &adw::Application,
    window: &adw::ApplicationWindow,
    hold_guard: gtk4::gio::ApplicationHoldGuard,
    lang: AppLang,
    favorites: TrayFavoritesState,
    active_mode: TrayActiveModeState,
    shortcuts: TrayShortcutsState,
    window_visible: TrayWindowVisibleState,
    choices: TrayChoiceStateHandle,
    icon_style: TrayIconStyle,
    on_select_favorite: SelectFavoriteCallback,
    on_select_shortcut: SelectShortcutCallback,
    on_select_option: SelectOptionCallback,
) -> TrayControl {
    let (tx, rx) = async_channel::unbounded::<TrayAction>();
    let control = TrayControl(tx.clone());

    let tray = TitanTray {
        tx,
        lang,
        favorites,
        active_mode,
        shortcuts,
        window_visible,
        choices,
        icon_style,
    };
    let handle = tray.spawn().expect("Nie można utworzyć ikony tray");

    let window_clone = window.clone();
    let app_clone = app.clone();

    glib::MainContext::default().spawn_local(async move {
        let _hold = hold_guard;
        let handle = handle;

        while let Ok(action) = rx.recv().await {
            match action {
                TrayAction::Toggle => {
                    if !window_clone.is_visible() {
                        window_clone.set_visible(true);
                        window_clone.present();
                        window_clone.grab_focus();
                    } else {
                        window_clone.set_visible(false);
                    }
                }
                TrayAction::SelectFavorite(mode) => {
                    on_select_favorite(mode);
                }
                TrayAction::SelectShortcut(id) => {
                    on_select_shortcut(&id);
                }
                TrayAction::SelectOption(id, index) => {
                    on_select_option(&id, index);
                }
                TrayAction::SetIconStyle(style) => {
                    handle.update(|tray| tray.icon_style = style);
                }
                TrayAction::Refresh => {
                    handle.update(|_| {});
                }
                TrayAction::Quit => app_clone.quit(),
            }
        }
    });
    control
}