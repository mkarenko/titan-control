use crate::monitor;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};

pub const APP_ID: &str = "pl.mkarenko.titan_control";
pub const APP_DISPLAY_NAME: &str = "Titan Control";
pub const APP_ICON_NAME: &str = APP_ID;

const AUTOSTART_FILE_NAME: &str = "pl.mkarenko.titan_control.desktop";

pub fn basic_profile_values() -> HashMap<u8, u16> {
    HashMap::from([
        (monitor::VCP_BRIGHTNESS, 100),
        (monitor::VCP_CONTRAST, 50),
        (monitor::VCP_DCR, 0),
        (monitor::VCP_SHARPNESS, 0),
        (monitor::VCP_SHADOW_BALANCE, 50),
        (monitor::VCP_CR_ENHANCE, 0),
        (monitor::VCP_COLOR_ENHANCE, 0),
        (monitor::VCP_SUPER_RES, 0),
        (monitor::VCP_LOW_BLUE, 0),
        (monitor::VCP_COLOR_TEMP, monitor::COLOR_TEMP_VALUES[0]),
        (monitor::VCP_RED, 48),
        (monitor::VCP_GREEN, 50),
        (monitor::VCP_BLUE, 47),
        (monitor::VCP_HDR, monitor::HDR_VALUES[0]),
        (monitor::VCP_GAMMA, monitor::GAMMA_VALUES[2]),
        (monitor::VCP_NIGHT_VISION, monitor::NIGHT_VISION_VALUES[0]),
        (monitor::VCP_DYNAMIC_OD, monitor::DYNAMIC_OD_VALUES[0]),
        (monitor::VCP_DYDS, monitor::DYDS_VALUES[0]),
        (monitor::VCP_HUE_RED, 50),
        (monitor::VCP_HUE_GREEN, 50),
        (monitor::VCP_HUE_BLUE, 50),
        (monitor::VCP_HUE_CYAN, 50),
        (monitor::VCP_HUE_MAGENTA, 50),
        (monitor::VCP_HUE_YELLOW, 50),
        (monitor::VCP_SATURATION_RED, 50),
        (monitor::VCP_SATURATION_GREEN, 50),
        (monitor::VCP_SATURATION_BLUE, 50),
        (monitor::VCP_SATURATION_CYAN, 50),
        (monitor::VCP_SATURATION_MAGENTA, 50),
        (monitor::VCP_SATURATION_YELLOW, 50),
    ])
}

pub fn basic_profile_values_for_mode(_mode_index: u16) -> HashMap<u8, u16> {
    basic_profile_values()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomProfileSettings {
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub values: HashMap<u8, u16>,
}

impl Default for CustomProfileSettings {
    fn default() -> Self {
        Self {
            active: false,
            values: basic_profile_values(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AppSettings {
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default)]
    pub start_minimized: bool,
    #[serde(default)]
    pub last_picture_mode: Option<u16>,
    #[serde(default)]
    pub custom_profiles: HashMap<u16, CustomProfileSettings>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct UiState {
    #[serde(default)]
    collapsible_sections: HashMap<String, bool>,
}

#[derive(Debug, Deserialize, Default)]
struct StoredAppSettings {
    #[serde(default)]
    auto_start: bool,
    #[serde(default)]
    start_minimized: bool,
    #[serde(default)]
    last_picture_mode: Option<u16>,
    #[serde(default)]
    custom_profiles: HashMap<u16, CustomProfileSettings>,
    #[serde(default)]
    profile_custom_active: bool,
    #[serde(default)]
    custom_profile_values: HashMap<u8, u16>,
}

impl From<StoredAppSettings> for AppSettings {
    fn from(value: StoredAppSettings) -> Self {
        let mut custom_profiles = value.custom_profiles;

        for profile in custom_profiles.values_mut() {
            if profile.values.is_empty() {
                profile.values = CustomProfileSettings::default().values;
            }
        }

        if value.profile_custom_active || !value.custom_profile_values.is_empty() {
            custom_profiles
                .entry(0)
                .or_insert_with(|| CustomProfileSettings {
                    active: value.profile_custom_active,
                    values: value.custom_profile_values,
                });
        }

        Self {
            auto_start: value.auto_start,
            start_minimized: value.start_minimized,
            last_picture_mode: value.last_picture_mode,
            custom_profiles,
        }
    }
}

impl AppSettings {
    pub fn custom_profile(&self, mode: u16) -> CustomProfileSettings {
        self.custom_profiles
            .get(&mode)
            .cloned()
            .map(|mut profile| {
                if profile.values.is_empty() {
                    profile.values = CustomProfileSettings::default().values;
                }
                profile
            })
            .unwrap_or_default()
    }
}

pub fn load() -> AppSettings {
    let path = settings_file();
    fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str::<StoredAppSettings>(&data).ok())
        .map(AppSettings::from)
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

pub fn collapsible_section_expanded(key: &str) -> bool {
    load_ui_state()
        .collapsible_sections
        .get(key)
        .copied()
        .unwrap_or(true)
}

pub fn set_collapsible_section_expanded(key: &str, expanded: bool) -> io::Result<()> {
    let mut state = load_ui_state();
    state.collapsible_sections.insert(key.to_string(), expanded);
    save_ui_state(&state)
}

pub fn ensure_desktop_entry() -> io::Result<()> {
    let desktop_path = desktop_file();
    ensure_parent_dir(&desktop_path)?;
    fs::write(desktop_path, desktop_entry())
}

fn settings_file() -> PathBuf {
    config_dir().join("titan_control").join("app_settings.json")
}

fn ui_state_file() -> PathBuf {
    config_dir().join("titan_control").join("ui_state.json")
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

fn load_ui_state() -> UiState {
    let path = ui_state_file();
    fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str::<UiState>(&data).ok())
        .unwrap_or_default()
}

fn save_ui_state(state: &UiState) -> io::Result<()> {
    let path = ui_state_file();
    ensure_parent_dir(&path)?;
    let data = serde_json::to_string_pretty(state)?;
    fs::write(path, data)
}

fn desktop_entry() -> String {
    let exec = std::env::current_exe()
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
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
        .find(|candidate| candidate.join("pl.mkarenko.titan_control.svg").exists())
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
    resolve_assets_dir().map(|assets_dir| assets_dir.join("pl.mkarenko.titan_control.svg"))
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
