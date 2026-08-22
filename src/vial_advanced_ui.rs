//! Native GTK editors for the remaining Vial feature surfaces.

use crate::dialog;
use crate::experimental::via;
use crate::vial_features::{self, AltRepeatEntry, MacroBank, MatrixSnapshot};
use crate::vial_native;
use crate::vial_sync::{self, VialBoardSnapshot};
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, CheckButton, DropDown, Entry, Fixed, Frame, Label, Notebook,
    Orientation, PolicyType, ProgressBar, ScrolledWindow, StringList, TextView,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

const UNIT_PX: f64 = 40.0;
const GAP_PX: f64 = 3.0;

type StatusFn = Rc<dyn Fn(String)>;
type SelectedBoardFn = Rc<dyn Fn() -> Option<VialBoardSnapshot>>;

pub struct AdvancedVialPage {
    pub page: GtkBox,
    pub refresh: Rc<dyn Fn()>,
}

pub fn build_page(status: StatusFn) -> AdvancedVialPage {
    let boards: Rc<RefCell<Vec<VialBoardSnapshot>>> = Rc::new(RefCell::new(Vec::new()));
    let selected = Rc::new(Cell::new(0usize));

    let device_dd = DropDown::from_strings(&["No Vial keyboard"]);
    device_dd.set_hexpand(true);
    let refresh_devices = Button::with_label("Refresh keyboards");
    refresh_devices.add_css_class("suggested-action");

    let selected_board: SelectedBoardFn = {
        let boards = Rc::clone(&boards);
        let selected = Rc::clone(&selected);
        Rc::new(move || boards.borrow().get(selected.get()).cloned())
    };

    let notebook = Notebook::new();
    notebook.set_scrollable(true);
    notebook.set_hexpand(true);
    notebook.set_vexpand(true);

    notebook.append_page(
        &build_macros_page(Rc::clone(&selected_board), Rc::clone(&status)),
        Some(&Label::new(Some("Macros"))),
    );
    notebook.append_page(
        &build_encoders_page(Rc::clone(&selected_board), Rc::clone(&status)),
        Some(&Label::new(Some("Encoders"))),
    );
    notebook.append_page(
        &build_alt_repeat_page(Rc::clone(&selected_board), Rc::clone(&status)),
        Some(&Label::new(Some("Alt Repeat"))),
    );
    notebook.append_page(
        &build_qmk_settings_page(Rc::clone(&selected_board), Rc::clone(&status)),
        Some(&Label::new(Some("QMK Settings"))),
    );
    notebook.append_page(
        &build_security_page(Rc::clone(&selected_board), Rc::clone(&status)),
        Some(&Label::new(Some("Security"))),
    );
    notebook.append_page(
        &build_matrix_page(Rc::clone(&selected_board), Rc::clone(&status)),
        Some(&Label::new(Some("Matrix Test"))),
    );

    let header = GtkBox::new(Orientation::Horizontal, 8);
    header.append(&refresh_devices);
    header.append(&device_dd);

    let page = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .margin_top(10)
        .margin_bottom(10)
        .margin_start(10)
        .margin_end(10)
        .build();
    page.append(&header);
    page.append(&notebook);

    let refresh: Rc<dyn Fn()> = {
        let boards = Rc::clone(&boards);
        let selected = Rc::clone(&selected);
        let device_dd = device_dd.clone();
        let status = Rc::clone(&status);
        Rc::new(move || match vial_sync::discover() {
            Ok(found) => {
                let labels: Vec<String> = if found.boards.is_empty() {
                    vec!["No Vial keyboard".into()]
                } else {
                    found.boards.iter().map(|b| b.display_name()).collect()
                };
                let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
                device_dd.set_model(Some(&StringList::new(&refs)));
                device_dd.set_selected(0);
                selected.set(0);
                let count = found.boards.len();
                let diagnostics = found.diagnostics.len();
                *boards.borrow_mut() = found.boards;
                status(format!(
                    "Advanced Vial: {count} board(s), {diagnostics} diagnostic(s)"
                ));
            }
            Err(err) => status(format!("Vial discovery failed: {err}")),
        })
    };

    refresh_devices.connect_clicked({
        let refresh = Rc::clone(&refresh);
        move |_| refresh()
    });
    device_dd.connect_selected_notify({
        let selected = Rc::clone(&selected);
        let device_dd = device_dd.clone();
        move |_| selected.set(device_dd.selected() as usize)
    });

    refresh();
    AdvancedVialPage { page, refresh }
}

