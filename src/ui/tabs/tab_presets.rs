use crate::i18n::{tr, AppLang, LangUpdaters};
use crate::monitor;
use crate::ui::helpers::*;
use adw::prelude::*;
use adw::{ActionRow, ComboRow, PreferencesGroup, PreferencesPage};
use gtk4::{Align, Box as GtkBox, Grid, Label, Orientation, Revealer, Scale, StringList, ToggleButton};
use libadwaita as adw;

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
    ComboRow,
    Scale,
    Scale,
    Scale,
    ComboRow,
    ComboRow,
    ComboRow,
    ComboRow,
    Vec<Scale>,
    Vec<Scale>,
) {
    let page = PreferencesPage::builder()
        .icon_name("emblem-favorite-symbolic")
        .build();
    tr_page(u, &page, "tab_profiles", lang);

    let g_mode = PreferencesGroup::new();
    tr_group(u, &g_mode, "display_mode", lang);
    let combo_mode = ComboRow::builder()
        .model(&StringList::new(&monitor::PICTURE_MODE_NAMES[..]))
        .build();
    tr_row(u, &combo_mode.clone().upcast::<ActionRow>(), "active_mode", lang);
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

    let g_cust = PreferencesGroup::new();
    tr_group(u, &g_cust, "manual_settings", lang);
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
        grid.attach(all_w[i], 1, i as i32, 1, 1);
    }
    g_cust.add(&grid);
    custom_container.append(&g_cust);

    let g_temp = PreferencesGroup::new();
    tr_group(u, &g_temp, "color_temp", lang);
    let combo_temp = ComboRow::builder()
        .model(&StringList::new(
            &["Warm", "Cold", "Natural", "User1", "User2", "User3"][..],
        ))
        .build();
    tr_row(u, &combo_temp.clone().upcast::<ActionRow>(), "color_temp_profile", lang);
    let r_r = ActionRow::new();
    let sr = create_scale();
    tr_row(u, &r_r, "red", lang);
    r_r.add_suffix(&sr);
    let r_g = ActionRow::new();
    let sg = create_scale();
    tr_row(u, &r_g, "green", lang);
    r_g.add_suffix(&sg);
    let r_bl = ActionRow::new();
    let sbl = create_scale();
    tr_row(u, &r_bl, "blue", lang);
    r_bl.add_suffix(&sbl);
    g_temp.add(&combo_temp);
    g_temp.add(&r_r);
    g_temp.add(&r_g);
    g_temp.add(&r_bl);
    custom_container.append(&g_temp);

    let g_list = PreferencesGroup::new();
    tr_group(u, &g_list, "lists_group", lang);
    let c_hdr = ComboRow::builder()
        .model(&StringList::new(&["Disabled", "Auto", "Game", "Movie"][..]))
        .build();
    tr_row(u, &c_hdr.clone().upcast::<ActionRow>(), "hdr", lang);
    let c_gam = ComboRow::builder()
        .model(&StringList::new(
            &["1.8", "2.0", "2.2", "2.4", "2.6", "S.curve"][..],
        ))
        .build();
    tr_row(u, &c_gam.clone().upcast::<ActionRow>(), "gamma", lang);
    let c_nv = ComboRow::builder()
        .model(&StringList::new(
            &["Disabled", "Lvl 1", "Lvl 2", "Lvl 3", "Auto-L1", "Auto-L2"][..],
        ))
        .build();
    tr_row(u, &c_nv.clone().upcast::<ActionRow>(), "night_vision", lang);
    let c_od = ComboRow::builder()
        .model(&StringList::new(
            &["Disabled", "Lvl 1", "Lvl 2", "Lvl 3", "Topspeed"][..],
        ))
        .build();
    tr_row(u, &c_od.clone().upcast::<ActionRow>(), "dynamic_od", lang);
    g_list.add(&c_hdr);
    g_list.add(&c_gam);
    g_list.add(&c_nv);
    g_list.add(&c_od);
    custom_container.append(&g_list);

    let mut h_v = Vec::new();
    let mut s_v = Vec::new();
    let _g_hue = PreferencesGroup::new();
    tr_group(u, &_g_hue, "hue", lang);
    let _g_sat = PreferencesGroup::new();
    tr_group(u, &_g_sat, "saturation", lang);
    let _colors = ["R", "G", "B", "C", "M", "Y"];
    for _i in 0..6 {
        let h = create_scale();
        let s = create_scale();
        h_v.push(h);
        s_v.push(s);
    }

    let g_wrapper = PreferencesGroup::new();
    tr_group(u, &g_wrapper, "custom_config", lang);
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
