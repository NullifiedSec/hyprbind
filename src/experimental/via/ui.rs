//! VIA / Vial hardware keymap page — layout preview, layer select, live remap,
//! plus Vial tap dance / combos / key overrides when the board supports them.

use crate::debounce::Debouncer;
use crate::dialog;
use crate::experimental::via::{
    self, BoardCapabilities, Combo, DiscoveredDevice, KeyOverride, KeymapSnapshot, LightingSnapshot,
    TapDance, ViaDefinition,
};
use crate::experimental::via::studio_ui;
use crate::experimental::via::vial_ui;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, DropDown, Entry, Fixed, Frame, Label, ListBox, ListBoxRow,
    Notebook, Orientation, PolicyType, Scale, ScrolledWindow, StringList, Window,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct ViaPage {
    pub page: GtkBox,
}

const UNIT_PX: f64 = 44.0;
const GAP_PX: f64 = 3.0;

struct ViaState {
    definitions: Vec<ViaDefinition>,
    devices: Vec<DiscoveredDevice>,
    def_idx: Option<usize>,
    device_idx: Option<usize>,
    layer: u8,
    snapshot: Option<KeymapSnapshot>,
    selected: Option<(u8, u8)>,
    lighting: Option<LightingSnapshot>,
    caps: BoardCapabilities,
    vial_protocol: Option<u32>,
    tap_dances: Vec<TapDance>,
    combos: Vec<Combo>,
    key_overrides: Vec<KeyOverride>,
}

