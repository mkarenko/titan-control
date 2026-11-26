//! Lightweight Auto HDR automation: watches launched applications via Windows foreground events
//! and kernel process waits (zero CPU when disabled, near-zero when idle), and toggles Windows HDR
//! via DisplayConfig APIs (`Win11 24H2+` and classic fallback).

use std::{
    collections::{HashMap, HashSet},
    ffi::c_void,
    fs, mem,
    path::{Path, PathBuf},
    ptr,
    sync::atomic::{AtomicI32, AtomicIsize, Ordering},
};

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, HWND, INVALID_HANDLE_VALUE, LPARAM};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegEnumValueW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER,
    HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE, KEY_READ, REG_EXPAND_SZ, REG_NONE, REG_SZ,
};
use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, GetWindowThreadProcessId, PostMessageW, HWND_BROADCAST, SM_CXSCREEN, SM_CYSCREEN,
    WM_DISPLAYCHANGE,
};

use crate::{
    settings::{self, AutoProfileApp},
    win,
};

const SYNCHRONIZE_ACCESS: u32 = 0x0010_0000;
const WAIT_OBJECT_0: u32 = 0;
const INFINITE_WAIT: u32 = 0xFFFF_FFFF;
const WT_EXECUTEONLYONCE: u32 = 0x0000_0008;
const EVENT_SYSTEM_FOREGROUND: u32 = 0x0003;
const WINEVENT_OUTOFCONTEXT: u32 = 0x0000;
const WINEVENT_SKIPOWNPROCESS: u32 = 0x0002;
const TH32CS_SNAPPROCESS: u32 = 0x0000_0002;

static HOOK_HOST: AtomicIsize = AtomicIsize::new(0);
static HDR_API_MODE: AtomicI32 = AtomicI32::new(-1); // -1 unknown, 0 legacy, 1 win11 24h2+

#[link(name = "user32")]
unsafe extern "system" {
    fn SetWinEventHook(
        event_min: u32,
        event_max: u32,
        hmod_win_event_proc: *mut c_void,
        pfn_win_event_proc: Option<unsafe extern "system" fn(isize, u32, HWND, i32, i32, u32, u32)>,
        id_process: u32,
        id_thread: u32,
        dw_flags: u32,
    ) -> isize;
    fn UnhookWinEvent(h_win_event_hook: isize) -> i32;
    fn GetDisplayConfigBufferSizes(flags: u32, num_paths: *mut u32, num_modes: *mut u32) -> i32;
    fn QueryDisplayConfig(
        flags: u32,
        num_paths: *mut u32,
        paths: *mut DisplayConfigPathInfo,
        num_modes: *mut u32,
        modes: *mut DisplayConfigModeInfo,
        current_topology_id: *mut c_void,
    ) -> i32;
    fn DisplayConfigGetDeviceInfo(packet: *mut c_void) -> i32;
    fn DisplayConfigSetDeviceInfo(packet: *const c_void) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn QueryFullProcessImageNameW(process: HANDLE, flags: u32, exe_name: *mut u16, size: *mut u32) -> i32;
    fn CreateToolhelp32Snapshot(flags: u32, th32_process_id: u32) -> HANDLE;
    fn Process32FirstW(snapshot: HANDLE, entry: *mut ProcessEntry32W) -> i32;
    fn Process32NextW(snapshot: HANDLE, entry: *mut ProcessEntry32W) -> i32;
    fn RegisterWaitForSingleObject(
        new_wait_object: *mut HANDLE,
        object: HANDLE,
        callback: Option<unsafe extern "system" fn(*mut c_void, u8)>,
        context: *mut c_void,
        milliseconds: u32,
        flags: u32,
    ) -> i32;
    fn UnregisterWaitEx(wait_handle: HANDLE, completion_event: HANDLE) -> i32;
}

#[link(name = "comdlg32")]
unsafe extern "system" {
    fn GetOpenFileNameW(ofn: *mut OpenFileNameW) -> i32;
}

#[repr(C)]
struct ProcessEntry32W {
    dw_size: u32,
    cnt_usage: u32,
    th32_process_id: u32,
    th32_default_heap_id: usize,
    th32_module_id: u32,
    cnt_threads: u32,
    th32_parent_process_id: u32,
    pc_pri_class_base: i32,
    dw_flags: u32,
    sz_exe_file: [u16; 260],
}

#[repr(C)]
struct OpenFileNameW {
    l_struct_size: u32,
    hwnd_owner: HWND,
    h_instance: *mut c_void,
    lpstr_filter: *const u16,
    lpstr_custom_filter: *mut u16,
    n_max_custom_filter: u32,
    n_filter_index: u32,
    lpstr_file: *mut u16,
    n_max_file: u32,
    lpstr_file_title: *mut u16,
    n_max_file_title: u32,
    lpstr_initial_dir: *const u16,
    lpstr_title: *const u16,
    flags: u32,
    n_file_offset: u16,
    n_file_extension: u16,
    lpstr_def_ext: *const u16,
    l_cust_data: isize,
    lpfn_hook: *const c_void,
    lp_template_name: *const u16,
    pv_reserved: *mut c_void,
    dw_reserved: u32,
    flags_ex: u32,
}

// ───────────────────────────── DisplayConfig HDR API ─────────────────────────────

