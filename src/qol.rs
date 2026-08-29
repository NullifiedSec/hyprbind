//! Progressive quality-of-life controls for settings/editors.
//!
//! The existing Entry widgets remain the source of truth so writer/autosave behavior
//! stays unchanged. Known bounded/enumerated fields get a friendlier control layered
//! on top; custom or out-of-range values fall back to the original Entry instead of
//! being silently clamped or rewritten.

use gtk4::prelude::*;
use gtk4::{Adjustment, Box as GtkBox, DropDown, Entry, Label, Orientation, Scale, Widget, Window};
use std::cell::Cell;
use std::rc::Rc;

const CONTROL_NAME: &str = "hyprbinds-qol-control";

#[derive(Clone, Copy)]
enum SliderDisplay {
    Pixels,
    Percent,
    Decimal,
    Integer,
}

pub fn install() {
    enhance_all();

    // Editors are transient top-level GTK windows created after startup. Observe the
    // global toplevel model and enhance them once their widget tree has been mounted.
    let toplevels = Window::toplevels();
    toplevels.connect_items_changed(|_, _, _, _| {
        gtk4::glib::idle_add_local_once(enhance_all);
    });
}

fn enhance_all() {
    for root in Window::list_toplevels() {
        enhance_tree(&root);
    }
}

fn enhance_tree(widget: &Widget) {
    if let Ok(block) = widget.clone().downcast::<GtkBox>() {
        enhance_field_block(&block);
    }

    let mut child = widget.first_child();
    while let Some(current) = child {
        child = current.next_sibling();
        enhance_tree(&current);
    }
}

fn enhance_field_block(block: &GtkBox) {
    let children = direct_children(block);
    if children
        .iter()
        .any(|w| w.widget_name().as_str() == CONTROL_NAME)
    {
        return;
    }

    let Some(label) = children
        .iter()
        .find_map(|w| w.clone().downcast::<Label>().ok())
    else {
        return;
    };
    let Some(entry) = children
        .iter()
        .find_map(|w| w.clone().downcast::<Entry>().ok())
    else {
        return;
    };

    let title = label.text();
    match title.as_str() {
        // Match the curated Look & Feel ranges so the raw Config page and workspace
        // dialogs expose the same sane interaction instead of arbitrary text boxes.
        "gaps_in" => install_slider(block, &entry, 0.0, 40.0, 1.0, 0, 5.0, SliderDisplay::Pixels),
        "gaps_out" => install_slider(block, &entry, 0.0, 60.0, 1.0, 0, 20.0, SliderDisplay::Pixels),
        "border_size" => install_slider(block, &entry, 0.0, 10.0, 1.0, 0, 1.0, SliderDisplay::Pixels),
        "rounding" => install_slider(block, &entry, 0.0, 40.0, 1.0, 0, 10.0, SliderDisplay::Pixels),
        "active_opacity" | "inactive_opacity" => {
            install_slider(block, &entry, 0.3, 1.0, 0.05, 2, 1.0, SliderDisplay::Percent)
        }
        "blur.size" => install_slider(block, &entry, 0.0, 20.0, 1.0, 0, 3.0, SliderDisplay::Pixels),
        "sensitivity" | "Sensitivity" => {
            install_slider(block, &entry, -1.0, 1.0, 0.05, 2, 0.0, SliderDisplay::Decimal)
        }
        "Fingers" => install_slider(block, &entry, 2.0, 9.0, 1.0, 0, 3.0, SliderDisplay::Integer),
        "Scale" => install_slider(block, &entry, 0.25, 3.0, 0.05, 2, 1.0, SliderDisplay::Decimal),

        "layout" => install_choice(block, &entry, &[
            ("Dwindle", "dwindle"),
            ("Master", "master"),
        ]),
        "follow_mouse" => install_choice(block, &entry, &[
            ("0 · Keyboard focus stays put", "0"),
            ("1 · Focus follows cursor", "1"),
            ("2 · Cursor separate; click focuses", "2"),
            ("3 · Cursor separate; click keeps focus", "3"),
        ]),
        "misc.force_default_wallpaper" => install_choice(block, &entry, &[
            ("Random default wallpaper", "-1"),
            ("Default wallpaper 0", "0"),
            ("Default wallpaper 1", "1"),
            ("Default wallpaper 2", "2"),
        ]),
        "misc.vrr" => install_choice(block, &entry, &[
            ("Off", "0"),
            ("On", "1"),
            ("Fullscreen only", "2"),
            ("Fullscreen video / game", "3"),
        ]),
        "master.new_status" => install_choice(block, &entry, &[
            ("Master", "master"),
            ("Slave", "slave"),
            ("Inherit", "inherit"),
        ]),
        "Transform 0-7" => install_choice(block, &entry, &[
            ("0 · Normal", "0"),
            ("1 · Rotate 90°", "1"),
            ("2 · Rotate 180°", "2"),
            ("3 · Rotate 270°", "3"),
            ("4 · Flipped", "4"),
            ("5 · Flipped + 90°", "5"),
            ("6 · Flipped + 180°", "6"),
            ("7 · Flipped + 270°", "7"),
        ]),
        "Direction (horizontal/vertical/…)" => install_choice(block, &entry, &[
            ("Any swipe", "swipe"),
            ("Horizontal", "horizontal"),
            ("Vertical", "vertical"),
            ("Left", "left"),
            ("Right", "right"),
            ("Up", "up"),
            ("Down", "down"),
            ("Pinch", "pinch"),
            ("Pinch in", "pinchin"),
            ("Pinch out", "pinchout"),
        ]),
        _ => {}
    }
}

