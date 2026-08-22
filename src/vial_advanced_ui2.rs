//! Advanced native Vial editors: macros, encoders, Alt Repeat, QMK settings,
//! security/unlock, and the safe switch-matrix tester.

use crate::experimental::via;
use crate::vial_features::{self, AltRepeatEntry, MacroBank};
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
    #[allow(dead_code)]
    pub refresh: Rc<dyn Fn()>,
}

pub fn build_page(status: StatusFn) -> AdvancedVialPage {
    let boards: Rc<RefCell<Vec<VialBoardSnapshot>>> = Rc::new(RefCell::new(Vec::new()));
    let selected_index = Rc::new(Cell::new(0usize));
    let device_dd = DropDown::from_strings(&["No Vial keyboard"]);
    device_dd.set_hexpand(true);
    let refresh_btn = Button::with_label("Refresh keyboards");
    refresh_btn.add_css_class("suggested-action");

    let selected_board: SelectedBoardFn = {
        let boards = Rc::clone(&boards);
        let selected_index = Rc::clone(&selected_index);
        Rc::new(move || boards.borrow().get(selected_index.get()).cloned())
    };

    let notebook = Notebook::new();
    notebook.set_scrollable(true);
    notebook.set_hexpand(true);
    notebook.set_vexpand(true);
    notebook.append_page(
        &macros_tab(Rc::clone(&selected_board), Rc::clone(&status)),
        Some(&Label::new(Some("Macros"))),
    );
    notebook.append_page(
        &encoders_tab(Rc::clone(&selected_board), Rc::clone(&status)),
        Some(&Label::new(Some("Encoders"))),
    );
    notebook.append_page(
        &alt_repeat_tab(Rc::clone(&selected_board), Rc::clone(&status)),
        Some(&Label::new(Some("Alt Repeat"))),
    );
    notebook.append_page(
        &settings_tab(Rc::clone(&selected_board), Rc::clone(&status)),
        Some(&Label::new(Some("QMK Settings"))),
    );
    notebook.append_page(
        &security_tab(Rc::clone(&selected_board), Rc::clone(&status)),
        Some(&Label::new(Some("Security"))),
    );
    notebook.append_page(
        &matrix_tab(Rc::clone(&selected_board), Rc::clone(&status)),
        Some(&Label::new(Some("Matrix Test"))),
    );

    let header = GtkBox::new(Orientation::Horizontal, 8);
    header.append(&refresh_btn);
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
        let selected_index = Rc::clone(&selected_index);
        let device_dd = device_dd.clone();
        let status = Rc::clone(&status);
        Rc::new(move || match vial_sync::discover() {
            Ok(found) => {
                let labels: Vec<String> = if found.boards.is_empty() {
                    vec!["No Vial keyboard".into()]
                } else {
                    found.boards.iter().map(VialBoardSnapshot::display_name).collect()
                };
                set_dropdown(&device_dd, &labels);
                selected_index.set(0);
                let board_count = found.boards.len();
                let diagnostics = found.diagnostics.len();
                *boards.borrow_mut() = found.boards;
                status(format!(
                    "Advanced Vial: {board_count} board(s), {diagnostics} diagnostic(s)"
                ));
            }
            Err(err) => status(format!("Vial discovery failed: {err}")),
        })
    };
    refresh_btn.connect_clicked({
        let refresh = Rc::clone(&refresh);
        move |_| refresh()
    });
    device_dd.connect_selected_notify({
        let selected_index = Rc::clone(&selected_index);
        let device_dd = device_dd.clone();
        move |_| selected_index.set(device_dd.selected() as usize)
    });

    refresh();
    AdvancedVialPage { page, refresh }
}

