//! Main window built from the Blueprint file `window.blp` (compiled by build.rs).

pub mod bindings;
pub mod scroll;
pub mod translate;

use crate::app_settings::{self, StepModifier};
use adw::prelude::*;
use gtk4::{gdk, gdk_pixbuf, gio, glib};
use libadwaita as adw;
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;

const WINDOW_UI: &str = include_str!(concat!(env!("OUT_DIR"), "/window.ui"));
const STYLE: &str = include_str!("style.css");
/// Screen area of `assets/p275mv_plus.png` (x, y, width, height) for the wallpaper.
const MONITOR_SCREEN: (i32, i32, i32, i32) = (3, 3, 504, 284);

pub struct Ui {
    builder: gtk4::Builder,
    pub window: adw::ApplicationWindow,
    pub prefs: adw::PreferencesDialog,
    pub translator: translate::Translator,
    /// Every slider of the window (sliders that share an adjustment show the same setting).
    pub scales: Vec<gtk4::Scale>,
    favorites: Rc<Favorites>,
}

impl Ui {
    pub fn new(app: &adw::Application) -> Self {
        let builder = gtk4::Builder::from_string(WINDOW_UI);
        let window: adw::ApplicationWindow = builder.object("window").expect("window in window.ui");
        window.set_application(Some(app));
        let prefs: adw::PreferencesDialog = builder.object("prefs_dialog").expect("prefs_dialog in window.ui");

        let css = gtk4::CssProvider::new();
        css.load_from_string(STYLE);
        if let Some(display) = gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(&display, &css, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
        }

        let mut scales = Vec::new();
        collect_scales(window.upcast_ref(), &mut scales);
        for scale in &scales {
            bindings::track_scale_pointer(scale);
        }
        scroll::install(&window);
        scroll::install(&prefs);

        let stack: gtk4::Stack = builder.object("stack").expect("stack in window.ui");
        // The dialog builds its inner widgets only when it is first shown, so its page is given directly.
        let prefs_page: adw::PreferencesPage = builder.object("prefs_page").expect("prefs_page in window.ui");
        let translator = translate::Translator::collect(
            &[window.clone().upcast(), prefs.clone().upcast(), prefs_page.upcast()],
            &[stack],
        );

        let favorites = Rc::new(Favorites::default());
        favorites.track(&window);

        let ui = Self { builder, window, prefs, translator, scales, favorites };
        ui.setup_crosshair_icons();
        ui.show_wallpaper(None);
        ui
    }

    /// Monitor picture of the overview with `wallpaper` on its screen; the plain (black) screen when there is no
    /// wallpaper or it cannot be loaded.
    pub fn show_wallpaper(&self, wallpaper: Option<PathBuf>) {
        let Some(frame) = app_settings::resolve_assets_dir().map(|dir| dir.join("p275mv_plus.png")) else { return };
        let image: gtk4::Image = self.get("ov_monitor_picture");
        let Some(wallpaper) = wallpaper else {
            image.set_from_file(Some(&frame));
            return;
        };
        // Decoding a large wallpaper takes a moment: off the main thread.
        glib::spawn_future_local(async move {
            let frame_path = frame.clone();
            let composed = gio::spawn_blocking(move || compose_wallpaper(&frame_path, &wallpaper)).await.ok().flatten();
            match composed {
                Some((bytes, width, height, stride)) => {
                    let texture = gdk::MemoryTexture::new(width, height, gdk::MemoryFormat::R8g8b8a8, &bytes, stride);
                    image.set_paintable(Some(&texture));
                }
                None => image.set_from_file(Some(&frame)),
            }
        });
    }

    /// Object of the Blueprint file by id (a missing id is a programming error).
    pub fn get<T: IsA<glib::Object>>(&self, id: &str) -> T {
        self.builder
            .object(id)
            .unwrap_or_else(|| panic!("object `{id}` missing in window.ui or of another type"))
    }

