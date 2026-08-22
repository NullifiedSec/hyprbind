//! Interactive keyboard visualizer backed by live Vial firmware state.

use crate::bind::BindCollection;
use crate::dialog;
use crate::experimental::via;
use crate::keys::normalize_keys_input;
use crate::vial_sync::{self, VialDiscovery};
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, CheckButton, DropDown, Fixed, Frame, Label, ListBox,
    ListBoxRow, Orientation, PolicyType, ScrolledWindow, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

const UNIT_PX: f64 = 44.0;
const GAP_PX: f64 = 3.0;

pub struct VialVisualizerPage {
    pub page: GtkBox,
    pub refresh: Rc<dyn Fn()>,
}

pub fn build_page(
    collection: Rc<RefCell<Option<BindCollection>>>,
    status: Rc<dyn Fn(String)>,
) -> VialVisualizerPage {
    let discovery = Rc::new(RefCell::new(VialDiscovery::default()));
    let selected_board = Rc::new(RefCell::new(0usize));
    let selected_layer = Rc::new(RefCell::new(0u8));

    let refresh_btn = Button::builder()
        .label("Refresh keyboard")
        .css_classes(["suggested-action"])
        .build();
    let board_dd = DropDown::from_strings(&["No Vial keyboard"]);
    board_dd.set_hexpand(true);
    let layer_dd = DropDown::from_strings(&["Layer 0"]);

    let super_mod = CheckButton::with_label("SUPER");
    let ctrl_mod = CheckButton::with_label("CTRL");
    let alt_mod = CheckButton::with_label("ALT");
    let shift_mod = CheckButton::with_label("SHIFT");

    let toolbar = GtkBox::new(Orientation::Horizontal, 8);
    toolbar.append(&refresh_btn);
    toolbar.append(&board_dd);
    toolbar.append(&layer_dd);
    toolbar.append(&super_mod);
    toolbar.append(&ctrl_mod);
    toolbar.append(&alt_mod);
    toolbar.append(&shift_mod);

    let board_fixed = Fixed::new();
    board_fixed.add_css_class("hyprbinds-via-board");
    let board_frame = Frame::builder()
        .child(&board_fixed)
        .css_classes(["hyprbinds-via-frame"])
        .hexpand(true)
        .build();
    let board_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .min_content_height(280)
        .vexpand(true)
        .child(&board_frame)
        .build();

    let selected_label = Label::builder()
        .label("Click a key to inspect its Hyprland chord.")
        .halign(Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["hyprbinds-vial-selected"])
        .build();

    let matches_list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    let matches_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .min_content_height(160)
        .child(&matches_list)
        .build();

    let body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .hexpand(true)
        .vexpand(true)
        .build();
    body.append(&toolbar);
    body.append(&Label::builder()
        .label("Live Vial firmware state · modifier toggles build the Hyprland chord to inspect")
        .halign(Align::Start)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build());
    body.append(&board_scroll);
    body.append(&selected_label);
    body.append(&dialog::section_label("Matching Hyprland binds"));
    body.append(&matches_scroll);

    let render_matches: Rc<dyn Fn(String)> = {
        let collection = Rc::clone(&collection);
        let matches_list = matches_list.clone();
        let selected_label = selected_label.clone();
        Rc::new(move |chord: String| {
            clear_list(&matches_list);
            selected_label.set_text(&format!("Selected chord: {chord}"));
            let normalized = normalize_keys_input(&chord);
            let Some(collection) = collection.borrow().as_ref().cloned() else {
                matches_list.append(&message_row("No Hyprland config loaded."));
                return;
            };
            let mut count = 0usize;
            for bind in &collection.binds {
                if normalize_keys_input(&bind.keys) != normalized {
                    continue;
                }
                count += 1;
                let title = if bind.name.trim().is_empty() {
                    bind.keys.clone()
                } else {
                    bind.name.clone()
                };
                let col = dialog::list_row_column_compact();
                col.append(&dialog::row_title(&title));
                col.append(&dialog::row_sub(&bind.action));
                let meta = format!("{} · {}", bind.submap_label(), bind.source_label());
                col.append(&dialog::row_meta(&meta));
                matches_list.append(&ListBoxRow::builder().child(&col).build());
            }
            if count == 0 {
                matches_list.append(&message_row("No exact Hyprland bind for this chord."));
            }
        })
    };

    let rebuild_board: Rc<dyn Fn()> = {
        let discovery = Rc::clone(&discovery);
        let selected_board = Rc::clone(&selected_board);
        let selected_layer = Rc::clone(&selected_layer);
        let board_fixed = board_fixed.clone();
        let super_mod = super_mod.clone();
        let ctrl_mod = ctrl_mod.clone();
        let alt_mod = alt_mod.clone();
        let shift_mod = shift_mod.clone();
        let render_matches = Rc::clone(&render_matches);
        Rc::new(move || {
            while let Some(child) = board_fixed.first_child() {
                board_fixed.remove(&child);
            }

            let discovery = discovery.borrow();
            let Some(board) = discovery.boards.get(*selected_board.borrow()) else {
                board_fixed.set_size_request(220, 90);
                return;
            };
            let layer = *selected_layer.borrow();
            let width = ((board.definition.layout.width() as f64) * (UNIT_PX + GAP_PX) + GAP_PX)
                .ceil() as i32;
            let height = ((board.definition.layout.height() as f64) * (UNIT_PX + GAP_PX) + GAP_PX)
                .ceil() as i32;
            board_fixed.set_size_request(width.max(220), height.max(90));

            for key in board.physical_keys().iter().cloned() {
                let code = board.keycode_at(layer, key.row, key.col).unwrap_or(0);
                let label = via::keycode_short_label(code, &board.definition.custom_keycodes);
                let host_key = qmk_label_to_hypr(&label);
                let btn = Button::builder()
                    .label(&label)
                    .tooltip_text(&via::keycode_full_label(code, &board.definition.custom_keycodes))
                    .css_classes(["hyprbinds-via-key", "hyprbinds-vial-bind-key"])
                    .sensitive(host_key.is_some())
                    .build();
                let w = (key.w as f64 * UNIT_PX
                    + (key.w as f64 - 1.0).max(0.0) * GAP_PX)
                    .max(28.0);
                let h = (key.h as f64 * UNIT_PX
                    + (key.h as f64 - 1.0).max(0.0) * GAP_PX)
                    .max(28.0);
                btn.set_size_request(w as i32, h as i32);
                let x = GAP_PX + key.x as f64 * (UNIT_PX + GAP_PX);
                let y = GAP_PX + key.y as f64 * (UNIT_PX + GAP_PX);
                board_fixed.put(&btn, x, y);

                if let Some(host_key) = host_key {
                    btn.connect_clicked({
                        let super_mod = super_mod.clone();
                        let ctrl_mod = ctrl_mod.clone();
                        let alt_mod = alt_mod.clone();
                        let shift_mod = shift_mod.clone();
                        let render_matches = Rc::clone(&render_matches);
                        move |_| {
                            let mut parts = Vec::new();
                            if super_mod.is_active() { parts.push("SUPER".to_string()); }
                            if ctrl_mod.is_active() { parts.push("CTRL".to_string()); }
                            if alt_mod.is_active() { parts.push("ALT".to_string()); }
                            if shift_mod.is_active() { parts.push("SHIFT".to_string()); }
                            parts.push(host_key.clone());
                            render_matches(parts.join(" + "));
                        }
                    });
                }
            }
        })
    };

    let refresh: Rc<dyn Fn()> = {
        let discovery = Rc::clone(&discovery);
        let selected_board = Rc::clone(&selected_board);
        let selected_layer = Rc::clone(&selected_layer);
        let board_dd = board_dd.clone();
        let layer_dd = layer_dd.clone();
        let rebuild_board = Rc::clone(&rebuild_board);
        let status = Rc::clone(&status);
        Rc::new(move || match vial_sync::discover() {
            Ok(found) => {
                let labels: Vec<String> = if found.boards.is_empty() {
                    vec!["No Vial keyboard".into()]
                } else {
                    found.boards.iter().map(|b| b.display_name()).collect()
                };
                board_dd.set_model(Some(&StringList::new(
                    &labels.iter().map(String::as_str).collect::<Vec<_>>(),
                )));
                board_dd.set_selected(0);
                *selected_board.borrow_mut() = 0;
                *selected_layer.borrow_mut() = 0;
                let layer_count = found.boards.first().map(|b| b.layer_count()).unwrap_or(1).max(1);
                let layer_labels: Vec<String> = (0..layer_count).map(|i| format!("Layer {i}")).collect();
                layer_dd.set_model(Some(&StringList::new(
                    &layer_labels.iter().map(String::as_str).collect::<Vec<_>>(),
                )));
                layer_dd.set_selected(0);
                let diagnostics = found.diagnostics.len();
                let boards = found.boards.len();
                *discovery.borrow_mut() = found;
                rebuild_board();
                status(format!("Vial visualizer: {boards} board(s), {diagnostics} diagnostic(s)"));
            }
            Err(err) => status(format!("Vial discovery failed: {err}")),
        })
    };

    refresh_btn.connect_clicked({
        let refresh = Rc::clone(&refresh);
        move |_| refresh()
    });

    board_dd.connect_selected_notify({
        let discovery = Rc::clone(&discovery);
        let selected_board = Rc::clone(&selected_board);
        let selected_layer = Rc::clone(&selected_layer);
        let layer_dd = layer_dd.clone();
        let board_dd = board_dd.clone();
        let rebuild_board = Rc::clone(&rebuild_board);
        move |_| {
            let index = board_dd.selected() as usize;
            *selected_board.borrow_mut() = index;
            *selected_layer.borrow_mut() = 0;
            let count = discovery.borrow().boards.get(index).map(|b| b.layer_count()).unwrap_or(1).max(1);
            let labels: Vec<String> = (0..count).map(|i| format!("Layer {i}")).collect();
            layer_dd.set_model(Some(&StringList::new(
                &labels.iter().map(String::as_str).collect::<Vec<_>>(),
            )));
            layer_dd.set_selected(0);
            rebuild_board();
        }
    });

    layer_dd.connect_selected_notify({
        let selected_layer = Rc::clone(&selected_layer);
        let layer_dd = layer_dd.clone();
        let rebuild_board = Rc::clone(&rebuild_board);
        move |_| {
            *selected_layer.borrow_mut() = layer_dd.selected() as u8;
            rebuild_board();
        }
    });

    refresh();

    VialVisualizerPage { page: body, refresh }
}

