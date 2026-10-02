//! Connects monitor settings (VCP codes) with UI controls.
//!
//! Several controls can show the same setting (e.g. brightness in Overview and Picture). A user change is sent to
//! the worker and mirrored to the other controls; values read from the monitor update all controls of a code,
//! except while the user is changing that setting (dragging a slider or a change in the last moments), so a late
//! read never makes a control jump back ("fighting" sliders).

use crate::monitor::WorkerCmd;
use adw::prelude::*;
use gtk4::{gdk, glib};
use libadwaita as adw;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

thread_local! {
    static SUPPRESS: Cell<u32> = const { Cell::new(0) };
    static POINTER_DOWN: Cell<u32> = const { Cell::new(0) };
}

/// Runs `f` with change handlers silenced (programmatic updates are not sent to the monitor).
pub fn suppress<R>(f: impl FnOnce() -> R) -> R {
    SUPPRESS.with(|count| count.set(count.get() + 1));
    let result = f();
    SUPPRESS.with(|count| count.set(count.get() - 1));
    result
}

pub fn suppressed() -> bool {
    SUPPRESS.with(|count| count.get() > 0)
}

/// Whether a slider is held with the pointer (also sliders in tray popups).
pub fn pointer_down() -> bool {
    POINTER_DOWN.with(|count| count.get() > 0)
}

/// Tracks pointer presses on a slider (raw events, before GtkRange's own gestures).
pub fn track_scale_pointer(scale: &gtk4::Scale) {
    let pressed = Rc::new(Cell::new(false));
    let events = gtk4::EventControllerLegacy::new();
    events.set_propagation_phase(gtk4::PropagationPhase::Capture);
    let held = pressed.clone();
    events.connect_event(move |_, event| {
        match event.event_type() {
            gdk::EventType::ButtonPress | gdk::EventType::TouchBegin if !held.replace(true) => {
                POINTER_DOWN.with(|count| count.set(count.get() + 1));
            }
            gdk::EventType::ButtonRelease | gdk::EventType::TouchEnd | gdk::EventType::TouchCancel
                if held.replace(false) =>
            {
                POINTER_DOWN.with(|count| count.set(count.get().saturating_sub(1)));
            }
            _ => {}
        }
        glib::Propagation::Proceed
    });
    scale.add_controller(events);
    scale.connect_unmap(move |_| {
        if pressed.replace(false) {
            POINTER_DOWN.with(|count| count.set(count.get().saturating_sub(1)));
        }
    });
}

/// A control showing one monitor setting.
#[derive(Clone)]
pub enum Control {
    /// Slider value × `factor` = monitor value (e.g. OSD transparency 0-5 = 0-100).
    Adjustment { adj: gtk4::Adjustment, factor: u16 },
    Switch { row: adw::SwitchRow, on: u16, off: u16 },
    Toggle { button: gtk4::ToggleButton, on: u16, off: u16 },
    Toggles { group: adw::ToggleGroup, values: Vec<u16> },
    Combo { row: adw::ComboRow, values: Vec<u16> },
    /// Expander with an enable switch: 1 = on, 0 = off.
    Expander { row: adw::ExpanderRow },
    /// Radio-like toggle buttons (color swatches).
    Buttons { buttons: Vec<gtk4::ToggleButton>, values: Vec<u16> },
}

impl Control {
    fn value(&self) -> Option<u16> {
        match self {
            Control::Adjustment { adj, factor } => Some(adj.value().round() as u16 * factor),
            Control::Switch { row, on, off } => Some(if row.is_active() { *on } else { *off }),
            Control::Toggle { button, on, off } => Some(if button.is_active() { *on } else { *off }),
            Control::Toggles { group, values } => values.get(group.active() as usize).copied(),
            Control::Combo { row, values } => values.get(row.selected() as usize).copied(),
            Control::Expander { row } => Some(row.enables_expansion() as u16),
            Control::Buttons { buttons, values } => buttons
                .iter()
                .position(|button| button.is_active())
                .and_then(|index| values.get(index).copied()),
        }
    }