fn macros_tab(selected: SelectedBoardFn, status: StatusFn) -> GtkBox {
    let bank: Rc<RefCell<Option<MacroBank>>> = Rc::new(RefCell::new(None));
    let slot = DropDown::from_strings(&["Macro 0"]);
    let text = TextView::new();
    text.set_monospace(true);
    text.set_wrap_mode(gtk4::WrapMode::WordChar);
    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .min_content_height(330)
        .vexpand(true)
        .child(&text)
        .build();
    let read = Button::with_label("Read macros");
    let write = Button::with_label("Apply slot + write bank");
    write.add_css_class("suggested-action");
    let reset = Button::with_label("Reset macros");
    reset.add_css_class("destructive-action");

    let render: Rc<dyn Fn()> = {
        let bank = Rc::clone(&bank);
        let slot = slot.clone();
        let text = text.clone();
        let status = Rc::clone(&status);
        Rc::new(move || {
            let guard = bank.borrow();
            let Some(bank) = guard.as_ref() else { return; };
            let Some(actions) = bank.macros.get(slot.selected() as usize) else { return; };
            match vial_features::macro_to_json(actions) {
                Ok(json) => text.buffer().set_text(&json),
                Err(err) => status(format!("Macro render failed: {err}")),
            }
        })
    };

    read.connect_clicked({
        let selected = Rc::clone(&selected);
        let bank = Rc::clone(&bank);
        let slot = slot.clone();
        let render = Rc::clone(&render);
        let status = Rc::clone(&status);
        move |_| {
            let Some(board) = selected() else { status("Select a Vial keyboard first".into()); return; };
            match vial_features::read_macro_bank(board.device.vendor_id, board.device.product_id) {
                Ok(value) => {
                    let labels = (0..value.count).map(|i| format!("Macro {i}")).collect::<Vec<_>>();
                    set_dropdown(&slot, &labels);
                    status(format!(
                        "Read {} macro slot(s), {} bytes, Vial protocol {}",
                        value.count, value.buffer_size, value.vial_protocol
                    ));
                    *bank.borrow_mut() = Some(value);
                    render();
                }
                Err(err) => status(format!("Read macros failed: {err}")),
            }
        }
    });
    slot.connect_selected_notify({ let render = Rc::clone(&render); move |_| render() });

    write.connect_clicked({
        let selected = Rc::clone(&selected);
        let bank = Rc::clone(&bank);
        let slot = slot.clone();
        let text = text.clone();
        let status = Rc::clone(&status);
        move |_| {
            let Some(board) = selected() else { status("Select a Vial keyboard first".into()); return; };
            let buffer = text.buffer();
            let json = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
            let actions = match vial_features::macro_from_json(json.as_str()) {
                Ok(value) => value,
                Err(err) => { status(err); return; }
            };
            let mut guard = bank.borrow_mut();
            let Some(bank) = guard.as_mut() else { status("Read macros before editing".into()); return; };
            let index = slot.selected() as usize;
            let Some(target) = bank.macros.get_mut(index) else { status("Invalid macro slot".into()); return; };
            *target = actions;
            match vial_features::write_macro_bank(board.device.vendor_id, board.device.product_id, bank) {
                Ok(()) => status(format!("Macro {index} written with interruption-safe commit")),
                Err(err) => status(format!("Macro write failed: {err}")),
            }
        }
    });
    reset.connect_clicked({
        let selected = Rc::clone(&selected);
        let bank = Rc::clone(&bank);
        let text = text.clone();
        let status = Rc::clone(&status);
        move |_| {
            let Some(board) = selected() else { status("Select a Vial keyboard first".into()); return; };
            match vial_native::reset_macros(board.device.vendor_id, board.device.product_id) {
                Ok(()) => { *bank.borrow_mut() = None; text.buffer().set_text(""); status("Macro table reset requested".into()); }
                Err(err) => status(format!("Macro reset failed: {err}")),
            }
        }
    });

    let page = tab_shell("Advanced macros", "JSON actions preserve Text/Tap/Down/Up/Delay and unknown Raw bytes. Writes use QMK's invalid-buffer sentinel until the complete bank is committed.");
    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.append(&read); row.append(&slot); row.append(&write); row.append(&reset);
    page.append(&row); page.append(&scroll); page
}

