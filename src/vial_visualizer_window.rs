//! Standalone entry point for exercising the Vial/Hyprland visualizer while
//! the page is being integrated into the main settings navigation.

use crate::bind::BindCollection;
use crate::config;
use crate::vial_visualizer;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box as GtkBox, Label, Orientation};
use std::cell::RefCell;
use std::rc::Rc;

pub fn build(app: &Application) {
    crate::ui_prefs::apply_dark_mode(crate::ui_prefs::load().dark_mode);

    let collection: Rc<RefCell<Option<BindCollection>>> = Rc::new(RefCell::new(None));
    let status_label = Label::builder()
        .label("Loading Hyprland config and Vial keyboard…")
        .halign(gtk4::Align::Start)
        .xalign(0.0)
        .wrap(true)
        .css_classes(["dim-label", "caption"])
        .build();

    match config::load_binds(None) {
        Ok(mut loaded) => {
            loaded.finalize();
            status_label.set_text(&format!(
                "Loaded {} Hyprland bind(s). Discovering Vial hardware…",
                loaded.binds.len()
            ));
            *collection.borrow_mut() = Some(loaded);
        }
        Err(err) => {
            status_label.set_text(&format!(
                "Hyprland config failed to load ({err}). Keyboard inspection still works."
            ));
        }
    }

    let status: Rc<dyn Fn(String)> = {
        let status_label = status_label.clone();
        Rc::new(move |message| status_label.set_text(&message))
    };
    let visualizer = vial_visualizer::build_page(Rc::clone(&collection), status);

    let body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .build();
    body.append(&visualizer.page);
    body.append(&status_label);

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Hyprbinds · Vial keybind visualizer")
        .default_width(1180)
        .default_height(720)
        .child(&body)
        .build();
    window.present();
}
