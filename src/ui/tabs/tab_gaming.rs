use crate::app_settings;
use crate::i18n::{AppLang, LangUpdaters};
use crate::ui::helpers::*;
use adw::prelude::*;
use adw::{ActionRow, ExpanderRow, PreferencesPage};
use gtk4::{Align, Box as GtkBox, Image, Label, Scale, Switch, ToggleButton};
use libadwaita as adw;
use std::fs;
use std::path::{Path, PathBuf};

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

fn create_crosshair_buttons() -> (GtkBox, Vec<ToggleButton>) {
    let container = GtkBox::builder()
        .css_classes(["linked"])
        .halign(Align::End)
        .valign(Align::Center)
        .build();
    let mut buttons = Vec::new();

    let assets_dir = app_settings::resolve_assets_dir();
    let style_manager = adw::StyleManager::default();
    let first_btn = ToggleButton::builder().active(true).build();
    first_btn.add_css_class("crosshair-shape-btn");
    set_crosshair_button_image(&first_btn, assets_dir.as_ref(), 1, style_manager.is_dark());
    {
        let first_btn = first_btn.clone();
        let assets_dir = assets_dir.clone();
        style_manager.connect_dark_notify(move |manager| {
            set_crosshair_button_image(&first_btn, assets_dir.as_ref(), 1, manager.is_dark());
        });
    }
    container.append(&first_btn);
    buttons.push(first_btn.clone());

    for index in 2..=6 {
        let btn = ToggleButton::builder().group(&first_btn).build();
        btn.add_css_class("crosshair-shape-btn");
        set_crosshair_button_image(&btn, assets_dir.as_ref(), index, style_manager.is_dark());
        {
            let btn = btn.clone();
            let assets_dir = assets_dir.clone();
            style_manager.connect_dark_notify(move |manager| {
                set_crosshair_button_image(&btn, assets_dir.as_ref(), index, manager.is_dark());
            });
        }
        container.append(&btn);
        buttons.push(btn);
    }

    (container, buttons)
}

fn set_row_title_with_info(
    row: &ActionRow,
    u: &LangUpdaters,
    key: &'static str,
    tooltip: &'static str,
    lang: &AppLang,
) {
    row.set_title("");
    let title_box = GtkBox::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(6)
        .valign(Align::Center)
        .build();
    let label = Label::builder()
        .label(crate::i18n::tr(lang, key))
        .xalign(0.0)
        .valign(Align::Center)
        .build();
    let info = create_info_icon(tooltip);
    title_box.append(&label);
    title_box.append(&info);
    row.add_prefix(&title_box);

    let label_ref = label.clone();
    u.borrow_mut().push(Box::new(move |l| {
        label_ref.set_label(&crate::i18n::tr(l, key))
    }));
}
fn set_crosshair_button_image(
    button: &ToggleButton,
    assets_dir: Option<&PathBuf>,
    index: usize,
    dark: bool,
) {
    if let Some(path) = assets_dir.and_then(|dir| themed_crosshair_svg(dir, index, dark)) {
        let image = Image::from_file(path);
        image.set_pixel_size(28);
        button.set_child(Some(&image));
        return;
    }

    button.set_label(&index.to_string());
}

