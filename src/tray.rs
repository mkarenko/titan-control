// use libadwaita as adw;
// use std::path::Path;
// use tray_icon::menu::{Menu, MenuItem};
// use tray_icon::{Icon, TrayIconBuilder};

// pub fn setup_tray(app: &adw::Application, window: &adw::ApplicationWindow) -> tray_icon::TrayIcon {
//     let menu = Menu::new();
//     let _show_item = MenuItem::with_id("show", "Pokaż", true, None);
//     let _quit_item = MenuItem::with_id("quit", "Wyjdź", true, None);

//     // menu.append(Box::new(show_item));
//     // menu.append(Box::new(quit_item));

//     let img = image::open(Path::new("assets/icon-light.png"))
//         .expect("Nie można załadować ikony")
//         .into_rgba8();

//     let (width, height) = img.dimensions();
//     let icon = Icon::from_rgba(img.into_raw(), width, height).expect("Nie można utworzyć ikony");

//     let _window_clone = window.clone();
//     let _app_clone = app.clone();

//     TrayIconBuilder::new()
//         .with_menu(Box::new(menu))
//         .with_tooltip("Titan Control")
//         .with_icon(icon)
//         .build()
//         .unwrap()
// }

// // gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
// //     if let Ok(event) = TrayIconEvent::receiver().try_recv() {
// //         if let TrayIconEvent::Click {
// //             button,
// //             button_state,
// //             ..
// //         } = event
// //         {
// //             if button == MouseButton::Left && button_state == MouseButtonState::Up {
// //                 if window_clone.is_visible() {
// //                     window_clone.hide();
// //                 } else {
// //                     window_clone.present();
// //                 }
// //             }
// //         }
// //     }

// //     if let Ok(event) = MenuEvent::receiver().try_recv() {
// //         if event.id == quit_id {
// //             app_clone.quit();
// //         } else if event.id == show_id {
// //             window_clone.present();
// //         }
// //     }
// //     gtk4::glib::ControlFlow::Continue
// // });
