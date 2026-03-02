use crate::ui::helpers::create_scale;
use adw::prelude::*;
use adw::{ActionRow, ComboRow, ExpanderRow, PreferencesGroup, PreferencesPage};
use gtk4::{Adjustment, Orientation, Scale, StringList, Switch};
use libadwaita as adw;

pub fn build() -> (
    PreferencesPage,
    ExpanderRow,
    ComboRow, // Full Game (DualMode)
    ExpanderRow,
    ComboRow, // Refresh Rate
    ExpanderRow,
    ComboRow,
    ComboRow, // Crosshair
    ExpanderRow,
    ComboRow,
    ComboRow, // Stopwatch
    ExpanderRow,
    ComboRow,
    ComboRow, // Game Time
    ExpanderRow,
    Switch,
    ComboRow,
    ComboRow,
    ComboRow, // Magnifier
    ExpanderRow,
    ComboRow, // Alignment
    ExpanderRow,
    ComboRow,
    ComboRow,
    ComboRow, // Hawkeye
    // Picture Enhance
    Switch,
    Switch,
    ComboRow,
    ComboRow,
    ComboRow,
    ComboRow,
    ComboRow,
    Scale,
    Scale,
    Scale,
    Scale,
    Scale,
) {
    let page = PreferencesPage::builder()
        .title("Gaming")
        .icon_name("input-gaming-symbolic")
        .build();

    // --- SEKCJA: GAME AID ---
    let g_aid = PreferencesGroup::builder().title("Game Aid").build();

    // 1. Full Game (Screen Size)
    let exp_size = ExpanderRow::builder()
        .title("Rozmiar ekranu (DualMode)")
        .show_enable_switch(true)
        .build();
    let c_size = ComboRow::builder()
        .title("Format")
        .model(&StringList::new(&["Wide", "25\"", "sPX"][..]))
        .build();
    exp_size.add_row(&c_size);

    // 2. Refresh Rate
    let exp_hz = ExpanderRow::builder()
        .title("Licznik FPS/Hz")
        .show_enable_switch(true)
        .build();
    let c_hz_pos = ComboRow::builder()
        .title("Pozycja")
        .model(&StringList::new(
            &["Top Right", "Top Left", "Bottom Right", "Bottom Left"][..],
        ))
        .build();
    exp_hz.add_row(&c_hz_pos);

    // 3. Crosshair
    let exp_cross = ExpanderRow::builder()
        .title("Celownik (Crosshair)")
        .show_enable_switch(true)
        .build();
    let c_cross_shape = ComboRow::builder()
        .title("Kształt")
        .model(&StringList::new(
            &["Typ 1", "Typ 2", "Typ 3", "Typ 4", "Typ 5", "Typ 6"][..],
        ))
        .build();
    let c_cross_color = ComboRow::builder()
        .title("Kolor")
        .model(&StringList::new(
            &[
                "Red", "Yellow", "Green", "Cyan", "Blue", "Purple", "White", "Auto",
            ][..],
        ))
        .build();
    exp_cross.add_row(&c_cross_shape);
    exp_cross.add_row(&c_cross_color);

    // 4. Stopwatch
    let exp_stop = ExpanderRow::builder()
        .title("Stoper")
        .show_enable_switch(true)
        .build();
    let c_stop_time = ComboRow::builder()
        .title("Czas (min)")
        .model(&StringList::new(&["15", "30", "45", "60"][..]))
        .build();
    let c_stop_pos = ComboRow::builder()
        .title("Pozycja")
        .model(&StringList::new(
            &["Top Right", "Top Left", "Bottom Right", "Bottom Left"][..],
        ))
        .build();
    exp_stop.add_row(&c_stop_time);
    exp_stop.add_row(&c_stop_pos);

    // 5. Game Time
    let exp_gt = ExpanderRow::builder()
        .title("Czas gry")
        .show_enable_switch(true)
        .build();
    let c_gt_time = ComboRow::builder()
        .title("Czas (min)")
        .model(&StringList::new(&["15", "30", "45", "60"][..]))
        .build();
    let c_gt_pos = ComboRow::builder()
        .title("Pozycja")
        .model(&StringList::new(
            &["Top Right", "Top Left", "Bottom Right", "Bottom Left"][..],
        ))
        .build();
    exp_gt.add_row(&c_gt_time);
    exp_gt.add_row(&c_gt_pos);

    // 6. Magnifier Mode
    let exp_mag = ExpanderRow::builder()
        .title("Lupa (Magnifier)")
        .show_enable_switch(true)
        .build();
    let sw_mag_nv = Switch::builder().valign(gtk4::Align::Center).build();
    let r_mag_nv = ActionRow::builder().title("Night Vision").build();
    r_mag_nv.add_suffix(&sw_mag_nv);
    let c_mag_zoom = ComboRow::builder()
        .title("Powiększenie")
        .model(&StringList::new(&["x1.5", "x2", "x4"][..]))
        .build();
    let c_mag_size = ComboRow::builder()
        .title("Rozmiar")
        .model(&StringList::new(&["Small", "Medium", "Large"][..]))
        .build();
    let c_mag_pos = ComboRow::builder()
        .title("Pozycja")
        .model(&StringList::new(
            &["Top Right", "Top Left", "Bottom Right", "Bottom Left"][..],
        ))
        .build();
    exp_mag.add_row(&r_mag_nv);
    exp_mag.add_row(&c_mag_zoom);
    exp_mag.add_row(&c_mag_size);
    exp_mag.add_row(&c_mag_pos);

    // 7. Alignment Aid
    let exp_align = ExpanderRow::builder()
        .title("Wspomaganie wyrównania")
        .show_enable_switch(true)
        .build();
    let c_align = ComboRow::builder()
        .title("Opcja")
        .model(&StringList::new(&["Opcja 1", "Opcja 2"][..]))
        .build();
    exp_align.add_row(&c_align);

    // 8. Hawkeye Vision
    let exp_hawk = ExpanderRow::builder()
        .title("Hawkeye Vision")
        .show_enable_switch(true)
        .build();
    let c_hawk_size = ComboRow::builder()
        .title("Rozmiar")
        .model(&StringList::new(&["Small", "Medium", "Large"][..]))
        .build();
    let c_hawk_pos = ComboRow::builder()
        .title("Pozycja")
        .model(&StringList::new(
            &["Top Right", "Top Left", "Bottom Right", "Bottom Left"][..],
        ))
        .build();
    let c_hawk_lvl = ComboRow::builder()
        .title("Poziom")
        .model(&StringList::new(
            &["Level 1", "Level 2", "Level 3", "Level 4", "Level 5"][..],
        ))
        .build();
    exp_hawk.add_row(&c_hawk_size);
    exp_hawk.add_row(&c_hawk_pos);
    exp_hawk.add_row(&c_hawk_lvl);

    g_aid.add(&exp_size);
    g_aid.add(&exp_hz);
    g_aid.add(&exp_cross);
    g_aid.add(&exp_stop);
    g_aid.add(&exp_gt);
    g_aid.add(&exp_mag);
    g_aid.add(&exp_align);
    g_aid.add(&exp_hawk);

    // --- SEKCJA: PICTURE ENHANCE ---
    let g_enh = PreferencesGroup::builder().title("Picture Enhance").build();

    let sw_async = Switch::builder().valign(gtk4::Align::Center).build();
    let r_async = ActionRow::builder().title("Adaptive-Sync").build();
    r_async.add_suffix(&sw_async);

    let sw_rush = Switch::builder().valign(gtk4::Align::Center).build();
    let r_rush = ActionRow::builder().title("Game Rush").build();
    r_rush.add_suffix(&sw_rush);

    let c_dim = ComboRow::builder()
        .title("Local Dimming")
        .model(&StringList::new(
            &["Disabled", "Low", "Smooth", "Medium", "High"][..],
        ))
        .build();
    let c_dyds = ComboRow::builder()
        .title("DyDs")
        .model(&StringList::new(
            &[
                "Disabled",
                "Low",
                "Medium",
                "High",
                "UII-Level 1",
                "UII-Level 2",
                "UII-Level 3",
            ][..],
        ))
        .build();
    let c_nv = ComboRow::builder()
        .title("Night Vision")
        .model(&StringList::new(
            &[
                "Disabled",
                "Level 1",
                "Level 2",
                "Auto-Level 1",
                "Auto-Level 2",
            ][..],
        ))
        .build();
    let c_od = ComboRow::builder()
        .title("Dynamic OD")
        .model(&StringList::new(
            &["Disabled", "Level 1", "Level 2", "Level 3", "Topspeed"][..],
        ))
        .build();
    let c_hdr = ComboRow::builder()
        .title("HDR")
        .model(&StringList::new(&["Disabled", "Auto", "Game", "Movie"][..]))
        .build();

    let s_col = create_scale(); // 0-10
    let r_col = ActionRow::builder().title("Color Enhance").build();
    r_col.add_suffix(&s_col);

    let s_cr = Scale::builder()
        .adjustment(&Adjustment::new(0.0, 0.0, 5.0, 1.0, 1.0, 0.0))
        .orientation(Orientation::Horizontal)
        .hexpand(true)
        .build();
    let r_cr = ActionRow::builder().title("CR Enhance").build();
    r_cr.add_suffix(&s_cr);

    let s_sh = create_scale(); // 0-100
    let r_sh = ActionRow::builder().title("Shadow Enhance").build();
    r_sh.add_suffix(&s_sh);

    let s_sr = Scale::builder()
        .adjustment(&Adjustment::new(0.0, 0.0, 5.0, 1.0, 1.0, 0.0))
        .orientation(Orientation::Horizontal)
        .hexpand(true)
        .build();
    let r_sr = ActionRow::builder().title("SR (Super Resolution)").build();
    r_sr.add_suffix(&s_sr);

    let s_halo = create_scale(); // 0-100
    let r_halo = ActionRow::builder().title("Halo Control").build();
    r_halo.add_suffix(&s_halo);

    g_enh.add(&r_async);
    g_enh.add(&r_rush);
    g_enh.add(&c_dim);
    g_enh.add(&c_dyds);
    g_enh.add(&c_nv);
    g_enh.add(&c_od);
    g_enh.add(&c_hdr);
    g_enh.add(&r_col);
    g_enh.add(&r_cr);
    g_enh.add(&r_sh);
    g_enh.add(&r_sr);
    g_enh.add(&r_halo);

    page.add(&g_aid);
    page.add(&g_enh);

    (
        page,
        exp_size,
        c_size,
        exp_hz,
        c_hz_pos,
        exp_cross,
        c_cross_shape,
        c_cross_color,
        exp_stop,
        c_stop_time,
        c_stop_pos,
        exp_gt,
        c_gt_time,
        c_gt_pos,
        exp_mag,
        sw_mag_nv,
        c_mag_zoom,
        c_mag_size,
        c_mag_pos,
        exp_align,
        c_align,
        exp_hawk,
        c_hawk_size,
        c_hawk_pos,
        c_hawk_lvl,
        sw_async,
        sw_rush,
        c_dim,
        c_dyds,
        c_nv,
        c_od,
        c_hdr,
        s_col,
        s_cr,
        s_sh,
        s_sr,
        s_halo,
    )
}