pub fn build_via_page(parent: &impl IsA<Window>, status: Rc<dyn Fn(String)>) -> ViaPage {
    let parent = parent.clone().upcast::<Window>();

    let state = Rc::new(RefCell::new(ViaState {
        definitions: Vec::new(),
        devices: Vec::new(),
        def_idx: None,
        device_idx: None,
        layer: 0,
        snapshot: None,
        selected: None,
        lighting: None,
        caps: BoardCapabilities::default(),
        vial_protocol: None,
        tap_dances: Vec::new(),
        combos: Vec::new(),
        key_overrides: Vec::new(),
    }));

    let refresh_btn = Button::builder()
        .label("Refresh")
        .tooltip_text("Rescan VIA devices and reload definitions")
        .css_classes(["suggested-action"])
        .build();
    let load_def_btn = Button::builder()
        .label("Load definition…")
        .tooltip_text("Sideload a VIA keyboard definition JSON (Design-tab style)")
        .build();
    let apply_btn = Button::builder()
        .label("Apply keycode")
        .tooltip_text("Write the selected keycode to the keyboard via VIA")
        .sensitive(false)
        .build();

    let device_dd = DropDown::from_strings(&["No VIA device"]);
    device_dd.set_tooltip_text(Some("Connected VIA-compatible keyboards"));
    device_dd.set_hexpand(true);

    let def_dd = DropDown::from_strings(&["No definition"]);
    def_dd.set_tooltip_text(Some("VIA definition JSON (matrix + layout)"));
    def_dd.set_hexpand(true);

    let layer_dd = DropDown::from_strings(&["Layer 0"]);
    layer_dd.set_tooltip_text(Some("Dynamic keymap layer"));

    let toolbar = GtkBox::new(Orientation::Horizontal, 8);
    toolbar.append(&refresh_btn);
    toolbar.append(&load_def_btn);
    toolbar.append(&device_dd);
    toolbar.append(&def_dd);
    toolbar.append(&layer_dd);
    toolbar.append(&apply_btn);

    let info_label = Label::builder()
        .label(
            "Connect a VIA/Vial keyboard. Vial boards autofetch their definition; \
             stock VIA needs a sideloaded JSON.",
        )
        .halign(Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();

    let board_fixed = Fixed::new();
    board_fixed.add_css_class("hyprbinds-via-board");
    let board_frame = Frame::builder()
        .child(&board_fixed)
        .css_classes(["hyprbinds-via-frame"])
        .hexpand(true)
        .vexpand(true)
        .build();
    let board_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .hexpand(true)
        .min_content_height(280)
        .child(&board_frame)
        .build();

    let selected_label = Label::builder()
        .label("No key selected")
        .halign(Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label"])
        .build();

    let keycode_search = Entry::builder()
        .placeholder_text("Filter keycodes…")
        .hexpand(true)
        .build();
    let keycode_list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();
    let keycode_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .min_content_height(200)
        .child(&keycode_list)
        .build();

    let hex_entry = Entry::builder()
        .placeholder_text("Or hex e.g. 001F")
        .width_chars(12)
        .build();

    let picker_col = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .width_request(280)
        .build();
    picker_col.append(&dialog::section_label("Selected"));
    picker_col.append(&selected_label);
    picker_col.append(&dialog::section_label("Keycode"));
    picker_col.append(&keycode_search);
    picker_col.append(&keycode_scroll);
    picker_col.append(&hex_entry);

    let keymap_body = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(14)
        .vexpand(true)
        .build();
    let left = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .hexpand(true)
        .vexpand(true)
        .build();
    left.append(&info_label);
    left.append(&board_scroll);
    keymap_body.append(&left);
    keymap_body.append(&picker_col);

    // --- Lighting tab ---
    let light_info = Label::builder()
        .label("No lighting channel detected yet.")
        .halign(Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    let brightness_scale = range_scale(0.0, 255.0, 128.0);
    let speed_scale = range_scale(0.0, 255.0, 128.0);
    let hue_scale = range_scale(0.0, 255.0, 0.0);
    let sat_scale = range_scale(0.0, 255.0, 255.0);
    let effect_dd = DropDown::from_strings(&["All Off"]);
    effect_dd.set_hexpand(true);
    let color_preview = Frame::builder()
        .css_classes(["hyprbinds-via-color-swatch"])
        .width_request(48)
        .height_request(28)
        .halign(Align::Start)
        .build();
    let save_light_btn = Button::builder()
        .label("Save to EEPROM")
        .tooltip_text("Persist lighting settings on the keyboard")
        .css_classes(["suggested-action"])
        .sensitive(false)
        .build();
    let reload_light_btn = Button::builder().label("Reload lighting").build();

    let speed_row = labeled_row("Effect speed", &speed_scale);
    let hue_row = labeled_row("Hue", &hue_scale);
    let sat_row = labeled_row("Saturation", &sat_scale);
    let color_row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .build();
    color_row.append(
        &Label::builder()
            .label("Color")
            .halign(Align::Start)
            .css_classes(["dim-label", "caption"])
            .width_request(100)
            .build(),
    );
    color_row.append(&color_preview);

    let light_actions = GtkBox::new(Orientation::Horizontal, 8);
    light_actions.append(&reload_light_btn);
    light_actions.append(&save_light_btn);

    let studio_hint = Label::builder()
        .label("Per-key colors and animations → Studio tab.")
        .halign(Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();

    let light_col = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(4)
        .margin_end(4)
        .css_classes(["hyprbinds-via-lighting"])
        .build();
    light_col.append(&light_info);
    light_col.append(&dialog::section_label("Backlight"));
    light_col.append(&labeled_row("Brightness", &brightness_scale));
    light_col.append(&labeled_row("Effect", &effect_dd));
    light_col.append(&speed_row);
    light_col.append(&color_row);
    light_col.append(&hue_row);
    light_col.append(&sat_row);
    light_col.append(&light_actions);
    light_col.append(&studio_hint);

    let light_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .child(&light_col)
        .build();

    let hub = Notebook::new();
    hub.add_css_class("hyprbinds-hub");
    hub.set_scrollable(true);
    hub.set_vexpand(true);
    hub.set_hexpand(true);
    hub.append_page(&keymap_body, Some(&Label::new(Some("Keymap"))));
    hub.append_page(&light_scroll, Some(&Label::new(Some("Lighting"))));

    let get_vid_pid: Rc<dyn Fn() -> Option<(u16, u16)>> = {
        let state = Rc::clone(&state);
        Rc::new(move || {
            let st = state.borrow();
            st.device_idx.map(|i| {
                let d = &st.devices[i];
                (d.vendor_id, d.product_id)
            })
        })
    };
    let studio_tab = super::studio_ui::build_studio_tab(Rc::clone(&get_vid_pid), Rc::clone(&status));
    hub.append_page(&studio_tab.page, Some(&Label::new(Some("Studio"))));
    let sync_studio = studio_tab.sync;

    let vial_tabs = super::vial_ui::build_vial_tabs(Rc::clone(&get_vid_pid), Rc::clone(&status));
    hub.append_page(
        &vial_tabs.tap_dance_page,
        Some(&Label::new(Some("Tap Dance"))),
    );
    hub.append_page(&vial_tabs.combo_page, Some(&Label::new(Some("Combos"))));
    hub.append_page(
        &vial_tabs.override_page,
        Some(&Label::new(Some("Overrides"))),
    );
    let sync_vial_tabs = vial_tabs.sync;

    let page = dialog::page_shell(
        "VIA / Vial keymap (experimental)",
        "Experimental hardware remaps and backlight over VIA HID. Vial boards also expose \
         tap dance, combos, and key overrides — definitions autofetch from firmware when possible.",
        &toolbar,
        &hub,
    );

    let key_buttons: Rc<RefCell<Vec<(u8, u8, Button)>>> = Rc::new(RefCell::new(Vec::new()));
    let suppress_dd = Rc::new(Cell::new(false));
    let suppress_light = Rc::new(Cell::new(false));
    let effect_ids: Rc<RefCell<Vec<u16>>> = Rc::new(RefCell::new(Vec::new()));
    let light_debounce = Debouncer::new();

    let update_selected_label = {
        let state = Rc::clone(&state);
        let selected_label = selected_label.clone();
        let apply_btn = apply_btn.clone();
        Rc::new(move || {
            let st = state.borrow();
            let Some((row, col)) = st.selected else {
                selected_label.set_text("No key selected");
                apply_btn.set_sensitive(false);
                return;
            };
            let code = st
                .snapshot
                .as_ref()
                .and_then(|s| {
                    s.map
                        .get(st.layer as usize)
                        .and_then(|rows| rows.get(row as usize))
                        .and_then(|cols| cols.get(col as usize))
                        .copied()
                })
                .unwrap_or(0);
            let customs = st
                .def_idx
                .and_then(|i| st.definitions.get(i))
                .map(|d| d.custom_keycodes.clone())
                .unwrap_or_default();
            selected_label.set_text(&format!(
                "Matrix ({row},{col}) · {}",
                via::keycode_full_label(code, &customs)
            ));
            apply_btn.set_sensitive(
                st.device_idx.is_some() && st.def_idx.is_some() && st.snapshot.is_some(),
            );
        })
    };

    let paint_board = {
        let board_fixed = board_fixed.clone();
        let state = Rc::clone(&state);
        let key_buttons = Rc::clone(&key_buttons);
        let update_selected_label = Rc::clone(&update_selected_label);
        let status = Rc::clone(&status);
        Rc::new(move || {
            while let Some(child) = board_fixed.first_child() {
                board_fixed.remove(&child);
            }
            key_buttons.borrow_mut().clear();

            let st = state.borrow();
            let Some(di) = st.def_idx else {
                board_fixed.set_size_request(200, 80);
                return;
            };
            let Some(def) = st.definitions.get(di) else {
                return;
            };
            let layer = st.layer as usize;
            let selected = st.selected;
            let customs = def.custom_keycodes.clone();
            let keys = def.layout.keys.clone();
            let snapshot = st.snapshot.clone();
            let width = ((def.layout.width() as f64) * (UNIT_PX + GAP_PX) + GAP_PX).ceil() as i32;
            let height = ((def.layout.height() as f64) * (UNIT_PX + GAP_PX) + GAP_PX).ceil() as i32;
            drop(st);

            board_fixed.set_size_request(width.max(200), height.max(80));

            for key in keys {
                let code = snapshot
                    .as_ref()
                    .and_then(|s| {
                        s.map
                            .get(layer)
                            .and_then(|rows| rows.get(key.row as usize))
                            .and_then(|cols| cols.get(key.col as usize))
                            .copied()
                    })
                    .unwrap_or(0);
                let label = if snapshot.is_some() {
                    via::keycode_short_label(code, &customs)
                } else {
                    format!("{},{}", key.row, key.col)
                };
                let btn = Button::builder()
                    .label(&label)
                    .tooltip_text(&via::keycode_full_label(code, &customs))
                    .css_classes(["hyprbinds-via-key"])
                    .build();
                if selected == Some((key.row, key.col)) {
                    btn.add_css_class("hyprbinds-via-key-selected");
                }
                let w = (key.w as f64 * UNIT_PX + (key.w as f64 - 1.0).max(0.0) * GAP_PX).max(28.0);
                let h = (key.h as f64 * UNIT_PX + (key.h as f64 - 1.0).max(0.0) * GAP_PX).max(28.0);
                btn.set_size_request(w as i32, h as i32);
                let x = GAP_PX + key.x as f64 * (UNIT_PX + GAP_PX);
                let y = GAP_PX + key.y as f64 * (UNIT_PX + GAP_PX);
                board_fixed.put(&btn, x, y);

                let row = key.row;
                let col = key.col;
                btn.connect_clicked({
                    let state = Rc::clone(&state);
                    let key_buttons = Rc::clone(&key_buttons);
                    let update_selected_label = Rc::clone(&update_selected_label);
                    let status = Rc::clone(&status);
                    move |_| {
                        state.borrow_mut().selected = Some((row, col));
                        for (r, c, b) in key_buttons.borrow().iter() {
                            if (*r, *c) == (row, col) {
                                b.add_css_class("hyprbinds-via-key-selected");
                            } else {
                                b.remove_css_class("hyprbinds-via-key-selected");
                            }
                        }
                        update_selected_label();
                        status(format!("Selected key ({row},{col})"));
                    }
                });
                key_buttons.borrow_mut().push((key.row, key.col, btn));
            }
        })
    };

    let rebuild_picker = {
        let keycode_list = keycode_list.clone();
        let keycode_search = keycode_search.clone();
        let state = Rc::clone(&state);
        Rc::new(move || {
            while let Some(child) = keycode_list.first_child() {
                keycode_list.remove(&child);
            }
            let filter = keycode_search.text().to_lowercase();
            let customs = {
                let st = state.borrow();
                st.def_idx
                    .and_then(|i| st.definitions.get(i).map(|d| d.custom_keycodes.clone()))
                    .unwrap_or_default()
            };
            for (label, code) in via::picker_entries(&customs) {
                if !filter.is_empty() {
                    let hay = format!("{label} {code:04x}").to_lowercase();
                    if !hay.contains(&filter) {
                        continue;
                    }
                }
                let row_box = dialog::list_row_column();
                row_box.append(&dialog::row_title(&label));
                row_box.append(&dialog::row_body(&format!("0x{code:04X}")));
                let row = ListBoxRow::builder().child(&row_box).build();
                row.set_widget_name(&format!("{code}"));
                keycode_list.append(&row);
            }
        })
    };

    let sync_layer_dropdown = {
        let layer_dd = layer_dd.clone();
        let state = Rc::clone(&state);
        let suppress_dd = Rc::clone(&suppress_dd);
        Rc::new(move || {
            suppress_dd.set(true);
            let layers = state
                .borrow()
                .snapshot
                .as_ref()
                .map(|s| s.layers)
                .unwrap_or(1)
                .max(1);
            let labels: Vec<String> = (0..layers).map(|i| format!("Layer {i}")).collect();
            let list = StringList::new(&labels.iter().map(|s| s.as_str()).collect::<Vec<_>>());
            layer_dd.set_model(Some(&list));
            let cur = state.borrow().layer.min(layers.saturating_sub(1));
            layer_dd.set_selected(cur as u32);
            state.borrow_mut().layer = cur;
            suppress_dd.set(false);
        })
    };

    let sync_lighting_ui = {
        let state = Rc::clone(&state);
        let suppress_light = Rc::clone(&suppress_light);
        let effect_ids = Rc::clone(&effect_ids);
        let light_info = light_info.clone();
        let brightness_scale = brightness_scale.clone();
        let speed_scale = speed_scale.clone();
        let hue_scale = hue_scale.clone();
        let sat_scale = sat_scale.clone();
        let effect_dd = effect_dd.clone();
        let speed_row = speed_row.clone();
        let hue_row = hue_row.clone();
        let sat_row = sat_row.clone();
        let color_row = color_row.clone();
        let color_preview = color_preview.clone();
        let save_light_btn = save_light_btn.clone();
        let reload_light_btn = reload_light_btn.clone();
        Rc::new(move || {
            suppress_light.set(true);
            let st = state.borrow();
            let effects = st
                .def_idx
                .and_then(|i| st.definitions.get(i))
                .map(via::effects_for_definition)
                .unwrap_or_else(via::default_rgb_matrix_effects);
            let labels: Vec<String> = effects.iter().map(|(n, _)| n.clone()).collect();
            let ids: Vec<u16> = effects.iter().map(|(_, id)| *id).collect();
            *effect_ids.borrow_mut() = ids.clone();
            let list = StringList::new(&labels.iter().map(|s| s.as_str()).collect::<Vec<_>>());
            effect_dd.set_model(Some(&list));

            let Some(light) = st.lighting.clone() else {
                drop(st);
                light_info.set_text(
                    "No lighting channel on this device (or keyboard not connected).",
                );
                brightness_scale.set_sensitive(false);
                speed_scale.set_sensitive(false);
                hue_scale.set_sensitive(false);
                sat_scale.set_sensitive(false);
                effect_dd.set_sensitive(false);
                save_light_btn.set_sensitive(false);
                reload_light_btn.set_sensitive(state.borrow().device_idx.is_some());
                suppress_light.set(false);
                return;
            };
            drop(st);

            light_info.set_text(&format!(
                "{} · brightness {} · effect {} · speed {}",
                light.label, light.brightness, light.effect, light.speed
            ));
            brightness_scale.set_sensitive(true);
            effect_dd.set_sensitive(true);
            save_light_btn.set_sensitive(true);
            reload_light_btn.set_sensitive(true);

            brightness_scale.set_value(light.brightness as f64);
            speed_scale.set_value(light.speed as f64);
            hue_scale.set_value(light.hue as f64);
            sat_scale.set_value(light.saturation as f64);

            let idx = ids
                .iter()
                .position(|id| *id == light.effect)
                .unwrap_or(0);
            effect_dd.set_selected(idx as u32);

            let show_speed = light.effect != 0;
            let show_color = !via::effect_hides_color(light.effect);
            speed_row.set_visible(show_speed);
            hue_row.set_visible(show_color);
            sat_row.set_visible(show_color);
            color_row.set_visible(show_color);
            speed_scale.set_sensitive(show_speed);
            hue_scale.set_sensitive(show_color);
            sat_scale.set_sensitive(show_color);
            set_color_swatch(&color_preview, light.hue, light.saturation);
            suppress_light.set(false);
        })
    };

    let push_lighting = {
        let state = Rc::clone(&state);
        let suppress_light = Rc::clone(&suppress_light);
        let effect_ids = Rc::clone(&effect_ids);
        let brightness_scale = brightness_scale.clone();
        let speed_scale = speed_scale.clone();
        let hue_scale = hue_scale.clone();
        let sat_scale = sat_scale.clone();
        let effect_dd = effect_dd.clone();
        let light_info = light_info.clone();
        let speed_row = speed_row.clone();
        let hue_row = hue_row.clone();
        let sat_row = sat_row.clone();
        let color_row = color_row.clone();
        let color_preview = color_preview.clone();
        let status = Rc::clone(&status);
        Rc::new(move || {
            if suppress_light.get() {
                return;
            }
            let (vid, pid, label) = {
                let st = state.borrow();
                let Some(dev_i) = st.device_idx else {
                    return;
                };
                let d = &st.devices[dev_i];
                let label = st
                    .lighting
                    .as_ref()
                    .map(|l| l.label.clone())
                    .unwrap_or_else(|| "Lighting".into());
                (d.vendor_id, d.product_id, label)
            };
            let effect = {
                let ids = effect_ids.borrow();
                let idx = effect_dd.selected() as usize;
                ids.get(idx).copied().unwrap_or(0)
            };
            let snap = LightingSnapshot {
                label: label.clone(),
                brightness: brightness_scale.value().round().clamp(0.0, 255.0) as u8,
                effect,
                speed: speed_scale.value().round().clamp(0.0, 255.0) as u8,
                hue: hue_scale.value().round().clamp(0.0, 255.0) as u8,
                saturation: sat_scale.value().round().clamp(0.0, 255.0) as u8,
            };
            match via::write_lighting(vid, pid, &snap) {
                Ok(()) => {
                    light_info.set_text(&format!(
                        "{} · brightness {} · effect {} · speed {}",
                        snap.label, snap.brightness, snap.effect, snap.speed
                    ));
                    let show_speed = snap.effect != 0;
                    let show_color = !via::effect_hides_color(snap.effect);
                    speed_row.set_visible(show_speed);
                    hue_row.set_visible(show_color);
                    sat_row.set_visible(show_color);
                    color_row.set_visible(show_color);
                    speed_scale.set_sensitive(show_speed);
                    hue_scale.set_sensitive(show_color);
                    sat_scale.set_sensitive(show_color);
                    set_color_swatch(&color_preview, snap.hue, snap.saturation);
                    state.borrow_mut().lighting = Some(snap);
                }
                Err(e) => status(format!("Lighting write failed: {e}")),
            }
        })
    };

    let load_lighting_from_device = {
        let state = Rc::clone(&state);
        let sync_lighting_ui = Rc::clone(&sync_lighting_ui);
        let sync_studio = Rc::clone(&sync_studio);
        let status = Rc::clone(&status);
        Rc::new(move || {
            let vid_pid = {
                let st = state.borrow();
                st.device_idx.map(|dev_i| {
                    let d = &st.devices[dev_i];
                    (d.vendor_id, d.product_id)
                })
            };
            let Some((vid, pid)) = vid_pid else {
                state.borrow_mut().lighting = None;
                sync_lighting_ui();
                {
                    let st = state.borrow();
                    let def = st.def_idx.and_then(|i| st.definitions.get(i));
                    sync_studio(def, st.caps.paint_backend, None);
                }
                return;
            };
            match via::read_lighting(vid, pid) {
                Ok(Some(snap)) => {
                    status(format!("Lighting: {}", snap.label));
                    state.borrow_mut().lighting = Some(snap);
                    sync_lighting_ui();
                    {
                        let st = state.borrow();
                        let def = st.def_idx.and_then(|i| st.definitions.get(i));
                        sync_studio(def, st.caps.paint_backend, st.lighting.clone());
                    }
                }
                Ok(None) => {
                    state.borrow_mut().lighting = None;
                    sync_lighting_ui();
                    {
                        let st = state.borrow();
                        let def = st.def_idx.and_then(|i| st.definitions.get(i));
                        sync_studio(def, st.caps.paint_backend, None);
                    }
                    status("No VIA lighting channel on this keyboard".into());
                }
                Err(e) => {
                    state.borrow_mut().lighting = None;
                    sync_lighting_ui();
                    {
                        let st = state.borrow();
                        let def = st.def_idx.and_then(|i| st.definitions.get(i));
                        sync_studio(def, st.caps.paint_backend, None);
                    }
                    status(format!("Lighting read failed: {e}"));
                }
            }
        })
    };

    let sync_vial_from_state = {
        let state = Rc::clone(&state);
        let sync_vial_tabs = Rc::clone(&sync_vial_tabs);
        Rc::new(move || {
            let st = state.borrow();
            let customs = st
                .def_idx
                .and_then(|i| st.definitions.get(i))
                .map(|d| d.custom_keycodes.clone())
                .unwrap_or_default();
            sync_vial_tabs(
                &st.caps,
                &st.tap_dances,
                &st.combos,
                &st.key_overrides,
                &customs,
            );
        })
    };

    let sync_studio_from_state = {
        let state = Rc::clone(&state);
        let sync_studio = Rc::clone(&sync_studio);
        Rc::new(move || {
            let st = state.borrow();
            let def = st.def_idx.and_then(|i| st.definitions.get(i));
            sync_studio(def, st.caps.paint_backend, st.lighting.clone());
        })
    };

    let clear_vial_features = {
        let state = Rc::clone(&state);
        let sync_vial_from_state = Rc::clone(&sync_vial_from_state);
        Rc::new(move || {
            {
                let mut st = state.borrow_mut();
                st.caps = BoardCapabilities::default();
                st.vial_protocol = None;
                st.tap_dances.clear();
                st.combos.clear();
                st.key_overrides.clear();
            }
            sync_vial_from_state();
        })
    };

    // Probe Vial, autofetch definition into the dropdown, and load dynamic slots.
    let apply_vial_for_device = {
        let state = Rc::clone(&state);
        let def_dd = def_dd.clone();
        let suppress_dd = Rc::clone(&suppress_dd);
        let sync_vial_from_state = Rc::clone(&sync_vial_from_state);
        let status = Rc::clone(&status);
        Rc::new(move || -> String {
            let vid_pid = {
                let st = state.borrow();
                st.device_idx.map(|i| {
                    let d = &st.devices[i];
                    (d.vendor_id, d.product_id)
                })
            };
            let Some((vid, pid)) = vid_pid else {
                {
                    let mut st = state.borrow_mut();
                    st.caps = BoardCapabilities::default();
                    st.vial_protocol = None;
                    st.tap_dances.clear();
                    st.combos.clear();
                    st.key_overrides.clear();
                }
                sync_vial_from_state();
                return String::new();
            };

            let vial_info = via::detect_vial(vid, pid).ok().flatten();
            let caps = via::probe_capabilities(vid, pid).unwrap_or_default();

            {
                let mut st = state.borrow_mut();
                st.caps = caps.clone();
                st.vial_protocol = vial_info.as_ref().map(|v| v.protocol_version);
            }

            if caps.is_vial {
                match via::fetch_vial_definition(vid, pid) {
                    Ok(def) => {
                        let mut st = state.borrow_mut();
                        let name = def.name.clone();
                        if let Some(i) =
                            via::find_definition_for(&st.definitions, vid, pid)
                        {
                            st.definitions[i] = def;
                            st.def_idx = Some(i);
                        } else {
                            st.definitions.push(def);
                            st.def_idx = Some(st.definitions.len() - 1);
                        }
                        // Refresh definition dropdown labels + selection.
                        let def_labels: Vec<String> = st
                            .definitions
                            .iter()
                            .map(|d| {
                                format!(
                                    "{} ({:04X}:{:04X})",
                                    d.name, d.vendor_id, d.product_id
                                )
                            })
                            .collect();
                        let list = StringList::new(
                            &def_labels.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                        );
                        suppress_dd.set(true);
                        def_dd.set_model(Some(&list));
                        if let Some(i) = st.def_idx {
                            def_dd.set_selected(i as u32);
                        }
                        suppress_dd.set(false);
                        status(format!("Fetched Vial definition · {name}"));
                    }
                    Err(e) => {
                        status(format!(
                            "Vial board detected, but definition fetch failed: {e}"
                        ));
                    }
                }
            }

            // Load dynamic EEPROM slots when Vial reports capacity.
            {
                let mut st = state.borrow_mut();
                if caps.is_vial && caps.tap_dance > 0 {
                    st.tap_dances = via::read_tap_dances(vid, pid).unwrap_or_default();
                } else {
                    st.tap_dances.clear();
                }
                if caps.is_vial && caps.combo > 0 {
                    st.combos = via::read_combos(vid, pid).unwrap_or_default();
                } else {
                    st.combos.clear();
                }
                if caps.is_vial && caps.key_override > 0 {
                    st.key_overrides =
                        via::read_key_overrides(vid, pid).unwrap_or_default();
                } else {
                    st.key_overrides.clear();
                }
            }
            sync_vial_from_state();

            if caps.is_vial {
                let p = vial_info
                    .map(|v| v.protocol_version)
                    .unwrap_or(0);
                format!(
                    "Vial p={p} · TD={} · Combo={} · KO={}",
                    caps.tap_dance, caps.combo, caps.key_override
                )
            } else {
                String::new()
            }
        })
    };

    let load_keymap_from_device = {
        let state = Rc::clone(&state);
        let info_label = info_label.clone();
        let paint_board = Rc::clone(&paint_board);
        let sync_layer_dropdown = Rc::clone(&sync_layer_dropdown);
        let update_selected_label = Rc::clone(&update_selected_label);
        let load_lighting_from_device = Rc::clone(&load_lighting_from_device);
        let status = Rc::clone(&status);
        Rc::new(move || {
            let (vid, pid, rows, cols, name) = {
                let st = state.borrow();
                let Some(dev_i) = st.device_idx else {
                    status("No VIA device selected".into());
                    return;
                };
                let Some(def_i) = st.def_idx else {
                    status("No definition selected — Load definition… or pick one".into());
                    return;
                };
                let dev = &st.devices[dev_i];
                let def = &st.definitions[def_i];
                (
                    dev.vendor_id,
                    dev.product_id,
                    def.rows,
                    def.cols,
                    format!("{} · {}", def.name, dev.product),
                )
            };
            match via::read_keymap(vid, pid, rows, cols) {
                Ok(snap) => {
                    let mismatch = {
                        let st = state.borrow();
                        match (st.device_idx, st.def_idx) {
                            (Some(di), Some(fi)) => {
                                let d = &st.devices[di];
                                let f = &st.definitions[fi];
                                d.vendor_id != f.vendor_id || d.product_id != f.product_id
                            }
                            _ => false,
                        }
                    };
                    let vial_bit = {
                        let st = state.borrow();
                        if st.caps.is_vial {
                            let p = st.vial_protocol.unwrap_or(0);
                            format!(
                                " · Vial p={p} · TD={} · Combo={} · KO={}",
                                st.caps.tap_dance, st.caps.combo, st.caps.key_override
                            )
                        } else {
                            String::new()
                        }
                    };
                    let mut info = format!(
                        "{name} · VIA protocol {} · {} layers · {}×{} matrix{vial_bit}",
                        snap.protocol, snap.layers, rows, cols
                    );
                    if mismatch {
                        info.push_str(
                            " · warning: definition VID/PID differs from device (layout may be wrong)",
                        );
                    }
                    info_label.set_text(&info);
                    state.borrow_mut().snapshot = Some(snap);
                    sync_layer_dropdown();
                    paint_board();
                    update_selected_label();
                    status(format!("Loaded keymap from {name}{vial_bit}"));
                    load_lighting_from_device();
                }
                Err(e) => {
                    state.borrow_mut().snapshot = None;
                    paint_board();
                    status(format!("VIA read failed: {e}"));
                }
            }
        })
    };

    let refresh_all = {
        let state = Rc::clone(&state);
        let device_dd = device_dd.clone();
        let def_dd = def_dd.clone();
        let info_label = info_label.clone();
        let suppress_dd = Rc::clone(&suppress_dd);
        let paint_board = Rc::clone(&paint_board);
        let rebuild_picker = Rc::clone(&rebuild_picker);
        let load_keymap_from_device = Rc::clone(&load_keymap_from_device);
        let apply_vial_for_device = Rc::clone(&apply_vial_for_device);
        let clear_vial_features = Rc::clone(&clear_vial_features);
        let sync_studio_from_state = Rc::clone(&sync_studio_from_state);
        let status = Rc::clone(&status);
        Rc::new(move || {
            if let Some(hint) = via::hid_permission_hint() {
                status(hint);
            }

            let defs = via::list_definitions().unwrap_or_else(|e| {
                status(format!("Definitions: {e}"));
                Vec::new()
            });
            let devices = via::discover_devices().unwrap_or_else(|e| {
                status(format!("Devices: {e}"));
                Vec::new()
            });

            suppress_dd.set(true);
            {
                let mut st = state.borrow_mut();
                st.definitions = defs;
                st.devices = devices;

                // Device dropdown
                let dev_labels: Vec<String> = if st.devices.is_empty() {
                    vec!["No VIA device".into()]
                } else {
                    st.devices
                        .iter()
                        .map(|d| {
                            let name = if !d.product.is_empty() {
                                d.product.as_str()
                            } else if !d.manufacturer.is_empty() {
                                d.manufacturer.as_str()
                            } else {
                                "VIA keyboard"
                            };
                            format!(
                                "{} ({:04X}:{:04X})",
                                name, d.vendor_id, d.product_id
                            )
                        })
                        .collect()
                };
                let list = StringList::new(
                    &dev_labels
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>(),
                );
                device_dd.set_model(Some(&list));

                // Prefer previously selected device, else first.
                let device_idx = if st.devices.is_empty() {
                    None
                } else {
                    st.device_idx
                        .filter(|&i| i < st.devices.len())
                        .or(Some(0))
                };
                st.device_idx = device_idx;
                if let Some(i) = device_idx {
                    device_dd.set_selected(i as u32);
                }

                // Definition dropdown — preliminary; Vial autofetch may replace.
                let def_labels: Vec<String> = if st.definitions.is_empty() {
                    vec!["No definition — Load definition…".into()]
                } else {
                    st.definitions
                        .iter()
                        .map(|d| {
                            format!(
                                "{} ({:04X}:{:04X})",
                                d.name, d.vendor_id, d.product_id
                            )
                        })
                        .collect()
                };
                let list = StringList::new(
                    &def_labels
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>(),
                );
                def_dd.set_model(Some(&list));

                let matched = device_idx.and_then(|di| {
                    let d = &st.devices[di];
                    via::find_definition_for(&st.definitions, d.vendor_id, d.product_id)
                });
                let def_idx = matched
                    .or_else(|| st.def_idx.filter(|&i| i < st.definitions.len()))
                    .or_else(|| (!st.definitions.is_empty()).then_some(0));
                st.def_idx = def_idx;
                if let Some(i) = def_idx {
                    def_dd.set_selected(i as u32);
                }

                if st.devices.is_empty() {
                    info_label.set_text(
                        "No VIA/Vial keyboard found (HID usage page 0xFF60). Plug one in and Refresh.",
                    );
                    st.snapshot = None;
                } else if st.definitions.is_empty() {
                    info_label.set_text(
                        "Device found, but no definition JSON yet. Vial boards autofetch on \
                         connect; otherwise use Load definition… (VIA Design-tab JSON).",
                    );
                    st.snapshot = None;
                }
            }
            suppress_dd.set(false);

            let vial_status = if state.borrow().device_idx.is_some() {
                apply_vial_for_device()
            } else {
                clear_vial_features();
                String::new()
            };

            rebuild_picker();
            paint_board();
            sync_studio_from_state();

            let can_load = {
                let st = state.borrow();
                st.device_idx.is_some() && st.def_idx.is_some()
            };
            if can_load {
                load_keymap_from_device();
            } else if !vial_status.is_empty() {
                status(vial_status);
            } else {
                status(format!(
                    "VIA ready · {} definition(s) in {}",
                    state.borrow().definitions.len(),
                    via::definitions_dir().display()
                ));
            }
        })
    };

    refresh_btn.connect_clicked({
        let refresh_all = Rc::clone(&refresh_all);
        move |_| refresh_all()
    });

    load_def_btn.connect_clicked({
        let parent = parent.clone();
        let refresh_all = Rc::clone(&refresh_all);
        let status = Rc::clone(&status);
        move |_| {
            let refresh_all = Rc::clone(&refresh_all);
            let status = Rc::clone(&status);
            let dialog = gtk4::FileDialog::builder()
                .title("Load VIA definition JSON")
                .build();
            let filter = gtk4::FileFilter::new();
            filter.add_pattern("*.json");
            filter.set_name(Some("VIA definitions (*.json)"));
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
                if !via::looks_like_via_definition(&path) {
                    status(
                        "File does not look like a VIA definition (need vendorId, productId, \
                         matrix, layouts.keymap)"
                            .into(),
                    );
                    return;
                }
                match via::import_definition(&path) {
                    Ok(def) => {
                        status(format!(
                            "Imported {} → {}",
                            def.name,
                            def.path.display()
                        ));
                        refresh_all();
                    }
                    Err(e) => status(format!("Import failed: {e}")),
                }
            });
        }
    });

    device_dd.connect_selected_notify({
        let state = Rc::clone(&state);
        let suppress_dd = Rc::clone(&suppress_dd);
        let def_dd = def_dd.clone();
        let load_keymap_from_device = Rc::clone(&load_keymap_from_device);
        let apply_vial_for_device = Rc::clone(&apply_vial_for_device);
        let clear_vial_features = Rc::clone(&clear_vial_features);
        let rebuild_picker = Rc::clone(&rebuild_picker);
        let paint_board = Rc::clone(&paint_board);
        let sync_studio_from_state = Rc::clone(&sync_studio_from_state);
        move |dd| {
            if suppress_dd.get() {
                return;
            }
            let idx = dd.selected() as usize;
            let mut st = state.borrow_mut();
            if st.devices.is_empty() || idx >= st.devices.len() {
                st.device_idx = None;
                st.snapshot = None;
                drop(st);
                clear_vial_features();
                paint_board();
                sync_studio_from_state();
                return;
            }
            st.device_idx = Some(idx);
            let d = st.devices[idx].clone();
            if let Some(mi) = via::find_definition_for(&st.definitions, d.vendor_id, d.product_id)
            {
                st.def_idx = Some(mi);
                suppress_dd.set(true);
                def_dd.set_selected(mi as u32);
                suppress_dd.set(false);
            }
            drop(st);
            apply_vial_for_device();
            rebuild_picker();
            if state.borrow().def_idx.is_some() {
                load_keymap_from_device();
            } else {
                paint_board();
                sync_studio_from_state();
            }
        }
    });

    def_dd.connect_selected_notify({
        let state = Rc::clone(&state);
        let suppress_dd = Rc::clone(&suppress_dd);
        let rebuild_picker = Rc::clone(&rebuild_picker);
        let load_keymap_from_device = Rc::clone(&load_keymap_from_device);
        let paint_board = Rc::clone(&paint_board);
        let sync_studio_from_state = Rc::clone(&sync_studio_from_state);
        move |dd| {
            if suppress_dd.get() {
                return;
            }
            let idx = dd.selected() as usize;
            let mut st = state.borrow_mut();
            if st.definitions.is_empty() || idx >= st.definitions.len() {
                st.def_idx = None;
                st.snapshot = None;
                drop(st);
                rebuild_picker();
                paint_board();
                sync_studio_from_state();
                return;
            }
            st.def_idx = Some(idx);
            st.selected = None;
            drop(st);
            rebuild_picker();
            if state.borrow().device_idx.is_some() {
                load_keymap_from_device();
            } else {
                paint_board();
                sync_studio_from_state();
            }
        }
    });

    layer_dd.connect_selected_notify({
        let state = Rc::clone(&state);
        let suppress_dd = Rc::clone(&suppress_dd);
        let paint_board = Rc::clone(&paint_board);
        let update_selected_label = Rc::clone(&update_selected_label);
        move |dd| {
            if suppress_dd.get() {
                return;
            }
            let layer = dd.selected() as u8;
            state.borrow_mut().layer = layer;
            paint_board();
            update_selected_label();
        }
    });

    keycode_search.connect_changed({
        let rebuild_picker = Rc::clone(&rebuild_picker);
        move |_| rebuild_picker()
    });

    keycode_list.connect_row_activated({
        let hex_entry = hex_entry.clone();
        move |_, row| {
            let name = row.widget_name().to_string();
            if let Ok(code) = name.parse::<u16>() {
                hex_entry.set_text(&format!("{code:04X}"));
            }
        }
    });

    apply_btn.connect_clicked({
        let state = Rc::clone(&state);
        let hex_entry = hex_entry.clone();
        let keycode_list = keycode_list.clone();
        let load_keymap_from_device = Rc::clone(&load_keymap_from_device);
        let status = Rc::clone(&status);
        move |_| {
            let (vid, pid, layer, row, col) = {
                let st = state.borrow();
                let Some((row, col)) = st.selected else {
                    status("Select a key on the board first".into());
                    return;
                };
                let Some(dev_i) = st.device_idx else {
                    status("No device".into());
                    return;
                };
                let d = &st.devices[dev_i];
                (d.vendor_id, d.product_id, st.layer, row, col)
            };

            let code = if let Ok(parsed) = parse_hex_keycode(&hex_entry.text()) {
                parsed
            } else if let Some(row) = keycode_list.selected_row() {
                match row.widget_name().parse::<u16>() {
                    Ok(c) => c,
                    Err(_) => {
                        status("Pick a keycode or enter hex".into());
                        return;
                    }
                }
            } else {
                status("Pick a keycode or enter hex".into());
                return;
            };

            match via::set_keycode(vid, pid, layer, row, col, code) {
                Ok(()) => {
                    status(format!(
                        "Wrote 0x{code:04X} → L{layer} ({row},{col})"
                    ));
                    load_keymap_from_device();
                }
                Err(e) => status(format!("VIA write failed: {e}")),
            }
        }
    });

    let schedule_light_push = {
        let push_lighting = Rc::clone(&push_lighting);
        let light_debounce = light_debounce.clone();
        Rc::new(move || {
            let push_lighting = Rc::clone(&push_lighting);
            light_debounce.schedule_after(180, move || push_lighting());
        })
    };

    brightness_scale.connect_value_changed({
        let schedule_light_push = Rc::clone(&schedule_light_push);
        let suppress_light = Rc::clone(&suppress_light);
        move |_| {
            if !suppress_light.get() {
                schedule_light_push();
            }
        }
    });
    speed_scale.connect_value_changed({
        let schedule_light_push = Rc::clone(&schedule_light_push);
        let suppress_light = Rc::clone(&suppress_light);
        move |_| {
            if !suppress_light.get() {
                schedule_light_push();
            }
        }
    });
    hue_scale.connect_value_changed({
        let schedule_light_push = Rc::clone(&schedule_light_push);
        let suppress_light = Rc::clone(&suppress_light);
        let color_preview = color_preview.clone();
        let hue_scale = hue_scale.clone();
        let sat_scale = sat_scale.clone();
        move |_| {
            if suppress_light.get() {
                return;
            }
            set_color_swatch(
                &color_preview,
                hue_scale.value().round() as u8,
                sat_scale.value().round() as u8,
            );
            schedule_light_push();
        }
    });
    sat_scale.connect_value_changed({
        let schedule_light_push = Rc::clone(&schedule_light_push);
        let suppress_light = Rc::clone(&suppress_light);
        let color_preview = color_preview.clone();
        let hue_scale = hue_scale.clone();
        let sat_scale = sat_scale.clone();
        move |_| {
            if suppress_light.get() {
                return;
            }
            set_color_swatch(
                &color_preview,
                hue_scale.value().round() as u8,
                sat_scale.value().round() as u8,
            );
            schedule_light_push();
        }
    });
    effect_dd.connect_selected_notify({
        let schedule_light_push = Rc::clone(&schedule_light_push);
        let suppress_light = Rc::clone(&suppress_light);
        move |_| {
            if !suppress_light.get() {
                schedule_light_push();
            }
        }
    });

    reload_light_btn.connect_clicked({
        let load_lighting_from_device = Rc::clone(&load_lighting_from_device);
        move |_| load_lighting_from_device()
    });

    save_light_btn.connect_clicked({
        let state = Rc::clone(&state);
        let push_lighting = Rc::clone(&push_lighting);
        let status = Rc::clone(&status);
        move |_| {
            push_lighting();
            let (vid, pid) = {
                let st = state.borrow();
                let Some(dev_i) = st.device_idx else {
                    status("No device".into());
                    return;
                };
                let d = &st.devices[dev_i];
                (d.vendor_id, d.product_id)
            };
            match via::save_lighting(vid, pid) {
                Ok(()) => status("Lighting saved to EEPROM".into()),
                Err(e) => status(format!("Lighting save failed: {e}")),
            }
        }
    });

    refresh_all();

    ViaPage { page }
}

fn glib_propagate() -> glib::Propagation {
    glib::Propagation::Proceed
}

fn glib_inhibit() -> glib::Propagation {
    glib::Propagation::Stop
}

fn range_scale(min: f64, max: f64, initial: f64) -> Scale {
    let adj = gtk4::Adjustment::new(initial, min, max, 1.0, 8.0, 0.0);
    let scale = Scale::new(Orientation::Horizontal, Some(&adj));
    scale.set_draw_value(true);
    scale.set_hexpand(true);
    scale.set_digits(0);
    scale
}

fn labeled_row(label: &str, widget: &impl IsA<gtk4::Widget>) -> GtkBox {
    let row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .build();
    row.append(
        &Label::builder()
            .label(label)
            .halign(Align::Start)
            .css_classes(["dim-label", "caption"])
            .width_request(100)
            .build(),
    );
    row.append(widget);
    row
}

fn set_color_swatch(frame: &Frame, hue: u8, sat: u8) {
    // Approximate HSV→RGB with value=1.0 for the preview swatch.
    let (r, g, b) = via::hsv_to_rgb(hue, sat, 255);
    let css = format!(
        ".hyprbinds-via-color-swatch {{ background-color: rgb({r},{g},{b}); border-radius: 6px; border: 1px solid alpha(currentColor, 0.2); }}"
    );
    let provider = gtk4::CssProvider::new();
    provider.load_from_string(&css);
    frame.style_context().add_provider(
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn parse_hex_keycode(s: &str) -> Result<u16, ()> {
    let s = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    if s.is_empty() {
        return Err(());
    }
    u16::from_str_radix(s, 16).map_err(|_| ())
}
