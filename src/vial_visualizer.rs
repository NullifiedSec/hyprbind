//! Interactive keyboard visualizer backed by live Vial firmware state.
//!
//! The visualizer deliberately keeps four layers of meaning separate:
//! physical switch -> firmware action -> host/XKB identity -> Hyprland bind.
//! Firmware actions such as MO/LT/MT/Tap Dance/macros are never presented as
//! simple Linux keys unless an exact host key can actually be determined.

use crate::bind::{BindCollection, Keybind};
use crate::config;
use crate::dialog;
use crate::experimental::via;
use crate::keys::normalize_keys_input;
use crate::variables::expand_template;
use crate::vial_key_identity::{self, HardwareKeyIdentity};
use crate::vial_sync::{self, VialBoardSnapshot, VialDiscovery};
use crate::writer;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, CheckButton, DropDown, Entry, Fixed, Frame, Label, ListBox,
    ListBoxRow, Orientation, PolicyType, ScrolledWindow, StringList,
};
use std::cell::{Cell, RefCell};
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
    let selected_board = Rc::new(Cell::new(0usize));
    let selected_layer = Rc::new(Cell::new(0u8));
    let selected_submap = Rc::new(RefCell::new(String::new()));
    let selected_bind_id = Rc::new(Cell::new(None::<usize>));

    let refresh_btn = Button::builder()
        .label("Refresh keyboard")
        .css_classes(["suggested-action"])
        .build();
    let board_dd = DropDown::from_strings(&["No Vial keyboard"]);
    board_dd.set_hexpand(true);
    let layer_dd = DropDown::from_strings(&["Layer 0"]);
    let submap_dd = DropDown::from_strings(&["All submaps", "Global"]);
    submap_dd.set_tooltip_text(Some("Hyprland submap used for overlay matching"));

    let super_mod = CheckButton::with_label("SUPER");
    let ctrl_mod = CheckButton::with_label("CTRL");
    let alt_mod = CheckButton::with_label("ALT");
    let shift_mod = CheckButton::with_label("SHIFT");

    let toolbar = GtkBox::new(Orientation::Horizontal, 8);
    toolbar.append(&refresh_btn);
    toolbar.append(&board_dd);
    toolbar.append(&layer_dd);
    toolbar.append(&submap_dd);
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
        .min_content_height(300)
        .vexpand(true)
        .child(&board_frame)
        .build();

    let selected_label = Label::builder()
        .label("Click a physical key to inspect firmware and Hyprland behavior.")
        .halign(Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["hyprbinds-vial-selected"])
        .build();

    let matches_list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();
    let matches_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .min_content_height(150)
        .child(&matches_list)
        .build();

    // Lightweight bind editor. It uses the same safe writer layer as the main
    // Binds page, so keyboard-driven edits still receive atomic writes/backups.
    let chord_entry = Entry::builder().editable(false).hexpand(true).build();
    let name_entry = Entry::builder()
        .placeholder_text("Bind name")
        .hexpand(true)
        .build();
    let action_entry = Entry::builder()
        .placeholder_text("hl.dsp.* action or shell command")
        .hexpand(true)
        .build();
    let submap_entry = Entry::builder()
        .placeholder_text("Submap (blank = global)")
        .hexpand(true)
        .build();
    let add_bind_btn = Button::with_label("Add bind for chord");
    add_bind_btn.add_css_class("suggested-action");
    let save_bind_btn = Button::with_label("Save selected bind");
    save_bind_btn.set_sensitive(false);
    let clear_edit_btn = Button::with_label("New bind");

    let editor_grid = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(7)
        .build();
    editor_grid.append(&field_row("Chord", &chord_entry));
    editor_grid.append(&field_row("Name", &name_entry));
    editor_grid.append(&field_row("Action", &action_entry));
    editor_grid.append(&field_row("Submap", &submap_entry));
    let editor_actions = GtkBox::new(Orientation::Horizontal, 8);
    editor_actions.append(&add_bind_btn);
    editor_actions.append(&save_bind_btn);
    editor_actions.append(&clear_edit_btn);
    editor_grid.append(&editor_actions);

    let body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .hexpand(true)
        .vexpand(true)
        .build();
    body.append(&toolbar);
    body.append(
        &Label::builder()
            .label("Live Vial firmware state · modifier toggles construct the host chord · highlighted keys are bound in Hyprland")
            .halign(Align::Start)
            .xalign(0.0)
            .css_classes(["dim-label", "caption"])
            .build(),
    );
    body.append(&board_scroll);
    body.append(&selected_label);
    body.append(&dialog::section_label("Matching Hyprland binds"));
    body.append(&matches_scroll);
    body.append(&dialog::section_label("Configure selected chord"));
    body.append(&editor_grid);

    let reload_collection: Rc<dyn Fn()> = {
        let collection = Rc::clone(&collection);
        let status = Rc::clone(&status);
        Rc::new(move || match config::load_binds(None) {
            Ok(mut loaded) => {
                loaded.finalize();
                let count = loaded.binds.len();
                *collection.borrow_mut() = Some(loaded);
                status(format!("Reloaded Hyprland config: {count} bind(s)"));
            }
            Err(err) => status(format!("Hyprland reload failed: {err}")),
        })
    };

    let reset_editor: Rc<dyn Fn()> = {
        let selected_bind_id = Rc::clone(&selected_bind_id);
        let name_entry = name_entry.clone();
        let action_entry = action_entry.clone();
        let submap_entry = submap_entry.clone();
        let save_bind_btn = save_bind_btn.clone();
        Rc::new(move || {
            selected_bind_id.set(None);
            name_entry.set_text("");
            action_entry.set_text("");
            submap_entry.set_text("");
            submap_entry.set_editable(true);
            save_bind_btn.set_sensitive(false);
        })
    };

    let render_matches: Rc<dyn Fn(Option<String>, String)> = {
        let collection = Rc::clone(&collection);
        let matches_list = matches_list.clone();
        let selected_label = selected_label.clone();
        let chord_entry = chord_entry.clone();
        let selected_submap = Rc::clone(&selected_submap);
        let reset_editor = Rc::clone(&reset_editor);
        Rc::new(move |chord: Option<String>, hardware_explanation: String| {
            clear_list(&matches_list);
            reset_editor();
            let Some(chord) = chord else {
                chord_entry.set_text("");
                selected_label.set_text(&hardware_explanation);
                matches_list.append(&message_row(
                    "This firmware action does not have one exact host/XKB key, so Hyprbinds will not invent a Hyprland chord for it.",
                ));
                return;
            };

            chord_entry.set_text(&chord);
            selected_label.set_text(&format!("{hardware_explanation} · host chord: {chord}"));
            let Some(collection) = collection.borrow().as_ref().cloned() else {
                matches_list.append(&message_row("No Hyprland config loaded."));
                return;
            };
            let filter = selected_submap.borrow().clone();
            let matches = matching_binds(&collection, &chord, &filter);
            if matches.is_empty() {
                matches_list.append(&message_row("No Hyprland bind for this chord in the selected submap context."));
                return;
            }
            for bind in matches {
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
                let row = ListBoxRow::builder().child(&col).build();
                row.set_widget_name(&format!("vial-bind-{}", bind.id));
                matches_list.append(&row);
            }
        })
    };

    matches_list.connect_row_activated({
        let collection = Rc::clone(&collection);
        let selected_bind_id = Rc::clone(&selected_bind_id);
        let name_entry = name_entry.clone();
        let action_entry = action_entry.clone();
        let submap_entry = submap_entry.clone();
        let save_bind_btn = save_bind_btn.clone();
        move |_, row| {
            let Some(id) = row
                .widget_name()
                .strip_prefix("vial-bind-")
                .and_then(|s| s.parse::<usize>().ok())
            else {
                return;
            };
            let guard = collection.borrow();
            let Some(bind) = guard.as_ref().and_then(|c| c.bind_by_id(id)) else {
                return;
            };
            selected_bind_id.set(Some(id));
            name_entry.set_text(&bind.name);
            action_entry.set_text(&bind.action);
            submap_entry.set_text(&bind.submap);
            // writer::save_bind edits the existing call and intentionally does
            // not move it between submaps; adding a new bind can choose submap.
            submap_entry.set_editable(false);
            save_bind_btn.set_sensitive(true);
        }
    });

    clear_edit_btn.connect_clicked({
        let reset_editor = Rc::clone(&reset_editor);
        move |_| reset_editor()
    });

    add_bind_btn.connect_clicked({
        let collection = Rc::clone(&collection);
        let chord_entry = chord_entry.clone();
        let name_entry = name_entry.clone();
        let action_entry = action_entry.clone();
        let submap_entry = submap_entry.clone();
        let status = Rc::clone(&status);
        let reload_collection = Rc::clone(&reload_collection);
        move |_| {
            let chord = chord_entry.text().trim().to_string();
            let action = action_entry.text().trim().to_string();
            if chord.is_empty() {
                status("Select a host-visible physical key first".into());
                return;
            }
            if action.is_empty() {
                status("Enter an action before adding the bind".into());
                return;
            }
            let guard = collection.borrow();
            let Some(loaded) = guard.as_ref() else {
                status("Load a Hyprland config before adding binds".into());
                return;
            };
            let name = if name_entry.text().trim().is_empty() {
                format!("keyboard-{}", chord.replace(" + ", "-").to_ascii_lowercase())
            } else {
                name_entry.text().trim().to_string()
            };
            let result = writer::add_bind(
                &loaded.config_path,
                &name,
                &chord,
                &action,
                submap_entry.text().trim(),
            );
            drop(guard);
            match result {
                Ok(write) => {
                    status(format!("Added {chord} bind via {}", write.path));
                    reload_collection();
                }
                Err(err) => status(format!("Add bind failed: {err}")),
            }
        }
    });

    save_bind_btn.connect_clicked({
        let collection = Rc::clone(&collection);
        let selected_bind_id = Rc::clone(&selected_bind_id);
        let chord_entry = chord_entry.clone();
        let name_entry = name_entry.clone();
        let action_entry = action_entry.clone();
        let status = Rc::clone(&status);
        let reload_collection = Rc::clone(&reload_collection);
        move |_| {
            let Some(id) = selected_bind_id.get() else {
                status("Double-click a matching bind first".into());
                return;
            };
            let chord = chord_entry.text().trim().to_string();
            let name = name_entry.text().trim().to_string();
            let action = action_entry.text().trim().to_string();
            let guard = collection.borrow();
            let Some(loaded) = guard.as_ref() else {
                status("Hyprland config is not loaded".into());
                return;
            };
            let Some(bind) = loaded.bind_by_id(id).cloned() else {
                status("Selected bind disappeared after reload".into());
                return;
            };
            let shared = loaded.source_share_count(&bind) > 1;
            drop(guard);
            match writer::save_bind(&bind, &name, &chord, &action, shared) {
                Ok(write) => {
                    status(format!("Saved {} via {}", bind.name, write.path));
                    reload_collection();
                }
                Err(err) => status(format!("Save bind failed: {err}")),
            }
        }
    });

    let rebuild_board: Rc<dyn Fn()> = {
        let discovery = Rc::clone(&discovery);
        let selected_board = Rc::clone(&selected_board);
        let selected_layer = Rc::clone(&selected_layer);
        let selected_submap = Rc::clone(&selected_submap);
        let board_fixed = board_fixed.clone();
        let super_mod = super_mod.clone();
        let ctrl_mod = ctrl_mod.clone();
        let alt_mod = alt_mod.clone();
        let shift_mod = shift_mod.clone();
        let collection = Rc::clone(&collection);
        let render_matches = Rc::clone(&render_matches);
        Rc::new(move || {
            while let Some(child) = board_fixed.first_child() {
                board_fixed.remove(&child);
            }

            let discovery = discovery.borrow();
            let Some(board) = discovery.boards.get(selected_board.get()) else {
                board_fixed.set_size_request(220, 90);
                return;
            };
            let layer = selected_layer.get();
            let width = ((board.definition.layout.width() as f64) * (UNIT_PX + GAP_PX) + GAP_PX)
                .ceil() as i32;
            let height = ((board.definition.layout.height() as f64) * (UNIT_PX + GAP_PX) + GAP_PX)
                .ceil() as i32;
            board_fixed.set_size_request(width.max(220), height.max(90));

            let modifiers = active_modifiers(&super_mod, &ctrl_mod, &alt_mod, &shift_mod);
            let filter = selected_submap.borrow().clone();
            let loaded = collection.borrow().as_ref().cloned();

            for key in board.physical_keys().iter().cloned() {
                let resolved = resolve_key_identity(board, layer, key.row, key.col);
                let hardware_label = resolved
                    .as_ref()
                    .map(|(label, _, source_layer)| {
                        if *source_layer == layer {
                            label.clone()
                        } else {
                            format!("{label}↙L{source_layer}")
                        }
                    })
                    .unwrap_or_else(|| "NO".into());
                let identity = resolved
                    .as_ref()
                    .map(|(_, identity, _)| identity.clone())
                    .unwrap_or(HardwareKeyIdentity::Disabled);

                let chord = identity
                    .host_key()
                    .map(|host| compose_chord(&modifiers, host));
                let bind_count = chord
                    .as_ref()
                    .and_then(|chord| loaded.as_ref().map(|c| matching_binds(c, chord, &filter).len()))
                    .unwrap_or(0);

                let button_label = if bind_count == 0 {
                    hardware_label.clone()
                } else {
                    format!("{hardware_label}\n{bind_count} bind{}", if bind_count == 1 { "" } else { "s" })
                };
                let tooltip = format!(
                    "{} · {}",
                    resolved
                        .as_ref()
                        .map(|(label, _, _)| via::keycode_full_label(
                            board.keycode_at(layer, key.row, key.col).unwrap_or(0),
                            &board.definition.custom_keycodes,
                        ))
                        .unwrap_or_else(|| "No keycode".into()),
                    identity.explanation()
                );
                let btn = Button::builder()
                    .label(&button_label)
                    .tooltip_text(&tooltip)
                    .css_classes(["hyprbinds-via-key", "hyprbinds-vial-bind-key"])
                    .build();
                if bind_count == 1 {
                    btn.add_css_class("suggested-action");
                } else if bind_count > 1 {
                    btn.add_css_class("destructive-action");
                }
                if matches!(identity, HardwareKeyIdentity::Disabled) {
                    btn.set_sensitive(false);
                }

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

                btn.connect_clicked({
                    let identity = identity.clone();
                    let hardware_label = hardware_label.clone();
                    let super_mod = super_mod.clone();
                    let ctrl_mod = ctrl_mod.clone();
                    let alt_mod = alt_mod.clone();
                    let shift_mod = shift_mod.clone();
                    let render_matches = Rc::clone(&render_matches);
                    move |_| {
                        if let Some(modifier) = identity.host_modifier() {
                            let toggle = match modifier {
                                "SUPER" => &super_mod,
                                "CTRL" => &ctrl_mod,
                                "ALT" => &alt_mod,
                                "SHIFT" => &shift_mod,
                                _ => return,
                            };
                            toggle.set_active(!toggle.is_active());
                            render_matches(
                                None,
                                format!("{hardware_label}: hardware modifier {modifier}; toggled chord filter"),
                            );
                            return;
                        }
                        let chord = identity.host_key().map(|host| {
                            compose_chord(
                                &active_modifiers(&super_mod, &ctrl_mod, &alt_mod, &shift_mod),
                                host,
                            )
                        });
                        render_matches(
                            chord,
                            format!("{hardware_label}: {}", identity.explanation()),
                        );
                    }
                });
            }
        })
    };

    let refresh_submaps: Rc<dyn Fn()> = {
        let collection = Rc::clone(&collection);
        let submap_dd = submap_dd.clone();
        let selected_submap = Rc::clone(&selected_submap);
        Rc::new(move || {
            let mut labels = vec!["All submaps".to_string(), "Global".to_string()];
            if let Some(collection) = collection.borrow().as_ref() {
                labels.extend(collection.submap_names());
            }
            let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
            submap_dd.set_model(Some(&StringList::new(&refs)));
            submap_dd.set_selected(0);
            selected_submap.borrow_mut().clear();
        })
    };

    let refresh: Rc<dyn Fn()> = {
        let discovery = Rc::clone(&discovery);
        let selected_board = Rc::clone(&selected_board);
        let selected_layer = Rc::clone(&selected_layer);
        let board_dd = board_dd.clone();
        let layer_dd = layer_dd.clone();
        let rebuild_board = Rc::clone(&rebuild_board);
        let refresh_submaps = Rc::clone(&refresh_submaps);
        let status = Rc::clone(&status);
        Rc::new(move || match vial_sync::discover() {
            Ok(found) => {
                let labels: Vec<String> = if found.boards.is_empty() {
                    vec!["No Vial keyboard".into()]
                } else {
                    found.boards.iter().map(|b| b.display_name()).collect()
                };
                let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
                board_dd.set_model(Some(&StringList::new(&refs)));
                board_dd.set_selected(0);
                selected_board.set(0);
                selected_layer.set(0);
                let layer_count = found
                    .boards
                    .first()
                    .map(|b| b.layer_count())
                    .unwrap_or(1)
                    .max(1);
                set_layer_model(&layer_dd, layer_count);
                let diagnostics = found.diagnostics.len();
                let boards = found.boards.len();
                *discovery.borrow_mut() = found;
                refresh_submaps();
                rebuild_board();
                status(format!(
                    "Vial visualizer: {boards} board(s), {diagnostics} diagnostic(s)"
                ));
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
            selected_board.set(index);
            selected_layer.set(0);
            let count = discovery
                .borrow()
                .boards
                .get(index)
                .map(|b| b.layer_count())
                .unwrap_or(1)
                .max(1);
            set_layer_model(&layer_dd, count);
            rebuild_board();
        }
    });

    layer_dd.connect_selected_notify({
        let selected_layer = Rc::clone(&selected_layer);
        let layer_dd = layer_dd.clone();
        let rebuild_board = Rc::clone(&rebuild_board);
        move |_| {
            selected_layer.set(layer_dd.selected() as u8);
            rebuild_board();
        }
    });

    submap_dd.connect_selected_notify({
        let selected_submap = Rc::clone(&selected_submap);
        let submap_dd = submap_dd.clone();
        let rebuild_board = Rc::clone(&rebuild_board);
        move |_| {
            let selected = submap_dd.selected();
            let value = match selected {
                0 => String::new(),
                1 => "__global__".into(),
                n => submap_dd
                    .model()
                    .and_then(|m| m.item(n))
                    .and_then(|o| o.downcast::<gtk4::StringObject>().ok())
                    .map(|o| o.string().to_string())
                    .unwrap_or_default(),
            };
            *selected_submap.borrow_mut() = value;
            rebuild_board();
        }
    });

    for toggle in [&super_mod, &ctrl_mod, &alt_mod, &shift_mod] {
        toggle.connect_toggled({
            let rebuild_board = Rc::clone(&rebuild_board);
            move |_| rebuild_board()
        });
    }

    refresh();

    VialVisualizerPage { page: body, refresh }
}