    /// Adds a star to a row: while the favorites key is held it shows whether the setting is in the tray menu.
    pub fn add_favorite_star(&self, row_id: &str, active: bool) -> gtk4::ToggleButton {
        let star = gtk4::ToggleButton::builder()
            .icon_name("starred-symbolic")
            .valign(gtk4::Align::Center)
            .active(active)
            .css_classes(["flat", "favorite-star"])
            .build();
        let row: glib::Object = self.get(row_id);
        if let Some(row) = row.downcast_ref::<adw::ActionRow>() {
            row.add_prefix(&star);
        } else if let Some(row) = row.downcast_ref::<adw::ExpanderRow>() {
            row.add_prefix(&star);
        }
        let expander = row.is::<adw::ExpanderRow>();
        if expander {
            star.add_css_class("in-expander");
        }
        mark_prefixes(star.upcast_ref(), expander);
        self.favorites.watch(&star);
        star
    }

    /// Gives every row without a star the same 6px prefix the star leaves, so all titles line up.
    /// Call after the stars are added.
    pub fn align_rows(&self) {
        fn visit(widget: &gtk4::Widget) {
            let row_prefix = |add: &dyn Fn(&gtk4::Widget), expander: bool| {
                let spacer = gtk4::Box::builder().css_classes(["row-spacer"]).build();
                if expander {
                    spacer.add_css_class("in-expander");
                }
                add(spacer.upcast_ref());
                mark_prefixes(spacer.upcast_ref(), expander);
            };
            let has_star = |widget: &gtk4::Widget| {
                let mut found = false;
                let mut stack = vec![widget.clone()];
                while let Some(current) = stack.pop() {
                    if current.has_css_class("favorite-star") {
                        found = true;
                        break;
                    }
                    // Only the row's own header, not nested rows of an expander.
                    if !current.is::<adw::PreferencesRow>() || current == *widget {
                        let mut child = current.first_child();
                        while let Some(next) = child {
                            stack.push(next.clone());
                            child = next.next_sibling();
                        }
                    }
                }
                found
            };
            if let Some(row) = widget.downcast_ref::<adw::ExpanderRow>() {
                if !has_star(widget) {
                    row_prefix(&|spacer| row.add_prefix(spacer), true);
                }
            } else if let Some(row) = widget.downcast_ref::<adw::ActionRow>()
                && !has_star(widget)
            {
                row_prefix(&|spacer| row.add_prefix(spacer), false);
            }
            let mut child = widget.first_child();
            while let Some(current) = child {
                visit(&current);
                child = current.next_sibling();
            }
        }
        visit(self.window.upcast_ref());
    }

    /// Moves the explanatory subtitle of each row behind an info button (popover and tooltip).
    /// Returns the buttons, labels and the Polish source texts (for language changes).
    pub fn subtitles_to_info(&self, ids: &[&str]) -> Vec<(gtk4::MenuButton, gtk4::Label, String)> {
        let mut infos = Vec::new();
        for id in ids {
            let row: glib::Object = self.get(id);
            let text = row.property::<Option<String>>("subtitle").unwrap_or_default();
            if text.is_empty() {
                continue;
            }
            row.set_property("subtitle", "");
            let label = gtk4::Label::builder()
                .label(&text)
                .wrap(true)
                .max_width_chars(40)
                .xalign(0.0)
                .margin_top(8)
                .margin_bottom(8)
                .margin_start(10)
                .margin_end(10)
                .build();
            let popover = gtk4::Popover::builder().child(&label).build();
            let button = gtk4::MenuButton::builder()
                .icon_name("help-about-symbolic")
                .tooltip_text(&text)
                .valign(gtk4::Align::Center)
                .css_classes(["flat", "circular", "info-button"])
                .popover(&popover)
                .build();
            place_after_title(row.downcast_ref::<gtk4::Widget>().expect("rows are widgets"), &button);
            infos.push((button, label, text));
        }
        infos
    }

    /// Development aid: shows the stars without holding the key.
    pub fn show_favorite_stars(&self, visible: bool) {
        self.favorites.forced.set(visible);
        self.favorites.set_active(visible);
    }

    pub fn set_favorite_modifier(&self, modifier: StepModifier) {
        self.favorites.binding.set(modifier);
        self.favorites.set_active(false);
    }