fn build_macros_page(selected: SelectedBoardFn, status: StatusFn) -> GtkBox {
    let bank: Rc<RefCell<Option<MacroBank>>> = Rc::new(RefCell::new(None));
    let slot_dd = DropDown::from_strings(&["Macro 0"]);
    let editor = TextView::new();
    editor.set_monospace(true);
    editor.set_wrap_mode(gtk4::WrapMode::WordChar);
    let editor_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .min_content_height(320)
        .vexpand(true)
        .child(&editor)
        .build();
    let help = Label::builder()
        .label("Actions are edited as JSON: Text, Tap, Down, Up, Delay, or Raw. Keycodes are decimal u16 values; hex literals are shown in firmware key labels elsewhere.")
        .halign(Align::Start)
        .xalign(0.0)
        .wrap(true)
        .css_classes(["dim-label", "caption"])
        .build();
    let load = Button::with_label("Read macros");
    let save = Button::with_label("Apply slot + write bank");
    save.add_css_class("suggested-action");
    let reset = Button::with_label("Reset macros");
    reset.add_css_class("destructive-action");

    let render_slot: Rc<dyn Fn()> = {
        let bank = Rc::clone(&bank);
        let slot_dd = slot_dd.clone();
        let editor = editor.clone();
        let status = Rc::clone(&status);
        Rc::new(move || {
            let Some(bank) = bank.borrow().as_ref().cloned() else {
                return;
            };
            let idx = slot_dd.selected() as usize;
            let Some(actions) = bank.macros.get(idx) else {
                return;
            };
            match vial_features::macro_to_json(actions) {
                Ok(text) => editor.buffer().set_text(&text),
                Err(err) => status(format!("Macro render failed: {err}")),
            }
        })
    };

    load.connect_clicked({
        let selected = Rc::clone(&selected);
        let bank = Rc::clone(&bank);
        let slot_dd = slot_dd.clone();
        let render_slot = Rc::clone(&render_slot);
        let status = Rc::clone(&status);
        move |_| {
            let Some(board) = selected() else {
                status("Select a Vial keyboard first".into());
                return;
            };
            match vial_features::read_macro_bank(board.device.vendor_id, board.device.product_id) {
                Ok(value) => {
                    let labels: Vec<String> = (0..value.count).map(|i| format!("Macro {i}" )).collect();
                    let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
                    slot_dd.set_model(Some(&StringList::new(&refs)));
                    slot_dd.set_selected(0);
                    status(format!(
                        "Read {} macro slot(s), {} bytes, Vial protocol {}",
                        value.count, value.buffer_size, value.vial_protocol
                    ));
                    *bank.borrow_mut() = Some(value);
                    render_slot();
                }
                Err(err) => status(format!("Read macros failed: {err}")),
            }
        }
    });

    slot_dd.connect_selected_notify({
        let render_slot = Rc::clone(&render_slot);
        move |_| render_slot()
    });

    save.connect_clicked({
        let selected = Rc::clone(&selected);
        let bank = Rc::clone(&bank);
        let slot_dd = slot_dd.clone();
        let editor = editor.clone();
        let status = Rc::clone(&status);
        move |_| {
            let Some(board) = selected() else {
                status("Select a Vial keyboard first".into());
                return;
            };
            let mut guard = bank.borrow_mut();
            let Some(bank) = guard.as_mut() else {
                status("Read macros before editing".into());
                return;
            };
            let buffer = editor.buffer();
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
            let actions = match vial_features::macro_from_json(text.as_str()) {
                Ok(actions) => actions,
                Err(err) => {
                    status(err);
                    return;
                }
            };
            let idx = slot_dd.selected() as usize;
            let Some(slot) = bank.macros.get_mut(idx) else {
                status("Invalid macro slot".into());
                return;
            };
            *slot = actions;
            match vial_features::write_macro_bank(board.device.vendor_id, board.device.product_id, bank) {
                Ok(()) => status(format!("Macro {idx} saved safely")),
                Err(err) => status(format!("Macro write failed: {err}")),
            }
        }
    });

    reset.connect_clicked({
        let selected = Rc::clone(&selected);
        let bank = Rc::clone(&bank);
        let editor = editor.clone();
        let status = Rc::clone(&status);
        move |_| {
            let Some(board) = selected() else {
                status("Select a Vial keyboard first".into());
                return;
            };
            match vial_native::reset_macros(board.device.vendor_id, board.device.product_id) {
                Ok(()) => {
                    *bank.borrow_mut() = None;
                    editor.buffer().set_text("");
                    status("Macro EEPROM reset requested".into());
                }
                Err(err) => status(format!("Macro reset failed: {err}")),
            }
        }
    });

    let controls = GtkBox::new(Orientation::Horizontal, 8);
    controls.append(&load);
    controls.append(&slot_dd);
    controls.append(&save);
    controls.append(&reset);

    let page = page_shell("Advanced macros", &help);
    page.append(&controls);
    page.append(&editor_scroll);
    page
}

