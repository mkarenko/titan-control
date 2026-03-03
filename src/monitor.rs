use ddc::Ddc;
use i2c_linux::I2c;
use std::{fs::File, io::Read, sync::mpsc, thread};

pub struct MonitorValues {
    pub name: String,
    pub brightness: u16,
    pub contrast: u16,
    pub volume: u16,
    pub low_blue_light: u16,
    pub r: u16,
    pub g: u16,
    pub b: u16,
    pub temp: u16,
    pub input: u16,
    pub lang: u16,
    pub mute: u16,
    pub mode: u16,
    pub hz: u16,
    pub ctrl: String,
    pub firm: String,
    pub usage_mins: u16,
    pub sharpness: u16,
    pub cr_enhance: u16,
    pub color_enhance: u16,
    pub super_res: u16,
    pub shadow_bal: u16,
    pub hdr: u16,
    pub gamma: u16,
}

pub enum WorkerCmd {
    SetBrightness(u16),
    SetContrast(u16),
    SetVolume(u16),
    SetLowBlueLight(u16),
    SetR(u16),
    SetG(u16),
    SetB(u16),
    SetTemp(u16),
    SetInput(u16),
    SetLang(u16),
    SetMute(bool),
    SetMode(u16),
    SetSharpness(u16),
    SetCREnhance(u16),
    SetColorEnhance(u16),
    SetSuperRes(u16),
    SetShadowBalance(u16),
    SetHDR(u16),
    SetPowerSaving(u16),
    SetPowerLed(u16),
    SetOsdHPos(u16),
    SetOsdVPos(u16),
    ResetFactory,
    ResetBC,
    ResetColor,
    PowerOff,
    SetHue(usize, u16),
    SetSaturation(usize, u16),
}

pub enum UiCmd {
    MonitorFound(MonitorValues),
    Error(String),
}