const QDC_ONLY_ACTIVE_PATHS: u32 = 0x0000_0002;
const ERROR_SUCCESS_CODE: i32 = 0;
const ERROR_INSUFFICIENT_BUFFER_CODE: i32 = 122;
const ERROR_NOT_SUPPORTED_CODE: i32 = 50;
const ERROR_INVALID_PARAMETER_CODE: i32 = 87;
const DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE: u32 = 1;
const DISPLAYCONFIG_PATH_MODE_IDX_INVALID: u32 = 0xFFFF_FFFF;

const DC_GET_SOURCE_NAME: i32 = 1;
const DC_GET_TARGET_NAME: i32 = 2;
const DC_GET_ADVANCED_COLOR_INFO: i32 = 9;
const DC_SET_ADVANCED_COLOR_STATE: i32 = 10;
const DC_GET_ADVANCED_COLOR_INFO_2: i32 = 15;
const DC_SET_HDR_STATE: i32 = 16;

#[repr(C)]
#[derive(Clone, Copy)]
struct Luid {
    low_part: u32,
    high_part: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DisplayConfigDeviceInfoHeader {
    kind: i32,
    size: u32,
    adapter_id: Luid,
    id: u32,
}

#[repr(C)]
struct DisplayConfigGetAdvancedColorInfo {
    header: DisplayConfigDeviceInfoHeader,
    value: u32,
    color_encoding: i32,
    bits_per_color_channel: u32,
}

#[repr(C)]
struct DisplayConfigGetAdvancedColorInfo2 {
    header: DisplayConfigDeviceInfoHeader,
    value: u32,
    color_encoding: i32,
    bits_per_color_channel: u32,
    active_color_mode: i32,
}

#[repr(C)]
struct DisplayConfigSetAdvancedColorState {
    header: DisplayConfigDeviceInfoHeader,
    enable_advanced_color: u32,
}

#[repr(C)]
struct DisplayConfigSetHdrState {
    header: DisplayConfigDeviceInfoHeader,
    value: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DisplayConfigPathSourceInfo {
    adapter_id: Luid,
    id: u32,
    mode_info_idx: u32,
    status_flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DisplayConfigPathTargetInfo {
    adapter_id: Luid,
    id: u32,
    mode_info_idx: u32,
    output_technology: u32,
    rotation: u32,
    scaling: u32,
    refresh_rate_num: u32,
    refresh_rate_den: u32,
    scan_line_ordering: u32,
    target_available: i32,
    status_flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DisplayConfigPathInfo {
    source_info: DisplayConfigPathSourceInfo,
    target_info: DisplayConfigPathTargetInfo,
    flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DisplayConfigModeInfo {
    info_type: u32,
    id: u32,
    adapter_id: Luid,
    union_words: [u64; 6],
}

#[repr(C)]
struct DisplayConfigSourceDeviceName {
    header: DisplayConfigDeviceInfoHeader,
    view_gdi_device_name: [u16; 32],
}

#[repr(C)]
struct DisplayConfigTargetDeviceName {
    header: DisplayConfigDeviceInfoHeader,
    flags: u32,
    output_technology: u32,
    edid_manufacture_id: u16,
    edid_product_code_id: u16,
    connector_instance: u32,
    monitor_friendly_device_name: [u16; 64],
    monitor_device_path: [u16; 128],
}

fn is_win11_24h2_or_newer() -> bool {
    let mut info: windows_sys::Win32::System::SystemInformation::OSVERSIONINFOW = unsafe { mem::zeroed() };
    info.dwOSVersionInfoSize = mem::size_of_val(&info) as u32;
    unsafe {
        if windows_sys::Win32::System::SystemInformation::GetVersionExW(&mut info) != 0 {
            return info.dwBuildNumber >= 26100;
        }
    }
    true
}

fn query_active_paths() -> Result<(Vec<DisplayConfigPathInfo>, Vec<DisplayConfigModeInfo>), i32> {
    let mut last_err = 0;
    for _ in 0..3 {
        let mut path_count = 0u32;
        let mut mode_count = 0u32;
        let rc = unsafe { GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count) };
        if rc != ERROR_SUCCESS_CODE {
            return Err(rc);
        }
        let mut paths = vec![unsafe { mem::zeroed::<DisplayConfigPathInfo>() }; path_count as usize];
        let mut modes = vec![unsafe { mem::zeroed::<DisplayConfigModeInfo>() }; mode_count as usize];
        let rc = unsafe {
            QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut path_count,
                paths.as_mut_ptr(),
                &mut mode_count,
                modes.as_mut_ptr(),
                ptr::null_mut(),
            )
        };
        if rc == ERROR_SUCCESS_CODE {
            paths.truncate(path_count as usize);
            modes.truncate(mode_count as usize);
            return Ok((paths, modes));
        }
        last_err = rc;
        if rc != ERROR_INSUFFICIENT_BUFFER_CODE {
            return Err(rc);
        }
    }
    Err(last_err)
}

fn source_gdi_name(path: &DisplayConfigPathInfo) -> String {
    let mut req: DisplayConfigSourceDeviceName = unsafe { mem::zeroed() };
    req.header = DisplayConfigDeviceInfoHeader {
        kind: DC_GET_SOURCE_NAME,
        size: mem::size_of::<DisplayConfigSourceDeviceName>() as u32,
        adapter_id: path.source_info.adapter_id,
        id: path.source_info.id,
    };
    let rc = unsafe { DisplayConfigGetDeviceInfo(&mut req as *mut _ as *mut c_void) };
    if rc == ERROR_SUCCESS_CODE {
        win::from_wide(&req.view_gdi_device_name)
    } else {
        String::new()
    }
}

fn target_friendly_name(adapter_id: Luid, target_id: u32) -> String {
    let mut req: DisplayConfigTargetDeviceName = unsafe { mem::zeroed() };
    req.header = DisplayConfigDeviceInfoHeader {
        kind: DC_GET_TARGET_NAME,
        size: mem::size_of::<DisplayConfigTargetDeviceName>() as u32,
        adapter_id,
        id: target_id,
    };
    let rc = unsafe { DisplayConfigGetDeviceInfo(&mut req as *mut _ as *mut c_void) };
    if rc == ERROR_SUCCESS_CODE {
        win::from_wide(&req.monitor_friendly_device_name)
    } else {
        String::new()
    }
}

fn is_primary_path(path: &DisplayConfigPathInfo, modes: &[DisplayConfigModeInfo]) -> bool {
    let idx = path.source_info.mode_info_idx;
    if idx == DISPLAYCONFIG_PATH_MODE_IDX_INVALID || idx as usize >= modes.len() {
        return false;
    }
    let mode = &modes[idx as usize];
    if mode.info_type != DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE {
        return false;
    }
    let pos_x = (mode.union_words[1] >> 32) as i32;
    let pos_y = (mode.union_words[2] & 0xFFFF_FFFF) as i32;
    pos_x == 0 && pos_y == 0
}

fn try_get_hdr_state(adapter_id: Luid, target_id: u32) -> Option<(bool, bool, bool)> {
    let api_mode = HDR_API_MODE.load(Ordering::Relaxed);
    if api_mode != 0 {
        let mut info2: DisplayConfigGetAdvancedColorInfo2 = unsafe { mem::zeroed() };
        info2.header = DisplayConfigDeviceInfoHeader {
            kind: DC_GET_ADVANCED_COLOR_INFO_2,
            size: mem::size_of::<DisplayConfigGetAdvancedColorInfo2>() as u32,
            adapter_id,
            id: target_id,
        };
        let rc = unsafe { DisplayConfigGetDeviceInfo(&mut info2 as *mut _ as *mut c_void) };
        if rc == ERROR_SUCCESS_CODE {
            HDR_API_MODE.store(1, Ordering::Relaxed);
            let supported = (info2.value & 0x10) != 0;
            let enabled = supported && ((info2.value & 0x20) != 0 || info2.active_color_mode == 2);
            return Some((supported, enabled, true));
        }
        if api_mode == -1
            && (rc == ERROR_INVALID_PARAMETER_CODE || rc == ERROR_NOT_SUPPORTED_CODE)
            && !is_win11_24h2_or_newer()
        {
            HDR_API_MODE.store(0, Ordering::Relaxed);
        }
        if HDR_API_MODE.load(Ordering::Relaxed) == 1 {
            return None;
        }
    }

    let mut info: DisplayConfigGetAdvancedColorInfo = unsafe { mem::zeroed() };
    info.header = DisplayConfigDeviceInfoHeader {
        kind: DC_GET_ADVANCED_COLOR_INFO,
        size: mem::size_of::<DisplayConfigGetAdvancedColorInfo>() as u32,
        adapter_id,
        id: target_id,
    };
    let rc = unsafe { DisplayConfigGetDeviceInfo(&mut info as *mut _ as *mut c_void) };
    if rc != ERROR_SUCCESS_CODE {
        return None;
    }
    let supported = (info.value & 0x1) != 0;
    let enabled = supported && (info.value & 0x2) != 0;
    Some((supported, enabled, false))
}

fn matching_paths<'a>(
    paths: &'a [DisplayConfigPathInfo],
    modes: &[DisplayConfigModeInfo],
    connector: Option<&str>,
) -> Vec<&'a DisplayConfigPathInfo> {
    if let Some(conn) = connector.filter(|c| !c.is_empty()) {
        let by_conn: Vec<_> = paths
            .iter()
            .filter(|p| source_gdi_name(p).eq_ignore_ascii_case(conn))
            .collect();
        if !by_conn.is_empty() {
            return by_conn;
        }
    }
    let by_name: Vec<_> = paths
        .iter()
        .filter(|p| target_friendly_name(p.target_info.adapter_id, p.target_info.id).contains("P275MV"))
        .collect();
    if !by_name.is_empty() {
        return by_name;
    }
    let by_primary: Vec<_> = paths.iter().filter(|p| is_primary_path(p, modes)).collect();
    if !by_primary.is_empty() {
        return by_primary;
    }
    paths.iter().collect()
}

/// Queries whether HDR is supported and currently enabled on the target monitor.
pub fn query_hdr(connector: Option<&str>) -> Option<(bool, bool)> {
    let (paths, modes) = query_active_paths().ok()?;
    for path in matching_paths(&paths, &modes, connector) {
        if let Some((supported, enabled, _)) = try_get_hdr_state(path.target_info.adapter_id, path.target_info.id) {
            return Some((supported, enabled));
        }
    }
    None
}

/// Enables or disables Windows HDR on the target monitor via DisplayConfig.
pub fn set_hdr(connector: Option<&str>, enable: bool) -> Result<(), String> {
    let (paths, modes) = query_active_paths().map_err(|rc| format!("QueryDisplayConfig failed ({rc})"))?;
    let mut switched_any = false;
    let mut last_error = None;
    let mut seen_targets = HashSet::new();

    for path in matching_paths(&paths, &modes, connector) {
        let key = (
            path.target_info.adapter_id.low_part,
            path.target_info.adapter_id.high_part,
            path.target_info.id,
        );
        if !seen_targets.insert(key) {
            continue;
        }
        let Some((supported, current_enabled, use_new_api)) =
            try_get_hdr_state(path.target_info.adapter_id, path.target_info.id)
        else {
            continue;
        };
        if !supported {
            continue;
        }
        if current_enabled == enable {
            return Ok(());
        }
        let rc = if use_new_api {
            let pkt = DisplayConfigSetHdrState {
                header: DisplayConfigDeviceInfoHeader {
                    kind: DC_SET_HDR_STATE,
                    size: mem::size_of::<DisplayConfigSetHdrState>() as u32,
                    adapter_id: path.target_info.adapter_id,
                    id: path.target_info.id,
                },
                value: u32::from(enable),
            };
            unsafe { DisplayConfigSetDeviceInfo(&pkt as *const _ as *const c_void) }
        } else {
            let pkt = DisplayConfigSetAdvancedColorState {
                header: DisplayConfigDeviceInfoHeader {
                    kind: DC_SET_ADVANCED_COLOR_STATE,
                    size: mem::size_of::<DisplayConfigSetAdvancedColorState>() as u32,
                    adapter_id: path.target_info.adapter_id,
                    id: path.target_info.id,
                },
                enable_advanced_color: u32::from(enable),
            };
            unsafe { DisplayConfigSetDeviceInfo(&pkt as *const _ as *const c_void) }
        };
        if rc == ERROR_SUCCESS_CODE {
            switched_any = true;
        } else {
            last_error = Some(format!("DisplayConfigSetDeviceInfo failed ({rc})"));
        }
    }

    if switched_any {
        broadcast_display_change();
        Ok(())
    } else {
        Err(last_error.unwrap_or_else(|| "No active HDR-capable display found".to_string()))
    }
}

fn broadcast_display_change() {
    unsafe {
        let width = GetSystemMetrics(SM_CXSCREEN) & 0xFFFF;
        let height = GetSystemMetrics(SM_CYSCREEN) & 0xFFFF;
        let lparam = ((height as isize) << 16) | (width as isize);
        PostMessageW(HWND_BROADCAST, WM_DISPLAYCHANGE, 32, lparam);
    }
}

// ───────────────────────────── Zero-polling watcher ─────────────────────────────

pub enum AutoEvent {
    Activated { profile: Option<String> },
    Deactivated,
}

pub struct Watcher {
    hook: isize,
    wait_handle: isize,
    proc_handle: isize,
    pub active_pid: u32,
    pub active_name: Option<String>,
    pub active_profile: Option<String>,
    pub turned_on_by_us: bool,
    host: HWND,
    last_fg: (isize, u32),
}

impl Watcher {
    pub fn new(host: HWND) -> Self {
        Self {
            hook: 0,
            wait_handle: 0,
            proc_handle: 0,
            active_pid: 0,
            active_name: None,
            active_profile: None,
            turned_on_by_us: false,
            host,
            last_fg: (0, 0),
        }
    }