fn encoders_tab(selected: SelectedBoardFn, status: StatusFn) -> GtkBox {
    let layer = DropDown::from_strings(&["Layer 0"]);
    let encoder = DropDown::from_strings(&["Encoder 0"]);
    let ccw = hex_entry("CCW keycode");
    let cw = hex_entry("CW keycode");
    let read = Button::with_label("Read encoder");
    let write = Button::with_label("Apply encoder");
    write.add_css_class("suggested-action");

    read.connect_clicked({
        let selected = Rc::clone(&selected);
        let layer = layer.clone(); let encoder = encoder.clone(); let ccw = ccw.clone(); let cw = cw.clone();
        let status = Rc::clone(&status);
        move |_| {
            let Some(board) = selected() else { status("Select a Vial keyboard first".into()); return; };
            let layers = (0..board.layer_count().max(1)).map(|i| format!("Layer {i}")).collect::<Vec<_>>();
            set_dropdown(&layer, &layers);
            let count = match vial_features::encoder_count(board.device.vendor_id, board.device.product_id) {
                Ok(value) => value,
                Err(err) => { status(format!("Encoder discovery failed: {err}")); return; }
            };
            let labels = (0..count).map(|i| format!("Encoder {i}")).collect::<Vec<_>>();
            if labels.is_empty() { status("Firmware definition exposes no encoders".into()); return; }
            set_dropdown(&encoder, &labels);
            match vial_features::read_vial_encoder(board.device.vendor_id, board.device.product_id, layer.selected() as u8, encoder.selected() as u8) {
                Ok(pair) => { ccw.set_text(&format!("{:04X}", pair.counter_clockwise)); cw.set_text(&format!("{:04X}", pair.clockwise)); status(format!("Read {count} encoder(s)")); }
                Err(err) => status(format!("Encoder read failed: {err}")),
            }
        }
    });
    write.connect_clicked({
        let selected = Rc::clone(&selected);
        let layer = layer.clone(); let encoder = encoder.clone(); let ccw = ccw.clone(); let cw = cw.clone();
        let status = Rc::clone(&status);
        move |_| {
            let Some(board) = selected() else { status("Select a Vial keyboard first".into()); return; };
            let Some(ccw_code) = parse_hex_u16(&ccw.text()) else { status("Invalid CCW keycode".into()); return; };
            let Some(cw_code) = parse_hex_u16(&cw.text()) else { status("Invalid CW keycode".into()); return; };
            let layer_index = layer.selected() as u8; let encoder_index = encoder.selected() as u8;
            let result = vial_features::write_vial_encoder(board.device.vendor_id, board.device.product_id, layer_index, encoder_index, false, ccw_code)
                .and_then(|_| vial_features::write_vial_encoder(board.device.vendor_id, board.device.product_id, layer_index, encoder_index, true, cw_code));
            match result { Ok(()) => status(format!("Updated encoder {encoder_index} on layer {layer_index}")), Err(err) => status(format!("Encoder write failed: {err}")) }
        }
    });

    let page = tab_shell("Encoders", "Uses Vial's native encoder pair commands from the onboard definition instead of pretending encoders are matrix switches.");
    let row = GtkBox::new(Orientation::Horizontal, 8); row.append(&read); row.append(&layer); row.append(&encoder); page.append(&row);
    page.append(&field("Counter-clockwise", &ccw)); page.append(&field("Clockwise", &cw)); page.append(&write); page
}

