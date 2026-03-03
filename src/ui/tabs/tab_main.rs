use crate::i18n::{AppLang, LangUpdaters};
use crate::monitor;
use crate::ui::helpers::*;
use adw::prelude::*;
use adw::{ActionRow, ComboRow, PreferencesGroup, PreferencesPage};
use gtk4::{Align, Box as GtkBox, Button, Scale, StringList, Switch, ToggleButton};
use libadwaita as adw;

pub fn build(
    lang: &AppLang,
    u: &LangUpdaters,
) -> (
    PreferencesPage,
    Scale,
    Switch,
    ActionRow,
    ActionRow,
    ActionRow,
    ActionRow,
    ActionRow,
    Button,
    ToggleButton,
    ToggleButton,
    ToggleButton,
    ToggleButton,
    ToggleButton,
    ToggleButton,
    ToggleButton,
    ComboRow,
    ComboRow,
    Switch,
    ComboRow,
    Scale,
    Scale,
    Scale,
    Scale,
    Button,
    Button,
    Button,
) {
    let page = PreferencesPage::builder().build();
    tr_page(u, &page, "tab_main", lang);

    let g_dev = PreferencesGroup::new();
    tr_group(u, &g_dev, "device_group", lang);
    let im = ActionRow::new();
    tr_row(u, &im, "model", lang);
    let rh = ActionRow::new();
    tr_row(u, &rh, "refresh_rate", lang);
    let rc = ActionRow::new();
    tr_row(u, &rc, "controller", lang);
    let rf = ActionRow::new();
    tr_row(u, &rf, "firmware", lang);
    let ru = ActionRow::new();
    tr_row(u, &ru, "usage_time", lang);
    g_dev.add(&im);
    g_dev.add(&rh);
    g_dev.add(&rc);
    g_dev.add(&rf);
    g_dev.add(&ru);

    let g_aud = PreferencesGroup::new();
    tr_group(u, &g_aud, "audio_group", lang);
    let r_v = ActionRow::new();
    tr_row(u, &r_v, "volume", lang);
    let sv = create_scale();
    r_v.add_suffix(&sv);
    let sw_m = Switch::builder().valign(Align::Center).build();
    let r_m = ActionRow::new();
    tr_row(u, &r_m, "mute", lang);
    r_m.add_suffix(&sw_m);
    g_aud.add(&r_v);
    g_aud.add(&r_m);

    let g_io = PreferencesGroup::new();
    tr_group(u, &g_io, "io_group", lang);
    let ci = ComboRow::builder()
        .model(&StringList::new(
            &["HDMI-1", "HDMI-2", "USB-C", "DisplayPort"][..],
        ))
        .build();
    tr_row(u, &ci.clone().upcast::<ActionRow>(), "input_source", lang);
    let c_range = ComboRow::builder()
        .model(&StringList::new(&["Auto", "Limit", "Full"][..]))
        .build();
    tr_row(
        u,
        &c_range.clone().upcast::<ActionRow>(),
        "output_range",
        lang,
    );
    let sw_boot = Switch::builder().valign(Align::Center).build();
    let r_boot = ActionRow::new();
    tr_row(u, &r_boot, "quick_boot", lang);
    r_boot.add_suffix(&sw_boot);
    g_io.add(&ci);
    g_io.add(&c_range);
    g_io.add(&r_boot);

    let g_osd = PreferencesGroup::new();
    tr_group(u, &g_osd, "osd_group", lang);
    let cl = ComboRow::builder()
        .model(&StringList::new(&monitor::OSD_LANGUAGE_NAMES[..]))
        .build();
    tr_row(u, &cl.clone().upcast::<ActionRow>(), "osd_lang", lang);
    let s_time = create_scale_with_max(60.0);
    let r_time = ActionRow::new();
    tr_row(u, &r_time, "osd_time", lang);
    r_time.add_suffix(&s_time);
    let s_hpos = create_scale();
    let r_hpos = ActionRow::new();
    tr_row(u, &r_hpos, "osd_h_pos", lang);
    r_hpos.add_suffix(&s_hpos);
    let s_vpos = create_scale();
    let r_vpos = ActionRow::new();
    tr_row(u, &r_vpos, "osd_v_pos", lang);
    r_vpos.add_suffix(&s_vpos);
    let s_trans = create_scale();
    let r_trans = ActionRow::new();
    tr_row(u, &r_trans, "osd_trans", lang);
    r_trans.add_suffix(&s_trans);
    g_osd.add(&cl);
    g_osd.add(&r_time);
    g_osd.add(&r_hpos);
    g_osd.add(&r_vpos);
    g_osd.add(&r_trans);

    let g_pwr = PreferencesGroup::new();
    tr_group(u, &g_pwr, "power_group", lang);
    let bo = Button::new();
    bo.add_css_class("destructive-action");
    tr_button(u, &bo, "power_off", lang);
    let r_o = ActionRow::new();
    tr_row(u, &r_o, "power_button", lang);
    r_o.add_suffix(&bo);

    let b_ps_off = ToggleButton::builder().label("Off").active(true).build();
    let b_ps_l1 = ToggleButton::builder()
        .label("Level 1")
        .group(&b_ps_off)
        .build();
    let b_ps_l2 = ToggleButton::builder()
        .label("Level 2")
        .group(&b_ps_off)
        .build();
    let box_ps = GtkBox::builder()
        .css_classes(["linked"])
        .halign(Align::End)
        .valign(Align::Center)
        .build();
    box_ps.append(&b_ps_off);
    box_ps.append(&b_ps_l1);
    box_ps.append(&b_ps_l2);
    let r_ps = ActionRow::new();
    tr_row(u, &r_ps, "power_saving", lang);
    r_ps.add_suffix(&box_ps);

    let b_pl_off = ToggleButton::builder().label("Off").active(true).build();
    let b_pl_l1 = ToggleButton::builder()
        .label("Level 1")
        .group(&b_pl_off)
        .build();
    let b_pl_l2 = ToggleButton::builder()
        .label("Level 2")
        .group(&b_pl_off)
        .build();
    let b_pl_l3 = ToggleButton::builder()
        .label("Level 3")
        .group(&b_pl_off)
        .build();
    let box_pl = GtkBox::builder()
        .css_classes(["linked"])
        .halign(Align::End)
        .valign(Align::Center)
        .build();
    box_pl.append(&b_pl_off);
    box_pl.append(&b_pl_l1);
    box_pl.append(&b_pl_l2);
    box_pl.append(&b_pl_l3);
    let r_pl = ActionRow::new();
    tr_row(u, &r_pl, "power_led", lang);
    r_pl.add_suffix(&box_pl);

    g_pwr.add(&r_o);
    g_pwr.add(&r_ps);
    g_pwr.add(&r_pl);

    let g_res = PreferencesGroup::new();
    tr_group(u, &g_res, "reset_group", lang);
    let b1 = Button::new();
    tr_button(u, &b1, "reset_btn", lang);
    let b2 = Button::new();
    tr_button(u, &b2, "reset_btn", lang);
    let b3 = Button::new();
    tr_button(u, &b3, "reset_btn", lang);
    let row1 = ActionRow::new();
    tr_row(u, &row1, "reset_factory", lang);
    row1.add_suffix(&b1);
    let row2 = ActionRow::new();
    tr_row(u, &row2, "reset_br_con", lang);
    row2.add_suffix(&b2);
    let row3 = ActionRow::new();
    tr_row(u, &row3, "reset_colors", lang);
    row3.add_suffix(&b3);
    g_res.add(&row1);
    g_res.add(&row2);
    g_res.add(&row3);

    page.add(&g_dev);
    page.add(&g_aud);
    page.add(&g_io);
    page.add(&g_osd);
    page.add(&g_pwr);
    page.add(&g_res);

    (
        page, sv, sw_m, im, rh, rc, rf, ru, bo, b_ps_off, b_ps_l1, b_ps_l2, b_pl_off, b_pl_l1,
        b_pl_l2, b_pl_l3, ci, c_range, sw_boot, cl, s_time, s_hpos, s_vpos, s_trans, b1, b2, b3,
    )
}