    fn clear_process_wait(&mut self) {
        if self.wait_handle != 0 {
            unsafe {
                UnregisterWaitEx(self.wait_handle as HANDLE, INVALID_HANDLE_VALUE);
            }
            self.wait_handle = 0;
        }
        if self.proc_handle != 0 {
            unsafe {
                CloseHandle(self.proc_handle as HANDLE);
            }
            self.proc_handle = 0;
        }
        self.active_pid = 0;
        self.active_name = None;
        self.active_profile = None;
    }

    fn unhook(&mut self) {
        HOOK_HOST.store(0, Ordering::Relaxed);
        if self.hook != 0 {
            unsafe {
                UnhookWinEvent(self.hook);
            }
            self.hook = 0;
        }
        self.last_fg = (0, 0);
    }

    /// Synchronizes WinEvent hooks and process waits with the current `auto_profiles_enabled` & list.
    /// When disabled (or no monitored apps), releases all hooks and handles so CPU load is 0%.
    pub fn sync(&mut self, connector: &str, enabled: bool, apps: &[AutoProfileApp]) -> Option<AutoEvent> {
        if !enabled || apps.is_empty() {
            let was_active = self.active_pid != 0 || self.turned_on_by_us;
            self.unhook();
            self.clear_process_wait();
            if self.turned_on_by_us {
                if let Err(err) = set_hdr(Some(connector), false) {
                    log!("[AUTO-PROFILES] restore SDR failed: {err}");
                } else {
                    log!("[AUTO-PROFILES] disabled: restored Windows SDR");
                }
                self.turned_on_by_us = false;
            }
            return if was_active { Some(AutoEvent::Deactivated) } else { None };
        }

        if self.hook == 0 {
            HOOK_HOST.store(self.host as isize, Ordering::Relaxed);
            self.hook = unsafe {
                SetWinEventHook(
                    EVENT_SYSTEM_FOREGROUND,
                    EVENT_SYSTEM_FOREGROUND,
                    ptr::null_mut(),
                    Some(foreground_hook_proc),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
                )
            };
        }

        // Verify whether the currently watched PID still matches a listed app.
        if self.active_pid != 0 && !is_process_still_alive(self.proc_handle) {
            self.clear_process_wait();
        }
        if self.active_pid == 0 {
            if let Some((pid, app_entry)) = scan_running_for_match(apps) {
                return self.activate_for_app(connector, pid, &app_entry);
            }
            if self.turned_on_by_us {
                let _ = set_hdr(Some(connector), false);
                self.turned_on_by_us = false;
            }
        }
        None
    }