fn build_encoders_page(selected: SelectedBoardFn, status: StatusFn) -> GtkBox {
    let layer_dd = DropDown::from_strings(&["Layer 0"]);
    let encoder_dd = DropDown::from_strings(&["Encoder 0"]);
    let ccw = hex_entry("counter-clockwise keycode");
    let cw = hex_entry("clockwise keycode");
    let load = Button::with_label("Read encoder");
    let apply = Button::with_label("Apply encoder");
    apply.add_css_class("suggested-action");

    let sync_models: Rc<dyn Fn()> = {
        let selected = Rc::clone(&selected);
        let layer_dd = layer_dd.clone();
        let encoder_dd = encoder_dd.clone();
        let status = Rc::clone(&status);
        Rc::new(move || {
            let Some(board) = selected() else {
                return;
            };
            let layers: Vec<String> = (0..board.layer_count()).map(|i| format!("Layer {i}")).collect();
            let layer_refs: Vec<&str> = layers.iter().map(String::as_str).collect();
            layer_dd.set_model(Some(&StringList::new(&layer_refs)));
            layer_dd.set_selected(0);
            match vial_features::encoder_count(board.device.vendor_id, board.device.product_id) {
                Ok(count) => {
                    let labels: Vec<String> = if count == 0 {
                        vec!["No encoders".into()]
                    } else {
                        (0..count).map(|i| format!("Encoder {i}")).collect()
                    };
                    let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
                    encoder_dd.set_model(Some(&StringList::new(&refs)));
                    encoder_dd.set_selected(0);
                    status(format!("Encoder layout: {count} encoder(s)"));
                }
                Err(err) => status(format!("Encoder discovery failed: {err}")),
            }
        })
    };

    load.connect_clicked({
        let selected = Rc::clone(&selected);
        let layer_dd = layer_dd.clone();
        let encoder_dd = encoder_dd.clone();
        let ccw = ccw.clone();
        let cw = cw.clone();
        let status = Rc::clone(&status);
        let sync_models = Rc::clone(&sync_models);
        move |_| {
            sync_models();
            let Some(board) = selected() else {
                status("Select a Vial keyboard first".into());
                return;
            };
            let layer = layer_dd.selected() as u8;
            let encoder = encoder_dd.selected() as u8;
            match vial_features::read_vial_encoder(
                board.device.vendor_id,
                board.device.product_id,
                layer,
                encoder,
            ) {
                Ok(pair) => {
                    ccw.set_text(&format!("{:04X}", pair.counter_clockwise));
                    cw.set_text(&format!("{:04X}", pair.clockwise));
                    status(format!("Read encoder {encoder}, layer {layer}"));
                }
                Err(err) => status(format!("Encoder read failed: {err}")),
            }
        }
    });

    apply.connect_clicked({
        let selected = Rc::clone(&selected);
        let layer_dd = layer_dd.clone();
        let encoder_dd = encoder_dd.clone();
        let ccw = ccw.clone();
        let cw = cw.clone();
        let status = Rc::clone(&status);
        move |_| {
            let Some(board) = selected() else {
                status("Select a Vial keyboard first".into());
                return;
            };
            let Some(ccw_code) = parse_hex_u16(&ccw.text()) else {
                status("Invalid counter-clockwise keycode".into());
                return;
            };
            let Some(cw_code) = parse_hex_u16(&cw.text()) else {
                status("Invalid clockwise keycode".into());
                return;
            };
            let layer = layer_dd.selected() as u8;
            let encoder = encoder_dd.selected() as u8;
            let result = vial_features::write_vial_encoder(
                board.device.vendor_id,
                board.device.product_id,
                layer,
                encoder,
                false,
                ccw_code,
            )
            .and_then(|_| {
                vial_features::write_vial_encoder(
                    board.device.vendor_id,
                    board.device.product_id,
                    layer,
                    encoder,
                    true,
                    cw_code,
                )
            });
            match result {
                Ok(()) => status(format!("Updated encoder {encoder}, layer {layer}")),
                Err(err) => status(format!("Encoder write failed: {err}")),
            }
        }
    });

    let page = page_shell(
        "Encoders",
        &info_label("Vial encoder pairs are read/written through the firmware's native encoder command, not guessed from matrix keys."),
    );
    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.append(&load);
    row.append(&layer_dd);
    row.append(&encoder_dd);
    page.append(&row);
    page.append(&field_row("Counter-clockwise", &ccw));
    page.append(&field_row("Clockwise", &cw));
    page.append(&apply);
    page
}

