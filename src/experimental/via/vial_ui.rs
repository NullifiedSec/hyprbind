//! Vial notebook tabs — tap dance, combos, and key overrides.

use crate::dialog;
use crate::experimental::via::{self, BoardCapabilities, Combo, CustomKeycode, KeyOverride, TapDance};
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, CheckButton, Entry, Label, ListBox, ListBoxRow, Orientation,
    PolicyType, ScrolledWindow,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Controllers returned by [`build_vial_tabs`].
pub struct VialTabs {
    pub tap_dance_page: ScrolledWindow,
    pub combo_page: ScrolledWindow,
    pub override_page: ScrolledWindow,
    /// Refresh list/editor UI from probed capabilities + EEPROM snapshots.
    pub sync: Rc<
        dyn Fn(
            &BoardCapabilities,
            &[TapDance],
            &[Combo],
            &[KeyOverride],
            &[CustomKeycode],
        ),
    >,
}

type VidPidFn = Rc<dyn Fn() -> Option<(u16, u16)>>;
type StatusFn = Rc<dyn Fn(String)>;

pub fn build_vial_tabs(get_vid_pid: VidPidFn, status: StatusFn) -> VialTabs {
    let suppress = Rc::new(Cell::new(false));

    let td = build_tap_dance_tab(Rc::clone(&get_vid_pid), Rc::clone(&status), Rc::clone(&suppress));
    let combo = build_combo_tab(Rc::clone(&get_vid_pid), Rc::clone(&status), Rc::clone(&suppress));
    let ko = build_override_tab(get_vid_pid, status, Rc::clone(&suppress));

    let sync = {
        let td_sync = Rc::clone(&td.sync);
        let combo_sync = Rc::clone(&combo.sync);
        let ko_sync = Rc::clone(&ko.sync);
        Rc::new(
            move |caps: &BoardCapabilities,
                  tap_dances: &[TapDance],
                  combos: &[Combo],
                  overrides: &[KeyOverride],
                  customs: &[CustomKeycode]| {
                td_sync(caps, tap_dances, customs);
                combo_sync(caps, combos, customs);
                ko_sync(caps, overrides, customs);
            },
        ) as Rc<
            dyn Fn(
                &BoardCapabilities,
                &[TapDance],
                &[Combo],
                &[KeyOverride],
                &[CustomKeycode],
            ),
        >
    };

    VialTabs {
        tap_dance_page: td.page,
        combo_page: combo.page,
        override_page: ko.page,
        sync,
    }
}

struct TabBuilt<F: ?Sized> {
    page: ScrolledWindow,
    sync: Rc<F>,
}

fn feature_shell(info: &Label, body: &impl IsA<gtk4::Widget>) -> ScrolledWindow {
    let col = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(4)
        .margin_end(4)
        .build();
    col.append(info);
    col.append(body);
    ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .child(&col)
        .build()
}

fn hex_entry(placeholder: &str) -> Entry {
    Entry::builder()
        .placeholder_text(placeholder)
        .width_chars(8)
        .hexpand(true)
        .build()
}

fn field_row(label: &str, widget: &impl IsA<gtk4::Widget>) -> GtkBox {
    let row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .build();
    row.append(
        &Label::builder()
            .label(label)
            .halign(Align::Start)
            .css_classes(["dim-label", "caption"])
            .width_request(110)
            .build(),
    );
    row.append(widget);
    row
}

fn parse_hex_u16(s: &str) -> Result<u16, ()> {
    let s = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    if s.is_empty() {
        return Err(());
    }
    u16::from_str_radix(s, 16).map_err(|_| ())
}

fn parse_hex_u8(s: &str) -> Result<u8, ()> {
    let s = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    if s.is_empty() {
        return Err(());
    }
    u8::from_str_radix(s, 16).map_err(|_| ())
}

