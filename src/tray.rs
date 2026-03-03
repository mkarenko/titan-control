use crate::i18n::{tr, AppLang};
use adw::prelude::*;
use gtk4::glib;
use image::GenericImageView;
use ksni::blocking::TrayMethods;
use libadwaita as adw;
use std::path::Path;
use std::sync::LazyLock;

enum TrayAction {
    ToggleVisibility,
    Show,
    Quit,
}

struct TitanTray {
    tx: async_channel::Sender<TrayAction>,
    lang: AppLang,
}

impl ksni::Tray for TitanTray {
    fn id(&self) -> String {
        "org.titan.MonitorControl".into()
    }

    fn title(&self) -> String {
        "Titan Control".into()
    }

    fn category(&self) -> ksni::Category {
        ksni::Category::Hardware
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![ICON.clone()]
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: "Titan Control".into(),
            ..Default::default()
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.tx.send_blocking(TrayAction::ToggleVisibility);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        vec![
            StandardItem {
                label: tr(&self.lang, "tray_show"),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send_blocking(TrayAction::Show);
                }),
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            StandardItem {
                label: tr(&self.lang, "tray_quit"),
                icon_name: "application-exit".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send_blocking(TrayAction::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

static ICON: LazyLock<ksni::Icon> = LazyLock::new(|| {
    let path = Path::new("assets/icons/tray-light.png");
    if path.exists() {
        let img = image::open(path).expect("Nie można załadować ikony");
        let (width, height) = img.dimensions();
        let mut data = img.into_rgba8().into_vec();
        for pixel in data.chunks_exact_mut(4) {
            pixel.rotate_right(1);
        }
        ksni::Icon {
            width: width as i32,
            height: height as i32,
            data,
        }
    } else {
        let size = 32i32;
        let mut data = vec![0u8; (size * size * 4) as usize];
        let center = size as f32 / 2.0;
        let radius = center - 1.0;
        for y in 0..size {
            for x in 0..size {
                let idx = ((y * size + x) * 4) as usize;
                let dx = x as f32 - center;
                let dy = y as f32 - center;
                if dx * dx + dy * dy <= radius * radius {
                    data[idx] = 255; // A
                    data[idx + 1] = 80; // R
                    data[idx + 2] = 140; // G
                    data[idx + 3] = 240; // B
                }
            }
        }
        ksni::Icon {
            width: size,
            height: size,
            data,
        }
    }
});

pub fn setup_tray(
    app: &adw::Application,
    window: &adw::ApplicationWindow,
    hold_guard: gtk4::gio::ApplicationHoldGuard,
    lang: AppLang,
) {
    let (tx, rx) = async_channel::unbounded::<TrayAction>();

    let tray = TitanTray { tx, lang };
    let handle = tray.spawn().expect("Nie można utworzyć ikony tray");

    let window_clone = window.clone();
    let app_clone = app.clone();

    glib::MainContext::default().spawn_local(async move {
        let _hold = hold_guard;
        let _handle = handle;

        while let Ok(action) = rx.recv().await {
            match action {
                TrayAction::ToggleVisibility => {
                    if window_clone.is_visible() {
                        window_clone.set_visible(false);
                    } else {
                        window_clone.present();
                    }
                }
                TrayAction::Show => window_clone.present(),
                TrayAction::Quit => app_clone.quit(),
            }
        }
    });
}