    /// Stops all hooks and waits cleanly on exit.
    pub fn shutdown(&mut self, connector: &str) {
        self.unhook();
        self.clear_process_wait();
        if self.turned_on_by_us {
            let _ = set_hdr(Some(connector), false);
            self.turned_on_by_us = false;
        }
    }

    pub fn on_foreground(
        &mut self,
        connector: &str,
        enabled: bool,
        apps: &[AutoProfileApp],
        fg_hwnd: HWND,
    ) -> Option<AutoEvent> {
        if !enabled || apps.is_empty() || fg_hwnd.is_null() {
            return None;
        }
        let mut pid = 0u32;
        unsafe {
            GetWindowThreadProcessId(fg_hwnd, &mut pid);
        }
        if pid == 0 {
            return None;
        }
        let key = (fg_hwnd as isize, pid);
        if self.last_fg == key {
            return None;
        }
        self.last_fg = key;

        if pid == self.active_pid {
            return None;
        }
        if let Some(exe_path) = query_process_path(pid)
            && let Some(matched) = match_monitored_app(&exe_path, apps)
        {
            return self.activate_for_app(connector, pid, matched);
        }
        None
    }

    pub fn on_process_exit(&mut self, connector: &str, enabled: bool, apps: &[AutoProfileApp]) -> Option<AutoEvent> {
        self.clear_process_wait();
        if enabled && !apps.is_empty() {
            if let Some((pid, app_entry)) = scan_running_for_match(apps) {
                return self.activate_for_app(connector, pid, &app_entry);
            }
        }
        if self.turned_on_by_us {
            match set_hdr(Some(connector), false) {
                Ok(()) => log!("[AUTO-PROFILES] monitored app exited: restored Windows SDR"),
                Err(err) => log!("[AUTO-PROFILES] restore SDR failed: {err}"),
            }
            self.turned_on_by_us = false;
        }
        Some(AutoEvent::Deactivated)
    }