fn parse_dec_u16(s: &str) -> Result<u16, ()> {
    let s = s.trim();
    if s.is_empty() {
        return Err(());
    }
    s.parse::<u16>().map_err(|_| ())
}

fn set_hex(entry: &Entry, code: u16) {
    entry.set_text(&format!("{code:04X}"));
}

fn kc_hint(code: u16, customs: &[CustomKeycode]) -> String {
    format!("0x{code:04X} · {}", via::keycode_short_label(code, customs))
}

// --- Tap Dance ----------------------------------------------------------------

fn build_tap_dance_tab(
    get_vid_pid: VidPidFn,
    status: StatusFn,
    suppress: Rc<Cell<bool>>,
) -> TabBuilt<dyn Fn(&BoardCapabilities, &[TapDance], &[CustomKeycode])> {
    let info = Label::builder()
        .label("Tap dance requires a Vial keyboard with slots enabled in firmware.")
        .halign(Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();

    let list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();
    let list_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .min_content_height(140)
        .vexpand(true)
        .child(&list)
        .build();

    let on_tap = hex_entry("e.g. 001F");
    let on_hold = hex_entry("e.g. 00E1");
    let on_double = hex_entry("e.g. 0000");
    let on_tap_hold = hex_entry("e.g. 0000");
    let term = Entry::builder()
        .placeholder_text("ms")
        .width_chars(8)
        .text("200")
        .build();
    let apply = Button::builder()
        .label("Apply tap dance")
        .css_classes(["suggested-action"])
        .sensitive(false)
        .build();

    let editor = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .build();
    editor.append(&dialog::section_label("Edit slot"));
    editor.append(&field_row("On tap", &on_tap));
    editor.append(&field_row("On hold", &on_hold));
    editor.append(&field_row("Double tap", &on_double));
    editor.append(&field_row("Tap-hold", &on_tap_hold));
    editor.append(&field_row("Term (ms)", &term));
    editor.append(&apply);

    let body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .build();
    body.append(&dialog::section_label("Slots"));
    body.append(&list_scroll);
    body.append(&editor);

    let page = feature_shell(&info, &body);
    let entries: Rc<RefCell<Vec<TapDance>>> = Rc::new(RefCell::new(Vec::new()));
    let selected: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));

    let fill_editor = {
        let on_tap = on_tap.clone();
        let on_hold = on_hold.clone();
        let on_double = on_double.clone();
        let on_tap_hold = on_tap_hold.clone();
        let term = term.clone();
        let apply = apply.clone();
        let entries = Rc::clone(&entries);
        let selected = Rc::clone(&selected);
        Rc::new(move || {
            let Some(i) = selected.get() else {
                apply.set_sensitive(false);
                return;
            };
            let ents = entries.borrow();
            let Some(e) = ents.get(i) else {
                apply.set_sensitive(false);
                return;
            };
            set_hex(&on_tap, e.on_tap);
            set_hex(&on_hold, e.on_hold);
            set_hex(&on_double, e.on_double_tap);
            set_hex(&on_tap_hold, e.on_tap_hold);
            term.set_text(&e.tapping_term.to_string());
            apply.set_sensitive(true);
        })
    };

    let sync = {
        let info = info.clone();
        let list = list.clone();
        let editor = editor.clone();
        let entries = Rc::clone(&entries);
        let selected = Rc::clone(&selected);
        let fill_editor = Rc::clone(&fill_editor);
        let suppress = Rc::clone(&suppress);
        let apply = apply.clone();
        Rc::new(
            move |caps: &BoardCapabilities, tap_dances: &[TapDance], customs: &[CustomKeycode]| {
                suppress.set(true);
                while let Some(child) = list.first_child() {
                    list.remove(&child);
                }
                *entries.borrow_mut() = tap_dances.to_vec();

                let enabled = caps.is_vial && caps.tap_dance > 0;
                editor.set_sensitive(enabled);
                list.set_sensitive(enabled);
                if !caps.is_vial {
                    info.set_text(
                        "Not a Vial board — tap dance is unavailable. Stock VIA keyboards use the Keymap tab only.",
                    );
                    selected.set(None);
                    apply.set_sensitive(false);
                    suppress.set(false);
                    return;
                }
                if caps.tap_dance == 0 {
                    info.set_text(
                        "Vial detected, but this firmware reports 0 tap-dance slots.",
                    );
                    selected.set(None);
                    apply.set_sensitive(false);
                    suppress.set(false);
                    return;
                }

                info.set_text(&format!(
                    "Vial tap dance · {} slot(s). Select a slot, edit keycodes (hex) and tapping term, then Apply.",
                    caps.tap_dance
                ));

                for (i, e) in tap_dances.iter().enumerate() {
                    let title = format!("Tap dance #{i}");
                    let body = format!(
                        "tap {} · hold {} · 2× {} · tap-hold {} · {} ms",
                        kc_hint(e.on_tap, customs),
                        kc_hint(e.on_hold, customs),
                        kc_hint(e.on_double_tap, customs),
                        kc_hint(e.on_tap_hold, customs),
                        e.tapping_term
                    );
                    let col = dialog::list_row_column();
                    col.append(&dialog::row_title(&title));
                    col.append(&dialog::row_body(&body));
                    let row = ListBoxRow::builder().child(&col).build();
                    row.set_widget_name(&format!("{i}"));
                    list.append(&row);
                }

                let keep = selected
                    .get()
                    .filter(|&i| i < tap_dances.len())
                    .or_else(|| (!tap_dances.is_empty()).then_some(0));
                selected.set(keep);
                if let Some(i) = keep {
                    if let Some(row) = list.row_at_index(i as i32) {
                        list.select_row(Some(&row));
                    }
                }
                fill_editor();
                suppress.set(false);
            },
        )
            as Rc<dyn Fn(&BoardCapabilities, &[TapDance], &[CustomKeycode])>
    };

    list.connect_row_selected({
        let selected = Rc::clone(&selected);
        let fill_editor = Rc::clone(&fill_editor);
        let suppress = Rc::clone(&suppress);
        move |_, row| {
            if suppress.get() {
                return;
            }
            let idx = row.and_then(|r| r.widget_name().parse::<usize>().ok());
            selected.set(idx);
            fill_editor();
        }
    });

    apply.connect_clicked({
        let get_vid_pid = Rc::clone(&get_vid_pid);
        let status = Rc::clone(&status);
        let selected = Rc::clone(&selected);
        let entries = Rc::clone(&entries);
        let on_tap = on_tap.clone();
        let on_hold = on_hold.clone();
        let on_double = on_double.clone();
        let on_tap_hold = on_tap_hold.clone();
        let term = term.clone();
        let list = list.clone();
        move |_| {
            let Some((vid, pid)) = get_vid_pid() else {
                status("No device selected".into());
                return;
            };
            let Some(idx) = selected.get() else {
                status("Select a tap-dance slot".into());
                return;
            };
            let entry = TapDance {
                on_tap: match parse_hex_u16(&on_tap.text()) {
                    Ok(v) => v,
                    Err(()) => {
                        status("Invalid on-tap keycode (hex)".into());
                        return;
                    }
                },
                on_hold: match parse_hex_u16(&on_hold.text()) {
                    Ok(v) => v,
                    Err(()) => {
                        status("Invalid on-hold keycode (hex)".into());
                        return;
                    }
                },
                on_double_tap: match parse_hex_u16(&on_double.text()) {
                    Ok(v) => v,
                    Err(()) => {
                        status("Invalid double-tap keycode (hex)".into());
                        return;
                    }
                },
                on_tap_hold: match parse_hex_u16(&on_tap_hold.text()) {
                    Ok(v) => v,
                    Err(()) => {
                        status("Invalid tap-hold keycode (hex)".into());
                        return;
                    }
                },
                tapping_term: match parse_dec_u16(&term.text()) {
                    Ok(v) => v,
                    Err(()) => {
                        status("Invalid tapping term (milliseconds)".into());
                        return;
                    }
                },
            };
            match via::write_tap_dance(vid, pid, idx as u8, &entry) {
                Ok(()) => {
                    if let Some(slot) = entries.borrow_mut().get_mut(idx) {
                        *slot = entry.clone();
                    }
                    if let Some(row) = list.row_at_index(idx as i32) {
                        if let Some(child) = row.child() {
                            if let Ok(col) = child.downcast::<GtkBox>() {
                                while let Some(c) = col.first_child() {
                                    col.remove(&c);
                                }
                                col.append(&dialog::row_title(&format!("Tap dance #{idx}")));
                                col.append(&dialog::row_body(&format!(
                                    "tap 0x{:04X} · hold 0x{:04X} · 2× 0x{:04X} · tap-hold 0x{:04X} · {} ms",
                                    entry.on_tap,
                                    entry.on_hold,
                                    entry.on_double_tap,
                                    entry.on_tap_hold,
                                    entry.tapping_term
                                )));
                            }
                        }
                    }
                    status(format!("Wrote tap dance #{idx}"));
                }
                Err(e) => status(format!("Tap dance write failed: {e}")),
            }
        }
    });

    TabBuilt { page, sync }
}

