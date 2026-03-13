use crate::i18n::{AppLang, LangUpdaters, tr};
use crate::monitor;
use crate::ui::helpers::*;
use adw::prelude::*;
use adw::{ActionRow, ComboRow, PreferencesGroup, PreferencesPage};
use gtk4::{
    Align, Box as GtkBox, Grid, Label, Orientation, Revealer, Scale, StringList, ToggleButton,
};
use libadwaita as adw;

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

pub fn build(
    lang: &AppLang,
    u: &LangUpdaters,
) -> (
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
    Vec<ToggleButton>,
    Scale,
    Scale,
    Scale,
    Vec<ToggleButton>,
    Vec<ToggleButton>,
    Vec<ToggleButton>,
    Vec<ToggleButton>,
    Vec<Scale>,
    Vec<Scale>,
) {
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
    let sshb = create_scale();
    let sce = create_scale_with_max(5.0);
    let scoe = create_scale_with_max(10.0);
    let ssr = create_scale_with_max(5.0);
    let slbl = create_scale_with_max(4.0);

    let lab_keys = [
        "brightness",
        "contrast",
        "sharpness",
        "shadow_balance",
        "cr_enhance",
        "color_enhance",
        "super_res",
        "low_blue_light",
    ];
    let all_w = [&sb, &sc, &ss, &sshb, &sce, &scoe, &ssr, &slbl];
    for i in 0..8 {
        let lbl = Label::builder()
            .label(&tr(lang, lab_keys[i]))
            .halign(Align::Start)
            .width_request(120)
            .build();
        tr_label(u, &lbl, lab_keys[i], lang);
        grid.attach(&lbl, 0, i as i32, 1, 1);
        grid.attach(&create_scale_control(all_w[i]), 1, i as i32, 1, 1);
    }
    g_cust.add(&grid);
    custom_container.append(&g_cust_wrap);

    let (g_temp_wrap, g_temp) = create_collapsible_group(u, "color_temp", lang);
    let (box_temp, combo_temp) =
        create_linked_buttons(&["Warm", "Natural", "Cold", "User 1", "User 2", "User 3"][..]);
    let row_temp = ActionRow::new();
    tr_row(u, &row_temp, "color_temp_profile", lang);
    row_temp.add_suffix(&box_temp);
    let r_r = ActionRow::new();
    let sr = create_scale();
    tr_row(u, &r_r, "red", lang);
    r_r.add_suffix(&create_scale_control(&sr));
    let r_g = ActionRow::new();
    let sg = create_scale();
    tr_row(u, &r_g, "green", lang);
    r_g.add_suffix(&create_scale_control(&sg));
    let r_bl = ActionRow::new();
    let sbl = create_scale();
    tr_row(u, &r_bl, "blue", lang);
    r_bl.add_suffix(&create_scale_control(&sbl));
    g_temp.add(&row_temp);
    g_temp.add(&r_r);
    g_temp.add(&r_g);
    g_temp.add(&r_bl);
    custom_container.append(&g_temp_wrap);

    let (g_list_wrap, g_list) = create_collapsible_group(u, "lists_group", lang);
    let (box_hdr, c_hdr) = create_linked_buttons(&["Disabled", "Auto", "Game", "Movie"][..]);
    let r_hdr = ActionRow::new();
    tr_row(u, &r_hdr, "hdr", lang);
    r_hdr.add_suffix(&box_hdr);
    let (box_gam, c_gam) =
        create_linked_buttons(&["1.8", "2.0", "2.2", "2.4", "2.6", "S Curve"][..]);
    let r_gam = ActionRow::new();
    tr_row(u, &r_gam, "gamma", lang);
    r_gam.add_suffix(&box_gam);
    let (box_nv, c_nv) =
        create_linked_buttons(&["Disabled", "Lvl 1", "Lvl 2", "Auto-L1", "Auto-L2"][..]);
    let r_nv = ActionRow::new();
    tr_row(u, &r_nv, "night_vision", lang);
    r_nv.add_suffix(&box_nv);
    let (box_od, c_od) =
        create_linked_buttons(&["Disabled", "Lvl 1", "Lvl 2", "Lvl 3", "Topspeed"][..]);
    let r_od = ActionRow::new();
    tr_row(u, &r_od, "dynamic_od", lang);
    r_od.add_suffix(&box_od);
    g_list.add(&r_hdr);
    g_list.add(&r_gam);
    g_list.add(&r_nv);
    g_list.add(&r_od);
    custom_container.append(&g_list_wrap);

    let mut h_v = Vec::new();
    let mut s_v = Vec::new();
    let (g_hue_wrap, g_hue) = create_collapsible_group(u, "hue", lang);
    let (g_sat_wrap, g_sat) = create_collapsible_group(u, "saturation", lang);
    let colors = ["R", "G", "B", "C", "M", "Y"];

    let grid_hue = Grid::builder().column_spacing(20).row_spacing(12).build();
    let grid_sat = Grid::builder().column_spacing(20).row_spacing(12).build();
    for i in 0..6 {
        let h = create_scale();
        let s = create_scale();
        let lbl_h = Label::builder()
            .label(colors[i])
            .halign(Align::Start)
            .width_request(120)
            .build();
        let lbl_s = Label::builder()
            .label(colors[i])
            .halign(Align::Start)
            .width_request(120)
            .build();
        grid_hue.attach(&lbl_h, 0, i as i32, 1, 1);
        grid_hue.attach(&create_scale_control(&h), 1, i as i32, 1, 1);
        grid_sat.attach(&lbl_s, 0, i as i32, 1, 1);
        grid_sat.attach(&create_scale_control(&s), 1, i as i32, 1, 1);
        h_v.push(h);
        s_v.push(s);
    }
    g_hue.add(&grid_hue);
    g_sat.add(&grid_sat);
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
        combo_temp,
        sr,
        sg,
        sbl,
        c_hdr,
        c_gam,
        c_nv,
        c_od,
        h_v,
        s_v,
    )
}
