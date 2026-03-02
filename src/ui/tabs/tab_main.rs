use crate::monitor;
use crate::ui::helpers::create_scale;
use adw::prelude::*;
use adw::{ActionRow, ComboRow, PreferencesGroup, PreferencesPage};
use gtk4::{
    Adjustment, Align, Box as GtkBox, Button, Orientation, Scale, StringList, Switch, ToggleButton,
};
use libadwaita as adw;

pub fn build() -> (
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
    let page = PreferencesPage::builder()
        .title("Główne")
        .icon_name("display-symbolic")
        .build();

    let g_aud = PreferencesGroup::builder().title("Dźwięk").build();
    let (r_v, sv) = (ActionRow::new(), create_scale());
    r_v.set_title("Głośność");
    r_v.add_suffix(&sv);
    let sw_m = Switch::builder().valign(Align::Center).build();
    let r_m = ActionRow::builder().title("Wyciszenie").build();
    r_m.add_suffix(&sw_m);
    g_aud.add(&r_v);
    g_aud.add(&r_m);

    let g_dev = PreferencesGroup::builder().title("Urządzenie").build();
    let im = ActionRow::builder().title("Model").build();
    let rh: ActionRow = ActionRow::builder().title("Odświeżanie").build();
    let rc = ActionRow::builder().title("Kontroler").build();
    let rf = ActionRow::builder().title("Firmware").build();
    let ru = ActionRow::builder().title("Czas pracy").build();
    g_dev.add(&im);
    g_dev.add(&rh);
    g_dev.add(&rc);
    g_dev.add(&rf);
    g_dev.add(&ru);

    let g_pwr = PreferencesGroup::builder().title("Zasilanie").build();
    let bo = Button::builder()
        .label("Wyłącz ekran")
        .css_classes(["destructive-action"])
        .build();
    let r_o = ActionRow::builder().title("Przycisk zasilania").build();
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
        .build();
    box_ps.append(&b_ps_off);
    box_ps.append(&b_ps_l1);
    box_ps.append(&b_ps_l2);
    let r_ps = ActionRow::builder().title("Power Saving").build();
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
        .build();
    box_pl.append(&b_pl_off);
    box_pl.append(&b_pl_l1);
    box_pl.append(&b_pl_l2);
    box_pl.append(&b_pl_l3);
    let r_pl = ActionRow::builder().title("Dioda LED").build();
    r_pl.add_suffix(&box_pl);

    g_pwr.add(&r_o);
    g_pwr.add(&r_ps);
    g_pwr.add(&r_pl);

    let g_io = PreferencesGroup::builder()
        .title("Wejścia i Sygnał")
        .build();
    let ci = ComboRow::builder()
        .title("Źródło")
        .model(&StringList::new(&["HDMI-1", "HDMI-2", "USB-C", "DP-2"][..]))
        .build();
    let c_range = ComboRow::builder()
        .title("Output Range")
        .model(&StringList::new(&["Auto", "Limit", "Full"][..]))
        .build();
    let sw_boot = Switch::builder().valign(Align::Center).build();
    let r_boot = ActionRow::builder().title("Quick Boot").build();
    r_boot.add_suffix(&sw_boot);
    g_io.add(&ci);
    g_io.add(&c_range);
    g_io.add(&r_boot);

    let g_osd = PreferencesGroup::builder().title("Ustawienia OSD").build();
    let cl = ComboRow::builder()
        .title("Język")
        .model(&StringList::new(&monitor::OSD_LANGUAGE_NAMES[..]))
        .build();
    let s_time = Scale::builder()
        .orientation(Orientation::Horizontal)
        .adjustment(&Adjustment::new(0.0, 0.0, 60.0, 1.0, 5.0, 0.0))
        .hexpand(true)
        .build();
    let r_time = ActionRow::builder().title("Czas wyświetlania").build();
    r_time.add_suffix(&s_time);
    let s_hpos = create_scale();
    let r_hpos = ActionRow::builder().title("Pozycja H").build();
    r_hpos.add_suffix(&s_hpos);
    let s_vpos = create_scale();
    let r_vpos = ActionRow::builder().title("Pozycja V").build();
    r_vpos.add_suffix(&s_vpos);
    let s_trans = create_scale();
    let r_trans = ActionRow::builder().title("Przezroczystość").build();
    r_trans.add_suffix(&s_trans);
    g_osd.add(&cl);
    g_osd.add(&r_time);
    g_osd.add(&r_hpos);
    g_osd.add(&r_vpos);
    g_osd.add(&r_trans);

    let g_res = PreferencesGroup::builder().title("Resetowanie").build();
    let b1 = Button::builder().label("Resetuj").build();
    let b2 = Button::builder().label("Resetuj").build();
    let b3 = Button::builder().label("Resetuj").build();
    let row1 = ActionRow::new();
    row1.set_title("Ustawienia fabryczne");
    row1.add_suffix(&b1);
    let row2 = ActionRow::new();
    row2.set_title("Jasność i Kontrast");
    row2.add_suffix(&b2);
    let row3 = ActionRow::new();
    row3.set_title("Kolory RGB");
    row3.add_suffix(&b3);
    g_res.add(&row1);
    g_res.add(&row2);
    g_res.add(&row3);

    page.add(&g_aud);
    page.add(&g_dev);
    page.add(&g_pwr);
    page.add(&g_io);
    page.add(&g_osd);
    page.add(&g_res);

    (
        page, sv, sw_m, im, rh, rc, rf, ru, bo, b_ps_off, b_ps_l1, b_ps_l2, b_pl_off, b_pl_l1,
        b_pl_l2, b_pl_l3, ci, c_range, sw_boot, cl, s_time, s_hpos, s_vpos, s_trans, b1, b2, b3,
    )
}
