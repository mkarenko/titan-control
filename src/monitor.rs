mod kde;

use i2c_linux::I2c;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};

const DDC_ADDR: u16 = 0x37;
const DDC_DEST: u8 = 0x6E;
const DDC_SRC: u8 = 0x51;

pub const VCP_BRIGHTNESS: u8 = 0x10;
pub const VCP_CONTRAST: u8 = 0x12;
pub const VCP_COLOR_TEMP: u8 = 0x14;
pub const VCP_RED: u8 = 0x16;
pub const VCP_GREEN: u8 = 0x17;
pub const VCP_BLUE: u8 = 0x18;
pub const VCP_RESET_FACTORY: u8 = 0x04;
pub const VCP_RESET_BC: u8 = 0x05;
pub const VCP_RESET_COLOR: u8 = 0x08;
pub const VCP_GAMMA: u8 = 0x26;
pub const VCP_SHARPNESS: u8 = 0x87;
pub const VCP_OSD_LANG: u8 = 0xCC;
/// MCCS power mode (plain): 1 on, 3 turns the USB hub off, 5 turns the whole monitor off (2 and 4 untested).
pub const VCP_DPMS: u8 = 0xD6;
/// MCCS Input Source (plain register of 0x60): 0x0F DP-1, 0x10 DP-2, 0x11 HDMI-1, 0x12 HDMI-2.
pub const VCP_INPUT_SOURCE: u8 = 0x60;
pub const VCP_MODE: u8 = 0x22;
/// Low Blue Light 0-4 (manufacturer register).
pub const VCP_LOW_BLUE: u8 = 0xD8;

pub const VCP_USER1_RED: u8 = 0x16;
pub const VCP_USER1_GREEN: u8 = 0x17;
pub const VCP_USER1_BLUE: u8 = 0x18;
pub const VCP_USER2_RED: u8 = 0x19;
pub const VCP_USER2_GREEN: u8 = 0x1A;
pub const VCP_USER2_BLUE: u8 = 0x1B;
pub const VCP_USER3_RED: u8 = 0x1C;
pub const VCP_USER3_GREEN: u8 = 0x1D;
pub const VCP_USER3_BLUE: u8 = 0x1E;

/// Refresh rate counter 0/1 (manufacturer register; reported max 12 is wrong, plain register = Low Blue Light 0-100).
pub const VCP_FPS_COUNTER: u8 = 0x30;
/// Refresh rate counter position 0-3 (reported max 12 is wrong).
pub const VCP_FPS_POS: u8 = 0x31;
pub const VCP_CROSSHAIR_COLOR: u8 = 0x32;
pub const VCP_STOPWATCH_TIME: u8 = 0x33;
pub const VCP_CROSSHAIR_SHAPE: u8 = 0x34;
pub const VCP_STOPWATCH_POS: u8 = 0x35;
/// Game time 1 = 15, 2 = 30, 3 = 45, 4 = 60 min.
pub const VCP_GAME_TIME_VAL: u8 = 0x36;
pub const VCP_GAME_TIME_POS: u8 = 0x37;
pub const VCP_MAGNIFIER_POS: u8 = 0x38;
// Confirmed on P275MV PLUS: 0 = rear LEDs on, 1 = off.
pub const VCP_REAR_LED: u8 = 0x39;
/// Full Game: 0 = Wide, 1 = 25" (the monitor accepts only 0/1 over DDC).
pub const VCP_SCREEN_SIZE: u8 = 0x3A;
pub const VCP_ALIGNMENT: u8 = 0x3B;
pub const VCP_CROSSHAIR: u8 = 0x3D;
pub const VCP_STOPWATCH: u8 = 0x3E;
pub const VCP_GAME_TIME: u8 = 0x3F;

pub const VCP_COLOR_ENHANCE: u8 = 0x40;
pub const VCP_CR_ENHANCE: u8 = 0x41;
pub const VCP_SHADOW_BALANCE: u8 = 0x42;
pub const VCP_GAME_RUSH: u8 = 0x43;
pub const VCP_SUPER_RES: u8 = 0x44;
pub const VCP_NIGHT_VISION: u8 = 0x45;
pub const VCP_HALO_CONTROL: u8 = 0x46;
pub const VCP_LOCAL_DIMMING: u8 = 0x47;
pub const VCP_DYDS: u8 = 0x48;
pub const VCP_DYNAMIC_OD: u8 = 0x49;
pub const VCP_HDR: u8 = 0x4A;
pub const VCP_MAGNIFIER: u8 = 0x4B;
pub const VCP_MAGNIFIER_SIZE: u8 = 0x4C;
pub const VCP_MAGNIFIER_ZOOM: u8 = 0x4D;
pub const VCP_SATURATION_RED: u8 = 0x59;
pub const VCP_SATURATION_YELLOW: u8 = 0x5A;
pub const VCP_SATURATION_GREEN: u8 = 0x5B;
pub const VCP_SATURATION_CYAN: u8 = 0x5C;
pub const VCP_SATURATION_BLUE: u8 = 0x5D;
pub const VCP_SATURATION_MAGENTA: u8 = 0x5E;

/// Output Range lives in the manufacturer register of 0x60 (after `GET 99`), while the plain register of 0x60 is
/// the MCCS Input Source. Settings are keyed by code, so Output Range uses this key; [`route`] sends it as 0x60
/// after `99`. Never written on its own (a bare `SET 99` is harmless: it precedes every manufacturer write).
pub const VCP_OUTPUT_RANGE: u8 = 0x99;
pub const VCP_QUICK_BOOT: u8 = 0x61;
/// DCR 0/1 (manufacturer register; the plain register of 0xE1 is Low Blue Light 0-4).
pub const VCP_DCR: u8 = 0xE1;
/// Adaptive-Sync 0/1 (plain register only).
pub const VCP_ADAPTIVE_SYNC: u8 = 0xE2;
pub const VCP_HAWKEYE: u8 = 0x63;
pub const VCP_HAWKEYE_SIZE: u8 = 0x64;
pub const VCP_HAWKEYE_POS: u8 = 0x65;
pub const VCP_HAWKEYE_LEVEL: u8 = 0x66;
/// OSD show time 5-60 s (manufacturer register; the plain register of 0xC0 counts usage hours).
pub const VCP_OSD_TIME: u8 = 0xC0;
pub const VCP_OSD_H_POS: u8 = 0xC1;
pub const VCP_OSD_V_POS: u8 = 0xC2;
/// OSD transparency: OSD levels 0-5 = DDC 0-100 in steps of 20.
pub const VCP_OSD_TRANS: u8 = 0xC3;
/// Power LED: 1 off, 2 level 1, 3 level 2, 4 level 3.
pub const VCP_POWER_LED: u8 = 0xC5;
pub const VCP_POWER_SAVING: u8 = 0xC6;
pub const VCP_HUE_RED: u8 = 0x9B;
pub const VCP_HUE_YELLOW: u8 = 0x9C;
pub const VCP_HUE_GREEN: u8 = 0x9D;
pub const VCP_HUE_CYAN: u8 = 0x9E;
pub const VCP_HUE_BLUE: u8 = 0x9F;
pub const VCP_HUE_MAGENTA: u8 = 0xA0;
pub const VCP_VOLUME: u8 = 0xF6;
/// Audio mute (manufacturer register): 0 = sound, 1 = muted. The plain register of 0xF7 is part of the serial number.
pub const VCP_MUTE: u8 = 0xF7;

// Codes confirmed on the P275MV PLUS (docs/p275mv_plus.md); value ranges in the comments.
/// MCCS New control value: 2 = a setting was changed in the OSD; writing 1 clears it (and 0x52).
pub const VCP_NEW_CONTROL_VALUE: u8 = 0x02;
/// Aspect ratio (manufacturer register, max 6): 1 wide, 2 4:3, 3 1:1, 4 21:9, 5 auto.
pub const VCP_RATIO: u8 = 0x24;
/// PIP/PBP: 0 off, 1 PIP, 2 PBP 1:1, 3 PBP 2:1, 4 PBP 1:2.
pub const VCP_PIP_MODE: u8 = 0x50;
/// PIP sub-signal source: 0 DP, 3 USB-C, 4 HDMI-1, 5 HDMI-2 (low byte of `ff05/69xx`).
pub const VCP_PIP_SOURCE: u8 = 0x51;
/// Audio source: 0 auto, 3 USB-C, 4 HDMI-1, 5 HDMI-2 (low byte of `ff05/69xx`).
pub const VCP_AUDIO_SOURCE: u8 = 0x52;
/// PIP position: 0 top right, 1 top left, 2 bottom right, 3 bottom left.
pub const VCP_PIP_POSITION: u8 = 0x53;
/// PIP size: 0 small, 1 medium, 2 large.
pub const VCP_PIP_SIZE: u8 = 0x54;
/// Swap PIP inputs: 1 = swap.
pub const VCP_PIP_SWAP: u8 = 0x55;
/// Reset PIP settings: 1 = reset.
pub const VCP_PIP_RESET: u8 = 0x56;
/// Input selection: 0 auto, 1/2 DP, 3 USB-C, 5 HDMI-1, 6 HDMI-2 (low byte of `ff05/69xx`).
pub const VCP_INPUT_SELECT: u8 = 0x57;
/// USB hub follows: 0 USB-C, 1 USB-B.
pub const VCP_USB_HUB_SOURCE: u8 = 0x5F;
/// OSD lock: 0 off, 1 on.
pub const VCP_OSD_LOCK: u8 = 0xC4;
/// Front button lock (MCCS OSD/Button control, plain): 1 = buttons 2 and 3 locked, 2 = unlocked.
pub const VCP_BUTTON_LOCK: u8 = 0xCA;
/// Eyeshield reminder: 0 off, 1 = 30 min ... 8 = 4 h (reported max 2 is wrong; DDC writes only 0/1 confirmed).
pub const VCP_EYESHIELD_REMINDER: u8 = 0xD4;
/// USB power in sleep: 0 off, 1 on.
pub const VCP_USB_POWER_SLEEP: u8 = 0xD5;
/// MCCS Display Application (plain): 0 standard, 3 movie, 5 games; changes contrast and local dimming.
pub const VCP_DISPLAY_APPLICATION: u8 = 0xDC;
/// Second Adaptive-Sync switch (manufacturer register, 0/1).
pub const VCP_ADAPTIVE_SYNC_ALT: u8 = 0xE4;
/// LED effects (manufacturer registers). Color: 1 red, 2 green, 3 blue, 4 yellow, 5 purple, 6 cyan,
/// 7 colorful (only in the Breathe and Plain Water modes).
pub const VCP_LED_COLOR: u8 = 0xE5;
/// LED strength: 1 highest, 2 standard, 3 soft.
pub const VCP_LED_STRENGTH: u8 = 0xE6;
/// LED mode: 1 normal, 2 breathe, 3 flicker, 4 plain water, 5 star, 6 colorful pearls, 7 colorful water.
pub const VCP_LED_MODE: u8 = 0xE7;
/// LED front/rear color (only in mode 7 "colorful water"), values 1-6 as [`VCP_LED_COLOR`].
pub const VCP_LED_FRONT_COLOR: u8 = 0xE8;
pub const VCP_LED_REAR_COLOR: u8 = 0xE9;
/// Manufacturer factory reset: `SET 99 = 0` -> `SET C7 = 0` (value 1 does nothing).
pub const VCP_VENDOR_FACTORY_RESET: u8 = 0xC7;

const VCP_REFRESH_RATE: u8 = 0xAE;
const VCP_VERSION: u8 = 0xDF;
const VCP_USAGE_TIME: u8 = 0xF3;
const VCP_FIRMWARE: u8 = 0xFE;
const VCP_DDCCI_INIT: u8 = 0x99;
const PROFILE_TABLE_LEN: usize = 38;
/// Profile code reply plus nine FE fragments, four bytes each.
const PROFILE_TABLE_CHUNKS: usize = 10;
const VCP_RESEND: u8 = 0xFF;
/// Minimum start-to-start interval of requests (reads and writes); shorter gaps produce many lost replies.
const REQUEST_GAP: Duration = Duration::from_millis(100);
/// Write -> read delay of a GET request.
const REPLY_DELAY: Duration = Duration::from_millis(40);
/// Pause after a failed reply (as the manufacturer's program).
const RETRY_WAIT: Duration = Duration::from_millis(324);
const UNSUPPORTED_WAIT: Duration = Duration::from_millis(200);
/// Pause between a write and its read-back.
/// Measured on the monitor: a written value is returned by the very next read (plain ~145 ms, after 99 ~245 ms
/// including the read itself), so only a short pause is kept.
const VERIFY_DELAY: Duration = Duration::from_millis(50);
const READ_TRIES: usize = 4;
const TABLE_RESEND_WAIT: Duration = Duration::from_millis(150);
const TABLE_RESEND_TRIES: usize = 12;
const TABLE_BUDGET: Duration = Duration::from_secs(60);
/// Two consecutive identical reads are required before the table is trusted.
const TABLE_CONFIRMATIONS: usize = 2;

#[derive(Clone, Copy, Debug)]
pub struct VcpReply {
    pub maximum: u16,
    pub current: u16,
}

#[derive(Clone, Debug)]
pub struct ProfileTable {
    pub scene_id: u8,
    pub bytes: [u8; PROFILE_TABLE_LEN],
}

