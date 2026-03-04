mod i18n;
mod monitor;
mod tray;
mod ui;

use adw::prelude::*;
use gtk4::{Align, Orientation, Scale, Switch, ToggleButton, glib};
use i18n::{LangUpdaters, lang_from_index, tr};
use libadwaita::{self as adw, ExpanderRow};
use monitor::WorkerCmd;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::mpsc;

// --- Wire helpers ---

fn wire_scale(scale: &Scale, tx: &mpsc::Sender<WorkerCmd>, code: u8, save: bool, init: &Rc<Cell<bool>>) {
    let tx = tx.clone();
    let init = init.clone();
    scale.connect_value_changed(move |s| {
        if init.get() { return; }
        let _ = tx.send(if save {
            WorkerCmd::SetSave(code, s.value() as u16)
        } else {
            WorkerCmd::Set(code, s.value() as u16)
        });
    });
}

fn wire_switch(sw: &Switch, tx: &mpsc::Sender<WorkerCmd>, code: u8, save: bool, init: &Rc<Cell<bool>>) {
    let tx = tx.clone();
    let init = init.clone();
    sw.connect_state_set(move |_, state| {
        if init.get() { return glib::Propagation::Proceed; }
        let val = if state { 1 } else { 0 };
        let _ = tx.send(if save {
            WorkerCmd::SetSave(code, val)
        } else {
            WorkerCmd::Set(code, val)
        });
        glib::Propagation::Proceed
    });
}

fn wire_combo(combo: &adw::ComboRow, tx: &mpsc::Sender<WorkerCmd>, code: u8, save: bool, init: &Rc<Cell<bool>>) {
    let tx = tx.clone();
    let init = init.clone();
    combo.connect_selected_notify(move |c| {
        if init.get() { return; }
        let _ = tx.send(if save {
            WorkerCmd::SetSave(code, c.selected() as u16)
        } else {
            WorkerCmd::Set(code, c.selected() as u16)
        });
    });
}

fn wire_toggles(buttons: &[ToggleButton], tx: &mpsc::Sender<WorkerCmd>, code: u8, save: bool, offset: u16, init: &Rc<Cell<bool>>) {
    for (i, btn) in buttons.iter().enumerate() {
        let tx = tx.clone();
        let init = init.clone();
        let val = offset + i as u16;
        btn.connect_toggled(move |b| {
            if init.get() { return; }
            if b.is_active() {
                let _ = tx.send(if save {
                    WorkerCmd::SetSave(code, val)
                } else {
                    WorkerCmd::Set(code, val)
                });
            }
        });
    }
}

fn wire_expander(exp: &ExpanderRow, tx: &mpsc::Sender<WorkerCmd>, code: u8, save: bool, init: &Rc<Cell<bool>>) {
    let tx = tx.clone();
    let init = init.clone();
    exp.connect_enable_expansion_notify(move |e| {
        if init.get() { return; }
        let val = if e.enables_expansion() { 1u16 } else { 0 };
        let _ = tx.send(if save {
            WorkerCmd::SetSave(code, val)
        } else {
            WorkerCmd::Set(code, val)
        });
    });
}

fn main() {
    let app = adw::Application::builder()
        .application_id("org.titan.MonitorControl")
        .build();
    app.connect_activate(build_application);
    app.run();
}