fn resolve_key_identity(
    board: &VialBoardSnapshot,
    selected_layer: u8,
    row: u8,
    col: u8,
) -> Option<(String, HardwareKeyIdentity, u8)> {
    let mut layer = selected_layer as i16;
    while layer >= 0 {
        let code = board.keycode_at(layer as u8, row, col)?;
        let label = via::keycode_short_label(code, &board.definition.custom_keycodes);
        let identity = vial_key_identity::classify_qmk_label(&label);
        if identity.is_transparent() {
            layer -= 1;
            continue;
        }
        return Some((label, identity, layer as u8));
    }
    Some(("TRNS".into(), HardwareKeyIdentity::Transparent, 0))
}

fn matching_binds(collection: &BindCollection, chord: &str, submap_filter: &str) -> Vec<Keybind> {
    let normalized = normalize_keys_input(chord);
    collection
        .binds
        .iter()
        .filter(|bind| submap_matches(bind, submap_filter))
        .filter(|bind| {
            let resolved = expand_template(&bind.keys, &collection.variables)
                .unwrap_or_else(|_| bind.keys.clone());
            normalize_keys_input(&resolved) == normalized
        })
        .cloned()
        .collect()
}

fn submap_matches(bind: &Keybind, filter: &str) -> bool {
    match filter {
        "" => true,
        "__global__" => bind.submap.trim().is_empty(),
        name => bind.submap == name,
    }
}

