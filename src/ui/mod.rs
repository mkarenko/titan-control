pub mod helpers;
pub mod tabs;

use crate::app_settings;
use crate::i18n::{AppLang, LangUpdaters, tr};
use adw::prelude::*;
use adw::{ActionRow, ComboRow, HeaderBar, ViewStack, ViewSwitcher};
use gtk4::{Box as GtkBox, Button, Orientation, Revealer, Scale, Switch, ToggleButton};
use libadwaita::{self as adw, ExpanderRow};

#[allow(dead_code)]
pub struct MainWidgets {
    pub window: adw::ApplicationWindow,
    pub combo_lang: ComboRow,
    pub combo_theme: ComboRow,
    pub switch_auto_start: Switch,
    pub switch_start_minimized: Switch,

    // --- SYSTEM (tab_display) ---
    pub row_info_model: ActionRow,
    pub row_info_resolution: ActionRow,
    pub row_info_hz: ActionRow,
    pub row_info_firmware: ActionRow,
    pub row_info_usage: ActionRow,
    pub scale_audio_volume: Scale,
    pub switch_audio_mute: Switch,
    pub button_power_off: Button,
    pub button_power_save_off: ToggleButton,
    pub button_power_save_lvl1: ToggleButton,
    pub button_power_save_lvl2: ToggleButton,
    pub button_led_off: ToggleButton,
    pub button_led_lvl1: ToggleButton,
    pub button_led_lvl2: ToggleButton,
    pub button_led_lvl3: ToggleButton,
    pub combo_input_source: ComboRow,
    pub combo_output_range: ComboRow,
    pub switch_quick_boot: Switch,
    pub combo_osd_language: ComboRow,
    pub scale_osd_time: Scale,
    pub scale_osd_h_position: Scale,
    pub scale_osd_v_position: Scale,
    pub scale_osd_transparency: Scale,
    pub button_reset_factory: Button,

    // --- PROFILE (tab_profiles) ---
    pub combo_picture_mode: ComboRow,
    pub button_profile_default: ToggleButton,
    pub button_profile_custom: ToggleButton,
    pub custom_revealer: Revealer,
    pub scale_custom_brightness: Scale,
    pub scale_custom_contrast: Scale,
    pub scale_custom_sharpness: Scale,
    pub scale_custom_shadow_balance: Scale,
    pub scale_custom_cr_enhance: Scale,
    pub scale_custom_color_enhance: Scale,
    pub scale_custom_super_res: Scale,
    pub scale_custom_low_blue_light: Scale,
    pub combo_custom_color_temp: Vec<ToggleButton>,
    pub scale_custom_red_gain: Scale,
    pub scale_custom_green_gain: Scale,
    pub scale_custom_blue_gain: Scale,
    pub combo_custom_hdr: Vec<ToggleButton>,
    pub combo_custom_gamma: Vec<ToggleButton>,
    pub combo_custom_night_vision: Vec<ToggleButton>,
    pub combo_custom_dynamic_od: Vec<ToggleButton>,
    pub hue_scales_vector: Vec<Scale>,
    pub saturation_scales_vector: Vec<Scale>,

    // --- GAMING (tab_gaming) ---
    pub row_screen_size: ActionRow,
    pub buttons_screen_size: Vec<ToggleButton>,
    pub expander_fps_counter: ExpanderRow,
    pub buttons_fps_pos: Vec<ToggleButton>,
    pub expander_crosshair: ExpanderRow,
    pub buttons_crosshair_shape: Vec<ToggleButton>,
    pub buttons_crosshair_color: Vec<ToggleButton>,
    pub expander_stopwatch: ExpanderRow,
    pub buttons_stopwatch_time: Vec<ToggleButton>,
    pub buttons_stopwatch_pos: Vec<ToggleButton>,
    pub expander_game_time: ExpanderRow,
    pub buttons_game_time_val: Vec<ToggleButton>,
    pub buttons_game_time_pos: Vec<ToggleButton>,
    pub expander_magnifier: ExpanderRow,
    pub switch_magnifier_night_vision: Switch,
    pub buttons_magnifier_zoom: Vec<ToggleButton>,
    pub buttons_magnifier_size: Vec<ToggleButton>,
    pub buttons_magnifier_pos: Vec<ToggleButton>,
    pub switch_alignment: Switch,
    pub expander_hawkeye: ExpanderRow,
    pub buttons_hawkeye_size: Vec<ToggleButton>,
    pub buttons_hawkeye_pos: Vec<ToggleButton>,
    pub buttons_hawkeye_lvl: Vec<ToggleButton>,
    pub switch_gaming_async: Switch,
    pub switch_gaming_rush: Switch,
    pub buttons_gaming_local_dimming: Vec<ToggleButton>,
    pub buttons_gaming_dyds: Vec<ToggleButton>,
    pub buttons_gaming_night_vision: Vec<ToggleButton>,
    pub buttons_gaming_dynamic_od: Vec<ToggleButton>,
    pub buttons_gaming_hdr: Vec<ToggleButton>,
    pub scale_gaming_color_enhance: Scale,
    pub scale_gaming_cr_enhance: Scale,
    pub scale_gaming_shadow_enhance: Scale,
    pub scale_gaming_super_res: Scale,
    pub scale_gaming_halo_control: Scale,
}

