use i2c_linux::I2c;
use std::{
    collections::HashMap,
    fs,
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    path::Path,
    sync::mpsc,
    thread,
    time::Duration,
};

// DDC/CI protocol constants
const DDC_ADDR: u16 = 0x37;
const DDC_DEST: u8 = 0x6E;
const DDC_SRC: u8 = 0x51;

// --- Standard DDC/CI VCP codes ---
pub const VCP_BRIGHTNESS: u8 = 0x10;
pub const VCP_CONTRAST: u8 = 0x12;
pub const VCP_COLOR_TEMP: u8 = 0x14;
pub const VCP_RED: u8 = 0x16;
pub const VCP_GREEN: u8 = 0x18;
pub const VCP_BLUE: u8 = 0x1A;
pub const VCP_RESET_BC: u8 = 0x05;
pub const VCP_RESET_COLOR: u8 = 0x08;
pub const VCP_GAMMA: u8 = 0x72;
pub const VCP_SHARPNESS: u8 = 0x87;
pub const VCP_OSD_LANG: u8 = 0xCC;
pub const VCP_DPMS: u8 = 0xD6;
pub const VCP_MODE: u8 = 0xE0;
pub const VCP_LOW_BLUE: u8 = 0xE1;

// --- MSI Custom VCP codes: Game Aid ---
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

// --- MSI Custom VCP codes: Picture Enhancer ---
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

// --- MSI Custom VCP codes: System/OSD ---
pub const VCP_OUTPUT_RANGE: u8 = 0x60;
pub const VCP_QUICK_BOOT: u8 = 0x61;
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
pub const VCP_RESET_FACTORY: u8 = 0xC7;
pub const VCP_VOLUME: u8 = 0xF6;
pub const VCP_MUTE: u8 = 0xF7;

// Info-only codes
const VCP_REFRESH_RATE: u8 = 0xAE;
const VCP_USAGE_TIME: u8 = 0xF3;
const VCP_FIRMWARE: u8 = 0xFE;

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

    pub fn set_vcp(&mut self, code: u8, value: u16) -> io::Result<()> {
        let vh = (value >> 8) as u8;
        let vl = (value & 0xFF) as u8;
        let chk = DDC_DEST ^ DDC_SRC ^ 0x84 ^ 0x03 ^ code ^ vh ^ vl;
        let buf = [DDC_SRC, 0x84, 0x03, code, vh, vl, chk];
        self.i2c.write_all(&buf)?;
        thread::sleep(Duration::from_millis(50));
        Ok(())
    }

    pub fn set_vcp_save(&mut self, code: u8, value: u16) -> io::Result<()> {
        self.set_vcp(code, value)?;
        self.set_vcp(0x99, 0x00F6)
    }

    pub fn get_vcp(&mut self, code: u8) -> io::Result<u16> {
        let chk = DDC_DEST ^ DDC_SRC ^ 0x82 ^ 0x01 ^ code;
        let req = [DDC_SRC, 0x82, 0x01, code, chk];
        self.i2c.write_all(&req)?;
        thread::sleep(Duration::from_millis(50));

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

pub struct MonitorInfo {
    pub name: String,
    pub hz: u16,
    pub firm: String,
    pub usage_mins: u16,
    pub settings: HashMap<u8, u16>,
}

pub enum WorkerCmd {
    Set(u8, u16),
    SetSave(u8, u16),
}

pub enum UiCmd {
    MonitorFound(MonitorInfo),
    Progress(String),
    Error(String),
}

pub const COLOR_TEMP_VALUES: [u16; 6] = [0x05, 0x06, 0x08, 0x0b, 0x0c, 0x0d];
pub const OSD_LANGUAGE_KEYS: [&str; 22] = [
    "osd_lang_chinese_h",
    "osd_lang_english",
    "osd_lang_french",
    "osd_lang_german",
    "osd_lang_italian",
    "osd_lang_japanese",
    "osd_lang_korean",
    "osd_lang_portuguese",
    "osd_lang_russian",
    "osd_lang_spanish",
    "osd_lang_turkish",
    "osd_lang_chinese_t",
    "osd_lang_portuguese_br",
    "osd_lang_arabic",
    "osd_lang_dutch",
    "osd_lang_finnish",
    "osd_lang_greek",
    "osd_lang_hindi",
    "osd_lang_polish",
    "osd_lang_thai",
    "osd_lang_ukrainian",
    "osd_lang_vietnamese",
];
pub const PICTURE_MODE_NAMES: [&str; 15] = [
    "Standard",
    "DyDs/ULL FPS",
    "DyDs/LD",
    "RTS/RPG Mode",
    "FPS Mode",
    "MOBA Mode",
    "Movie Mode",
    "Reading Mode",
    "Night Mode",
    "Eye Care Mode",
    "Mac View Mode",
    "E-Book Mode",
    "sRGB Mode",
    "Adobe Mode",
    "DCI-P3 Mode",
];

pub fn vcp_to_combo_index(values: &[u16], vcp_val: u16) -> u32 {
    values.iter().position(|&v| v == vcp_val).unwrap_or(0) as u32
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
            return Some(format!("{} {}", mfg, n.trim()));
        }
    }
    Some(mfg)
}

fn get_edid_name(path: &str) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut i2c = I2c::new(file);
    i2c.smbus_set_slave_address(0x50, false).ok()?;
    let mut data = [0u8; 128];
    i2c.read_exact(&mut data).ok()?;
    parse_edid_name(&data[..])
}

fn get_drm_monitor_names() -> Vec<String> {
    let mut names = Vec::new();
    let drm_dir = Path::new("/sys/class/drm");
    let entries = match fs::read_dir(drm_dir) {
        Ok(e) => e,
        Err(_) => return names,
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
                    eprintln!("[DRM] {} connected: {}", dir_str, name);
                    names.push(name);
                }
            }
        }
    }
    names
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

