//! Windows DDC/CI transport (`dxva2.dll`) and monitor discovery.
//!
//! The protocol itself — register routing, the profile table stream and the settings tables — is the same as on
//! Linux and lives in [`crate::monitor`]. Only the byte transport differs: `dxva2!GetVCPFeatureAndVCPFeatureReply` and
//! `dxva2!SetVCPFeature`, the calls the manufacturer's own Windows tool (VMW) uses.
//!
//! Two properties of `dxva2.dll` shape this module:
//!
//! * A request that never got a reply (NULL frame, busy monitor, periodic outage) is not reported as an error: the
//!   call succeeds with a zero reply. Reads therefore reject `00/00`, and the settle loop requires the MCCS version to
//!   come back, which an empty or stale reply cannot fake.
//! * The API returns the decoded maximum/current pair instead of the frame, so a stale reply cannot be recognised by
//!   its header. A reply identical to the one the previous request got for the same code in the other register is
//!   treated as stale and retried.

use std::{
    ffi::c_void,
    io, mem, ptr, thread,
    time::{Duration, Instant},
};

use windows_sys::Win32::Foundation::{GetLastError, HANDLE, HWND, LPARAM, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    EnumDisplayDevicesW, EnumDisplayMonitors, EnumDisplaySettingsW, GetMonitorInfoW, DEVMODEW, DISPLAY_DEVICEW,
    ENUM_CURRENT_SETTINGS, HDC, HMONITOR, MONITORINFOEXW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::MONITORINFOF_PRIMARY;

use crate::monitor::{FrameError, Link, VcpReply, REQUEST_GAP, VCP_VERSION};

/// Physical monitor handles owned by one `dxva2` enumeration.
#[repr(C)]
#[derive(Clone, Copy)]
struct PhysicalMonitor {
    handle: HANDLE,
    description: [u16; 128],
}

#[link(name = "dxva2")]
unsafe extern "system" {
    fn GetNumberOfPhysicalMonitorsFromHMONITOR(monitor: HMONITOR, count: *mut u32) -> i32;
    fn GetPhysicalMonitorsFromHMONITOR(monitor: HMONITOR, count: u32, monitors: *mut PhysicalMonitor) -> i32;
    fn DestroyPhysicalMonitors(count: u32, monitors: *const PhysicalMonitor) -> i32;
    fn GetVCPFeatureAndVCPFeatureReply(
        monitor: HANDLE,
        code: u8,
        kind: *mut u32,
        current: *mut u32,
        maximum: *mut u32,
    ) -> i32;
    fn SetVCPFeature(monitor: HANDLE, code: u8, value: u32) -> i32;


}

// Values from winerror.h, delivered through `GetLastError` by the calls above.
const DDCCI_VCP_NOT_SUPPORTED: u32 = 0xC026_2584;
const DDCCI_INVALID_DATA: u32 = 0xC026_2585;
const DDCCI_INVALID_MESSAGE_COMMAND: u32 = 0xC026_2589;
const DDCCI_INVALID_MESSAGE_LENGTH: u32 = 0xC026_258A;
const DDCCI_INVALID_MESSAGE_CHECKSUM: u32 = 0xC026_258B;
const I2C_ERROR_TRANSMITTING_DATA: u32 = 0xC026_2582;
const I2C_ERROR_RECEIVING_DATA: u32 = 0xC026_2583;

/// Busy or lost handle: `dxva2` cannot reach the monitor any more.
const MONITOR_NO_LONGER_EXISTS: u32 = 0xC026_2586;

// ───────────────────────────── monitor discovery ─────────────────────────────

/// A connected monitor as Windows reports it.
#[derive(Clone, Debug)]
pub struct DisplayMonitor {
    /// Model name from the EDID (`P275MV PLUS`), or the device name when the EDID cannot be read.
    pub name: String,
    /// Current mode, `WIDTHxHEIGHT`.
    pub resolution: String,

    /// Display device the monitor belongs to, `\\.\DISPLAY1`.
    pub connector: String,
    pub primary: bool,
    /// Handle of the monitor in `EnumDisplayMonitors`, used to reach its DDC/CI handles.
    monitor: HMONITOR,
}

/// Every monitor Windows currently drives, including ones without DDC/CI.
pub fn enumerate_monitors() -> Vec<DisplayMonitor> {
    let mut handles: Vec<HMONITOR> = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            ptr::null_mut::<c_void>() as HDC,
            ptr::null::<RECT>(),
            Some(collect_monitor),
            &mut handles as *mut Vec<HMONITOR> as LPARAM,
        );
    }

    let mut monitors = Vec::new();
    for monitor in handles {
        let mut info: MONITORINFOEXW = unsafe { mem::zeroed() };
        info.monitorInfo.cbSize = mem::size_of::<MONITORINFOEXW>() as u32;
        if unsafe { GetMonitorInfoW(monitor, &mut info.monitorInfo) } == 0 {
            continue;
        }
        let connector = crate::win::from_wide(&info.szDevice);
        let (resolution, device_id) = display_device(&connector);
        let edid = device_id.as_deref().and_then(read_edid);
        let name = match edid {
            Some((name, _serial)) => name,
            None => connector.clone(),
        };
        log!("[DISPLAY] {connector}: {name} [{resolution}] id={device_id:?}");
        monitors.push(DisplayMonitor {
            name,
            resolution,
            connector,
            primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
            monitor,
        });
    }
    monitors
}