pub fn build_ui(app: &adw::Application, lang: &AppLang, updaters: &LangUpdaters) -> MainWidgets {
    if let Some(display) = gtk4::gdk::Display::default() {
        let icon_theme = gtk4::IconTheme::for_display(&display);
        for icon_path in app_settings::icon_search_paths() {
            icon_theme.add_search_path(&icon_path);
        }
    }

    let css = gtk4::CssProvider::new();
    css.load_from_data(concat!(
        ".linked > button.toggle {",
        "  min-height: 32px;",
        "  min-width: 0;",
        "  padding: 4px 12px;",
        "  font-size: 13px;",
        "}",
        ".scale-stepper > button {",
        "  min-height: 32px;",
        "  min-width: 32px;",
        "  padding: 0;",
        "}",
        ".linked > button.color-btn.toggle {",
        "  min-width: 42px;",
        "  padding: 0;",
        "  border: 1px solid alpha(@window_fg_color, 0.3);",
        "}",
        ".crosshair-shape-btn image {",
        "  min-width: 28px;",
        "  min-height: 28px;",
        "}",
        ".crosshair-shape-btn { padding: 2px 8px; }",
        ".color-btn.color-red { background-color: #ff0000; }",
        ".color-btn.color-yel { background-color: #ffff00; }",
        ".color-btn.color-grn { background-color: #00ff00; }",
        ".color-btn.color-cya { background-color: #00ffff; }",
        ".color-btn.color-blu { background-color: #0066ff; }",
        ".color-btn.color-pur { background-color: #cc00ff; }",
        ".color-btn.color-wht { background-color: #ffffff; }",
        ".color-btn:checked { outline: 2px solid @accent_color; outline-offset: -2px; }",
    ));
    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().expect("display"),
        &css,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let stack = ViewStack::new();
    let header = HeaderBar::builder()
        .decoration_layout(":minimize,maximize,close")
        .build();
    let switcher = ViewSwitcher::builder()
        .stack(&stack)
        .policy(adw::ViewSwitcherPolicy::Wide)
        .build();
    header.set_title_widget(Some(&switcher));

    // 1. SYSTEM
    let (
        p_main,
        sv,
        sw_mute,
        im,
        ir,
        rh,
        rf,
        ru,
        bo,
        bps_off,
        bps_l1,
        bps_l2,
        bpl_off,
        bpl_l1,
        bpl_l2,
        bpl_l3,
        ci,
        cr,
        swb,
        cl,
        st,
        sh,
        sv_osd,
        str_osd,
        b1,
    ) = tabs::tab_display::build(lang, updaters);

    // 2. PROFILE
    let (
        p_pre,
        cm,
        bdef,
        bcust,
        rev,
        sb,
        sc,
        ss,
        sshb,
        sce,
        scoe,
        ssr,
        slbl,
        ct,
        sr,
        sg,
        sbl,
        chdr,
        cgam,
        cnv,
        cod,
        h_v,
        s_v,
    ) = tabs::tab_profiles::build(lang, updaters);

    // 3. GAMING
    let (
        p_gam,
        e_size,
        b_size,
        e_hz,
        b_hz,
        e_cr,
        b_cr_s,
        b_cr_c,
        e_st,
        b_st_t,
        b_st_p,
        e_gt,
        b_gt_t,
        b_gt_p,
        e_mag,
        s_mag_n,
        b_mag_z,
        b_mag_s,
        b_mag_p,
        sw_al,
        e_hawk,
        b_hawk_s,
        b_hawk_p,
        b_hawk_l,
        sw_a,
        sw_r,
        b_dim,
        b_dyds,
        b_nv_e,
        b_od_e,
        b_hdr_e,
        s_col_e,
        s_cr_e,
        s_sh_e,
        s_sr_e,
        s_halo_e,
    ) = tabs::tab_gaming::build(lang, updaters);

    let (p_inf, c_lang, c_theme, sw_auto_start, sw_start_minimized) =
        tabs::tab_info::build(lang, updaters);

    stack
        .add_titled(&p_main, Some("main"), &tr(lang, "tab_display"))
        .set_icon_name(Some("tab-monitor-symbolic"));
    stack
        .add_titled(&p_pre, Some("profiles"), &tr(lang, "tab_profiles"))
        .set_icon_name(Some("tab-profiles-symbolic"));
    stack
        .add_titled(&p_gam, Some("gaming"), &tr(lang, "tab_gaming"))
        .set_icon_name(Some("tab-gaming-symbolic"));
    stack
        .add_titled(&p_inf, Some("info"), &tr(lang, "tab_info"))
        .set_icon_name(Some("tab-info-symbolic"));

    {
        let s = stack.clone();
        updaters.borrow_mut().push(Box::new(move |l| {
            if let Some(pg) = s.child_by_name("main") {
                s.page(&pg).set_title(Some(&tr(l, "tab_display")));
            }
            if let Some(pg) = s.child_by_name("profiles") {
                s.page(&pg).set_title(Some(&tr(l, "tab_profiles")));
            }
            if let Some(pg) = s.child_by_name("gaming") {
                s.page(&pg).set_title(Some(&tr(l, "tab_gaming")));
            }
            if let Some(pg) = s.child_by_name("info") {
                s.page(&pg).set_title(Some(&tr(l, "tab_info")));
            }
        }));
    }

    let layout = GtkBox::new(Orientation::Vertical, 0);
    layout.append(&header);
    layout.append(&stack);

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .icon_name(app_settings::APP_ICON_NAME)
        .default_width(650)
        .default_height(800)
        .content(&layout)
        .build();

    MainWidgets {
        window,
        combo_lang: c_lang,
        combo_theme: c_theme,
        switch_auto_start: sw_auto_start,
        switch_start_minimized: sw_start_minimized,
        row_info_model: im,
        row_info_resolution: ir,
        row_info_hz: rh,
        row_info_firmware: rf,
        row_info_usage: ru,
        scale_audio_volume: sv,
        switch_audio_mute: sw_mute,
        button_power_off: bo,
        button_power_save_off: bps_off,
        button_power_save_lvl1: bps_l1,
        button_power_save_lvl2: bps_l2,
        button_led_off: bpl_off,
        button_led_lvl1: bpl_l1,
        button_led_lvl2: bpl_l2,
        button_led_lvl3: bpl_l3,
        combo_input_source: ci,
        combo_output_range: cr,
        switch_quick_boot: swb,
        combo_osd_language: cl,
        scale_osd_time: st,
        scale_osd_h_position: sh,
        scale_osd_v_position: sv_osd,
        scale_osd_transparency: str_osd,
        button_reset_factory: b1,
        combo_picture_mode: cm,
        button_profile_default: bdef,
        button_profile_custom: bcust,
        custom_revealer: rev,
        scale_custom_brightness: sb,
        scale_custom_contrast: sc,
        scale_custom_sharpness: ss,
        scale_custom_shadow_balance: sshb,
        scale_custom_cr_enhance: sce,
        scale_custom_color_enhance: scoe,
        scale_custom_super_res: ssr,
        scale_custom_low_blue_light: slbl,
        combo_custom_color_temp: ct,
        scale_custom_red_gain: sr,
        scale_custom_green_gain: sg,
        scale_custom_blue_gain: sbl,
        combo_custom_hdr: chdr,
        combo_custom_gamma: cgam,
        combo_custom_night_vision: cnv,
        combo_custom_dynamic_od: cod,
        hue_scales_vector: h_v,
        saturation_scales_vector: s_v,
        row_screen_size: e_size,
        buttons_screen_size: b_size,
        expander_fps_counter: e_hz,
        buttons_fps_pos: b_hz,
        expander_crosshair: e_cr,
        buttons_crosshair_shape: b_cr_s,
        buttons_crosshair_color: b_cr_c,
        expander_stopwatch: e_st,
        buttons_stopwatch_time: b_st_t,
        buttons_stopwatch_pos: b_st_p,
        expander_game_time: e_gt,
        buttons_game_time_val: b_gt_t,
        buttons_game_time_pos: b_gt_p,
        expander_magnifier: e_mag,
        switch_magnifier_night_vision: s_mag_n,
        buttons_magnifier_zoom: b_mag_z,
        buttons_magnifier_size: b_mag_s,
        buttons_magnifier_pos: b_mag_p,
        switch_alignment: sw_al,
        expander_hawkeye: e_hawk,
        buttons_hawkeye_size: b_hawk_s,
        buttons_hawkeye_pos: b_hawk_p,
        buttons_hawkeye_lvl: b_hawk_l,
        switch_gaming_async: sw_a,
        switch_gaming_rush: sw_r,
        buttons_gaming_local_dimming: b_dim,
        buttons_gaming_dyds: b_dyds,
        buttons_gaming_night_vision: b_nv_e,
        buttons_gaming_dynamic_od: b_od_e,
        buttons_gaming_hdr: b_hdr_e,
        scale_gaming_color_enhance: s_col_e,
        scale_gaming_cr_enhance: s_cr_e,
        scale_gaming_shadow_enhance: s_sh_e,
        scale_gaming_super_res: s_sr_e,
        scale_gaming_halo_control: s_halo_e,
    }
}