fn alt_repeat_tab(selected: SelectedBoardFn, status: StatusFn) -> GtkBox {
    let entries: Rc<RefCell<Vec<AltRepeatEntry>>> = Rc::new(RefCell::new(Vec::new()));
    let slot = DropDown::from_strings(&["Slot 0"]);
    let key = hex_entry("primary keycode"); let alt = hex_entry("alternate keycode"); let mods = hex_entry("allowed mods byte");
    let enabled = CheckButton::with_label("Enabled");
    let default_alt = CheckButton::with_label("Default to alternate");
    let bidirectional = CheckButton::with_label("Bidirectional");
    let ignore_handedness = CheckButton::with_label("Ignore modifier handedness");
    let read = Button::with_label("Read Alt Repeat"); let write = Button::with_label("Apply slot"); write.add_css_class("suggested-action");

    let render: Rc<dyn Fn()> = {
        let entries = Rc::clone(&entries); let slot = slot.clone(); let key = key.clone(); let alt = alt.clone(); let mods = mods.clone();
        let enabled = enabled.clone(); let default_alt = default_alt.clone(); let bidirectional = bidirectional.clone(); let ignore_handedness = ignore_handedness.clone();
        Rc::new(move || {
            let guard = entries.borrow(); let Some(e) = guard.get(slot.selected() as usize) else { return; };
            key.set_text(&format!("{:04X}", e.keycode)); alt.set_text(&format!("{:04X}", e.alt_keycode)); mods.set_text(&format!("{:02X}", e.allowed_mods));
            enabled.set_active(e.enabled()); default_alt.set_active(e.default_to_alt()); bidirectional.set_active(e.bidirectional()); ignore_handedness.set_active(e.ignore_mod_handedness());
        })
    };
    read.connect_clicked({
        let selected = Rc::clone(&selected); let entries = Rc::clone(&entries); let slot = slot.clone(); let render = Rc::clone(&render); let status = Rc::clone(&status);
        move |_| {
            let Some(board) = selected() else { status("Select a Vial keyboard first".into()); return; };
            match vial_features::read_alt_repeat_entries(board.device.vendor_id, board.device.product_id) {
                Ok(found) => { let labels = (0..found.len()).map(|i| format!("Slot {i}")).collect::<Vec<_>>(); if labels.is_empty() { status("Firmware exposes no Alt Repeat slots".into()); return; } set_dropdown(&slot, &labels); let count = found.len(); *entries.borrow_mut() = found; render(); status(format!("Read {count} Alt Repeat slot(s)")); }
                Err(err) => status(format!("Alt Repeat read failed: {err}")),
            }
        }
    });
    slot.connect_selected_notify({ let render = Rc::clone(&render); move |_| render() });
    write.connect_clicked({
        let selected = Rc::clone(&selected); let entries = Rc::clone(&entries); let slot = slot.clone(); let key = key.clone(); let alt = alt.clone(); let mods = mods.clone();
        let enabled = enabled.clone(); let default_alt = default_alt.clone(); let bidirectional = bidirectional.clone(); let ignore_handedness = ignore_handedness.clone(); let status = Rc::clone(&status);
        move |_| {
            let Some(board) = selected() else { status("Select a Vial keyboard first".into()); return; };
            let Some(keycode) = parse_hex_u16(&key.text()) else { status("Invalid primary keycode".into()); return; };
            let Some(alt_keycode) = parse_hex_u16(&alt.text()) else { status("Invalid alternate keycode".into()); return; };
            let Some(allowed_mods) = parse_hex_u8(&mods.text()) else { status("Invalid modifier byte".into()); return; };
            let mut options = 0u8; if default_alt.is_active() { options |= 1; } if bidirectional.is_active() { options |= 2; } if ignore_handedness.is_active() { options |= 4; } if enabled.is_active() { options |= 8; }
            let entry = AltRepeatEntry { keycode, alt_keycode, allowed_mods, options }; let index = slot.selected() as usize;
            match vial_features::write_alt_repeat_entry(board.device.vendor_id, board.device.product_id, index as u8, &entry) {
                Ok(()) => { if let Some(target) = entries.borrow_mut().get_mut(index) { *target = entry; } status(format!("Updated Alt Repeat slot {index}")); }
                Err(err) => status(format!("Alt Repeat write failed: {err}")),
            }
        }
    });

    let page = tab_shell("Alt Repeat", "Native dynamic Alt Repeat entries: primary/alternate keycodes, allowed modifier mask and all four official behavior flags.");
    let row = GtkBox::new(Orientation::Horizontal, 8); row.append(&read); row.append(&slot); page.append(&row);
    page.append(&field("Primary keycode", &key)); page.append(&field("Alternate keycode", &alt)); page.append(&field("Allowed modifiers", &mods));
    page.append(&enabled); page.append(&default_alt); page.append(&bidirectional); page.append(&ignore_handedness); page.append(&write); page
}

