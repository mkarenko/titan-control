mod i18n;
mod monitor;
mod tray;
mod ui;

use adw::prelude::*;
use gtk4::{glib, Align, Orientation};
use i18n::{lang_from_index, tr, AppLang, LangUpdaters};
use libadwaita as adw;
use monitor::{UiCmd, WorkerCmd};
use std::cell::Cell;
use std::rc::Rc;
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

    let lang = i18n::detect_system_lang();
    let updaters: LangUpdaters = Rc::new(std::cell::RefCell::new(Vec::new()));

    // Okno inicjalizacji
    let splash_spinner = gtk4::Spinner::builder()
        .spinning(true)
        .width_request(48)
        .height_request(48)
        .build();
    let splash_label = gtk4::Label::new(Some(&tr(&lang, "splash_searching")));
    let splash_box = gtk4::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(16)
        .halign(Align::Center)
        .valign(Align::Center)
        .vexpand(true)
        .build();
    splash_box.append(&splash_spinner);
    splash_box.append(&splash_label);
    let splash = adw::Window::builder()
        .title("Titan Control")
        .default_width(300)
        .default_height(200)
        .content(&splash_box)
        .application(app)
        .build();
    splash.present();

    let widgets = ui::build_ui(app, &lang, &updaters);

    // Language switching
    let lang_cell = Rc::new(Cell::new(lang));
    {
        let updaters_clone = updaters.clone();
        let lang_cell_clone = lang_cell.clone();
        widgets
            .combo_lang
            .connect_selected_notify(move |c| {
                let new_lang = lang_from_index(c.selected());
                lang_cell_clone.set(new_lang);
                for cb in updaters_clone.borrow().iter() {
                    cb(&new_lang);
                }
            });
    }

    // DESTRUKTURYZACJA z użyciem czytelnych nazw
    let ui::MainWidgets {
        window,
        combo_lang: _,
        // Informacje (System)
        row_info_model,
        row_info_hz,
        row_info_controller,
        row_info_firmware,
        row_info_usage,
        // Audio (System)
        scale_audio_volume,
        switch_audio_mute,
        // Zasilanie (System)
        button_power_off,
        button_power_save_off,
        button_power_save_lvl1,
        button_power_save_lvl2,
        button_led_off,
        button_led_lvl1,
        button_led_lvl2,
        button_led_lvl3,
        // I/O & OSD (System)
        combo_input_source,
        combo_output_range,
        switch_quick_boot,
        combo_osd_language,
        scale_osd_time,
        scale_osd_h_position,
        scale_osd_v_position,
        scale_osd_transparency,
        // Resety (System)
        button_reset_factory,
        button_reset_brightness_contrast,
        button_reset_color,
        // Profile (Profile)
        combo_picture_mode,
        button_profile_default,
        button_profile_custom,
        custom_revealer,
        // Ustawienia Ręczne (Profile -> Custom)
        scale_custom_brightness,
        scale_custom_contrast,
        scale_custom_sharpness,
        scale_custom_shadow_balance,
        scale_custom_cr_enhance,
        scale_custom_color_enhance,
        scale_custom_super_res,
        scale_custom_low_blue_light,
        // Kolory (Profile -> Custom)
        combo_custom_color_temp,
        scale_custom_red_gain,
        scale_custom_green_gain,
        scale_custom_blue_gain,
        // Listy (Profile -> Custom)
        combo_custom_hdr,
        combo_custom_gamma,
        hue_scales_vector,
        saturation_scales_vector,
        ..
    } = widgets;

    monitor::start_worker(worker_rx, ui_tx);

    // --- LOGIKA LOKALNA UI ---

    // Rozwijanie sekcji Custom
    button_profile_custom.connect_toggled(glib::clone!(
        #[strong]
        custom_revealer,
        move |btn| {
            custom_revealer.set_reveal_child(btn.is_active());
        }
    ));

    // Blokowanie suwaków RGB w zależności od temperatury
    combo_custom_color_temp.connect_selected_notify(glib::clone!(
        #[strong]
        scale_custom_red_gain,
        #[strong]
        scale_custom_green_gain,
        #[strong]
        scale_custom_blue_gain,
        move |c| {
            let is_user_mode = c.selected() >= 3; // User 1, 2, 3
            scale_custom_red_gain.set_sensitive(is_user_mode);
            scale_custom_green_gain.set_sensitive(is_user_mode);
            scale_custom_blue_gain.set_sensitive(is_user_mode);
        }
    ));

    // --- KOMUNIKACJA MONITOR -> UI (Odbieranie danych) ---
    glib::MainContext::default().spawn_local(glib::clone!(
        #[strong]
        splash,
        #[strong]
        splash_spinner,
        #[strong]
        splash_label,
        #[strong]
        window,
        #[strong]
        row_info_model,
        #[strong]
        row_info_hz,
        #[strong]
        row_info_controller,
        #[strong]
        row_info_firmware,
        #[strong]
        row_info_usage,
        #[strong]
        scale_custom_brightness,
        #[strong]
        scale_custom_contrast,
        #[strong]
        scale_audio_volume,
        #[strong]
        switch_audio_mute,
        #[strong]
        combo_picture_mode,
        #[strong]
        combo_custom_color_temp,
        #[strong]
        scale_custom_red_gain,
        #[strong]
        scale_custom_green_gain,
        #[strong]
        scale_custom_blue_gain,
        #[strong]
        scale_custom_sharpness,
        #[strong]
        scale_custom_shadow_balance,
        #[strong]
        scale_custom_cr_enhance,
        #[strong]
        scale_custom_color_enhance,
        #[strong]
        scale_custom_super_res,
        #[strong]
        scale_custom_low_blue_light,
        #[strong]
        combo_custom_hdr,
        async move {
            while let Ok(msg) = ui_rx.recv().await {
                match msg {
                    UiCmd::MonitorFound(v) => {
                        splash.close();
                        window.present();

                        // Dane techniczne
                        row_info_model.set_subtitle(&v.name);
                        row_info_hz.set_subtitle(&format!("{} Hz", v.hz));
                        row_info_controller.set_subtitle(&v.ctrl);
                        row_info_firmware.set_subtitle(&v.firm);
                        row_info_usage.set_subtitle(&format!(
                            "{} h {} min",
                            v.usage_mins / 60,
                            v.usage_mins % 60
                        ));

                        // Wartości suwaków
                        scale_custom_brightness.set_value(v.brightness as f64);
                        scale_custom_contrast.set_value(v.contrast as f64);
                        scale_audio_volume.set_value(v.volume as f64);
                        scale_custom_low_blue_light.set_value(v.low_blue_light as f64);
                        scale_custom_red_gain.set_value(v.r as f64);
                        scale_custom_green_gain.set_value(v.g as f64);
                        scale_custom_blue_gain.set_value(v.b as f64);
                        switch_audio_mute.set_active(v.mute == 1);

                        // Listy rozwijane
                        combo_picture_mode.set_selected(v.mode as u32);
                        combo_custom_color_temp.set_selected(monitor::vcp_to_combo_index(
                            &monitor::COLOR_TEMP_VALUES[..],
                            v.temp,
                        ));

                        // Ustawienia zaawansowane
                        scale_custom_sharpness.set_value(v.sharpness as f64);
                        scale_custom_cr_enhance.set_value(v.cr_enhance as f64);
                        scale_custom_color_enhance.set_value(v.color_enhance as f64);
                        scale_custom_super_res.set_value(v.super_res as f64);
                        scale_custom_shadow_balance.set_value(v.shadow_bal as f64);
                        combo_custom_hdr.set_selected(v.hdr as u32);
                    }
                    UiCmd::Error(e) => {
                        splash_spinner.set_spinning(false);
                        splash_label.set_label(&e);
                    }
                }
            }
        }
    ));

    // --- SYGNAŁY UI -> WORKER (Wysyłanie danych) ---
    let tx = worker_tx.clone();

    // System / Main
    scale_audio_volume.connect_value_changed(glib::clone!(
        #[strong]
        tx,
        move |s| {
            let _ = tx.send(WorkerCmd::SetVolume(s.value() as u16));
        }
    ));
    switch_audio_mute.connect_state_set(glib::clone!(
        #[strong]
        tx,
        move |_, s| {
            let _ = tx.send(WorkerCmd::SetMute(s));
            glib::Propagation::Proceed
        }
    ));

    // Power Saving Buttons
    button_power_save_off.connect_toggled(glib::clone!(
        #[strong]
        tx,
        move |b| if b.is_active() {
            let _ = tx.send(WorkerCmd::SetPowerSaving(0));
        }
    ));
    button_power_save_lvl1.connect_toggled(glib::clone!(
        #[strong]
        tx,
        move |b| if b.is_active() {
            let _ = tx.send(WorkerCmd::SetPowerSaving(1));
        }
    ));
    button_power_save_lvl2.connect_toggled(glib::clone!(
        #[strong]
        tx,
        move |b| if b.is_active() {
            let _ = tx.send(WorkerCmd::SetPowerSaving(2));
        }
    ));

    // Profile / Custom
    scale_custom_brightness.connect_value_changed(glib::clone!(
        #[strong]
        tx,
        move |s| {
            let _ = tx.send(WorkerCmd::SetBrightness(s.value() as u16));
        }
    ));
    scale_custom_contrast.connect_value_changed(glib::clone!(
        #[strong]
        tx,
        move |s| {
            let _ = tx.send(WorkerCmd::SetContrast(s.value() as u16));
        }
    ));
    scale_custom_sharpness.connect_value_changed(glib::clone!(
        #[strong]
        tx,
        move |s| {
            let _ = tx.send(WorkerCmd::SetSharpness(s.value() as u16));
        }
    ));

    // Hue / Saturation (Pętle po wektorach)
    for (idx, s) in hue_scales_vector.into_iter().enumerate() {
        let tx_c = tx.clone();
        s.connect_value_changed(move |sc| {
            let _ = tx_c.send(WorkerCmd::SetHue(idx, sc.value() as u16));
        });
    }
    for (idx, s) in saturation_scales_vector.into_iter().enumerate() {
        let tx_c = tx.clone();
        s.connect_value_changed(move |sc| {
            let _ = tx_c.send(WorkerCmd::SetSaturation(idx, sc.value() as u16));
        });
    }

    // Przyciski akcji
    button_reset_factory.connect_clicked(glib::clone!(
        #[strong]
        tx,
        move |_| {
            let _ = tx.send(WorkerCmd::ResetFactory);
        }
    ));
    button_power_off.connect_clicked(glib::clone!(
        #[strong]
        tx,
        move |_| {
            let _ = tx.send(WorkerCmd::PowerOff);
        }
    ));

    // Minimalizacja do tray zamiast zamykania
    window.connect_close_request(glib::clone!(
        #[strong]
        window,
        move |_| {
            window.set_visible(false);
            glib::Propagation::Stop
        }
    ));

    let hold_guard = app.hold();
    tray::setup_tray(app, &window, hold_guard, lang);
}