    fn activate_for_app(&mut self, connector: &str, pid: u32, app_entry: &AutoProfileApp) -> Option<AutoEvent> {
        if self.active_pid == pid {
            return None;
        }
        self.clear_process_wait();
        let proc_handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE_ACCESS, 0, pid) };
        if proc_handle.is_null() {
            return None;
        }
        let mut wait_handle: HANDLE = ptr::null_mut();
        let ok = unsafe {
            RegisterWaitForSingleObject(
                &mut wait_handle,
                proc_handle,
                Some(process_exit_callback),
                self.host as *mut c_void,
                INFINITE_WAIT,
                WT_EXECUTEONLYONCE,
            )
        };
        if ok == 0 {
            unsafe {
                CloseHandle(proc_handle);
            }
            return None;
        }
        self.proc_handle = proc_handle as isize;
        self.wait_handle = wait_handle as isize;
        self.active_pid = pid;
        self.active_name = Some(app_entry.name.clone());
        self.active_profile = app_entry.profile.clone();

        if app_entry.auto_hdr {
            let hdr_already_on = query_hdr(Some(connector)).is_some_and(|(_, on)| on);
            if !hdr_already_on {
                match set_hdr(Some(connector), true) {
                    Ok(()) => {
                        self.turned_on_by_us = true;
                        log!("[AUTO-PROFILES] enabled Windows HDR for {} (pid {pid})", app_entry.name);
                    }
                    Err(err) => log!("[AUTO-PROFILES] failed enabling HDR for {}: {err}", app_entry.name),
                }
            } else {
                log!("[AUTO-PROFILES] tracking {} (pid {pid}; Windows HDR already on)", app_entry.name);
            }
        } else {
            log!("[AUTO-PROFILES] tracking {} (pid {pid})", app_entry.name);
        }

        Some(AutoEvent::Activated {
            profile: app_entry.profile.clone(),
        })
    }
}

unsafe extern "system" fn foreground_hook_proc(
    _hook: isize,
    _event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    let host = HOOK_HOST.load(Ordering::Relaxed);
    if host != 0 && !hwnd.is_null() {
        unsafe {
            PostMessageW(host as HWND, crate::WM_AUTO_HDR_FOREGROUND, 0, hwnd as LPARAM);
        }
    }
}

unsafe extern "system" fn process_exit_callback(context: *mut c_void, _timed_out: u8) {
    let host = context as HWND;
    if !host.is_null() {
        unsafe {
            PostMessageW(host, crate::WM_AUTO_HDR_EXIT, 0, 0);
        }
    }
}

fn is_process_still_alive(proc_handle: isize) -> bool {
    if proc_handle == 0 {
        return false;
    }
    unsafe { WaitForSingleObject(proc_handle as HANDLE, 0) != WAIT_OBJECT_0 }
}

fn query_process_path(pid: u32) -> Option<String> {
    if pid == 0 {
        return None;
    }
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut len);
        CloseHandle(handle);
        if ok == 0 || len == 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    }
}

