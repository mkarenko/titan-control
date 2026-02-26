mod monitor;
mod ui;
use adw::prelude::*;
use libadwaita as adw;
use monitor::{UiCmd, WorkerCmd};
use std::sync::mpsc;

fn main() {
    let app = adw::Application::builder()
        .application_id("org.titan.MonitorControl")
        .build();
    app.connect_activate(build_application);
    app.run();
}

fn build_application(app: &adw::Application) {
    let (worker_tx, worker_rx) = mpsc::channel::<WorkerCmd>();
    let (ui_tx, ui_rx) = async_channel::unbounded::<UiCmd>();
    let widgets = ui::build_ui(app);
    monitor::start_worker(worker_rx, ui_tx);

    let sl = widgets.status_label.clone();
    let im = widgets.info_model.clone();
    let il = widgets.info_lang.clone();
    let sb = widgets.sb.clone();
    let sc = widgets.sc.clone();
    let ss = widgets.ss.clone();
    let sv = widgets.sv.clone();
    let slbl = widgets.slbl.clone();
    let sw_m = widgets.sw_mute.clone();
    let sw_d = widgets.sw_dcr.clone();
    let sw_s = widgets.sw_sensor.clone();
    let cg = widgets.combo_gamma.clone();
    let ca = widgets.combo_aspect.clone();
    let ct = widgets.combo_temp.clone();

    gtk4::glib::MainContext::default().spawn_local(async move {
        while let Ok(msg) = ui_rx.recv().await {
            match msg {
                UiCmd::MonitorFound(v) => {
                    sl.set_label("Połączono");
                    im.set_subtitle(&v.info.name);
                    il.set_subtitle(&v.info.language);
                    sb.set_value(v.brightness as f64);
                    sc.set_value(v.contrast as f64);
                    ss.set_value(v.sharpness as f64);
                    sv.set_value(v.volume as f64);
                    slbl.set_value(v.low_blue_light as f64);
                    sw_m.set_active(v.mute == 1);
                    sw_d.set_active(v.dcr == 1);
                    sw_s.set_active(v.light_sensor == 1);
                    // Uproszczone przypisanie indeksów
                    cg.set_selected(v.gamma as u32);
                    ca.set_selected(v.aspect_ratio as u32);
                    ct.set_selected(v.color_temp as u32);

                    // Odblokowanie wszystkich kontrolek
                    for widget in &[
                        sb.upcast_ref::<gtk4::Widget>(),
                        sc.upcast_ref(),
                        ss.upcast_ref(),
                        sv.upcast_ref(),
                        slbl.upcast_ref(),
                        sw_m.upcast_ref(),
                        sw_d.upcast_ref(),
                        sw_s.upcast_ref(),
                        cg.upcast_ref(),
                        ca.upcast_ref(),
                        ct.upcast_ref(),
                    ] {
                        widget.set_sensitive(true);
                    }
                }
                UiCmd::Error(err) => sl.set_label(&err),
            }
        }
    });

    // Podpięcie sygnałów
    let tx = worker_tx.clone();
    widgets.sb.connect_value_changed(move |s| {
        let _ = tx.send(WorkerCmd::SetBrightness(s.value() as u16));
    });
    let tx = worker_tx.clone();
    widgets.sc.connect_value_changed(move |s| {
        let _ = tx.send(WorkerCmd::SetContrast(s.value() as u16));
    });
    let tx = worker_tx.clone();
    widgets.ss.connect_value_changed(move |s| {
        let _ = tx.send(WorkerCmd::SetSharpness(s.value() as u16));
    });
    let tx = worker_tx.clone();
    widgets.sv.connect_value_changed(move |s| {
        let _ = tx.send(WorkerCmd::SetVolume(s.value() as u16));
    });
    let tx = worker_tx.clone();
    widgets.slbl.connect_value_changed(move |s| {
        let _ = tx.send(WorkerCmd::SetLowBlueLight(s.value() as u16));
    });

    let tx = worker_tx.clone();
    widgets.sw_mute.connect_state_set(move |_, state| {
        let _ = tx.send(WorkerCmd::SetMute(state));
        gtk4::glib::Propagation::Proceed
    });
    let tx = worker_tx.clone();
    widgets.sw_dcr.connect_state_set(move |_, state| {
        let _ = tx.send(WorkerCmd::SetDcr(state));
        gtk4::glib::Propagation::Proceed
    });
    let tx = worker_tx.clone();
    widgets.sw_sensor.connect_state_set(move |_, state| {
        let _ = tx.send(WorkerCmd::SetLightSensor(state));
        gtk4::glib::Propagation::Proceed
    });

    let tx = worker_tx.clone();
    widgets.combo_gamma.connect_selected_notify(move |c| {
        let _ = tx.send(WorkerCmd::SetGamma(c.selected() as u16));
    });
    let tx = worker_tx.clone();
    widgets.combo_aspect.connect_selected_notify(move |c| {
        let _ = tx.send(WorkerCmd::SetAspectRatio(c.selected() as u16));
    });
    let tx = worker_tx.clone();
    widgets.combo_temp.connect_selected_notify(move |c| {
        let _ = tx.send(WorkerCmd::SetColorTemp(c.selected() as u16));
    });

    widgets.window.present();
}
