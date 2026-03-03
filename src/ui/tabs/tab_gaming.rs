use crate::i18n::{AppLang, LangUpdaters};
use crate::ui::helpers::*;
use adw::prelude::*;
use adw::{ActionRow, ExpanderRow, PreferencesGroup, PreferencesPage};
use gtk4::{Align, Box as GtkBox, Scale, Switch, ToggleButton};
use libadwaita as adw;

// Helper to create a linked box of toggle buttons
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

#[allow(clippy::type_complexity)]
pub fn build(
    lang: &AppLang,
    u: &LangUpdaters,
) -> (
    PreferencesPage,
    ActionRow,
    Vec<ToggleButton>, // Full Game: r_size, btn_size
    ExpanderRow,
    Vec<ToggleButton>, // Refresh Rate: exp_hz, btn_hz_pos
    ExpanderRow,
    Vec<ToggleButton>,
    Vec<ToggleButton>, // Crosshair: exp_cross, btn_cross_shape, btn_cross_color
    ExpanderRow,
    Vec<ToggleButton>,
    Vec<ToggleButton>, // Stopwatch: exp_stop, btn_stop_time, btn_stop_pos
    ExpanderRow,
    Vec<ToggleButton>,
    Vec<ToggleButton>, // Game Time: exp_gt, btn_gt_time, btn_gt_pos
    ExpanderRow,
    Switch,
    Vec<ToggleButton>,
    Vec<ToggleButton>,
    Vec<ToggleButton>, // Magnifier
    Switch, // Alignment
    ExpanderRow,
    Vec<ToggleButton>,
    Vec<ToggleButton>,
    Vec<ToggleButton>, // Hawkeye
    // Picture Enhance
    Switch,
    Switch,            // async, rush
    Vec<ToggleButton>, // Local Dimming
    Vec<ToggleButton>, // DyDs
    Vec<ToggleButton>, // Night Vision
    Vec<ToggleButton>, // Dynamic OD
    Vec<ToggleButton>, // HDR
    Scale,
    Scale,
    Scale,
    Scale,
    Scale, // Scales
) {
    let page = PreferencesPage::builder()
        .icon_name("input-gaming-symbolic")
        .build();
    tr_page(u, &page, "tab_gaming", lang);

    // --- SEKCJA: GAME AID ---
    let g_aid = PreferencesGroup::new();
    tr_group(u, &g_aid, "game_aid", lang);

    // 1. Full Game
    let (box_size, btn_size) = create_linked_buttons(&["Wide", "25\"", "sPX"][..]);
    let r_size = ActionRow::new();
    tr_row(u, &r_size, "screen_size", lang);
    r_size.add_suffix(&box_size);

    // 2. Refresh Rate
    let exp_hz = ExpanderRow::builder()
        .show_enable_switch(true)
        .build();
    tr_expander(u, &exp_hz, "fps_counter", lang);
    let (box_hz_pos, btn_hz_pos) =
        create_linked_buttons(&["Top Right", "Top Left", "Bottom Right", "Bottom Left"][..]);
    let r_hz_pos = ActionRow::new();
    tr_row(u, &r_hz_pos, "position", lang);
    r_hz_pos.add_suffix(&box_hz_pos);
    exp_hz.add_row(&r_hz_pos);

    // 3. Crosshair
    let exp_cross = ExpanderRow::builder()
        .show_enable_switch(true)
        .build();
    tr_expander(u, &exp_cross, "crosshair", lang);
    let (box_cr_shape, btn_cross_shape) =
        create_linked_buttons(&["1", "2", "3", "4", "5", "6"][..]);
    let r_cr_shape = ActionRow::new();
    tr_row(u, &r_cr_shape, "shape", lang);
    r_cr_shape.add_suffix(&box_cr_shape);
    let (box_cr_color, btn_cross_color) = {
        let colors = [
            ("color-red", ""),
            ("color-yel", ""),
            ("color-grn", ""),
            ("color-cya", ""),
            ("color-blu", ""),
            ("color-pur", ""),
            ("color-wht", ""),
            ("", "Auto"),
        ];
        let container = GtkBox::builder()
            .css_classes(["linked"])
            .halign(Align::End)
            .valign(Align::Center)
            .build();
        let mut buttons = Vec::new();
        let first = ToggleButton::builder().active(true).build();
        if !colors[0].0.is_empty() {
            first.add_css_class("color-btn");
            first.add_css_class(colors[0].0);
        } else {
            first.set_label(colors[0].1);
        }
        container.append(&first);
        buttons.push(first.clone());
        for &(css_class, label) in &colors[1..] {
            let btn = ToggleButton::builder().group(&first).build();
            if !css_class.is_empty() {
                btn.add_css_class("color-btn");
                btn.add_css_class(css_class);
            } else {
                btn.set_label(label);
            }
            container.append(&btn);
            buttons.push(btn);
        }
        (container, buttons)
    };
    let r_cr_color = ActionRow::new();
    tr_row(u, &r_cr_color, "color", lang);
    r_cr_color.add_suffix(&box_cr_color);
    exp_cross.add_row(&r_cr_shape);
    exp_cross.add_row(&r_cr_color);

    // 4. Stopwatch
    let exp_stop = ExpanderRow::builder()
        .show_enable_switch(true)
        .build();
    tr_expander(u, &exp_stop, "stopwatch", lang);
    let (box_st_time, btn_stop_time) = create_linked_buttons(&["15", "30", "45", "60"][..]);
    let r_st_time = ActionRow::new();
    tr_row(u, &r_st_time, "time_min", lang);
    r_st_time.add_suffix(&box_st_time);
    let (box_st_pos, btn_stop_pos) =
        create_linked_buttons(&["Top Right", "Top Left", "Bottom Right", "Bottom Left"][..]);
    let r_st_pos = ActionRow::new();
    tr_row(u, &r_st_pos, "position", lang);
    r_st_pos.add_suffix(&box_st_pos);
    exp_stop.add_row(&r_st_time);
    exp_stop.add_row(&r_st_pos);

    // 5. Game Time
    let exp_gt = ExpanderRow::builder()
        .show_enable_switch(true)
        .build();
    tr_expander(u, &exp_gt, "game_time", lang);
    let (box_gt_time, btn_gt_time) = create_linked_buttons(&["15", "30", "45", "60"][..]);
    let r_gt_time = ActionRow::new();
    tr_row(u, &r_gt_time, "time_min", lang);
    r_gt_time.add_suffix(&box_gt_time);
    let (box_gt_pos, btn_gt_pos) =
        create_linked_buttons(&["Top Right", "Top Left", "Bottom Right", "Bottom Left"][..]);
    let r_gt_pos = ActionRow::new();
    tr_row(u, &r_gt_pos, "position", lang);
    r_gt_pos.add_suffix(&box_gt_pos);
    exp_gt.add_row(&r_gt_time);
    exp_gt.add_row(&r_gt_pos);

    // 6. Magnifier
    let exp_mag = ExpanderRow::builder()
        .show_enable_switch(true)
        .build();
    tr_expander(u, &exp_mag, "magnifier", lang);
    let sw_mag_nv = Switch::builder().valign(Align::Center).build();
    let r_mag_nv = ActionRow::new();
    tr_row(u, &r_mag_nv, "night_vision", lang);
    r_mag_nv.add_suffix(&sw_mag_nv);
    let (box_mag_zoom, btn_mag_zoom) = create_linked_buttons(&["x1.5", "x2", "x4"][..]);
    let r_mag_zoom = ActionRow::new();
    tr_row(u, &r_mag_zoom, "zoom", lang);
    r_mag_zoom.add_suffix(&box_mag_zoom);
    let (box_mag_size, btn_mag_size) = create_linked_buttons(&["Small", "Medium", "Large"][..]);
    let r_mag_size = ActionRow::new();
    tr_row(u, &r_mag_size, "size", lang);
    r_mag_size.add_suffix(&box_mag_size);
    let (box_mag_pos, btn_mag_pos) = create_linked_buttons(
        &["Top Right", "Top Left", "Central", "Bottom Right", "Bottom Left"][..],
    );
    let r_mag_pos = ActionRow::new();
    tr_row(u, &r_mag_pos, "position", lang);
    r_mag_pos.add_suffix(&box_mag_pos);
    exp_mag.add_row(&r_mag_nv);
    exp_mag.add_row(&r_mag_zoom);
    exp_mag.add_row(&r_mag_size);
    exp_mag.add_row(&r_mag_pos);

    // 7. Alignment
    let sw_align = Switch::builder().valign(Align::Center).build();
    let r_align = ActionRow::new();
    tr_row(u, &r_align, "alignment_aid", lang);
    r_align.add_suffix(&sw_align);

    // 8. Hawkeye
    let exp_hawk = ExpanderRow::builder()
        .show_enable_switch(true)
        .build();
    tr_expander(u, &exp_hawk, "hawkeye", lang);
    let (box_hawk_s, btn_hawk_size) = create_linked_buttons(&["Small", "Medium", "Large"][..]);
    let r_hawk_s = ActionRow::new();
    tr_row(u, &r_hawk_s, "size", lang);
    r_hawk_s.add_suffix(&box_hawk_s);
    let (box_hawk_p, btn_hawk_pos) = create_linked_buttons(
        &["Top Right", "Top Left", "Central", "Bottom Right", "Bottom Left"][..],
    );
    let r_hawk_p = ActionRow::new();
    tr_row(u, &r_hawk_p, "position", lang);
    r_hawk_p.add_suffix(&box_hawk_p);
    let (box_hawk_l, btn_hawk_lvl) = create_linked_buttons(&["1", "2", "3", "4", "5"][..]);
    let r_hawk_l = ActionRow::new();
    tr_row(u, &r_hawk_l, "level", lang);
    r_hawk_l.add_suffix(&box_hawk_l);
    exp_hawk.add_row(&r_hawk_s);
    exp_hawk.add_row(&r_hawk_p);
    exp_hawk.add_row(&r_hawk_l);

    g_aid.add(&r_size);
    g_aid.add(&exp_hz);
    g_aid.add(&exp_cross);
    g_aid.add(&exp_stop);
    g_aid.add(&exp_gt);
    g_aid.add(&exp_mag);
    g_aid.add(&r_align);
    g_aid.add(&exp_hawk);

    // --- SEKCJA: PICTURE ENHANCE ---
    let g_enh = PreferencesGroup::new();
    tr_group(u, &g_enh, "pic_enhance", lang);

    let sw_async = Switch::builder().valign(Align::Center).build();
    let r_async = ActionRow::new();
    tr_row(u, &r_async, "adaptive_sync", lang);
    r_async.add_suffix(&sw_async);
    let sw_rush = Switch::builder().valign(Align::Center).build();
    let r_rush = ActionRow::new();
    tr_row(u, &r_rush, "game_rush", lang);
    r_rush.add_suffix(&sw_rush);

    let (box_dim, btn_dim) = create_linked_buttons(&["Off", "Low", "Smooth", "Med", "High"][..]);
    let r_dim = ActionRow::new();
    tr_row(u, &r_dim, "local_dimming", lang);
    r_dim.add_suffix(&box_dim);

    let (box_dyds, btn_dyds) =
        create_linked_buttons(&["Off", "Low", "Med", "High", "ULL-1", "ULL-2", "ULL-3"][..]);
    let r_dyds = ActionRow::new();
    tr_row(u, &r_dyds, "dyds", lang);
    r_dyds.add_suffix(&box_dyds);

    let (box_nv, btn_nv_enh) =
        create_linked_buttons(&["Off", "Lvl 1", "Lvl 2", "Auto-Lvl 1", "Auto-Lvl 2"][..]);
    let r_nv = ActionRow::new();
    tr_row(u, &r_nv, "night_vision", lang);
    r_nv.add_suffix(&box_nv);

    let (box_od, btn_od_enh) =
        create_linked_buttons(&["Off", "Lvl 1", "Lvl 2", "Lvl 3", "Top Speed"][..]);
    let r_od = ActionRow::new();
    tr_row(u, &r_od, "dynamic_od", lang);
    r_od.add_suffix(&box_od);

    let (box_hdr, btn_hdr_enh) = create_linked_buttons(&["Off", "Auto", "Game", "Movie"][..]);
    let r_hdr = ActionRow::new();
    tr_row(u, &r_hdr, "hdr", lang);
    r_hdr.add_suffix(&box_hdr);

    let s_col = create_scale_with_max(10.0);
    let r_col = ActionRow::new();
    tr_row(u, &r_col, "color_enhance", lang);
    r_col.add_suffix(&s_col);
    let s_cr = create_scale_with_max(5.0);
    let r_cr = ActionRow::new();
    tr_row(u, &r_cr, "cr_enhance", lang);
    r_cr.add_suffix(&s_cr);
    let s_sh = create_scale();
    let r_sh = ActionRow::new();
    tr_row(u, &r_sh, "shadow_enhance", lang);
    r_sh.add_suffix(&s_sh);
    let s_sr = create_scale_with_max(5.0);
    let r_sr = ActionRow::new();
    tr_row(u, &r_sr, "super_resolution", lang);
    r_sr.add_suffix(&s_sr);

    let s_halo = create_scale();
    let r_halo = ActionRow::new();
    tr_row(u, &r_halo, "halo_control", lang);
    r_halo.add_suffix(&s_halo);

    g_enh.add(&r_async);
    g_enh.add(&r_rush);
    g_enh.add(&r_dim);
    g_enh.add(&r_dyds);
    g_enh.add(&r_nv);
    g_enh.add(&r_od);
    g_enh.add(&r_hdr);
    g_enh.add(&r_col);
    g_enh.add(&r_cr);
    g_enh.add(&r_sh);
    g_enh.add(&r_sr);
    g_enh.add(&r_halo);

    page.add(&g_aid);
    page.add(&g_enh);

    (
        page,
        r_size,
        btn_size,
        exp_hz,
        btn_hz_pos,
        exp_cross,
        btn_cross_shape,
        btn_cross_color,
        exp_stop,
        btn_stop_time,
        btn_stop_pos,
        exp_gt,
        btn_gt_time,
        btn_gt_pos,
        exp_mag,
        sw_mag_nv,
        btn_mag_zoom,
        btn_mag_size,
        btn_mag_pos,
        sw_align,
        exp_hawk,
        btn_hawk_size,
        btn_hawk_pos,
        btn_hawk_lvl,
        sw_async,
        sw_rush,
        btn_dim,
        btn_dyds,
        btn_nv_enh,
        btn_od_enh,
        btn_hdr_enh,
        s_col,
        s_cr,
        s_sh,
        s_sr,
        s_halo,
    )
}