fn try_connect(
    dev_path: &str,
    name: &str,
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

    // Verify DDC/CI works
    if ddc.get_vcp(VCP_BRIGHTNESS).is_err() {
        thread::sleep(Duration::from_millis(100));
        if ddc.get_vcp(VCP_BRIGHTNESS).is_err() {
            eprintln!("[DDC] {} DDC/CI test failed", dev_path);
            return None;
        }
    }

    eprintln!("[DDC] Monitor ready on {}: {}", dev_path, name);
    let _ = ui_tx.send_blocking(UiCmd::Progress(format!("Reading info: {}...", name)));

    let hz = read_vcp(&mut ddc, VCP_REFRESH_RATE)
        .map(|v| v / 100)
        .unwrap_or(0);
    let fe = read_vcp(&mut ddc, VCP_FIRMWARE).unwrap_or(0);
    let mins = read_vcp(&mut ddc, VCP_USAGE_TIME).unwrap_or(0);

    // Read all monitor settings
    let codes: Vec<u8> = vec![
        // Display tab
        VCP_BRIGHTNESS,
        VCP_CONTRAST,
        VCP_VOLUME,
        VCP_MUTE,
        VCP_OUTPUT_RANGE,
        VCP_QUICK_BOOT,
        VCP_OSD_LANG,
        VCP_OSD_TIME,
        VCP_OSD_H_POS,
        VCP_OSD_V_POS,
        VCP_OSD_TRANS,
        VCP_POWER_SAVING,
        VCP_POWER_LED,
        // Profile tab
        VCP_MODE,
        VCP_SHARPNESS,
        VCP_LOW_BLUE,
        VCP_SHADOW_BALANCE,
        VCP_CR_ENHANCE,
        VCP_COLOR_ENHANCE,
        VCP_SUPER_RES,
        VCP_COLOR_TEMP,
        VCP_RED,
        VCP_GREEN,
        VCP_BLUE,
        VCP_HDR,
        VCP_GAMMA,
        VCP_NIGHT_VISION,
        VCP_DYNAMIC_OD,
        // Gaming - Game Aid
        VCP_SCREEN_SIZE,
        VCP_FPS_COUNTER,
        VCP_FPS_POS,
        VCP_CROSSHAIR,
        VCP_CROSSHAIR_SHAPE,
        VCP_CROSSHAIR_COLOR,
        VCP_STOPWATCH,
        VCP_STOPWATCH_TIME,
        VCP_STOPWATCH_POS,
        VCP_GAME_TIME,
        VCP_GAME_TIME_VAL,
        VCP_GAME_TIME_POS,
        VCP_MAGNIFIER,
        VCP_MAGNIFIER_NV,
        VCP_MAGNIFIER_ZOOM,
        VCP_MAGNIFIER_SIZE,
        VCP_MAGNIFIER_POS,
        VCP_ALIGNMENT,
        VCP_HAWKEYE,
        VCP_HAWKEYE_SIZE,
        VCP_HAWKEYE_POS,
        VCP_HAWKEYE_LEVEL,
        // Gaming - Picture Enhance
        VCP_GAME_RUSH,
        VCP_LOCAL_DIMMING,
        VCP_DYDS,
        VCP_HALO_CONTROL,
    ];
    #[allow(clippy::needless_range_loop)]
    let mut settings = HashMap::new();
    for (i, &code) in codes.iter().enumerate() {
        if i % 10 == 0 {
            let _ = ui_tx.send_blocking(UiCmd::Progress(format!(
                "Reading settings... {}/{}",
                i,
                codes.len()
            )));
        }
        thread::sleep(Duration::from_millis(30));
        match ddc.get_vcp(code) {
            Ok(val) => {
                eprintln!("[DDC] VCP 0x{:02X} = {}", code, val);
                settings.insert(code, val);
            }
            Err(_) => {
                eprintln!("[DDC] VCP 0x{:02X} not available", code);
            }
        }
    }

    let info = MonitorInfo {
        name: name.to_string(),
        hz,
        firm: format!("v {}.{}.{}", (fe >> 12) & 0xF, (fe >> 8) & 0xF, fe & 0xFF),
        usage_mins: mins,
        settings,
    };

    Some((ddc, info))
}

pub fn start_worker(worker_rx: mpsc::Receiver<WorkerCmd>, ui_tx: async_channel::Sender<UiCmd>) {
    thread::spawn(move || {
        let mut found = None;

        let _ = ui_tx.send_blocking(UiCmd::Progress("Detecting monitors...".into()));
        let drm_names = get_drm_monitor_names();
        let target_name = drm_names.iter().find(|n| n.contains("P275MV")).cloned();
        if let Some(ref name) = target_name {
            eprintln!("[DRM] Target monitor: {}", name);
        }

        let _ = ui_tx.send_blocking(UiCmd::Progress("Searching for monitors...".into()));
        for i in 0..32 {
            let p = format!("/dev/i2c-{}", i);
            if !Path::new(&p).exists() {
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
            if let Some((ddc, info)) = try_connect(&p, &name, &ui_tx) {
                let _ = ui_tx.send_blocking(UiCmd::MonitorFound(info));
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

        while let Ok(cmd) = worker_rx.recv() {
            let (code, result) = match cmd {
                WorkerCmd::Set(c, v) => (c, ddc.set_vcp(c, v)),
                WorkerCmd::SetSave(c, v) => (c, ddc.set_vcp_save(c, v)),
            };
            if let Err(e) = result {
                eprintln!("[DDC] VCP 0x{:02X} error: {}", code, e);
            }
        }
    });
}
