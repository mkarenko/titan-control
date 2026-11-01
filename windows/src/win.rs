//! Thin Win32 helpers: string conversion, error text, icon loading, DPI and theme plumbing.

use std::{ffi::c_void, mem, ptr};

use windows_sys::Win32::Foundation::{GetLastError, HWND};
use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
use windows_sys::Win32::Graphics::Gdi::{
    CreateBitmap, CreateDIBSection, DeleteObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP,
    HDC, HGDIOBJ,
};
use windows_sys::Win32::UI::HiDpi::{GetDpiForSystem, GetDpiForWindow};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, DestroyIcon, GetSystemMetrics, ICONINFO, SM_CXSCREEN, SM_CYSCREEN,
};

/// A null-terminated UTF-16 string for the `W` APIs.
pub fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Copies a UTF-16 buffer that the API terminates.
pub fn from_wide(buffer: &[u16]) -> String {
    let end = buffer.iter().position(|&unit| unit == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

pub fn last_error() -> String {
    let code = unsafe { GetLastError() };
    std::io::Error::from_raw_os_error(code as i32).to_string()
}

// ───────────────────────────── icons ─────────────────────────────

/// Builds an `HICON` from an `.ico` file held in memory, picking the entry closest to `wanted` pixels.
pub fn icon_from_ico(data: &[u8], wanted: u32) -> Option<*mut c_void> {
    if data.len() < 6 || data[0] != 0 || data[1] != 0 || data[2] != 1 || data[3] != 0 {
        return None;
    }
    let count = u16::from_le_bytes([data[4], data[5]]) as usize;
    let mut best: Option<(u32, usize, usize)> = None;
    for index in 0..count {
        let entry = 6 + index * 16;
        if entry + 16 > data.len() {
            return None;
        }
        let size = if data[entry] == 0 { 256 } else { data[entry] as u32 };
        let length =
            u32::from_le_bytes([data[entry + 8], data[entry + 9], data[entry + 10], data[entry + 11]]) as usize;
        let offset =
            u32::from_le_bytes([data[entry + 12], data[entry + 13], data[entry + 14], data[entry + 15]]) as usize;
        if offset + length > data.len() {
            return None;
        }
        let distance = size.abs_diff(wanted);
        if best.is_none_or(|(best_size, _, _)| distance < best_size.abs_diff(wanted)) {
            best = Some((size, offset, length));
        }
    }
    let (size, offset, length) = best?;
    create_icon(&data[offset..offset + length], size)
}

fn create_icon(dib: &[u8], size: u32) -> Option<*mut c_void> {
    // Resource layout: BITMAPINFOHEADER, the 32-bit XOR bitmap (bottom-up), the AND mask.
    if dib.len() < 40 {
        return None;
    }
    let xor_size = (size * size * 4) as usize;
    let mask_row = size.div_ceil(32) * 4;
    let mask_size = (mask_row * size) as usize;
    if dib.len() < 40 + xor_size {
        return None;
    }

    unsafe {
        let mut info: BITMAPINFO = mem::zeroed();
        info.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = size as i32;
        info.bmiHeader.biHeight = -(size as i32); // top-down; the resource is bottom-up
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB;

        let mut bits: *mut c_void = ptr::null_mut();
        let color: HBITMAP =
            CreateDIBSection(ptr::null_mut::<c_void>() as HDC, &info, DIB_RGB_COLORS, &mut bits, ptr::null_mut(), 0);
        if color.is_null() || bits.is_null() {
            return None;
        }
        let source = &dib[40..40 + xor_size];
        let destination = std::slice::from_raw_parts_mut(bits as *mut u8, xor_size);
        let row_bytes = (size * 4) as usize;
        for row in 0..size as usize {
            let from = (size as usize - 1 - row) * row_bytes;
            destination[row * row_bytes..(row + 1) * row_bytes]
                .copy_from_slice(&source[from..from + row_bytes]);
        }

        let mask_bits = if dib.len() >= 40 + xor_size + mask_size {
            Some(&dib[40 + xor_size..40 + xor_size + mask_size])
        } else {
            None
        };
        let mask: HBITMAP = match mask_bits {
            Some(bits) => CreateBitmap(size as i32, size as i32, 1, 1, bits.as_ptr() as *const c_void),
            None => CreateBitmap(size as i32, size as i32, 1, 1, ptr::null()),
        };
        if mask.is_null() {
            DeleteObject(color as HGDIOBJ);
            return None;
        }

        let icon_info = ICONINFO { fIcon: 1, xHotspot: 0, yHotspot: 0, hbmMask: mask, hbmColor: color };
        let icon = CreateIconIndirect(&icon_info);
        DeleteObject(color as HGDIOBJ);
        DeleteObject(mask as HGDIOBJ);
        if icon.is_null() { None } else { Some(icon as *mut c_void) }
    }
}

pub fn destroy_icon(icon: *mut c_void) {
    if !icon.is_null() {
        unsafe {
            DestroyIcon(icon);
        }
    }
}

/// `HICON` from `.ico` data, scaled for the DPI of `window` (or of the system when there is no window yet).
pub fn icon_for_dpi(data: &[u8], window: HWND, base: u32) -> *mut c_void {
    let dpi = if window.is_null() { system_dpi() } else { dpi_of(window) };
    let wanted = (base as u64 * dpi as u64 / 96) as u32;
    icon_from_ico(data, wanted).unwrap_or(ptr::null_mut())
}

// ───────────────────────────── DPI and geometry ─────────────────────────────

/// DPI of a window; the manifest declares per-monitor awareness.
pub fn dpi_of(window: HWND) -> u32 {
    let dpi = unsafe { GetDpiForWindow(window) };
    if dpi == 0 { system_dpi() } else { dpi }
}

pub fn system_dpi() -> u32 {
    unsafe { GetDpiForSystem() }.max(96)
}

/// Scales a length given at 96 DPI.
pub fn scaled(value: i32, dpi: u32) -> i32 {
    (value as i64 * dpi as i64 / 96) as i32
}

pub fn screen_size() -> (i32, i32) {
    unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) }
}