fn settings_tab(selected: SelectedBoardFn, status: StatusFn) -> GtkBox {
    let ids: Rc<RefCell<Vec<u16>>> = Rc::new(RefCell::new(Vec::new()));
    let setting = DropDown::from_strings(&["No settings"]);
    let value = Entry::builder().placeholder_text("decimal or 0xHEX").hexpand(true).build();
    let metadata = info("Unknown setting");
    let read_all = Button::with_label("Read setting IDs"); let read_one = Button::with_label("Read selected");
    let write = Button::with_label("Apply selected"); write.add_css_class("suggested-action");
    let reset = Button::with_label("Reset all QMK settings"); reset.add_css_class("destructive-action");

    let refresh_metadata: Rc<dyn Fn()> = {
        let ids = Rc::clone(&ids); let setting = setting.clone(); let metadata = metadata.clone(); let write_button = write.clone();
        Rc::new(move || {
            let Some(id) = ids.borrow().get(setting.selected() as usize).copied() else { metadata.set_text("No selected setting"); write_button.set_sensitive(false); return; };
            let (name, width) = qmk_meta(id);
            metadata.set_text(&format!("{name} · ID 0x{id:04X} · {}", width.map(|v| format!("{v} byte(s), little-endian")).unwrap_or_else(|| "unknown width: read-only".into())));
            write_button.set_sensitive(width.is_some());
        })
    };
    read_all.connect_clicked({
        let selected = Rc::clone(&selected); let ids = Rc::clone(&ids); let setting = setting.clone(); let refresh_metadata = Rc::clone(&refresh_metadata); let status = Rc::clone(&status);
        move |_| {
            let Some(board) = selected() else { status("Select a Vial keyboard first".into()); return; };
            match vial_native::inspect(board.device.vendor_id, board.device.product_id) {
                Ok(snapshot) => { let labels = snapshot.qmk_setting_ids.iter().map(|id| format!("{} · 0x{id:04X}", qmk_meta(*id).0)).collect::<Vec<_>>(); set_dropdown(&setting, &labels); let count = snapshot.qmk_setting_ids.len(); *ids.borrow_mut() = snapshot.qmk_setting_ids; refresh_metadata(); status(format!("Read {count} QMK setting ID(s)")); }
                Err(err) => status(format!("QMK settings query failed: {err}")),
            }
        }
    });
    setting.connect_selected_notify({ let refresh_metadata = Rc::clone(&refresh_metadata); move |_| refresh_metadata() });
    read_one.connect_clicked({
        let selected = Rc::clone(&selected); let ids = Rc::clone(&ids); let setting = setting.clone(); let value = value.clone(); let status = Rc::clone(&status);
        move |_| {
            let Some(board) = selected() else { status("Select a Vial keyboard first".into()); return; }; let Some(id) = ids.borrow().get(setting.selected() as usize).copied() else { status("Select a setting first".into()); return; };
            match vial_features::qmk_setting_get(board.device.vendor_id, board.device.product_id, id) {
                Ok(raw) => { let width = qmk_meta(id).1.unwrap_or(raw.len().min(4)); let bytes = &raw[..width.min(raw.len())]; let number = bytes.iter().enumerate().fold(0u64, |acc, (i,b)| acc | ((*b as u64) << (8*i))); value.set_text(&number.to_string()); status(format!("Read 0x{id:04X}: {}", hex_bytes(bytes))); }
                Err(err) => status(format!("QMK setting read failed: {err}")),
            }
        }
    });
    write.connect_clicked({
        let selected = Rc::clone(&selected); let ids = Rc::clone(&ids); let setting = setting.clone(); let value = value.clone(); let status = Rc::clone(&status);
        move |_| {
            let Some(board) = selected() else { status("Select a Vial keyboard first".into()); return; }; let Some(id) = ids.borrow().get(setting.selected() as usize).copied() else { status("Select a setting first".into()); return; };
            let Some(width) = qmk_meta(id).1 else { status("Unknown setting width; refusing to guess a write format".into()); return; }; let Some(number) = parse_u64(&value.text()) else { status("Invalid setting value".into()); return; };
            if width < 8 && number >= (1u64 << (width*8)) { status(format!("Value does not fit in {width} byte(s)")); return; }
            let bytes = number.to_le_bytes(); match vial_features::qmk_setting_set(board.device.vendor_id, board.device.product_id, id, &bytes[..width]) { Ok(()) => status(format!("Updated QMK setting 0x{id:04X}")), Err(err) => status(format!("QMK setting write failed: {err}")) }
        }
    });
    reset.connect_clicked({
        let selected = Rc::clone(&selected); let status = Rc::clone(&status);
        move |_| { let Some(board) = selected() else { status("Select a Vial keyboard first".into()); return; }; match vial_features::qmk_settings_reset(board.device.vendor_id, board.device.product_id) { Ok(()) => status("QMK settings reset requested".into()), Err(err) => status(format!("QMK settings reset failed: {err}")) } }
    });

    let page = tab_shell("QMK Settings", "Settings discovered from firmware are readable. Writes are enabled only where the official Vial wire width is known; unknown IDs are never guessed.");
    let row = GtkBox::new(Orientation::Horizontal, 8); row.append(&read_all); row.append(&setting); row.append(&read_one); page.append(&row); page.append(&metadata); page.append(&field("Value", &value));
    let actions = GtkBox::new(Orientation::Horizontal, 8); actions.append(&write); actions.append(&reset); page.append(&actions); page
}