fn active_modifiers(
    super_mod: &CheckButton,
    ctrl_mod: &CheckButton,
    alt_mod: &CheckButton,
    shift_mod: &CheckButton,
) -> Vec<&'static str> {
    let mut out = Vec::new();
    if super_mod.is_active() {
        out.push("SUPER");
    }
    if ctrl_mod.is_active() {
        out.push("CTRL");
    }
    if alt_mod.is_active() {
        out.push("ALT");
    }
    if shift_mod.is_active() {
        out.push("SHIFT");
    }
    out
}

fn compose_chord(modifiers: &[&str], key: &str) -> String {
    let mut parts: Vec<String> = modifiers.iter().map(|m| (*m).to_string()).collect();
    parts.push(key.to_string());
    parts.join(" + ")
}

fn set_layer_model(layer_dd: &DropDown, count: u8) {
    let labels: Vec<String> = (0..count.max(1)).map(|i| format!("Layer {i}")).collect();
    let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
    layer_dd.set_model(Some(&StringList::new(&refs)));
    layer_dd.set_selected(0);
}

fn field_row(label: &str, widget: &impl IsA<gtk4::Widget>) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.append(
        &Label::builder()
            .label(label)
            .halign(Align::Start)
            .width_request(90)
            .build(),
    );
    row.append(widget);
    row
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
                .wrap(true)
                .css_classes(["dim-label"])
                .build(),
        )
        .build()
}
