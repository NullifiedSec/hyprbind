//! Unified Vial control center.
//!
//! This is the native Hyprbind surface for keyboard firmware state. It combines
//! the Hyprland-aware visualizer with the existing native VIA/Vial editor and
//! keeps an upstream Vial GUI launcher as a compatibility escape hatch while
//! remaining protocol surfaces are ported to Rust.

use crate::bind::BindCollection;
use crate::config;
use crate::experimental::via;
use crate::vial_native;
use crate::vial_visualizer;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box as GtkBox, Button, Label, Notebook, Orientation,
};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;

const UPSTREAM_VIAL_COMMIT: &str = "aef8222a2d0429a183b2ed692d5f9efcfd383f08";

pub fn build(app: &Application) {
    crate::ui_prefs::apply_dark_mode(crate::ui_prefs::load().dark_mode);

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Hyprbinds · Vial Control Center")
        .default_width(1280)
        .default_height(820)
        .build();

    let status_label = Label::builder()
        .label("Loading Hyprland config and keyboard firmware state…")
        .halign(Align::Start)
        .xalign(0.0)
        .wrap(true)
        .css_classes(["dim-label", "caption"])
        .build();

    let collection: Rc<RefCell<Option<BindCollection>>> = Rc::new(RefCell::new(None));
    match config::load_binds(None) {
        Ok(mut loaded) => {
            loaded.finalize();
            status_label.set_text(&format!(
                "Loaded {} Hyprland bind(s). Connect a Vial/VIA keyboard to begin.",
                loaded.binds.len()
            ));
            *collection.borrow_mut() = Some(loaded);
        }
        Err(err) => {
            status_label.set_text(&format!(
                "Hyprland config failed to load ({err}). Firmware controls remain available."
            ));
        }
    }

    let status: Rc<dyn Fn(String)> = {
        let label = status_label.clone();
        Rc::new(move |message| label.set_text(&message))
    };

    let notebook = Notebook::new();
    notebook.set_scrollable(true);
    notebook.set_hexpand(true);
    notebook.set_vexpand(true);
    notebook.add_css_class("hyprbinds-hub");

    let visualizer = vial_visualizer::build_page(Rc::clone(&collection), Rc::clone(&status));
    notebook.append_page(
        &visualizer.page,
        Some(&Label::new(Some("Hyprland Sync"))),
    );

    // Reuse the mature native editor instead of duplicating firmware controls.
    // It already contains Keymap, Lighting, Studio, Tap Dance, Combos and
    // Overrides, with Vial definition auto-fetch and stock-VIA JSON fallback.
    let native = via::build_via_page(&window, Rc::clone(&status));
    notebook.append_page(&native.page, Some(&Label::new(Some("Firmware"))));

    notebook.append_page(
        &build_capabilities_page(Rc::clone(&status)),
        Some(&Label::new(Some("Capabilities"))),
    );

    notebook.append_page(
        &build_upstream_page(Rc::clone(&status)),
        Some(&Label::new(Some("Upstream Vial"))),
    );

    let body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    body.append(&notebook);
    body.append(&status_label);

    window.set_child(Some(&body));
    window.present();
}

