use i2c_linux::I2c;
use std::{
    collections::{HashMap, HashSet},
    fs,
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::Duration,
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
pub const VCP_DPMS: u8 = 0xD6;
pub const VCP_INPUT_SOURCE: u8 = 0x60;
pub const VCP_MODE: u8 = 0x22;
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

pub const VCP_FPS_COUNTER: u8 = 0x30;
pub const VCP_FPS_POS: u8 = 0x31;
pub const VCP_CROSSHAIR_COLOR: u8 = 0x32;
pub const VCP_STOPWATCH_TIME: u8 = 0x33;
pub const VCP_CROSSHAIR_SHAPE: u8 = 0x34;
pub const VCP_STOPWATCH_POS: u8 = 0x35;
pub const VCP_GAME_TIME_VAL: u8 = 0x36;
pub const VCP_GAME_TIME_POS: u8 = 0x37;
pub const VCP_MAGNIFIER_POS: u8 = 0x38;
pub const VCP_MAGNIFIER_NV: u8 = 0x39;
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

#[allow(dead_code)]
pub const VCP_SCENE_MODE: u8 = 0xDC;
#[allow(dead_code)]
pub const VCP_SHADOW_BALANCE_LEGACY: u8 = 0x0E;

pub const VCP_OUTPUT_RANGE: u8 = 0x60;
pub const VCP_QUICK_BOOT: u8 = 0x61;
pub const VCP_ADAPTIVE_SYNC: u8 = 0xE2;
pub const VCP_HAWKEYE: u8 = 0x63;
pub const VCP_HAWKEYE_SIZE: u8 = 0x64;
pub const VCP_HAWKEYE_POS: u8 = 0x65;
pub const VCP_HAWKEYE_LEVEL: u8 = 0x66;
pub const VCP_OSD_TIME: u8 = 0xC0;
pub const VCP_OSD_H_POS: u8 = 0xC1;
pub const VCP_OSD_V_POS: u8 = 0xC2;
pub const VCP_OSD_TRANS: u8 = 0xC3;
pub const VCP_POWER_LED: u8 = 0xC5;
pub const VCP_POWER_SAVING: u8 = 0xC6;
pub const VCP_HUE_RED: u8 = 0x9B;
pub const VCP_HUE_YELLOW: u8 = 0x9C;
pub const VCP_HUE_GREEN: u8 = 0x9D;
pub const VCP_HUE_CYAN: u8 = 0x9E;
pub const VCP_HUE_BLUE: u8 = 0x9F;
pub const VCP_HUE_MAGENTA: u8 = 0xA0;
pub const VCP_VOLUME: u8 = 0xF6;
pub const VCP_MUTE: u8 = 0xF7;

const VCP_REFRESH_RATE: u8 = 0xAE;
const VCP_CONTROLLER_TYPE: u8 = 0xC8;
const VCP_USAGE_TIME: u8 = 0xF3;
const VCP_FIRMWARE: u8 = 0xFE;
const VCP_DDCCI_INIT: u8 = 0x99;

pub struct MsiDdc {
    i2c: I2c<File>,
}

impl MsiDdc {
    pub fn open(path: &str) -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        let mut i2c = I2c::new(file);
        i2c.smbus_set_slave_address(DDC_ADDR, false)?;
        Ok(Self { i2c })
    }

    fn write_vcp_frame(&mut self, code: u8, value: u16) -> io::Result<()> {
        let vh = (value >> 8) as u8;
        let vl = (value & 0xFF) as u8;
        let chk = DDC_DEST ^ DDC_SRC ^ 0x84 ^ 0x03 ^ code ^ vh ^ vl;
        let buf = [DDC_SRC, 0x84, 0x03, code, vh, vl, chk];

        self.i2c.write_all(&buf)
    }

    fn write_standard_vcp(&mut self, code: u8, value: u16) -> io::Result<()> {
        let delay_ms = write_delay_ms(code);
        for _ in 0..3 {
            self.write_vcp_frame(code, value)?;
            thread::sleep(Duration::from_millis(delay_ms));
        }
        Ok(())
    }

    pub fn set_vcp(&mut self, code: u8, value: u16) -> io::Result<()> {
        if uses_standard_vcp(code) {
            return self.write_standard_vcp(code, value);
        }
        let delay_ms = write_delay_ms(code);
        for _ in 0..3 {
            self.write_vcp_frame(code, value)?;
            thread::sleep(Duration::from_millis(delay_ms));
            self.write_vcp_frame(0x99, 0x00F6)?;
            thread::sleep(Duration::from_millis(delay_ms));
        }
        Ok(())
    }

    pub fn set_vcp_save(&mut self, code: u8, value: u16) -> io::Result<()> {
        self.set_vcp(code, value)
    }

    pub fn get_vcp(&mut self, code: u8) -> io::Result<u16> {
        let chk = DDC_DEST ^ DDC_SRC ^ 0x82 ^ 0x01 ^ code;
        let req = [DDC_SRC, 0x82, 0x01, code, chk];
        self.i2c.write_all(&req)?;
        thread::sleep(Duration::from_millis(250));

        let mut resp = [0u8; 11];
        self.i2c.read_exact(&mut resp)?;

        if resp[2] != 0x02 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("VCP 0x{:02X}: bad reply 0x{:02X}", code, resp[2]),
            ));
        }
        if resp[3] != 0x00 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("VCP 0x{:02X}: unsupported", code),
            ));
        }

        Ok(((resp[8] as u16) << 8) | resp[9] as u16)
    }
}

