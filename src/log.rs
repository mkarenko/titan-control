//! Log file for bug reports: everything the app prints with `log!` goes to stderr and to
//! `~/.local/state/titan_control/titan_control.log` (`$XDG_STATE_HOME` is respected). The file is cut when it
//! grows past 1 MiB (the previous one is kept as `titan_control.log.1`), so it never becomes large.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

const MAX_SIZE: u64 = 1024 * 1024;

static FILE: Mutex<Option<File>> = Mutex::new(None);

/// Writes a line to stderr and to the log file, with a timestamp.
macro_rules! log {
    ($($arg:tt)*) => {
        $crate::log::write(format_args!($($arg)*))
    };
}

pub fn dir() -> PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".local").join("state")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("titan_control")
}

pub fn file_path() -> PathBuf {
    dir().join("titan_control.log")
}

pub fn write(message: std::fmt::Arguments) {
    let line = format!("{} {message}", timestamp());
    eprintln!("{message}");
    if let Ok(mut guard) = FILE.lock()
        && let Some(file) = guard.as_mut()
    {
        let _ = writeln!(file, "{line}");
    }
}

fn timestamp() -> String {
    gtk4::glib::DateTime::now_local()
        .ok()
        .and_then(|now| {
            let text = now.format("%Y-%m-%d %H:%M:%S").ok()?;
            Some(format!("{text}.{:03}", now.microsecond() / 1000))
        })
        .unwrap_or_default()
}

/// Opens the log file (cutting an oversized one), records what a bug report needs and logs panics.
pub fn init() {
    let path = file_path();
    let _ = fs::create_dir_all(dir());
    if fs::metadata(&path).is_ok_and(|meta| meta.len() > MAX_SIZE) {
        let _ = fs::rename(&path, path.with_extension("log.1"));
    }
    if let Ok(file) = OpenOptions::new().create(true).append(true).open(&path)
        && let Ok(mut guard) = FILE.lock()
    {
        *guard = Some(file);
    }
    log!("---- Titan Control {} started ----", crate::app_settings::APP_VERSION);
    let env = |name: &str| std::env::var(name).unwrap_or_else(|_| "-".into());
    log!(
        "system: {} | kernel {} | desktop {} | session {}",
        os_release(),
        fs::read_to_string("/proc/sys/kernel/osrelease").unwrap_or_default().trim(),
        env("XDG_CURRENT_DESKTOP"),
        env("XDG_SESSION_TYPE"),
    );
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log!("PANIC: {info}");
        previous(info);
    }));
}

/// Library versions (needs GTK to be initialized, so it is logged when the application activates).
pub fn versions() {
    log!(
        "GTK {}.{}.{} | libadwaita {}.{}.{}",
        gtk4::major_version(),
        gtk4::minor_version(),
        gtk4::micro_version(),
        libadwaita::major_version(),
        libadwaita::minor_version(),
        libadwaita::micro_version(),
    );
}

fn os_release() -> String {
    fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|text| {
            text.lines()
                .find_map(|line| line.strip_prefix("PRETTY_NAME="))
                .map(|name| name.trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "unknown".into())
}
