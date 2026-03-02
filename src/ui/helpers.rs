use gtk4::{Adjustment, Orientation, Scale};

pub fn create_scale() -> Scale {
    Scale::builder()
        .orientation(Orientation::Horizontal)
        .adjustment(&Adjustment::new(0.0, 0.0, 100.0, 1.0, 10.0, 0.0))
        .hexpand(true)
        .sensitive(false)
        .width_request(180)
        .build()
}