// --- Combos -------------------------------------------------------------------

fn build_combo_tab(
    get_vid_pid: VidPidFn,
    status: StatusFn,
    suppress: Rc<Cell<bool>>,
) -> TabBuilt<dyn Fn(&BoardCapabilities, &[Combo], &[CustomKeycode])> {
    let info = Label::builder()
        .label("Combos require a Vial keyboard with combo slots enabled in firmware.")
        .halign(Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();

    let list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();
    let list_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .min_content_height(140)
        .vexpand(true)
        .child(&list)
        .build();

    let in0 = hex_entry("input 0");
    let in1 = hex_entry("input 1");
    let in2 = hex_entry("input 2");
    let in3 = hex_entry("input 3");
    let output = hex_entry("output");
    let apply = Button::builder()
        .label("Apply combo")
        .css_classes(["suggested-action"])
        .sensitive(false)
        .build();

    let editor = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .build();
    editor.append(&dialog::section_label("Edit slot"));
    editor.append(&field_row("Input 1", &in0));
    editor.append(&field_row("Input 2", &in1));
    editor.append(&field_row("Input 3", &in2));
    editor.append(&field_row("Input 4", &in3));
    editor.append(&field_row("Output", &output));
    editor.append(&apply);

    let body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .build();
    body.append(&dialog::section_label("Slots"));
    body.append(&list_scroll);
    body.append(&editor);

    let page = feature_shell(&info, &body);
    let entries: Rc<RefCell<Vec<Combo>>> = Rc::new(RefCell::new(Vec::new()));
    let selected: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));

    let fill_editor = {
        let in0 = in0.clone();
        let in1 = in1.clone();
        let in2 = in2.clone();
        let in3 = in3.clone();
        let output = output.clone();
        let apply = apply.clone();
        let entries = Rc::clone(&entries);
        let selected = Rc::clone(&selected);
        Rc::new(move || {
            let Some(i) = selected.get() else {
                apply.set_sensitive(false);
                return;
            };
            let ents = entries.borrow();
            let Some(e) = ents.get(i) else {
                apply.set_sensitive(false);
                return;
            };
            set_hex(&in0, e.input[0]);
            set_hex(&in1, e.input[1]);
            set_hex(&in2, e.input[2]);
            set_hex(&in3, e.input[3]);
            set_hex(&output, e.output);
            apply.set_sensitive(true);
        })
    };

    let sync = {
        let info = info.clone();
        let list = list.clone();
        let editor = editor.clone();
        let entries = Rc::clone(&entries);
        let selected = Rc::clone(&selected);
        let fill_editor = Rc::clone(&fill_editor);
        let suppress = Rc::clone(&suppress);
        let apply = apply.clone();
        Rc::new(
            move |caps: &BoardCapabilities, combos: &[Combo], customs: &[CustomKeycode]| {
                suppress.set(true);
                while let Some(child) = list.first_child() {
                    list.remove(&child);
                }
                *entries.borrow_mut() = combos.to_vec();

                let enabled = caps.is_vial && caps.combo > 0;
                editor.set_sensitive(enabled);
                list.set_sensitive(enabled);
                if !caps.is_vial {
                    info.set_text(
                        "Not a Vial board — combos are unavailable. Stock VIA keyboards use the Keymap tab only.",
                    );
                    selected.set(None);
                    apply.set_sensitive(false);
                    suppress.set(false);
                    return;
                }
                if caps.combo == 0 {
                    info.set_text("Vial detected, but this firmware reports 0 combo slots.");
                    selected.set(None);
                    apply.set_sensitive(false);
                    suppress.set(false);
                    return;
                }

                info.set_text(&format!(
                    "Vial combos · {} slot(s). Up to 4 input keycodes → one output. Use 0000 for unused inputs.",
                    caps.combo
                ));

                for (i, e) in combos.iter().enumerate() {
                    let title = format!("Combo #{i}");
                    let inputs: Vec<String> = e
                        .input
                        .iter()
                        .filter(|c| **c != 0)
                        .map(|c| kc_hint(*c, customs))
                        .collect();
                    let body = if inputs.is_empty() {
                        format!("(empty) → {}", kc_hint(e.output, customs))
                    } else {
                        format!("{} → {}", inputs.join(" + "), kc_hint(e.output, customs))
                    };
                    let col = dialog::list_row_column();
                    col.append(&dialog::row_title(&title));
                    col.append(&dialog::row_body(&body));
                    let row = ListBoxRow::builder().child(&col).build();
                    row.set_widget_name(&format!("{i}"));
                    list.append(&row);
                }

                let keep = selected
                    .get()
                    .filter(|&i| i < combos.len())
                    .or_else(|| (!combos.is_empty()).then_some(0));
                selected.set(keep);
                if let Some(i) = keep {
                    if let Some(row) = list.row_at_index(i as i32) {
                        list.select_row(Some(&row));
                    }
                }
                fill_editor();
                suppress.set(false);
            },
        ) as Rc<dyn Fn(&BoardCapabilities, &[Combo], &[CustomKeycode])>
    };

    list.connect_row_selected({
        let selected = Rc::clone(&selected);
        let fill_editor = Rc::clone(&fill_editor);
        let suppress = Rc::clone(&suppress);
        move |_, row| {
            if suppress.get() {
                return;
            }
            let idx = row.and_then(|r| r.widget_name().parse::<usize>().ok());
            selected.set(idx);
            fill_editor();
        }
    });

    apply.connect_clicked({
        let get_vid_pid = Rc::clone(&get_vid_pid);
        let status = Rc::clone(&status);
        let selected = Rc::clone(&selected);
        let entries = Rc::clone(&entries);
        let in0 = in0.clone();
        let in1 = in1.clone();
        let in2 = in2.clone();
        let in3 = in3.clone();
        let output = output.clone();
        let list = list.clone();
        move |_| {
            let Some((vid, pid)) = get_vid_pid() else {
                status("No device selected".into());
                return;
            };
            let Some(idx) = selected.get() else {
                status("Select a combo slot".into());
                return;
            };
            let parse_in = |e: &Entry, label: &str| -> Option<u16> {
                match parse_hex_u16(&e.text()) {
                    Ok(v) => Some(v),
                    Err(()) => {
                        status(format!("Invalid {label} keycode (hex)"));
                        None
                    }
                }
            };
            let (Some(a), Some(b), Some(c), Some(d), Some(out)) = (
                parse_in(&in0, "input 1"),
                parse_in(&in1, "input 2"),
                parse_in(&in2, "input 3"),
                parse_in(&in3, "input 4"),
                parse_in(&output, "output"),
            ) else {
                return;
            };
            let entry = Combo {
                input: [a, b, c, d],
                output: out,
            };
            match via::write_combo(vid, pid, idx as u8, &entry) {
                Ok(()) => {
                    if let Some(slot) = entries.borrow_mut().get_mut(idx) {
                        *slot = entry.clone();
                    }
                    if let Some(row) = list.row_at_index(idx as i32) {
                        if let Some(child) = row.child() {
                            if let Ok(col) = child.downcast::<GtkBox>() {
                                while let Some(ch) = col.first_child() {
                                    col.remove(&ch);
                                }
                                let inputs: Vec<String> = entry
                                    .input
                                    .iter()
                                    .filter(|c| **c != 0)
                                    .map(|c| format!("0x{c:04X}"))
                                    .collect();
                                let summary = if inputs.is_empty() {
                                    format!("(empty) → 0x{:04X}", entry.output)
                                } else {
                                    format!("{} → 0x{:04X}", inputs.join(" + "), entry.output)
                                };
                                col.append(&dialog::row_title(&format!("Combo #{idx}")));
                                col.append(&dialog::row_body(&summary));
                            }
                        }
                    }
                    status(format!("Wrote combo #{idx}"));
                }
                Err(e) => status(format!("Combo write failed: {e}")),
            }
        }
    });

    TabBuilt { page, sync }
}