fn themed_crosshair_svg(assets_dir: &Path, index: usize, dark: bool) -> Option<PathBuf> {
    let source = assets_dir
        .join("crosshairs")
        .join(format!("crosshair_{index}.svg"));
    let svg = fs::read_to_string(source).ok()?;
    let stroke = if dark { "white" } else { "black" };
    let themed_svg = svg.replace("white", stroke);

    let dir = std::env::temp_dir().join("titan_control_crosshairs");
    fs::create_dir_all(&dir).ok()?;
    let output = dir.join(format!(
        "crosshair_{index}_{}.svg",
        if dark { "dark" } else { "light" }
    ));
    fs::write(&output, themed_svg).ok()?;
    Some(output)
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
    Switch,            // Alignment
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
    let page = PreferencesPage::builder().build();
    tr_page(u, &page, "tab_gaming", lang);

    // --- SEKCJA: GAME AID ---
    let (g_aid_wrap, g_aid) = create_collapsible_group(u, "game_aid", lang);

    // 1. Full Game
    let (box_size, btn_size) = create_linked_buttons(&["Wide", "25\"", "sPX"][..]);
    let r_size = ActionRow::new();
    tr_row(u, &r_size, "screen_size", lang);
    r_size.add_suffix(&box_size);

    // 2. Refresh Rate
    let exp_hz = ExpanderRow::builder().show_enable_switch(true).build();
    tr_expander(u, &exp_hz, "fps_counter", lang);
    let (box_hz_pos, btn_hz_pos) =
        create_linked_buttons(&["Top Right", "Top Left", "Bottom Right", "Bottom Left"][..]);
    let r_hz_pos = ActionRow::new();
    tr_row(u, &r_hz_pos, "position", lang);
    r_hz_pos.add_suffix(&box_hz_pos);
    exp_hz.add_row(&r_hz_pos);

    // 3. Crosshair
    let exp_cross = ExpanderRow::builder().show_enable_switch(true).build();
    tr_expander(u, &exp_cross, "crosshair", lang);
    let (box_cr_shape, btn_cross_shape) = create_crosshair_buttons();
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
            first.set_width_request(42);
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
                btn.set_width_request(42);
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
    let exp_stop = ExpanderRow::builder().show_enable_switch(true).build();
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
    let exp_gt = ExpanderRow::builder().show_enable_switch(true).build();
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
    let exp_mag = ExpanderRow::builder().show_enable_switch(true).build();
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
        &[
            "Top Right",
            "Top Left",
            "Central",
            "Bottom Right",
            "Bottom Left",
        ][..],
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
    set_row_title_with_info(
        &r_align,
        u,
        "alignment_aid",
        "Nie polecam nigdy tego włączać, nie mam pojęcia czy dobrze to działa, czy nie, ale tragicznie wygląda",
        lang,
    );
    r_align.add_suffix(&sw_align);

    // 8. Hawkeye
    let exp_hawk = ExpanderRow::builder().show_enable_switch(true).build();
    tr_expander(u, &exp_hawk, "hawkeye", lang);
    let (box_hawk_s, btn_hawk_size) = create_linked_buttons(&["Small", "Medium", "Large"][..]);
    let r_hawk_s = ActionRow::new();
    tr_row(u, &r_hawk_s, "size", lang);
    r_hawk_s.add_suffix(&box_hawk_s);
    let (box_hawk_p, btn_hawk_pos) = create_linked_buttons(
        &[
            "Top Right",
            "Top Left",
            "Central",
            "Bottom Right",
            "Bottom Left",
        ][..],
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
    g_aid.add(&exp_hawk);
    g_aid.add(&r_align);

    // --- SEKCJA: PICTURE ENHANCE ---
    let (g_enh_wrap, g_enh) = create_collapsible_group(u, "pic_enhance", lang);

    let sw_async = Switch::builder().valign(Align::Center).build();
    let r_async = ActionRow::new();
    tr_row(u, &r_async, "adaptive_sync", lang);
    r_async.add_suffix(&sw_async);
    let sw_rush = Switch::builder().valign(Align::Center).build();
    let r_rush = ActionRow::new();
    set_row_title_with_info(
        &r_rush,
        u,
        "game_rush",
        "Nie mam pojęcia co to robi, w ustawieniach monitora jest to zawsze włączone oraz wyszarzone",
        lang,
    );
    r_rush.add_suffix(&sw_rush);

    let (box_dim, btn_dim) =
        create_linked_buttons(&["Disabled", "Low", "Smooth", "Medium", "High"][..]);
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
    r_col.add_suffix(&create_scale_control(&s_col));
    let s_cr = create_scale_with_max(5.0);
    let r_cr = ActionRow::new();
    tr_row(u, &r_cr, "cr_enhance", lang);
    r_cr.add_suffix(&create_scale_control(&s_cr));
    let s_sh = create_scale();
    let r_sh = ActionRow::new();
    tr_row(u, &r_sh, "shadow_enhance", lang);
    r_sh.add_suffix(&create_scale_control(&s_sh));
    let s_sr = create_scale_with_max(5.0);
    let r_sr = ActionRow::new();
    tr_row(u, &r_sr, "super_resolution", lang);
    r_sr.add_suffix(&create_scale_control(&s_sr));

    let s_halo = create_scale();
    let r_halo = ActionRow::new();
    tr_row(u, &r_halo, "halo_control", lang);
    r_halo.add_suffix(&create_scale_control(&s_halo));

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

    page.add(&g_aid_wrap);
    page.add(&g_enh_wrap);

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
