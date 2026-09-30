use crate::i18n::{AppLang, LangUpdaters, tr};
use crate::monitor;
use crate::ui::helpers::*;
use adw::prelude::*;
use adw::{ActionRow, ComboRow, PreferencesGroup, PreferencesPage};
use gtk4::{
    Align, Box as GtkBox, Grid, Label, Orientation, Revealer, Scale, StringList, Switch,
    ToggleButton,
};
use libadwaita as adw;

const PROFILE_LABEL_WIDTH: i32 = 170;

pub type ProfileTabWidgets = (
    PreferencesPage,
    ComboRow,
    ToggleButton,
    ToggleButton,
    Revealer,
    Scale,
    Scale,
    Scale,
    Scale,
    Scale,
    Scale,
    Scale,
    Scale,
    Switch,
    Vec<ToggleButton>,
    Scale,
    Scale,
    Scale,
    Vec<ToggleButton>,
    Vec<ToggleButton>,
    Vec<ToggleButton>,
    Vec<ToggleButton>,
    Vec<ToggleButton>,
    Vec<Scale>,
    Vec<Scale>,
    Switch,
    Switch,
    Vec<ToggleButton>,
    Scale,
);

fn create_linked_buttons(labels: &[&str]) -> (GtkBox, Vec<ToggleButton>) {
    let container = GtkBox::builder()
        .css_classes(["linked"])
        .halign(Align::End)
        .valign(Align::Center)
        .build();
    let mut buttons = Vec::new();

    let first_btn = ToggleButton::builder()
        .label(labels[0])
        .active(true)
        .build();
    container.append(&first_btn);
    buttons.push(first_btn.clone());

    for label in &labels[1..] {
        let btn = ToggleButton::builder()
            .label(*label)
            .group(&first_btn)
            .build();
        container.append(&btn);
        buttons.push(btn);
    }

    (container, buttons)
}

