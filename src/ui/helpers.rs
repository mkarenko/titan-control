use crate::i18n::{AppLang, LangUpdaters, tr};
use adw::prelude::*;
use adw::{ActionRow, ExpanderRow, PreferencesGroup, PreferencesPage};
use gtk4::{Adjustment, Button, Label, Orientation, Scale};
use libadwaita as adw;

pub fn create_scale() -> Scale {
    create_scale_with_max(100.0)
}

pub fn create_scale_with_max(max: f64) -> Scale {
    let page = if max <= 10.0 { 1.0 } else { 10.0 };
    Scale::builder()
        .orientation(Orientation::Horizontal)
        .adjustment(&Adjustment::new(0.0, 0.0, max, 1.0, page, 0.0))
        .hexpand(false)
        .width_request(200)
        .build()
}

// --- Translation helpers: set initial text + register live update callback ---

pub fn tr_page(u: &LangUpdaters, w: &PreferencesPage, key: &'static str, lang: &AppLang) {
    w.set_title(&tr(lang, key));
    let w = w.clone();
    u.borrow_mut()
        .push(Box::new(move |l| w.set_title(&tr(l, key))));
}

pub fn tr_group(u: &LangUpdaters, w: &PreferencesGroup, key: &'static str, lang: &AppLang) {
    w.set_title(&tr(lang, key));
    let w = w.clone();
    u.borrow_mut()
        .push(Box::new(move |l| w.set_title(&tr(l, key))));
}

pub fn tr_group_desc(u: &LangUpdaters, w: &PreferencesGroup, key: &'static str, lang: &AppLang) {
    w.set_description(Some(&tr(lang, key)));
    let w = w.clone();
    u.borrow_mut()
        .push(Box::new(move |l| w.set_description(Some(&tr(l, key)))));
}

pub fn tr_row(u: &LangUpdaters, w: &ActionRow, key: &'static str, lang: &AppLang) {
    w.set_title(&tr(lang, key));
    let w = w.clone();
    u.borrow_mut()
        .push(Box::new(move |l| w.set_title(&tr(l, key))));
}

pub fn tr_row_sub(u: &LangUpdaters, w: &ActionRow, key: &'static str, lang: &AppLang) {
    w.set_subtitle(&tr(lang, key));
    let w = w.clone();
    u.borrow_mut()
        .push(Box::new(move |l| w.set_subtitle(&tr(l, key))));
}

pub fn tr_expander(u: &LangUpdaters, w: &ExpanderRow, key: &'static str, lang: &AppLang) {
    w.set_title(&tr(lang, key));
    let w = w.clone();
    u.borrow_mut()
        .push(Box::new(move |l| w.set_title(&tr(l, key))));
}

pub fn tr_button(u: &LangUpdaters, w: &Button, key: &'static str, lang: &AppLang) {
    w.set_label(&tr(lang, key));
    let w = w.clone();
    u.borrow_mut()
        .push(Box::new(move |l| w.set_label(&tr(l, key))));
}

pub fn tr_label(u: &LangUpdaters, w: &Label, key: &'static str, lang: &AppLang) {
    w.set_label(&tr(lang, key));
    let w = w.clone();
    u.borrow_mut()
        .push(Box::new(move |l| w.set_label(&tr(l, key))));
}
