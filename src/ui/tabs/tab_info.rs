use adw::prelude::*;
use adw::{ActionRow, ComboRow, PreferencesGroup, PreferencesPage};
use gtk4::{Align, Button, StringList, Switch};
use libadwaita as adw;

pub fn build() -> PreferencesPage {
    let page = PreferencesPage::builder()
        .title("Info")
        .icon_name("info-symbolic")
        .build();

    let g_app_settings = PreferencesGroup::builder().title("Titan Control").build();

    let c_lang = ComboRow::builder()
        .title("Titan Control")
        .model(&StringList::new(
            &[
                "English",
                "Polski",
                "Deutsch",
                "Español",
                "Français",
                "Ukraińska",
            ][..],
        ))
        .build();
    let c_theme = ComboRow::builder()
        .title("Motyw aplikacji")
        .model(&{
            let s1 = "Motyw systemowy";
            let s2 = "Jasny motyw";
            let s3 = "Ciemny motyw";
            StringList::new(&[s1, s2, s3][..])
        })
        .build();

    let sw_auto = Switch::builder().valign(Align::Center).build();
    let r_auto = ActionRow::builder().title("Uruchom z systemem").build();
    r_auto.add_suffix(&sw_auto);
    let sw_min = Switch::builder().valign(Align::Center).build();
    let r_min = ActionRow::builder()
        .title("Uruchom zminimalizowany")
        .build();
    r_min.add_suffix(&sw_min);

    let g_about = PreferencesGroup::builder()
        .title("Titan Control")
        .description("Aplikacja do sterowania monitorem")
        .build();

    let gh_button = Button::builder()
        .label("Otwórz")
        .valign(gtk4::Align::Center)
        .build();
    gh_button.connect_clicked(|_| {
        let _ = std::process::Command::new("xdg-open")
            .arg("https://github.com/mkarenko/titan-control")
            .spawn();
    });
    let row_source = ActionRow::builder()
        .title("Kod źródłowy")
        .subtitle("GitHub Repository")
        .build();
    row_source.add_suffix(&gh_button);

    let gh_issue_button = Button::builder()
        .label("Zgłoś")
        .valign(gtk4::Align::Center)
        .build();
    gh_issue_button.connect_clicked(|_| {
        let _ = std::process::Command::new("xdg-open")
            .arg("https://github.com/mkarenko/titan-control/issues/new")
            .spawn();
    });
    let row_issues = ActionRow::builder()
        .title("Zgłoś problem")
        .subtitle("Masz błąd lub propozycję?")
        .build();
    row_issues.add_suffix(&gh_issue_button);

    let buy_me_a_coffee_button = Button::builder()
        .label("Kup kawę ☕")
        .valign(gtk4::Align::Center)
        .css_classes(["suggested-action"])
        .build();
    buy_me_a_coffee_button.connect_clicked(|_| {
        let _ = std::process::Command::new("xdg-open")
            .arg("https://buymeacoffee.com/mkarenko")
            .spawn();
    });
    let row_coffee = ActionRow::builder()
        .title("Wesprzyj autora")
        .subtitle("Pomóż w rozwoju projektu")
        .build();
    row_coffee.add_suffix(&buy_me_a_coffee_button);

    let row_version = ActionRow::builder()
        .title("Wersja")
        .subtitle("0.1.0 (Alpha)")
        .build();

    g_app_settings.add(&c_lang);
    g_app_settings.add(&c_theme);
    g_app_settings.add(&r_auto);
    g_app_settings.add(&r_min);

    g_about.add(&row_version);
    g_about.add(&row_source);
    g_about.add(&row_issues);
    g_about.add(&row_coffee);

    page.add(&g_app_settings);
    page.add(&g_about);
    page
}