fn clear_list(list: &ListBox) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
}

fn message_row(text: &str) -> ListBoxRow {
    ListBoxRow::builder()
        .selectable(false)
        .activatable(false)
        .child(
            &Label::builder()
                .label(text)
                .halign(Align::Start)
                .xalign(0.0)
                .margin_top(10)
                .margin_bottom(10)
                .margin_start(12)
                .margin_end(12)
                .css_classes(["dim-label"])
                .build(),
        )
        .build()
}

fn qmk_label_to_hypr(label: &str) -> Option<String> {
    let raw = label.trim();
    if raw.is_empty() || raw == "▽" || raw == "TRNS" || raw == "NO" || raw == "KC_NO" {
        return None;
    }
    let upper = raw.to_ascii_uppercase();
    let mapped = match upper.as_str() {
        "ESC" | "ESCAPE" => "Escape",
        "ENT" | "ENTER" | "RETURN" => "Return",
        "BSPC" | "BACKSPACE" => "BackSpace",
        "SPC" | "SPACE" => "SPACE",
        "DEL" | "DELETE" => "DELETE",
        "INS" | "INSERT" => "Insert",
        "PGUP" | "PAGEUP" => "Page_Up",
        "PGDN" | "PAGEDOWN" => "Page_Down",
        "LEFT" => "left",
        "RIGHT" => "right",
        "UP" => "up",
        "DOWN" => "down",
        "MUTE" => "XF86AudioMute",
        "VOLU" | "VOL+" => "XF86AudioRaiseVolume",
        "VOLD" | "VOL-" => "XF86AudioLowerVolume",
        "MPLY" | "PLAY" => "XF86AudioPlay",
        "MNXT" | "NEXT" => "XF86AudioNext",
        "MPRV" | "PREV" => "XF86AudioPrev",
        _ => {
            if upper.len() == 1 || upper.starts_with('F') && upper[1..].chars().all(|c| c.is_ascii_digit()) {
                return Some(upper);
            }
            if upper.chars().all(|c| c.is_ascii_digit()) {
                return Some(upper);
            }
            return None;
        }
    };
    Some(mapped.to_string())
}