impl ProfileTable {
    pub fn settings(&self) -> HashMap<u8, u16> {
        let mut settings = HashMap::new();

        settings.insert(VCP_BRIGHTNESS, self.bytes[2] as u16);
        settings.insert(VCP_CONTRAST, self.bytes[3] as u16);
        settings.insert(VCP_SHARPNESS, self.bytes[4] as u16);
        settings.insert(VCP_COLOR_ENHANCE, self.bytes[5] as u16);
        settings.insert(VCP_CR_ENHANCE, self.bytes[6] as u16);
        settings.insert(VCP_SHADOW_BALANCE, self.bytes[7] as u16);
        settings.insert(VCP_COLOR_TEMP, self.bytes[8] as u16);
        settings.insert(VCP_USER1_RED, self.bytes[9] as u16);
        settings.insert(VCP_USER1_GREEN, self.bytes[10] as u16);
        settings.insert(VCP_USER1_BLUE, self.bytes[11] as u16);
        settings.insert(VCP_USER2_RED, self.bytes[12] as u16);
        settings.insert(VCP_USER2_GREEN, self.bytes[13] as u16);
        settings.insert(VCP_USER2_BLUE, self.bytes[14] as u16);
        settings.insert(VCP_USER3_RED, self.bytes[15] as u16);
        settings.insert(VCP_USER3_GREEN, self.bytes[16] as u16);
        settings.insert(VCP_USER3_BLUE, self.bytes[17] as u16);
        settings.insert(VCP_HUE_RED, self.bytes[18] as u16);
        settings.insert(VCP_HUE_GREEN, self.bytes[19] as u16);
        settings.insert(VCP_HUE_BLUE, self.bytes[20] as u16);
        settings.insert(VCP_HUE_YELLOW, self.bytes[21] as u16);
        settings.insert(VCP_HUE_CYAN, self.bytes[22] as u16);
        settings.insert(VCP_HUE_MAGENTA, self.bytes[23] as u16);
        settings.insert(VCP_SATURATION_RED, self.bytes[24] as u16);
        settings.insert(VCP_SATURATION_GREEN, self.bytes[25] as u16);
        settings.insert(VCP_SATURATION_BLUE, self.bytes[26] as u16);
        settings.insert(VCP_SATURATION_YELLOW, self.bytes[27] as u16);
        settings.insert(VCP_SATURATION_CYAN, self.bytes[28] as u16);
        settings.insert(VCP_SATURATION_MAGENTA, self.bytes[29] as u16);
        // The table keeps Low Blue Light as 0-100 in steps of 25; the setting (0xD8) is the level 0-4.
        settings.insert(VCP_LOW_BLUE, (self.bytes[30] as u16 + 12) / 25);
        settings.insert(VCP_HDR, self.bytes[31] as u16);
        settings.insert(VCP_GAMMA, self.bytes[33] as u16);
        settings.insert(VCP_SUPER_RES, self.bytes[34] as u16);
        settings.insert(VCP_NIGHT_VISION, self.bytes[35] as u16);
        settings.insert(VCP_DYNAMIC_OD, self.bytes[36] as u16);

        settings
    }
}

pub struct MsiDdc {
    i2c: I2c<File>,
    last_request: Option<Instant>,
    /// Set after a failed exchange: the next operation first waits for the monitor to answer again.
    needs_settle: bool,
}

/// Active picture profile as read from the monitor.
#[derive(Clone, Debug)]
pub enum ActiveProfile {
    /// Custom profile with its settings table.
    Custom(ProfileTable),
    /// Default profile (odd code): its settings are read one by one.
    Default(u8),
}

enum TableRead {
    Table(u8, [u8; PROFILE_TABLE_CHUNKS * 4]),
    Default(u8),
}

/// Custom profiles have even codes (Standard Custom = 2 ... DyDs/LD Custom = 0x1E), Default profiles odd ones.
pub fn is_custom_profile(scene_id: u8) -> bool {
    scene_id.is_multiple_of(2)
}

/// Outcome of a strict single DDC/CI reply read.
#[derive(Debug)]
enum FrameError {
    /// `6e 80 be`: the monitor is busy; a stream fragment was not consumed.
    Null,
    /// Bytes that are not a reply: the monitor did not answer in time or is in an outage.
    Garbage,
    /// Valid reply with status "unsupported" (sometimes transient).
    Unsupported,
    Io(io::Error),
}

impl FrameError {
    fn describe(&self) -> String {
        match self {
            FrameError::Null => "NULL reply".into(),
            FrameError::Garbage => "invalid reply".into(),
            FrameError::Unsupported => "unsupported".into(),
            FrameError::Io(error) => format!("I/O: {error}"),
        }
    }
}

/// Register a setting lives in (docs/p275mv_plus.md, "Dostęp do kodów").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Access {
    /// Manufacturer register: read `GET 99` -> `GET code`, write `SET 99 = v` -> `SET code = v`.
    Vendor,
    /// Codes 00-1E: read plain (after `GET 99` they select a profile table stream), written with `99` like VMW.
    Table,
    /// Standard MCCS register: plain read and plain write.
    Plain,
}

#[derive(Clone, Copy, Debug)]
struct Route {
    code: u8,
    access: Access,
    /// Read the value back after a write (actions such as resets and power modes cannot be verified).
    verify: bool,
}

impl Route {
    /// Value of a reply. Manufacturer registers such as 0x51/0x52/0x57 answer `ff05/69xx`: the low byte is the value.
    fn value(&self, reply: VcpReply) -> u16 {
        if self.access == Access::Vendor
            && reply.maximum >> 8 == 0xFF
            && reply.maximum != 0xFFFF
            && reply.current > 0xFF
        {
            reply.current & 0xFF
        } else {
            reply.current
        }
    }
}

/// Codes whose working register is the plain MCCS one, although some of them also answer after `GET 99`.
const PLAIN_CODES: &[u8] = &[
    0x02, // New control value
    0x39, // LED: only a plain SET changes it
    VCP_INPUT_SOURCE,
    0x62, // Volume (MCCS mirror of 0xF6)
    0x68, // OSD language (0 = English)
    0x69, // input signal active lines
    0x6C, 0x6E, 0x70, // video black level R/G/B
    0x8D, // Audio mute (MCCS mirror of 0xF7, inverted)
    0xAC, 0xAE, // signal frequencies
    0xB0, // save/restore settings
    0xC8, // controller type
    VCP_BUTTON_LOCK,
    VCP_OSD_LANG,
    VCP_DPMS,
    VCP_DISPLAY_APPLICATION,
    VCP_VERSION,
    VCP_ADAPTIVE_SYNC,
    VCP_USAGE_TIME,
    VCP_FIRMWARE,
];

/// MCCS restore commands: written plain with value 1 (0x04 factory, 0x05 brightness/contrast, 0x06 geometry,
/// 0x08 colors, 0x0A TV).
const RESET_CODES: [u8; 5] = [0x04, 0x05, 0x06, 0x08, 0x0A];

fn route(key: u8) -> Route {
    if key == VCP_OUTPUT_RANGE {
        return Route { code: VCP_INPUT_SOURCE, access: Access::Vendor, verify: true };
    }
    let access = if RESET_CODES.contains(&key) || PLAIN_CODES.contains(&key) {
        Access::Plain
    } else if key <= 0x1E {
        Access::Table
    } else {
        Access::Vendor
    };
    let verify = !RESET_CODES.contains(&key)
        && ![
            VCP_NEW_CONTROL_VALUE,
            0xB0,
            VCP_VENDOR_FACTORY_RESET,
            VCP_DPMS,
            VCP_PIP_SWAP,
            VCP_PIP_RESET,
        ]
        .contains(&key);
    Route { code: key, access, verify }
}

/// Result of reading a written setting back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Readback {
    Confirmed,
    /// The monitor reports another value (write lost, rejected or adjusted).
    Differs(u16),
    Unreadable,
}

/// Whether a write can be checked by reading the setting back (actions such as resets cannot).
pub fn is_verifiable(key: u8) -> bool {
    route(key).verify
}

/// Writes that are never sent: `0x22 = 0` and `0xF0` (values 1 and 2 turn the monitor off).
fn check_write(key: u8, value: u16) -> io::Result<()> {
    if (key == VCP_MODE && value == 0) || key == 0xF0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("VCP 0x{key:02X} = {value} is blocked"),
        ));
    }
    Ok(())
}