fn security_tab(selected: SelectedBoardFn, status: StatusFn) -> GtkBox {
    let state = Label::builder().label("Security state not read yet").halign(Align::Start).xalign(0.0).wrap(true).build();
    let keys = info(""); let progress = ProgressBar::new(); progress.set_show_text(true);
    let read = Button::with_label("Read lock state"); let unlock = Button::with_label("Begin unlock"); unlock.add_css_class("suggested-action"); let lock = Button::with_label("Lock now");

    let refresh: Rc<dyn Fn()> = {
        let selected = Rc::clone(&selected); let state = state.clone(); let keys = keys.clone(); let status = Rc::clone(&status);
        Rc::new(move || {
            let Some(board) = selected() else { status("Select a Vial keyboard first".into()); return; };
            match vial_features::read_unlock_status(board.device.vendor_id, board.device.product_id) {
                Ok(value) => { state.set_text(if value.unlocked { "Keyboard is UNLOCKED" } else if value.in_progress { "Unlock is in progress" } else { "Keyboard is locked" }); let rendered = if value.required_keys.is_empty() { "Firmware reports no dedicated unlock key chord".to_string() } else { format!("Hold matrix keys: {}", value.required_keys.iter().map(|(r,c)| format!("({r},{c})")).collect::<Vec<_>>().join(", ")) }; keys.set_text(&rendered); status("Vial security state refreshed".into()); }
                Err(err) => status(format!("Security read failed: {err}")),
            }
        })
    };
    read.connect_clicked({ let refresh = Rc::clone(&refresh); move |_| refresh() });
    unlock.connect_clicked({
        let selected = Rc::clone(&selected); let progress = progress.clone(); let state = state.clone(); let status = Rc::clone(&status); let refresh = Rc::clone(&refresh);
        move |_| {
            let Some(board) = selected() else { status("Select a Vial keyboard first".into()); return; }; let vid = board.device.vendor_id; let pid = board.device.product_id;
            match vial_features::unlock_start(vid, pid) {
                Ok(()) => { state.set_text("Unlock started — hold the firmware-reported physical keys"); progress.set_fraction(0.0); progress.set_text(Some("Hold unlock keys")); let progress = progress.clone(); let status = Rc::clone(&status); let refresh = Rc::clone(&refresh); let max_counter = Rc::new(Cell::new(1u8)); glib::timeout_add_local(Duration::from_millis(200), move || match vial_features::unlock_poll(vid, pid) { Ok(poll) => { let max_seen = max_counter.get().max(poll.counter).max(1); max_counter.set(max_seen); progress.set_fraction((1.0 - poll.counter as f64/max_seen as f64).clamp(0.0,1.0)); if poll.unlocked { progress.set_text(Some("Unlocked")); status("Vial keyboard unlocked".into()); refresh(); glib::ControlFlow::Break } else { progress.set_text(Some("Keep holding…")); glib::ControlFlow::Continue } }, Err(err) => { status(format!("Unlock poll failed: {err}")); glib::ControlFlow::Break } }); }
                Err(err) => status(format!("Unlock start failed: {err}")),
            }
        }
    });
    lock.connect_clicked({ let selected = Rc::clone(&selected); let status = Rc::clone(&status); let refresh = Rc::clone(&refresh); move |_| { let Some(board) = selected() else { status("Select a Vial keyboard first".into()); return; }; match vial_features::lock(board.device.vendor_id, board.device.product_id) { Ok(()) => { status("Vial keyboard locked".into()); refresh(); }, Err(err) => status(format!("Lock failed: {err}")) } } });

    let page = tab_shell("Vial security", "Unlock protected firmware operations only on a computer you trust. Replugging or Lock now exits unlocked mode.");
    let row = GtkBox::new(Orientation::Horizontal, 8); row.append(&read); row.append(&unlock); row.append(&lock); page.append(&row); page.append(&state); page.append(&keys); page.append(&progress); page
}