fn build_alt_repeat_page(selected: SelectedBoardFn, status: StatusFn) -> GtkBox {
    let entries: Rc<RefCell<Vec<AltRepeatEntry>>> = Rc::new(RefCell::new(Vec::new()));
    let slot_dd = DropDown::from_strings(&["Slot 0"]);
    let keycode = hex_entry("keycode");
    let alt_keycode = hex_entry("alternate keycode");
    let allowed_mods = hex_entry("allowed mods byte");
    let enabled = CheckButton::with_label("Enabled");
    let default_alt = CheckButton::with_label("Default to alternate");
    let bidirectional = CheckButton::with_label("Bidirectional");
    let ignore_handedness = CheckButton::with_label("Ignore modifier handedness");
    let load = Button::with_label("Read Alt Repeat");
    let apply = Button::with_label("Apply slot");
    apply.add_css_class("suggested-action");

    let render: Rc<dyn Fn()> = {
        let entries = Rc::clone(&entries);
        let slot_dd = slot_dd.clone();
        let keycode = keycode.clone();
        let alt_keycode = alt_keycode.clone();
        let allowed_mods = allowed_mods.clone();
        let enabled = enabled.clone();
        let default_alt = default_alt.clone();
        let bidirectional = bidirectional.clone();
        let ignore_handedness = ignore_handedness.clone();
        Rc::new(move || {
            let idx = slot_dd.selected() as usize;
            let guard = entries.borrow();
            let Some(entry) = guard.get(idx) else {
                return;
            };
            keycode.set_text(&format!("{:04X}", entry.keycode));
            alt_keycode.set_text(&format!("{:04X}", entry.alt_keycode));
            allowed_mods.set_text(&format!("{:02X}", entry.allowed_mods));
            enabled.set_active(entry.enabled());
            default_alt.set_active(entry.default_to_alt());
            bidirectional.set_active(entry.bidirectional());
            ignore_handedness.set_active(entry.ignore_mod_handedness());
        })
    };

    load.connect_clicked({
        let selected = Rc::clone(&selected);
        let entries = Rc::clone(&entries);
        let slot_dd = slot_dd.clone();
        let render = Rc::clone(&render);
        let status = Rc::clone(&status);
        move |_| {
            let Some(board) = selected() else {
                status("Select a Vial keyboard first".into());
                return;
            };
            match vial_features::read_alt_repeat_entries(board.device.vendor_id, board.device.product_id) {
                Ok(found) => {
                    let labels: Vec<String> = if found.is_empty() {
                        vec!["No Alt Repeat slots".into()]
                    } else {
                        (0..found.len()).map(|i| format!("Slot {i}")).collect()
                    };
                    let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
                    slot_dd.set_model(Some(&StringList::new(&refs)));
                    slot_dd.set_selected(0);
                    let count = found.len();
                    *entries.borrow_mut() = found;
                    render();
                    status(format!("Read {count} Alt Repeat slot(s)"));
                }
                Err(err) => status(format!("Alt Repeat read failed: {err}")),
            }
        }
    });
    slot_dd.connect_selected_notify({
        let render = Rc::clone(&render);
        move |_| render()
    });

    apply.connect_clicked({
        let selected = Rc::clone(&selected);
        let entries = Rc::clone(&entries);
        let slot_dd = slot_dd.clone();
        let keycode = keycode.clone();
        let alt_keycode = alt_keycode.clone();
        let allowed_mods = allowed_mods.clone();
        let enabled = enabled.clone();
        let default_alt = default_alt.clone();
        let bidirectional = bidirectional.clone();
        let ignore_handedness = ignore_handedness.clone();
        let status = Rc::clone(&status);
        move |_| {
            let Some(board) = selected() else {
                status("Select a Vial keyboard first".into());
                return;
            };
            let Some(kc) = parse_hex_u16(&keycode.text()) else {
                status("Invalid Alt Repeat keycode".into());
                return;
            };
            let Some(alt) = parse_hex_u16(&alt_keycode.text()) else {
                status("Invalid alternate keycode".into());
                return;
            };
            let Some(mods) = parse_hex_u8(&allowed_mods.text()) else {
                status("Invalid allowed-modifiers byte".into());
                return;
            };
            let mut options = 0u8;
            if default_alt.is_active() { options |= 1 << 0; }
            if bidirectional.is_active() { options |= 1 << 1; }
            if ignore_handedness.is_active() { options |= 1 << 2; }
            if enabled.is_active() { options |= 1 << 3; }
            let entry = AltRepeatEntry {
                keycode: kc,
                alt_keycode: alt,
                allowed_mods: mods,
                options,
            };
            let idx = slot_dd.selected() as usize;
            match vial_features::write_alt_repeat_entry(
                board.device.vendor_id,
                board.device.product_id,
                idx as u8,
                &entry,
            ) {
                Ok(()) => {
                    if let Some(slot) = entries.borrow_mut().get_mut(idx) {
                        *slot = entry;
                    }
                    status(format!("Updated Alt Repeat slot {idx}"));
                }
                Err(err) => status(format!("Alt Repeat write failed: {err}")),
            }
        }
    });

    let page = page_shell(
        "Alt Repeat",
        &info_label("Vial dynamic Alt Repeat entries: primary key, alternate key, modifier mask, and behavior flags."),
    );
    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.append(&load);
    row.append(&slot_dd);
    page.append(&row);
    page.append(&field_row("Keycode", &keycode));
    page.append(&field_row("Alternate", &alt_keycode));
    page.append(&field_row("Allowed mods", &allowed_mods));
    page.append(&enabled);
    page.append(&default_alt);
    page.append(&bidirectional);
    page.append(&ignore_handedness);
    page.append(&apply);
    page
}