fn build_application(app: &adw::Application) {
    let (worker_tx, worker_rx) = mpsc::channel::<WorkerCmd>();
    let (ui_tx, ui_rx) = async_channel::unbounded::<monitor::UiCmd>();

    let lang = i18n::detect_system_lang();
    let updaters: LangUpdaters = Rc::new(std::cell::RefCell::new(Vec::new()));

    // Splash screen
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
        widgets.combo_lang.connect_selected_notify(move |c| {
            let new_lang = lang_from_index(c.selected());
            lang_cell_clone.set(new_lang);
            for cb in updaters_clone.borrow().iter() {
                cb(&new_lang);
            }
        });
    }

    // Theme switching
    widgets.combo_theme.connect_selected_notify(|c| {
        let scheme = match c.selected() {
            1 => adw::ColorScheme::ForceLight,
            2 => adw::ColorScheme::ForceDark,
            _ => adw::ColorScheme::Default,
        };
        adw::StyleManager::default().set_color_scheme(scheme);
    });

    // Destructure ALL widgets
    let ui::MainWidgets {
        window,
        combo_lang: _,
        combo_theme: _,
        // Display tab
        row_info_model,
        row_info_hz,
        row_info_controller: _,
        row_info_firmware,
        row_info_usage,
        scale_audio_volume,
        switch_audio_mute,
        button_power_off,
        button_power_save_off,
        button_power_save_lvl1,
        button_power_save_lvl2,
        button_led_off,
        button_led_lvl1,
        button_led_lvl2,
        button_led_lvl3,
        combo_input_source: _,
        combo_output_range,
        switch_quick_boot,
        combo_osd_language,
        scale_osd_time,
        scale_osd_h_position,
        scale_osd_v_position,
        scale_osd_transparency,
        button_reset_factory,
        button_reset_brightness_contrast,
        button_reset_color,
        // Profile tab
        combo_picture_mode,
        button_profile_default: _,
        button_profile_custom,
        custom_revealer,
        scale_custom_brightness,
        scale_custom_contrast,
        scale_custom_sharpness,
        scale_custom_shadow_balance,
        scale_custom_cr_enhance,
        scale_custom_color_enhance,
        scale_custom_super_res,
        scale_custom_low_blue_light,
        combo_custom_color_temp,
        scale_custom_red_gain,
        scale_custom_green_gain,
        scale_custom_blue_gain,
        combo_custom_hdr,
        combo_custom_gamma,
        combo_custom_night_vision,
        combo_custom_dynamic_od,
        hue_scales_vector: _,
        saturation_scales_vector: _,
        // Gaming tab
        row_screen_size: _,
        buttons_screen_size,
        expander_fps_counter,
        buttons_fps_pos,
        expander_crosshair,
        buttons_crosshair_shape,
        buttons_crosshair_color,
        expander_stopwatch,
        buttons_stopwatch_time,
        buttons_stopwatch_pos,
        expander_game_time,
        buttons_game_time_val,
        buttons_game_time_pos,
        expander_magnifier,
        switch_magnifier_night_vision,
        buttons_magnifier_zoom,
        buttons_magnifier_size,
        buttons_magnifier_pos,
        switch_alignment,
        expander_hawkeye,
        buttons_hawkeye_size,
        buttons_hawkeye_pos,
        buttons_hawkeye_lvl,
        switch_gaming_async: _,
        switch_gaming_rush,
        buttons_gaming_local_dimming,
        buttons_gaming_dyds,
        buttons_gaming_night_vision,
        buttons_gaming_dynamic_od,
        buttons_gaming_hdr,
        scale_gaming_color_enhance,
        scale_gaming_cr_enhance,
        scale_gaming_shadow_enhance,
        scale_gaming_super_res,
        scale_gaming_halo_control,
    } = widgets;

    monitor::start_worker(worker_rx, ui_tx);

    // --- LOCAL UI LOGIC ---

    button_profile_custom.connect_toggled(glib::clone!(
        #[strong]
        custom_revealer,
        move |btn| {
            custom_revealer.set_reveal_child(btn.is_active());
        }
    ));

    combo_custom_color_temp.connect_selected_notify(glib::clone!(
        #[strong]
        scale_custom_red_gain,
        #[strong]
        scale_custom_green_gain,
        #[strong]
        scale_custom_blue_gain,
        move |c| {
            let is_user_mode = c.selected() >= 3;
            scale_custom_red_gain.set_sensitive(is_user_mode);
            scale_custom_green_gain.set_sensitive(is_user_mode);
            scale_custom_blue_gain.set_sensitive(is_user_mode);
        }
    ));

    // --- MONITOR -> UI ---

    let init = Rc::new(Cell::new(true)); // block signals during UI init

    // Collect scale bindings: (Scale, VCP code)
    let scale_binds: Vec<(Scale, u8)> = vec![
        (scale_audio_volume.clone(), monitor::VCP_VOLUME),
        (scale_osd_time.clone(), monitor::VCP_OSD_TIME),
        (scale_osd_h_position.clone(), monitor::VCP_OSD_H_POS),
        (scale_osd_v_position.clone(), monitor::VCP_OSD_V_POS),
        (scale_osd_transparency.clone(), monitor::VCP_OSD_TRANS),
        (scale_custom_brightness.clone(), monitor::VCP_BRIGHTNESS),
        (scale_custom_contrast.clone(), monitor::VCP_CONTRAST),
        (scale_custom_sharpness.clone(), monitor::VCP_SHARPNESS),
        (scale_custom_shadow_balance.clone(), monitor::VCP_SHADOW_BALANCE),
        (scale_custom_cr_enhance.clone(), monitor::VCP_CR_ENHANCE),
        (scale_custom_color_enhance.clone(), monitor::VCP_COLOR_ENHANCE),
        (scale_custom_super_res.clone(), monitor::VCP_SUPER_RES),
        (scale_custom_low_blue_light.clone(), monitor::VCP_LOW_BLUE),
        (scale_custom_red_gain.clone(), monitor::VCP_RED),
        (scale_custom_green_gain.clone(), monitor::VCP_GREEN),
        (scale_custom_blue_gain.clone(), monitor::VCP_BLUE),
        (scale_gaming_color_enhance.clone(), monitor::VCP_COLOR_ENHANCE),
        (scale_gaming_cr_enhance.clone(), monitor::VCP_CR_ENHANCE),
        (scale_gaming_shadow_enhance.clone(), monitor::VCP_SHADOW_BALANCE),
        (scale_gaming_super_res.clone(), monitor::VCP_SUPER_RES),
        (scale_gaming_halo_control.clone(), monitor::VCP_HALO_CONTROL),
    ];

    // Collect combo bindings: (ComboRow, VCP code)
    let combo_binds: Vec<(adw::ComboRow, u8)> = vec![
        (combo_output_range.clone(), monitor::VCP_OUTPUT_RANGE),
        (combo_osd_language.clone(), monitor::VCP_OSD_LANG),
        (combo_picture_mode.clone(), monitor::VCP_MODE),
        (combo_custom_hdr.clone(), monitor::VCP_HDR),
        (combo_custom_gamma.clone(), monitor::VCP_GAMMA),
        (combo_custom_night_vision.clone(), monitor::VCP_NIGHT_VISION),
        (combo_custom_dynamic_od.clone(), monitor::VCP_DYNAMIC_OD),
    ];

    // Collect switch bindings: (Switch, VCP code)
    let switch_binds: Vec<(Switch, u8)> = vec![
        (switch_quick_boot.clone(), monitor::VCP_QUICK_BOOT),
        (switch_magnifier_night_vision.clone(), monitor::VCP_MAGNIFIER_NV),
        (switch_alignment.clone(), monitor::VCP_ALIGNMENT),
        (switch_gaming_rush.clone(), monitor::VCP_GAME_RUSH),
    ];

    // Collect expander bindings: (ExpanderRow, VCP code)
    let expander_binds: Vec<(ExpanderRow, u8)> = vec![
        (expander_fps_counter.clone(), monitor::VCP_FPS_COUNTER),
        (expander_crosshair.clone(), monitor::VCP_CROSSHAIR),
        (expander_stopwatch.clone(), monitor::VCP_STOPWATCH),
        (expander_game_time.clone(), monitor::VCP_GAME_TIME),
        (expander_magnifier.clone(), monitor::VCP_MAGNIFIER),
        (expander_hawkeye.clone(), monitor::VCP_HAWKEYE),
    ];

    // Collect toggle bindings: (buttons, VCP code, offset)
    let toggle_binds: Vec<(Vec<ToggleButton>, u8, u16)> = vec![
        (vec![button_power_save_off.clone(), button_power_save_lvl1.clone(), button_power_save_lvl2.clone()], monitor::VCP_POWER_SAVING, 0),
        (vec![button_led_off.clone(), button_led_lvl1.clone(), button_led_lvl2.clone(), button_led_lvl3.clone()], monitor::VCP_POWER_LED, 1),
        (buttons_screen_size.clone(), monitor::VCP_SCREEN_SIZE, 0),
        (buttons_fps_pos.clone(), monitor::VCP_FPS_POS, 0),
        (buttons_crosshair_shape.clone(), monitor::VCP_CROSSHAIR_SHAPE, 0),
        (buttons_crosshair_color.clone(), monitor::VCP_CROSSHAIR_COLOR, 0),
        (buttons_stopwatch_time.clone(), monitor::VCP_STOPWATCH_TIME, 0),
        (buttons_stopwatch_pos.clone(), monitor::VCP_STOPWATCH_POS, 0),
        (buttons_game_time_val.clone(), monitor::VCP_GAME_TIME_VAL, 0),
        (buttons_game_time_pos.clone(), monitor::VCP_GAME_TIME_POS, 0),
        (buttons_magnifier_zoom.clone(), monitor::VCP_MAGNIFIER_ZOOM, 0),
        (buttons_magnifier_size.clone(), monitor::VCP_MAGNIFIER_SIZE, 0),
        (buttons_magnifier_pos.clone(), monitor::VCP_MAGNIFIER_POS, 0),
        (buttons_hawkeye_size.clone(), monitor::VCP_HAWKEYE_SIZE, 0),
        (buttons_hawkeye_pos.clone(), monitor::VCP_HAWKEYE_POS, 0),
        (buttons_hawkeye_lvl.clone(), monitor::VCP_HAWKEYE_LEVEL, 0),
        (buttons_gaming_local_dimming.clone(), monitor::VCP_LOCAL_DIMMING, 0),
        (buttons_gaming_dyds.clone(), monitor::VCP_DYDS, 0),
        (buttons_gaming_night_vision.clone(), monitor::VCP_NIGHT_VISION, 0),
        (buttons_gaming_dynamic_od.clone(), monitor::VCP_DYNAMIC_OD, 0),
        (buttons_gaming_hdr.clone(), monitor::VCP_HDR, 0),
    ];

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
        row_info_firmware,
        #[strong]
        row_info_usage,
        #[strong]
        switch_audio_mute,
        #[strong]
        combo_custom_color_temp,
        #[strong]
        init,
        async move {
            while let Ok(msg) = ui_rx.recv().await {
                match msg {
                    monitor::UiCmd::MonitorFound(info) => {
                        let s = &info.settings;

                        // Info rows
                        row_info_model.set_subtitle(&info.name);
                        row_info_hz.set_subtitle(&format!("{} Hz", info.hz));
                        row_info_firmware.set_subtitle(&info.firm);
                        row_info_usage.set_subtitle(&format!(
                            "{} h {} min",
                            info.usage_mins / 60,
                            info.usage_mins % 60
                        ));

                        // Scales
                        for (scale, code) in &scale_binds {
                            if let Some(&val) = s.get(code) {
                                scale.set_value(val as f64);
                            }
                        }

                        // Combos
                        for (combo, code) in &combo_binds {
                            if let Some(&val) = s.get(code) {
                                combo.set_selected(val as u32);
                            }
                        }

                        // Color temp (special mapping)
                        if let Some(&val) = s.get(&monitor::VCP_COLOR_TEMP) {
                            combo_custom_color_temp.set_selected(
                                monitor::vcp_to_combo_index(&monitor::COLOR_TEMP_VALUES[..], val),
                            );
                        }

                        // Switches
                        for (sw, code) in &switch_binds {
                            if let Some(&val) = s.get(code) {
                                sw.set_active(val != 0);
                            }
                        }

                        // Mute (special: 2=on, 1=off)
                        if let Some(&val) = s.get(&monitor::VCP_MUTE) {
                            switch_audio_mute.set_active(val == 2);
                        }

                        // Expanders
                        for (exp, code) in &expander_binds {
                            if let Some(&val) = s.get(code) {
                                exp.set_enable_expansion(val != 0);
                            }
                        }

                        // Toggles
                        for (btns, code, offset) in &toggle_binds {
                            if let Some(&val) = s.get(code) {
                                let idx = (val as i32) - (*offset as i32);
                                if idx >= 0 && (idx as usize) < btns.len() {
                                    btns[idx as usize].set_active(true);
                                }
                            }
                        }

                        // Done - unblock signals
                        init.set(false);
                        splash.close();
                        window.present();
                    }
                    monitor::UiCmd::Progress(msg) => splash_label.set_label(&msg),
                    monitor::UiCmd::Error(e) => {
                        splash_spinner.set_spinning(false);
                        splash_label.set_label(&e);
                    }
                }
            }
        }
    ));

    // --- UI -> WORKER ---

    let tx = worker_tx.clone();

    // ===== DISPLAY TAB =====

    // Audio
    wire_scale(&scale_audio_volume, &tx, monitor::VCP_VOLUME, true, &init);
    {
        let tx = tx.clone();
        let init = init.clone();
        switch_audio_mute.connect_state_set(move |_, state| {
            if init.get() { return glib::Propagation::Proceed; }
            let val = if state { 2 } else { 1 }; // MSI: enabled=2, disabled=1
            let _ = tx.send(WorkerCmd::SetSave(monitor::VCP_MUTE, val));
            glib::Propagation::Proceed
        });
    }

    // I/O & OSD
    wire_combo(&combo_output_range, &tx, monitor::VCP_OUTPUT_RANGE, true, &init);
    wire_switch(&switch_quick_boot, &tx, monitor::VCP_QUICK_BOOT, true, &init);
    wire_combo(&combo_osd_language, &tx, monitor::VCP_OSD_LANG, false, &init);
    wire_scale(&scale_osd_time, &tx, monitor::VCP_OSD_TIME, true, &init);
    wire_scale(&scale_osd_h_position, &tx, monitor::VCP_OSD_H_POS, true, &init);
    wire_scale(&scale_osd_v_position, &tx, monitor::VCP_OSD_V_POS, true, &init);
    wire_scale(&scale_osd_transparency, &tx, monitor::VCP_OSD_TRANS, true, &init);

    // Power Saving (Off=0, Lvl1=1, Lvl2=2)
    {
        let ps = vec![button_power_save_off.clone(), button_power_save_lvl1.clone(), button_power_save_lvl2.clone()];
        wire_toggles(&ps, &tx, monitor::VCP_POWER_SAVING, true, 0, &init);
    }

    // Power LED (Off=1, Lvl1=2, Lvl2=3, Lvl3=4)
    {
        let pl = vec![button_led_off.clone(), button_led_lvl1.clone(), button_led_lvl2.clone(), button_led_lvl3.clone()];
        wire_toggles(&pl, &tx, monitor::VCP_POWER_LED, true, 1, &init);
    }

    // Resets & Power Off
    {
        let tx = tx.clone();
        button_power_off.connect_clicked(move |_| {
            let _ = tx.send(WorkerCmd::Set(monitor::VCP_DPMS, 0x05));
        });
    }
    {
        let tx = tx.clone();
        button_reset_factory.connect_clicked(move |_| {
            let _ = tx.send(WorkerCmd::SetSave(monitor::VCP_RESET_FACTORY, 1));
        });
    }
    {
        let tx = tx.clone();
        button_reset_brightness_contrast.connect_clicked(move |_| {
            let _ = tx.send(WorkerCmd::Set(monitor::VCP_RESET_BC, 1));
        });
    }
    {
        let tx = tx.clone();
        button_reset_color.connect_clicked(move |_| {
            let _ = tx.send(WorkerCmd::Set(monitor::VCP_RESET_COLOR, 1));
        });
    }

    // ===== PROFILE TAB =====

    wire_combo(&combo_picture_mode, &tx, monitor::VCP_MODE, false, &init);
    wire_scale(&scale_custom_brightness, &tx, monitor::VCP_BRIGHTNESS, false, &init);
    wire_scale(&scale_custom_contrast, &tx, monitor::VCP_CONTRAST, false, &init);
    wire_scale(&scale_custom_sharpness, &tx, monitor::VCP_SHARPNESS, false, &init);
    wire_scale(&scale_custom_shadow_balance, &tx, monitor::VCP_SHADOW_BALANCE, true, &init);
    wire_scale(&scale_custom_cr_enhance, &tx, monitor::VCP_CR_ENHANCE, true, &init);
    wire_scale(&scale_custom_color_enhance, &tx, monitor::VCP_COLOR_ENHANCE, true, &init);
    wire_scale(&scale_custom_super_res, &tx, monitor::VCP_SUPER_RES, true, &init);
    wire_scale(&scale_custom_low_blue_light, &tx, monitor::VCP_LOW_BLUE, false, &init);

    // Color temperature (special mapping)
    {
        let tx = tx.clone();
        let init_c = init.clone();
        combo_custom_color_temp.connect_selected_notify(move |c| {
            if init_c.get() { return; }
            let idx = c.selected() as usize;
            if idx < monitor::COLOR_TEMP_VALUES.len() {
                let _ = tx.send(WorkerCmd::Set(
                    monitor::VCP_COLOR_TEMP,
                    monitor::COLOR_TEMP_VALUES[idx],
                ));
            }
        });
    }

    wire_scale(&scale_custom_red_gain, &tx, monitor::VCP_RED, false, &init);
    wire_scale(&scale_custom_green_gain, &tx, monitor::VCP_GREEN, false, &init);
    wire_scale(&scale_custom_blue_gain, &tx, monitor::VCP_BLUE, false, &init);
    wire_combo(&combo_custom_hdr, &tx, monitor::VCP_HDR, true, &init);
    wire_combo(&combo_custom_gamma, &tx, monitor::VCP_GAMMA, false, &init);
    wire_combo(&combo_custom_night_vision, &tx, monitor::VCP_NIGHT_VISION, true, &init);
    wire_combo(&combo_custom_dynamic_od, &tx, monitor::VCP_DYNAMIC_OD, true, &init);

    // ===== GAMING TAB =====

    // Game Aid
    wire_toggles(&buttons_screen_size, &tx, monitor::VCP_SCREEN_SIZE, true, 0, &init);
    wire_expander(&expander_fps_counter, &tx, monitor::VCP_FPS_COUNTER, true, &init);
    wire_toggles(&buttons_fps_pos, &tx, monitor::VCP_FPS_POS, true, 0, &init);
    wire_expander(&expander_crosshair, &tx, monitor::VCP_CROSSHAIR, true, &init);
    wire_toggles(&buttons_crosshair_shape, &tx, monitor::VCP_CROSSHAIR_SHAPE, true, 0, &init);
    wire_toggles(&buttons_crosshair_color, &tx, monitor::VCP_CROSSHAIR_COLOR, true, 0, &init);
    wire_expander(&expander_stopwatch, &tx, monitor::VCP_STOPWATCH, true, &init);
    wire_toggles(&buttons_stopwatch_time, &tx, monitor::VCP_STOPWATCH_TIME, true, 0, &init);
    wire_toggles(&buttons_stopwatch_pos, &tx, monitor::VCP_STOPWATCH_POS, true, 0, &init);
    wire_expander(&expander_game_time, &tx, monitor::VCP_GAME_TIME, true, &init);
    wire_toggles(&buttons_game_time_val, &tx, monitor::VCP_GAME_TIME_VAL, true, 0, &init);
    wire_toggles(&buttons_game_time_pos, &tx, monitor::VCP_GAME_TIME_POS, true, 0, &init);
    wire_expander(&expander_magnifier, &tx, monitor::VCP_MAGNIFIER, true, &init);
    wire_switch(&switch_magnifier_night_vision, &tx, monitor::VCP_MAGNIFIER_NV, true, &init);
    wire_toggles(&buttons_magnifier_zoom, &tx, monitor::VCP_MAGNIFIER_ZOOM, true, 0, &init);
    wire_toggles(&buttons_magnifier_size, &tx, monitor::VCP_MAGNIFIER_SIZE, true, 0, &init);
    wire_toggles(&buttons_magnifier_pos, &tx, monitor::VCP_MAGNIFIER_POS, true, 0, &init);
    wire_switch(&switch_alignment, &tx, monitor::VCP_ALIGNMENT, true, &init);
    wire_expander(&expander_hawkeye, &tx, monitor::VCP_HAWKEYE, true, &init);
    wire_toggles(&buttons_hawkeye_size, &tx, monitor::VCP_HAWKEYE_SIZE, true, 0, &init);
    wire_toggles(&buttons_hawkeye_pos, &tx, monitor::VCP_HAWKEYE_POS, true, 0, &init);
    wire_toggles(&buttons_hawkeye_lvl, &tx, monitor::VCP_HAWKEYE_LEVEL, true, 0, &init);

    // Picture Enhance
    wire_switch(&switch_gaming_rush, &tx, monitor::VCP_GAME_RUSH, true, &init);
    wire_toggles(&buttons_gaming_local_dimming, &tx, monitor::VCP_LOCAL_DIMMING, true, 0, &init);
    wire_toggles(&buttons_gaming_dyds, &tx, monitor::VCP_DYDS, true, 0, &init);
    wire_toggles(&buttons_gaming_night_vision, &tx, monitor::VCP_NIGHT_VISION, true, 0, &init);
    wire_toggles(&buttons_gaming_dynamic_od, &tx, monitor::VCP_DYNAMIC_OD, true, 0, &init);
    wire_toggles(&buttons_gaming_hdr, &tx, monitor::VCP_HDR, true, 0, &init);
    wire_scale(&scale_gaming_color_enhance, &tx, monitor::VCP_COLOR_ENHANCE, true, &init);
    wire_scale(&scale_gaming_cr_enhance, &tx, monitor::VCP_CR_ENHANCE, true, &init);
    wire_scale(&scale_gaming_shadow_enhance, &tx, monitor::VCP_SHADOW_BALANCE, true, &init);
    wire_scale(&scale_gaming_super_res, &tx, monitor::VCP_SUPER_RES, true, &init);
    wire_scale(&scale_gaming_halo_control, &tx, monitor::VCP_HALO_CONTROL, true, &init);

    // Minimize to tray
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
