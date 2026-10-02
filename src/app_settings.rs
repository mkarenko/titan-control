use crate::monitor;
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// Application ID (D-Bus name and window class): reverse-DNS, as GTK requires at least one dot.
pub const APP_ID: &str = "io.github.mkarenko.titan_control";
pub const APP_DISPLAY_NAME: &str = "Titan Control";
pub const APP_ICON_NAME: &str = "titan_control";
pub const APP_VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (Alpha)");

const AUTOSTART_FILE_NAME: &str = "titan_control.desktop";
/// Older versions named the entry after the application ID.
const LEGACY_DESKTOP_FILE_NAME: &str = "pl.mkarenko.titan_control.desktop";

/// App preferences. Monitor settings are not stored here: the monitor is read on every start
/// (Custom profile values come from its profile table).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub auto_start: bool,
    pub start_minimized: bool,
    pub favorite_modifier: StepModifier,
    pub favorite_picture_modes: Vec<u16>,
    pub favorite_tray_controls: Vec<String>,
    pub tray_icon_style: TrayIconStyle,
    pub firmware_package: FirmwarePackage,
    /// Index of the app language (see `i18n::lang_from_index`); `None` = system language.
    pub language: Option<u32>,
    /// 0 = system, 1 = light, 2 = dark.
    pub theme: u32,
}

/// Look of the tray icon.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum TrayIconStyle {
    /// Colored by the desktop theme (`ColorScheme-Text`).
    #[default]
    Theme,
    /// Monochrome light (white) icon, for dark panels.
    #[serde(alias = "DarkPanel")]
    Light,
    /// Monochrome dark (black) icon, for light panels.
    #[serde(alias = "LightPanel")]
    Dark,
}

/// Firmware package installed on the monitor. Both report 5.1.1 over DDC, so the user picks it.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum FirmwarePackage {
    /// "P275MV PLUS Firmware Update (M001 20241211 5.1.1)" - the original, public package.
    #[default]
    M001_20241211,
    /// "P275MV PLUS (增强版) 固件补丁升级包 (M001 20250920 5.1.1)" - newer, for the Chinese Enhanced version;
    /// removed from the manufacturer's site. DyDs works together with Adaptive-Sync (VRR).
    M001_20250920,
}

impl FirmwarePackage {
    pub const ALL: [FirmwarePackage; 2] = [Self::M001_20241211, Self::M001_20250920];

    /// DyDs modes keep Adaptive-Sync on (the older firmware turns DyDs off with Adaptive-Sync).
    pub fn dyds_with_adaptive_sync(self) -> bool {
        self == Self::M001_20250920
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum StepModifier {
    #[default]
    Ctrl,
    Shift,
    Alt,
    Super,
}

impl StepModifier {
    pub const ALL: [StepModifier; 4] = [Self::Ctrl, Self::Shift, Self::Alt, Self::Super];
}

impl AppSettings {
    pub fn favorite_picture_mode_set(&self) -> Vec<u16> {
        let mut favorites = self.favorite_picture_modes.clone();
        favorites.sort_unstable();
        favorites.dedup();
        favorites
            .into_iter()
            .filter(|mode| (*mode as usize) < monitor::PICTURE_MODE_NAMES.len())
            .take(5)
            .collect()
    }

    pub fn is_favorite_picture_mode(&self, mode: u16) -> bool {
        self.favorite_picture_modes.contains(&mode)
    }

    pub fn favorite_tray_control_set(&self) -> Vec<String> {
        let mut favorites = self.favorite_tray_controls.clone();
        favorites.sort_unstable();
        favorites.dedup();
        favorites.into_iter().take(5).collect()
    }

    pub fn is_favorite_tray_control(&self, id: &str) -> bool {
        self.favorite_tray_controls.iter().any(|favorite| favorite == id)
    }
}

pub fn load() -> AppSettings {
    let path = settings_file();
    fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str::<AppSettings>(&data).ok())
        .unwrap_or_default()
}

pub fn save(settings: &AppSettings) -> io::Result<()> {
    let path = settings_file();
    ensure_parent_dir(&path)?;
    let data = serde_json::to_string_pretty(settings)?;
    fs::write(path, data)
}

pub fn sync_autostart(settings: &AppSettings) -> io::Result<()> {
    let autostart_path = autostart_file();
    if settings.auto_start {
        ensure_parent_dir(&autostart_path)?;
        fs::write(&autostart_path, desktop_entry())?;
    } else if autostart_path.exists() {
        fs::remove_file(autostart_path)?;
    }
    Ok(())
}

pub fn ensure_desktop_entry() -> io::Result<()> {
    // A package (installed under /usr) brings its own menu entry; only a build run from elsewhere writes one.
    if std::env::current_exe().is_ok_and(|exe| exe.starts_with("/usr")) {
        return Ok(());
    }
    // Remove the entries an older version created under the application ID, so the menu shows only one.
    let _ = fs::remove_file(data_dir().join("applications").join(LEGACY_DESKTOP_FILE_NAME));
    let _ = fs::remove_file(data_dir().join("icons/hicolor/scalable/apps/pl.mkarenko.titan_control.svg"));
    let legacy_autostart = config_dir().join("autostart").join(LEGACY_DESKTOP_FILE_NAME);
    if legacy_autostart.exists() {
        let _ = fs::remove_file(legacy_autostart);
        let _ = sync_autostart(&load());
    }
    let desktop_path = desktop_file();
    ensure_parent_dir(&desktop_path)?;
    fs::write(desktop_path, desktop_entry())
}

fn settings_file() -> PathBuf {
    config_dir().join("titan_control").join("app_settings.json")
}

fn autostart_file() -> PathBuf {
    config_dir().join("autostart").join(AUTOSTART_FILE_NAME)
}

fn desktop_file() -> PathBuf {
    data_dir().join("applications").join(AUTOSTART_FILE_NAME)
}

fn config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(dir)
    } else if let Some(home) = dirs::home_dir() {
        home.join(".config")
    } else {
        PathBuf::from(".")
    }
}

fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(dir)
    } else if let Some(home) = dirs::home_dir() {
        home.join(".local").join("share")
    } else {
        PathBuf::from(".")
    }
}

fn ensure_parent_dir(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn desktop_entry() -> String {
    // An AppImage runs from a temporary mount: its menu entry has to start the AppImage file itself.
    let exec = std::env::var("APPIMAGE")
        .ok()
        .or_else(|| std::env::current_exe().ok().map(|p| p.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "titan_control".into());
    let icon = resolve_desktop_icon();

    format!(
        "[Desktop Entry]\nType=Application\nVersion=1.0\nName={}\nComment={}\nExec={}\nIcon={}\nTerminal=false\nCategories=Settings;Utility;\nStartupNotify=true\nStartupWMClass={}\nX-GNOME-Autostart-enabled=true\n",
        APP_DISPLAY_NAME, APP_DISPLAY_NAME, exec, icon, APP_ID,
    )
}

pub fn resolve_assets_dir() -> Option<PathBuf> {
    asset_dir_candidates()
        .into_iter()
        .find(|candidate| candidate.join("titan_control.svg").exists())
}

pub fn icon_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(assets_dir) = resolve_assets_dir() {
        paths.push(assets_dir.clone());

        let icons_dir = assets_dir.join("icons");
        if icons_dir.exists() {
            paths.push(icons_dir);
        }
    }

    paths
}

pub fn resolve_logo_path() -> Option<PathBuf> {
    resolve_assets_dir().map(|assets_dir| assets_dir.join("titan_control.svg"))
}

fn resolve_desktop_icon() -> String {
    if has_themed_icon(APP_ICON_NAME) {
        APP_ICON_NAME.into()
    } else {
        resolve_logo_path()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| APP_ICON_NAME.into())
    }
}

fn asset_dir_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(current_dir.join("assets"));
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            candidates.push(exe_dir.join("assets"));
            candidates.push(exe_dir.join("..").join("assets"));
            candidates.push(
                exe_dir
                    .join("..")
                    .join("share")
                    .join("titan_control")
                    .join("assets"),
            );
            candidates.push(
                exe_dir
                    .join("..")
                    .join("..")
                    .join("share")
                    .join("titan_control")
                    .join("assets"),
            );
        }

        for ancestor in current_exe.ancestors() {
            candidates.push(ancestor.join("assets"));
        }
    }

    candidates.push(data_dir().join("titan_control").join("assets"));
    candidates.push(PathBuf::from("/usr/local/share/titan_control/assets"));
    candidates.push(PathBuf::from("/usr/share/titan_control/assets"));

    unique_existing_dirs(candidates)
}

fn themed_icon_candidates(icon_name: &str) -> Vec<PathBuf> {
    let file_names = [format!("{icon_name}.svg"), format!("{icon_name}.png")];
    let relative_dirs = [
        PathBuf::from("icons/hicolor/scalable/apps"),
        PathBuf::from("icons/hicolor/256x256/apps"),
        PathBuf::from("icons/hicolor/128x128/apps"),
        PathBuf::from("icons/hicolor/64x64/apps"),
        PathBuf::from("icons/hicolor/48x48/apps"),
        PathBuf::from("icons/hicolor/32x32/apps"),
        PathBuf::from("pixmaps"),
    ];
    let roots = [
        data_dir(),
        PathBuf::from("/usr/local/share"),
        PathBuf::from("/usr/share"),
    ];

    let mut candidates = Vec::new();
    for root in roots {
        for relative_dir in &relative_dirs {
            for file_name in &file_names {
                candidates.push(root.join(relative_dir).join(file_name));
            }
        }
    }

    candidates
}

fn has_themed_icon(icon_name: &str) -> bool {
    themed_icon_candidates(icon_name)
        .into_iter()
        .any(|candidate| candidate.exists())
}

fn unique_existing_dirs(candidates: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut unique = Vec::new();

    for candidate in candidates {
        if candidate.exists() && !unique.iter().any(|existing| existing == &candidate) {
            unique.push(candidate);
        }
    }

    unique
}