unsafe extern "system" fn collect_monitor(monitor: HMONITOR, _hdc: HDC, _rect: *mut RECT, data: LPARAM) -> i32 {
    let list = unsafe { &mut *(data as *mut Vec<HMONITOR>) };
    list.push(monitor);
    1
}

/// Device id and current mode of a display device name (`\\.\DISPLAY1`).
fn display_device(connector: &str) -> (String, Option<String>) {
    let name = crate::win::wide(connector);
    unsafe {
        let mut device: DISPLAY_DEVICEW = mem::zeroed();
        device.cb = mem::size_of::<DISPLAY_DEVICEW>() as u32;
        // With the interface flag the device id of the monitor (`\\?\DISPLAY#...`) names the registry key that
        // holds the EDID.
        let device_id = if EnumDisplayDevicesW(name.as_ptr(), 0, &mut device, EDD_GET_DEVICE_INTERFACE_NAME) != 0 {
            Some(crate::win::from_wide(&device.DeviceID)).filter(|id| !id.is_empty())
        } else {
            None
        };

        let mut mode: DEVMODEW = mem::zeroed();
        mode.dmSize = mem::size_of::<DEVMODEW>() as u16;
        let resolution = if EnumDisplaySettingsW(name.as_ptr(), ENUM_CURRENT_SETTINGS, &mut mode) != 0 {
            format!("{}x{}", mode.dmPelsWidth, mode.dmPelsHeight)
        } else {
            String::new()
        };
        (resolution, device_id)
    }
}

/// `EDD_GET_DEVICE_INTERFACE_NAME` of `EnumDisplayDevices`.
const EDD_GET_DEVICE_INTERFACE_NAME: u32 = 1;