pub const COLOR_TEMP_VALUES: [u16; 6] = [0x05, 0x06, 0x08, 0x0b, 0x0c, 0x0d];
pub const INPUT_SOURCE_VALUES: [u16; 4] = [0x11, 0x12, 0x0f, 0x10];
pub const OSD_LANGUAGE_VALUES: [u16; 22] = [
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0c, 0x0d, 0x0e, 0x0f, 0x14, 0x16,
    0x17, 0x19, 0x1e, 0x23, 0x24, 0x25,
];
pub const OSD_LANGUAGE_NAMES: [&str; 22] = [
    "Chiński (H)",
    "Angielski",
    "Francuski",
    "Niemiecki",
    "Włoski",
    "Japoński",
    "Koreański",
    "Portugalski",
    "Rosyjski",
    "Hiszpański",
    "Turecki",
    "Chiński (U)",
    "Portugalski (B)",
    "Arabski",
    "Holenderski",
    "Fiński",
    "Grecki",
    "Hindi",
    "Polski",
    "Tajski",
    "Ukraiński",
    "Wietnam",
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

/// Try to read EDID from an i2c device. Returns Some(name) only if valid EDID header is found.
fn get_edid_name(path: &str) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut i2c = I2c::new(file);
    i2c.smbus_set_slave_address(0x50, false).ok()?;
    let mut data = [0u8; 128];
    i2c.read_exact(&mut data).ok()?;

    // Validate EDID magic header
    if data[0..8] != EDID_HEADER {
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

pub fn start_worker(worker_rx: mpsc::Receiver<WorkerCmd>, ui_tx: async_channel::Sender<UiCmd>) {
    thread::spawn(move || {
        let mut found = None;
        for i in 0..32 {
            let p = format!("/dev/i2c-{}", i);
            if !std::path::Path::new(&p).exists() {
                continue;
            }
            // Only accept devices with valid EDID (confirms it's a real monitor)
            let name = match get_edid_name(&p) {
                Some(n) => n,
                None => continue,
            };
            let mut ddc = match ddc_i2c::from_i2c_device(&p) {
                Ok(d) => d,
                Err(_) => continue,
            };
            let b = match ddc.get_vcp_feature(0x10) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let mins = ddc.get_vcp_feature(0xF3).map(|v| v.value()).unwrap_or(0);
            let fe = ddc.get_vcp_feature(0xFE).map(|v| v.value()).unwrap_or(0);
            let f7 = ddc.get_vcp_feature(0xF7).map(|v| v.value()).unwrap_or(0);
            let fd = ddc.get_vcp_feature(0xFD).map(|v| v.value()).unwrap_or(0);

            let vals = MonitorValues {
                name,
                brightness: b.value(),
                contrast: ddc.get_vcp_feature(0x12).map(|v| v.value()).unwrap_or(0),
                volume: ddc.get_vcp_feature(0x62).map(|v| v.value()).unwrap_or(0),
                mute: ddc.get_vcp_feature(0x8D).map(|v| v.value()).unwrap_or(2),
                low_blue_light: ddc.get_vcp_feature(0xE1).map(|v| v.value()).unwrap_or(0),
                r: ddc.get_vcp_feature(0x16).map(|v| v.value()).unwrap_or(0),
                g: ddc.get_vcp_feature(0x18).map(|v| v.value()).unwrap_or(0),
                b: ddc.get_vcp_feature(0x1A).map(|v| v.value()).unwrap_or(0),
                temp: ddc.get_vcp_feature(0x14).map(|v| v.value()).unwrap_or(0),
                input: ddc.get_vcp_feature(0x60).map(|v| v.value()).unwrap_or(0),
                lang: ddc.get_vcp_feature(0xCC).map(|v| v.value()).unwrap_or(0),
                mode: ddc.get_vcp_feature(0xE0).map(|v| v.value()).unwrap_or(0),
                hz: ddc
                    .get_vcp_feature(0xAE)
                    .map(|v| v.value() / 100)
                    .unwrap_or(0),
                usage_mins: mins,
                sharpness: ddc.get_vcp_feature(0x87).map(|v| v.value()).unwrap_or(0),
                cr_enhance: ddc.get_vcp_feature(0xE2).map(|v| v.value()).unwrap_or(0),
                color_enhance: ddc.get_vcp_feature(0xE3).map(|v| v.value()).unwrap_or(0),
                super_res: ddc.get_vcp_feature(0xE4).map(|v| v.value()).unwrap_or(0),
                shadow_bal: ddc.get_vcp_feature(0xE5).map(|v| v.value()).unwrap_or(0),
                hdr: ddc.get_vcp_feature(0xE6).map(|v| v.value()).unwrap_or(0),
                gamma: ddc.get_vcp_feature(0x72).map(|v| v.value()).unwrap_or(0),
                firm: format!("v {}.{}.{}", (fe >> 12) & 0xF, (fe >> 8) & 0xF, fe & 0xFF),
                ctrl: format!(
                    "NB{}{}-{:02X}",
                    (f7 >> 8) as u8 as char,
                    (f7 & 0xFF) as u8 as char,
                    fd
                ),
            };
            let _ = ui_tx.send_blocking(UiCmd::MonitorFound(vals));
            found = Some(ddc);
            break;
        }
        let mut ddc = match found {
            Some(d) => d,
            _ => {
                let _ = ui_tx.send_blocking(UiCmd::Error("Błąd I2C".into()));
                return;
            }
        };
        while let Ok(cmd) = worker_rx.recv() {
            match cmd {
                WorkerCmd::SetBrightness(v) => {
                    let _ = ddc.set_vcp_feature(0x10, v);
                }
                WorkerCmd::SetContrast(v) => {
                    let _ = ddc.set_vcp_feature(0x12, v);
                }
                WorkerCmd::SetVolume(v) => {
                    let _ = ddc.set_vcp_feature(0x62, v);
                }
                WorkerCmd::SetMute(on) => {
                    let _ = ddc.set_vcp_feature(0x8D, if on { 1 } else { 2 });
                }
                WorkerCmd::SetLowBlueLight(v) => {
                    let _ = ddc.set_vcp_feature(0xE1, v);
                }
                WorkerCmd::SetMode(v) => {
                    let _ = ddc.set_vcp_feature(0xE0, v);
                }
                WorkerCmd::SetPowerSaving(v) => {
                    let _ = ddc.set_vcp_feature(0xD6, v);
                }
                WorkerCmd::SetPowerLed(v) => {
                    let _ = ddc.set_vcp_feature(0xEB, v);
                }
                WorkerCmd::SetOsdHPos(v) => {
                    let _ = ddc.set_vcp_feature(0x20, v);
                }
                WorkerCmd::SetOsdVPos(v) => {
                    let _ = ddc.set_vcp_feature(0x30, v);
                }
                WorkerCmd::ResetFactory => {
                    let _ = ddc.set_vcp_feature(0x04, 1);
                }
                WorkerCmd::ResetBC => {
                    let _ = ddc.set_vcp_feature(0x05, 1);
                }
                WorkerCmd::ResetColor => {
                    let _ = ddc.set_vcp_feature(0x08, 1);
                }
                WorkerCmd::PowerOff => {
                    let _ = ddc.set_vcp_feature(0xD6, 0x05);
                }
                _ => {}
            }
        }
    });
}
