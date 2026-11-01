//! Log file for bug reports: everything the application prints with `log!` goes to stderr (when there is a console)
//! and to `%LOCALAPPDATA%\Titan Control\titan_control.log`. The file is cut when it grows past 1 MiB (the previous
//! one is kept as `titan_control.log.1`), so it never becomes large.

use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::Mutex,
};

const MAX_SIZE: u64 = 1024 * 1024;

static FILE: Mutex<Option<File>> = Mutex::new(None);

/// Writes a line to stderr and to the log file, with a timestamp.
macro_rules! log {
    ($($arg:tt)*) => {
        $crate::log::write(format_args!($($arg)*))
    };
}

pub fn dir() -> Option<PathBuf> {
    crate::settings::local_dir()
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

/// Local time as `YYYY-MM-DD HH:MM:SS.mmm` without pulling in a date library.
fn timestamp() -> String {
    use windows_sys::Win32::Foundation::SYSTEMTIME;
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;
    unsafe {
        let mut now: SYSTEMTIME = std::mem::zeroed();
        GetLocalTime(&mut now);
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
            now.wYear, now.wMonth, now.wDay, now.wHour, now.wMinute, now.wSecond, now.wMilliseconds
        )
    }
}

/// Opens the log file (cutting an oversized one), records what a bug report needs and logs panics.
pub fn init() {
    let Some(dir) = dir() else {
        return;
    };
    let path = dir.join("titan_control.log");
    let _ = fs::create_dir_all(&dir);
    if fs::metadata(&path).is_ok_and(|meta| meta.len() > MAX_SIZE) {
        let _ = fs::rename(&path, path.with_extension("log.1"));
    }
    if let Ok(file) = OpenOptions::new().create(true).append(true).open(&path)
        && let Ok(mut guard) = FILE.lock()
    {
        *guard = Some(file);
    }
    log!("---- Titan Control {} started ----", crate::settings::APP_VERSION);
    log!(
        "system: {} | {}",
        os_version(),
        std::env::consts::ARCH,
    );
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log!("PANIC: {info}");
        previous(info);
    }));
}

/// Windows build name and number as reported by the registry.
fn os_version() -> String {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE, REG_NONE,
    };
    let key = crate::win::wide("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion");
    let names = [crate::win::wide("ProductName"), crate::win::wide("CurrentBuildNumber")];
    let mut parts = Vec::new();
    unsafe {
        let mut handle: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, key.as_ptr(), 0, KEY_QUERY_VALUE, &mut handle) != 0 {
            return "Windows".into();
        }
        for name in &names {
            let mut buffer = [0u16; 128];
            let mut size = (buffer.len() * 2) as u32;
            let mut kind = REG_NONE;
            if RegQueryValueExW(
                handle,
                name.as_ptr(),
                std::ptr::null_mut(),
                &mut kind,
                buffer.as_mut_ptr() as *mut u8,
                &mut size,
            ) == 0
            {
                parts.push(crate::win::from_wide(&buffer));
            }
        }
        RegCloseKey(handle);
    }
    if parts.is_empty() { "Windows".into() } else { parts.join(" ") }
}