fn uses_standard_vcp(code: u8) -> bool {
    [
        VCP_RESET_FACTORY,
        VCP_RESET_BC,
        VCP_RESET_COLOR,
        VCP_ADAPTIVE_SYNC,
        VCP_DPMS,
        VCP_INPUT_SOURCE,
        VCP_OSD_LANG,
    ]
    .contains(&code)
}

fn write_delay_ms(code: u8) -> u64 {
    if [
        VCP_VOLUME,
        VCP_OSD_TIME,
        VCP_OSD_H_POS,
        VCP_OSD_V_POS,
        VCP_OSD_TRANS,
        VCP_BRIGHTNESS,
        VCP_CONTRAST,
        VCP_SHARPNESS,
        VCP_SHADOW_BALANCE,
        VCP_CR_ENHANCE,
        VCP_COLOR_ENHANCE,
        VCP_SUPER_RES,
        VCP_LOW_BLUE,
        VCP_RED,
        VCP_GREEN,
        VCP_BLUE,
        VCP_USER2_RED,
        VCP_USER2_GREEN,
        VCP_USER2_BLUE,
        VCP_USER3_RED,
        VCP_USER3_GREEN,
        VCP_USER3_BLUE,
        VCP_HALO_CONTROL,
    ]
    .contains(&code)
        || HUE_CODES.contains(&code)
        || SATURATION_CODES.contains(&code)
    {
        25
    } else {
        125
    }
}

pub struct MonitorInfo {
    pub name: String,
    pub resolution: String,
    pub hz: u16,
    pub firm: String,
    pub usage_mins: u16,
    pub settings: HashMap<u8, u16>,
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
    SetSave(u8, u16),
}

pub enum UiCmd {
    MonitorFound(MonitorInfo),
    Progress(String),
    Error(String),
    Busy(bool),
}

fn worker_cmd_code(cmd: &WorkerCmd) -> u8 {
    match cmd {
        WorkerCmd::Set(code, _) | WorkerCmd::SetSave(code, _) => *code,
    }
}

fn collapse_worker_cmds(mut batch: Vec<WorkerCmd>) -> Vec<WorkerCmd> {
    let mut seen = HashSet::new();
    let mut collapsed = Vec::with_capacity(batch.len());

    while let Some(cmd) = batch.pop() {
        let code = worker_cmd_code(&cmd);
        if seen.insert(code) {
            collapsed.push(cmd);
        }
    }

    collapsed.reverse();
    collapsed
}