    /// Crosshair shapes from assets/crosshairs as icons (white strokes recolored for the light theme).
    fn setup_crosshair_icons(&self) {
        let group: adw::ToggleGroup = self.get("crosshair_shape");
        let Some(dir) = crosshair_icon_dir() else {
            // No assets: number the shapes instead.
            for index in 0..group.n_toggles() {
                if let Some(toggle) = group.toggle(index) {
                    toggle.set_label(Some(&(index + 1).to_string()));
                }
            }
            return;
        };
        if let Some(display) = gdk::Display::default() {
            gtk4::IconTheme::for_display(&display).add_search_path(&dir);
        }
        let apply = move |dark: bool| {
            for index in 0..group.n_toggles() {
                if let Some(toggle) = group.toggle(index) {
                    toggle.set_icon_name(Some(&format!(
                        "titan-crosshair-{}-{}",
                        index + 1,
                        if dark { "dark" } else { "light" }
                    )));
                }
            }
        };
        let style = adw::StyleManager::default();
        apply(style.is_dark());
        style.connect_dark_notify(move |style| apply(style.is_dark()));
    }
}

/// Puts the info button right after the row's title text: the title box stops taking the free space, the button
/// takes it instead (an expander row's own header comes first in its widget tree).
fn place_after_title(row: &gtk4::Widget, button: &gtk4::MenuButton) {
    let mut stack = vec![row.clone()];
    while let Some(widget) = stack.pop() {
        if widget.has_css_class("title")
            && widget.is::<gtk4::Box>()
            && let Some(header) = widget.parent().and_downcast::<gtk4::Box>()
        {
            widget.set_hexpand(false);
            button.set_hexpand(true);
            button.set_halign(gtk4::Align::Start);
            header.insert_child_after(button, Some(&widget));
            return;
        }
        // Depth first, in widget order.
        let mut children = Vec::new();
        let mut child = widget.first_child();
        while let Some(current) = child {
            child = current.next_sibling();
            children.push(current);
        }
        stack.extend(children.into_iter().rev());
    }
}

/// Monitor picture with the wallpaper scaled to cover its screen: RGBA bytes, width, height and row stride.
fn compose_wallpaper(frame: &Path, wallpaper: &Path) -> Option<(glib::Bytes, i32, i32, usize)> {
    let (x, y, width, height) = MONITOR_SCREEN;
    let frame = gdk_pixbuf::Pixbuf::from_file(frame).ok()?;
    if !frame.has_alpha() {
        return None;
    }
    let (_, source_width, source_height) = gdk_pixbuf::Pixbuf::file_info(wallpaper)?;
    let scale = (width as f64 / source_width as f64).max(height as f64 / source_height as f64);
    let scaled_width = ((source_width as f64 * scale).ceil() as i32).max(width);
    let scaled_height = ((source_height as f64 * scale).ceil() as i32).max(height);
    let picture = gdk_pixbuf::Pixbuf::from_file_at_scale(wallpaper, scaled_width, scaled_height, false).ok()?;
    let frame = frame.copy()?;
    picture.copy_area((scaled_width - width) / 2, (scaled_height - height) / 2, width, height, &frame, x, y);
    Some((frame.read_pixel_bytes(), frame.width(), frame.height(), frame.rowstride() as usize))
}

fn collect_scales(widget: &gtk4::Widget, scales: &mut Vec<gtk4::Scale>) {
    if let Some(scale) = widget.downcast_ref::<gtk4::Scale>() {
        scales.push(scale.clone());
    }
    let mut child = widget.first_child();
    while let Some(current) = child {
        collect_scales(&current, scales);
        child = current.next_sibling();
    }
}