/// Reads the EDID blob of a monitor from the registry and decodes its name and serial number.
///
/// `device_id` is the device interface name of the monitor (`\\?\DISPLAY#LHC907D#5&d74d7d7&0&UID4355#{.}`).
fn read_edid(device_id: &str) -> Option<(String, String)> {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE, REG_NONE,
    };

    // `\\?\DISPLAY#LHC907D#5&d74d7d7&0&UID4355#{.}` ->
    // `SYSTEM\CurrentControlSet\Enum\DISPLAY\LHC907D\5&d74d7d7&0&UID4355\Device Parameters`.
    let trimmed = device_id.strip_prefix(r"\\?\")?;
    let mut parts = trimmed.split('#');
    let class = parts.next()?;
    let vendor = parts.next()?;
    let instance = parts.next()?;
    let path = format!("SYSTEM\\CurrentControlSet\\Enum\\{class}\\{vendor}\\{instance}\\Device Parameters");
    let key = crate::win::wide(&path);
    let value = crate::win::wide("EDID");

    unsafe {
        let mut handle: HKEY = ptr::null_mut();
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, key.as_ptr(), 0, KEY_QUERY_VALUE, &mut handle) != 0 {
            return None;
        }
        let mut size = 0u32;
        let mut kind = REG_NONE;
        let mut status =
            RegQueryValueExW(handle, value.as_ptr(), ptr::null_mut(), &mut kind, ptr::null_mut(), &mut size);
        let mut data = vec![0u8; size as usize];
        if status == 0 && size >= 128 {
            status = RegQueryValueExW(
                handle,
                value.as_ptr(),
                ptr::null_mut(),
                &mut kind,
                data.as_mut_ptr(),
                &mut size,
            );
        }
        RegCloseKey(handle);
        if status != 0 || size < 128 {
            return None;
        }
        let name = crate::monitor::parse_edid_name(&data)?;
        let serial = crate::monitor::parse_edid_serial(&data).unwrap_or_default();
        Some((name, serial))
    }
}

// ───────────────────────────── DDC/CI transport ─────────────────────────────

/// A DDC/CI connection to one physical monitor.
pub struct MsiDdc {
    /// Handles returned by `dxva2`; destroyed together with the connection.
    physical: Vec<PhysicalMonitor>,
    monitor: HANDLE,
    last_request: Option<Instant>,
    /// Set after a failed exchange: the next operation first waits for the monitor to answer again.
    needs_settle: bool,
    /// Reply of the previous request, used to recognise a stale answer.
    last_reply: Option<(u8, bool, u16, u16)>,
}

// The DDC/CI handle of one monitor may be used from any thread; the worker owns it alone.
unsafe impl Send for MsiDdc {}

impl Drop for MsiDdc {
    fn drop(&mut self) {
        if !self.physical.is_empty() {
            unsafe {
                DestroyPhysicalMonitors(self.physical.len() as u32, self.physical.as_ptr());
            }
        }
    }
}

impl MsiDdc {
    /// Opens the DDC/CI handles of a monitor.
    pub fn open(monitor: &DisplayMonitor) -> io::Result<Self> {
        let mut count = 0u32;
        unsafe {
            if GetNumberOfPhysicalMonitorsFromHMONITOR(monitor.monitor, &mut count) == 0 || count == 0 {
                return Err(io::Error::other(format!(
                    "{}: no physical monitors ({})",
                    monitor.connector,
                    crate::win::last_error()
                )));
            }
        }
        let mut physical =
            vec![PhysicalMonitor { handle: ptr::null_mut(), description: [0; 128] }; count as usize];
        unsafe {
            if GetPhysicalMonitorsFromHMONITOR(monitor.monitor, count, physical.as_mut_ptr()) == 0 {
                return Err(io::Error::other(format!(
                    "{}: cannot open the monitor ({})",
                    monitor.connector,
                    crate::win::last_error()
                )));
            }
        }
        let handle = physical[0].handle;
        if handle.is_null() {
            unsafe {
                DestroyPhysicalMonitors(count, physical.as_ptr());
            }
            return Err(io::Error::other(format!("{}: DDC/CI handle is null", monitor.connector)));
        }
        log!(
            "[DDC] {} ready: {} ({})",
            monitor.connector,
            monitor.name,
            crate::win::from_wide(&physical[0].description)
        );
        Ok(Self {
            physical,
            monitor: handle,
            last_request: None,
            needs_settle: true,
            last_reply: None,
        })
    }


}

impl Link for MsiDdc {
    fn pace(&mut self) {
        if let Some(last) = self.last_request
            && let Some(wait) = REQUEST_GAP.checked_sub(last.elapsed())
        {
            thread::sleep(wait);
        }
        self.last_request = Some(Instant::now());
    }