impl MsiDdc {
    pub fn open(path: &str) -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        let mut i2c = I2c::new(file);
        i2c.smbus_set_slave_address(DDC_ADDR, false)?;
        Ok(Self {
            i2c,
            last_request: None,
            needs_settle: true,
        })
    }

    /// Waits until `REQUEST_GAP` has passed since the previous request (reads and writes alike).
    fn pace(&mut self) {
        if let Some(last) = self.last_request
            && let Some(wait) = REQUEST_GAP.checked_sub(last.elapsed())
        {
            thread::sleep(wait);
        }
        self.last_request = Some(Instant::now());
    }

    /// One SET VCP frame: `51 84 03 code hi lo chk`.
    fn write_vcp_frame(&mut self, code: u8, value: u16) -> io::Result<()> {
        let vh = (value >> 8) as u8;
        let vl = (value & 0xFF) as u8;
        let chk = DDC_DEST ^ DDC_SRC ^ 0x84 ^ 0x03 ^ code ^ vh ^ vl;
        self.pace();
        self.i2c.write_all(&[DDC_SRC, 0x84, 0x03, code, vh, vl, chk])
    }

    /// Sends a write the way the manufacturer's program does, without checking the result.
    ///
    /// Manufacturer and table registers: `SET 99 = value`, then `SET code = value`; plain MCCS registers:
    /// `SET code = value`. An I/O error (usually a short outage of the monitor) is retried once.
    pub fn send_vcp(&mut self, key: u8, value: u16) -> io::Result<()> {
        check_write(key, value)?;
        let route = route(key);
        if let Err(error) = self.send_write(route, value) {
            log!("[DDC] VCP 0x{:02X} write failed: {error}, retrying", route.code);
            self.needs_settle = true;
            thread::sleep(RETRY_WAIT);
            return self.send_write(route, value);
        }
        Ok(())
    }

    /// Reads a written setting back (some time after the write, see `VERIFY_DELAY`).
    pub fn verify_vcp(&mut self, key: u8, value: u16) -> Readback {
        let route = route(key);
        match self.read_register(route, 3) {
            Ok(reply) => {
                let current = route.value(reply);
                if current == value { Readback::Confirmed } else { Readback::Differs(current) }
            }
            Err(error) => {
                log!("[DDC] VCP 0x{:02X} verify read failed: {}", route.code, error.describe());
                Readback::Unreadable
            }
        }
    }

    /// Writes a setting and waits for the read-back; a write whose read-back differs is sent once more.
    /// Returns `Some(actual)` when the monitor reports a different value afterwards (rejected or adjusted),
    /// `None` when the value is confirmed or cannot be read back. Used where the next step depends on the result.
    pub fn set_vcp(&mut self, key: u8, value: u16) -> io::Result<Option<u16>> {
        let mut actual = None;
        for attempt in 0..2 {
            self.send_vcp(key, value)?;
            if !is_verifiable(key) {
                return Ok(None);
            }
            thread::sleep(VERIFY_DELAY);
            match self.verify_vcp(key, value) {
                Readback::Confirmed | Readback::Unreadable => return Ok(None),
                Readback::Differs(current) => actual = Some(current),
            }
            if attempt == 0 {
                log!("[DDC] VCP 0x{key:02X} = {value} not confirmed (read {actual:?}), resending");
            }
        }
        Ok(actual)
    }

    fn send_write(&mut self, route: Route, value: u16) -> io::Result<()> {
        if self.needs_settle {
            self.settle_ddc();
        }
        if route.access != Access::Plain {
            self.write_vcp_frame(VCP_DDCCI_INIT, value)?;
        }
        self.write_vcp_frame(route.code, value)
    }

    /// Reads a setting from the register it lives in (see [`route`]), with validation and retries.
    pub fn get_vcp_reply(&mut self, key: u8) -> io::Result<VcpReply> {
        let route = route(key);
        self.read_register(route, READ_TRIES)
            .map_err(|error| io::Error::other(format!("VCP 0x{:02X}: {}", route.code, error.describe())))
    }

    pub fn get_vcp(&mut self, key: u8) -> io::Result<u16> {
        let route = route(key);
        self.get_vcp_reply(key).map(|reply| route.value(reply))
    }

    /// Plain register of 0x52 (MCCS Active control): code of the setting changed last. The register after
    /// `GET 99` is the audio source ([`VCP_AUDIO_SOURCE`]).
    fn get_active_control(&mut self) -> io::Result<u8> {
        let route = Route { code: 0x52, access: Access::Plain, verify: false };
        self.read_register(route, READ_TRIES)
            .map(|reply| reply.current as u8)
            .map_err(|error| io::Error::other(format!("VCP 0x52: {}", error.describe())))
    }

    fn read_register(&mut self, route: Route, tries: usize) -> Result<VcpReply, FrameError> {
        let mut last = FrameError::Garbage;
        for _ in 0..tries {
            if self.needs_settle {
                self.settle_ddc();
            }
            match self.read_register_once(route) {
                Ok(reply) => return Ok(reply),
                // "Unsupported" is sometimes transient; it does not disturb the bus.
                Err(FrameError::Unsupported) => {
                    last = FrameError::Unsupported;
                    thread::sleep(UNSUPPORTED_WAIT);
                }
                Err(error) => {
                    last = error;
                    self.needs_settle = true;
                    thread::sleep(RETRY_WAIT);
                }
            }
        }
        Err(last)
    }

    fn read_register_once(&mut self, route: Route) -> Result<VcpReply, FrameError> {
        if route.access != Access::Vendor {
            return self.get_vcp_frame(route.code);
        }
        let init = self.get_vcp_frame(VCP_DDCCI_INIT)?;
        if init.maximum != 0xFAFA || init.current != 0xFAFA {
            return Err(FrameError::Garbage);
        }
        let reply = self.get_vcp_frame(route.code)?;
        if route.code == VCP_MODE {
            // GET 99 -> GET 22 -> GET 00..1E would read a profile table: close the sequence.
            let _ = self.get_vcp_frame(VCP_VERSION);
        }
        Ok(reply)
    }

    /// One request/reply exchange with strict validation (header, checksum, code, status).
    fn get_vcp_frame(&mut self, code: u8) -> Result<VcpReply, FrameError> {
        let chk = DDC_DEST ^ DDC_SRC ^ 0x82 ^ 0x01 ^ code;
        self.pace();
        self.i2c
            .write_all(&[DDC_SRC, 0x82, 0x01, code, chk])
            .map_err(FrameError::Io)?;
        thread::sleep(REPLY_DELAY);

        let mut resp = [0u8; 11];
        self.i2c.read_exact(&mut resp).map_err(FrameError::Io)?;

        if resp[..3] == [0x6E, 0x80, 0xBE] {
            return Err(FrameError::Null);
        }
        let checksum = resp.iter().fold(0x50u8, |acc, byte| acc ^ byte);
        if resp[..3] != [0x6E, 0x88, 0x02] || checksum != 0 || resp[4] != code {
            return Err(FrameError::Garbage);
        }
        if resp[3] != 0 {
            return Err(FrameError::Unsupported);
        }
        Ok(VcpReply {
            maximum: ((resp[6] as u16) << 8) | resp[7] as u16,
            current: ((resp[8] as u16) << 8) | resp[9] as u16,
        })
    }

    /// Plain GET 0xDF until two consecutive valid replies. Leaves a wedged stream or an outage.
    /// DF (VCP version) is used because codes 00-1E right after GET 99 -> GET 22 start a table stream.
    fn settle_ddc(&mut self) -> bool {
        let mut good = 0;
        for _ in 0..30 {
            match self.get_vcp_frame(VCP_VERSION) {
                Ok(_) => {
                    good += 1;
                    if good == 2 {
                        self.needs_settle = false;
                        return true;
                    }
                }
                Err(_) => good = 0,
            }
        }
        false
    }

    /// Reads the active profile. A Custom profile comes with its table, returned only after
    /// `TABLE_CONFIRMATIONS` identical reads. The profile is the one the monitor reports in the GET 0x22 reply
    /// of the table sequence.
    pub fn read_active_profile(&mut self, confirmations: usize) -> io::Result<ActiveProfile> {
        let deadline = Instant::now() + TABLE_BUDGET;
        let mut previous: Option<(u8, [u8; PROFILE_TABLE_CHUNKS * 4])> = None;
        let mut matches = 1;
        let mut attempts = 0;
        let mut last_error = String::from("no attempt made");

        while Instant::now() < deadline {
            attempts += 1;
            match self.read_profile_table_once() {
                Ok(TableRead::Default(scene_id)) => return Ok(ActiveProfile::Default(scene_id)),
                Ok(TableRead::Table(scene_id, data)) if confirmations <= 1 || previous == Some((scene_id, data)) => {
                    matches += 1;
                    if matches >= confirmations {
                        let mut bytes = [0u8; PROFILE_TABLE_LEN];
                        for (slot, value) in bytes.iter_mut().skip(2).zip(data.iter()) {
                            *slot = *value;
                        }
                        return Ok(ActiveProfile::Custom(ProfileTable { scene_id, bytes }));
                    }
                }
                Ok(TableRead::Table(scene_id, data)) => {
                    previous = Some((scene_id, data));
                    matches = 1;
                }
                Err(error) => {
                    previous = None;
                    matches = 1;
                    last_error = error;
                }
            }
            thread::sleep(Duration::from_millis(500));
        }

        Err(io::Error::other(format!(
            "profile table read failed after {attempts} attempts: {last_error}"
        )))
    }

    /// GET 99 -> GET 22 -> GET profile -> GET FE ... -> 0000/5101, with recovery of lost replies:
    /// NULL leaves the fragment unread (repeat the request), garbage consumes it (FF resends it).
    /// Only Custom profiles (even codes) are read as tables: the manufacturer's program reads only those, and the
    /// stream of a Default profile has a different length (9 fragments for Standard Default) and an unknown layout.
    fn read_profile_table_once(&mut self) -> Result<TableRead, String> {
        let mut entered = false;
        for _ in 0..8 {
            if !self.settle_ddc() {
                continue;
            }
            match self.get_vcp_frame(VCP_DDCCI_INIT) {
                Ok(reply) if reply.maximum == 0xFAFA && reply.current == 0xFAFA => {
                    entered = true;
                    break;
                }
                _ => {}
            }
        }
        if !entered {
            return Err("GET 0x99 did not return FAFA/FAFA".into());
        }

        let mode = self
            .get_vcp_frame(VCP_MODE)
            .map_err(|error| format!("GET 0x22: {}", error.describe()))?;
        if mode.maximum != 0x0026 {
            return Err(format!("GET 0x22: unexpected maximum 0x{:04X}", mode.maximum));
        }

        let scene_id = u8::try_from(mode.current)
            .ok()
            .filter(|id| (1..=30).contains(id))
            .ok_or_else(|| format!("GET 0x22: unknown profile {}", mode.current))?;
        if !is_custom_profile(scene_id) {
            // Close the sequence: GET 00..1E right after GET 99 -> GET 22 would start a table stream.
            let _ = self.get_vcp_frame(VCP_VERSION);
            return Ok(TableRead::Default(scene_id));
        }

        let mut chunks: Vec<[u8; 4]> = Vec::with_capacity(PROFILE_TABLE_CHUNKS);
        let mut code = scene_id;
        loop {
            let reply = self.next_table_chunk(code, chunks.len())?;
            let chunk = [
                (reply.maximum >> 8) as u8,
                reply.maximum as u8,
                (reply.current >> 8) as u8,
                reply.current as u8,
            ];
            code = VCP_FIRMWARE;
            if !chunks.is_empty() && chunk == [0, 0, 0x51, 0x01] {
                break;
            }
            if chunks.len() >= PROFILE_TABLE_CHUNKS {
                return Err("table stream did not end".into());
            }
            chunks.push(chunk);
        }
        if chunks.len() != PROFILE_TABLE_CHUNKS {
            return Err(format!("table has {} fragments instead of {}", chunks.len(), PROFILE_TABLE_CHUNKS));
        }

        let mut data = [0u8; PROFILE_TABLE_CHUNKS * 4];
        for (index, chunk) in chunks.iter().enumerate() {
            data[index * 4..index * 4 + 4].copy_from_slice(chunk);
        }
        Ok(TableRead::Table(scene_id, data))
    }

    fn next_table_chunk(&mut self, code: u8, received: usize) -> Result<VcpReply, String> {
        let mut request = code;
        let mut error = match self.get_vcp_frame(request) {
            Ok(reply) => return Ok(reply),
            Err(error) => error,
        };
        for _ in 0..TABLE_RESEND_TRIES {
            thread::sleep(TABLE_RESEND_WAIT);
            if !matches!(error, FrameError::Null) {
                request = VCP_RESEND;
            }
            match self.get_vcp_frame(request) {
                // FF outside a stream answers 0000/0200: the lost reply was the 0000/5101 terminator.
                Ok(reply)
                    if request == VCP_RESEND
                        && reply.maximum == 0
                        && reply.current == 0x0200 =>
                {
                    if received == PROFILE_TABLE_CHUNKS {
                        return Ok(VcpReply { maximum: 0, current: 0x5101 });
                    }
                    return Err(format!("stream ended after {received} fragments"));
                }
                Ok(reply) => return Ok(reply),
                Err(next) => error = next,
            }
        }
        Err(format!("fragment {received}: {}", error.describe()))
    }
}

pub struct MonitorInfo {
    pub name: String,
    pub connector: String,
    pub resolution: String,
    pub hz: u16,
    pub firm: String,
    pub usage_mins: u16,
    pub settings: HashMap<u8, u16>,
}

#[derive(Clone, Debug, Default)]
pub struct SystemDisplayState {
    pub output: String,
    pub current_resolution: String,
    pub current_refresh: String,
    pub current_scale_percent: u16,
    pub available_resolutions: Vec<String>,
    pub refresh_rates: HashMap<String, Vec<String>>,
    pub scaling_options: Vec<u16>,
    /// HDR of the output in the compositor (`None` = not available or unknown).
    pub hdr: Option<bool>,
    /// Desktop number of the output (KDE: wallpaper per screen).
    pub screen_index: Option<usize>,
}

#[derive(Clone, Debug)]
struct DrmMonitorInfo {
    name: String,
    resolution: String,
    serial_number: String,
    connector: String,
}

pub enum WorkerCmd {
    Set(u8, u16),
    ReadUsageTime,
    /// Read the monitor info, the active profile and every setting again (refresh button).
    RefreshAll,
    /// Firmware 2025-09-20 keeps Adaptive-Sync on in DyDs profiles; older firmware needs it off first.
    DydsWithAdaptiveSync(bool),
}

pub enum UiCmd {
    MonitorFound(MonitorInfo),
    /// Values read from the monitor.
    Settings(HashMap<u8, u16>),
    /// The monitor did not accept a written value and keeps this one instead (always shown).
    Corrected(HashMap<u8, u16>),
    UsageTimeUpdated(u16),
    /// The settings of the (new) picture profile have been read: its controls can be used again.
    ProfileLoaded,
    /// Monitor info read again (refresh button); `None` = not answered this time.
    InfoRefreshed { hz: Option<u16>, usage_mins: Option<u16> },
    Progress(String),
    Error(String),
}

fn worker_cmd_code(cmd: &WorkerCmd) -> Option<u8> {
    match cmd {
        WorkerCmd::Set(code, _) => Some(*code),
        WorkerCmd::ReadUsageTime | WorkerCmd::RefreshAll | WorkerCmd::DydsWithAdaptiveSync(_) => None,
    }
}

fn collapse_worker_cmds(mut batch: Vec<WorkerCmd>) -> Vec<WorkerCmd> {
    let mut seen = HashSet::new();
    let mut collapsed = Vec::with_capacity(batch.len());

    while let Some(cmd) = batch.pop() {
        let Some(code) = worker_cmd_code(&cmd) else {
            collapsed.push(cmd);
            continue;
        };
        if seen.insert(code) {
            collapsed.push(cmd);
        }
    }

    collapsed.reverse();
    collapsed
}

pub const COLOR_TEMP_VALUES: [u16; 6] = [0x05, 0x06, 0x08, 0x0b, 0x0c, 0x0d];
pub const OUTPUT_RANGE_VALUES: [u16; 3] = [0, 1, 2];
pub const HDR_VALUES: [u16; 4] = [0, 1, 2, 3];
/// Gamma 1.8, 2.0, 2.2, 2.4, 2.6, S-Curve (manufacturer register of 0x26 and profile table byte 33).
pub const GAMMA_VALUES: [u16; 6] = [2, 4, 6, 8, 10, 12];
pub const NIGHT_VISION_VALUES: [u16; 5] = [0, 1, 2, 3, 4];
pub const DYNAMIC_OD_VALUES: [u16; 5] = [0, 1, 2, 3, 4];
/// Local Dimming Off, Low, Smooth, Medium, High (the monitor accepts only 2-6).
pub const LOCAL_DIMMING_VALUES: [u16; 5] = [2, 3, 4, 5, 6];
pub const DYDS_VALUES: [u16; 7] = [2, 3, 4, 5, 6, 7, 8];
pub const POWER_SAVING_VALUES: [u16; 4] = [0, 1, 2, 3];
pub const SATURATION_CODES: [u8; 6] = [
    VCP_SATURATION_RED,
    VCP_SATURATION_GREEN,
    VCP_SATURATION_BLUE,
    VCP_SATURATION_CYAN,
    VCP_SATURATION_MAGENTA,
    VCP_SATURATION_YELLOW,
];

/// Saturation (red, green, blue, cyan, magenta, yellow, as [`SATURATION_CODES`]) that Color Enhance sets itself: with
/// level 1-10 the six saturation axes follow it and are locked. Level 0 has no table (the axes are free).
/// Read from the monitor at each level, 2026-10-02.
pub fn color_enhance_saturation(level: u16) -> Option<[u16; 6]> {
    Some(match level {
        1 => [53, 53, 53, 52, 52, 52],
        2 => [57, 57, 56, 53, 53, 53],
        3 => [58, 58, 58, 55, 55, 54],
        4 => [59, 59, 60, 56, 56, 55],
        5 => [61, 61, 62, 57, 57, 57],
        6 => [63, 63, 64, 59, 58, 58],
        7 => [65, 66, 67, 61, 60, 61],
        8 => [68, 69, 70, 63, 62, 63],
        9 => [71, 73, 74, 65, 64, 65],
        10 => [74, 76, 77, 67, 66, 67],
        _ => return None,
    })
}

pub const PICTURE_MODE_NAMES: [&str; 15] = [
    "Standard",
    "DyDs/ULL FPS",
    "DyDs/LD",
    "RTS Mode",
    "FPS Mode",
    "MOBA Mode",
    "Movie Mode",
    "Reading Mode",
    "Night Mode",
    "Eye Care Mode",
    "Mac View Mode",
    "E-Book Mode",
    "sRGB Mode",
    "AdobeRGB Mode",
    "DCI-P3 Mode",
];
/// Values of 0x22 per profile (order of [`PICTURE_MODE_NAMES`]), confirmed with Frida and on Linux
/// (`22 = 1` selects Standard Default). Never write 0.
pub const PICTURE_MODE_DEFAULT_VALUES: [u16; 15] = [
    0x01, 0x1B, 0x1D, 0x03, 0x05, 0x07, 0x09, 0x0B, 0x0D, 0x0F, 0x11, 0x13, 0x15, 0x17, 0x19,
];
pub const PICTURE_MODE_CUSTOM_VALUES: [Option<u16>; 15] = [
    Some(0x02),
    Some(0x1C),
    Some(0x1E),
    Some(0x04),
    Some(0x06),
    Some(0x08),
    Some(0x0A),
    Some(0x0C),
    Some(0x0E),
    Some(0x10),
    Some(0x12),
    Some(0x14),
    Some(0x16),
    Some(0x18),
    Some(0x1A),
];

