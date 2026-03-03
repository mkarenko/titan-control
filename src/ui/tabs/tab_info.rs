use crate::i18n::{AppLang, LangUpdaters, lang_index};
use crate::ui::helpers::*;
use adw::prelude::*;
use adw::{ActionRow, ComboRow, PreferencesGroup, PreferencesPage};
use gtk4::{Align, Button, StringList, Switch};
use libadwaita as adw;

pub fn build(lang: &AppLang, u: &LangUpdaters) -> (PreferencesPage, ComboRow) {
    let page = PreferencesPage::builder().build();
    tr_page(u, &page, "tab_info", lang);

    let g_app_settings = PreferencesGroup::new();
    tr_group(u, &g_app_settings, "app_name", lang);

    let c_lang = ComboRow::builder()
        .model(&StringList::new(
            &[
                "English",
                "Polski",
                "Deutsch",
                "Español",
                "Français",
                "Українська",
            ][..],
        ))
        .selected(lang_index(lang))
        .build();
    tr_row(u, &c_lang.clone().upcast::<ActionRow>(), "app_lang", lang);

    let c_theme = ComboRow::new();
    tr_row(u, &c_theme.clone().upcast::<ActionRow>(), "app_theme", lang);
    {
        let model = StringList::new(&[
            &crate::i18n::tr(lang, "theme_system"),
            &crate::i18n::tr(lang, "theme_light"),
            &crate::i18n::tr(lang, "theme_dark"),
        ]);
        c_theme.set_model(Some(&model));
        let ct = c_theme.clone();
        u.borrow_mut().push(Box::new(move |l| {
            let sel = ct.selected();
            let m = StringList::new(&[
                &crate::i18n::tr(l, "theme_system"),
                &crate::i18n::tr(l, "theme_light"),
                &crate::i18n::tr(l, "theme_dark"),
            ]);
            ct.set_model(Some(&m));
            ct.set_selected(sel);
        }));
    }

    let sw_auto = Switch::builder().valign(Align::Center).build();
    let r_auto = ActionRow::new();
    tr_row(u, &r_auto, "auto_start", lang);
    r_auto.add_suffix(&sw_auto);
    let sw_min = Switch::builder().valign(Align::Center).build();
    let r_min = ActionRow::new();
    tr_row(u, &r_min, "start_minimized", lang);
    r_min.add_suffix(&sw_min);

    let g_about = PreferencesGroup::new();
    tr_group(u, &g_about, "app_name", lang);
    tr_group_desc(u, &g_about, "app_desc", lang);

    let gh_button = Button::builder().valign(Align::Center).build();
    tr_button(u, &gh_button, "open_btn", lang);
    gh_button.connect_clicked(|_| {
        let _ = std::process::Command::new("xdg-open")
            .arg("https://github.com/mkarenko/titan-control")
            .spawn();
    });
    let row_source = ActionRow::new();
    tr_row(u, &row_source, "source_code", lang);
    tr_row_sub(u, &row_source, "source_code_sub", lang);
    row_source.add_suffix(&gh_button);

    let gh_issue_button = Button::builder().valign(Align::Center).build();
    tr_button(u, &gh_issue_button, "report_btn", lang);
    gh_issue_button.connect_clicked(|_| {
        let _ = std::process::Command::new("xdg-open")
            .arg("https://github.com/mkarenko/titan-control/issues/new")
            .spawn();
    });
    let row_issues = ActionRow::new();
    tr_row(u, &row_issues, "report_bug", lang);
    tr_row_sub(u, &row_issues, "report_bug_sub", lang);
    row_issues.add_suffix(&gh_issue_button);

    let buy_me_a_coffee_button = Button::builder()
        .valign(Align::Center)
        .css_classes(["suggested-action"])
        .build();
    tr_button(u, &buy_me_a_coffee_button, "buy_coffee", lang);
    buy_me_a_coffee_button.connect_clicked(|_| {
        let _ = std::process::Command::new("xdg-open")
            .arg("https://buymeacoffee.com/mkarenko")
            .spawn();
    });
    let row_coffee = ActionRow::new();
    tr_row(u, &row_coffee, "support_author", lang);
    tr_row_sub(u, &row_coffee, "support_author_sub", lang);
    row_coffee.add_suffix(&buy_me_a_coffee_button);

    let row_version = ActionRow::builder().subtitle("0.1.0 (Alpha)").build();
    tr_row(u, &row_version, "version", lang);

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

    (page, c_lang)
}