pub const COLOR_TEMP_VALUES: [u16; 6] = [0x05, 0x06, 0x08, 0x0b, 0x0c, 0x0d];
pub const OUTPUT_RANGE_VALUES: [u16; 3] = [0, 1, 2];
pub const INPUT_SOURCE_VALUES: [u16; 4] = [0x11, 0x12, 0x0F, 0x10];
pub const HDR_VALUES: [u16; 4] = [0, 1, 2, 3];
pub const GAMMA_VALUES: [u16; 6] = [0x02, 0x04, 0x03, 0x08, 0x0A, 0x0C];
pub const NIGHT_VISION_VALUES: [u16; 5] = [0, 1, 2, 3, 4];
pub const DYNAMIC_OD_VALUES: [u16; 5] = [0, 1, 2, 3, 4];
pub const LOCAL_DIMMING_VALUES: [u16; 5] = [2, 3, 4, 5, 6];
pub const DYDS_VALUES: [u16; 7] = [2, 3, 4, 5, 6, 7, 8];
pub const POWER_SAVING_VALUES: [u16; 4] = [1, 2, 3, 4];
pub const HUE_CODES: [u8; 6] = [
    VCP_HUE_RED,
    VCP_HUE_GREEN,
    VCP_HUE_BLUE,
    VCP_HUE_CYAN,
    VCP_HUE_MAGENTA,
    VCP_HUE_YELLOW,
];
pub const SATURATION_CODES: [u8; 6] = [
    VCP_SATURATION_RED,
    VCP_SATURATION_GREEN,
    VCP_SATURATION_BLUE,
    VCP_SATURATION_CYAN,
    VCP_SATURATION_MAGENTA,
    VCP_SATURATION_YELLOW,
];
pub const OSD_LANGUAGE_VALUES: [u16; 22] = [
    0x02, 0x1E, 0x04, 0x0A, 0x03, 0x05, 0x24, 0x14, 0x16, 0x17, 0x08, 0x0E, 0x0C, 0x0F, 0x19, 0x07,
    0x25, 0x06, 0x23, 0x0D, 0x01, 0x09,
];
pub const OSD_LANGUAGE_KEYS: [&str; 22] = [
    "osd_lang_english",
    "osd_lang_polish",
    "osd_lang_german",
    "osd_lang_spanish",
    "osd_lang_french",
    "osd_lang_italian",
    "osd_lang_ukrainian",
    "osd_lang_dutch",
    "osd_lang_finnish",
    "osd_lang_greek",
    "osd_lang_portuguese",
    "osd_lang_portuguese_br",
    "osd_lang_turkish",
    "osd_lang_arabic",
    "osd_lang_hindi",
    "osd_lang_korean",
    "osd_lang_vietnamese",
    "osd_lang_japanese",
    "osd_lang_thai",
    "osd_lang_chinese_h",
    "osd_lang_chinese_t",
    "osd_lang_russian",
];
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
pub const PICTURE_MODE_DEFAULT_VALUES: [u16; 15] = [
    0x02, 0x1B, 0x1D, 0x04, 0x06, 0x07, 0x09, 0x0B, 0x0D, 0x0F, 0x11, 0x13, 0x15, 0x17, 0x19,
];
pub const PICTURE_MODE_CUSTOM_VALUES: [Option<u16>; 15] = [
    Some(0x03),
    Some(0x1C),
    Some(0x1E),
    Some(0x05),
    None,
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

pub fn vcp_to_combo_index(values: &[u16], vcp_val: u16) -> u32 {
    values.iter().position(|&v| v == vcp_val).unwrap_or(0) as u32
}

pub fn picture_mode_vcp_value(index: usize, custom: bool) -> Option<u16> {
    if custom {
        PICTURE_MODE_CUSTOM_VALUES.get(index).copied().flatten()
    } else {
        PICTURE_MODE_DEFAULT_VALUES.get(index).copied()
    }
}

pub fn picture_mode_has_custom(index: usize) -> bool {
    PICTURE_MODE_CUSTOM_VALUES
        .get(index)
        .and_then(|value| *value)
        .is_some()
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
        (VCP_BRIGHTNESS, 100),
        (VCP_CONTRAST, 50),
        (VCP_SHARPNESS, 0),
        (VCP_COLOR_TEMP, 0x05),
        (VCP_RED, 48),
        (VCP_GREEN, 50),
        (VCP_BLUE, 47),
        (VCP_LOW_BLUE, 0),
        (VCP_GAMMA, 3),
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
        (VCP_NIGHT_VISION, 0),
        (VCP_DYNAMIC_OD, 0),
        (VCP_ADAPTIVE_SYNC, 0),
        (VCP_GAME_RUSH, 1),
        (VCP_LOCAL_DIMMING, 1),
        (VCP_DYDS, 1),
        (VCP_FPS_COUNTER, 0),
        (VCP_CROSSHAIR, 0),
        (VCP_STOPWATCH, 0),
        (VCP_GAME_TIME, 0),
        (VCP_MAGNIFIER, 0),
        (VCP_HAWKEYE, 0),
        (VCP_CROSSHAIR_COLOR, 8),
        (VCP_VOLUME, 50),
        (VCP_MUTE, 1),
        (VCP_OUTPUT_RANGE, 1),
        (VCP_QUICK_BOOT, 1),
        (VCP_OSD_LANG, 0x02),
        (VCP_OSD_TIME, 10),
        (VCP_OSD_H_POS, 50),
        (VCP_OSD_V_POS, 50),
        (VCP_OSD_TRANS, 0),
        (VCP_POWER_SAVING, POWER_SAVING_VALUES[0]),
        (VCP_POWER_LED, 3),
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
                .filter(|&&c| c >= 32 && c <= 126)
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
                .filter(|&&c| c >= 32 && c <= 126)
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
        if let Ok(contents) = fs::read_to_string(path.join(file_name)) {
            if let Some(mode) = contents.lines().find(|line| !line.trim().is_empty()) {
                return mode.trim().to_string();
            }
        }
    }
    String::new()
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
        if let Ok(mut f) = File::open(&edid_path) {
            if f.read_to_end(&mut edid_data).is_ok() && edid_data.len() >= 128 {
                if let Some(name) = parse_edid_name(&edid_data) {
                    let resolution = read_drm_resolution(&path);
                    let serial_number = parse_edid_serial(&edid_data).unwrap_or_default();
                    eprintln!("[DRM] {} connected: {} [{}]", dir_str, name, resolution);
                    monitors.push(DrmMonitorInfo {
                        name,
                        resolution,
                        serial_number,
                        connector: dir_str.to_string(),
                    });
                }
            }
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

fn read_vcp(ddc: &mut MsiDdc, code: u8) -> Option<u16> {
    for attempt in 0..3 {
        thread::sleep(Duration::from_millis(40));
        match ddc.get_vcp(code) {
            Ok(v) => return Some(v),
            Err(e) => {
                let msg = format!("{}", e);
                if msg.contains("unsupported") {
                    return None;
                }
                eprintln!("[DDC] VCP 0x{:02X} attempt {}: {}", code, attempt + 1, e);
                if attempt < 2 {
                    thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }
    None
}

fn wake_ddc(ddc: &mut MsiDdc) {
    let _ = ddc.get_vcp(VCP_DDCCI_INIT);
    thread::sleep(Duration::from_millis(250));
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
    controller: String,
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
    if let Ok(data) = fs::read_to_string(&path) {
        if let Ok(store) = serde_json::from_str(&data) {
            return store;
        }
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

fn read_controller_type(ddc: &mut MsiDdc) -> Option<String> {
    let value = read_vcp(ddc, VCP_CONTROLLER_TYPE)?;
    let controller = match value {
        0x56 => "Mstar",
        _ => "Unknown",
    };
    Some(format!("{} (sl=0x05, sh=0x{:02X})", controller, value))
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
) -> Option<(MsiDdc, MonitorInfo)> {
    let _ = ui_tx.send_blocking(UiCmd::Progress(format!("Connecting to {}...", name)));

    let mut ddc = match MsiDdc::open(dev_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[DDC] {} open failed: {}, retrying...", dev_path, e);
            thread::sleep(Duration::from_millis(200));
            match MsiDdc::open(dev_path) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("[DDC] {} retry failed: {}", dev_path, e);
                    return None;
                }
            }
        }
    };

    wake_ddc(&mut ddc);

    // Verify DDC/CI works
    if read_vcp(&mut ddc, VCP_BRIGHTNESS).is_none() {
        wake_ddc(&mut ddc);
        if read_vcp(&mut ddc, VCP_BRIGHTNESS).is_none() {
            eprintln!("[DDC] {} DDC/CI test failed", dev_path);
            return None;
        }
    }

    eprintln!("[DDC] Monitor ready on {}: {}", dev_path, name);
    let cached_snapshot = load_cached_snapshot(name);
    let _ = ui_tx.send_blocking(UiCmd::Progress(if cached_snapshot.is_some() {
        format!("Using cached monitor info: {}...", name)
    } else {
        format!("Reading info: {}...", name)
    }));

    let hz = cached_snapshot
        .as_ref()
        .map(|snapshot| snapshot.hz)
        .unwrap_or_else(|| {
            read_vcp(&mut ddc, VCP_REFRESH_RATE)
                .map(|v| v / 100)
                .unwrap_or(0)
        });
    let fe = if cached_snapshot.is_some() {
        0
    } else {
        read_vcp(&mut ddc, VCP_FIRMWARE).unwrap_or(0)
    };
    let mins = read_vcp(&mut ddc, VCP_USAGE_TIME).unwrap_or(0);
    let input_source = if cached_snapshot.is_some() {
        None
    } else {
        read_vcp(&mut ddc, VCP_INPUT_SOURCE)
    };
    let picture_mode = read_vcp(&mut ddc, VCP_MODE);
    let osd_lang = if cached_snapshot.is_some() {
        None
    } else {
        read_vcp(&mut ddc, VCP_OSD_LANG)
    };

    let mut settings = load_cached_settings(name, ui_tx);
    if let Some(value) = input_source {
        settings.insert(VCP_INPUT_SOURCE, value);
    }
    if let Some(value) = picture_mode {
        settings.insert(VCP_MODE, value);
    }
    if let Some(value) = osd_lang {
        settings.insert(VCP_OSD_LANG, value);
    }
    apply_color_temp_cache(name, &mut settings);
    save_cache_for(name, &settings, Some(dev_path));

    let controller = cached_snapshot
        .as_ref()
        .map(|snapshot| snapshot.controller.clone())
        .or_else(|| read_controller_type(&mut ddc))
        .or_else(|| drm_info.map(|info| format!("{} / {}", info.connector, dev_path)))
        .unwrap_or_else(|| dev_path.to_string());
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
            controller,
            firm: info.firm.clone(),
            usage_mins: info.usage_mins,
        },
        Some(dev_path),
    );

    Some((ddc, info))
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
            eprintln!("[DRM] Target monitor: {}", name);
        }

        if let Some(ref cached_path) = cached_i2c_path {
            if Path::new(&cached_path).exists() {
                let cached_name = target_name
                    .clone()
                    .or_else(|| get_edid_name_retry(&cached_path, 1));
                if let Some(name) = cached_name {
                    let drm_info = drm_monitors.iter().find(|monitor| monitor.name == name);
                    if let Some((ddc, info)) = try_connect(&cached_path, &name, drm_info, &ui_tx) {
                        let _ = ui_tx.send_blocking(UiCmd::MonitorFound(info));
                        current_monitor_name = Some(name);
                        found = Some(ddc);
                    }
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
            eprintln!("[I2C] {} EDID: {}", p, name);

            if let Some(ref target) = target_name {
                if !name.contains("P275MV") && target.contains("P275MV") {
                    eprintln!("[I2C] {} skipping (not target)", p);
                    continue;
                }
            }

            let _ = ui_tx.send_blocking(UiCmd::Progress(format!("Found: {}...", name)));
            let drm_info = drm_monitors.iter().find(|monitor| monitor.name == name);
            if let Some((ddc, info)) = try_connect(&p, &name, drm_info, &ui_tx) {
                let _ = ui_tx.send_blocking(UiCmd::MonitorFound(info));
                current_monitor_name = Some(name.clone());
                found = Some(ddc);
                break;
            }
        }

        let mut ddc = match found {
            Some(d) => d,
            _ => {
                eprintln!("[DDC] No compatible monitor found");
                let _ = ui_tx.send_blocking(UiCmd::Error("Monitor not found".into()));
                return;
            }
        };

        let monitor_name = current_monitor_name.unwrap_or_else(|| "default".into());
        let mut cache = load_cache_for(&monitor_name);

        while let Ok(first_cmd) = worker_rx.recv() {
            let mut batch = vec![first_cmd];
            while let Ok(cmd) = worker_rx.try_recv() {
                batch.push(cmd);
            }

            let _ = ui_tx.send_blocking(UiCmd::Busy(true));

            for cmd in collapse_worker_cmds(batch) {
                let (code, value, result) = match cmd {
                    WorkerCmd::Set(c, v) => (c, v, ddc.set_vcp(c, v)),
                    WorkerCmd::SetSave(c, v) => (c, v, ddc.set_vcp_save(c, v)),
                };
                if result.is_err() {
                    if let Err(e) = result {
                        eprintln!("[DDC] VCP 0x{:02X} error: {}", code, e);
                    }
                    continue;
                }
                if code == VCP_RESET_FACTORY {
                    cache = factory_reset_defaults();
                    apply_color_temp_cache(&monitor_name, &mut cache);
                    clear_color_temp_profile_cache(&monitor_name);
                    save_cache_for(&monitor_name, &cache, None);
                    continue;
                }
                // successfully wrote value, update cache
                cache.insert(code, value);
                if let Some(cache_code) = cache_rgb_key_for_vcp_code(code) {
                    cache.insert(cache_code, value);
                }
                if code == VCP_COLOR_TEMP {
                    apply_color_temp_cache(&monitor_name, &mut cache);
                }
                if is_color_temp_rgb_code(code) {
                    persist_active_color_temp_profile(&monitor_name, &cache, None);
                }
                save_cache_for(&monitor_name, &cache, None);
            }

            let _ = ui_tx.send_blocking(UiCmd::Busy(false));
        }
    });
}