// Value lists of features that the UI does not show yet (docs/p275mv_plus.md). Order = order of the OSD options.
#[allow(dead_code)]
pub mod values {
    /// LED color (0xE5): red, green, blue, yellow, purple, cyan, colorful (7: Breathe and Plain Water only).
    pub const LED_COLOR: [u16; 7] = [1, 2, 3, 4, 5, 6, 7];
    /// LED front/rear colors (0xE8, 0xE9): red, green, blue, yellow, purple, cyan.
    pub const LED_SIDE_COLOR: [u16; 6] = [1, 2, 3, 4, 5, 6];
    pub const LED_COLOR_COLORFUL: u16 = 7;
    /// LED modes that offer the "colorful" color: Breathe, Plain Water.
    pub const LED_MODES_WITH_COLORFUL: [u16; 2] = [2, 4];
    /// LED strength (0xE6): highest, standard, soft.
    pub const LED_STRENGTH: [u16; 3] = [1, 2, 3];
    /// LED mode (0xE7): normal, breathe, flicker, plain water, star, colorful pearls, colorful water.
    pub const LED_MODE: [u16; 7] = [1, 2, 3, 4, 5, 6, 7];
    /// LED mode in which front and rear colors are set separately (0xE8/0xE9).
    pub const LED_MODE_COLORFUL_WATER: u16 = 7;
    /// Rear LED (0x39): 0 = on, 1 = off.
    pub const REAR_LED_ON: u16 = 0;
    pub const REAR_LED_OFF: u16 = 1;
    /// Power LED (0xC5): off, level 1, level 2, level 3.
    pub const POWER_LED: [u16; 4] = [1, 2, 3, 4];

    /// PIP/PBP mode (0x50): off, PIP, PBP 1:1, PBP 2:1, PBP 1:2.
    pub const PIP_MODE: [u16; 5] = [0, 1, 2, 3, 4];
    pub const PIP_OFF: u16 = 0;
    /// PIP sub-signal source (0x51): DP, USB-C, HDMI-1, HDMI-2.
    pub const PIP_SOURCE: [u16; 4] = [0, 3, 4, 5];
    /// PIP position (0x53): top right, top left, bottom right, bottom left.
    pub const PIP_POSITION: [u16; 4] = [0, 1, 2, 3];
    /// PIP size (0x54): small, medium, large.
    pub const PIP_SIZE: [u16; 3] = [0, 1, 2];
    /// Audio source (0x52): auto, USB-C, HDMI-1, HDMI-2 (only connected inputs are accepted).
    pub const AUDIO_SOURCE: [u16; 4] = [0, 3, 4, 5];
    /// Input selection (0x57): auto, DP, DP, USB-C, HDMI-1, HDMI-2.
    pub const INPUT_SELECT: [u16; 6] = [0, 1, 2, 3, 5, 6];
    /// USB hub follows (0x5F): USB-C, USB-B.
    pub const USB_HUB_SOURCE: [u16; 2] = [0, 1];

    /// Button lock (0xCA): buttons 2 and 3 locked / unlocked.
    pub const BUTTONS_LOCKED: u16 = 1;
    pub const BUTTONS_UNLOCKED: u16 = 2;
    /// Eyeshield reminder (0xD4): off, 30 min, 1 h, 1.5 h, 2 h, 2.5 h, 3 h, 3.5 h, 4 h.
    /// All levels are read correctly; writes over DDC were confirmed only for 0 and 1.
    pub const EYESHIELD_REMINDER: [u16; 9] = [0, 1, 2, 3, 4, 5, 6, 7, 8];

    /// Aspect ratio (0x24): wide, 4:3, 1:1, 21:9, auto. 1, 2, 3 and 5 checked in the OSD (2026-10-02); 21:9 = 4 is
    /// the remaining value (0 is not accepted).
    pub const RATIO: [u16; 5] = [1, 2, 3, 4, 5];
    pub const RATIO_AUTO: u16 = 5;
    /// Power mode (0xD6): 1 = on, 5 = monitor off (only the power button turns it on again).
    pub const POWER_ON: u16 = 1;
    pub const POWER_OFF: u16 = 5;
}

/// Settings the monitor turns off while PIP/PBP is active (OSD greys them out).
pub const PIP_DISABLED_CODES: &[u8] = &[
    VCP_DCR, VCP_HDR, VCP_ADAPTIVE_SYNC, VCP_ADAPTIVE_SYNC_ALT, VCP_DYDS, VCP_DYNAMIC_OD, VCP_GAME_RUSH,
    VCP_HALO_CONTROL, VCP_SCREEN_SIZE, VCP_FPS_COUNTER, VCP_CROSSHAIR, VCP_STOPWATCH, VCP_GAME_TIME,
    VCP_MAGNIFIER, VCP_ALIGNMENT, VCP_HAWKEYE,
];

/// Whether a setting can be changed with the given monitor state (`settings` as sent in `UiCmd::Settings`).
#[allow(dead_code)]
pub fn is_setting_available(settings: &HashMap<u8, u16>, code: u8) -> bool {
    let pip_on = settings.get(&VCP_PIP_MODE).is_some_and(|&mode| mode != values::PIP_OFF);
    if pip_on && PIP_DISABLED_CODES.contains(&code) {
        return false;
    }
    match code {
        VCP_PIP_SOURCE | VCP_PIP_POSITION | VCP_PIP_SIZE | VCP_PIP_SWAP => pip_on,
        VCP_LED_FRONT_COLOR | VCP_LED_REAR_COLOR => {
            settings.get(&VCP_LED_MODE) == Some(&values::LED_MODE_COLORFUL_WATER)
        }
        // "Colorful pearls" locks the other LED settings.
        VCP_LED_COLOR | VCP_LED_STRENGTH => !matches!(settings.get(&VCP_LED_MODE), Some(6)),
        _ => true,
    }
}

/// Whether the "colorful" LED color (7) can be chosen in the given LED mode (Breathe, Plain Water).
#[allow(dead_code)]
pub fn led_mode_has_colorful(mode: Option<u16>) -> bool {
    mode.is_some_and(|mode| values::LED_MODES_WITH_COLORFUL.contains(&mode))
}


pub fn picture_mode_vcp_value(index: usize, custom: bool) -> Option<u16> {
    if custom {
        PICTURE_MODE_CUSTOM_VALUES.get(index).copied().flatten()
    } else {
        PICTURE_MODE_DEFAULT_VALUES.get(index).copied()
    }
}


pub fn picture_mode_from_vcp(value: u16) -> Option<(usize, bool)> {
    if let Some(index) = PICTURE_MODE_DEFAULT_VALUES
        .iter()
        .position(|&mapped| mapped == value)
    {
        return Some((index, false));
    }
    PICTURE_MODE_CUSTOM_VALUES
        .iter()
        .position(|&mapped| mapped == Some(value))
        .map(|index| (index, true))
}

pub fn builtin_color_temp_rgb(value: u16) -> Option<(u16, u16, u16)> {
    match value {
        0x05 => Some((48, 50, 47)),
        0x06 => Some((48, 50, 48)),
        0x08 => Some((8, 47, 50)),
        _ => None,
    }
}

pub fn is_user_color_temp(value: u16) -> bool {
    (0x0B..=0x0D).contains(&value)
}

pub fn color_temp_rgb_codes(value: u16) -> Option<(u8, u8, u8)> {
    match value {
        0x0B => Some((VCP_USER1_RED, VCP_USER1_GREEN, VCP_USER1_BLUE)),
        0x0C => Some((VCP_USER2_RED, VCP_USER2_GREEN, VCP_USER2_BLUE)),
        0x0D => Some((VCP_USER3_RED, VCP_USER3_GREEN, VCP_USER3_BLUE)),
        _ => None,
    }
}

pub fn cache_rgb_key_for_vcp_code(code: u8) -> Option<u8> {
    match code {
        VCP_USER1_RED | VCP_USER2_RED | VCP_USER3_RED => Some(VCP_RED),
        VCP_USER1_GREEN | VCP_USER2_GREEN | VCP_USER3_GREEN => Some(VCP_GREEN),
        VCP_USER1_BLUE | VCP_USER2_BLUE | VCP_USER3_BLUE => Some(VCP_BLUE),
        _ => None,
    }
}

pub fn is_color_temp_rgb_code(code: u8) -> bool {
    cache_rgb_key_for_vcp_code(code).is_some()
}

fn factory_reset_defaults() -> HashMap<u8, u16> {
    HashMap::from([
        (VCP_MODE, PICTURE_MODE_DEFAULT_VALUES[0]),
        // Values measured after a factory reset (brightness 90, contrast 50).
        (VCP_BRIGHTNESS, 90),
        (VCP_CONTRAST, 50),
        (VCP_SHARPNESS, 0),
        (VCP_COLOR_TEMP, 0x05),
        (VCP_RED, 48),
        (VCP_GREEN, 50),
        (VCP_BLUE, 47),
        (VCP_LOW_BLUE, 0),
        (VCP_GAMMA, GAMMA_VALUES[2]),
        (VCP_SHADOW_BALANCE, 50),
        (VCP_CR_ENHANCE, 0),
        (VCP_COLOR_ENHANCE, 0),
        (VCP_SUPER_RES, 0),
        (VCP_HUE_RED, 50),
        (VCP_HUE_GREEN, 50),
        (VCP_HUE_BLUE, 50),
        (VCP_HUE_CYAN, 50),
        (VCP_HUE_MAGENTA, 50),
        (VCP_HUE_YELLOW, 50),
        (VCP_SATURATION_RED, 50),
        (VCP_SATURATION_GREEN, 50),
        (VCP_SATURATION_BLUE, 50),
        (VCP_SATURATION_CYAN, 50),
        (VCP_SATURATION_MAGENTA, 50),
        (VCP_SATURATION_YELLOW, 50),
        (VCP_HALO_CONTROL, 0),
        (VCP_HDR, 0),
        (VCP_DCR, 0),
        (VCP_NIGHT_VISION, 0),
        (VCP_DYNAMIC_OD, 0),
        (VCP_ADAPTIVE_SYNC, 0),
        (VCP_GAME_RUSH, 1),
        (VCP_LOCAL_DIMMING, LOCAL_DIMMING_VALUES[4]),
        (VCP_DYDS, DYDS_VALUES[0]),
        (VCP_FPS_COUNTER, 0),
        (VCP_CROSSHAIR, 1),
        (VCP_STOPWATCH, 0),
        (VCP_GAME_TIME, 0),
        (VCP_MAGNIFIER, 0),
        (VCP_HAWKEYE, 0),
        (VCP_CROSSHAIR_COLOR, 8),
        (VCP_VOLUME, 50),
        (VCP_MUTE, 0),
        (VCP_OUTPUT_RANGE, OUTPUT_RANGE_VALUES[0]),
        (VCP_QUICK_BOOT, 0),
        (VCP_OSD_LANG, 0x02),
        (VCP_OSD_TIME, 10),
        (VCP_OSD_H_POS, 50),
        (VCP_OSD_V_POS, 50),
        (VCP_OSD_TRANS, 0),
        (VCP_POWER_SAVING, POWER_SAVING_VALUES[0]),
        (VCP_POWER_LED, 3),
        (VCP_REAR_LED, 0),
    ])
}

const EDID_HEADER: [u8; 8] = [0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00];

fn parse_edid_name(data: &[u8]) -> Option<String> {
    if data.len() < 128 || data[0..8] != EDID_HEADER {
        return None;
    }
    let mfg = format!(
        "{}{}{}",
        (((data[8] >> 2) & 0x1F) + 64) as char,
        ((((data[8] & 0x03) << 3) | ((data[9] >> 5) & 0x07)) + 64) as char,
        ((data[9] & 0x1F) + 64) as char
    );
    for b in 0..4 {
        let o = 54 + (b * 18);
        if data[o..o + 4] == [0, 0, 0, 0xFC] {
            let n: String = data[o + 5..o + 18]
                .iter()
                .filter(|&&c| (32..=126).contains(&c))
                .map(|&c| c as char)
                .collect();
            return Some(normalize_monitor_name(&format!("{} {}", mfg, n.trim())));
        }
    }
    Some(normalize_monitor_name(&mfg))
}

fn parse_edid_serial(data: &[u8]) -> Option<String> {
    if data.len() < 128 || data[0..8] != EDID_HEADER {
        return None;
    }
    for b in 0..4 {
        let o = 54 + (b * 18);
        if data[o..o + 4] == [0, 0, 0, 0xFF] {
            let serial: String = data[o + 5..o + 18]
                .iter()
                .filter(|&&c| (32..=126).contains(&c))
                .map(|&c| c as char)
                .collect();
            let serial = serial.trim().to_string();
            if !serial.is_empty() {
                return Some(serial);
            }
        }
    }
    None
}

fn normalize_monitor_name(name: &str) -> String {
    let trimmed = name.trim();
    trimmed
        .strip_prefix("LHC ")
        .or_else(|| trimmed.strip_prefix("LHC"))
        .unwrap_or(trimmed)
        .trim()
        .to_string()
}

fn get_edid_name(path: &str) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut i2c = I2c::new(file);
    i2c.smbus_set_slave_address(0x50, false).ok()?;
    let mut data = [0u8; 128];
    i2c.read_exact(&mut data).ok()?;
    parse_edid_name(&data[..])
}

fn read_drm_resolution(path: &Path) -> String {
    for file_name in ["mode", "modes"] {
        if let Ok(contents) = fs::read_to_string(path.join(file_name))
            && let Some(mode) = contents.lines().find(|line| !line.trim().is_empty())
        {
            return mode.trim().to_string();
        }
    }
    String::new()
}

fn current_session_is_kde_wayland() -> bool {
    std::env::var("XDG_SESSION_TYPE").unwrap_or_default().eq_ignore_ascii_case("wayland")
        && std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default()
            .split(':').any(|desktop| desktop.eq_ignore_ascii_case("KDE"))
}