/// Writes `titan-crosshair-N-dark.svg` (white) and `-light.svg` (black) into a temporary icon directory.
fn crosshair_icon_dir() -> Option<PathBuf> {
    let source_dir = app_settings::resolve_assets_dir()?.join("crosshairs");
    let dir = std::env::temp_dir().join("titan_control_icons");
    std::fs::create_dir_all(&dir).ok()?;
    for index in 1..=6 {
        let svg = std::fs::read_to_string(source_dir.join(format!("crosshair_{index}.svg"))).ok()?;
        std::fs::write(dir.join(format!("titan-crosshair-{index}-dark.svg")), &svg).ok()?;
        std::fs::write(dir.join(format!("titan-crosshair-{index}-light.svg")), svg.replace("white", "black")).ok()?;
    }
    Some(dir)
}

/// Marks the box that holds a row's prefixes (cancels the gap to the title). An expander row keeps its prefixes
/// in an extra box inside its header row, so the box one level up is the one with the gap.
fn mark_prefixes(prefix: &gtk4::Widget, expander: bool) {
    let mut prefixes = prefix.parent();
    if expander {
        prefixes = prefixes.and_then(|inner| inner.parent());
    }
    if let Some(prefixes) = prefixes {
        prefixes.add_css_class("favorite-prefixes");
    }
}

/// The star stays in the layout (so row titles never move) and is only faded in while the key is held.
fn show_star(star: &gtk4::ToggleButton, visible: bool) {
    star.set_opacity(if visible { 1.0 } else { 0.0 });
    star.set_can_target(visible);
    star.set_can_focus(visible);
}

/// Shows the favorite stars while the configured key is held.
#[derive(Default)]
struct Favorites {
    binding: Cell<StepModifier>,
    active: Cell<bool>,
    /// Shown regardless of the key (UI check only).
    forced: Cell<bool>,
    stars: RefCell<Vec<glib::WeakRef<gtk4::ToggleButton>>>,
}

impl Favorites {
    fn watch(&self, star: &gtk4::ToggleButton) {
        show_star(star, self.active.get());
        self.stars.borrow_mut().push(star.downgrade());
    }

    fn set_active(&self, active: bool) {
        let active = active || self.forced.get();
        if self.active.replace(active) == active {
            return;
        }
        for star in self.stars.borrow().iter().filter_map(|star| star.upgrade()) {
            show_star(&star, active);
        }
    }

    fn held(&self, state: gdk::ModifierType) -> bool {
        state.contains(match self.binding.get() {
            StepModifier::Ctrl => gdk::ModifierType::CONTROL_MASK,
            StepModifier::Shift => gdk::ModifierType::SHIFT_MASK,
            StepModifier::Alt => gdk::ModifierType::ALT_MASK,
            StepModifier::Super => gdk::ModifierType::SUPER_MASK,
        })
    }

    fn is_key(&self, key: gdk::Key) -> bool {
        match self.binding.get() {
            StepModifier::Ctrl => matches!(key, gdk::Key::Control_L | gdk::Key::Control_R),
            StepModifier::Shift => matches!(key, gdk::Key::Shift_L | gdk::Key::Shift_R),
            StepModifier::Alt => matches!(key, gdk::Key::Alt_L | gdk::Key::Alt_R | gdk::Key::Meta_L | gdk::Key::Meta_R),
            StepModifier::Super => matches!(key, gdk::Key::Super_L | gdk::Key::Super_R),
        }
    }

    fn track(self: &Rc<Self>, window: &adw::ApplicationWindow) {
        let keys = gtk4::EventControllerKey::new();
        keys.set_propagation_phase(gtk4::PropagationPhase::Capture);
        let this = self.clone();
        keys.connect_key_pressed(move |_, key, _, state| {
            this.set_active(this.is_key(key) || this.held(state));
            glib::Propagation::Proceed
        });
        let this = self.clone();
        keys.connect_key_released(move |_, key, _, state| {
            this.set_active(!this.is_key(key) && this.held(state));
        });
        let this = self.clone();
        keys.connect_modifiers(move |_, state| {
            this.set_active(this.held(state));
            glib::Propagation::Proceed
        });
        window.add_controller(keys);
        // Releasing the key outside the window must not leave the stars visible.
        let this = self.clone();
        window.connect_is_active_notify(move |window| {
            if !window.is_active() {
                this.set_active(false);
            }
        });
    }
}
