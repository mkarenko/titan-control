//! Application preferences, Windows paths and autostart.
//!
//! The JSON file keeps the field names of the Linux application (`app_settings.json`), so an existing configuration
//! can be copied over. Autostart uses the per-user `Run` key instead of an XDG autostart entry.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::monitor;

pub const APP_DISPLAY_NAME: &str = "Titan Control";
pub const APP_VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (Windows)");
/// Name of the `Run` value and of the single-instance mutex.
pub const APP_REGISTRY_NAME: &str = "Titan Control";

/// App preferences. Monitor settings are not stored here: the monitor is read on every start.
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
    /// Follows the taskbar (white icon on a dark taskbar, black on a light one).
    #[default]
    Theme,
    /// Monochrome light (white) icon, for dark taskbars.
    #[serde(alias = "DarkPanel")]
    Light,
    /// Monochrome dark (black) icon, for light taskbars.
    #[serde(alias = "LightPanel")]
    Dark,
}

/// Firmware package installed on the monitor. Both report 5.1.1 over DDC, so the user picks it.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum FirmwarePackage {
    /// "P275MV PLUS Firmware Update (M001 20241211 5.1.1)" - the original, public package.
    #[default]
    M001_20241211,
    /// "P275MV PLUS (增强版) 固件补丁升级包 (M001 20250920 5.1.1)" - newer, for the Chinese Enhanced version.
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

// ───────────────────────────── paths ─────────────────────────────

/// `%APPDATA%\Titan Control`: settings and the monitor cache.
pub fn config_dir() -> Option<PathBuf> {
    base_dir("APPDATA").map(|dir| dir.join(APP_DISPLAY_NAME))
}

/// `%LOCALAPPDATA%\Titan Control`: log file.
pub fn local_dir() -> Option<PathBuf> {
    base_dir("LOCALAPPDATA").map(|dir| dir.join(APP_DISPLAY_NAME))
}

fn base_dir(variable: &str) -> Option<PathBuf> {
    std::env::var_os(variable)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .or_else(|| {
            // Running under a system account without the usual profile variables.
            std::env::var_os("USERPROFILE")
                .map(PathBuf::from)
                .map(|profile| profile.join(variable.to_lowercase()))
        })
}

fn settings_file() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("app_settings.json"))
}

pub fn load() -> AppSettings {
    let Some(path) = settings_file() else {
        return AppSettings::default();
    };
    fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str::<AppSettings>(&data).ok())
        .unwrap_or_default()
}

pub fn save(settings: &AppSettings) -> io::Result<()> {
    let path = settings_file().ok_or_else(|| io::Error::other("no configuration directory"))?;
    ensure_parent_dir(&path)?;
    let data = serde_json::to_string_pretty(settings)?;
    fs::write(path, data)
}

fn ensure_parent_dir(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

// ───────────────────────────── autostart ─────────────────────────────

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

/// Adds or removes the `Run` entry so that the application starts with Windows in the tray.
pub fn sync_autostart(settings: &AppSettings) -> io::Result<()> {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ,
    };

    let key = crate::win::wide(RUN_KEY);
    let name = crate::win::wide(APP_REGISTRY_NAME);
    let mut handle: HKEY = std::ptr::null_mut();
    unsafe {
        if RegOpenKeyExW(HKEY_CURRENT_USER, key.as_ptr(), 0, KEY_SET_VALUE, &mut handle) != 0 {
            return Err(io::Error::other(crate::win::last_error()));
        }
        let result = if settings.auto_start {
            let executable = std::env::current_exe()?;
            // The window stays in the tray; quoting keeps paths with spaces working.
            let command = crate::win::wide(&format!("\"{}\" --minimized", executable.display()));
            RegSetValueExW(
                handle,
                name.as_ptr(),
                0,
                REG_SZ,
                command.as_ptr() as *const u8,
                (command.len() * std::mem::size_of::<u16>()) as u32,
            )
        } else {
            // Removing an entry that is not there is not an error.
            RegDeleteValueW(handle, name.as_ptr())
        };
        RegCloseKey(handle);
        if result != 0 && result != 2 {
            Err(io::Error::other(crate::win::last_error()))
        } else {
            Ok(())
        }
    }
}

/// True when the application is registered to start with Windows.
pub fn autostart_enabled() -> bool {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, REG_NONE,
    };
    let key = crate::win::wide(RUN_KEY);
    let name = crate::win::wide(APP_REGISTRY_NAME);
    unsafe {
        let mut handle: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(HKEY_CURRENT_USER, key.as_ptr(), 0, KEY_QUERY_VALUE, &mut handle) != 0 {
            return false;
        }
        let mut kind = REG_NONE;
        let mut size = 0u32;
        let status =
            RegQueryValueExW(handle, name.as_ptr(), std::ptr::null_mut(), &mut kind, std::ptr::null_mut(), &mut size);
        RegCloseKey(handle);
        status == 0
    }
}

/// Opens the folder that holds the log file in Explorer.
pub fn open_log_folder() {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let Some(dir) = crate::log::dir() else {
        return;
    };
    let operation = crate::win::wide("open");
    let target = crate::win::wide(&dir.to_string_lossy());
    unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
    }
}

/// Opens a link (project page, issue tracker) in the default browser.
pub fn open_url(url: &str) {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let operation = crate::win::wide("open");
    let target = crate::win::wide(url);
    unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
    }
    log!("[APP] opened {url}");
}