fn build_qmk_settings_page(selected: SelectedBoardFn, status: StatusFn) -> GtkBox {
    let ids: Rc<RefCell<Vec<u16>>> = Rc::new(RefCell::new(Vec::new()));
    let setting_dd = DropDown::from_strings(&["No settings"]);
    let value = Entry::builder()
        .placeholder_text("little-endian value, e.g. 200 or 0x00C8")
        .hexpand(true)
        .build();
    let metadata = Label::builder()
        .label("Read settings to inspect supported IDs.")
        .halign(Align::Start)
        .xalign(0.0)
        .wrap(true)
        .css_classes(["dim-label", "caption"])
        .build();
    let load = Button::with_label("Read settings");
    let read = Button::with_label("Read selected");
    let apply = Button::with_label("Apply selected");
    apply.add_css_class("suggested-action");
    let reset = Button::with_label("Reset all settings");
    reset.add_css_class("destructive-action");

    let refresh_meta: Rc<dyn Fn()> = {
        let ids = Rc::clone(&ids);
        let setting_dd = setting_dd.clone();
        let metadata = metadata.clone();
        Rc::new(move || {
            let idx = setting_dd.selected() as usize;
            let Some(id) = ids.borrow().get(idx).copied() else {
                metadata.set_text("No selected setting.");
                return;
            };
            let (name, width) = qmk_setting_metadata(id);
            metadata.set_text(&format!(
                "{} · ID 0x{id:04X} · {}",
                name,
                width.map(|w| format!("{} byte(s), little-endian", w)).unwrap_or_else(|| "unknown width (read-only)".into())
            ));
            apply.set_sensitive(width.is_some());
        })
    };

    load.connect_clicked({
        let selected = Rc::clone(&selected);
        let ids = Rc::clone(&ids);
        let setting_dd = setting_dd.clone();
        let refresh_meta = Rc::clone(&refresh_meta);
        let status = Rc::clone(&status);
        move |_| {
            let Some(board) = selected() else {
                status("Select a Vial keyboard first".into());
                return;
            };
            match vial_native::inspect(board.device.vendor_id, board.device.product_id) {
                Ok(snapshot) => {
                    let labels: Vec<String> = snapshot
                        .qmk_setting_ids
                        .iter()
                        .map(|id| format!("{} · 0x{id:04X}", qmk_setting_metadata(*id).0))
                        .collect();
                    let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
                    setting_dd.set_model(Some(&StringList::new(&refs)));
                    setting_dd.set_selected(0);
                    let count = snapshot.qmk_setting_ids.len();
                    *ids.borrow_mut() = snapshot.qmk_setting_ids;
                    refresh_meta();
                    status(format!("Read {count} QMK setting ID(s)"));
                }
                Err(err) => status(format!("QMK settings query failed: {err}")),
            }
        }
    });
    setting_dd.connect_selected_notify({
        let refresh_meta = Rc::clone(&refresh_meta);
        move |_| refresh_meta()
    });

    read.connect_clicked({
        let selected = Rc::clone(&selected);
        let ids = Rc::clone(&ids);
        let setting_dd = setting_dd.clone();
        let value = value.clone();
        let status = Rc::clone(&status);
        move |_| {
            let Some(board) = selected() else {
                status("Select a Vial keyboard first".into());
                return;
            };
            let Some(id) = ids.borrow().get(setting_dd.selected() as usize).copied() else {
                status("Select a setting first".into());
                return;
            };
            match vial_features::qmk_setting_get(board.device.vendor_id, board.device.product_id, id) {
                Ok(raw) => {
                    let width = qmk_setting_metadata(id).1.unwrap_or(raw.len().min(4));
                    let bytes = &raw[..width.min(raw.len())];
                    let numeric = bytes.iter().enumerate().fold(0u64, |acc, (i, b)| acc | ((*b as u64) << (i * 8)));
                    value.set_text(&numeric.to_string());
                    status(format!("Read QMK setting 0x{id:04X}: {}", hex_bytes(bytes)));
                }
                Err(err) => status(format!("QMK setting read failed: {err}")),
            }
        }
    });

    apply.connect_clicked({
        let selected = Rc::clone(&selected);
        let ids = Rc::clone(&ids);
        let setting_dd = setting_dd.clone();
        let value = value.clone();
        let status = Rc::clone(&status);
        move |_| {
            let Some(board) = selected() else {
                status("Select a Vial keyboard first".into());
                return;
            };
            let Some(id) = ids.borrow().get(setting_dd.selected() as usize).copied() else {
                status("Select a setting first".into());
                return;
            };
            let Some(width) = qmk_setting_metadata(id).1 else {
                status("Unknown setting width; refusing an unsafe write".into());
                return;
            };
            let Some(numeric) = parse_u64(&value.text()) else {
                status("Invalid numeric setting value".into());
                return;
            };
            if width < 8 && numeric >= (1u64 << (width * 8)) {
                status(format!("Value does not fit in {width} byte(s)"));
                return;
            }
            let bytes = numeric.to_le_bytes();
            match vial_features::qmk_setting_set(
                board.device.vendor_id,
                board.device.product_id,
                id,
                &bytes[..width],
            ) {
                Ok(()) => status(format!("Updated QMK setting 0x{id:04X}")),
                Err(err) => status(format!("QMK setting write failed: {err}")),
            }
        }
    });

    reset.connect_clicked({
        let selected = Rc::clone(&selected);
        let status = Rc::clone(&status);
        move |_| {
            let Some(board) = selected() else {
                status("Select a Vial keyboard first".into());
                return;
            };
            match vial_features::qmk_settings_reset(board.device.vendor_id, board.device.product_id) {
                Ok(()) => status("QMK settings reset requested".into()),
                Err(err) => status(format!("QMK settings reset failed: {err}")),
            }
        }
    });

    let page = page_shell(
        "QMK Settings",
        &info_label("Only settings with a known official Vial width are writable. Unknown IDs remain inspectable but Hyprbinds refuses to guess their wire size."),
    );
    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.append(&load);
    row.append(&setting_dd);
    row.append(&read);
    page.append(&row);
    page.append(&metadata);
    page.append(&field_row("Value", &value));
    let actions = GtkBox::new(Orientation::Horizontal, 8);
    actions.append(&apply);
    actions.append(&reset);
    page.append(&actions);
    page
}