fn current_session_is_cosmic_wayland() -> bool {
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    session_type.eq_ignore_ascii_case("wayland") && desktop.to_ascii_uppercase().contains("COSMIC")
}

fn is_internal_output(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    upper.starts_with("EDP") || upper.starts_with("LVDS") || upper.starts_with("DSI")
}

fn strip_ansi_sequences(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if matches!(chars.peek(), Some('[')) {
                chars.next();
                for next in chars.by_ref() {
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
                continue;
            }
            continue;
        }

        output.push(ch);
    }

    output
}

#[derive(Default)]
struct CosmicOutputState {
    name: String,
    enabled: bool,
    scale_percent: u16,
    modes: Vec<(String, String, bool)>,
}

fn parse_cosmic_randr_list(text: &str) -> Vec<CosmicOutputState> {
    let mut outputs = Vec::new();
    let mut current: Option<CosmicOutputState> = None;

    for raw_line in text.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let is_output_header = !raw_line.starts_with(' ')
            && (trimmed.contains("(enabled)") || trimmed.contains("(disabled)"));

        if is_output_header {
            if let Some(output) = current.take() {
                outputs.push(output);
            }

            let name = trimmed
                .split_once(" (")
                .map(|(name, _)| name)
                .unwrap_or(trimmed)
                .trim();
            let enabled = trimmed.contains("(enabled)");
            current = Some(CosmicOutputState {
                name: name.to_string(),
                enabled,
                scale_percent: 100,
                modes: Vec::new(),
            });
            continue;
        }

        if let Some(output) = current.as_mut()
            && let Some(scale_value) = trimmed.strip_prefix("Scale: ")
        {
            if let Ok(scale_percent) = scale_value.trim_end_matches('%').trim().parse::<u16>() {
                output.scale_percent = scale_percent.clamp(100, 300);
            }
            continue;
        }

        if raw_line.starts_with("    ")
            && trimmed.contains(" @ ")
            && let Some(output) = current.as_mut()
            && let Some((resolution, rest)) = trimmed.split_once(" @ ")
        {
            let refresh = rest
                .split_whitespace()
                .next()
                .unwrap_or("60.000")
                .to_string();
            let is_current = trimmed.contains("(current)");
            output
                .modes
                .push((resolution.to_string(), refresh, is_current));
        }
    }

    if let Some(output) = current.take() {
        outputs.push(output);
    }

    outputs
}

pub fn query_system_display_state(preferred_output: Option<&str>) -> io::Result<SystemDisplayState> {
    // sysfs reports card1-DP-3; compositors use DP-3.
    let preferred_output = preferred_output.map(|name| {
        match name.split_once('-') {
            Some((card, connector)) if card.strip_prefix("card")
                .is_some_and(|index| !index.is_empty() && index.bytes().all(|c| c.is_ascii_digit())) => connector,
            _ => name,
        }
    });
    if current_session_is_kde_wayland() {
        return kde::query(preferred_output);
    }
    if !current_session_is_cosmic_wayland() {
        return Err(io::Error::other(
            "System display controls are currently supported on KDE Wayland and COSMIC Wayland.",
        ));
    }

    let output = Command::new("cosmic-randr")
        .arg("list")
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::other(stderr.trim().to_string()));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    log!("[COSMIC-RANDR] raw list output:\n{text}");
    let plain_text = strip_ansi_sequences(&text);
    log!("[COSMIC-RANDR] sanitized list output:\n{plain_text}");
    let outputs = parse_cosmic_randr_list(&plain_text);
    log!(
        "[COSMIC-RANDR] parsed outputs: {:?}",
        outputs
            .iter()
            .map(|output| format!(
                "{} enabled={} modes={}",
                output.name,
                output.enabled,
                output.modes.len()
            ))
            .collect::<Vec<_>>()
    );
    let selected = preferred_output
        .and_then(|name| outputs.iter().find(|output| output.name == name && output.enabled))
        .or_else(|| {
            outputs
                .iter()
                .find(|output| output.enabled && !is_internal_output(&output.name))
        })
        .or_else(|| outputs.iter().find(|output| output.enabled))
        .ok_or_else(|| {
            io::Error::other(format!(
                "No enabled display outputs were found. Parsed {} outputs.",
                outputs.len()
            ))
        })?;

    let mut available_resolutions = Vec::new();
    let mut refresh_rates: HashMap<String, Vec<String>> = HashMap::new();
    let mut current_resolution = String::new();
    let mut current_refresh = String::new();

    for (resolution, refresh, is_current) in &selected.modes {
        if !available_resolutions.contains(resolution) {
            available_resolutions.push(resolution.clone());
        }
        let rates = refresh_rates.entry(resolution.clone()).or_default();
        if !rates.contains(refresh) {
            rates.push(refresh.clone());
        }
        if *is_current {
            current_resolution = resolution.clone();
            current_refresh = refresh.clone();
        }
    }

    if current_resolution.is_empty()
        && let Some((resolution, refresh, _)) = selected.modes.first()
    {
        current_resolution = resolution.clone();
        current_refresh = refresh.clone();
    }

    Ok(SystemDisplayState {
        output: selected.name.clone(),
        current_resolution,
        current_refresh,
        current_scale_percent: selected.scale_percent.max(100),
        available_resolutions,
        refresh_rates,
        scaling_options: vec![100, 125, 150, 175, 200],
        hdr: None,
        screen_index: None,
    })
}

/// Turns HDR of the output on or off in the compositor (KDE Wayland only).
pub fn set_system_hdr(output_name: &str, enable: bool) -> io::Result<()> {
    if current_session_is_kde_wayland() {
        return kde::set_hdr(output_name, enable);
    }
    Err(io::Error::other("System HDR is currently supported on KDE Wayland."))
}

/// Desktop wallpaper of the output (KDE Wayland only).
pub fn system_wallpaper(state: &SystemDisplayState) -> Option<std::path::PathBuf> {
    if current_session_is_kde_wayland() {
        return kde::wallpaper(state.screen_index?);
    }
    None
}

pub fn set_system_display_mode(
    output_name: &str,
    resolution: &str,
    refresh: Option<&str>,
    scale_percent: Option<u16>,
) -> io::Result<()> {
    if current_session_is_kde_wayland() {
        return kde::apply(output_name, resolution, refresh, scale_percent);
    }
    if !current_session_is_cosmic_wayland() {
        return Err(io::Error::other(
            "System display controls are currently supported on KDE Wayland and COSMIC Wayland.",
        ));
    }

    let (width, height) = resolution
        .split_once('x')
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid resolution format"))?;

    let mut command = Command::new("cosmic-randr");
    command.args(["mode", output_name, width, height]);

    if let Some(refresh) = refresh {
        command.args(["--refresh", refresh]);
    }

    if let Some(scale_percent) = scale_percent {
        let scale_value = format!("{:.2}", (scale_percent as f64) / 100.0);
        command.args(["--scale", &scale_value]);
    }

    log!(
        "[COSMIC-RANDR] mode output={} resolution={} refresh={:?} scale={:?}",
        output_name, resolution, refresh, scale_percent
    );
    let output = command.output()?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log!(
            "[COSMIC-RANDR] failed output={} resolution={} refresh={:?} scale={:?}: {}",
            output_name,
            resolution,
            refresh,
            scale_percent,
            stderr.trim()
        );
        Err(io::Error::other(stderr.trim().to_string()))
    }
}

fn get_drm_monitors() -> Vec<DrmMonitorInfo> {
    let mut monitors = Vec::new();
    let drm_dir = Path::new("/sys/class/drm");
    let entries = match fs::read_dir(drm_dir) {
        Ok(e) => e,
        Err(_) => return monitors,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let dir_name = entry.file_name();
        let dir_str = dir_name.to_string_lossy();
        if !dir_str.contains('-') || dir_str.starts_with("render") {
            continue;
        }
        let status_path = path.join("status");
        if let Ok(status) = fs::read_to_string(&status_path) {
            if status.trim() != "connected" {
                continue;
            }
        } else {
            continue;
        }
        let edid_path = path.join("edid");
        let mut edid_data = Vec::new();
        if let Ok(mut f) = File::open(&edid_path)
            && f.read_to_end(&mut edid_data).is_ok()
            && edid_data.len() >= 128
            && let Some(name) = parse_edid_name(&edid_data)
        {
            let resolution = read_drm_resolution(&path);
            let serial_number = parse_edid_serial(&edid_data).unwrap_or_default();
            log!("[DRM] {} connected: {} [{}]", dir_str, name, resolution);
            monitors.push(DrmMonitorInfo {
                name,
                resolution,
                serial_number,
                connector: dir_str.to_string(),
            });
        }
    }
    monitors
}

fn get_edid_name_retry(path: &str, attempts: u8) -> Option<String> {
    for attempt in 0..attempts {
        if let Some(name) = get_edid_name(path) {
            return Some(name);
        }
        if attempt + 1 < attempts {
            thread::sleep(Duration::from_millis(150));
        }
    }
    None
}

/// Reads one setting (validated, with retries inside [`MsiDdc::get_vcp`]); `None` when it cannot be read.
fn read_vcp(ddc: &mut MsiDdc, key: u8) -> Option<u16> {
    match ddc.get_vcp(key) {
        Ok(value) => Some(value),
        Err(error) => {
            log!("[DDC] {error}");
            None
        }
    }
}

/// Waits until the monitor answers plain requests again (start of a session, after an outage).
fn wake_ddc(ddc: &mut MsiDdc) -> bool {
    ddc.settle_ddc()
}

fn load_cached_settings(name: &str, ui_tx: &async_channel::Sender<UiCmd>) -> HashMap<u8, u16> {
    let _ = ui_tx.send_blocking(UiCmd::Progress("Loading cached settings...".into()));
    load_cache_for(name)
}

// -----------------------------------------------------------------------------
// Cache helpers
// -----------------------------------------------------------------------------

#[derive(Clone, Default, Serialize, Deserialize)]
struct CachedMonitorSnapshot {
    resolution: String,
    hz: u16,
    serial_number: String,
    firm: String,
    usage_mins: u16,
}

#[derive(Default, Serialize, Deserialize)]
struct CacheStore {
    monitors: HashMap<String, HashMap<u8, u16>>,
    color_profiles: HashMap<String, HashMap<u16, HashMap<u8, u16>>>,
    snapshots: HashMap<String, CachedMonitorSnapshot>,
    last_monitor: Option<String>,
    last_i2c_path: Option<String>,
}

fn cache_file() -> PathBuf {
    if let Ok(cfg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(cfg)
            .join("titan_control")
            .join("settings_cache.json")
    } else if let Some(home) = dirs::home_dir() {
        home.join(".config")
            .join("titan_control")
            .join("settings_cache.json")
    } else {
        PathBuf::from("./settings_cache.json")
    }
}

fn load_cache_store() -> CacheStore {
    let path = cache_file();
    if let Ok(data) = fs::read_to_string(&path)
        && let Ok(store) = serde_json::from_str(&data)
    {
        return store;
    }
    CacheStore::default()
}

fn load_cache_for(name: &str) -> HashMap<u8, u16> {
    load_cache_store()
        .monitors
        .get(name)
        .cloned()
        .unwrap_or_default()
}

pub fn load_color_temp_profile_cache(name: &str, color_temp: u16) -> HashMap<u8, u16> {
    load_cache_store()
        .color_profiles
        .get(name)
        .and_then(|profiles| profiles.get(&color_temp))
        .cloned()
        .unwrap_or_default()
}

fn load_cached_snapshot(name: &str) -> Option<CachedMonitorSnapshot> {
    load_cache_store().snapshots.get(name).cloned()
}

pub fn load_last_monitor_name() -> Option<String> {
    load_cache_store().last_monitor
}

fn load_last_i2c_path() -> Option<String> {
    load_cache_store().last_i2c_path
}

fn save_cache_for(name: &str, map: &HashMap<u8, u16>, i2c_path: Option<&str>) {
    let path = cache_file();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut store = load_cache_store();
    store.monitors.insert(name.to_string(), map.clone());
    store.last_monitor = Some(name.to_string());
    if let Some(i2c_path) = i2c_path {
        store.last_i2c_path = Some(i2c_path.to_string());
    }
    if let Ok(data) = serde_json::to_string_pretty(&store) {
        let _ = fs::write(&path, data);
    }
}

fn save_color_temp_profile_cache(
    name: &str,
    color_temp: u16,
    map: &HashMap<u8, u16>,
    i2c_path: Option<&str>,
) {
    let path = cache_file();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut store = load_cache_store();
    store
        .color_profiles
        .entry(name.to_string())
        .or_default()
        .insert(color_temp, map.clone());
    store.last_monitor = Some(name.to_string());
    if let Some(i2c_path) = i2c_path {
        store.last_i2c_path = Some(i2c_path.to_string());
    }
    if let Ok(data) = serde_json::to_string_pretty(&store) {
        let _ = fs::write(&path, data);
    }
}

fn clear_color_temp_profile_cache(name: &str) {
    let path = cache_file();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut store = load_cache_store();
    store.color_profiles.remove(name);
    store.last_monitor = Some(name.to_string());
    if let Ok(data) = serde_json::to_string_pretty(&store) {
        let _ = fs::write(&path, data);
    }
}

fn save_cached_snapshot(name: &str, snapshot: CachedMonitorSnapshot, i2c_path: Option<&str>) {
    let path = cache_file();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut store = load_cache_store();
    store.snapshots.insert(name.to_string(), snapshot);
    store.last_monitor = Some(name.to_string());
    if let Some(i2c_path) = i2c_path {
        store.last_i2c_path = Some(i2c_path.to_string());
    }
    if let Ok(data) = serde_json::to_string_pretty(&store) {
        let _ = fs::write(&path, data);
    }
}

