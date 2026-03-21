mod app_settings;
mod i18n;
mod monitor;
mod tray;
mod ui;

use adw::prelude::*;
use gtk4::{Align, Orientation, Scale, Switch, ToggleButton, glib};
use i18n::{LangUpdaters, lang_from_index, tr};
use libadwaita::{self as adw, ExpanderRow};
use monitor::WorkerCmd;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

fn wire_scale(
    scale: &Scale,
    tx: &mpsc::Sender<WorkerCmd>,
    code: u8,
    save: bool,
    init: &Rc<Cell<bool>>,
) {
    let tx = tx.clone();
    let init = init.clone();
    let debounce_generation = Rc::new(Cell::new(0u64));

    scale.connect_value_changed(move |s| {
        if init.get() || !s.is_sensitive() {
            return;
        }
        let generation = debounce_generation.get().wrapping_add(1);
        debounce_generation.set(generation);
        let tx = tx.clone();
        let debounce_generation = debounce_generation.clone();
        let value = s.value() as u16;
        glib::timeout_add_local(Duration::from_millis(300), move || {
            if debounce_generation.get() != generation {
                return glib::ControlFlow::Break;
            }
            let _ = tx.send(if save {
                WorkerCmd::SetSave(code, value)
            } else {
                WorkerCmd::Set(code, value)
            });
            glib::ControlFlow::Break
        });
    });
}

fn wire_color_temp_rgb_scale(
    scale: &Scale,
    tx: &mpsc::Sender<WorkerCmd>,
    color_temp_buttons: &[ToggleButton],
    channel_index: usize,
    init: &Rc<Cell<bool>>,
) {
    let tx = tx.clone();
    let init = init.clone();
    let color_temp_buttons = color_temp_buttons.to_vec();
    let debounce_generation = Rc::new(Cell::new(0u64));

    scale.connect_value_changed(move |s| {
        if init.get() || !s.is_sensitive() {
            return;
        }
        let Some(color_temp) =
            selected_toggle_value(&color_temp_buttons, &monitor::COLOR_TEMP_VALUES[..])
        else {
            return;
        };
        let Some((red_code, green_code, blue_code)) = monitor::color_temp_rgb_codes(color_temp)
        else {
            return;
        };
        let code = [red_code, green_code, blue_code][channel_index];
        let generation = debounce_generation.get().wrapping_add(1);
        debounce_generation.set(generation);
        let tx = tx.clone();
        let debounce_generation = debounce_generation.clone();
        let value = s.value() as u16;
        glib::timeout_add_local(Duration::from_millis(300), move || {
            if debounce_generation.get() != generation {
                return glib::ControlFlow::Break;
            }
            let _ = tx.send(WorkerCmd::Set(code, value));
            glib::ControlFlow::Break
        });
    });
}

fn wire_switch(
    sw: &Switch,
    tx: &mpsc::Sender<WorkerCmd>,
    code: u8,
    save: bool,
    init: &Rc<Cell<bool>>,
) {
    wire_switch_values(sw, tx, code, 2, 1, save, init);
}

fn wire_switch_values(
    sw: &Switch,
    tx: &mpsc::Sender<WorkerCmd>,
    code: u8,
    on_value: u16,
    off_value: u16,
    save: bool,
    init: &Rc<Cell<bool>>,
) {
    let tx = tx.clone();
    let init = init.clone();
    let sw_ref = sw.clone();
    sw.connect_state_set(move |_, state| {
        if init.get() || !sw_ref.is_sensitive() {
            return glib::Propagation::Proceed;
        }
        let val = if state { on_value } else { off_value };
        let _ = tx.send(if save {
            WorkerCmd::SetSave(code, val)
        } else {
            WorkerCmd::Set(code, val)
        });
        glib::Propagation::Proceed
    });
}

fn wire_combo_values(
    combo: &adw::ComboRow,
    tx: &mpsc::Sender<WorkerCmd>,
    code: u8,
    values: &'static [u16],
    save: bool,
    init: &Rc<Cell<bool>>,
) {
    let tx = tx.clone();
    let init = init.clone();
    combo.connect_selected_notify(move |c| {
        if init.get() || !c.is_sensitive() {
            return;
        }
        let idx = c.selected() as usize;
        if idx >= values.len() {
            return;
        }
        let _ = tx.send(if save {
            WorkerCmd::SetSave(code, values[idx])
        } else {
            WorkerCmd::Set(code, values[idx])
        });
    });
}

