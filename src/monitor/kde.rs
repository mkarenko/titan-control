//! KDE Wayland display settings through KScreen's public command line interface.
use super::{SystemDisplayState, is_internal_output};
use serde::Deserialize;
use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Deserialize)]
struct Config {
    outputs: Vec<Output>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Output {
    name: String,
    enabled: bool,
    connected: bool,
    current_mode_id: Option<String>,
    #[serde(default = "default_scale")]
    scale: f64,
    #[serde(default)]
    modes: Vec<Mode>,
    /// Missing when the output cannot do HDR.
    hdr: Option<bool>,
    /// 1 = primary screen; Plasma numbers its desktops (`lastScreen`) by priority from 0.
    #[serde(default)]
    priority: u32,
}

fn default_scale() -> f64 {
    1.0
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Mode {
    id: String,
    refresh_rate: f64,
    size: Size,
}

#[derive(Deserialize)]
struct Size {
    width: u32,
    height: u32,
}

impl Mode {
    fn resolution(&self) -> String {
        format!("{}x{}", self.size.width, self.size.height)
    }

    fn refresh(&self) -> String {
        format!("{:.3}", self.refresh_rate)
    }
}

fn read_config() -> io::Result<Config> {
    let result = Command::new("kscreen-doctor").arg("--json").output()?;
    if !result.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&result.stderr).trim().to_owned(),
        ));
    }
    serde_json::from_slice(&result.stdout).map_err(io::Error::other)
}

fn select_output<'a>(config: &'a Config, preferred: Option<&str>) -> io::Result<&'a Output> {
    let active = |output: &&Output| output.enabled && output.connected;
    let selected = if let Some(name) = preferred {
        // Never change another screen when the monitor's connector disappears.
        config
            .outputs
            .iter()
            .filter(active)
            .find(|output| output.name == name)
    } else {
        config
            .outputs
            .iter()
            .filter(active)
            .find(|output| !is_internal_output(&output.name))
            .or_else(|| config.outputs.iter().find(active))
    };
    selected.ok_or_else(|| {
        io::Error::other("The requested display is not connected or enabled in KScreen")
    })
}

pub(super) fn query(preferred: Option<&str>) -> io::Result<SystemDisplayState> {
    let config = read_config()?;
    let output = select_output(&config, preferred)?;
    let current = output
        .modes
        .iter()
        .find(|mode| Some(mode.id.as_str()) == output.current_mode_id.as_deref())
        .ok_or_else(|| io::Error::other("KScreen did not report the active display mode"))?;
    let mut modes = output.modes.iter().collect::<Vec<_>>();
    modes.sort_by(|a, b| {
        b.size
            .width
            .cmp(&a.size.width)
            .then(b.size.height.cmp(&a.size.height))
            .then(b.refresh_rate.total_cmp(&a.refresh_rate))
    });
    let mut resolutions = Vec::new();
    let mut refresh_rates: HashMap<String, Vec<String>> = HashMap::new();
    for mode in modes {
        let resolution = mode.resolution();
        if !resolutions.contains(&resolution) {
            resolutions.push(resolution.clone());
        }
        let rates = refresh_rates.entry(resolution).or_default();
        let rate = mode.refresh();
        if !rates.contains(&rate) {
            rates.push(rate);
        }
    }
    let scale = (output.scale * 100.0).round() as u16;
    let mut scaling_options = vec![100, 125, 150, 175, 200, 225, 250, 275, 300, scale];
    scaling_options.sort_unstable();
    scaling_options.dedup();
    Ok(SystemDisplayState {
        output: output.name.clone(),
        current_resolution: current.resolution(),
        current_refresh: current.refresh(),
        current_scale_percent: scale,
        available_resolutions: resolutions,
        refresh_rates,
        scaling_options,
        hdr: output.hdr,
        screen_index: output.priority.checked_sub(1).map(|index| index as usize),
    })
}