fn update_cached_snapshot_usage_time(name: &str, usage_mins: u16) {
    let path = cache_file();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut store = load_cache_store();
    if let Some(snapshot) = store.snapshots.get_mut(name) {
        snapshot.usage_mins = usage_mins;
        if let Ok(data) = serde_json::to_string_pretty(&store) {
            let _ = fs::write(&path, data);
        }
    }
}

fn apply_color_temp_cache(name: &str, settings: &mut HashMap<u8, u16>) {
    let Some(&color_temp) = settings.get(&VCP_COLOR_TEMP) else {
        return;
    };

    if let Some((red, green, blue)) = builtin_color_temp_rgb(color_temp) {
        settings.insert(VCP_RED, red);
        settings.insert(VCP_GREEN, green);
        settings.insert(VCP_BLUE, blue);
        return;
    }

    if is_user_color_temp(color_temp) {
        for (code, value) in load_color_temp_profile_cache(name, color_temp) {
            settings.insert(code, value);
        }
    }
}

fn persist_active_color_temp_profile(
    name: &str,
    settings: &HashMap<u8, u16>,
    i2c_path: Option<&str>,
) {
    let Some(&color_temp) = settings.get(&VCP_COLOR_TEMP) else {
        return;
    };

    if !is_user_color_temp(color_temp) {
        return;
    }

    let mut profile = load_color_temp_profile_cache(name, color_temp);
    for code in [VCP_RED, VCP_GREEN, VCP_BLUE] {
        if let Some(&value) = settings.get(&code) {
            profile.insert(code, value);
        }
    }
    save_color_temp_profile_cache(name, color_temp, &profile, i2c_path);
}

fn try_connect(
    dev_path: &str,
    name: &str,
    drm_info: Option<&DrmMonitorInfo>,
    ui_tx: &async_channel::Sender<UiCmd>,
) -> Option<(MsiDdc, MonitorInfo, bool)> {
    let mut ddc = match MsiDdc::open(dev_path) {
        Ok(d) => d,
        Err(e) => {
            log!("[DDC] {} open failed: {}, retrying...", dev_path, e);
            thread::sleep(Duration::from_millis(200));
            match MsiDdc::open(dev_path) {
                Ok(d) => d,
                Err(e) => {
                    log!("[DDC] {} retry failed: {}", dev_path, e);
                    return None;
                }
            }
        }
    };

    // Verify DDC/CI works: two valid replies in a row (a second round covers a periodic outage).
    if !wake_ddc(&mut ddc) && !wake_ddc(&mut ddc) {
        log!("[DDC] {} DDC/CI test failed", dev_path);
        return None;
    }

    log!("[DDC] Monitor ready on {}: {}", dev_path, name);
    let cached_snapshot = load_cached_snapshot(name);

    // The signal can change between sessions, so the refresh rate is always read (plain, cheap).
    let hz = read_vcp(&mut ddc, VCP_REFRESH_RATE)
        .map(|v| v / 100)
        .or_else(|| cached_snapshot.as_ref().map(|snapshot| snapshot.hz))
        .unwrap_or(0);
    let fe = if cached_snapshot.is_some() {
        0
    } else {
        read_vcp(&mut ddc, VCP_FIRMWARE).unwrap_or(0)
    };
    let mins = read_vcp(&mut ddc, VCP_USAGE_TIME).unwrap_or(0);

    // Cached values are shown at once; the worker then reads every setting from the monitor
    // (see `startup_read_keys`), so the cache is only a fast first draft.
    let mut settings = load_cached_settings(name, ui_tx);
    let _ = ui_tx.send_blocking(UiCmd::Progress("Reading picture profile...".to_string()));
    let table_read = match ddc.read_active_profile(TABLE_CONFIRMATIONS) {
        Ok(ActiveProfile::Custom(table)) => {
            log!("[DDC] Active profile table 0x{:02X} read", table.scene_id);
            settings.insert(VCP_MODE, table.scene_id as u16);
            settings.extend(table.settings());
            true
        }
        Ok(ActiveProfile::Default(scene_id)) => {
            log!("[DDC] Active profile 0x{scene_id:02X} (Default): settings are read one by one");
            settings.insert(VCP_MODE, scene_id as u16);
            false
        }
        Err(error) => {
            log!("[DDC] Active profile table read failed: {}", error);
            false
        }
    };
    apply_color_temp_cache(name, &mut settings);
    save_cache_for(name, &settings, Some(dev_path));

    let resolution = drm_info
        .map(|info| info.resolution.clone())
        .or_else(|| {
            cached_snapshot
                .as_ref()
                .map(|snapshot| snapshot.resolution.clone())
        })
        .unwrap_or_default();
    let serial_number = drm_info
        .map(|info| info.serial_number.clone())
        .or_else(|| {
            cached_snapshot
                .as_ref()
                .map(|snapshot| snapshot.serial_number.clone())
        })
        .unwrap_or_default();
    let firm = cached_snapshot
        .as_ref()
        .map(|snapshot| snapshot.firm.clone())
        .unwrap_or_else(|| format!("v {}.{}.{}", (fe >> 12) & 0xF, (fe >> 8) & 0xF, fe & 0xFF));

    let info = MonitorInfo {
        name: name.to_string(),
        connector: drm_info.map(|info| info.connector.clone()).unwrap_or_default(),
        resolution,
        hz,
        firm,
        usage_mins: mins,
        settings,
    };

    save_cached_snapshot(
        name,
        CachedMonitorSnapshot {
            resolution: info.resolution.clone(),
            hz: info.hz,
            serial_number,
            firm: info.firm.clone(),
            usage_mins: info.usage_mins,
        },
        Some(dev_path),
    );

    Some((ddc, info, table_read))
}

pub fn start_worker(worker_rx: mpsc::Receiver<WorkerCmd>, ui_tx: async_channel::Sender<UiCmd>) {
    thread::spawn(move || {
        let mut found = None;
        let mut current_monitor_name = None;

        let _ = ui_tx.send_blocking(UiCmd::Progress("Detecting monitors...".into()));
        let drm_monitors = get_drm_monitors();
        let cached_monitor_name = load_last_monitor_name();
        let cached_i2c_path = load_last_i2c_path();
        let target_name = drm_monitors
            .iter()
            .find(|n| {
                cached_monitor_name
                    .as_ref()
                    .map(|cached| n.name == *cached)
                    .unwrap_or(false)
            })
            .or_else(|| drm_monitors.iter().find(|n| n.name.contains("P275MV")))
            .map(|n| n.name.clone());
        if let Some(ref name) = target_name {
            log!("[DRM] Target monitor: {}", name);
        }

        if let Some(ref cached_path) = cached_i2c_path
            && Path::new(cached_path).exists()
        {
            let cached_name = target_name
                .clone()
                .or_else(|| get_edid_name_retry(cached_path, 1));
            if let Some(name) = cached_name {
                let drm_info = drm_monitors.iter().find(|monitor| monitor.name == name);
                if let Some((ddc, info, table_read)) = try_connect(cached_path, &name, drm_info, &ui_tx) {
                    let _ = ui_tx.send_blocking(UiCmd::MonitorFound(info));
                    current_monitor_name = Some(name);
                    found = Some((ddc, table_read));
                }
            }
        }

        let _ = ui_tx.send_blocking(UiCmd::Progress("Searching for monitors...".into()));
        for i in 0..32 {
            if found.is_some() {
                break;
            }
            let p = format!("/dev/i2c-{}", i);
            if !Path::new(&p).exists() {
                continue;
            }
            if cached_i2c_path.as_deref() == Some(p.as_str()) {
                continue;
            }

            let retries = if target_name.is_some() { 3 } else { 1 };
            let name = match get_edid_name_retry(&p, retries) {
                Some(n) => n,
                _none => continue,
            };
            log!("[I2C] {} EDID: {}", p, name);

            if let Some(ref target) = target_name
                && !name.contains("P275MV")
                && target.contains("P275MV")
            {
                log!("[I2C] {} skipping (not target)", p);
                continue;
            }

            let drm_info = drm_monitors.iter().find(|monitor| monitor.name == name);
            if let Some((ddc, info, table_read)) = try_connect(&p, &name, drm_info, &ui_tx) {
                let _ = ui_tx.send_blocking(UiCmd::MonitorFound(info));
                current_monitor_name = Some(name.clone());
                found = Some((ddc, table_read));
                break;
            }
        }

        let Some((ddc, table_read)) = found else {
            log!("[DDC] No compatible monitor found");
            let _ = ui_tx.send_blocking(UiCmd::Error("Monitor not found".into()));
            return;
        };

        let monitor_name = current_monitor_name.unwrap_or_else(|| "default".into());
        Worker::new(ddc, monitor_name, ui_tx, table_read).run(&worker_rx);
    });
}

/// Settings that are read from the monitor after connecting. Picture settings of a Custom profile come from the
/// profile table; they are read one by one for a Default profile or when the table could not be read.
fn startup_read_keys(table_read: bool) -> Vec<u8> {
    if table_read {
        return GLOBAL_SETTING_KEYS.to_vec();
    }
    let mut keys = vec![VCP_MODE];
    keys.extend_from_slice(PROFILE_SETTING_KEYS);
    keys.extend_from_slice(GLOBAL_SETTING_KEYS);
    keys
}

/// Settings of the active picture profile that can be read one by one (Default profiles, failed table reads).
/// The User 1-3 RGB gains are only in the profile table: 0x17, 0x19, 0x1B-0x1D answer "unsupported" and 0x1E
/// reads as max 2 (checked on the monitor 2026-10-01).
const PROFILE_SETTING_KEYS: &[u8] = &[
    VCP_BRIGHTNESS, VCP_CONTRAST, VCP_SHARPNESS, VCP_COLOR_ENHANCE, VCP_CR_ENHANCE, VCP_SHADOW_BALANCE,
    VCP_COLOR_TEMP, VCP_HUE_RED, VCP_HUE_GREEN, VCP_HUE_BLUE,
    VCP_HUE_YELLOW, VCP_HUE_CYAN, VCP_HUE_MAGENTA, VCP_SATURATION_RED, VCP_SATURATION_GREEN,
    VCP_SATURATION_BLUE, VCP_SATURATION_YELLOW, VCP_SATURATION_CYAN, VCP_SATURATION_MAGENTA, VCP_LOW_BLUE,
    VCP_HDR, VCP_GAMMA, VCP_SUPER_RES, VCP_NIGHT_VISION, VCP_DYNAMIC_OD,
];

/// Settings outside the profile table, read from the monitor after connecting.
const GLOBAL_SETTING_KEYS: &[u8] = &[
    // Picture (global)
    // Halo Control is not read: its register does not return the set value (see UNRELIABLE_READBACK);
    // the slider shows the last value written by the app (cache).
    VCP_DCR, VCP_LOCAL_DIMMING, VCP_DYDS, VCP_ADAPTIVE_SYNC, VCP_RATIO, VCP_EYESHIELD_REMINDER,
    // Game Aid
    VCP_SCREEN_SIZE, VCP_FPS_COUNTER, VCP_FPS_POS, VCP_CROSSHAIR, VCP_CROSSHAIR_SHAPE, VCP_CROSSHAIR_COLOR,
    VCP_STOPWATCH, VCP_STOPWATCH_TIME, VCP_STOPWATCH_POS, VCP_GAME_TIME, VCP_GAME_TIME_VAL, VCP_GAME_TIME_POS,
    VCP_MAGNIFIER, VCP_MAGNIFIER_SIZE, VCP_MAGNIFIER_ZOOM, VCP_MAGNIFIER_POS, VCP_HAWKEYE,
    VCP_HAWKEYE_SIZE, VCP_HAWKEYE_POS, VCP_HAWKEYE_LEVEL,
    // Inputs, PIP/PBP, USB
    VCP_INPUT_SOURCE, VCP_INPUT_SELECT, VCP_OUTPUT_RANGE, VCP_PIP_MODE, VCP_PIP_SOURCE, VCP_PIP_POSITION,
    VCP_PIP_SIZE, VCP_AUDIO_SOURCE, VCP_USB_HUB_SOURCE, VCP_USB_POWER_SLEEP,
    // Audio
    VCP_VOLUME, VCP_MUTE,
    // LED
    VCP_REAR_LED, VCP_POWER_LED, VCP_LED_MODE, VCP_LED_COLOR, VCP_LED_STRENGTH, VCP_LED_FRONT_COLOR,
    VCP_LED_REAR_COLOR,
    // OSD and system
    VCP_OSD_LANG, VCP_OSD_TIME, VCP_OSD_H_POS, VCP_OSD_V_POS, VCP_OSD_TRANS, VCP_OSD_LOCK, VCP_BUTTON_LOCK,
    VCP_QUICK_BOOT, VCP_POWER_SAVING,
];

/// Settings read per worker step, so user commands are not delayed by a long read.
const READ_CHUNK: usize = 4;
/// Interval of the OSD change check (`0x02` New control value).
const OSD_POLL_INTERVAL: Duration = Duration::from_secs(3);
/// Pause after a reset before its results are read back.
const RESET_SETTLE: Duration = Duration::from_millis(1500);
/// Read-backs of one write: the first mismatch resends the value, the last one reports what the monitor keeps.
const VERIFY_ATTEMPTS: u8 = 3;
/// Pause before the last read-back of a write.
const FINAL_VERIFY_DELAY: Duration = Duration::from_millis(1200);
/// Settings written by the app are not re-read after an OSD-change flag for this long.
const OWN_WRITE_QUIET: Duration = Duration::from_secs(5);
/// Settings whose register does not reliably show a written value (the OSD does change): a mismatch is only logged.
/// Halo Control: after writing 70 (OSD showed it) `46 [99]` read `ff64/0001` (2026-10-01).
const UNRELIABLE_READBACK: &[u8] = &[VCP_HALO_CONTROL];