fn wire_toggles(
    buttons: &[ToggleButton],
    tx: &mpsc::Sender<WorkerCmd>,
    code: u8,
    save: bool,
    offset: u16,
    init: &Rc<Cell<bool>>,
) {
    for (i, btn) in buttons.iter().enumerate() {
        let tx = tx.clone();
        let init = init.clone();
        let val = offset + i as u16;
        btn.connect_toggled(move |b| {
            if init.get() || !b.is_sensitive() {
                return;
            }
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

fn selected_toggle_index(buttons: &[ToggleButton]) -> Option<usize> {
    buttons.iter().position(|button| button.is_active())
}

fn selected_toggle_value(buttons: &[ToggleButton], values: &[u16]) -> Option<u16> {
    selected_toggle_index(buttons).and_then(|index| values.get(index).copied())
}

fn activate_toggle_index(buttons: &[ToggleButton], index: usize) {
    if let Some(button) = buttons.get(index) {
        button.set_active(true);
    }
}

fn activate_toggle_value(buttons: &[ToggleButton], values: &[u16], value: u16) {
    if let Some(index) = values.iter().position(|&mapped| mapped == value) {
        activate_toggle_index(buttons, index);
    }
}

fn wire_toggle_values(
    buttons: &[ToggleButton],
    tx: &mpsc::Sender<WorkerCmd>,
    code: u8,
    values: &[u16],
    save: bool,
    init: &Rc<Cell<bool>>,
) {
    for (btn, &val) in buttons.iter().zip(values.iter()) {
        let tx = tx.clone();
        let init = init.clone();
        btn.connect_toggled(move |b| {
            if init.get() || !b.is_sensitive() {
                return;
            }
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

fn wire_expander(
    exp: &ExpanderRow,
    tx: &mpsc::Sender<WorkerCmd>,
    code: u8,
    save: bool,
    init: &Rc<Cell<bool>>,
) {
    let tx = tx.clone();
    let init = init.clone();
    exp.connect_enable_expansion_notify(move |e| {
        if init.get() || !e.is_sensitive() {
            return;
        }
        let val = if e.enables_expansion() { 1u16 } else { 0 };
        let _ = tx.send(if save {
            WorkerCmd::SetSave(code, val)
        } else {
            WorkerCmd::Set(code, val)
        });
    });
}

fn send_profile_values_to_monitor(
    tx: &mpsc::Sender<WorkerCmd>,
    values: &std::collections::HashMap<u8, u16>,
) {
    let color_temp = values
        .get(&monitor::VCP_COLOR_TEMP)
        .copied()
        .unwrap_or(monitor::COLOR_TEMP_VALUES[0]);

    for (&code, &value) in values {
        match code {
            monitor::VCP_RED | monitor::VCP_GREEN | monitor::VCP_BLUE => {
                if let Some((r_code, g_code, b_code)) = monitor::color_temp_rgb_codes(color_temp) {
                    let actual_code = match code {
                        monitor::VCP_RED => r_code,
                        monitor::VCP_GREEN => g_code,
                        _ => b_code,
                    };
                    let _ = tx.send(WorkerCmd::Set(actual_code, value));
                }
            }
            monitor::VCP_HDR
            | monitor::VCP_NIGHT_VISION
            | monitor::VCP_DYNAMIC_OD
            | monitor::VCP_GAMMA
            | monitor::VCP_COLOR_TEMP => {
                let _ = tx.send(WorkerCmd::SetSave(code, value));
            }
            _ => {
                let _ = tx.send(WorkerCmd::Set(code, value));
            }
        }
    }
}

fn main() {
    glib::set_application_name(app_settings::APP_DISPLAY_NAME);

    let app = adw::Application::builder()
        .application_id(app_settings::APP_ID)
        .build();
    app.connect_activate(build_application);
    app.run();
}

fn register_app_icons() {
    if let Some(display) = gtk4::gdk::Display::default() {
        let icon_theme = gtk4::IconTheme::for_display(&display);

        for icon_path in app_settings::icon_search_paths() {
            icon_theme.add_search_path(&icon_path);
        }
    }

    gtk4::Window::set_default_icon_name(app_settings::APP_ICON_NAME);
}

fn translate_startup_message(lang: &i18n::AppLang, message: &str) -> String {
    if let Some(name) = message
        .strip_prefix("Connecting to ")
        .and_then(|msg| msg.strip_suffix("..."))
    {
        return tr(lang, "splash_connecting").replace("{name}", name);
    }
    if let Some(name) = message
        .strip_prefix("Using cached monitor info: ")
        .and_then(|msg| msg.strip_suffix("..."))
    {
        return tr(lang, "splash_using_cached_info").replace("{name}", name);
    }
    if let Some(name) = message
        .strip_prefix("Reading info: ")
        .and_then(|msg| msg.strip_suffix("..."))
    {
        return tr(lang, "splash_reading_info").replace("{name}", name);
    }

    match message {
        "Searching for monitor..." => tr(lang, "splash_searching"),
        "Detecting monitors..." => tr(lang, "splash_detecting_monitors"),
        "Searching for monitors..." => tr(lang, "splash_searching_monitors"),
        "Loading cached settings..." => tr(lang, "splash_loading_cached"),
        "Monitor not found" => tr(lang, "splash_monitor_not_found"),
        _ => message.to_string(),
    }
}

fn build_application(app: &adw::Application) {
    register_app_icons();
    let _ = app_settings::ensure_desktop_entry();

    let (worker_tx, worker_rx) = mpsc::channel::<WorkerCmd>();
    let (ui_tx, ui_rx) = async_channel::unbounded::<monitor::UiCmd>();

    let lang = i18n::detect_system_lang();
    let updaters: LangUpdaters = Rc::new(std::cell::RefCell::new(Vec::new()));
    let app_cfg = Rc::new(RefCell::new(app_settings::load()));
    let startup_finished = Rc::new(Cell::new(false));

    // Splash screen
    let splash_logo = if let Some(path) = app_settings::resolve_logo_path() {
        let image = gtk4::Image::from_file(path);
        image.set_pixel_size(96);
        image
    } else {
        gtk4::Image::from_icon_name(app_settings::APP_ICON_NAME)
    };
    splash_logo.set_halign(Align::Center);
    splash_logo.set_valign(Align::Center);
    splash_logo.set_opacity(0.85);
    let splash_logo_phase = Rc::new(Cell::new(0.0f64));
    glib::timeout_add_local(
        Duration::from_millis(32),
        glib::clone!(
            #[strong]
            splash_logo,
            #[strong]
            splash_logo_phase,
            move || {
                let next_phase = splash_logo_phase.get() + 0.09;
                splash_logo_phase.set(next_phase);
                let opacity = 0.82 + ((next_phase.sin() + 1.0) * 0.5 * 0.18);
                splash_logo.set_opacity(opacity);
                glib::ControlFlow::Continue
            }
        ),
    );
    let splash_label = gtk4::Label::builder()
        .label(tr(&lang, "splash_searching"))
        .wrap(true)
        .justify(gtk4::Justification::Center)
        .build();
    let splash_box = gtk4::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(16)
        .halign(Align::Center)
        .valign(Align::Center)
        .vexpand(true)
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .build();
    splash_box.append(&splash_logo);
    splash_box.append(&splash_label);
    let splash = adw::Window::builder()
        .title("Titan Control")
        .icon_name(app_settings::APP_ICON_NAME)
        .default_width(360)
        .default_height(260)
        .content(&splash_box)
        .application(app)
        .build();
    glib::timeout_add_local_once(
        Duration::from_millis(350),
        glib::clone!(
            #[strong]
            splash,
            #[strong]
            startup_finished,
            move || {
                if !startup_finished.get() {
                    splash.present();
                }
            }
        ),
    );

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
        switch_auto_start,
        switch_start_minimized,
        // Display tab
        row_info_model,
        row_info_resolution,
        row_info_hz,
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
        combo_input_source,
        combo_output_range,
        switch_quick_boot,
        combo_osd_language,
        scale_osd_time,
        scale_osd_h_position,
        scale_osd_v_position,
        scale_osd_transparency,
        button_reset_factory,
        // Profile tab
        combo_picture_mode,
        button_profile_default,
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
        hue_scales_vector,
        saturation_scales_vector,
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
        switch_gaming_async,
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

    switch_auto_start.set_active(app_cfg.borrow().auto_start);
    switch_start_minimized.set_active(app_cfg.borrow().start_minimized);
    switch_start_minimized.set_sensitive(app_cfg.borrow().auto_start);
    if let Some(last_picture_mode) = app_cfg.borrow().last_picture_mode {
        let max_index = monitor::PICTURE_MODE_NAMES.len().saturating_sub(1) as u16;
        combo_picture_mode.set_selected(last_picture_mode.min(max_index) as u32);
    }
    {
        let initial_custom_profile = app_cfg
            .borrow()
            .custom_profile(combo_picture_mode.selected() as u16);
        let has_custom = monitor::picture_mode_has_custom(combo_picture_mode.selected() as usize);
        button_profile_custom.set_sensitive(has_custom);
        button_profile_custom.set_active(has_custom && initial_custom_profile.active);
        button_profile_default.set_active(!has_custom || !initial_custom_profile.active);
        custom_revealer.set_reveal_child(has_custom && initial_custom_profile.active);
    }

    monitor::start_worker(worker_rx, ui_tx);

    let init = Rc::new(Cell::new(true)); // block signals during UI init

    // --- LOCAL UI LOGIC ---

    let syncing_custom_profile_ui = Rc::new(Cell::new(false));
    let active_picture_mode = Rc::new(Cell::new(combo_picture_mode.selected() as u16));
    let active_picture_mode_has_custom = Rc::new(Cell::new(monitor::picture_mode_has_custom(
        combo_picture_mode.selected() as usize,
    )));

    let sync_picture_mode_controls: Rc<dyn Fn(u16, bool)> = Rc::new({
        let button_profile_custom = button_profile_custom.clone();
        let button_profile_default = button_profile_default.clone();
        let custom_revealer = custom_revealer.clone();
        let syncing_custom_profile_ui = syncing_custom_profile_ui.clone();
        move |mode, custom_active| {
            let has_custom = monitor::picture_mode_has_custom(mode as usize);
            syncing_custom_profile_ui.set(true);
            button_profile_custom.set_sensitive(has_custom);
            if has_custom {
                button_profile_custom.set_active(custom_active);
                button_profile_default.set_active(!custom_active);
                custom_revealer.set_reveal_child(custom_active);
            } else {
                button_profile_default.set_active(true);
                button_profile_custom.set_active(false);
                custom_revealer.set_reveal_child(false);
            }
            syncing_custom_profile_ui.set(false);
        }
    });

    let collect_custom_profile_values: Rc<dyn Fn() -> std::collections::HashMap<u8, u16>> =
        Rc::new({
            let scale_custom_brightness = scale_custom_brightness.clone();
            let scale_custom_contrast = scale_custom_contrast.clone();
            let scale_custom_sharpness = scale_custom_sharpness.clone();
            let scale_custom_shadow_balance = scale_custom_shadow_balance.clone();
            let scale_custom_cr_enhance = scale_custom_cr_enhance.clone();
            let scale_custom_color_enhance = scale_custom_color_enhance.clone();
            let scale_custom_super_res = scale_custom_super_res.clone();
            let scale_custom_low_blue_light = scale_custom_low_blue_light.clone();
            let combo_custom_color_temp = combo_custom_color_temp.clone();
            let scale_custom_red_gain = scale_custom_red_gain.clone();
            let scale_custom_green_gain = scale_custom_green_gain.clone();
            let scale_custom_blue_gain = scale_custom_blue_gain.clone();
            let combo_custom_hdr = combo_custom_hdr.clone();
            let combo_custom_gamma = combo_custom_gamma.clone();
            let combo_custom_night_vision = combo_custom_night_vision.clone();
            let combo_custom_dynamic_od = combo_custom_dynamic_od.clone();
            let hue_scales_vector = hue_scales_vector.clone();
            let saturation_scales_vector = saturation_scales_vector.clone();
            move || {
                let mut values = std::collections::HashMap::new();
                values.insert(
                    monitor::VCP_BRIGHTNESS,
                    scale_custom_brightness.value() as u16,
                );
                values.insert(monitor::VCP_CONTRAST, scale_custom_contrast.value() as u16);
                values.insert(
                    monitor::VCP_SHARPNESS,
                    scale_custom_sharpness.value() as u16,
                );
                values.insert(
                    monitor::VCP_SHADOW_BALANCE,
                    scale_custom_shadow_balance.value() as u16,
                );
                values.insert(
                    monitor::VCP_CR_ENHANCE,
                    scale_custom_cr_enhance.value() as u16,
                );
                values.insert(
                    monitor::VCP_COLOR_ENHANCE,
                    scale_custom_color_enhance.value() as u16,
                );
                values.insert(
                    monitor::VCP_SUPER_RES,
                    scale_custom_super_res.value() as u16,
                );
                values.insert(
                    monitor::VCP_LOW_BLUE,
                    scale_custom_low_blue_light.value() as u16,
                );
                if let Some(value) =
                    selected_toggle_value(&combo_custom_color_temp, &monitor::COLOR_TEMP_VALUES[..])
                {
                    values.insert(monitor::VCP_COLOR_TEMP, value);
                }
                values.insert(monitor::VCP_RED, scale_custom_red_gain.value() as u16);
                values.insert(monitor::VCP_GREEN, scale_custom_green_gain.value() as u16);
                values.insert(monitor::VCP_BLUE, scale_custom_blue_gain.value() as u16);
                if let Some(value) =
                    selected_toggle_value(&combo_custom_hdr, &monitor::HDR_VALUES[..])
                {
                    values.insert(monitor::VCP_HDR, value);
                }
                if let Some(value) =
                    selected_toggle_value(&combo_custom_gamma, &monitor::GAMMA_VALUES[..])
                {
                    values.insert(monitor::VCP_GAMMA, value);
                }
                if let Some(value) = selected_toggle_value(
                    &combo_custom_night_vision,
                    &monitor::NIGHT_VISION_VALUES[..],
                ) {
                    values.insert(monitor::VCP_NIGHT_VISION, value);
                }
                if let Some(value) =
                    selected_toggle_value(&combo_custom_dynamic_od, &monitor::DYNAMIC_OD_VALUES[..])
                {
                    values.insert(monitor::VCP_DYNAMIC_OD, value);
                }
                for (scale, code) in hue_scales_vector.iter().zip(monitor::HUE_CODES.iter()) {
                    values.insert(*code, scale.value() as u16);
                }
                for (scale, code) in saturation_scales_vector
                    .iter()
                    .zip(monitor::SATURATION_CODES.iter())
                {
                    values.insert(*code, scale.value() as u16);
                }
                values
            }
        });

    let apply_custom_profile_values: Rc<dyn Fn(&std::collections::HashMap<u8, u16>)> = Rc::new({
        let scale_custom_brightness = scale_custom_brightness.clone();
        let scale_custom_contrast = scale_custom_contrast.clone();
        let scale_custom_sharpness = scale_custom_sharpness.clone();
        let scale_custom_shadow_balance = scale_custom_shadow_balance.clone();
        let scale_custom_cr_enhance = scale_custom_cr_enhance.clone();
        let scale_custom_color_enhance = scale_custom_color_enhance.clone();
        let scale_custom_super_res = scale_custom_super_res.clone();
        let scale_custom_low_blue_light = scale_custom_low_blue_light.clone();
        let combo_custom_color_temp = combo_custom_color_temp.clone();
        let scale_custom_red_gain = scale_custom_red_gain.clone();
        let scale_custom_green_gain = scale_custom_green_gain.clone();
        let scale_custom_blue_gain = scale_custom_blue_gain.clone();
        let combo_custom_hdr = combo_custom_hdr.clone();
        let combo_custom_gamma = combo_custom_gamma.clone();
        let combo_custom_night_vision = combo_custom_night_vision.clone();
        let combo_custom_dynamic_od = combo_custom_dynamic_od.clone();
        let hue_scales_vector = hue_scales_vector.clone();
        let saturation_scales_vector = saturation_scales_vector.clone();
        move |values: &std::collections::HashMap<u8, u16>| {
            if let Some(&v) = values.get(&monitor::VCP_BRIGHTNESS) {
                scale_custom_brightness.set_value(v as f64);
            }
            if let Some(&v) = values.get(&monitor::VCP_CONTRAST) {
                scale_custom_contrast.set_value(v as f64);
            }
            if let Some(&v) = values.get(&monitor::VCP_SHARPNESS) {
                scale_custom_sharpness.set_value(v as f64);
            }
            if let Some(&v) = values.get(&monitor::VCP_SHADOW_BALANCE) {
                scale_custom_shadow_balance.set_value(v as f64);
            }
            if let Some(&v) = values.get(&monitor::VCP_CR_ENHANCE) {
                scale_custom_cr_enhance.set_value(v as f64);
            }
            if let Some(&v) = values.get(&monitor::VCP_COLOR_ENHANCE) {
                scale_custom_color_enhance.set_value(v as f64);
            }
            if let Some(&v) = values.get(&monitor::VCP_SUPER_RES) {
                scale_custom_super_res.set_value(v as f64);
            }
            if let Some(&v) = values.get(&monitor::VCP_LOW_BLUE) {
                scale_custom_low_blue_light.set_value(v as f64);
            }
            if let Some(&v) = values.get(&monitor::VCP_COLOR_TEMP) {
                activate_toggle_value(&combo_custom_color_temp, &monitor::COLOR_TEMP_VALUES[..], v);
            }
            if let Some(&v) = values.get(&monitor::VCP_RED) {
                scale_custom_red_gain.set_value(v as f64);
            }
            if let Some(&v) = values.get(&monitor::VCP_GREEN) {
                scale_custom_green_gain.set_value(v as f64);
            }
            if let Some(&v) = values.get(&monitor::VCP_BLUE) {
                scale_custom_blue_gain.set_value(v as f64);
            }
            if let Some(&v) = values.get(&monitor::VCP_HDR) {
                activate_toggle_value(&combo_custom_hdr, &monitor::HDR_VALUES[..], v);
            }
            if let Some(&v) = values.get(&monitor::VCP_GAMMA) {
                activate_toggle_value(&combo_custom_gamma, &monitor::GAMMA_VALUES[..], v);
            }
            if let Some(&v) = values.get(&monitor::VCP_NIGHT_VISION) {
                activate_toggle_value(
                    &combo_custom_night_vision,
                    &monitor::NIGHT_VISION_VALUES[..],
                    v,
                );
            }
            if let Some(&v) = values.get(&monitor::VCP_DYNAMIC_OD) {
                activate_toggle_value(&combo_custom_dynamic_od, &monitor::DYNAMIC_OD_VALUES[..], v);
            }
            for (scale, code) in hue_scales_vector.iter().zip(monitor::HUE_CODES.iter()) {
                if let Some(&v) = values.get(code) {
                    scale.set_value(v as f64);
                }
            }
            for (scale, code) in saturation_scales_vector
                .iter()
                .zip(monitor::SATURATION_CODES.iter())
            {
                if let Some(&v) = values.get(code) {
                    scale.set_value(v as f64);
                }
            }
        }
    });

    let apply_profile_values_silently: Rc<dyn Fn(&std::collections::HashMap<u8, u16>)> = Rc::new({
        let init = init.clone();
        let apply_custom_profile_values = apply_custom_profile_values.clone();
        move |values: &std::collections::HashMap<u8, u16>| {
            let previous_init = init.replace(true);
            (apply_custom_profile_values)(values);
            init.set(previous_init);
        }
    });

    let persist_current_custom_profile_state: Rc<dyn Fn()> = Rc::new({
        let active_picture_mode = active_picture_mode.clone();
        let button_profile_custom = button_profile_custom.clone();
        let app_cfg = app_cfg.clone();
        let collect_custom_profile_values = collect_custom_profile_values.clone();
        move || {
            let mut cfg = app_cfg.borrow().clone();
            let profile = cfg
                .custom_profiles
                .entry(active_picture_mode.get())
                .or_default();
            profile.active = button_profile_custom.is_active();
            profile.values = (collect_custom_profile_values)();
            let _ = app_settings::save(&cfg);
            *app_cfg.borrow_mut() = cfg;
        }
    });

    let load_custom_profile_for_mode: Rc<dyn Fn(u16)> = Rc::new({
        let app_cfg = app_cfg.clone();
        let apply_profile_values_silently = apply_profile_values_silently.clone();
        let sync_picture_mode_controls = sync_picture_mode_controls.clone();
        let active_picture_mode_has_custom = active_picture_mode_has_custom.clone();
        move |mode| {
            let profile = app_cfg.borrow().custom_profile(mode);
            let default_values = app_settings::basic_profile_values_for_mode(mode);
            let has_custom = monitor::picture_mode_has_custom(mode as usize);
            active_picture_mode_has_custom.set(has_custom);
            let custom_active = has_custom && profile.active;

            (sync_picture_mode_controls)(mode, custom_active);
            let values = if custom_active {
                &profile.values
            } else {
                &default_values
            };
            (apply_profile_values_silently)(values);
        }
    });

    {
        let worker_tx = worker_tx.clone();
        button_profile_custom.connect_toggled(glib::clone!(
            #[strong]
            custom_revealer,
            #[strong]
            app_cfg,
            #[strong]
            active_picture_mode,
            #[strong]
            active_picture_mode_has_custom,
            #[strong]
            syncing_custom_profile_ui,
            #[strong]
            apply_profile_values_silently,
            move |btn| {
                if syncing_custom_profile_ui.get()
                    || !btn.is_active()
                    || !active_picture_mode_has_custom.get()
                {
                    return;
                }

                custom_revealer.set_reveal_child(btn.is_active());
                let mut cfg = app_cfg.borrow().clone();
                let profile = cfg
                    .custom_profiles
                    .entry(active_picture_mode.get())
                    .or_default();
                profile.active = true;
                let values_to_apply = profile.values.clone();
                let _ = app_settings::save(&cfg);
                *app_cfg.borrow_mut() = cfg;

                // Send VCP_MODE custom variant to monitor
                if let Some(mode_value) =
                    monitor::picture_mode_vcp_value(active_picture_mode.get() as usize, true)
                {
                    let _ = worker_tx.send(WorkerCmd::Set(monitor::VCP_MODE, mode_value));
                }

                // Send all custom values to monitor
                send_profile_values_to_monitor(&worker_tx, &values_to_apply);

                syncing_custom_profile_ui.set(true);
                (apply_profile_values_silently)(&values_to_apply);
                syncing_custom_profile_ui.set(false);
            }
        ));
    }

    {
        let worker_tx = worker_tx.clone();
        button_profile_default.connect_toggled(glib::clone!(
            #[strong]
            app_cfg,
            #[strong]
            custom_revealer,
            #[strong]
            active_picture_mode,
            #[strong]
            syncing_custom_profile_ui,
            #[strong]
            apply_profile_values_silently,
            move |btn| {
                if syncing_custom_profile_ui.get() || !btn.is_active() {
                    return;
                }

                custom_revealer.set_reveal_child(false);
                let mut cfg = app_cfg.borrow().clone();
                let profile = cfg
                    .custom_profiles
                    .entry(active_picture_mode.get())
                    .or_default();
                profile.active = false;
                let _ = app_settings::save(&cfg);
                *app_cfg.borrow_mut() = cfg;

                // Send VCP_MODE default variant to monitor
                if let Some(mode_value) =
                    monitor::picture_mode_vcp_value(active_picture_mode.get() as usize, false)
                {
                    let _ = worker_tx.send(WorkerCmd::Set(monitor::VCP_MODE, mode_value));
                }

                let default_values =
                    app_settings::basic_profile_values_for_mode(active_picture_mode.get());
                syncing_custom_profile_ui.set(true);
                (apply_profile_values_silently)(&default_values);
                syncing_custom_profile_ui.set(false);
            }
        ));
    }

    for scale in [
        scale_custom_brightness.clone(),
        scale_custom_contrast.clone(),
        scale_custom_sharpness.clone(),
        scale_custom_shadow_balance.clone(),
        scale_custom_cr_enhance.clone(),
        scale_custom_color_enhance.clone(),
        scale_custom_super_res.clone(),
        scale_custom_low_blue_light.clone(),
        scale_custom_red_gain.clone(),
        scale_custom_green_gain.clone(),
        scale_custom_blue_gain.clone(),
    ]
    .into_iter()
    .chain(hue_scales_vector.clone().into_iter())
    .chain(saturation_scales_vector.clone().into_iter())
    {
        let init = init.clone();
        let syncing_custom_profile_ui = syncing_custom_profile_ui.clone();
        let persist_current_custom_profile_state = persist_current_custom_profile_state.clone();
        scale.connect_value_changed(move |_| {
            if init.get() || syncing_custom_profile_ui.get() {
                return;
            }
            (persist_current_custom_profile_state)();
        });
    }

    for buttons in [
        combo_custom_color_temp.clone(),
        combo_custom_hdr.clone(),
        combo_custom_gamma.clone(),
        combo_custom_night_vision.clone(),
        combo_custom_dynamic_od.clone(),
    ] {
        for button in buttons {
            let init = init.clone();
            let syncing_custom_profile_ui = syncing_custom_profile_ui.clone();
            let persist_current_custom_profile_state = persist_current_custom_profile_state.clone();
            button.connect_toggled(move |button| {
                if init.get() || syncing_custom_profile_ui.get() || !button.is_active() {
                    return;
                }
                (persist_current_custom_profile_state)();
            });
        }
    }

    // --- MONITOR -> UI ---

    let current_monitor_name = Rc::new(RefCell::new(String::new()));
    let sync_gaming_constraints: Rc<dyn Fn()> = {
        let buttons_screen_size = buttons_screen_size.clone();
        let switch_gaming_async = switch_gaming_async.clone();
        let switch_gaming_rush = switch_gaming_rush.clone();
        let buttons_gaming_dyds = buttons_gaming_dyds.clone();
        let buttons_gaming_dynamic_od = buttons_gaming_dynamic_od.clone();
        let buttons_gaming_local_dimming = buttons_gaming_local_dimming.clone();
        let expander_magnifier = expander_magnifier.clone();
        let expander_hawkeye = expander_hawkeye.clone();
        let switch_alignment = switch_alignment.clone();
        let scale_gaming_halo_control = scale_gaming_halo_control.clone();
        Rc::new(move || {
            let wide_mode_active = buttons_screen_size
                .first()
                .is_some_and(|btn| btn.is_active());
            let dyds_ull_active = buttons_gaming_dyds
                .iter()
                .enumerate()
                .skip(4)
                .any(|(_, btn)| btn.is_active());
            let adaptive_sync_active = switch_gaming_async.is_active();
            let local_dimming_enabled = buttons_gaming_local_dimming
                .iter()
                .enumerate()
                .skip(1)
                .any(|(_, btn)| btn.is_active());
            let magnifier_active = expander_magnifier.enables_expansion();
            let hawkeye_active = expander_hawkeye.enables_expansion();

            if !wide_mode_active {
                switch_gaming_async.set_active(false);
                if let Some(off_button) = buttons_gaming_dyds.first() {
                    off_button.set_active(true);
                }
                if let Some(off_button) = buttons_gaming_dynamic_od.first() {
                    off_button.set_active(true);
                }
                expander_magnifier.set_enable_expansion(false);
                expander_hawkeye.set_enable_expansion(false);
                scale_gaming_halo_control.set_value(0.0);
            }

            if adaptive_sync_active {
                if let Some(off_button) = buttons_gaming_dyds.first() {
                    off_button.set_active(true);
                }
                expander_magnifier.set_enable_expansion(false);
                expander_hawkeye.set_enable_expansion(false);
            }

            if magnifier_active && hawkeye_active {
                expander_hawkeye.set_enable_expansion(false);
            }

            if dyds_ull_active {
                if let Some(disabled_button) = buttons_gaming_local_dimming.first() {
                    disabled_button.set_active(true);
                }
            }

            if !switch_gaming_rush.is_active() {
                switch_gaming_rush.set_active(true);
            }

            switch_gaming_async.set_sensitive(true);
            switch_gaming_rush.set_sensitive(false);

            if let Some(spx_button) = buttons_screen_size.get(2) {
                spx_button.set_sensitive(false);
            }

            switch_alignment.set_sensitive(false);

            expander_magnifier
                .set_sensitive(wide_mode_active && !adaptive_sync_active && !hawkeye_active);
            expander_hawkeye
                .set_sensitive(wide_mode_active && !adaptive_sync_active && !magnifier_active);
            switch_gaming_async.set_sensitive(wide_mode_active);
            scale_gaming_halo_control
                .set_sensitive(wide_mode_active && local_dimming_enabled && !dyds_ull_active);

            for (index, button) in buttons_gaming_dyds.iter().enumerate() {
                button.set_sensitive(
                    wide_mode_active
                        && !adaptive_sync_active
                        && (!switch_gaming_async.is_active() || index == 0 || button.is_active()),
                );
            }

            for button in &buttons_gaming_dynamic_od {
                button.set_sensitive(wide_mode_active);
            }

            for button in &buttons_gaming_local_dimming {
                button.set_sensitive(!dyds_ull_active);
            }
        })
    };

    {
        let buttons_gaming_dyds = buttons_gaming_dyds.clone();
        let sync_gaming_constraints = sync_gaming_constraints.clone();
        switch_gaming_async.connect_active_notify(move |sw| {
            if sw.is_active() {
                if let Some(off_button) = buttons_gaming_dyds.first() {
                    off_button.set_active(true);
                }
            }
            sync_gaming_constraints();
        });
    }

    for button in &buttons_gaming_dyds {
        let sync_gaming_constraints = sync_gaming_constraints.clone();
        button.connect_toggled(move |_| {
            sync_gaming_constraints();
        });
    }

    for button in &buttons_gaming_local_dimming {
        let sync_gaming_constraints = sync_gaming_constraints.clone();
        button.connect_toggled(move |_| {
            sync_gaming_constraints();
        });
    }

    {
        let sync_gaming_constraints = sync_gaming_constraints.clone();
        expander_magnifier.connect_enable_expansion_notify(move |_| {
            sync_gaming_constraints();
        });
    }

    {
        let sync_gaming_constraints = sync_gaming_constraints.clone();
        expander_hawkeye.connect_enable_expansion_notify(move |_| {
            sync_gaming_constraints();
        });
    }

    let sync_color_temp_controls: Rc<dyn Fn()> = {
        let combo_custom_color_temp = combo_custom_color_temp.clone();
        let scale_custom_red_gain = scale_custom_red_gain.clone();
        let scale_custom_green_gain = scale_custom_green_gain.clone();
        let scale_custom_blue_gain = scale_custom_blue_gain.clone();
        let init = init.clone();
        let current_monitor_name = current_monitor_name.clone();
        Rc::new(move || {
            let Some(selected) = selected_toggle_index(&combo_custom_color_temp) else {
                return;
            };
            let Some(&color_temp) = monitor::COLOR_TEMP_VALUES.get(selected) else {
                return;
            };

            let previous_init = init.replace(true);

            if let Some((red, green, blue)) = monitor::builtin_color_temp_rgb(color_temp) {
                scale_custom_red_gain.set_sensitive(false);
                scale_custom_green_gain.set_sensitive(false);
                scale_custom_blue_gain.set_sensitive(false);
                scale_custom_red_gain.set_value(red as f64);
                scale_custom_green_gain.set_value(green as f64);
                scale_custom_blue_gain.set_value(blue as f64);
            } else {
                scale_custom_red_gain.set_sensitive(true);
                scale_custom_green_gain.set_sensitive(true);
                scale_custom_blue_gain.set_sensitive(true);

                let monitor_name = current_monitor_name.borrow().clone();
                if !monitor_name.is_empty() {
                    let cached = monitor::load_color_temp_profile_cache(&monitor_name, color_temp);
                    if let Some(&value) = cached.get(&monitor::VCP_RED) {
                        scale_custom_red_gain.set_value(value as f64);
                    }
                    if let Some(&value) = cached.get(&monitor::VCP_GREEN) {
                        scale_custom_green_gain.set_value(value as f64);
                    }
                    if let Some(&value) = cached.get(&monitor::VCP_BLUE) {
                        scale_custom_blue_gain.set_value(value as f64);
                    }
                }
            }

            init.set(previous_init);
        })
    };

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
        (
            scale_custom_shadow_balance.clone(),
            monitor::VCP_SHADOW_BALANCE,
        ),
        (scale_custom_cr_enhance.clone(), monitor::VCP_CR_ENHANCE),
        (
            scale_custom_color_enhance.clone(),
            monitor::VCP_COLOR_ENHANCE,
        ),
        (scale_custom_super_res.clone(), monitor::VCP_SUPER_RES),
        (scale_custom_low_blue_light.clone(), monitor::VCP_LOW_BLUE),
        (scale_custom_red_gain.clone(), monitor::VCP_RED),
        (scale_custom_green_gain.clone(), monitor::VCP_GREEN),
        (scale_custom_blue_gain.clone(), monitor::VCP_BLUE),
        (hue_scales_vector[0].clone(), monitor::HUE_CODES[0]),
        (hue_scales_vector[1].clone(), monitor::HUE_CODES[1]),
        (hue_scales_vector[2].clone(), monitor::HUE_CODES[2]),
        (hue_scales_vector[3].clone(), monitor::HUE_CODES[3]),
        (hue_scales_vector[4].clone(), monitor::HUE_CODES[4]),
        (hue_scales_vector[5].clone(), monitor::HUE_CODES[5]),
        (
            saturation_scales_vector[0].clone(),
            monitor::SATURATION_CODES[0],
        ),
        (
            saturation_scales_vector[1].clone(),
            monitor::SATURATION_CODES[1],
        ),
        (
            saturation_scales_vector[2].clone(),
            monitor::SATURATION_CODES[2],
        ),
        (
            saturation_scales_vector[3].clone(),
            monitor::SATURATION_CODES[3],
        ),
        (
            saturation_scales_vector[4].clone(),
            monitor::SATURATION_CODES[4],
        ),
        (
            saturation_scales_vector[5].clone(),
            monitor::SATURATION_CODES[5],
        ),
        (
            scale_gaming_color_enhance.clone(),
            monitor::VCP_COLOR_ENHANCE,
        ),
        (scale_gaming_cr_enhance.clone(), monitor::VCP_CR_ENHANCE),
        (
            scale_gaming_shadow_enhance.clone(),
            monitor::VCP_SHADOW_BALANCE,
        ),
        (scale_gaming_super_res.clone(), monitor::VCP_SUPER_RES),
        (scale_gaming_halo_control.clone(), monitor::VCP_HALO_CONTROL),
    ];

    // Collect combo bindings: (ComboRow, VCP code)
    let combo_binds: Vec<(adw::ComboRow, u8)> = vec![];

    // Collect switch bindings: (Switch, VCP code, active value)
    let switch_binds: Vec<(Switch, u8, u16)> = vec![
        (switch_quick_boot.clone(), monitor::VCP_QUICK_BOOT, 1),
        (switch_gaming_async.clone(), monitor::VCP_ADAPTIVE_SYNC, 1),
        (
            switch_magnifier_night_vision.clone(),
            monitor::VCP_MAGNIFIER_NV,
            1,
        ),
        (switch_alignment.clone(), monitor::VCP_ALIGNMENT, 1),
        (switch_gaming_rush.clone(), monitor::VCP_GAME_RUSH, 2),
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
        (
            vec![
                button_power_save_off.clone(),
                button_power_save_lvl1.clone(),
                button_power_save_lvl2.clone(),
            ],
            monitor::VCP_POWER_SAVING,
            1,
        ),
        (
            vec![
                button_led_off.clone(),
                button_led_lvl1.clone(),
                button_led_lvl2.clone(),
                button_led_lvl3.clone(),
            ],
            monitor::VCP_POWER_LED,
            1,
        ),
        (
            buttons_crosshair_shape.clone(),
            monitor::VCP_CROSSHAIR_SHAPE,
            1,
        ),
        (
            buttons_stopwatch_time.clone(),
            monitor::VCP_STOPWATCH_TIME,
            1,
        ),
        (buttons_game_time_val.clone(), monitor::VCP_GAME_TIME_VAL, 1),
        (
            buttons_magnifier_size.clone(),
            monitor::VCP_MAGNIFIER_SIZE,
            1,
        ),
    ];

    let toggle_value_binds: Vec<(Vec<ToggleButton>, u8, Vec<u16>)> = vec![
        (
            buttons_screen_size.clone(),
            monitor::VCP_SCREEN_SIZE,
            vec![0, 1, 2],
        ),
        (
            buttons_fps_pos.clone(),
            monitor::VCP_FPS_POS,
            vec![0, 1, 2, 3],
        ),
        (
            buttons_crosshair_color.clone(),
            monitor::VCP_CROSSHAIR_COLOR,
            vec![0, 1, 2, 3, 4, 5, 6, 7],
        ),
        (
            buttons_stopwatch_pos.clone(),
            monitor::VCP_STOPWATCH_POS,
            vec![0, 1, 2, 3],
        ),
        (
            buttons_game_time_pos.clone(),
            monitor::VCP_GAME_TIME_POS,
            vec![0, 1, 2, 3],
        ),
        (
            buttons_magnifier_zoom.clone(),
            monitor::VCP_MAGNIFIER_ZOOM,
            vec![0, 1, 2],
        ),
        (
            buttons_magnifier_pos.clone(),
            monitor::VCP_MAGNIFIER_POS,
            vec![1, 2, 5, 3, 4],
        ),
        (
            buttons_hawkeye_size.clone(),
            monitor::VCP_HAWKEYE_SIZE,
            vec![0, 1, 2],
        ),
        (
            buttons_hawkeye_pos.clone(),
            monitor::VCP_HAWKEYE_POS,
            vec![0, 1, 2, 3, 4],
        ),
        (
            buttons_hawkeye_lvl.clone(),
            monitor::VCP_HAWKEYE_LEVEL,
            vec![0, 1, 2, 3, 4],
        ),
        (
            buttons_gaming_local_dimming.clone(),
            monitor::VCP_LOCAL_DIMMING,
            monitor::LOCAL_DIMMING_VALUES.to_vec(),
        ),
        (
            buttons_gaming_dyds.clone(),
            monitor::VCP_DYDS,
            monitor::DYDS_VALUES.to_vec(),
        ),
        (
            buttons_gaming_night_vision.clone(),
            monitor::VCP_NIGHT_VISION,
            monitor::NIGHT_VISION_VALUES.to_vec(),
        ),
        (
            buttons_gaming_dynamic_od.clone(),
            monitor::VCP_DYNAMIC_OD,
            monitor::DYNAMIC_OD_VALUES.to_vec(),
        ),
        (
            buttons_gaming_hdr.clone(),
            monitor::VCP_HDR,
            monitor::HDR_VALUES.to_vec(),
        ),
    ];

    let mut monitor_control_widgets: Vec<gtk4::Widget> = scale_binds
        .iter()
        .map(|(scale, _)| scale.clone().upcast())
        .collect();
    monitor_control_widgets.extend([
        switch_audio_mute.clone().upcast(),
        combo_input_source.clone().upcast(),
        combo_output_range.clone().upcast(),
        combo_osd_language.clone().upcast(),
        combo_picture_mode.clone().upcast(),
        button_profile_default.clone().upcast(),
        button_profile_custom.clone().upcast(),
        button_power_off.clone().upcast(),
        button_reset_factory.clone().upcast(),
    ]);
    monitor_control_widgets.extend(
        switch_binds
            .iter()
            .map(|(sw, _, _)| sw.clone().upcast::<gtk4::Widget>()),
    );
    monitor_control_widgets.extend(
        expander_binds
            .iter()
            .map(|(exp, _)| exp.clone().upcast::<gtk4::Widget>()),
    );
    monitor_control_widgets.extend(toggle_binds.iter().flat_map(|(buttons, _, _)| {
        buttons
            .iter()
            .cloned()
            .map(|button| button.upcast::<gtk4::Widget>())
            .collect::<Vec<_>>()
    }));
    monitor_control_widgets.extend(toggle_value_binds.iter().flat_map(|(buttons, _, _)| {
        buttons
            .iter()
            .cloned()
            .map(|button| button.upcast::<gtk4::Widget>())
            .collect::<Vec<_>>()
    }));
    monitor_control_widgets.extend(
        combo_custom_color_temp
            .iter()
            .cloned()
            .map(|button| button.upcast::<gtk4::Widget>()),
    );
    monitor_control_widgets.extend(
        combo_custom_hdr
            .iter()
            .cloned()
            .map(|button| button.upcast::<gtk4::Widget>()),
    );
    monitor_control_widgets.extend(
        combo_custom_gamma
            .iter()
            .cloned()
            .map(|button| button.upcast::<gtk4::Widget>()),
    );
    monitor_control_widgets.extend(
        combo_custom_night_vision
            .iter()
            .cloned()
            .map(|button| button.upcast::<gtk4::Widget>()),
    );
    monitor_control_widgets.extend(
        combo_custom_dynamic_od
            .iter()
            .cloned()
            .map(|button| button.upcast::<gtk4::Widget>()),
    );

    let combo_picture_mode_ui = combo_picture_mode.clone();

    glib::MainContext::default().spawn_local(glib::clone!(
        #[strong]
        splash,
        #[strong]
        splash_label,
        #[strong]
        window,
        #[strong]
        row_info_model,
        #[strong]
        row_info_resolution,
        #[strong]
        row_info_hz,
        #[strong]
        row_info_firmware,
        #[strong]
        row_info_usage,
        #[strong]
        switch_audio_mute,
        #[strong]
        combo_input_source,
        #[strong]
        combo_output_range,
        #[strong]
        combo_picture_mode_ui,
        #[strong]
        combo_osd_language,
        #[strong]
        combo_custom_color_temp,
        #[strong]
        combo_custom_hdr,
        #[strong]
        combo_custom_gamma,
        #[strong]
        combo_custom_night_vision,
        #[strong]
        combo_custom_dynamic_od,
        #[strong]
        button_profile_custom,
        #[strong]
        monitor_control_widgets,
        #[strong]
        init,
        #[strong]
        app_cfg,
        #[strong]
        startup_finished,
        #[strong]
        active_picture_mode,
        #[strong]
        active_picture_mode_has_custom,
        #[strong]
        current_monitor_name,
        #[strong]
        load_custom_profile_for_mode,
        #[strong]
        sync_color_temp_controls,
        #[strong]
        sync_gaming_constraints,
        #[strong]
        sync_picture_mode_controls,
        #[strong]
        lang_cell,
        async move {
            while let Ok(msg) = ui_rx.recv().await {
                match msg {
                    monitor::UiCmd::MonitorFound(info) => {
                        let s = &info.settings;
                        *current_monitor_name.borrow_mut() = info.name.clone();

                        // Info rows
                        row_info_model.set_subtitle(&info.name);
                        row_info_resolution.set_subtitle(if info.resolution.is_empty() {
                            "-"
                        } else {
                            &info.resolution
                        });
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

                        if let Some(&val) = s.get(&monitor::VCP_OUTPUT_RANGE) {
                            combo_output_range.set_selected(monitor::vcp_to_combo_index(
                                &monitor::OUTPUT_RANGE_VALUES[..],
                                val,
                            ));
                        }

                        // Color temp (special mapping)
                        if let Some(&val) = s.get(&monitor::VCP_MODE) {
                            if let Some((mode_index, custom_active)) =
                                monitor::picture_mode_from_vcp(val)
                            {
                                combo_picture_mode_ui.set_selected(mode_index as u32);
                                active_picture_mode.set(mode_index as u16);
                                active_picture_mode_has_custom
                                    .set(monitor::picture_mode_has_custom(mode_index));
                                (sync_picture_mode_controls)(mode_index as u16, custom_active);
                            }
                        }

                        if let Some(&val) = s.get(&monitor::VCP_COLOR_TEMP) {
                            activate_toggle_value(
                                &combo_custom_color_temp,
                                &monitor::COLOR_TEMP_VALUES[..],
                                val,
                            );
                            sync_color_temp_controls();
                        } else {
                            sync_color_temp_controls();
                        }

                        if let Some(&val) = s.get(&monitor::VCP_HDR) {
                            activate_toggle_value(&combo_custom_hdr, &monitor::HDR_VALUES[..], val);
                        }

                        if let Some(&val) = s.get(&monitor::VCP_NIGHT_VISION) {
                            activate_toggle_value(
                                &combo_custom_night_vision,
                                &monitor::NIGHT_VISION_VALUES[..],
                                val,
                            );
                        }

                        if let Some(&val) = s.get(&monitor::VCP_DYNAMIC_OD) {
                            activate_toggle_value(
                                &combo_custom_dynamic_od,
                                &monitor::DYNAMIC_OD_VALUES[..],
                                val,
                            );
                        }

                        if let Some(&val) = s.get(&monitor::VCP_GAMMA) {
                            activate_toggle_value(
                                &combo_custom_gamma,
                                &monitor::GAMMA_VALUES[..],
                                val,
                            );
                        }

                        // OSD language (actual values from MCCS capability list)
                        if let Some(&val) = s.get(&monitor::VCP_OSD_LANG) {
                            combo_osd_language.set_selected(monitor::vcp_to_combo_index(
                                &monitor::OSD_LANGUAGE_VALUES[..],
                                val,
                            ));
                        }

                        // Input source uses MCCS values 15-18
                        if let Some(&val) = s.get(&monitor::VCP_INPUT_SOURCE) {
                            combo_input_source.set_selected(monitor::vcp_to_combo_index(
                                &monitor::INPUT_SOURCE_VALUES[..],
                                val,
                            ));
                        }

                        // Switches
                        for (sw, code, active_value) in &switch_binds {
                            if let Some(&val) = s.get(code) {
                                sw.set_active(val == *active_value);
                            }
                        }

                        // Mute (HKC: 2=mute, 1=unmute)
                        if let Some(&val) = s.get(&monitor::VCP_MUTE) {
                            switch_audio_mute.set_active(val > 1);
                        }

                        // Expanders
                        for (exp, code) in &expander_binds {
                            if let Some(&val) = s.get(code) {
                                exp.set_enable_expansion(val > 0);
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

                        for (btns, code, values) in &toggle_value_binds {
                            if let Some(&val) = s.get(code) {
                                if let Some(idx) = values.iter().position(|&mapped| mapped == val) {
                                    if idx < btns.len() {
                                        btns[idx].set_active(true);
                                    }
                                }
                            }
                        }

                        // Done - unblock signals
                        init.set(false);
                        sync_gaming_constraints();
                        (load_custom_profile_for_mode)(active_picture_mode.get());
                        startup_finished.set(true);
                        splash.close();
                        if !app_cfg.borrow().start_minimized {
                            window.present();
                        }
                    }
                    monitor::UiCmd::Progress(msg) => {
                        splash_label.set_label(&translate_startup_message(&lang_cell.get(), &msg));
                    }
                    monitor::UiCmd::Error(e) => {
                        startup_finished.set(true);
                        splash_label.set_label(&translate_startup_message(&lang_cell.get(), &e));
                        if !splash.is_visible() {
                            splash.present();
                        }
                        if e == "Monitor not found" {
                            splash.close();
                            if let Some(app) = window.application() {
                                app.quit();
                            }
                        }
                    }
                    monitor::UiCmd::Busy(busy) => {
                        for widget in &monitor_control_widgets {
                            widget.set_sensitive(!busy);
                        }
                        if !busy {
                            (sync_picture_mode_controls)(
                                active_picture_mode.get(),
                                button_profile_custom.is_active(),
                            );
                            sync_color_temp_controls();
                            sync_gaming_constraints();
                        }
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
        let switch_audio_mute_ref = switch_audio_mute.clone();
        switch_audio_mute.connect_state_set(move |_, state| {
            if init.get() || !switch_audio_mute_ref.is_sensitive() {
                return glib::Propagation::Proceed;
            }
            let val = if state { 2 } else { 1 };
            let _ = tx.send(WorkerCmd::SetSave(monitor::VCP_MUTE, val));
            glib::Propagation::Proceed
        });
    }

    // I/O & OSD
    {
        let tx = tx.clone();
        let init = init.clone();
        combo_input_source.connect_selected_notify(move |c| {
            if init.get() {
                return;
            }
            let idx = c.selected() as usize;
            if idx < monitor::INPUT_SOURCE_VALUES.len() {
                let _ = tx.send(WorkerCmd::Set(
                    monitor::VCP_INPUT_SOURCE,
                    monitor::INPUT_SOURCE_VALUES[idx],
                ));
            }
        });
    }
    {
        let tx = tx.clone();
        let init = init.clone();
        combo_output_range.connect_selected_notify(move |c| {
            if init.get() || !c.is_sensitive() {
                return;
            }
            let idx = c.selected() as usize;
            if idx >= monitor::OUTPUT_RANGE_VALUES.len() {
                return;
            }
            let val = monitor::OUTPUT_RANGE_VALUES[idx];
            if val == 0 {
                let _ = tx.send(WorkerCmd::SetSave(monitor::VCP_OUTPUT_RANGE, 0));
            } else {
                // Monitor requires reset to Auto (0) before switching to another mode
                let _ = tx.send(WorkerCmd::Set(monitor::VCP_OUTPUT_RANGE, 0));
                let tx = tx.clone();
                glib::timeout_add_local_once(Duration::from_millis(250), move || {
                    let _ = tx.send(WorkerCmd::SetSave(monitor::VCP_OUTPUT_RANGE, val));
                });
            }
        });
    }
    wire_switch_values(
        &switch_quick_boot,
        &tx,
        monitor::VCP_QUICK_BOOT,
        1,
        0,
        true,
        &init,
    );
    {
        let tx = tx.clone();
        let init = init.clone();
        combo_osd_language.connect_selected_notify(move |c| {
            if init.get() {
                return;
            }
            let idx = c.selected() as usize;
            if idx < monitor::OSD_LANGUAGE_VALUES.len() {
                let _ = tx.send(WorkerCmd::Set(
                    monitor::VCP_OSD_LANG,
                    monitor::OSD_LANGUAGE_VALUES[idx],
                ));
            }
        });
    }
    wire_scale(&scale_osd_time, &tx, monitor::VCP_OSD_TIME, true, &init);
    wire_scale(
        &scale_osd_h_position,
        &tx,
        monitor::VCP_OSD_H_POS,
        true,
        &init,
    );
    wire_scale(
        &scale_osd_v_position,
        &tx,
        monitor::VCP_OSD_V_POS,
        true,
        &init,
    );
    wire_scale(
        &scale_osd_transparency,
        &tx,
        monitor::VCP_OSD_TRANS,
        true,
        &init,
    );

    // Power Saving (Off=1, Lvl1=2, Lvl2=3, Lvl3=4)
    {
        let ps = vec![
            button_power_save_off.clone(),
            button_power_save_lvl1.clone(),
            button_power_save_lvl2.clone(),
        ];
        wire_toggles(&ps, &tx, monitor::VCP_POWER_SAVING, true, 1, &init);
    }

    // Power LED (Off=1, Lvl1=2, Lvl2=3, Lvl3=4)
    {
        let pl = vec![
            button_led_off.clone(),
            button_led_lvl1.clone(),
            button_led_lvl2.clone(),
            button_led_lvl3.clone(),
        ];
        wire_toggles(&pl, &tx, monitor::VCP_POWER_LED, true, 1, &init);
    }

    // Resets & Power Off
    {
        let tx = tx.clone();
        button_power_off.connect_clicked(move |_| {
            let _ = tx.send(WorkerCmd::Set(monitor::VCP_DPMS, 0x04));
        });
    }
    {
        let tx = tx.clone();
        button_reset_factory.connect_clicked(move |_| {
            let _ = tx.send(WorkerCmd::Set(monitor::VCP_RESET_FACTORY, 1));
        });
    }
    // ===== PROFILE TAB =====

    {
        let tx = tx.clone();
        let init = init.clone();
        let active_picture_mode = active_picture_mode.clone();
        let active_picture_mode_has_custom = active_picture_mode_has_custom.clone();
        let app_cfg = app_cfg.clone();
        let persist_current_custom_profile_state = persist_current_custom_profile_state.clone();
        let load_custom_profile_for_mode = load_custom_profile_for_mode.clone();
        let button_profile_custom = button_profile_custom.clone();
        combo_picture_mode.connect_selected_notify(move |c| {
            let new_mode = c.selected() as u16;

            let mut cfg = app_cfg.borrow().clone();
            cfg.last_picture_mode = Some(new_mode);
            let _ = app_settings::save(&cfg);
            *app_cfg.borrow_mut() = cfg;

            if init.get() {
                active_picture_mode.set(new_mode);
                active_picture_mode_has_custom
                    .set(monitor::picture_mode_has_custom(new_mode as usize));
                return;
            }

            (persist_current_custom_profile_state)();
            active_picture_mode.set(new_mode);
            active_picture_mode_has_custom.set(monitor::picture_mode_has_custom(new_mode as usize));
            let requested_custom =
                active_picture_mode_has_custom.get() && button_profile_custom.is_active();
            if let Some(mode_value) =
                monitor::picture_mode_vcp_value(new_mode as usize, requested_custom)
            {
                let _ = tx.send(WorkerCmd::Set(monitor::VCP_MODE, mode_value));
            }
            (load_custom_profile_for_mode)(new_mode);
        });
    }
    wire_scale(
        &scale_custom_brightness,
        &tx,
        monitor::VCP_BRIGHTNESS,
        false,
        &init,
    );
    wire_scale(
        &scale_custom_contrast,
        &tx,
        monitor::VCP_CONTRAST,
        false,
        &init,
    );
    wire_scale(
        &scale_custom_sharpness,
        &tx,
        monitor::VCP_SHARPNESS,
        false,
        &init,
    );
    wire_scale(
        &scale_custom_shadow_balance,
        &tx,
        monitor::VCP_SHADOW_BALANCE,
        true,
        &init,
    );
    wire_scale(
        &scale_custom_cr_enhance,
        &tx,
        monitor::VCP_CR_ENHANCE,
        true,
        &init,
    );
    wire_scale(
        &scale_custom_color_enhance,
        &tx,
        monitor::VCP_COLOR_ENHANCE,
        true,
        &init,
    );
    wire_scale(
        &scale_custom_super_res,
        &tx,
        monitor::VCP_SUPER_RES,
        true,
        &init,
    );
    wire_scale(
        &scale_custom_low_blue_light,
        &tx,
        monitor::VCP_LOW_BLUE,
        false,
        &init,
    );

    // Color temperature (special mapping)
    {
        let tx = tx.clone();
        let init_c = init.clone();
        let sync_color_temp_controls = sync_color_temp_controls.clone();
        let scale_custom_red_gain = scale_custom_red_gain.clone();
        let scale_custom_green_gain = scale_custom_green_gain.clone();
        let scale_custom_blue_gain = scale_custom_blue_gain.clone();
        for (idx, button) in combo_custom_color_temp.iter().enumerate() {
            let tx = tx.clone();
            let init_c = init_c.clone();
            let sync_color_temp_controls = sync_color_temp_controls.clone();
            let scale_custom_red_gain = scale_custom_red_gain.clone();
            let scale_custom_green_gain = scale_custom_green_gain.clone();
            let scale_custom_blue_gain = scale_custom_blue_gain.clone();
            button.connect_toggled(move |button| {
                if !button.is_active() {
                    return;
                }

                sync_color_temp_controls();
                if init_c.get() {
                    return;
                }
                if idx < monitor::COLOR_TEMP_VALUES.len() {
                    let color_temp = monitor::COLOR_TEMP_VALUES[idx];
                    let _ = tx.send(WorkerCmd::Set(monitor::VCP_COLOR_TEMP, color_temp));

                    if let Some((red_code, green_code, blue_code)) =
                        monitor::color_temp_rgb_codes(color_temp)
                    {
                        let _ = tx.send(WorkerCmd::Set(
                            red_code,
                            scale_custom_red_gain.value() as u16,
                        ));
                        let _ = tx.send(WorkerCmd::Set(
                            green_code,
                            scale_custom_green_gain.value() as u16,
                        ));
                        let _ = tx.send(WorkerCmd::Set(
                            blue_code,
                            scale_custom_blue_gain.value() as u16,
                        ));
                    }
                }
            });
        }
    }

    wire_color_temp_rgb_scale(
        &scale_custom_red_gain,
        &tx,
        &combo_custom_color_temp,
        0,
        &init,
    );
    wire_color_temp_rgb_scale(
        &scale_custom_green_gain,
        &tx,
        &combo_custom_color_temp,
        1,
        &init,
    );
    wire_color_temp_rgb_scale(
        &scale_custom_blue_gain,
        &tx,
        &combo_custom_color_temp,
        2,
        &init,
    );
    wire_toggle_values(
        &combo_custom_hdr,
        &tx,
        monitor::VCP_HDR,
        &monitor::HDR_VALUES[..],
        true,
        &init,
    );
    wire_toggle_values(
        &combo_custom_gamma,
        &tx,
        monitor::VCP_GAMMA,
        &monitor::GAMMA_VALUES[..],
        false,
        &init,
    );
    wire_toggle_values(
        &combo_custom_night_vision,
        &tx,
        monitor::VCP_NIGHT_VISION,
        &monitor::NIGHT_VISION_VALUES[..],
        true,
        &init,
    );
    wire_toggle_values(
        &combo_custom_dynamic_od,
        &tx,
        monitor::VCP_DYNAMIC_OD,
        &monitor::DYNAMIC_OD_VALUES[..],
        true,
        &init,
    );

    // ===== GAMING TAB =====

    // Game Aid
    wire_toggle_values(
        &buttons_screen_size,
        &tx,
        monitor::VCP_SCREEN_SIZE,
        &[0u16, 1, 2][..],
        true,
        &init,
    );
    wire_expander(
        &expander_fps_counter,
        &tx,
        monitor::VCP_FPS_COUNTER,
        true,
        &init,
    );
    wire_toggle_values(
        &buttons_fps_pos,
        &tx,
        monitor::VCP_FPS_POS,
        &[0u16, 1, 2, 3][..],
        true,
        &init,
    );
    wire_expander(
        &expander_crosshair,
        &tx,
        monitor::VCP_CROSSHAIR,
        true,
        &init,
    );
    wire_toggles(
        &buttons_crosshair_shape,
        &tx,
        monitor::VCP_CROSSHAIR_SHAPE,
        true,
        1,
        &init,
    );
    wire_toggle_values(
        &buttons_crosshair_color,
        &tx,
        monitor::VCP_CROSSHAIR_COLOR,
        &[0u16, 1, 2, 3, 4, 5, 6, 7][..],
        true,
        &init,
    );
    wire_expander(
        &expander_stopwatch,
        &tx,
        monitor::VCP_STOPWATCH,
        true,
        &init,
    );
    wire_toggles(
        &buttons_stopwatch_time,
        &tx,
        monitor::VCP_STOPWATCH_TIME,
        true,
        1,
        &init,
    );
    wire_toggle_values(
        &buttons_stopwatch_pos,
        &tx,
        monitor::VCP_STOPWATCH_POS,
        &[0u16, 1, 2, 3][..],
        true,
        &init,
    );
    wire_expander(
        &expander_game_time,
        &tx,
        monitor::VCP_GAME_TIME,
        true,
        &init,
    );
    wire_toggles(
        &buttons_game_time_val,
        &tx,
        monitor::VCP_GAME_TIME_VAL,
        true,
        1,
        &init,
    );
    wire_toggle_values(
        &buttons_game_time_pos,
        &tx,
        monitor::VCP_GAME_TIME_POS,
        &[0u16, 1, 2, 3][..],
        true,
        &init,
    );
    wire_expander(
        &expander_magnifier,
        &tx,
        monitor::VCP_MAGNIFIER,
        true,
        &init,
    );
    wire_switch_values(
        &switch_magnifier_night_vision,
        &tx,
        monitor::VCP_MAGNIFIER_NV,
        1,
        0,
        true,
        &init,
    );
    wire_toggle_values(
        &buttons_magnifier_zoom,
        &tx,
        monitor::VCP_MAGNIFIER_ZOOM,
        &[0u16, 1, 2][..],
        true,
        &init,
    );
    wire_toggles(
        &buttons_magnifier_size,
        &tx,
        monitor::VCP_MAGNIFIER_SIZE,
        true,
        1,
        &init,
    );
    wire_toggle_values(
        &buttons_magnifier_pos,
        &tx,
        monitor::VCP_MAGNIFIER_POS,
        &[1u16, 2, 5, 3, 4][..],
        true,
        &init,
    );
    wire_switch_values(
        &switch_alignment,
        &tx,
        monitor::VCP_ALIGNMENT,
        1,
        0,
        true,
        &init,
    );
    wire_expander(&expander_hawkeye, &tx, monitor::VCP_HAWKEYE, true, &init);
    wire_toggle_values(
        &buttons_hawkeye_size,
        &tx,
        monitor::VCP_HAWKEYE_SIZE,
        &[0u16, 1, 2][..],
        true,
        &init,
    );
    wire_toggle_values(
        &buttons_hawkeye_pos,
        &tx,
        monitor::VCP_HAWKEYE_POS,
        &[0u16, 1, 2, 3, 4][..],
        true,
        &init,
    );
    wire_toggle_values(
        &buttons_hawkeye_lvl,
        &tx,
        monitor::VCP_HAWKEYE_LEVEL,
        &[0u16, 1, 2, 3, 4][..],
        true,
        &init,
    );

    // Picture Enhance
    wire_switch(
        &switch_gaming_rush,
        &tx,
        monitor::VCP_GAME_RUSH,
        true,
        &init,
    );
    wire_switch_values(
        &switch_gaming_async,
        &tx,
        monitor::VCP_ADAPTIVE_SYNC,
        1,
        0,
        true,
        &init,
    );
    wire_toggle_values(
        &buttons_gaming_local_dimming,
        &tx,
        monitor::VCP_LOCAL_DIMMING,
        &monitor::LOCAL_DIMMING_VALUES[..],
        true,
        &init,
    );
    wire_toggle_values(
        &buttons_gaming_dyds,
        &tx,
        monitor::VCP_DYDS,
        &monitor::DYDS_VALUES[..],
        true,
        &init,
    );
    wire_toggle_values(
        &buttons_gaming_night_vision,
        &tx,
        monitor::VCP_NIGHT_VISION,
        &monitor::NIGHT_VISION_VALUES[..],
        true,
        &init,
    );
    wire_toggle_values(
        &buttons_gaming_dynamic_od,
        &tx,
        monitor::VCP_DYNAMIC_OD,
        &monitor::DYNAMIC_OD_VALUES[..],
        true,
        &init,
    );
    wire_toggle_values(
        &buttons_gaming_hdr,
        &tx,
        monitor::VCP_HDR,
        &monitor::HDR_VALUES[..],
        true,
        &init,
    );
    for (scale, code) in hue_scales_vector.iter().zip(monitor::HUE_CODES.iter()) {
        wire_scale(scale, &tx, *code, false, &init);
    }
    for (scale, code) in saturation_scales_vector
        .iter()
        .zip(monitor::SATURATION_CODES.iter())
    {
        wire_scale(scale, &tx, *code, false, &init);
    }
    wire_scale(
        &scale_gaming_color_enhance,
        &tx,
        monitor::VCP_COLOR_ENHANCE,
        true,
        &init,
    );
    wire_scale(
        &scale_gaming_cr_enhance,
        &tx,
        monitor::VCP_CR_ENHANCE,
        true,
        &init,
    );
    wire_scale(
        &scale_gaming_shadow_enhance,
        &tx,
        monitor::VCP_SHADOW_BALANCE,
        true,
        &init,
    );
    wire_scale(
        &scale_gaming_super_res,
        &tx,
        monitor::VCP_SUPER_RES,
        true,
        &init,
    );
    wire_scale(
        &scale_gaming_halo_control,
        &tx,
        monitor::VCP_HALO_CONTROL,
        true,
        &init,
    );

    // ===== APP SETTINGS =====

    {
        let app_cfg = app_cfg.clone();
        switch_auto_start.connect_state_set(move |_, state| {
            let mut cfg = app_cfg.borrow().clone();
            cfg.auto_start = state;
            let _ = app_settings::save(&cfg);
            let _ = app_settings::sync_autostart(&cfg);
            *app_cfg.borrow_mut() = cfg;
            glib::Propagation::Proceed
        });
    }

    {
        let app_cfg = app_cfg.clone();
        switch_start_minimized.connect_state_set(move |_, state| {
            let mut cfg = app_cfg.borrow().clone();
            cfg.start_minimized = state;
            let _ = app_settings::save(&cfg);
            *app_cfg.borrow_mut() = cfg;
            glib::Propagation::Proceed
        });
    }

    {
        let switch_start_minimized = switch_start_minimized.clone();
        switch_auto_start.connect_active_notify(move |sw| {
            switch_start_minimized.set_sensitive(sw.is_active());
        });
    }

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