    /// Shows a monitor value; a value outside the known options leaves the control without a selection.
    fn show(&self, value: u16) {
        match self {
            Control::Adjustment { adj, factor } => adj.set_value((value / (*factor).max(1)) as f64),
            Control::Switch { row, off, .. } => row.set_active(value != *off),
            Control::Toggle { button, off, .. } => button.set_active(value != *off),
            Control::Toggles { group, values } => group.set_active(index_of(values, value)),
            Control::Combo { row, values } => row.set_selected(index_of(values, value)),
            Control::Expander { row } => row.set_enable_expansion(value != 0),
            Control::Buttons { buttons, values } => match values.iter().position(|&known| known == value) {
                Some(index) => buttons[index].set_active(true),
                None => buttons.iter().for_each(|button| button.set_active(false)),
            },
        }
    }

    fn is_slider(&self) -> bool {
        matches!(self, Control::Adjustment { .. })
    }

    /// Rows that show this control (used to grey it out).
    fn rows(&self, scales: &[gtk4::Scale]) -> Vec<gtk4::Widget> {
        let row_of = |widget: &gtk4::Widget| -> gtk4::Widget {
            widget.ancestor(adw::PreferencesRow::static_type()).unwrap_or_else(|| widget.clone())
        };
        match self {
            Control::Adjustment { adj, .. } => scales
                .iter()
                .filter(|scale| scale.adjustment() == *adj)
                .map(|scale| row_of(scale.upcast_ref()))
                .collect(),
            Control::Switch { row, .. } => vec![row.clone().upcast()],
            Control::Toggle { button, .. } => vec![button.clone().upcast()],
            Control::Toggles { group, .. } => vec![row_of(group.upcast_ref())],
            Control::Combo { row, .. } => vec![row.clone().upcast()],
            Control::Expander { row } => vec![row.clone().upcast()],
            Control::Buttons { buttons, .. } => buttons.first().map(|b| vec![row_of(b.upcast_ref())]).unwrap_or_default(),
        }
    }
}

fn index_of(values: &[u16], value: u16) -> u32 {
    values
        .iter()
        .position(|&known| known == value)
        .map(|index| index as u32)
        .unwrap_or(gtk4::INVALID_LIST_POSITION)
}

/// A value read from the monitor does not overwrite a setting the user changed this recently.
const LOCAL_HOLD: Duration = Duration::from_millis(1500);
/// While dragging, a slider sends its value at most this often.
const DRAG_SEND_INTERVAL: Duration = Duration::from_millis(350);
/// Keyboard changes of a slider are sent after this pause.
const KEY_SEND_DELAY: Duration = Duration::from_millis(250);

#[derive(Default)]
struct SliderSend {
    pending: u16,
    changed: Option<Instant>,
    sent_at: Option<Instant>,
    sent: Option<u16>,
    running: bool,
}

type Sender = Rc<dyn Fn(u16)>;

pub struct Bindings {
    tx: mpsc::Sender<WorkerCmd>,
    controls: RefCell<Vec<(u8, Control)>>,
    /// Last known value of every setting (monitor reads and user changes).
    state: RefCell<HashMap<u8, u16>>,
    local: RefCell<HashMap<u8, Instant>>,
    senders: RefCell<HashMap<u8, Sender>>,
    sliders: RefCell<HashMap<u8, SliderSend>>,
    listeners: RefCell<Vec<Rc<dyn Fn()>>>,
    scales: RefCell<Vec<gtk4::Scale>>,
}