fn match_monitored_app<'a>(exe_path: &str, apps: &'a [AutoProfileApp]) -> Option<&'a AutoProfileApp> {
    if is_system_path(exe_path) {
        return None;
    }
    let stem = settings::normalize_process_name(exe_path);
    if stem.is_empty() {
        return None;
    }
    apps.iter().find(|entry| entry.process.eq_ignore_ascii_case(&stem))
}

fn scan_running_for_match(apps: &[AutoProfileApp]) -> Option<(u32, AutoProfileApp)> {
    if apps.is_empty() {
        return None;
    }
    let self_pid = std::process::id();
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE || snap.is_null() {
            return None;
        }
        let mut entry: ProcessEntry32W = mem::zeroed();
        entry.dw_size = mem::size_of::<ProcessEntry32W>() as u32;
        let mut found = None;
        if Process32FirstW(snap, &mut entry) != 0 {
            loop {
                if entry.th32_process_id != 0 && entry.th32_process_id != self_pid {
                    let exe_name = win::from_wide(&entry.sz_exe_file);
                    let stem = settings::normalize_process_name(&exe_name);
                    if let Some(app_entry) = apps.iter().find(|a| a.process.eq_ignore_ascii_case(&stem))
                        && query_process_path(entry.th32_process_id).is_none_or(|p| !is_system_path(&p))
                    {
                        found = Some((entry.th32_process_id, app_entry.clone()));
                        break;
                    }
                }
                if Process32NextW(snap, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
        found
    }
}

fn is_system_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.starts_with(r"c:\windows\")
        || lower.contains(r"\windows\system32\")
        || lower.contains(r"\windows\syswow64\")
}

// ───────────────────────────── App Discovery & Browse ─────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredApp {
    pub name: String,
    pub process: String,
    pub path: String,
    pub source: &'static str,
}

/// Locates running, registry-registered, and Steam-installed user executables in a few milliseconds.
pub fn discover_installed_apps() -> Vec<DiscoveredApp> {
    let mut map: HashMap<String, DiscoveredApp> = HashMap::new();
    let self_stem = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_ascii_lowercase()))
        .unwrap_or_default();

    let mut insert = |name: String, path: String, source: &'static str| {
        let cleaned_path = clean_exe_path(&path);
        if cleaned_path.is_empty() || is_system_path(&cleaned_path) {
            return;
        }
        let process = settings::normalize_process_name(&cleaned_path);
        if process.is_empty() || process == self_stem || is_noise_exe(&process) {
            return;
        }
        let display_name = if name.trim().is_empty() {
            process.clone()
        } else {
            name.trim().to_string()
        };
        map.entry(process.clone()).or_insert(DiscoveredApp {
            name: display_name,
            process,
            path: cleaned_path,
            source,
        });
    };

    collect_running_apps(&mut insert);
    collect_app_paths(&mut insert);
    collect_uninstall_apps(&mut insert);
    collect_steam_apps(&mut insert);

    let mut list: Vec<DiscoveredApp> = map.into_values().collect();
    list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    list
}