fn build_security_page(selected: SelectedBoardFn, status: StatusFn) -> GtkBox {
    let state = Label::builder()
        .label("Security state not read yet.")
        .halign(Align::Start)
        .xalign(0.0)
        .wrap(true)
        .build();
    let required = Label::builder()
        .label("")
        .halign(Align::Start)
        .xalign(0.0)
        .wrap(true)
        .css_classes(["dim-label", "caption"])
        .build();
    let progress = ProgressBar::new();
    progress.set_show_text(true);
    let read = Button::with_label("Read lock state");
    let unlock = Button::with_label("Begin unlock");
    unlock.add_css_class("suggested-action");
    let lock = Button::with_label("Lock now");

    let refresh_state: Rc<dyn Fn()> = {
        let selected = Rc::clone(&selected);
        let state = state.clone();
        let required = required.clone();
        let status = Rc::clone(&status);
        Rc::new(move || {
            let Some(board) = selected() else {
                status("Select a Vial keyboard first".into());
                return;
            };
            match vial_features::read_unlock_status(board.device.vendor_id, board.device.product_id) {
                Ok(unlock) => {
                    state.set_text(if unlock.unlocked {
                        "Keyboard is UNLOCKED. Firmware-changing protected operations are permitted."
                    } else if unlock.in_progress {
                        "Unlock is in progress — hold the required physical keys."
                    } else {
                        "Keyboard is locked."
                    });
                    let keys = unlock
                        .required_keys
                        .iter()
                        .map(|(r, c)| format!("({r},{c})"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    required.set_text(if keys.is_empty() {
                        "Firmware did not report dedicated unlock keys."
                    } else {
                        &format!("Hold matrix keys: {keys}")
                    });
                    status("Vial security state refreshed".into());
                }
                Err(err) => status(format!("Security status read failed: {err}")),
            }
        })
    };
    read.connect_clicked({
        let refresh_state = Rc::clone(&refresh_state);
        move |_| refresh_state()
    });

    unlock.connect_clicked({
        let selected = Rc::clone(&selected);
        let progress = progress.clone();
        let state = state.clone();
        let status = Rc::clone(&status);
        let refresh_state = Rc::clone(&refresh_state);
        move |_| {
            let Some(board) = selected() else {
                status("Select a Vial keyboard first".into());
                return;
            };
            let vid = board.device.vendor_id;
            let pid = board.device.product_id;
            match vial_features::unlock_start(vid, pid) {
                Ok(()) => {
                    state.set_text("Unlock started — hold the firmware-reported keys until complete.");
                    progress.set_fraction(0.0);
                    progress.set_text(Some("Hold unlock keys"));
                    let progress = progress.clone();
                    let status = Rc::clone(&status);
                    let refresh_state = Rc::clone(&refresh_state);
                    let max_counter = Rc::new(Cell::new(1u8));
                    glib::timeout_add_local(Duration::from_millis(200), move || {
                        match vial_features::unlock_poll(vid, pid) {
                            Ok(poll) => {
                                let max_seen = max_counter.get().max(poll.counter).max(1);
                                max_counter.set(max_seen);
                                let fraction = 1.0 - poll.counter as f64 / max_seen as f64;
                                progress.set_fraction(fraction.clamp(0.0, 1.0));
                                progress.set_text(Some(if poll.unlocked { "Unlocked" } else { "Keep holding…" }));
                                if poll.unlocked {
                                    status("Vial keyboard unlocked".into());
                                    refresh_state();
                                    glib::ControlFlow::Break
                                } else {
                                    glib::ControlFlow::Continue
                                }
                            }
                            Err(err) => {
                                status(format!("Unlock poll failed: {err}"));
                                glib::ControlFlow::Break
                            }
                        }
                    });
                }
                Err(err) => status(format!("Unlock start failed: {err}")),
            }
        }
    });

    lock.connect_clicked({
        let selected = Rc::clone(&selected);
        let status = Rc::clone(&status);
        let refresh_state = Rc::clone(&refresh_state);
        move |_| {
            let Some(board) = selected() else {
                status("Select a Vial keyboard first".into());
                return;
            };
            match vial_features::lock(board.device.vendor_id, board.device.product_id) {
                Ok(()) => {
                    status("Vial keyboard locked".into());
                    refresh_state();
                }
                Err(err) => status(format!("Lock failed: {err}")),
            }
        }
    });

    let page = page_shell(
        "Vial security",
        &info_label("Unlock only on a computer you trust. Lock again here or replug the keyboard when finished."),
    );
    let actions = GtkBox::new(Orientation::Horizontal, 8);
    actions.append(&read);
    actions.append(&unlock);
    actions.append(&lock);
    page.append(&actions);
    page.append(&state);
    page.append(&required);
    page.append(&progress);
    page
}

fn build_matrix_page(selected: SelectedBoardFn, status: StatusFn) -> GtkBox {
    let fixed = Fixed::new();
    let frame = Frame::builder().child(&fixed).hexpand(true).build();
    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .min_content_height(320)
        .vexpand(true)
        .child(&frame)
        .build();
    let running = Rc::new(Cell::new(false));
    let key_widgets: Rc<RefCell<Vec<(u8, u8, Button)>>> = Rc::new(RefCell::new(Vec::new()));
    let start = Button::with_label("Start matrix test");
    start.add_css_class("suggested-action");
    let stop = Button::with_label("Stop");
    let clear = Button::with_label("Clear latched keys");

    let build_geometry: Rc<dyn Fn(&VialBoardSnapshot)> = {
        let fixed = fixed.clone();
        let key_widgets = Rc::clone(&key_widgets);
        Rc::new(move |board: &VialBoardSnapshot| {
            while let Some(child) = fixed.first_child() {
                fixed.remove(&child);
            }
            key_widgets.borrow_mut().clear();
            let width = (board.definition.layout.width() as f64 * (UNIT_PX + GAP_PX) + GAP_PX).ceil() as i32;
            let height = (board.definition.layout.height() as f64 * (UNIT_PX + GAP_PX) + GAP_PX).ceil() as i32;
            fixed.set_size_request(width.max(220), height.max(100));
            for key in board.physical_keys() {
                let btn = Button::builder()
                    .label(&format!("{},{}", key.row, key.col))
                    .css_classes(["hyprbinds-via-key"])
                    .sensitive(false)
                    .build();
                let w = (key.w as f64 * UNIT_PX + (key.w as f64 - 1.0).max(0.0) * GAP_PX).max(28.0);
                let h = (key.h as f64 * UNIT_PX + (key.h as f64 - 1.0).max(0.0) * GAP_PX).max(28.0);
                btn.set_size_request(w as i32, h as i32);
                let x = GAP_PX + key.x as f64 * (UNIT_PX + GAP_PX);
                let y = GAP_PX + key.y as f64 * (UNIT_PX + GAP_PX);
                fixed.put(&btn, x, y);
                key_widgets.borrow_mut().push((key.row, key.col, btn));
            }
        })
    };

    start.connect_clicked({
        let selected = Rc::clone(&selected);
        let running = Rc::clone(&running);
        let key_widgets = Rc::clone(&key_widgets);
        let build_geometry = Rc::clone(&build_geometry);
        let status = Rc::clone(&status);
        move |_| {
            let Some(board) = selected() else {
                status("Select a Vial keyboard first".into());
                return;
            };
            if let Ok(Some(info)) = via::detect_vial(board.device.vendor_id, board.device.product_id) {
                if info.protocol_version < vial_features::VIAL_PROTOCOL_MATRIX_TESTER {
                    status(format!(
                        "Matrix tester requires Vial protocol >= {}, board reports {}",
                        vial_features::VIAL_PROTOCOL_MATRIX_TESTER,
                        info.protocol_version
                    ));
                    return;
                }
            }
            match vial_features::read_unlock_status(board.device.vendor_id, board.device.product_id) {
                Ok(lock) if !lock.unlocked => {
                    status("Unlock the keyboard in the Security tab before matrix testing".into());
                    return;
                }
                Err(err) => {
                    status(format!("Cannot verify unlock state: {err}"));
                    return;
                }
                _ => {}
            }
            build_geometry(&board);
            running.set(true);
            let running = Rc::clone(&running);
            let key_widgets = Rc::clone(&key_widgets);
            let status = Rc::clone(&status);
            let vid = board.device.vendor_id;
            let pid = board.device.product_id;
            let rows = board.definition.rows;
            let cols = board.definition.cols;
            glib::timeout_add_local(Duration::from_millis(30), move || {
                if !running.get() {
                    return glib::ControlFlow::Break;
                }
                match vial_features::matrix_poll(vid, pid, rows, cols) {
                    Ok(snapshot) => apply_matrix_snapshot(&snapshot, &key_widgets.borrow()),
                    Err(err) => {
                        status(format!("Matrix poll failed: {err}"));
                        running.set(false);
                        return glib::ControlFlow::Break;
                    }
                }
                glib::ControlFlow::Continue
            });
            status("Matrix tester running".into());
        }
    });
    stop.connect_clicked({
        let running = Rc::clone(&running);
        let status = Rc::clone(&status);
        move |_| {
            running.set(false);
            status("Matrix tester stopped".into());
        }
    });
    clear.connect_clicked({
        let key_widgets = Rc::clone(&key_widgets);
        move |_| {
            for (_, _, button) in key_widgets.borrow().iter() {
                button.remove_css_class("suggested-action");
                button.remove_css_class("destructive-action");
            }
        }
    });

    let page = page_shell(
        "Safe matrix tester",
        &info_label("Live switch state is polled through VIA keyboard value 0x03. The keyboard must be unlocked first, matching official Vial safety behavior."),
    );
    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.append(&start);
    row.append(&stop);
    row.append(&clear);
    page.append(&row);
    page.append(&scroll);
    page
}

fn apply_matrix_snapshot(snapshot: &MatrixSnapshot, widgets: &[(u8, u8, Button)]) {
    for (row, col, button) in widgets {
        let pressed = snapshot
            .pressed
            .get(*row as usize)
            .and_then(|r| r.get(*col as usize))
            .copied()
            .unwrap_or(false);
        if pressed {
            button.add_css_class("suggested-action");
            // Keep a latched visual trace after release.
            button.add_css_class("destructive-action");
        } else {
            button.remove_css_class("suggested-action");
        }
    }
}

fn page_shell(title: &str, info: &Label) -> GtkBox {
    let page = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(14)
        .margin_bottom(14)
        .margin_start(14)
        .margin_end(14)
        .build();
    page.append(&dialog::section_label(title));
    page.append(info);
    page
}

fn info_label(text: &str) -> Label {
    Label::builder()
        .label(text)
        .halign(Align::Start)
        .xalign(0.0)
        .wrap(true)
        .css_classes(["dim-label", "caption"])
        .build()
}

fn field_row(label: &str, widget: &impl IsA<gtk4::Widget>) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 10);
    row.append(
        &Label::builder()
            .label(label)
            .halign(Align::Start)
            .width_request(150)
            .build(),
    );
    row.append(widget);
    row
}