/// Background work of the worker. User commands always go first; the UI is never blocked.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Task {
    /// Read a written setting back; `attempt` counts read-backs of this write.
    Verify { code: u8, value: u16, attempt: u8 },
    /// Read the active profile number and its table.
    RefreshProfile,
    /// Read settings from the monitor (startup, after a reset or an OSD change).
    Read(Vec<u8>),
    /// Tells the UI that the profile settings queued before it have been read.
    ProfileLoaded,
    /// Reads the profile table again until two reads agree and corrects the settings if the first read was wrong.
    ConfirmProfile,
}

struct Scheduled {
    task: Task,
    due: Instant,
}

struct Worker {
    ddc: MsiDdc,
    cache: HashMap<u8, u16>,
    monitor_name: String,
    ui_tx: async_channel::Sender<UiCmd>,
    tasks: VecDeque<Scheduled>,
    last_poll: Instant,
    /// When each setting was last written by the app (OSD-change reads of these are skipped for a while).
    recent_writes: HashMap<u8, Instant>,
    /// DyDs profiles may be selected with Adaptive-Sync on (firmware 2025-09-20).
    dyds_with_adaptive_sync: bool,
}

impl Worker {
    fn new(
        ddc: MsiDdc,
        monitor_name: String,
        ui_tx: async_channel::Sender<UiCmd>,
        table_read: bool,
    ) -> Self {
        let mut worker = Self {
            cache: load_cache_for(&monitor_name),
            ddc,
            monitor_name,
            ui_tx,
            tasks: VecDeque::new(),
            last_poll: Instant::now(),
            recent_writes: HashMap::new(),
            dyds_with_adaptive_sync: false,
        };
        worker.queue_reads(&startup_read_keys(table_read), Instant::now());
        worker
    }

    fn run(&mut self, worker_rx: &mpsc::Receiver<WorkerCmd>) {
        loop {
            // Wait for a command until the next task is due (or the next OSD check when there is no task).
            let now = Instant::now();
            let wake = self
                .tasks
                .iter()
                .map(|scheduled| scheduled.due)
                .min()
                .unwrap_or(self.last_poll + OSD_POLL_INTERVAL);
            let wait = wake.saturating_duration_since(now).max(Duration::from_millis(1));
            match worker_rx.recv_timeout(wait) {
                Ok(first_cmd) => {
                    let mut batch = vec![first_cmd];
                    while let Ok(cmd) = worker_rx.try_recv() {
                        batch.push(cmd);
                    }
                    self.handle_batch(batch);
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if let Some(task) = self.take_due_task() {
                        self.run_task(task);
                    } else if self.tasks.is_empty() && self.last_poll.elapsed() >= OSD_POLL_INTERVAL {
                        self.poll_osd_changes();
                        self.last_poll = Instant::now();
                    }
                }
            }
        }
    }

    /// First task in queue order whose time has come.
    fn take_due_task(&mut self) -> Option<Task> {
        let now = Instant::now();
        let index = self.tasks.iter().position(|scheduled| scheduled.due <= now)?;
        self.tasks.remove(index).map(|scheduled| scheduled.task)
    }

    fn handle_batch(&mut self, batch: Vec<WorkerCmd>) {
        // The UI stays usable: commands are queued and a newer value of a setting replaces the older one.
        for cmd in collapse_worker_cmds(batch) {
            match cmd {
                WorkerCmd::Set(code, value) => {
                    self.write(code, value);
                    // A refused or failed profile change reads nothing: unlock the profile controls at once.
                    if code == VCP_MODE && !self.tasks.iter().any(|task| task.task == Task::RefreshProfile) {
                        let _ = self.ui_tx.send_blocking(UiCmd::ProfileLoaded);
                    }
                }
                WorkerCmd::ReadUsageTime => {
                    if let Some(mins) = read_vcp(&mut self.ddc, VCP_USAGE_TIME) {
                        update_cached_snapshot_usage_time(&self.monitor_name, mins);
                        let _ = self.ui_tx.send_blocking(UiCmd::UsageTimeUpdated(mins));
                    }
                }
                WorkerCmd::RefreshAll => {
                    self.refresh_info();
                    let now = Instant::now();
                    self.tasks.retain(|scheduled| !matches!(scheduled.task, Task::Read(_) | Task::RefreshProfile | Task::ConfirmProfile));
                    self.tasks.push_front(Scheduled { task: Task::RefreshProfile, due: now });
                    self.queue_reads(GLOBAL_SETTING_KEYS, now);
                }
                WorkerCmd::DydsWithAdaptiveSync(allowed) => self.dyds_with_adaptive_sync = allowed,
            }
        }
        // Our own writes may raise the OSD change flag; check it only after the monitor is quiet again.
        self.last_poll = Instant::now();
    }

    /// Switches Adaptive-Sync off and confirms it (a prerequisite of `feature`); false = the feature write is cancelled.
    fn adaptive_sync_off(&mut self, feature: &str) -> bool {
        match self.ddc.set_vcp(VCP_ADAPTIVE_SYNC, 0) {
            Ok(None) => {
                self.recent_writes.insert(VCP_ADAPTIVE_SYNC, Instant::now());
                self.store(HashMap::from([(VCP_ADAPTIVE_SYNC, 0)]));
                true
            }
            Ok(Some(actual)) => {
                log!("[DDC] Adaptive Sync stays {actual}; {feature} change cancelled");
                self.store(HashMap::from([(VCP_ADAPTIVE_SYNC, actual)]));
                false
            }
            Err(error) => {
                log!("[DDC] Cannot disable Adaptive Sync; {feature} change cancelled: {error}");
                false
            }
        }
    }

    /// Reads the refresh rate and usage time again and sends them to the UI.
    fn refresh_info(&mut self) {
        let hz = read_vcp(&mut self.ddc, VCP_REFRESH_RATE).map(|v| v / 100);
        let usage_mins = read_vcp(&mut self.ddc, VCP_USAGE_TIME);
        if let Some(mins) = usage_mins {
            update_cached_snapshot_usage_time(&self.monitor_name, mins);
        }
        let _ = self.ui_tx.send_blocking(UiCmd::InfoRefreshed { hz, usage_mins });
    }

    /// Sends a write and schedules its read-back.
    fn write(&mut self, code: u8, value: u16) {
        // A newer value replaces the one still waiting for its read-back (and a newer profile its table read).
        self.tasks.retain(|scheduled| match scheduled.task {
            Task::Verify { code: pending, .. } => pending != code,
            Task::RefreshProfile | Task::ConfirmProfile => code != VCP_MODE,
            Task::Read(_) | Task::ProfileLoaded => true,
        });

        // DyDs profiles need Adaptive-Sync off on firmware older than 2025-09-20; this one is confirmed before the
        // profile is written, after queue coalescing, so tray/default/custom paths all use the same order.
        if code == VCP_MODE
            && (0x1b..=0x1e).contains(&value)
            && !self.dyds_with_adaptive_sync
            && !self.adaptive_sync_off("DyDs")
        {
            return;
        }

        if let Err(error) = self.ddc.send_vcp(code, value) {
            log!("[DDC] VCP 0x{code:02X} error: {error}");
            return;
        }
        self.recent_writes.insert(code, Instant::now());
        log!("[DDC] write 0x{code:02X} = {value}");

        if is_reset_action(code) {
            if code == VCP_RESET_FACTORY || code == VCP_VENDOR_FACTORY_RESET {
                self.cache = factory_reset_defaults();
                apply_color_temp_cache(&self.monitor_name, &mut self.cache);
                clear_color_temp_profile_cache(&self.monitor_name);
                save_cache_for(&self.monitor_name, &self.cache, None);
            }
            // Read back what the reset really changed, once the monitor has applied it.
            let due = Instant::now() + RESET_SETTLE;
            self.tasks.push_front(Scheduled { task: Task::RefreshProfile, due });
            self.queue_reads(GLOBAL_SETTING_KEYS, due);
            return;
        }

        // Optimistic: the value counts as set; the read-back corrects it if the monitor disagrees.
        self.apply_written(code, value);
        if code == VCP_MODE {
            // A profile change loads that profile's picture settings (after the profile write is confirmed).
            self.tasks.push_front(Scheduled { task: Task::RefreshProfile, due: Instant::now() + VERIFY_DELAY });
        }
        // Settings the monitor changes by itself after this one (e.g. DCR turns local dimming off).
        let dependents = dependent_reads(code);
        if !dependents.is_empty() {
            self.tasks.push_front(Scheduled {
                task: Task::Read(dependents.to_vec()),
                due: Instant::now() + DEPENDENT_READ_DELAY,
            });
        }
        if is_verifiable(code) {
            self.tasks.push_front(Scheduled {
                task: Task::Verify { code, value, attempt: 1 },
                due: Instant::now() + VERIFY_DELAY,
            });
        }
    }

    /// Updates the cache after a write (also keeps the color-temperature RGB profiles in sync).
    fn apply_written(&mut self, code: u8, value: u16) {
        self.cache.insert(code, value);
        if let Some(cache_code) = cache_rgb_key_for_vcp_code(code) {
            self.cache.insert(cache_code, value);
        }
        if code == VCP_COLOR_TEMP {
            apply_color_temp_cache(&self.monitor_name, &mut self.cache);
        }
        if is_color_temp_rgb_code(code) {
            persist_active_color_temp_profile(&self.monitor_name, &self.cache, None);
        }
        save_cache_for(&self.monitor_name, &self.cache, None);
    }

    fn run_task(&mut self, task: Task) {
        match task {
            Task::Verify { code, value, attempt } => self.verify(code, value, attempt),
            Task::RefreshProfile => self.refresh_profile(),
            Task::Read(keys) => self.read_keys(keys),
            Task::ProfileLoaded => {
                let _ = self.ui_tx.send_blocking(UiCmd::ProfileLoaded);
            }
            Task::ConfirmProfile => {
                if let Ok(ActiveProfile::Custom(table)) = self.ddc.read_active_profile(TABLE_CONFIRMATIONS) {
                    let mut settings = table.settings();
                    settings.insert(VCP_MODE, table.scene_id as u16);
                    self.store(settings);
                }
            }
        }
    }

    fn verify(&mut self, code: u8, value: u16, attempt: u8) {
        match self.ddc.verify_vcp(code, value) {
            Readback::Confirmed => log!("[DDC] VCP 0x{code:02X} = {value} confirmed"),
            Readback::Unreadable if attempt < VERIFY_ATTEMPTS => self.schedule_verify(code, value, attempt + 1),
            Readback::Unreadable => log!("[DDC] VCP 0x{code:02X} = {value} could not be read back"),
            Readback::Differs(actual) if attempt < VERIFY_ATTEMPTS => {
                // A single write is sometimes lost: send it again once, then only read.
                if attempt == 1 {
                    log!("[DDC] VCP 0x{code:02X} = {value} not confirmed (read {actual}), resending");
                    if let Err(error) = self.ddc.send_vcp(code, value) {
                        log!("[DDC] VCP 0x{code:02X} error: {error}");
                    }
                }
                self.schedule_verify(code, value, attempt + 1);
            }
            Readback::Differs(actual) if UNRELIABLE_READBACK.contains(&code) => {
                log!("[DDC] VCP 0x{code:02X} = {value} written, register reads {actual} (read-back ignored)");
            }
            Readback::Differs(actual) => {
                // The monitor rejected or adjusted the value: keep what it reports and show it.
                log!("[DDC] VCP 0x{code:02X} = {value} not accepted, monitor reports {actual}");
                self.apply_written(code, actual);
                let _ = self.ui_tx.send_blocking(UiCmd::Corrected(HashMap::from([(code, actual)])));
            }
        }
    }

    fn schedule_verify(&mut self, code: u8, value: u16, attempt: u8) {
        // The last read-back waits longer: some settings take a moment before their register follows.
        let delay = if attempt >= VERIFY_ATTEMPTS { FINAL_VERIFY_DELAY } else { VERIFY_DELAY * 4 };
        self.tasks.push_front(Scheduled { task: Task::Verify { code, value, attempt }, due: Instant::now() + delay });
    }

    /// Reads settings from the monitor and sends the ones that could be read to the UI.
    fn read_keys(&mut self, keys: Vec<u8>) {
        let mut settings = HashMap::new();
        for key in keys {
            // Replies are validated (header, checksum, code), so every value read is real.
            if let Some(value) = read_vcp(&mut self.ddc, key) {
                settings.insert(key, value);
            }
        }
        self.store(settings);
    }

    /// Reads the active profile number and its table (picture settings).
    /// Reads the active profile. The first valid table is taken at once so the UI is unlocked quickly; a second,
    /// confirming read follows in the background (`Task::ConfirmProfile`).
    fn refresh_profile(&mut self) {
        match self.ddc.read_active_profile(1) {
            Ok(ActiveProfile::Custom(table)) => {
                let mut settings = table.settings();
                settings.insert(VCP_MODE, table.scene_id as u16);
                self.store(settings);
                let _ = self.ui_tx.send_blocking(UiCmd::ProfileLoaded);
                self.tasks.push_back(Scheduled { task: Task::ConfirmProfile, due: Instant::now() });
            }
            Ok(ActiveProfile::Default(scene_id)) => {
                self.store(HashMap::from([(VCP_MODE, scene_id as u16)]));
                self.read_profile_settings(PROFILE_SETTING_KEYS);
            }
            Err(error) => {
                log!("[DDC] Profile table read failed: {error}; reading settings one by one");
                let mut keys = vec![VCP_MODE];
                keys.extend_from_slice(PROFILE_SETTING_KEYS);
                self.read_profile_settings(&keys);
            }
        }
    }

    /// Reads the profile settings one by one, ahead of everything else in the queue, then tells the UI.
    fn read_profile_settings(&mut self, keys: &[u8]) {
        self.tasks.retain(|scheduled| !matches!(scheduled.task, Task::ProfileLoaded));
        let mut chunks: VecDeque<Scheduled> = keys
            .chunks(READ_CHUNK)
            .map(|chunk| Scheduled { task: Task::Read(chunk.to_vec()), due: Instant::now() })
            .collect();
        chunks.push_back(Scheduled { task: Task::ProfileLoaded, due: Instant::now() });
        for task in chunks.into_iter().rev() {
            self.tasks.push_front(task);
        }
    }