fn matrix_tab(selected: SelectedBoardFn, status: StatusFn) -> GtkBox {
    let running = Rc::new(Cell::new(false));
    let keys: Rc<RefCell<Vec<(u8,u8,Button)>>> = Rc::new(RefCell::new(Vec::new()));
    let fixed = Fixed::new(); let frame = Frame::builder().child(&fixed).hexpand(true).build();
    let scroll = ScrolledWindow::builder().hscrollbar_policy(PolicyType::Automatic).vscrollbar_policy(PolicyType::Automatic).min_content_height(330).vexpand(true).child(&frame).build();
    let start = Button::with_label("Start matrix test"); start.add_css_class("suggested-action"); let stop = Button::with_label("Stop"); let clear = Button::with_label("Clear latched hits");

    start.connect_clicked({
        let selected = Rc::clone(&selected); let running = Rc::clone(&running); let keys = Rc::clone(&keys); let fixed = fixed.clone(); let status = Rc::clone(&status);
        move |_| {
            let Some(board) = selected() else { status("Select a Vial keyboard first".into()); return; };
            match via::detect_vial(board.device.vendor_id, board.device.product_id) { Ok(Some(info)) if info.protocol_version < vial_features::VIAL_PROTOCOL_MATRIX_TESTER => { status(format!("Matrix tester requires Vial protocol >= {}, board reports {}", vial_features::VIAL_PROTOCOL_MATRIX_TESTER, info.protocol_version)); return; }, Err(err) => { status(format!("Vial protocol probe failed: {err}")); return; }, _ => {} }
            match vial_features::read_unlock_status(board.device.vendor_id, board.device.product_id) { Ok(lock) if !lock.unlocked => { status("Unlock the keyboard in Security before matrix testing".into()); return; }, Err(err) => { status(format!("Cannot verify unlock state: {err}")); return; }, _ => {} }
            while let Some(child) = fixed.first_child() { fixed.remove(&child); } keys.borrow_mut().clear();
            let width = (board.definition.layout.width() as f64*(UNIT_PX+GAP_PX)+GAP_PX).ceil() as i32; let height = (board.definition.layout.height() as f64*(UNIT_PX+GAP_PX)+GAP_PX).ceil() as i32; fixed.set_size_request(width.max(220),height.max(100));
            for key in board.physical_keys() { let button = Button::builder().label(&format!("{},{}",key.row,key.col)).css_classes(["hyprbinds-via-key"]).sensitive(false).build(); let w=(key.w as f64*UNIT_PX+(key.w as f64-1.0).max(0.0)*GAP_PX).max(28.0); let h=(key.h as f64*UNIT_PX+(key.h as f64-1.0).max(0.0)*GAP_PX).max(28.0); button.set_size_request(w as i32,h as i32); fixed.put(&button,GAP_PX+key.x as f64*(UNIT_PX+GAP_PX),GAP_PX+key.y as f64*(UNIT_PX+GAP_PX)); keys.borrow_mut().push((key.row,key.col,button)); }
            running.set(true); let running_tick = Rc::clone(&running); let keys_tick = Rc::clone(&keys); let status_tick = Rc::clone(&status); let vid=board.device.vendor_id; let pid=board.device.product_id; let rows=board.definition.rows; let cols=board.definition.cols;
            glib::timeout_add_local(Duration::from_millis(30), move || { if !running_tick.get() { return glib::ControlFlow::Break; } match vial_features::matrix_poll(vid,pid,rows,cols) { Ok(snapshot) => for (row,col,button) in keys_tick.borrow().iter() { let pressed=snapshot.pressed.get(*row as usize).and_then(|r|r.get(*col as usize)).copied().unwrap_or(false); if pressed { button.add_css_class("suggested-action"); button.add_css_class("hyprbinds-vial-matrix-hit"); } else { button.remove_css_class("suggested-action"); } }, Err(err) => { status_tick(format!("Matrix poll failed: {err}")); running_tick.set(false); return glib::ControlFlow::Break; } } glib::ControlFlow::Continue }); status("Matrix tester running".into());
        }
    });
    stop.connect_clicked({ let running=Rc::clone(&running); let status=Rc::clone(&status); move |_| { running.set(false); status("Matrix tester stopped".into()); } });
    clear.connect_clicked({ let keys=Rc::clone(&keys); move |_| for (_,_,button) in keys.borrow().iter() { button.remove_css_class("suggested-action"); button.remove_css_class("hyprbinds-vial-matrix-hit"); } });

    let page=tab_shell("Safe matrix tester","Polls VIA switch-matrix state only after Vial's security state reports unlocked; hits remain latched until cleared."); let row=GtkBox::new(Orientation::Horizontal,8); row.append(&start); row.append(&stop); row.append(&clear); page.append(&row); page.append(&scroll); page
}

