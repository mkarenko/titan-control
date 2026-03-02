mod monitor;
mod ui;
use adw::prelude::*;
use gtk4::glib;
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

    let ui::MainWidgets {
        window,
        status_label,
        info_model,
        row_hz,
        row_ctrl,
        row_firm,
        row_usage,
        sb,
        sc,
        sv,
        slbl,
        sr,
        sg,
        sbl,
        combo_temp,
        combo_input,
        combo_lang,
        combo_mode,
        sw_mute,
        btn_off,
        btn_reset_factory,
        btn_reset_br_con,
        btn_reset_color,
        btn_custom,
        custom_revealer,
        ss,
        sce,
        scoe,
        ssr,
        sshb,
        c_hdr,
        c_gam,
        hue_scales,
        sat_scales,
        btn_ps_off,
        btn_ps_l1,
        btn_ps_l2,
        btn_pl_off,
        btn_pl_l1,
        btn_pl_l2,
        btn_pl_l3,
        ..
    } = widgets;

    monitor::start_worker(worker_rx, ui_tx);

    btn_custom.connect_toggled(glib::clone!(
        #[strong]
        custom_revealer,
        move |btn| {
            custom_revealer.set_reveal_child(btn.is_active());
        }
    ));

    combo_temp.connect_selected_notify(glib::clone!(
        #[strong]
        sr,
        #[strong]
        sg,
        #[strong]
        sbl,
        move |c| {
            let is_user = c.selected() >= 3;
            sr.set_sensitive(is_user);
            sg.set_sensitive(is_user);
            sbl.set_sensitive(is_user);
        }
    ));

    glib::MainContext::default().spawn_local(glib::clone!(
        #[strong]
        status_label,
        #[strong]
        info_model,
        #[strong]
        row_hz,
        #[strong]
        row_ctrl,
        #[strong]
        row_firm,
        #[strong]
        row_usage,
        #[strong]
        sb,
        #[strong]
        sc,
        #[strong]
        sv,
        #[strong]
        slbl,
        #[strong]
        sr,
        #[strong]
        sg,
        #[strong]
        sbl,
        #[strong]
        combo_temp,
        #[strong]
        combo_input,
        #[strong]
        combo_lang,
        #[strong]
        combo_mode,
        #[strong]
        sw_mute,
        #[strong]
        btn_off,
        #[strong]
        btn_reset_factory,
        #[strong]
        btn_reset_br_con,
        #[strong]
        btn_reset_color,
        #[strong]
        btn_ps_off,
        #[strong]
        btn_ps_l1,
        #[strong]
        btn_ps_l2,
        #[strong]
        btn_pl_off,
        #[strong]
        btn_pl_l1,
        #[strong]
        btn_pl_l2,
        #[strong]
        btn_pl_l3,
        async move {
            while let Ok(msg) = ui_rx.recv().await {
                match msg {
                    UiCmd::MonitorFound(v) => {
                        status_label.set_label("Gotowy");
                        info_model.set_subtitle(&v.name);
                        row_hz.set_subtitle(&format!("{} Hz", v.hz));
                        row_ctrl.set_subtitle(&v.ctrl);
                        row_firm.set_subtitle(&v.firm);
                        row_usage.set_subtitle(&format!(
                            "{} h {} min",
                            v.usage_mins / 60,
                            v.usage_mins % 60
                        ));
                        sb.set_value(v.brightness as f64);
                        sc.set_value(v.contrast as f64);
                        sv.set_value(v.volume as f64);
                        slbl.set_value(v.low_blue_light as f64);
                        sr.set_value(v.r as f64);
                        sg.set_value(v.g as f64);
                        sbl.set_value(v.b as f64);
                        sw_mute.set_active(v.mute == 1);
                        combo_mode.set_selected(v.mode as u32);
                        combo_temp.set_selected(monitor::vcp_to_combo_index(
                            &monitor::COLOR_TEMP_VALUES[..],
                            v.temp,
                        ));

                        let w_list = &[
                            sb.upcast_ref::<gtk4::Widget>(),
                            sc.upcast_ref(),
                            sv.upcast_ref(),
                            slbl.upcast_ref(),
                            sr.upcast_ref(),
                            sg.upcast_ref(),
                            sbl.upcast_ref(),
                            sw_mute.upcast_ref(),
                            combo_temp.upcast_ref(),
                            combo_input.upcast_ref(),
                            combo_lang.upcast_ref(),
                            combo_mode.upcast_ref(),
                            btn_off.upcast_ref(),
                            btn_reset_factory.upcast_ref(),
                            btn_reset_br_con.upcast_ref(),
                            btn_reset_color.upcast_ref(),
                            btn_ps_off.upcast_ref(),
                            btn_ps_l1.upcast_ref(),
                            btn_ps_l2.upcast_ref(),
                            btn_pl_off.upcast_ref(),
                            btn_pl_l1.upcast_ref(),
                            btn_pl_l2.upcast_ref(),
                            btn_pl_l3.upcast_ref(),
                        ];
                        for w in w_list {
                            w.set_sensitive(true);
                        }
                    }
                    UiCmd::Error(e) => status_label.set_label(&e),
                }
            }
        }
    ));

    let tx = worker_tx.clone();
    sb.connect_value_changed(glib::clone!(
        #[strong]
        tx,
        move |s| {
            let _ = tx.send(WorkerCmd::SetBrightness(s.value() as u16));
        }
    ));
    sc.connect_value_changed(glib::clone!(
        #[strong]
        tx,
        move |s| {
            let _ = tx.send(WorkerCmd::SetContrast(s.value() as u16));
        }
    ));
    sv.connect_value_changed(glib::clone!(
        #[strong]
        tx,
        move |s| {
            let _ = tx.send(WorkerCmd::SetVolume(s.value() as u16));
        }
    ));
    sw_mute.connect_state_set(glib::clone!(
        #[strong]
        tx,
        move |_, s| {
            let _ = tx.send(WorkerCmd::SetMute(s));
            glib::Propagation::Proceed
        }
    ));
    btn_ps_off.connect_toggled(glib::clone!(
        #[strong]
        tx,
        move |b| if b.is_active() {
            let _ = tx.send(WorkerCmd::SetPowerSaving(0));
        }
    ));
    btn_ps_l1.connect_toggled(glib::clone!(
        #[strong]
        tx,
        move |b| if b.is_active() {
            let _ = tx.send(WorkerCmd::SetPowerSaving(1));
        }
    ));
    btn_ps_l2.connect_toggled(glib::clone!(
        #[strong]
        tx,
        move |b| if b.is_active() {
            let _ = tx.send(WorkerCmd::SetPowerSaving(2));
        }
    ));

    btn_reset_factory.connect_clicked(glib::clone!(
        #[strong]
        worker_tx,
        move |_| {
            let _ = worker_tx.send(WorkerCmd::ResetFactory);
        }
    ));
    btn_off.connect_clicked(glib::clone!(
        #[strong]
        worker_tx,
        move |_| {
            let _ = worker_tx.send(WorkerCmd::PowerOff);
        }
    ));
    window.present();
}
