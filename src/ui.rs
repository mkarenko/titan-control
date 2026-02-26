use adw::prelude::*;
use adw::{ActionRow, ComboRow, HeaderBar, PreferencesGroup, PreferencesPage, WindowTitle};
use gtk4::{Adjustment, Box as GtkBox, Orientation, Scale, StringList, Switch};
use libadwaita as adw;

pub struct MainWidgets {
    pub window: adw::ApplicationWindow,
    pub status_label: gtk4::Label,
    pub info_model: ActionRow,
    pub info_lang: ActionRow,
    pub sb: Scale,
    pub sc: Scale,
    pub ss: Scale,
    pub sv: Scale,
    pub slbl: Scale,
    pub sw_mute: Switch,
    pub sw_dcr: Switch,
    pub sw_sensor: Switch,
    pub combo_gamma: ComboRow,
    pub combo_aspect: ComboRow,
    pub combo_temp: ComboRow,
}

fn create_scale() -> Scale {
    Scale::builder()
        .orientation(Orientation::Horizontal)
        .adjustment(&Adjustment::new(0.0, 0.0, 100.0, 1.0, 10.0, 0.0))
        .hexpand(true)
        .sensitive(false)
        .width_request(180)
        .build()
}

pub fn build_ui(app: &adw::Application) -> MainWidgets {
    let header = HeaderBar::builder()
        .decoration_layout(":minimize,maximize,close")
        .build();
    header.set_title_widget(Some(&WindowTitle::new("Titan Control", "")));

    let status_label = gtk4::Label::new(Some("Szukanie monitora..."));
    let page = PreferencesPage::new();

    // Sekcja 1: Informacje
    let g_info = PreferencesGroup::builder().title("Informacje").build();
    let info_model = ActionRow::builder().title("Model").build();
    let info_lang = ActionRow::builder().title("Język OSD").build();
    g_info.add(&info_model);
    g_info.add(&info_lang);

    // Sekcja 2: Obraz
    let g_img = PreferencesGroup::builder().title("Obraz").build();
    let (row_b, sb) = (ActionRow::new(), create_scale());
    row_b.set_title("Jasność");
    row_b.add_suffix(&sb);
    let (row_c, sc) = (ActionRow::new(), create_scale());
    row_c.set_title("Kontrast");
    row_c.add_suffix(&sc);
    let (row_s, ss) = (ActionRow::new(), create_scale());
    row_s.set_title("Ostrość");
    row_s.add_suffix(&ss);
    let (row_lbl, slbl) = (ActionRow::new(), create_scale());
    row_lbl.set_title("Low Blue Light");
    row_lbl.add_suffix(&slbl);

    let sw_dcr = Switch::builder()
        .valign(gtk4::Align::Center)
        .sensitive(false)
        .build();
    let row_dcr = ActionRow::new();
    row_dcr.set_title("DCR");
    row_dcr.add_suffix(&sw_dcr);

    let sw_sensor = Switch::builder()
        .valign(gtk4::Align::Center)
        .sensitive(false)
        .build();
    let row_sensor = ActionRow::new();
    row_sensor.set_title("Czujnik Światła");
    row_sensor.add_suffix(&sw_sensor);

    g_img.add(&row_b);
    g_img.add(&row_c);
    g_img.add(&row_s);
    g_img.add(&row_lbl);
    g_img.add(&row_dcr);
    g_img.add(&row_sensor);

    // Sekcja 3: Zaawansowane
    let g_adv = PreferencesGroup::builder().title("Zaawansowane").build();
    let combo_gamma = ComboRow::builder()
        .title("Gamma")
        .sensitive(false)
        .model(&StringList::new(
            &["Standard", "1.8", "2.0", "2.2", "2.4", "2.6"][..],
        ))
        .build();
    let combo_aspect = ComboRow::builder()
        .title("Proporcje")
        .sensitive(false)
        .model(&StringList::new(
            &["Full", "4:3", "16:9", "16:10", "21:9", "Original"][..],
        ))
        .build();
    let combo_temp = ComboRow::builder()
        .title("Temperatura kolorów")
        .sensitive(false)
        .model(&StringList::new(
            &[
                "Native", "5000K", "6500K", "7500K", "8200K", "9300K", "User",
            ][..],
        ))
        .build();
    g_adv.add(&combo_gamma);
    g_adv.add(&combo_aspect);
    g_adv.add(&combo_temp);

    // Sekcja 4: Audio
    let g_audio = PreferencesGroup::builder().title("Audio").build();
    let (row_v, sv) = (ActionRow::new(), create_scale());
    row_v.set_title("Głośność");
    row_v.add_suffix(&sv);
    let sw_mute = Switch::builder()
        .valign(gtk4::Align::Center)
        .sensitive(false)
        .build();
    let row_mute = ActionRow::new();
    row_mute.set_title("Wyciszenie");
    row_mute.add_suffix(&sw_mute);
    g_audio.add(&row_v);
    g_audio.add(&row_mute);

    page.add(&g_info);
    page.add(&g_img);
    page.add(&g_adv);
    page.add(&g_audio);

    let main_layout = GtkBox::new(Orientation::Vertical, 0);
    main_layout.append(&header);
    main_layout.append(&status_label);
    main_layout.append(&page);

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .default_width(550)
        .default_height(650)
        .content(&main_layout)
        .build();

    MainWidgets {
        window,
        status_label,
        info_model,
        info_lang,
        sb,
        sc,
        ss,
        sv,
        slbl,
        sw_mute,
        sw_dcr,
        sw_sensor,
        combo_gamma,
        combo_aspect,
        combo_temp,
    }
}