// --- Key overrides ------------------------------------------------------------

const KO_ENABLED_BIT: u8 = 0x80;

fn build_override_tab(
    get_vid_pid: VidPidFn,
    status: StatusFn,
    suppress: Rc<Cell<bool>>,
) -> TabBuilt<dyn Fn(&BoardCapabilities, &[KeyOverride], &[CustomKeycode])> {
    let info = Label::builder()
        .label("Key overrides require a Vial keyboard with override slots enabled in firmware.")
        .halign(Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();

    let list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();
    let list_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .min_content_height(140)
        .vexpand(true)
        .child(&list)
        .build();

    let trigger = hex_entry("trigger");
    let replacement = hex_entry("replacement");
    let layers = hex_entry("e.g. FFFF");
    let trigger_mods = hex_entry("mods");
    let neg_mods = hex_entry("neg mask");
    let suppressed = hex_entry("suppress");
    let enabled = CheckButton::builder()
        .label("Enabled")
        .active(true)
        .build();
    let apply = Button::builder()
        .label("Apply override")
        .css_classes(["suggested-action"])
        .sensitive(false)
        .build();

    let editor = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .build();
    editor.append(&dialog::section_label("Edit slot"));
    editor.append(&field_row("Trigger", &trigger));
    editor.append(&field_row("Replacement", &replacement));
    editor.append(&field_row("Layers", &layers));
    editor.append(&field_row("Trigger mods", &trigger_mods));
    editor.append(&field_row("Neg mod mask", &neg_mods));
    editor.append(&field_row("Suppressed", &suppressed));
    editor.append(&enabled);
    editor.append(&apply);

    let body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .build();
    body.append(&dialog::section_label("Slots"));
    body.append(&list_scroll);
    body.append(&editor);

    let page = feature_shell(&info, &body);
    let entries: Rc<RefCell<Vec<KeyOverride>>> = Rc::new(RefCell::new(Vec::new()));
    let selected: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));
    // Preserve non-enabled option bits across edits.
    let options_base: Rc<Cell<u8>> = Rc::new(Cell::new(0));

    let fill_editor = {
        let trigger = trigger.clone();
        let replacement = replacement.clone();
        let layers = layers.clone();
        let trigger_mods = trigger_mods.clone();
        let neg_mods = neg_mods.clone();
        let suppressed = suppressed.clone();
        let enabled = enabled.clone();
        let apply = apply.clone();
        let entries = Rc::clone(&entries);
        let selected = Rc::clone(&selected);
        let options_base = Rc::clone(&options_base);
        Rc::new(move || {
            let Some(i) = selected.get() else {
                apply.set_sensitive(false);
                return;
            };
            let ents = entries.borrow();
            let Some(e) = ents.get(i) else {
                apply.set_sensitive(false);
                return;
            };
            set_hex(&trigger, e.trigger);
            set_hex(&replacement, e.replacement);
            set_hex(&layers, e.layers);
            trigger_mods.set_text(&format!("{:02X}", e.trigger_mods));
            neg_mods.set_text(&format!("{:02X}", e.negative_mod_mask));
            suppressed.set_text(&format!("{:02X}", e.suppressed_mods));
            options_base.set(e.options & !KO_ENABLED_BIT);
            enabled.set_active(e.options & KO_ENABLED_BIT != 0);
            apply.set_sensitive(true);
        })
    };

    let sync = {
        let info = info.clone();
        let list = list.clone();
        let editor = editor.clone();
        let entries = Rc::clone(&entries);
        let selected = Rc::clone(&selected);
        let fill_editor = Rc::clone(&fill_editor);
        let suppress = Rc::clone(&suppress);
        let apply = apply.clone();
        Rc::new(
            move |caps: &BoardCapabilities,
                  overrides: &[KeyOverride],
                  customs: &[CustomKeycode]| {
                suppress.set(true);
                while let Some(child) = list.first_child() {
                    list.remove(&child);
                }
                *entries.borrow_mut() = overrides.to_vec();

                let enabled_tab = caps.is_vial && caps.key_override > 0;
                editor.set_sensitive(enabled_tab);
                list.set_sensitive(enabled_tab);
                if !caps.is_vial {
                    info.set_text(
                        "Not a Vial board — key overrides are unavailable. Stock VIA keyboards use the Keymap tab only.",
                    );
                    selected.set(None);
                    apply.set_sensitive(false);
                    suppress.set(false);
                    return;
                }
                if caps.key_override == 0 {
                    info.set_text(
                        "Vial detected, but this firmware reports 0 key-override slots.",
                    );
                    selected.set(None);
                    apply.set_sensitive(false);
                    suppress.set(false);
                    return;
                }

                info.set_text(&format!(
                    "Vial key overrides · {} slot(s). Layers/mods are hex bitmasks; bit 7 of options = enabled.",
                    caps.key_override
                ));

                for (i, e) in overrides.iter().enumerate() {
                    let on = if e.options & KO_ENABLED_BIT != 0 {
                        "on"
                    } else {
                        "off"
                    };
                    let title = format!("Override #{i} ({on})");
                    let body = format!(
                        "{} → {} · layers 0x{:04X} · mods 0x{:02X}",
                        kc_hint(e.trigger, customs),
                        kc_hint(e.replacement, customs),
                        e.layers,
                        e.trigger_mods
                    );
                    let col = dialog::list_row_column();
                    col.append(&dialog::row_title(&title));
                    col.append(&dialog::row_body(&body));
                    let row = ListBoxRow::builder().child(&col).build();
                    row.set_widget_name(&format!("{i}"));
                    list.append(&row);
                }

                let keep = selected
                    .get()
                    .filter(|&i| i < overrides.len())
                    .or_else(|| (!overrides.is_empty()).then_some(0));
                selected.set(keep);
                if let Some(i) = keep {
                    if let Some(row) = list.row_at_index(i as i32) {
                        list.select_row(Some(&row));
                    }
                }
                fill_editor();
                suppress.set(false);
            },
        )
            as Rc<dyn Fn(&BoardCapabilities, &[KeyOverride], &[CustomKeycode])>
    };

    list.connect_row_selected({
        let selected = Rc::clone(&selected);
        let fill_editor = Rc::clone(&fill_editor);
        let suppress = Rc::clone(&suppress);
        move |_, row| {
            if suppress.get() {
                return;
            }
            let idx = row.and_then(|r| r.widget_name().parse::<usize>().ok());
            selected.set(idx);
            fill_editor();
        }
    });

    apply.connect_clicked({
        let get_vid_pid = Rc::clone(&get_vid_pid);
        let status = Rc::clone(&status);
        let selected = Rc::clone(&selected);
        let entries = Rc::clone(&entries);
        let options_base = Rc::clone(&options_base);
        let trigger = trigger.clone();
        let replacement = replacement.clone();
        let layers = layers.clone();
        let trigger_mods = trigger_mods.clone();
        let neg_mods = neg_mods.clone();
        let suppressed = suppressed.clone();
        let enabled = enabled.clone();
        let list = list.clone();
        move |_| {
            let Some((vid, pid)) = get_vid_pid() else {
                status("No device selected".into());
                return;
            };
            let Some(idx) = selected.get() else {
                status("Select an override slot".into());
                return;
            };
            let trigger_v = match parse_hex_u16(&trigger.text()) {
                Ok(v) => v,
                Err(()) => {
                    status("Invalid trigger keycode (hex)".into());
                    return;
                }
            };
            let replacement_v = match parse_hex_u16(&replacement.text()) {
                Ok(v) => v,
                Err(()) => {
                    status("Invalid replacement keycode (hex)".into());
                    return;
                }
            };
            let layers_v = match parse_hex_u16(&layers.text()) {
                Ok(v) => v,
                Err(()) => {
                    status("Invalid layers mask (hex)".into());
                    return;
                }
            };
            let trigger_mods_v = match parse_hex_u8(&trigger_mods.text()) {
                Ok(v) => v,
                Err(()) => {
                    status("Invalid trigger mods (hex)".into());
                    return;
                }
            };
            let neg_v = match parse_hex_u8(&neg_mods.text()) {
                Ok(v) => v,
                Err(()) => {
                    status("Invalid negative mod mask (hex)".into());
                    return;
                }
            };
            let supp_v = match parse_hex_u8(&suppressed.text()) {
                Ok(v) => v,
                Err(()) => {
                    status("Invalid suppressed mods (hex)".into());
                    return;
                }
            };
            let mut options = options_base.get();
            if enabled.is_active() {
                options |= KO_ENABLED_BIT;
            } else {
                options &= !KO_ENABLED_BIT;
            }
            let entry = KeyOverride {
                trigger: trigger_v,
                replacement: replacement_v,
                layers: layers_v,
                trigger_mods: trigger_mods_v,
                negative_mod_mask: neg_v,
                suppressed_mods: supp_v,
                options,
            };
            match via::write_key_override(vid, pid, idx as u8, &entry) {
                Ok(()) => {
                    if let Some(slot) = entries.borrow_mut().get_mut(idx) {
                        *slot = entry.clone();
                    }
                    if let Some(row) = list.row_at_index(idx as i32) {
                        if let Some(child) = row.child() {
                            if let Ok(col) = child.downcast::<GtkBox>() {
                                while let Some(ch) = col.first_child() {
                                    col.remove(&ch);
                                }
                                let on = if entry.options & KO_ENABLED_BIT != 0 {
                                    "on"
                                } else {
                                    "off"
                                };
                                col.append(&dialog::row_title(&format!(
                                    "Override #{idx} ({on})"
                                )));
                                col.append(&dialog::row_body(&format!(
                                    "0x{:04X} → 0x{:04X} · layers 0x{:04X} · mods 0x{:02X}",
                                    entry.trigger,
                                    entry.replacement,
                                    entry.layers,
                                    entry.trigger_mods
                                )));
                            }
                        }
                    }
                    status(format!("Wrote key override #{idx}"));
                }
                Err(e) => status(format!("Key override write failed: {e}")),
            }
        }
    });

    TabBuilt { page, sync }
}