fn collect_running_apps(insert: &mut impl FnMut(String, String, &'static str)) {
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE || snap.is_null() {
            return;
        }
        let mut entry: ProcessEntry32W = mem::zeroed();
        entry.dw_size = mem::size_of::<ProcessEntry32W>() as u32;
        if Process32FirstW(snap, &mut entry) != 0 {
            loop {
                if let Some(path) = query_process_path(entry.th32_process_id) {
                    let name = Path::new(&path)
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    insert(name, path, "Running");
                }
                if Process32NextW(snap, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
    }
}

fn collect_app_paths(insert: &mut impl FnMut(String, String, &'static str)) {
    let subkey = win::wide(r"Software\Microsoft\Windows\CurrentVersion\App Paths");
    for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        let mut handle: HKEY = ptr::null_mut();
        if unsafe { RegOpenKeyExW(root, subkey.as_ptr(), 0, KEY_READ, &mut handle) } != 0 {
            continue;
        }
        for child in enum_subkeys(handle) {
            if !child.to_ascii_lowercase().ends_with(".exe") {
                continue;
            }
            if let Some(path) = read_reg_string(handle, Some(&child), None)
                && Path::new(&clean_exe_path(&path)).is_file()
            {
                let name = Path::new(&child)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or(child);
                insert(name, path, "Installed");
            }
        }
        unsafe {
            RegCloseKey(handle);
        }
    }
}

fn collect_uninstall_apps(insert: &mut impl FnMut(String, String, &'static str)) {
    let paths = [
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
        r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ];
    for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        for path in paths {
            let wide_path = win::wide(path);
            let mut handle: HKEY = ptr::null_mut();
            if unsafe { RegOpenKeyExW(root, wide_path.as_ptr(), 0, KEY_READ, &mut handle) } != 0 {
                continue;
            }
            for sub in enum_subkeys(handle) {
                let wide_sub = win::wide(&sub);
                let mut entry_key: HKEY = ptr::null_mut();
                if unsafe { RegOpenKeyExW(handle, wide_sub.as_ptr(), 0, KEY_QUERY_VALUE, &mut entry_key) } != 0 {
                    continue;
                }
                let display_name = read_reg_string(entry_key, None, Some("DisplayName")).unwrap_or_default();
                if !display_name.trim().is_empty() {
                    if let Some(icon) = read_reg_string(entry_key, None, Some("DisplayIcon"))
                        && let Some(exe) = extract_exe_from_icon(&icon)
                        && Path::new(&exe).is_file()
                    {
                        insert(display_name.clone(), exe, "Installed");
                    } else if let Some(loc) = read_reg_string(entry_key, None, Some("InstallLocation"))
                        && let Some(exe) = pick_dir_exe(Path::new(loc.trim().trim_matches('"')))
                    {
                        insert(display_name, exe.to_string_lossy().into_owned(), "Installed");
                    }
                }
                unsafe {
                    RegCloseKey(entry_key);
                }
            }
            unsafe {
                RegCloseKey(handle);
            }
        }
    }
}

fn collect_steam_apps(insert: &mut impl FnMut(String, String, &'static str)) {
    let steam_key = win::wide(r"Software\Valve\Steam");
    let mut handle: HKEY = ptr::null_mut();
    if unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, steam_key.as_ptr(), 0, KEY_QUERY_VALUE, &mut handle) } != 0 {
        return;
    }
    let steam_path = read_reg_string(handle, None, Some("SteamPath"));
    unsafe {
        RegCloseKey(handle);
    }
    let Some(steam_root) = steam_path.map(PathBuf::from) else {
        return;
    };
    let mut libs = vec![steam_root.clone()];
    let lib_vdf = steam_root.join("steamapps").join("libraryfolders.vdf");
    if let Ok(content) = fs::read_to_string(lib_vdf) {
        for path in parse_steam_library_paths(&content) {
            let pb = PathBuf::from(path);
            if !libs.contains(&pb) {
                libs.push(pb);
            }
        }
    }

    for lib in libs {
        let steamapps = lib.join("steamapps");
        let Ok(entries) = fs::read_dir(&steamapps) else {
            continue;
        };
        for entry in entries.flatten() {
            let fname = entry.file_name();
            let s = fname.to_string_lossy();
            if !s.starts_with("appmanifest_") || !s.ends_with(".acf") {
                continue;
            }
            let Ok(text) = fs::read_to_string(entry.path()) else {
                continue;
            };
            let Some((name, installdir)) = parse_appmanifest_acf(&text) else {
                continue;
            };
            let game_dir = steamapps.join("common").join(installdir);
            if let Some(exe) = pick_dir_exe(&game_dir) {
                insert(name, exe.to_string_lossy().into_owned(), "Steam");
            }
        }
    }
}

fn parse_steam_library_paths(vdf: &str) -> Vec<String> {
    let mut result = Vec::new();
    for line in vdf.lines() {
        let mut cursor = line.trim();
        while let Some(idx) = cursor.find("\"path\"") {
            let after = &cursor[idx + 6..];
            if let Some(val) = extract_quoted_value(after) {
                result.push(val.replace("\\\\", "\\"));
            }
            cursor = after;
        }
    }
    result
}

fn parse_appmanifest_acf(acf: &str) -> Option<(String, String)> {
    let mut name = None;
    let mut installdir = None;
    for line in acf.lines() {
        let trimmed = line.trim();
        if name.is_none()
            && let Some(rest) = trimmed.strip_prefix("\"name\"")
        {
            name = extract_quoted_value(rest);
        } else if installdir.is_none()
            && let Some(rest) = trimmed.strip_prefix("\"installdir\"")
        {
            installdir = extract_quoted_value(rest);
        }
        if name.is_some() && installdir.is_some() {
            break;
        }
    }
    Some((name?, installdir?))
}

fn extract_quoted_value(after_key: &str) -> Option<String> {
    let first = after_key.find('"')?;
    let rest = &after_key[first + 1..];
    let second = rest.find('"')?;
    Some(rest[..second].to_string())
}

fn pick_dir_exe(dir: &Path) -> Option<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return None;
    };
    let mut candidates = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file()
            && path.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe"))
            && let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().to_ascii_lowercase())
            && !is_noise_exe(&stem)
        {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            candidates.push((size, path));
        }
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    candidates.into_iter().next().map(|(_, p)| p)
}

fn extract_exe_from_icon(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_matches('"');
    let without_idx = match trimmed.rsplit_once(',') {
        Some((before, after)) if after.trim().chars().all(|c| c.is_ascii_digit() || c == '-') => before,
        _ => trimmed,
    };
    let cleaned = clean_exe_path(without_idx);
    if cleaned.to_ascii_lowercase().ends_with(".exe") {
        Some(cleaned)
    } else {
        None
    }
}

fn clean_exe_path(raw: &str) -> String {
    raw.trim().trim_matches('"').to_string()
}

fn is_noise_exe(stem: &str) -> bool {
    const NOISE_PARTS: [&str; 12] = [
        "unins",
        "setup",
        "install",
        "redist",
        "vcredist",
        "dxsetup",
        "crashreport",
        "crashhandler",
        "updater",
        "update",
        "easyanticheat_setup",
        "battleye",
    ];
    NOISE_PARTS.iter().any(|part| stem.contains(part))
}