fn tab_shell(title: &str, description: &str) -> GtkBox {
    let page=GtkBox::builder().orientation(Orientation::Vertical).spacing(12).margin_top(14).margin_bottom(14).margin_start(14).margin_end(14).build();
    page.append(&Label::builder().label(title).halign(Align::Start).css_classes(["title-3"]).build()); page.append(&info(description)); page
}
fn info(text: &str) -> Label { Label::builder().label(text).halign(Align::Start).xalign(0.0).wrap(true).css_classes(["dim-label","caption"]).build() }
fn field(label: &str, widget: &impl IsA<gtk4::Widget>) -> GtkBox { let row=GtkBox::new(Orientation::Horizontal,10); row.append(&Label::builder().label(label).halign(Align::Start).width_request(160).build()); row.append(widget); row }
fn hex_entry(placeholder: &str) -> Entry { Entry::builder().placeholder_text(placeholder).width_chars(12).hexpand(true).build() }
fn set_dropdown(dropdown: &DropDown, labels: &[String]) { let fallback=["None".to_string()]; let labels=if labels.is_empty(){&fallback[..]}else{labels}; let refs=labels.iter().map(String::as_str).collect::<Vec<_>>(); dropdown.set_model(Some(&StringList::new(&refs))); dropdown.set_selected(0); }
fn parse_hex_u16(text:&str)->Option<u16>{u16::from_str_radix(text.trim().trim_start_matches("0x").trim_start_matches("0X"),16).ok()}
fn parse_hex_u8(text:&str)->Option<u8>{u8::from_str_radix(text.trim().trim_start_matches("0x").trim_start_matches("0X"),16).ok()}
fn parse_u64(text:&str)->Option<u64>{let text=text.trim(); if let Some(hex)=text.strip_prefix("0x").or_else(||text.strip_prefix("0X")){u64::from_str_radix(hex,16).ok()}else{text.parse().ok()}}
fn hex_bytes(bytes:&[u8])->String{bytes.iter().map(|b|format!("{b:02X}")).collect::<Vec<_>>().join(" ")}

/// Widths/titles from official Vial's qmk_settings.json plus viar's documented
/// newer pointing-device setting IDs. Unknown widths are intentionally read-only.
fn qmk_meta(id:u16)->(&'static str,Option<usize>){match id{
1=>("Grave Escape options",Some(1)),2=>("Combo term",Some(2)),3=>("Auto Shift options",Some(1)),4=>("Auto Shift timeout",Some(2)),5=>("One Shot tap toggle",Some(1)),6=>("One Shot timeout",Some(2)),7=>("Tapping term",Some(2)),8=>("Tap-Hold options",Some(1)),9=>("Mouse key delay",Some(2)),10=>("Mouse key interval",Some(2)),11=>("Mouse key step",Some(2)),12=>("Mouse key max speed",Some(2)),13=>("Mouse key acceleration time",Some(2)),14=>("Mouse wheel delay",Some(2)),15=>("Mouse wheel interval",Some(2)),16=>("Mouse wheel max steps",Some(2)),17=>("Mouse wheel acceleration time",Some(2)),18=>("Tap code delay",Some(2)),19=>("Tap Hold Caps delay",Some(2)),20=>("Tapping toggle",Some(1)),21=>("Magic keymap options",Some(4)),22=>("Permissive Hold",Some(1)),23=>("Hold On Other Key Press",Some(1)),24=>("Retro Tapping",Some(1)),25=>("Quick Tap term",Some(2)),26=>("Chordal Hold",Some(1)),27=>("Flow Tap",Some(2)),
0x0100=>("Pointing device DPI",Some(2)),0x0101=>("Scroll divisor",Some(1)),0x0102=>("Horizontal scroll divisor",Some(1)),0x0103=>("Invert pointing X",Some(1)),0x0104=>("Invert pointing Y",Some(1)),0x0105=>("Invert scroll",Some(1)),0x0106=>("Drag scroll",Some(1)),0x0107=>("Drag scroll divisor",Some(1)),0x0108=>("Second pointing-device DPI",Some(2)),0x0109=>("Sniping DPI",Some(2)),0x0110=>("Auto mouse enable",Some(1)),0x0111=>("Auto mouse layer",Some(1)),0x0112=>("Auto mouse timeout",Some(2)),_=>("Unknown QMK setting",None)}}