impl Bindings {
    pub fn new(tx: mpsc::Sender<WorkerCmd>, scales: Vec<gtk4::Scale>) -> Rc<Self> {
        Rc::new(Self {
            tx,
            controls: RefCell::default(),
            state: RefCell::default(),
            local: RefCell::default(),
            senders: RefCell::default(),
            sliders: RefCell::default(),
            listeners: RefCell::default(),
            scales: RefCell::new(scales),
        })
    }

    pub fn tx(&self) -> &mpsc::Sender<WorkerCmd> {
        &self.tx
    }

    pub fn get(&self, code: u8) -> Option<u16> {
        self.state.borrow().get(&code).copied()
    }

    pub fn state(&self) -> HashMap<u8, u16> {
        self.state.borrow().clone()
    }

    /// Called after every change of the state (user or monitor), e.g. to update dependencies between controls.
    pub fn on_change(&self, listener: impl Fn() + 'static) {
        self.listeners.borrow_mut().push(Rc::new(listener));
    }

    fn notify(&self) {
        let listeners: Vec<_> = self.listeners.borrow().clone();
        for listener in listeners {
            listener();
        }
    }

    /// Replaces the default write (`Set(code, value)`) of a setting.
    pub fn set_sender(&self, code: u8, sender: impl Fn(u16) + 'static) {
        self.senders.borrow_mut().insert(code, Rc::new(sender));
    }

    /// Records a user change made outside the bound controls (e.g. picture mode) and sends it.
    pub fn local_change(&self, code: u8, value: u16) {
        self.local.borrow_mut().insert(code, Instant::now());
        self.state.borrow_mut().insert(code, value);
        self.send(code, value);
        self.notify();
    }

    /// Like [`Bindings::local_change`] for a slider that is not bound to a fixed code (RGB gains): throttled sends.
    pub fn slider_change(self: &Rc<Self>, code: u8, value: u16) {
        self.local.borrow_mut().insert(code, Instant::now());
        self.state.borrow_mut().insert(code, value);
        self.schedule_slider_send(code, value);
        self.notify();
    }

    fn send(&self, code: u8, value: u16) {
        let sender = self.senders.borrow().get(&code).cloned();
        match sender {
            Some(sender) => sender(value),
            None => {
                let _ = self.tx.send(WorkerCmd::Set(code, value));
            }
        }
    }

    pub fn bind(self: &Rc<Self>, code: u8, control: Control) {
        let index = self.controls.borrow().len();
        self.controls.borrow_mut().push((code, control.clone()));
        let this = Rc::downgrade(self);
        let changed = move || {
            if suppressed() {
                return;
            }
            if let Some(this) = this.upgrade() {
                this.user_changed(index);
            }
        };
        match &control {
            Control::Adjustment { adj, .. } => {
                adj.connect_value_changed(move |_| changed());
            }
            Control::Switch { row, .. } => {
                row.connect_active_notify(move |_| changed());
            }
            Control::Toggle { button, .. } => {
                button.connect_toggled(move |_| changed());
            }
            Control::Toggles { group, .. } => {
                group.connect_active_notify(move |_| changed());
            }
            Control::Combo { row, .. } => {
                row.connect_selected_notify(move |_| changed());
            }
            Control::Expander { row } => {
                row.connect_enable_expansion_notify(move |_| changed());
            }
            Control::Buttons { buttons, .. } => {
                let changed = Rc::new(changed);
                for button in buttons {
                    let changed = changed.clone();
                    button.connect_toggled(move |button| {
                        if button.is_active() {
                            changed();
                        }
                    });
                }
            }
        }
    }

    fn user_changed(self: &Rc<Self>, index: usize) {
        let (code, control) = self.controls.borrow()[index].clone();
        let Some(value) = control.value() else { return };
        self.local.borrow_mut().insert(code, Instant::now());
        self.state.borrow_mut().insert(code, value);
        // Mirror to the other controls of this setting.
        suppress(|| {
            for (other_index, (other_code, other)) in self.controls.borrow().iter().enumerate() {
                if *other_code == code && other_index != index {
                    other.show(value);
                }
            }
        });
        if control.is_slider() {
            self.schedule_slider_send(code, value);
        } else {
            self.send(code, value);
        }
        self.notify();
    }