fn install_slider(
    block: &GtkBox,
    entry: &Entry,
    min: f64,
    max: f64,
    step: f64,
    digits: i32,
    fallback: f64,
    display: SliderDisplay,
) {
    let adjustment = Adjustment::new(fallback, min, max, step, step * 5.0, 0.0);
    let scale = Scale::new(Orientation::Horizontal, Some(&adjustment));
    scale.set_draw_value(false);
    scale.set_digits(digits);
    scale.set_hexpand(true);
    scale.set_tooltip_text(Some(
        "Sane range. Existing values outside it remain available as raw text.",
    ));

    let value = Label::builder()
        .halign(gtk4::Align::End)
        .css_classes(["dim-label", "caption"])
        .build();

    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.set_widget_name(CONTROL_NAME);
    row.append(&scale);
    row.append(&value);
    block.append(&row);

    let syncing = Rc::new(Cell::new(false));
    let sync_from_entry: Rc<dyn Fn()> = {
        let entry = entry.clone();
        let scale = scale.clone();
        let row = row.clone();
        let value = value.clone();
        let syncing = Rc::clone(&syncing);
        Rc::new(move || {
            if syncing.get() {
                return;
            }
            let Ok(parsed) = entry.text().trim().parse::<f64>() else {
                entry.set_visible(true);
                row.set_visible(false);
                return;
            };
            if !parsed.is_finite() || parsed < min || parsed > max {
                entry.set_visible(true);
                row.set_visible(false);
                return;
            }

            syncing.set(true);
            scale.set_value(parsed);
            value.set_text(&display_value(parsed, display));
            entry.set_visible(false);
            row.set_visible(true);
            syncing.set(false);
        })
    };

    entry.connect_changed({
        let sync_from_entry = Rc::clone(&sync_from_entry);
        move |_| sync_from_entry()
    });
    scale.connect_value_changed({
        let entry = entry.clone();
        let value = value.clone();
        let syncing = Rc::clone(&syncing);
        move |scale| {
            if syncing.get() {
                return;
            }
            let current = scale.value();
            syncing.set(true);
            entry.set_text(&storage_value(current, digits));
            value.set_text(&display_value(current, display));
            syncing.set(false);
        }
    });

    sync_from_entry();
}

fn install_choice(block: &GtkBox, entry: &Entry, choices: &[(&str, &str)]) {
    let mut labels: Vec<&str> = choices.iter().map(|(label, _)| *label).collect();
    labels.push("Custom…");
    let dropdown = DropDown::from_strings(&labels);
    dropdown.set_hexpand(true);
    dropdown.set_tooltip_text(Some("Common choices; Custom preserves arbitrary values."));

    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.set_widget_name(CONTROL_NAME);
    row.append(&dropdown);
    block.append(&row);

    let custom_index = choices.len() as u32;
    let syncing = Rc::new(Cell::new(false));
    let choice_values = Rc::new(
        choices
            .iter()
            .map(|(_, value)| (*value).to_string())
            .collect::<Vec<_>>(),
    );

    let sync_from_entry: Rc<dyn Fn()> = {
        let entry = entry.clone();
        let dropdown = dropdown.clone();
        let choice_values = Rc::clone(&choice_values);
        let syncing = Rc::clone(&syncing);
        Rc::new(move || {
            if syncing.get() {
                return;
            }
            let current = entry.text();
            let selected = choice_values
                .iter()
                .position(|value| value == current.as_str())
                .map(|idx| idx as u32)
                .unwrap_or(custom_index);

            syncing.set(true);
            dropdown.set_selected(selected);
            entry.set_visible(selected == custom_index);
            syncing.set(false);
        })
    };

    entry.connect_changed({
        let sync_from_entry = Rc::clone(&sync_from_entry);
        move |_| sync_from_entry()
    });
    dropdown.connect_selected_notify({
        let entry = entry.clone();
        let choice_values = Rc::clone(&choice_values);
        let syncing = Rc::clone(&syncing);
        move |dropdown| {
            if syncing.get() {
                return;
            }
            let selected = dropdown.selected();
            syncing.set(true);
            if let Some(value) = choice_values.get(selected as usize) {
                entry.set_text(value);
                entry.set_visible(false);
            } else {
                entry.set_visible(true);
                entry.grab_focus();
            }
            syncing.set(false);
        }
    });

    sync_from_entry();
}

fn display_value(value: f64, display: SliderDisplay) -> String {
    match display {
        SliderDisplay::Pixels => format!("{} px", value.round() as i64),
        SliderDisplay::Percent => format!("{}%", (value * 100.0).round() as i64),
        SliderDisplay::Decimal => format!("{value:.2}"),
        SliderDisplay::Integer => format!("{}", value.round() as i64),
    }
}

fn storage_value(value: f64, digits: i32) -> String {
    match digits {
        0 => format!("{}", value.round() as i64),
        1 => format!("{value:.1}"),
        _ => format!("{value:.2}"),
    }
}

fn direct_children(block: &GtkBox) -> Vec<Widget> {
    let widget: Widget = block.clone().upcast();
    let mut out = Vec::new();
    let mut child = widget.first_child();
    while let Some(current) = child {
        child = current.next_sibling();
        out.push(current);
    }
    out
}