    fn write_frame(&mut self, code: u8, value: u16) -> Result<(), FrameError> {
        self.pace();
        let ok = unsafe { SetVCPFeature(self.monitor, code, value as u32) };
        if ok == 0 {
            return Err(FrameError::Io(io::Error::other(describe(unsafe { GetLastError() }))));
        }
        Ok(())
    }

    fn read_frame(&mut self, code: u8, vendor: bool) -> Result<VcpReply, FrameError> {
        self.pace();
        let mut kind = 0u32;
        let mut current = 0u32;
        let mut maximum = 0u32;
        let ok = unsafe { GetVCPFeatureAndVCPFeatureReply(self.monitor, code, &mut kind, &mut current, &mut maximum) };
        let last_reply = self.last_reply;
        if ok == 0 {
            let error = unsafe { GetLastError() };
            return Err(match error {
                DDCCI_VCP_NOT_SUPPORTED => FrameError::Unsupported,
                DDCCI_INVALID_MESSAGE_CHECKSUM
                | DDCCI_INVALID_MESSAGE_LENGTH
                | DDCCI_INVALID_MESSAGE_COMMAND
                | DDCCI_INVALID_DATA => FrameError::Garbage,
                I2C_ERROR_RECEIVING_DATA | I2C_ERROR_TRANSMITTING_DATA => FrameError::Null,
                MONITOR_NO_LONGER_EXISTS => FrameError::Io(io::Error::other(describe(error))),
                other => FrameError::Io(io::Error::other(describe(other))),
            });
        }
        let reply = VcpReply { maximum: maximum as u16, current: current as u16 };
        // A reply that never arrived reaches us as a zero pair (NULL frame of the monitor).
        if reply.maximum == 0 && reply.current == 0 {
            return Err(FrameError::Null);
        }
        // The same code answered this exact pair for the other register: the API repeated the previous reply.
        if let Some((last_code, last_vendor, last_maximum, last_current)) = last_reply
            && last_code == code
            && last_vendor != vendor
            && last_maximum == reply.maximum
            && last_current == reply.current
        {
            return Err(FrameError::Garbage);
        }
        self.last_reply = Some((code, vendor, reply.maximum, reply.current));
        Ok(reply)
    }

    /// Two valid `DF` replies in a row leave a wedged stream or a periodic outage. `DF` is read plain because a code
    /// `00–1E` right after `GET 99 -> GET 22` would start a profile table stream.
    fn settle(&mut self) -> bool {
        let mut good = 0;
        for _ in 0..30 {
            match self.read_frame(VCP_VERSION, false) {
                // VCP version 2.x: neither an empty nor a stale reply can produce this for `DF`.
                Ok(reply) if reply.current >> 8 == 0x02 => {
                    good += 1;
                    if good == 2 {
                        self.needs_settle = false;
                        return true;
                    }
                }
                _ => good = 0,
            }
        }
        false
    }

    fn needs_settle(&self) -> bool {
        self.needs_settle
    }

    fn set_needs_settle(&mut self, value: bool) {
        self.needs_settle = value;
    }
}

fn describe(error: u32) -> String {
    let text = match error {
        DDCCI_VCP_NOT_SUPPORTED => "unsupported VCP code",
        DDCCI_INVALID_DATA => "invalid reply data",
        DDCCI_INVALID_MESSAGE_COMMAND => "invalid reply command",
        DDCCI_INVALID_MESSAGE_LENGTH => "invalid reply length",
        DDCCI_INVALID_MESSAGE_CHECKSUM => "invalid reply checksum",
        I2C_ERROR_TRANSMITTING_DATA => "I2C transmit error",
        I2C_ERROR_RECEIVING_DATA => "I2C receive error",
        MONITOR_NO_LONGER_EXISTS => "monitor no longer exists",
        _ => "DDC/CI error",
    };
    format!("{text} (0x{error:08X})")
}

/// Keeps the duration import used by the pacing documentation above.
const _: Duration = REQUEST_GAP;
/// `HWND` is not used here; keep the import list honest for readers of the module.
type _UnusedHwnd = HWND;