    /// Sliders: while dragging the value goes out every `DRAG_SEND_INTERVAL`, the final value on release;
    /// keyboard changes after `KEY_SEND_DELAY`.
    fn schedule_slider_send(self: &Rc<Self>, code: u8, value: u16) {
        let start = {
            let mut sliders = self.sliders.borrow_mut();
            let slider = sliders.entry(code).or_default();
            slider.pending = value;
            slider.changed = Some(Instant::now());
            !std::mem::replace(&mut slider.running, true)
        };
        if !start {
            return;
        }
        let this = Rc::downgrade(self);
        glib::timeout_add_local(Duration::from_millis(40), move || {
            let Some(this) = this.upgrade() else { return glib::ControlFlow::Break };
            let mut send = None;
            let mut done = false;
            {
                let mut sliders = this.sliders.borrow_mut();
                let slider = sliders.entry(code).or_default();
                let now = Instant::now();
                if pointer_down() {
                    let due = slider.sent_at.is_none_or(|at| now.duration_since(at) >= DRAG_SEND_INTERVAL);
                    if due && slider.sent != Some(slider.pending) {
                        send = Some(slider.pending);
                    }
                } else if slider.changed.is_none_or(|at| now.duration_since(at) >= KEY_SEND_DELAY)
                    || slider.sent_at.is_some()
                {
                    if slider.sent != Some(slider.pending) {
                        send = Some(slider.pending);
                    }
                    done = true;
                }
                if let Some(value) = send {
                    slider.sent = Some(value);
                    slider.sent_at = Some(now);
                }
                if done {
                    slider.running = false;
                    slider.sent_at = None;
                    slider.sent = None;
                }
            }
            if let Some(value) = send {
                this.local.borrow_mut().insert(code, Instant::now());
                this.send(code, value);
            }
            if done { glib::ControlFlow::Break } else { glib::ControlFlow::Continue }
        });
    }

    /// Shows values from the monitor. `read` = regular reads, which are skipped for settings the user is changing;
    /// corrections after a rejected write (`read == false`) are always shown.
    pub fn apply(&self, values: &HashMap<u8, u16>, read: bool) {
        let now = Instant::now();
        suppress(|| {
            for (&code, &value) in values {
                let recent = self
                    .local
                    .borrow()
                    .get(&code)
                    .is_some_and(|at| now.duration_since(*at) < LOCAL_HOLD);
                let sliding = self.sliders.borrow().get(&code).is_some_and(|slider| slider.running);
                if read && (recent || sliding) {
                    continue;
                }
                self.state.borrow_mut().insert(code, value);
                for (bound_code, control) in self.controls.borrow().iter() {
                    if *bound_code == code {
                        control.show(value);
                    }
                }
            }
        });
        self.notify();
    }

    /// Greys out (or enables) every control of a setting.
    pub fn set_sensitive(&self, code: u8, sensitive: bool) {
        let scales = self.scales.borrow();
        for (bound_code, control) in self.controls.borrow().iter() {
            if *bound_code == code {
                for row in control.rows(&scales) {
                    row.set_sensitive(sensitive);
                }
            }
        }
    }

    /// The control of a setting that a tray shortcut acts on (the first bound one).
    pub fn control(&self, code: u8) -> Option<Control> {
        self.controls
            .borrow()
            .iter()
            .find(|(bound_code, _)| *bound_code == code)
            .map(|(_, control)| control.clone())
    }

    /// Whether the first control of a setting can be used.
    pub fn is_sensitive(&self, code: u8) -> bool {
        let scales = self.scales.borrow();
        self.control(code)
            .map(|control| control.rows(&scales).iter().all(|row| row.is_sensitive()))
            .unwrap_or(false)
    }
}