fn hex_entry(placeholder: &str) -> Entry {
    Entry::builder()
        .placeholder_text(placeholder)
        .width_chars(12)
        .hexpand(true)
        .build()
}

fn parse_hex_u16(text: &str) -> Option<u16> {
    let text = text.trim().trim_start_matches("0x").trim_start_matches("0X");
    u16::from_str_radix(text, 16).ok()
}

fn parse_hex_u8(text: &str) -> Option<u8> {
    let text = text.trim().trim_start_matches("0x").trim_start_matches("0X");
    u8::from_str_radix(text, 16).ok()
}

fn parse_u64(text: &str) -> Option<u64> {
    let text = text.trim();
    if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).ok()
    } else {
        text.parse::<u64>().ok()
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ")
}

/// Metadata mirrored from official Vial's qmk_settings.json plus the newer
/// pointing-device IDs already documented by the Rust protocol backend.
fn qmk_setting_metadata(id: u16) -> (&'static str, Option<usize>) {
    match id {
        1 => ("Grave Escape options", Some(1)),
        2 => ("Combo term", Some(2)),
        3 => ("Auto Shift options", Some(1)),
        4 => ("Auto Shift timeout", Some(2)),
        5 => ("One Shot tap toggle", Some(1)),
        6 => ("One Shot timeout", Some(2)),
        7 => ("Tapping term", Some(2)),
        8 => ("Tap-Hold options", Some(1)),
        9 => ("Mouse key delay", Some(2)),
        10 => ("Mouse key interval", Some(2)),
        11 => ("Mouse key step", Some(2)),
        12 => ("Mouse key max speed", Some(2)),
        13 => ("Mouse key acceleration time", Some(2)),
        14 => ("Mouse wheel delay", Some(2)),
        15 => ("Mouse wheel interval", Some(2)),
        16 => ("Mouse wheel max steps", Some(2)),
        17 => ("Mouse wheel acceleration time", Some(2)),
        18 => ("Tap code delay", Some(2)),
        19 => ("Tap Hold Caps delay", Some(2)),
        20 => ("Tapping toggle", Some(1)),
        21 => ("Magic keymap options", Some(4)),
        22 => ("Permissive Hold", Some(1)),
        23 => ("Hold On Other Key Press", Some(1)),
        24 => ("Retro Tapping", Some(1)),
        25 => ("Quick Tap term", Some(2)),
        26 => ("Chordal Hold", Some(1)),
        27 => ("Flow Tap", Some(2)),
        0x0100 => ("Pointing device DPI", Some(2)),
        0x0101 => ("Scroll divisor", Some(1)),
        0x0102 => ("Horizontal scroll divisor", Some(1)),
        0x0103 => ("Invert pointing X", Some(1)),
        0x0104 => ("Invert pointing Y", Some(1)),
        0x0105 => ("Invert scroll", Some(1)),
        0x0106 => ("Drag scroll", Some(1)),
        0x0107 => ("Drag scroll divisor", Some(1)),
        0x0108 => ("Second pointing-device DPI", Some(2)),
        0x0109 => ("Sniping DPI", Some(2)),
        0x0110 => ("Auto mouse enable", Some(1)),
        0x0111 => ("Auto mouse layer", Some(1)),
        0x0112 => ("Auto mouse timeout", Some(2)),
        _ => ("Unknown QMK setting", None),
    }
}
