use ddc::Ddc;
use std::sync::mpsc;
use std::thread;

pub struct MonitorInfo {
    pub name: String,
    pub manufacturer: String,
    pub language: String,
}

pub struct MonitorValues {
    pub info: MonitorInfo,
    pub brightness: u16,
    pub contrast: u16,
    pub sharpness: u16,
    pub volume: u16,
    pub mute: u16,
    pub dcr: u16,
    pub light_sensor: u16,
    pub low_blue_light: u16,
    pub gamma: u16,
    pub aspect_ratio: u16,
    pub color_temp: u16,
}

pub enum WorkerCmd {
    SetBrightness(u16),
    SetContrast(u16),
    SetSharpness(u16),
    SetVolume(u16),
    SetMute(bool),
    SetDcr(bool),
    SetLightSensor(bool),
    SetLowBlueLight(u16),
    SetGamma(u16),
    SetAspectRatio(u16),
    SetColorTemp(u16),
}

pub enum UiCmd {
    MonitorFound(MonitorValues),
    Error(String),
}

fn parse_model_name(caps: &str) -> String {
    if let Some(start) = caps.find("model(") {
        let rest = &caps[start + 6..];
        if let Some(end) = rest.find(')') {
            return rest[..end].to_string();
        }
    }
    "Nieznany Model".to_string()
}

fn parse_language(code: u16) -> String {
    match code {
        1 => "Angielski".into(),
        2 => "Francuski".into(),
        3 => "Niemiecki".into(),
        10 => "Polski".into(),
        _ => "Inny/Nieznany".into(),
    }
}

pub fn start_worker(worker_rx: mpsc::Receiver<WorkerCmd>, ui_tx: async_channel::Sender<UiCmd>) {
    thread::spawn(move || {
        let mut found_ddc = None;
        for i in 0..20 {
            let path = format!("/dev/i2c-{}", i);
            if let Ok(mut ddc) = ddc_i2c::from_i2c_device(&path) {
                if let Ok(b) = ddc.get_vcp_feature(0x10) {
                    let name = if let Ok(caps_raw) = ddc.capabilities_string() {
                        let caps_text = String::from_utf8_lossy(&caps_raw);
                        parse_model_name(&caps_text)
                    } else {
                        "Monitor I2C".into()
                    };

                    let lang_code = ddc.get_vcp_feature(0x68).map(|v| v.value()).unwrap_or(0);

                    let values = MonitorValues {
                        info: MonitorInfo {
                            name,
                            manufacturer: "Wykryty przez I2C".into(),
                            language: parse_language(lang_code),
                        },
                        brightness: b.value(),
                        contrast: ddc.get_vcp_feature(0x12).map(|v| v.value()).unwrap_or(0),
                        sharpness: ddc.get_vcp_feature(0x87).map(|v| v.value()).unwrap_or(0),
                        volume: ddc.get_vcp_feature(0x62).map(|v| v.value()).unwrap_or(0),
                        mute: ddc.get_vcp_feature(0x8D).map(|v| v.value()).unwrap_or(2),
                        dcr: ddc.get_vcp_feature(0x5E).map(|v| v.value()).unwrap_or(2),
                        light_sensor: ddc.get_vcp_feature(0x6E).map(|v| v.value()).unwrap_or(2),
                        low_blue_light: ddc.get_vcp_feature(0xE1).map(|v| v.value()).unwrap_or(0),
                        gamma: ddc.get_vcp_feature(0x72).map(|v| v.value()).unwrap_or(0),
                        aspect_ratio: ddc.get_vcp_feature(0xDA).map(|v| v.value()).unwrap_or(0),
                        color_temp: ddc.get_vcp_feature(0x14).map(|v| v.value()).unwrap_or(0),
                    };

                    let _ = ui_tx.send_blocking(UiCmd::MonitorFound(values));
                    found_ddc = Some(ddc);
                    break;
                }
            }
        }

        let mut ddc = match found_ddc {
            Some(d) => d,
            _ => {
                let _ = ui_tx.send_blocking(UiCmd::Error("Nie wykryto monitora!".into()));
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
                WorkerCmd::SetSharpness(v) => {
                    let _ = ddc.set_vcp_feature(0x87, v);
                }
                WorkerCmd::SetVolume(v) => {
                    let _ = ddc.set_vcp_feature(0x62, v);
                }
                WorkerCmd::SetMute(on) => {
                    let _ = ddc.set_vcp_feature(0x8D, if on { 1 } else { 2 });
                }
                WorkerCmd::SetDcr(on) => {
                    let _ = ddc.set_vcp_feature(0x5E, if on { 1 } else { 2 });
                }
                WorkerCmd::SetLightSensor(on) => {
                    let _ = ddc.set_vcp_feature(0x6E, if on { 1 } else { 2 });
                }
                WorkerCmd::SetLowBlueLight(v) => {
                    let _ = ddc.set_vcp_feature(0xE1, v);
                }
                WorkerCmd::SetGamma(v) => {
                    let _ = ddc.set_vcp_feature(0x72, v);
                }
                WorkerCmd::SetAspectRatio(v) => {
                    let _ = ddc.set_vcp_feature(0xDA, v);
                }
                WorkerCmd::SetColorTemp(v) => {
                    let _ = ddc.set_vcp_feature(0x14, v);
                }
            }
        }
    });
}
