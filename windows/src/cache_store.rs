//! The monitor cache: last known settings per monitor, the User 1-3 RGB profiles per color temperature and a small
//! snapshot of the monitor info. Same JSON layout as the Linux application (`settings_cache.json`), so a cache copied
//! between systems keeps working.
//!
//! Every access re-reads and rewrites the whole file: the file is a few kilobytes and the application writes it only
//! when the user changes something.

use std::{collections::HashMap, fs};

use serde::{Deserialize, Serialize};

use crate::settings::config_dir;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct CachedMonitorSnapshot {
    pub resolution: String,
    pub hz: u16,
    pub serial: String,
    pub firm: String,
    pub usage_mins: u16,
}

#[derive(Default, Serialize, Deserialize)]
struct CacheStore {
    #[serde(default)]
    monitors: HashMap<String, HashMap<u8, u16>>,
    #[serde(default)]
    color_profiles: HashMap<String, HashMap<u16, HashMap<u8, u16>>>,
    #[serde(default)]
    snapshots: HashMap<String, CachedMonitorSnapshot>,
    #[serde(default)]
    last_monitor: Option<String>,
    #[serde(default)]
    last_connector: Option<String>,
}

fn cache_file() -> Option<std::path::PathBuf> {
    config_dir().map(|dir| dir.join("settings_cache.json"))
}

fn load_store() -> CacheStore {
    let Some(path) = cache_file() else {
        return CacheStore::default();
    };
    fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

fn save_store(store: &CacheStore) {
    let Some(path) = cache_file() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(store) {
        Ok(data) => {
            if let Err(error) = fs::write(&path, data) {
                log!("[CACHE] cannot write {}: {error}", path.display());
            }
        }
        Err(error) => log!("[CACHE] cannot encode the cache: {error}"),
    }
}

pub fn load_settings(name: &str) -> HashMap<u8, u16> {
    load_store().monitors.get(name).cloned().unwrap_or_default()
}

pub fn save_settings(name: &str, settings: &HashMap<u8, u16>) {
    let mut store = load_store();
    store.monitors.insert(name.to_string(), settings.clone());
    store.last_monitor = Some(name.to_string());
    save_store(&store);
}

pub fn load_color_temp_profile(name: &str, color_temp: u16) -> HashMap<u8, u16> {
    load_store()
        .color_profiles
        .get(name)
        .and_then(|profiles| profiles.get(&color_temp))
        .cloned()
        .unwrap_or_default()
}

pub fn save_color_temp_profile(name: &str, color_temp: u16, settings: &HashMap<u8, u16>) {
    let mut store = load_store();
    store
        .color_profiles
        .entry(name.to_string())
        .or_default()
        .insert(color_temp, settings.clone());
    store.last_monitor = Some(name.to_string());
    save_store(&store);
}

pub fn clear_color_temp_profile(name: &str) {
    let mut store = load_store();
    store.color_profiles.remove(name);
    save_store(&store);
}

pub fn load_snapshot(name: &str) -> Option<CachedMonitorSnapshot> {
    load_store().snapshots.get(name).cloned()
}

pub fn save_snapshot(name: &str, snapshot: &CachedMonitorSnapshot) {
    let mut store = load_store();
    store.snapshots.insert(name.to_string(), snapshot.clone());
    store.last_monitor = Some(name.to_string());
    save_store(&store);
}

pub fn update_snapshot_usage_time(name: &str, usage_mins: u16) {
    let mut store = load_store();
    if let Some(snapshot) = store.snapshots.get_mut(name) {
        snapshot.usage_mins = usage_mins;
        save_store(&store);
    }
}

pub fn load_last_monitor() -> Option<String> {
    load_store().last_monitor
}

pub fn save_last_monitor(name: &str, connector: &str) {
    let mut store = load_store();
    store.last_monitor = Some(name.to_string());
    store.last_connector = Some(connector.to_string());
    save_store(&store);
}