/// Turns HDR of the output on or off (KWin then also uses the wide color gamut).
pub(super) fn set_hdr(name: &str, enable: bool) -> io::Result<()> {
    let state = if enable { "enable" } else { "disable" };
    // HDR together with the wide color gamut, as the KDE display settings do.
    let result = Command::new("kscreen-doctor")
        .arg(format!("output.{name}.hdr.{state}"))
        .arg(format!("output.{name}.wcg.{state}"))
        .output()?;
    if result.status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{} {}",
            String::from_utf8_lossy(&result.stdout).trim(),
            String::from_utf8_lossy(&result.stderr).trim()
        )))
    }
}

/// Image of the Plasma desktop on screen `screen` (static image or the current slideshow picture).
pub(super) fn wallpaper(screen: usize) -> Option<PathBuf> {
    let config = dirs_config()?.join("plasma-org.kde.plasma.desktop-appletsrc");
    let text = std::fs::read_to_string(config).ok()?;
    let groups = parse_ini(&text);
    let screen = screen.to_string();
    // Desktop containments (not panels) of this screen; each knows its wallpaper plugin.
    let (_, id, plugin) = groups
        .iter()
        .filter_map(|(name, keys)| {
            let id = name.strip_prefix("Containments][")?;
            let desktop = keys.get("formfactor").is_none_or(|value| value == "0")
                && keys.get("plugin").is_some_and(|plugin| plugin != "org.kde.panel");
            if id.contains(']') || !desktop || keys.get("lastScreen") != Some(&screen) {
                return None;
            }
            Some((id.parse::<u32>().ok()?, id, keys.get("wallpaperplugin")?))
        })
        .min()?;
    let image = groups.get(&format!("Containments][{id}][Wallpaper][{plugin}][General"))?.get("Image")?;
    resolve_image(Path::new(image.strip_prefix("file://").unwrap_or(image)))
}

/// A wallpaper package (directory) holds the picture in several sizes: the largest one.
fn resolve_image(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path.to_path_buf());
    }
    let pixels = |file: &PathBuf| -> u64 {
        let stem = file.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default();
        stem.split_once('x')
            .and_then(|(width, height)| Some(width.parse::<u64>().ok()? * height.parse::<u64>().ok()?))
            .unwrap_or(0)
    };
    std::fs::read_dir(path.join("contents/images"))
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|file| file.is_file())
        .max_by_key(pixels)
}

fn dirs_config() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
}

/// KConfig file: `[A][B]` groups (stored as `A][B`) with their keys.
fn parse_ini(text: &str) -> HashMap<String, HashMap<String, String>> {
    let mut groups: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current = String::new();
    for line in text.lines().map(str::trim) {
        if let Some(name) = line.strip_prefix('[').and_then(|line| line.strip_suffix(']')) {
            current = name.to_string();
        } else if let Some((key, value)) = line.split_once('=') {
            groups.entry(current.clone()).or_default().insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    groups
}

pub(super) fn apply(
    name: &str,
    resolution: &str,
    refresh: Option<&str>,
    scale: Option<u16>,
) -> io::Result<()> {
    let config = read_config()?;
    let output = select_output(&config, Some(name))?;
    // Use mode IDs, including fractional refresh rates, rather than guessing a timing.
    let mode = output
        .modes
        .iter()
        .filter(|mode| {
            mode.resolution() == resolution && refresh.is_none_or(|rate| mode.refresh() == rate)
        })
        .min_by_key(|mode| Some(mode.id.as_str()) != output.current_mode_id.as_deref())
        .ok_or_else(|| io::Error::other("The selected display mode is no longer available"))?;
    let mut command = Command::new("kscreen-doctor");
    command.arg(format!("output.{}.mode.{}", output.name, mode.id));
    if let Some(scale) = scale {
        if scale == 0 {
            return Err(io::Error::other("Display scale must be greater than zero"));
        }
        command.arg(format!(
            "output.{}.scale.{:.2}",
            output.name,
            scale as f64 / 100.0
        ));
    }
    let result = command.output()?;
    if result.status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{}\n{}",
            String::from_utf8_lossy(&result.stdout).trim(),
            String::from_utf8_lossy(&result.stderr).trim()
        )))
    }
}
