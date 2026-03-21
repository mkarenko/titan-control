use crate::app_settings;
use crate::i18n::{AppLang, LangUpdaters, tr};
use adw::prelude::*;
use adw::{ActionRow, ExpanderRow, PreferencesGroup, PreferencesPage};
use gtk4::{
    Adjustment, Box as GtkBox, Button, EventControllerScroll, EventControllerScrollFlags,
    GestureClick, Label, MenuButton, Orientation, Popover, PositionType, PropagationPhase,
    Revealer, Scale, ToggleButton, Widget, glib,
};
use libadwaita as adw;

pub fn create_scale() -> Scale {
    create_scale_with_range(0.0, 100.0)
}

pub fn create_scale_with_max(max: f64) -> Scale {
    create_scale_with_range(0.0, max)
}

pub fn create_scale_with_range(min: f64, max: f64) -> Scale {
    let page = if max <= 10.0 { 1.0 } else { 10.0 };
    let scale = Scale::builder()
        .orientation(Orientation::Horizontal)
        .adjustment(&Adjustment::new(min, min, max, 1.0, page, 0.0))
        .hexpand(true)
        .width_request(320)
        .draw_value(false)
        .value_pos(PositionType::Right)
        .build();
    scale.set_digits(0);

    // Disable scroll wheel
    let scroll_ctrl = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
    scroll_ctrl.connect_scroll(|_, _, _| glib::Propagation::Stop);
    scale.add_controller(scroll_ctrl);

    scale
}

pub fn create_scale_control(scale: &Scale) -> GtkBox {
    let row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .hexpand(true)
        .build();

    let btn_down = Button::builder().icon_name("pan-down-symbolic").build();
    let btn_up = Button::builder().icon_name("pan-up-symbolic").build();
    let value_label = Label::builder()
        .label(&format!("{:>3}", scale.value() as i32))
        .width_chars(3)
        .xalign(1.0)
        .css_classes(["numeric"])
        .valign(gtk4::Align::Center)
        .build();
    let btn_box = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(0)
        .css_classes(["linked", "scale-stepper"])
        .valign(gtk4::Align::Center)
        .build();

    btn_down.set_valign(gtk4::Align::Center);
    btn_up.set_valign(gtk4::Align::Center);

    {
        let scale = scale.clone();
        let value_label = value_label.clone();
        btn_down.connect_clicked(move |_| {
            let adj = scale.adjustment();
            let next = (scale.value() - 1.0).max(adj.lower());
            scale.set_value(next);
            value_label.set_label(&format!("{:>3}", next as i32));
        });
    }

    {
        let scale = scale.clone();
        let value_label = value_label.clone();
        btn_up.connect_clicked(move |_| {
            let adj = scale.adjustment();
            let next = (scale.value() + 1.0).min(adj.upper());
            scale.set_value(next);
            value_label.set_label(&format!("{:>3}", next as i32));
        });
    }

    {
        let value_label = value_label.clone();
        scale.connect_value_changed(move |s| {
            value_label.set_label(&format!("{:>3}", s.value() as i32));
        });
    }

    {
        let btn_down = btn_down.clone();
        let btn_up = btn_up.clone();
        let value_label = value_label.clone();
        scale.connect_sensitive_notify(move |s| {
            let sensitive = s.is_sensitive();
            btn_down.set_sensitive(sensitive);
            btn_up.set_sensitive(sensitive);
            value_label.set_sensitive(sensitive);
        });
    }

    let initial_sensitive = scale.is_sensitive();
    btn_down.set_sensitive(initial_sensitive);
    btn_up.set_sensitive(initial_sensitive);
    value_label.set_sensitive(initial_sensitive);

    row.append(scale);
    row.append(&value_label);
    btn_box.append(&btn_down);
    btn_box.append(&btn_up);
    row.append(&btn_box);
    row
}

pub fn create_info_icon(tooltip: &str) -> Widget {
    let button = MenuButton::builder()
        .icon_name("info-outline-symbolic")
        .tooltip_text(tooltip)
        .valign(gtk4::Align::Center)
        .css_classes(["flat"])
        .build();

    let popover = Popover::new();
    let label = Label::builder()
        .label(tooltip)
        .wrap(true)
        .max_width_chars(36)
        .xalign(0.0)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(10)
        .margin_end(10)
        .build();
    popover.set_child(Some(&label));
    button.set_popover(Some(&popover));

    button.upcast()
}

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

#[allow(dead_code)]
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

pub fn create_collapsible_group(
    u: &LangUpdaters,
    key: &'static str,
    lang: &AppLang,
) -> (PreferencesGroup, PreferencesGroup) {
    let wrapper = PreferencesGroup::new();
    let inner = PreferencesGroup::new();
    let expanded = app_settings::collapsible_section_expanded(key);
    let revealer = Revealer::builder().reveal_child(expanded).build();
    let toggle = ToggleButton::builder()
        .active(expanded)
        .valign(gtk4::Align::Center)
        .css_classes(["flat"])
        .icon_name(if expanded {
            "pan-up-symbolic"
        } else {
            "pan-down-symbolic"
        })
        .tooltip_text(&tr(lang, key))
        .build();

    wrapper.set_title(&tr(lang, key));
    wrapper.set_property("header-suffix", &toggle);
    revealer.set_child(Some(&inner));
    wrapper.add(&revealer);

    {
        let revealer = revealer.clone();
        let toggle = toggle.clone();
        toggle.connect_toggled(move |btn| {
            let expanded = btn.is_active();
            revealer.set_reveal_child(expanded);
            btn.set_icon_name(if expanded {
                "pan-up-symbolic"
            } else {
                "pan-down-symbolic"
            });
            let _ = app_settings::set_collapsible_section_expanded(key, expanded);
        });
    }

    // Make the entire header row clickable (not just the toggle button)
    {
        let toggle_click = toggle.clone();
        let header_gesture = GestureClick::new();
        header_gesture.set_propagation_phase(PropagationPhase::Capture);
        header_gesture.connect_pressed(move |gesture, _, _x, y| {
            // Header height is approximately 52px; only trigger for clicks in that area
            if y <= 56.0 {
                toggle_click.set_active(!toggle_click.is_active());
                gesture.set_state(gtk4::EventSequenceState::Claimed);
            }
        });
        wrapper.add_controller(header_gesture);
    }

    let wrapper_ref = wrapper.clone();
    let toggle_ref = toggle.clone();
    u.borrow_mut().push(Box::new(move |l| {
        wrapper_ref.set_title(&tr(l, key));
        toggle_ref.set_tooltip_text(Some(&tr(l, key)));
    }));

    (wrapper, inner)
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