fn build_capabilities_page(status: Rc<dyn Fn(String)>) -> GtkBox {
    let title = Label::builder()
        .label("Native protocol capability probe")
        .halign(Align::Start)
        .xalign(0.0)
        .css_classes(["title-3"])
        .build();
    let description = Label::builder()
        .label(
            "Read-only probe of the native Rust protocol surface. It reports macro storage and Vial QMK settings exposed by each connected VIA/Vial HID interface. Encoder get/set support is provided by the backend and is used when a board definition exposes encoder positions.",
        )
        .halign(Align::Start)
        .xalign(0.0)
        .wrap(true)
        .build();
    let report = Label::builder()
        .label("No probe run yet.")
        .halign(Align::Start)
        .xalign(0.0)
        .selectable(true)
        .wrap(true)
        .css_classes(["monospace"])
        .build();
    let refresh = Button::builder()
        .label("Probe connected keyboards")
        .css_classes(["suggested-action"])
        .halign(Align::Start)
        .build();

    let run_probe: Rc<dyn Fn()> = {
        let report = report.clone();
        let status = Rc::clone(&status);
        Rc::new(move || match via::discover_devices() {
            Ok(devices) if devices.is_empty() => {
                report.set_text("No VIA/Vial HID interfaces detected.");
                status("Capability probe: no compatible keyboards detected".into());
            }
            Ok(devices) => {
                let mut output = String::new();
                let mut ok = 0usize;
                for device in devices {
                    let name = if device.product.trim().is_empty() {
                        "keyboard"
                    } else {
                        device.product.trim()
                    };
                    output.push_str(&format!(
                        "{name} ({:04x}:{:04x})\n",
                        device.vendor_id, device.product_id
                    ));
                    match vial_native::inspect(device.vendor_id, device.product_id) {
                        Ok(snapshot) => {
                            ok += 1;
                            output.push_str(&format!(
                                "  macros: {} slots, {} bytes\n",
                                snapshot
                                    .macro_count
                                    .map(|v| v.to_string())
                                    .unwrap_or_else(|| "unsupported".into()),
                                snapshot
                                    .macro_buffer_size
                                    .map(|v| v.to_string())
                                    .unwrap_or_else(|| "unsupported".into()),
                            ));
                            output.push_str(&format!(
                                "  qmk settings: {}{}\n",
                                if snapshot.qmk_settings_supported {
                                    "supported"
                                } else {
                                    "unsupported"
                                },
                                if snapshot.qmk_setting_ids.is_empty() {
                                    String::new()
                                } else {
                                    format!(" ({:?})", snapshot.qmk_setting_ids)
                                }
                            ));
                            output.push_str("  encoders: native get/set backend available\n");
                        }
                        Err(err) => output.push_str(&format!("  probe failed: {err}\n")),
                    }
                    output.push('\n');
                }
                report.set_text(output.trim_end());
                status(format!("Capability probe completed for {ok} keyboard(s)"));
            }
            Err(err) => {
                report.set_text(&format!("Discovery failed: {err}"));
                status(format!("Capability probe failed: {err}"));
            }
        })
    };

    refresh.connect_clicked({
        let run_probe = Rc::clone(&run_probe);
        move |_| run_probe()
    });

    let page = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(14)
        .margin_top(18)
        .margin_bottom(18)
        .margin_start(18)
        .margin_end(18)
        .build();
    page.append(&title);
    page.append(&description);
    page.append(&refresh);
    page.append(&report);
    run_probe();
    page
}

fn build_upstream_page(status: Rc<dyn Fn(String)>) -> GtkBox {
    let title = Label::builder()
        .label("Official Vial compatibility bridge")
        .halign(Align::Start)
        .xalign(0.0)
        .css_classes(["title-3"])
        .build();
    let description = Label::builder()
        .label(
            "Hyprbinds is porting Vial features natively in Rust. Until native parity is complete, \
             the pinned official GPL-2.0 Vial GUI can be bootstrapped into third_party/vial-gui and \
             launched here for every board-specific/upstream feature without leaving this checkout.",
        )
        .halign(Align::Start)
        .xalign(0.0)
        .wrap(true)
        .build();
    let pin = Label::builder()
        .label(&format!("Pinned upstream commit: {UPSTREAM_VIAL_COMMIT}"))
        .halign(Align::Start)
        .xalign(0.0)
        .selectable(true)
        .css_classes(["dim-label", "caption"])
        .build();

    let launch = Button::builder()
        .label("Launch official Vial GUI")
        .css_classes(["suggested-action"])
        .halign(Align::Start)
        .build();
    launch.connect_clicked(move |_| match launch_upstream_vial() {
        Ok(()) => status("Launched official Vial GUI companion".into()),
        Err(err) => status(format!("{err}. Run: bash scripts/sync-vial-upstream.sh")),
    });

    let hint = Label::builder()
        .label(
            "Native today: exact geometry, layers, live keymap/remap, lighting, per-key RGB Studio, \
             tap dance, combos, key overrides, macro-buffer transport, encoder transport, QMK settings, \
             and Hyprland chord correlation. The upstream bridge covers Vial surfaces that are still \
             being promoted into first-class native GTK editors.",
        )
        .halign(Align::Start)
        .xalign(0.0)
        .wrap(true)
        .css_classes(["dim-label"])
        .build();

    let page = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(14)
        .margin_top(18)
        .margin_bottom(18)
        .margin_start(18)
        .margin_end(18)
        .build();
    page.append(&title);
    page.append(&description);
    page.append(&pin);
    page.append(&launch);
    page.append(&hint);
    page
}

fn launch_upstream_vial() -> Result<(), String> {
    let root = upstream_root();
    let main = root.join("src/main/python/main.py");
    if !main.is_file() {
        return Err(format!(
            "Upstream Vial checkout not found at {}",
            root.display()
        ));
    }

    let workdir = main.parent().unwrap_or(Path::new("."));
    Command::new("python3")
        .arg("main.py")
        .current_dir(workdir)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("failed to launch upstream Vial: {e}"))
}

fn upstream_root() -> PathBuf {
    if let Ok(root) = std::env::var("HYPRBINDS_VIAL_UPSTREAM") {
        return PathBuf::from(root);
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("third_party/vial-gui")
}