    /// Queues reads in chunks at the end of the queue, skipping settings already waiting to be read.
    fn queue_reads(&mut self, keys: &[u8], due: Instant) {
        let queued: HashSet<u8> = self
            .tasks
            .iter()
            .filter_map(|scheduled| match &scheduled.task {
                Task::Read(keys) => Some(keys.clone()),
                _ => None,
            })
            .flatten()
            .collect();
        let missing: Vec<u8> = keys.iter().copied().filter(|key| !queued.contains(key)).collect();
        for chunk in missing.chunks(READ_CHUNK) {
            self.tasks.push_back(Scheduled { task: Task::Read(chunk.to_vec()), due });
        }
    }

    /// Updates the cache and the UI.
    fn store(&mut self, mut settings: HashMap<u8, u16>) {
        if settings.is_empty() {
            return;
        }
        if settings.contains_key(&VCP_COLOR_TEMP) {
            apply_color_temp_cache(&self.monitor_name, &mut settings);
        }
        self.cache.extend(settings.iter().map(|(&code, &value)| (code, value)));
        save_cache_for(&self.monitor_name, &self.cache, None);
        let _ = self.ui_tx.send_blocking(UiCmd::Settings(settings));
    }

    /// Detects settings changed with the monitor buttons: `0x02` = 2 means "changed in the OSD" and the plain
    /// register of `0x52` names the last changed code. Writing `0x02 = 1` clears both. Manufacturer settings
    /// do not always appear in `0x52`; an unknown code triggers a full read.
    fn poll_osd_changes(&mut self) {
        let Ok(flag) = self.ddc.get_vcp(VCP_NEW_CONTROL_VALUE) else {
            return;
        };
        if flag != 2 {
            return;
        }
        let changed = self.ddc.get_active_control().ok();
        if let Err(error) = self.ddc.send_vcp(VCP_NEW_CONTROL_VALUE, 1) {
            log!("[DDC] Cannot clear the OSD change flag: {error}");
        }
        log!("[DDC] Setting changed in the OSD: {changed:02X?}");
        let now = Instant::now();
        // Our own writes also raise the flag; reading such a setting back could show a lagging register value.
        if let Some(code) = changed
            && self.recent_writes.get(&code).is_some_and(|at| at.elapsed() < OWN_WRITE_QUIET)
        {
            return;
        }
        match changed {
            Some(VCP_MODE) => self.tasks.push_back(Scheduled { task: Task::RefreshProfile, due: now }),
            Some(code) if PROFILE_SETTING_KEYS.contains(&code) || GLOBAL_SETTING_KEYS.contains(&code) => {
                self.queue_reads(&[code], now);
            }
            _ => {
                self.tasks.push_back(Scheduled { task: Task::RefreshProfile, due: now });
                self.queue_reads(GLOBAL_SETTING_KEYS, now);
            }
        }
    }
}

/// Pause before settings that depend on a write are read back.
const DEPENDENT_READ_DELAY: Duration = Duration::from_millis(800);

/// Settings the monitor may change by itself after a write of `code` (OSD rules, observed in the old UI logic).
fn dependent_reads(code: u8) -> &'static [u8] {
    match code {
        VCP_SCREEN_SIZE => &[VCP_ADAPTIVE_SYNC, VCP_DYDS, VCP_DYNAMIC_OD, VCP_MAGNIFIER, VCP_HAWKEYE],
        VCP_ADAPTIVE_SYNC => &[VCP_DYDS, VCP_MAGNIFIER, VCP_HAWKEYE],
        VCP_DYDS => &[VCP_LOCAL_DIMMING, VCP_DCR, VCP_ADAPTIVE_SYNC],
        VCP_LOCAL_DIMMING => &[VCP_DCR],
        // Firmware 2025-09-20: HDR on sets local dimming to High and locks brightness, contrast and DCR.
        VCP_HDR => &[VCP_LOCAL_DIMMING, VCP_DCR, VCP_BRIGHTNESS, VCP_CONTRAST],
        VCP_DCR => &[VCP_BRIGHTNESS, VCP_CONTRAST, VCP_LOCAL_DIMMING],
        VCP_MAGNIFIER => &[VCP_HAWKEYE],
        VCP_HAWKEYE => &[VCP_MAGNIFIER],
        VCP_LED_MODE => &[VCP_LED_COLOR, VCP_LED_STRENGTH, VCP_LED_FRONT_COLOR, VCP_LED_REAR_COLOR],
        VCP_PIP_MODE => &[
            VCP_PIP_SOURCE, VCP_PIP_POSITION, VCP_PIP_SIZE, VCP_AUDIO_SOURCE, VCP_DCR, VCP_HDR, VCP_ADAPTIVE_SYNC,
            VCP_DYDS, VCP_DYNAMIC_OD, VCP_FPS_COUNTER, VCP_CROSSHAIR, VCP_STOPWATCH,
            VCP_GAME_TIME, VCP_MAGNIFIER, VCP_HAWKEYE,
        ],
        VCP_PIP_SWAP => &[VCP_PIP_SOURCE, VCP_INPUT_SELECT, VCP_AUDIO_SOURCE],
        _ => &[],
    }
}

/// Writes after which the monitor changes several settings at once.
fn is_reset_action(code: u8) -> bool {
    RESET_CODES.contains(&code) || code == VCP_VENDOR_FACTORY_RESET || code == VCP_PIP_RESET
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_enhance_sets_the_saturation() {
        assert_eq!(color_enhance_saturation(0), None);
        assert_eq!(color_enhance_saturation(11), None);
        // Level 10 as read: red 74, green 76, blue 77, yellow 67, cyan 67, magenta 66.
        assert_eq!(color_enhance_saturation(10), Some([74, 76, 77, 67, 66, 67]));
        assert_eq!(SATURATION_CODES[3], VCP_SATURATION_CYAN);
        assert_eq!(SATURATION_CODES[5], VCP_SATURATION_YELLOW);
    }

    #[test]
    fn registers_follow_the_code_map() {
        // Manufacturer register (GET 99 first).
        for key in [VCP_LOCAL_DIMMING, VCP_GAMMA, VCP_MUTE, VCP_OSD_TIME, VCP_DCR, VCP_LED_MODE, VCP_PIP_MODE] {
            assert_eq!(route(key).access, Access::Vendor, "0x{key:02X}");
        }
        // Profile table selectors: plain read, write with 99.
        for key in [VCP_BRIGHTNESS, VCP_CONTRAST, VCP_COLOR_TEMP, VCP_USER3_BLUE] {
            assert_eq!(route(key).access, Access::Table, "0x{key:02X}");
        }
        // Plain MCCS registers.
        for key in [VCP_REAR_LED, VCP_ADAPTIVE_SYNC, VCP_INPUT_SOURCE, VCP_DPMS, VCP_DISPLAY_APPLICATION,
            VCP_BUTTON_LOCK, VCP_RESET_FACTORY, VCP_RESET_BC, VCP_RESET_COLOR, VCP_NEW_CONTROL_VALUE]
        {
            assert_eq!(route(key).access, Access::Plain, "0x{key:02X}");
        }
        // Output Range is 0x60 after 99; the plain 0x60 is the input source.
        let output_range = route(VCP_OUTPUT_RANGE);
        assert_eq!((output_range.code, output_range.access), (0x60, Access::Vendor));
        // The manufacturer factory reset is written with 99.
        assert_eq!(route(VCP_VENDOR_FACTORY_RESET).access, Access::Vendor);
    }

    #[test]
    fn actions_are_not_verified() {
        for key in [VCP_RESET_FACTORY, VCP_RESET_BC, VCP_RESET_COLOR, VCP_VENDOR_FACTORY_RESET, VCP_DPMS,
            VCP_PIP_SWAP, VCP_PIP_RESET, VCP_NEW_CONTROL_VALUE]
        {
            assert!(!route(key).verify, "0x{key:02X}");
        }
        assert!(route(VCP_BRIGHTNESS).verify);
        assert!(route(VCP_LED_COLOR).verify);
    }

    #[test]
    fn dangerous_writes_are_blocked() {
        assert!(check_write(VCP_MODE, 0).is_err());
        assert!(check_write(0xF0, 1).is_err());
        assert!(check_write(VCP_MODE, 1).is_ok());
        assert!(check_write(VCP_DPMS, values::POWER_OFF).is_ok());
    }

    #[test]
    fn source_registers_use_the_low_byte() {
        let vendor = route(VCP_INPUT_SELECT);
        assert_eq!(vendor.value(VcpReply { maximum: 0xFF05, current: 0x6903 }), 3);
        assert_eq!(vendor.value(VcpReply { maximum: 0xFF64, current: 50 }), 50);
        // Plain registers keep the full value (e.g. usage minutes).
        let plain = route(VCP_USAGE_TIME);
        assert_eq!(plain.value(VcpReply { maximum: 0, current: 0x0E10 }), 3600);
    }

    #[test]
    fn profile_table_low_blue_light_is_a_level() {
        let mut bytes = [0u8; PROFILE_TABLE_LEN];
        for (raw, level) in [(0u8, 0u16), (25, 1), (50, 2), (75, 3), (100, 4)] {
            bytes[30] = raw;
            let table = ProfileTable { scene_id: 2, bytes };
            assert_eq!(table.settings()[&VCP_LOW_BLUE], level);
        }
    }

    #[test]
    fn pip_and_led_dependencies() {
        let mut settings = HashMap::from([(VCP_PIP_MODE, 0), (VCP_LED_MODE, 1)]);
        assert!(is_setting_available(&settings, VCP_HDR));
        assert!(!is_setting_available(&settings, VCP_PIP_SOURCE));
        assert!(!is_setting_available(&settings, VCP_LED_FRONT_COLOR));
        settings.insert(VCP_PIP_MODE, 1);
        settings.insert(VCP_LED_MODE, values::LED_MODE_COLORFUL_WATER);
        assert!(!is_setting_available(&settings, VCP_HDR));
        assert!(is_setting_available(&settings, VCP_PIP_SOURCE));
        assert!(is_setting_available(&settings, VCP_LED_FRONT_COLOR));
    }

    #[test]
    fn picture_modes_match_the_code_map() {
        assert_eq!(picture_mode_from_vcp(1), Some((0, false)));
        assert_eq!(picture_mode_from_vcp(2), Some((0, true)));
        assert_eq!(picture_mode_from_vcp(5), Some((4, false)));
        assert_eq!(picture_mode_from_vcp(6), Some((4, true)));
        assert!(PICTURE_MODE_DEFAULT_VALUES.iter().all(|&value| value != 0));
    }

    /// Hardware test that WRITES: brightness and OSD H-position are changed by 1 and restored.
    /// Measures how long the monitor needs before a read returns the written value (basis for `VERIFY_DELAY`).
    /// `TITAN_I2C=/dev/i2c-14 cargo test write_readback_delay -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn write_readback_delay() {
        let path = std::env::var("TITAN_I2C").unwrap_or_else(|_| "/dev/i2c-14".into());
        let mut ddc = MsiDdc::open(&path).expect("open i2c device");
        for key in [VCP_BRIGHTNESS, VCP_OSD_H_POS] {
            let original = ddc.get_vcp(key).expect("read original value");
            let target = if original >= 100 { original - 1 } else { original + 1 };
            for (step, value) in [target, original, target, original, target, original].into_iter().enumerate() {
                ddc.send_vcp(key, value).expect("write");
                let sent = Instant::now();
                let mut reads = 0;
                let elapsed = loop {
                    reads += 1;
                    match ddc.verify_vcp(key, value) {
                        Readback::Confirmed => break Some(sent.elapsed()),
                        _ if sent.elapsed() > Duration::from_secs(3) => break None,
                        _ => {}
                    }
                };
                println!("0x{key:02X} step {step}: {value} confirmed after {elapsed:?} ({reads} reads)");
            }
            assert_eq!(ddc.get_vcp(key).expect("read restored value"), original, "0x{key:02X} restored");
        }
    }

    /// Hardware test, reads only: `TITAN_I2C=/dev/i2c-14 cargo test reads_all_settings -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn reads_all_settings() {
        let path = std::env::var("TITAN_I2C").unwrap_or_else(|_| "/dev/i2c-14".into());
        let mut ddc = MsiDdc::open(&path).expect("open i2c device");
        let mut failed = Vec::new();
        for &key in GLOBAL_SETTING_KEYS.iter().chain(PROFILE_SETTING_KEYS) {
            match ddc.get_vcp(key) {
                Ok(value) => println!("0x{key:02X} = {value}"),
                Err(error) => {
                    println!("0x{key:02X}: {error}");
                    failed.push(key);
                }
            }
        }
        assert!(failed.is_empty(), "unreadable: {failed:02X?}");
    }

    /// Hardware test: `TITAN_I2C=/dev/i2c-14 cargo test profile_table -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn profile_table_reads_are_reliable() {
        let path = std::env::var("TITAN_I2C").unwrap_or_else(|_| "/dev/i2c-14".into());
        let mut ddc = MsiDdc::open(&path).expect("open i2c device");
        let mut first = None;
        for round in 0..10 {
            let started = Instant::now();
            let table = match ddc.read_active_profile(TABLE_CONFIRMATIONS).expect("table read") {
                ActiveProfile::Custom(table) => table,
                ActiveProfile::Default(scene_id) => {
                    println!("profile 0x{scene_id:02X} is a Default profile: no table, switch to a Custom one");
                    return;
                }
            };
            println!("round {round}: profile {} {:?} in {:?}", table.scene_id, &table.bytes[2..10], started.elapsed());
            match &first {
                None => first = Some(table.bytes),
                Some(bytes) => assert_eq!(bytes, &table.bytes),
            }
        }
    }
}
