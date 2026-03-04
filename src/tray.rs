use crate::i18n::{AppLang, tr};
use adw::prelude::*;
use gtk4::glib;
use ksni::blocking::TrayMethods;
use libadwaita as adw;

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

    fn icon_name(&self) -> String {
        "display-symbolic".into()
    }

    fn icon_theme_path(&self) -> String {
        if let Ok(mut path) = std::env::current_dir() {
            path.push("assets");
            path.push("icons");
            return path.to_string_lossy().into_owned();
        }
        String::new()
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
                icon_name: "document-properties-symbolic".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send_blocking(TrayAction::Show);
                }),
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            StandardItem {
                label: tr(&self.lang, "tray_quit"),
                icon_name: "application-exit-symbolic".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send_blocking(TrayAction::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

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
