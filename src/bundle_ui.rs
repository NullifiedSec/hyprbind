//! Import / Export page — backup and restore the hyprbinds-export JSON bundle.

use crate::bind::BindCollection;
use crate::bundle::{self, ImportOptions};
use crate::dialog;
use gtk4::gio;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, Label, Orientation, PolicyType, ScrolledWindow, Window,
};
use std::cell::RefCell;
use std::rc::Rc;

pub struct BundlePage {
    pub page: GtkBox,
}

pub fn build_bundle_page(
    collection: Rc<RefCell<Option<BindCollection>>>,
    reload: Rc<dyn Fn()>,
    status: Rc<dyn Fn(String)>,
    parent: &impl IsA<Window>,
) -> BundlePage {
    let summary = Label::builder()
        .label(
            "Export a single JSON file with Hyprland settings, Waybar, app prefs, and VIA assets. \
             Import translates that JSON back into each app’s native config.",
        )
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["hyprbinds-health-summary"])
        .build();

    let hypr_cb = CheckButton::builder()
        .label("Hyprland")
        .active(true)
        .tooltip_text("Binds, rules, monitors, config overrides, env, startup, …")
        .build();
    let waybar_cb = CheckButton::builder()
        .label("Waybar")
        .active(true)
        .build();
    let app_cb = CheckButton::builder()
        .label("App prefs")
        .active(true)
        .tooltip_text("Dark mode, developer mode, wallpaper prefs")
        .build();
    let via_cb = CheckButton::builder()
        .label("VIA defs / RGB presets")
        .active(true)
        .build();
    let system_cb = CheckButton::builder()
        .label("Portal conf")
        .active(true)
        .build();

    let sections = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .halign(gtk4::Align::Start)
        .build();
    sections.append(
        &Label::builder()
            .label("Sections")
            .halign(gtk4::Align::Start)
            .css_classes(["dim-label", "caption"])
            .build(),
    );
    sections.append(&hypr_cb);
    sections.append(&waybar_cb);
    sections.append(&app_cb);
    sections.append(&via_cb);
    sections.append(&system_cb);

    let export_btn = Button::builder()
        .label("Export…")
        .css_classes(["suggested-action"])
        .build();
    let import_btn = Button::builder().label("Import…").build();

    let actions = GtkBox::new(Orientation::Horizontal, 8);
    actions.set_halign(gtk4::Align::End);
    actions.append(&export_btn);
    actions.append(&import_btn);

    let detail = Label::builder()
        .label("Choose Export to write hyprbinds-export.json, or Import to restore from a file.")
        .halign(gtk4::Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label"])
        .build();

    let toolbar = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .build();
    toolbar.append(&summary);
    toolbar.append(&sections);
    toolbar.append(&actions);
    toolbar.append(&detail);

    let body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_top(8)
        .build();
    body.append(
        &Label::builder()
            .label(
                "Import replaces managed Hyprland sections and selected prefs after a backup \
                 snapshot. Unmanaged Lua outside hyprbinds markers is left alone.",
            )
            .halign(gtk4::Align::Start)
            .wrap(true)
            .xalign(0.0)
            .css_classes(["dim-label", "caption"])
            .build(),
    );

    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .child(&body)
        .build();

    let page = dialog::page_shell(
        "Import / Export",
        "Backup and restore Hyprbinds settings as one JSON file that this app can translate \
         into Hyprland Lua, Waybar, and related configs.",
        &toolbar,
        &scroll,
    );

    let parent = parent.clone().upcast::<Window>();

    let opts_from_ui = {
        let hypr_cb = hypr_cb.clone();
        let waybar_cb = waybar_cb.clone();
        let app_cb = app_cb.clone();
        let via_cb = via_cb.clone();
        let system_cb = system_cb.clone();
        Rc::new(move || ImportOptions {
            hyprland: hypr_cb.is_active(),
            waybar: waybar_cb.is_active(),
            app: app_cb.is_active(),
            via: via_cb.is_active(),
            system: system_cb.is_active(),
        })
    };

    export_btn.connect_clicked({
        let collection = Rc::clone(&collection);
        let status = Rc::clone(&status);
        let detail = detail.clone();
        let parent = parent.clone();
        move |_| {
            let Some(col) = collection.borrow().clone() else {
                status("Load a config before exporting.".into());
                return;
            };
            let status = Rc::clone(&status);
            let detail = detail.clone();
            let dialog = gtk4::FileDialog::builder()
                .title("Export hyprbinds settings")
                .initial_name("hyprbinds-export.json")
                .build();
            let filter = gtk4::FileFilter::new();
            filter.add_pattern("*.json");
            filter.set_name(Some("JSON (*.json)"));
            let filters = gio::ListStore::new::<gtk4::FileFilter>();
            filters.append(&filter);
            dialog.set_filters(Some(&filters));
            dialog.save(Some(&parent), None::<&gio::Cancellable>, move |result| {
                let Ok(file) = result else {
                    return;
                };
                let Some(mut path) = file.path() else {
                    return;
                };
                if path.extension().is_none() {
                    path.set_extension("json");
                }
                match bundle::export_to_path(&col, &path) {
                    Ok(()) => {
                        let msg = format!("Exported → {}", path.display());
                        detail.set_text(&msg);
                        status(msg);
                    }
                    Err(e) => {
                        let msg = format!("Export failed: {e}");
                        detail.set_text(&msg);
                        status(msg);
                    }
                }
            });
        }
    });

    import_btn.connect_clicked({
        let status = Rc::clone(&status);
        let detail = detail.clone();
        let reload = Rc::clone(&reload);
        let opts_from_ui = Rc::clone(&opts_from_ui);
        let parent = parent.clone();
        move |_| {
            let status = Rc::clone(&status);
            let detail = detail.clone();
            let reload = Rc::clone(&reload);
            let opts_from_ui = Rc::clone(&opts_from_ui);
            let dialog = gtk4::FileDialog::builder()
                .title("Import hyprbinds settings")
                .build();
            let filter = gtk4::FileFilter::new();
            filter.add_pattern("*.json");
            filter.set_name(Some("JSON (*.json)"));
            let filters = gio::ListStore::new::<gtk4::FileFilter>();
            filters.append(&filter);
            dialog.set_filters(Some(&filters));
            dialog.open(Some(&parent), None::<&gio::Cancellable>, move |result| {
                let Ok(file) = result else {
                    return;
                };
                let Some(path) = file.path() else {
                    return;
                };
                let opts = opts_from_ui();
                match bundle::import_from_path(&path, opts) {
                    Ok(report) => {
                        let msg = format!("Import: {}", report.summary());
                        detail.set_text(&msg);
                        status(msg);
                        if report.ok() || !report.messages.is_empty() {
                            reload();
                        }
                    }
                    Err(e) => {
                        let msg = format!("Import failed: {e}");
                        detail.set_text(&msg);
                        status(msg);
                    }
                }
            });
        }
    });

    BundlePage { page }
}
