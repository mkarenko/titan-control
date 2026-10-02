//! Mouse wheel scrolling of the settings pages.
//!
//! Wheel events are taken over by every scrolled page (in the capture phase, so they never change a slider under
//! the cursor). The page glides to a target that each wheel notch moves further; the animation runs on the
//! frame clock, so fast wheel turns accumulate instead of fighting each other.
//! Touchpads (pixel-precise scrolling) keep GTK's native kinetic scrolling.

use adw::prelude::*;
use gtk4::glib;
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;

/// Fraction of the remaining distance covered per frame (≈ 120 ms to settle at 60 Hz).
const EASING: f64 = 0.22;

struct Animation {
    target: f64,
    tick: Option<gtk4::TickCallbackId>,
}

/// Installs the wheel handling on every scrolled window below `root`.
pub fn install(root: &impl IsA<gtk4::Widget>) {
    fn visit(widget: &gtk4::Widget) {
        if let Some(scrolled) = widget.downcast_ref::<gtk4::ScrolledWindow>() {
            attach(scrolled);
        }
        let mut child = widget.first_child();
        while let Some(current) = child {
            visit(&current);
            child = current.next_sibling();
        }
    }
    visit(root.upcast_ref());
}

fn attach(scrolled: &gtk4::ScrolledWindow) {
    let animation = Rc::new(RefCell::new(Animation { target: 0.0, tick: None }));
    let controller = gtk4::EventControllerScroll::new(gtk4::EventControllerScrollFlags::VERTICAL);
    controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    let scrolled_ref = scrolled.downgrade();
    controller.connect_scroll(move |controller, _dx, dy| {
        // Touchpad: native kinetic scrolling.
        if controller.unit() == gtk4::gdk::ScrollUnit::Surface {
            return glib::Propagation::Proceed;
        }
        let Some(scrolled) = scrolled_ref.upgrade() else { return glib::Propagation::Proceed };
        let adj = scrolled.vadjustment();
        let max = (adj.upper() - adj.page_size()).max(adj.lower());
        // GTK's own wheel step: page size ^ 2/3.
        let step = adj.page_size().powf(2.0 / 3.0).max(24.0);

        let mut state = animation.borrow_mut();
        let base = if state.tick.is_some() { state.target } else { adj.value() };
        state.target = (base + dy * step).clamp(adj.lower(), max);
        if state.tick.is_none() {
            let animation = animation.clone();
            let tick = scrolled.add_tick_callback(move |scrolled, _clock| {
                let adj = scrolled.vadjustment();
                let mut state = animation.borrow_mut();
                let distance = state.target - adj.value();
                if distance.abs() < 0.5 {
                    adj.set_value(state.target);
                    state.tick = None;
                    return glib::ControlFlow::Break;
                }
                adj.set_value(adj.value() + distance * EASING);
                glib::ControlFlow::Continue
            });
            state.tick = Some(tick);
        }
        glib::Propagation::Stop
    });
    scrolled.add_controller(controller);
}