fn enum_subkeys(parent: HKEY) -> Vec<String> {
    let mut result = Vec::new();
    let mut index = 0u32;
    let mut buf = [0u16; 260];
    loop {
        let mut len = buf.len() as u32;
        let rc = unsafe {
            RegEnumKeyExW(
                parent,
                index,
                buf.as_mut_ptr(),
                &mut len,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };
        if rc != 0 {
            break;
        }
        result.push(String::from_utf16_lossy(&buf[..len as usize]));
        index += 1;
    }
    result
}

fn read_reg_string(root: HKEY, subkey: Option<&str>, value_name: Option<&str>) -> Option<String> {
    let mut opened: HKEY = ptr::null_mut();
    let target_key = if let Some(sub) = subkey {
        let wide_sub = win::wide(sub);
        if unsafe { RegOpenKeyExW(root, wide_sub.as_ptr(), 0, KEY_QUERY_VALUE, &mut opened) } != 0 {
            return None;
        }
        opened
    } else {
        root
    };
    let wide_val = value_name.map(win::wide);
    let val_ptr = wide_val.as_ref().map_or(ptr::null(), |v| v.as_ptr());
    let mut kind = REG_NONE;
    let mut size = 0u32;
    let rc = unsafe { RegQueryValueExW(target_key, val_ptr, ptr::null_mut(), &mut kind, ptr::null_mut(), &mut size) };
    let out = if rc == 0 && (kind == REG_SZ || kind == REG_EXPAND_SZ) && size >= 2 && size <= 4096 {
        let mut buf = vec![0u16; (size as usize).div_ceil(2)];
        if unsafe {
            RegQueryValueExW(
                target_key,
                val_ptr,
                ptr::null_mut(),
                &mut kind,
                buf.as_mut_ptr() as *mut u8,
                &mut size,
            )
        } == 0
        {
            Some(win::from_wide(&buf))
        } else {
            None
        }
    } else {
        let _ = RegEnumValueW;
        None
    };
    if !opened.is_null() {
        unsafe {
            RegCloseKey(opened);
        }
    }
    out
}

/// Opens a native Windows file chooser dialog filtered to `.exe` files.
pub fn browse_exe(owner: HWND, title: &str) -> Option<(String, String, String)> {
    const OFN_FILEMUSTEXIST: u32 = 0x0000_1000;
    const OFN_PATHMUSTEXIST: u32 = 0x0000_0800;
    const OFN_NOCHANGEDIR: u32 = 0x0000_0008;

    let mut file_buf = vec![0u16; 1024];
    let filter: Vec<u16> = "Executables (*.exe)\0*.exe\0All files (*.*)\0*.*\0\0"
        .encode_utf16()
        .collect();
    let title_w = win::wide(title);
    let mut ofn: OpenFileNameW = unsafe { mem::zeroed() };
    ofn.l_struct_size = mem::size_of::<OpenFileNameW>() as u32;
    ofn.hwnd_owner = owner;
    ofn.lpstr_filter = filter.as_ptr();
    ofn.n_filter_index = 1;
    ofn.lpstr_file = file_buf.as_mut_ptr();
    ofn.n_max_file = file_buf.len() as u32;
    ofn.lpstr_title = title_w.as_ptr();
    ofn.flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;

    if unsafe { GetOpenFileNameW(&mut ofn) } == 0 {
        return None;
    }
    let path = win::from_wide(&file_buf);
    if path.is_empty() {
        return None;
    }
    let stem = settings::normalize_process_name(&path);
    if stem.is_empty() {
        return None;
    }
    let name = Path::new(&path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| stem.clone());
    Some((name, stem, path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_struct_sizes_match_windows_displayconfig() {
        assert_eq!(mem::size_of::<DisplayConfigDeviceInfoHeader>(), 20);
        assert_eq!(mem::size_of::<DisplayConfigGetAdvancedColorInfo>(), 32);
        assert_eq!(mem::size_of::<DisplayConfigGetAdvancedColorInfo2>(), 36);
        assert_eq!(mem::size_of::<DisplayConfigSetAdvancedColorState>(), 24);
        assert_eq!(mem::size_of::<DisplayConfigSetHdrState>(), 24);
        assert_eq!(mem::size_of::<DisplayConfigPathInfo>(), 72);
        assert_eq!(mem::size_of::<DisplayConfigModeInfo>(), 64);
        assert_eq!(mem::size_of::<DisplayConfigSourceDeviceName>(), 84);
        assert_eq!(mem::size_of::<DisplayConfigTargetDeviceName>(), 420);
    }

    #[test]
    fn process_matching_and_steam_parsing() {
        let apps = vec![AutoProfileApp {
            name: "Cyberpunk 2077".into(),
            process: "cyberpunk2077".into(),
            path: None,
            profile: None,
            auto_hdr: true,
        }];
        assert!(match_monitored_app(r"D:\Games\Cyberpunk 2077\bin\x64\Cyberpunk2077.exe", &apps).is_some());
        assert!(match_monitored_app(r"C:\Windows\System32\cyberpunk2077.exe", &apps).is_none());

        let vdf = "\"libraryfolders\"\n{\n  \"0\" { \"path\" \"C:\\\\Program Files (x86)\\\\Steam\" }\n  \"1\" { \"path\" \"D:\\\\SteamLibrary\" }\n}";
        assert_eq!(
            parse_steam_library_paths(vdf),
            vec![r"C:\Program Files (x86)\Steam", r"D:\SteamLibrary"]
        );

        let acf = "\"AppState\"\n{\n  \"name\" \"Hades II\"\n  \"installdir\" \"Hades II\"\n}";
        assert_eq!(
            parse_appmanifest_acf(acf),
            Some(("Hades II".to_string(), "Hades II".to_string()))
        );
    }

    #[test]
    fn discovery_runs_and_hdr_query_does_not_panic() {
        let _ = discover_installed_apps();
        let _ = query_hdr(None);
    }
}
