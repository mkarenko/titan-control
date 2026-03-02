use crate::monitor;
use crate::ui::helpers::create_scale;
use adw::prelude::*;
use adw::{ActionRow, ComboRow, PreferencesGroup, PreferencesPage};
use gtk4::{
    Adjustment, Align, Box as GtkBox, Grid, Label, Orientation, Revealer, Scale, StringList,
    ToggleButton,
};
use libadwaita as adw;

pub fn build() -> (
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
        .title("Profile")
        .icon_name("emblem-favorite-symbolic")
        .build();
    let g_mode = PreferencesGroup::builder()
        .title("Tryb Wyświetlania")
        .build();
    let combo_mode = ComboRow::builder()
        .title("Aktywny tryb")
        .model(&StringList::new(&monitor::PICTURE_MODE_NAMES[..]))
        .build();
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

    let g_cust = PreferencesGroup::builder()
        .title("Ustawienia Ręczne")
        .build();
    let grid = Grid::builder().column_spacing(20).row_spacing(12).build();
    let sb = create_scale();
    let sc = create_scale();
    let ss = Scale::builder()
        .adjustment(&Adjustment::new(0.0, 0.0, 5.0, 1.0, 1.0, 0.0))
        .build();
    let sshb = create_scale();
    let sce = Scale::builder()
        .adjustment(&Adjustment::new(0.0, 0.0, 5.0, 1.0, 1.0, 0.0))
        .build();
    let scoe = Scale::builder()
        .adjustment(&Adjustment::new(0.0, 0.0, 10.0, 1.0, 1.0, 0.0))
        .build();
    let ssr = Scale::builder()
        .adjustment(&Adjustment::new(0.0, 0.0, 5.0, 1.0, 1.0, 0.0))
        .build();
    let slbl = Scale::builder()
        .adjustment(&Adjustment::new(0.0, 0.0, 4.0, 1.0, 1.0, 0.0))
        .build();

    let labs = [
        "Jasność",
        "Kontrast",
        "Ostrość",
        "Shadow Balance",
        "CR Enhance",
        "Color Enhance",
        "Super Res",
        "Low Blue Light",
    ];
    let all_w = [&sb, &sc, &ss, &sshb, &sce, &scoe, &ssr, &slbl];
    for i in 0..8 {
        grid.attach(
            &Label::builder()
                .label(labs[i])
                .halign(Align::Start)
                .width_request(120)
                .build(),
            0,
            i as i32,
            1,
            1,
        );
        grid.attach(all_w[i], 1, i as i32, 1, 1);
    }
    g_cust.add(&grid);
    custom_container.append(&g_cust);

    let g_temp = PreferencesGroup::builder()
        .title("Temperatura Kolorów")
        .build();
    let combo_temp = ComboRow::builder()
        .title("Profil")
        .model(&StringList::new(
            &["Warm", "Cold", "Natural", "User1", "User2", "User3"][..],
        ))
        .build();
    let (r_r, sr) = (ActionRow::new(), create_scale());
    r_r.set_title("Czerwony");
    r_r.add_suffix(&sr);
    let (r_g, sg) = (ActionRow::new(), create_scale());
    r_g.set_title("Zielony");
    r_g.add_suffix(&sg);
    let (r_bl, sbl) = (ActionRow::new(), create_scale());
    r_bl.set_title("Niebieski");
    r_bl.add_suffix(&sbl);
    g_temp.add(&combo_temp);
    g_temp.add(&r_r);
    g_temp.add(&r_g);
    g_temp.add(&r_bl);
    custom_container.append(&g_temp);

    let g_list = PreferencesGroup::builder().title("Listy Wyboru").build();
    let c_hdr = ComboRow::builder()
        .title("HDR")
        .model(&StringList::new(&["Disabled", "Auto", "Game", "Movie"][..]))
        .build();
    let c_gam = ComboRow::builder()
        .title("Gamma")
        .model(&StringList::new(
            &["1.8", "2.0", "2.2", "2.4", "2.6", "S.curve"][..],
        ))
        .build();
    let c_nv = ComboRow::builder()
        .title("Night Vision")
        .model(&StringList::new(
            &["Disabled", "Lvl 1", "Lvl 2", "Lvl 3", "Auto-L1", "Auto-L2"][..],
        ))
        .build();
    let c_od = ComboRow::builder()
        .title("Dynamic OD")
        .model(&StringList::new(
            &["Disabled", "Lvl 1", "Lvl 2", "Lvl 3", "Topspeed"][..],
        ))
        .build();
    g_list.add(&c_hdr);
    g_list.add(&c_gam);
    g_list.add(&c_nv);
    g_list.add(&c_od);
    custom_container.append(&g_list);

    let mut h_v = Vec::new();
    let mut s_v = Vec::new();
    let g_hue = PreferencesGroup::builder().title("Hue").build();
    let g_sat = PreferencesGroup::builder()
        .title("Color Saturation")
        .build();
    let colors = ["R", "G", "B", "C", "M", "Y"];
    for i in 0..6 {
        let h = create_scale();
        let s = create_scale();
        h_v.push(h);
        s_v.push(s);
    }
    // ... (dodanie do gridów jak wcześniej)

    let g_wrapper = PreferencesGroup::builder()
        .title("Konfiguracja Custom")
        .build();
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