/// Places a window in the middle of the primary screen.
pub fn center(window: HWND, width: i32, height: i32) {
    let (screen_width, screen_height) = screen_size();
    let x = ((screen_width - width) / 2).max(0);
    let y = ((screen_height - height) / 2).max(0);
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos(
            window,
            std::ptr::null_mut(),
            x,
            y,
            width,
            height,
            windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOZORDER
                | windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE,
        );
    }
}

// ───────────────────────────── dark mode ─────────────────────────────

/// Lets the window manager paint the frame and the title bar dark (or like the rest of the system).
pub fn set_dark_frame(window: HWND, dark: bool) {
    let value: i32 = if dark { 1 } else { 0 };
    unsafe {
        DwmSetWindowAttribute(
            window,
            DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
            &value as *const i32 as *const c_void,
            mem::size_of::<i32>() as u32,
        );
    }
}

/// Applies the explorer dark theme to a control, which covers scroll bars, combo boxes and edit fields.
pub fn set_control_theme(window: HWND, dark: bool) {
    #[link(name = "uxtheme")]
    unsafe extern "system" {
        fn SetWindowTheme(hwnd: HWND, sub_app_name: *const u16, sub_id_list: *const u16) -> i32;
    }
    let theme = if dark { wide("DarkMode_Explorer") } else { wide("") };
    let empty = wide("");
    unsafe {
        // `AllowDarkModeForWindow` (ordinal 133) is what makes the classic controls accept the dark theme.
        if dark {
            allow_dark_mode(window, true);
        }
        SetWindowTheme(window, theme.as_ptr(), empty.as_ptr());
    }
}

/// Tells `uxtheme` that this window may be drawn dark (undocumented, ignored where it does not exist).
fn allow_dark_mode(window: HWND, allow: bool) {
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    let library = wide("uxtheme.dll");
    unsafe {
        let module = LoadLibraryW(library.as_ptr());
        if module.is_null() {
            return;
        }
        if let Some(address) = GetProcAddress(module, 133 as usize as *const u8) {
            let function: unsafe extern "system" fn(HWND, i32) -> i32 = mem::transmute(address);
            function(window, if allow { 1 } else { 0 });
        }
    }
}

/// Whether the system asks applications for a dark look ("Settings → Personalisation → Colours").
pub fn system_prefers_dark() -> bool {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, REG_DWORD,
    };
    let key = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    let value = wide("AppsUseLightTheme");
    unsafe {
        let mut handle: HKEY = ptr::null_mut();
        if RegOpenKeyExW(HKEY_CURRENT_USER, key.as_ptr(), 0, KEY_QUERY_VALUE, &mut handle) != 0 {
            return false;
        }
        let mut data = 0u32;
        let mut size = mem::size_of::<u32>() as u32;
        let mut kind = REG_DWORD;
        let status = RegQueryValueExW(
            handle,
            value.as_ptr(),
            ptr::null_mut(),
            &mut kind,
            &mut data as *mut u32 as *mut u8,
            &mut size,
        );
        RegCloseKey(handle);
        status == 0 && data == 0
    }
}

/// Asks the common controls to render dark, the switch every Win32 application with dark mode support uses
/// (`uxtheme.dll` ordinals 135 and 136). Unsupported systems simply ignore it.
pub fn set_process_dark_mode(dark: bool) {
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    let library = wide("uxtheme.dll");
    unsafe {
        let module = LoadLibraryW(library.as_ptr());
        if module.is_null() {
            return;
        }
        if let Some(set_preferred_app_mode) = GetProcAddress(module, 135 as usize as *const u8) {
            let function: unsafe extern "system" fn(i32) -> i32 = std::mem::transmute(set_preferred_app_mode);
            // 2 = force dark, 3 = force light.
            function(if dark { 2 } else { 3 });
        }
        if let Some(flush_menu_themes) = GetProcAddress(module, 136 as usize as *const u8) {
            let function: unsafe extern "system" fn() = std::mem::transmute(flush_menu_themes);
            function();
        }
    }
}