pub fn build(lang: &AppLang, u: &LangUpdaters) -> ProfileTabWidgets {
    let page = PreferencesPage::builder().build();
    tr_page(u, &page, "tab_profiles", lang);

    let g_mode = PreferencesGroup::new();
    tr_group(u, &g_mode, "display_mode", lang);
    let combo_mode = ComboRow::builder()
        .model(&StringList::new(&monitor::PICTURE_MODE_NAMES[..]))
        .build();
    tr_row(
        u,
        &combo_mode.clone().upcast::<ActionRow>(),
        "active_mode",
        lang,
    );
    g_mode.add(&combo_mode);

    let btn_box = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .halign(Align::Center)
        .css_classes(["linked"])
        .margin_top(12)
        .build();
    let btn_default = ToggleButton::builder()
        .label("Default")
        .active(true)
        .build();
    let btn_custom = ToggleButton::builder()
        .label("Custom")
        .group(&btn_default)
        .build();
    btn_box.append(&btn_default);
    btn_box.append(&btn_custom);
    g_mode.add(&btn_box);
    page.add(&g_mode);

    let custom_revealer = Revealer::builder()
        .transition_type(gtk4::RevealerTransitionType::SlideDown)
        .build();
    let custom_container = GtkBox::new(Orientation::Vertical, 18);
    custom_revealer.set_child(Some(&custom_container));

    let (g_cust_wrap, g_cust) = create_collapsible_group(u, "manual_settings", lang);
    let grid = Grid::builder().column_spacing(20).row_spacing(12).build();
    let sb = create_scale();
    let sc = create_scale();
    let ss = create_scale_with_max(5.0);
    let slbl = create_scale_with_max(4.0);

    let lab_keys = [
        "brightness",
        "contrast",
        "sharpness",
        "color_enhance",
        "cr_enhance",
        "shadow_balance",
        "super_res",
        "halo_control",
        "low_blue_light",
    ];
    let scoe = create_scale_with_max(10.0);
    let sce = create_scale_with_max(5.0);
    let sshb = create_scale();
    let ssr = create_scale_with_max(5.0);
    let s_halo = create_scale();
    let all_w = [&sb, &sc, &ss, &scoe, &sce, &sshb, &ssr, &s_halo, &slbl];
    for i in 0..9 {
        let lbl = Label::builder()
            .label(tr(lang, lab_keys[i]))
            .halign(Align::Start)
            .xalign(0.0)
            .width_request(PROFILE_LABEL_WIDTH)
            .build();
        tr_label(u, &lbl, lab_keys[i], lang);
        grid.attach(&lbl, 0, i as i32, 1, 1);
        grid.attach(&create_scale_control(all_w[i]), 1, i as i32, 1, 1);
    }
    let sw_dcr = Switch::builder().valign(Align::Center).build();
    let lbl_dcr = Label::builder()
        .label(tr(lang, "dcr"))
        .halign(Align::Start)
        .xalign(0.0)
        .width_request(PROFILE_LABEL_WIDTH)
        .build();
    tr_label(u, &lbl_dcr, "dcr", lang);
    let dcr_box = GtkBox::builder()
        .halign(Align::End)
        .valign(Align::Center)
        .build();
    dcr_box.append(&sw_dcr);
    grid.attach(&lbl_dcr, 0, 9, 1, 1);
    grid.attach(&dcr_box, 1, 9, 1, 1);
    let manual_frame = GtkBox::builder()
        .css_classes(["profile-grid-frame"])
        .build();
    manual_frame.append(&grid);
    g_cust.add(&manual_frame);
    custom_container.append(&g_cust_wrap);

    let (g_temp_wrap, g_temp) = create_collapsible_group(u, "color_temp", lang);
    let (box_temp, combo_temp) =
        create_linked_buttons(&["Warm", "Natural", "Cold", "User 1", "User 2", "User 3"][..]);
    let row_temp = ActionRow::new();
    tr_row(u, &row_temp, "color_temp_profile", lang);
    row_temp.add_suffix(&box_temp);
    let sr = create_scale();
    let sg = create_scale();
    let sbl = create_scale();
    g_temp.add(&row_temp);

    let grid_temp = Grid::builder().column_spacing(20).row_spacing(12).build();
    let temp_labels = ["red", "green", "blue"];
    let temp_scales = [&sr, &sg, &sbl];
    for (i, key) in temp_labels.iter().enumerate() {
        let lbl = Label::builder()
            .label(tr(lang, key))
            .halign(Align::Start)
            .xalign(0.0)
            .width_request(PROFILE_LABEL_WIDTH)
            .build();
        tr_label(u, &lbl, key, lang);
        grid_temp.attach(&lbl, 0, i as i32, 1, 1);
        grid_temp.attach(&create_scale_control(temp_scales[i]), 1, i as i32, 1, 1);
    }
    let temp_frame = GtkBox::builder()
        .css_classes(["profile-grid-frame"])
        .build();
    temp_frame.append(&grid_temp);
    g_temp.add(&temp_frame);

    let (g_enh_wrap, g_enh) = create_collapsible_group(u, "pic_enhance", lang);

    let sw_async = Switch::builder().valign(Align::Center).build();
    let r_async = ActionRow::new();
    tr_row(u, &r_async, "adaptive_sync", lang);
    r_async.add_suffix(&sw_async);

    let sw_rush = Switch::builder().valign(Align::Center).build();
    let r_rush = ActionRow::new();
    tr_row(u, &r_rush, "game_rush", lang);
    r_rush.add_suffix(&sw_rush);

    let (box_dim, btn_dim) =
        create_linked_buttons(&["Disabled", "Low", "Smooth", "Medium", "High"][..]);
    let r_dim = ActionRow::new();
    tr_row(u, &r_dim, "local_dimming", lang);
    r_dim.add_suffix(&box_dim);

    let (box_dyds, c_dyds) =
        create_linked_buttons(&["Off", "Low", "Med", "High", "ULL-1", "ULL-2", "ULL-3"][..]);
    let r_dyds = ActionRow::new();
    tr_row(u, &r_dyds, "dyds", lang);
    r_dyds.add_suffix(&box_dyds);

    let (_box_nv, c_nv) =
        create_linked_buttons(&["Off", "Lvl 1", "Lvl 2", "Auto-Lvl 1", "Auto-Lvl 2"][..]);

    let (box_od, c_od) =
        create_linked_buttons(&["Off", "Lvl 1", "Lvl 2", "Lvl 3", "Top Speed"][..]);
    let r_od = ActionRow::new();
    tr_row(u, &r_od, "dynamic_od", lang);
    r_od.add_suffix(&box_od);

    let (box_hdr, c_hdr) = create_linked_buttons(&["Off", "Auto", "Game", "Movie"][..]);
    let r_hdr = ActionRow::new();
    tr_row(u, &r_hdr, "hdr", lang);
    r_hdr.add_suffix(&box_hdr);

    let (box_gam, c_gam) =
        create_linked_buttons(&["1.8", "2.0", "2.2", "2.4", "2.6", "S Curve"][..]);
    let r_gam = ActionRow::new();
    tr_row(u, &r_gam, "gamma", lang);
    r_gam.add_suffix(&box_gam);

    g_enh.add(&r_async);
    g_enh.add(&r_rush);
    g_enh.add(&r_hdr);
    g_enh.add(&r_gam);
    g_enh.add(&r_dim);
    g_enh.add(&r_od);
    g_enh.add(&r_dyds);
    custom_container.append(&g_enh_wrap);
    custom_container.append(&g_temp_wrap);

    let mut h_v = Vec::new();
    let mut s_v = Vec::new();
    let (g_hue_wrap, g_hue) = create_collapsible_group(u, "hue", lang);
    let (g_sat_wrap, g_sat) = create_collapsible_group(u, "saturation", lang);
    let color_keys = [
        "axis_red",
        "axis_green",
        "axis_blue",
        "axis_cyan",
        "axis_magenta",
        "axis_yellow",
    ];

    let grid_hue = Grid::builder().column_spacing(20).row_spacing(12).build();
    let grid_sat = Grid::builder().column_spacing(20).row_spacing(12).build();
    for (i, color_key) in color_keys.iter().enumerate() {
        let h = create_scale();
        let s = create_scale();
        let lbl_h = Label::builder()
            .label(tr(lang, color_key))
            .halign(Align::Start)
            .xalign(0.0)
            .width_request(PROFILE_LABEL_WIDTH)
            .build();
        tr_label(u, &lbl_h, color_key, lang);
        let lbl_s = Label::builder()
            .label(tr(lang, color_key))
            .halign(Align::Start)
            .xalign(0.0)
            .width_request(PROFILE_LABEL_WIDTH)
            .build();
        tr_label(u, &lbl_s, color_key, lang);
        grid_hue.attach(&lbl_h, 0, i as i32, 1, 1);
        grid_hue.attach(&create_scale_control(&h), 1, i as i32, 1, 1);
        grid_sat.attach(&lbl_s, 0, i as i32, 1, 1);
        grid_sat.attach(&create_scale_control(&s), 1, i as i32, 1, 1);
        h_v.push(h);
        s_v.push(s);
    }
    let hue_frame = GtkBox::builder()
        .css_classes(["profile-grid-frame"])
        .build();
    hue_frame.append(&grid_hue);
    g_hue.add(&hue_frame);
    let sat_frame = GtkBox::builder()
        .css_classes(["profile-grid-frame"])
        .build();
    sat_frame.append(&grid_sat);
    g_sat.add(&sat_frame);
    custom_container.append(&g_hue_wrap);
    custom_container.append(&g_sat_wrap);

    let g_wrapper = PreferencesGroup::new();
    g_wrapper.add(&custom_revealer);
    page.add(&g_wrapper);

    (
        page,
        combo_mode,
        btn_default,
        btn_custom,
        custom_revealer,
        sb,
        sc,
        ss,
        sshb,
        sce,
        scoe,
        ssr,
        slbl,
        sw_dcr,
        combo_temp,
        sr,
        sg,
        sbl,
        c_hdr,
        c_gam,
        c_nv,
        c_od,
        c_dyds,
        h_v,
        s_v,
        sw_async,
        sw_rush,
        btn_dim,
        s_halo,
    )
}
