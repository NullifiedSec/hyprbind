//! RGB Studio notebook tab — paint, patterns, frame timeline, playback, presets.

use crate::dialog;
use crate::via::{self, LightingSnapshot, PaintBackend, ViaDefinition};
use crate::via_studio::{
    self, AnimFrame, Animation, Rgb, DEFAULT_DELAY_MS,
};
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, CheckButton, DropDown, Entry, Fixed, Frame, Label, Orientation,
    PolicyType, Scale, ScrolledWindow, SpinButton, StringList, Switch,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

const UNIT_PX: f64 = 44.0;
const GAP_PX: f64 = 3.0;

type StatusFn = Rc<dyn Fn(String)>;
type VidPidFn = Rc<dyn Fn() -> Option<(u16, u16)>>;

pub struct StudioTab {
    pub page: ScrolledWindow,
    /// Refresh layout / backend / lighting snapshot used for exit restore.
    pub sync: Rc<dyn Fn(Option<&ViaDefinition>, PaintBackend, Option<LightingSnapshot>)>,
}

struct StudioState {
    anim: Animation,
    frame_idx: usize,
    backend: PaintBackend,
    lighting: Option<LightingSnapshot>,
    /// Layout keys `(row, col)` plus geometry for the board.
    keys: Vec<via_protocol::KeyPosition>,
    layout_order: Vec<(u8, u8)>,
    layout_w: f32,
    layout_h: f32,
}

impl Default for StudioState {
    fn default() -> Self {
        Self {
            anim: Animation::default(),
            frame_idx: 0,
            backend: PaintBackend::None,
            lighting: None,
            keys: Vec::new(),
            layout_order: Vec::new(),
            layout_w: 0.0,
            layout_h: 0.0,
        }
    }
}

pub fn build_studio_tab(get_vid_pid: VidPidFn, status: StatusFn) -> StudioTab {
    let state = Rc::new(RefCell::new(StudioState::default()));
    let playing = Rc::new(Cell::new(false));
    let key_buttons: Rc<RefCell<Vec<(u8, u8, Button)>>> = Rc::new(RefCell::new(Vec::new()));

    let info = Label::builder()
        .label("Connect a keyboard with OpenRGB-offset or VialRGB Direct to paint and animate.")
        .halign(Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();

    let hue_scale = range_scale(0.0, 255.0, 0.0);
    let sat_scale = range_scale(0.0, 255.0, 255.0);
    let val_scale = range_scale(0.0, 255.0, 255.0);
    let brush_preview = Frame::builder()
        .css_classes(["hyprbinds-via-color-swatch"])
        .width_request(48)
        .height_request(28)
        .halign(Align::Start)
        .build();
    let live_switch = Switch::builder()
        .valign(Align::Center)
        .active(true)
        .tooltip_text("Write each paint stroke to the keyboard immediately")
        .build();

    let pattern_row = GtkBox::new(Orientation::Horizontal, 6);
    let fill_btn = Button::builder().label("Fill").build();
    let clear_btn = Button::builder().label("Clear").build();
    let checker_btn = Button::builder().label("Checker").build();
    let gradient_btn = Button::builder().label("Gradient").build();
    let rainbow_btn = Button::builder().label("Rainbow").build();
    for b in [
        &fill_btn,
        &clear_btn,
        &checker_btn,
        &gradient_btn,
        &rainbow_btn,
    ] {
        pattern_row.append(b);
    }

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
        .min_content_height(200)
        .vexpand(false)
        .child(&board_frame)
        .build();

    let frame_label = Label::builder()
        .label("Frame 1 / 1")
        .halign(Align::Start)
        .css_classes(["title-4"])
        .build();
    let delay_spin = SpinButton::with_range(16.0, 10_000.0, 10.0);
    delay_spin.set_value(DEFAULT_DELAY_MS as f64);
    delay_spin.set_tooltip_text(Some("Delay after this frame (ms)"));
    let prev_btn = Button::builder().label("◀").tooltip_text("Previous frame").build();
    let next_btn = Button::builder().label("▶").tooltip_text("Next frame").build();
    let add_btn = Button::builder().label("Add").tooltip_text("Append empty frame").build();
    let dup_btn = Button::builder()
        .label("Duplicate")
        .tooltip_text("Duplicate current frame")
        .build();
    let del_btn = Button::builder()
        .label("Delete")
        .tooltip_text("Delete current frame")
        .build();

    let timeline_row = GtkBox::new(Orientation::Horizontal, 8);
    timeline_row.append(&prev_btn);
    timeline_row.append(&next_btn);
    timeline_row.append(&add_btn);
    timeline_row.append(&dup_btn);
    timeline_row.append(&del_btn);
    timeline_row.append(&labeled_inline("Delay ms", &delay_spin));

    let play_btn = Button::builder()
        .label("Play")
        .css_classes(["suggested-action"])
        .build();
    let stop_btn = Button::builder().label("Stop").sensitive(false).build();
    let loop_check = CheckButton::builder()
        .label("Loop")
        .active(true)
        .build();
    let transport_row = GtkBox::new(Orientation::Horizontal, 8);
    transport_row.append(&play_btn);
    transport_row.append(&stop_btn);
    transport_row.append(&loop_check);

    let preset_name = Entry::builder()
        .placeholder_text("Preset name")
        .hexpand(true)
        .build();
    let preset_dd = DropDown::from_strings(&["(no presets)"]);
    preset_dd.set_hexpand(true);
    let save_preset_btn = Button::builder()
        .label("Save")
        .css_classes(["suggested-action"])
        .build();
    let load_preset_btn = Button::builder().label("Load").build();
    let delete_preset_btn = Button::builder().label("Delete").build();
    let preset_row = GtkBox::new(Orientation::Horizontal, 8);
    preset_row.append(&preset_name);
    preset_row.append(&save_preset_btn);
    preset_row.append(&preset_dd);
    preset_row.append(&load_preset_btn);
    preset_row.append(&delete_preset_btn);

    let live_row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .build();
    live_row.append(
        &Label::builder()
            .label("Live paint")
            .halign(Align::Start)
            .css_classes(["dim-label", "caption"])
            .width_request(100)
            .build(),
    );
    live_row.append(&live_switch);

    let brush_row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .build();
    brush_row.append(
        &Label::builder()
            .label("Brush")
            .halign(Align::Start)
            .css_classes(["dim-label", "caption"])
            .width_request(100)
            .build(),
    );
    brush_row.append(&brush_preview);

    let body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(4)
        .margin_end(4)
        .build();
    body.append(&info);
    body.append(&dialog::section_label("Brush"));
    body.append(&labeled_row("Hue", &hue_scale));
    body.append(&labeled_row("Saturation", &sat_scale));
    body.append(&labeled_row("Value", &val_scale));
    body.append(&brush_row);
    body.append(&live_row);
    body.append(&dialog::section_label("Patterns"));
    body.append(&pattern_row);
    body.append(&dialog::section_label("Paint board"));
    body.append(&board_scroll);
    body.append(&dialog::section_label("Timeline"));
    body.append(&frame_label);
    body.append(&timeline_row);
    body.append(&dialog::section_label("Playback"));
    body.append(&transport_row);
    body.append(&dialog::section_label("Presets"));
    body.append(&preset_row);

    let page = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .child(&body)
        .build();

    let update_brush_preview = {
        let hue_scale = hue_scale.clone();
        let sat_scale = sat_scale.clone();
        let val_scale = val_scale.clone();
        let brush_preview = brush_preview.clone();
        Rc::new(move || {
            let h = hue_scale.value().round() as u8;
            let s = sat_scale.value().round() as u8;
            let v = val_scale.value().round() as u8;
            let rgb = Rgb::from_hsv(h, s, v);
            set_swatch(&brush_preview, rgb.r, rgb.g, rgb.b);
        })
    };

    let brush_color = {
        let hue_scale = hue_scale.clone();
        let sat_scale = sat_scale.clone();
        let val_scale = val_scale.clone();
        Rc::new(move || {
            Rgb::from_hsv(
                hue_scale.value().round() as u8,
                sat_scale.value().round() as u8,
                val_scale.value().round() as u8,
            )
        })
    };

    let refresh_presets_dd = {
        let preset_dd = preset_dd.clone();
        Rc::new(move || {
            let names = via_studio::list_presets().unwrap_or_default();
            let labels: Vec<String> = if names.is_empty() {
                vec!["(no presets)".into()]
            } else {
                names
            };
            let list = StringList::new(&labels.iter().map(|s| s.as_str()).collect::<Vec<_>>());
            preset_dd.set_model(Some(&list));
            if !labels.is_empty() && labels[0] != "(no presets)" {
                preset_dd.set_selected(0);
            }
        })
    };

    let suppress_delay = Rc::new(Cell::new(false));

    let rebuild_board = {
        let board_fixed = board_fixed.clone();
        let state = Rc::clone(&state);
        let key_buttons = Rc::clone(&key_buttons);
        let get_vid_pid = Rc::clone(&get_vid_pid);
        let status = Rc::clone(&status);
        let brush_color = Rc::clone(&brush_color);
        let live_switch = live_switch.clone();
        let playing = Rc::clone(&playing);
        Rc::new(move || {
            while let Some(child) = board_fixed.first_child() {
                board_fixed.remove(&child);
            }
            key_buttons.borrow_mut().clear();

            let st = state.borrow();
            if st.keys.is_empty() {
                board_fixed.set_size_request(200, 80);
                return;
            }
            let painted = st
                .anim
                .frames
                .get(st.frame_idx)
                .map(|f| f.cell_map())
                .unwrap_or_default();
            let keys = st.keys.clone();
            let width = ((st.layout_w as f64) * (UNIT_PX + GAP_PX) + GAP_PX).ceil() as i32;
            let height = ((st.layout_h as f64) * (UNIT_PX + GAP_PX) + GAP_PX).ceil() as i32;
            let backend = st.backend;
            let layout_order = st.layout_order.clone();
            drop(st);

            board_fixed.set_size_request(width.max(200), height.max(80));

            for key in keys {
                let label = format!("{},{}", key.row, key.col);
                let btn = Button::builder()
                    .label(&label)
                    .tooltip_text(&format!("Paint matrix ({},{})", key.row, key.col))
                    .css_classes(["hyprbinds-via-key"])
                    .sensitive(backend.is_available() && !playing.get())
                    .build();
                if let Some(rgb) = painted.get(&(key.row, key.col)) {
                    tint_key(&btn, rgb.r, rgb.g, rgb.b);
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
                    let get_vid_pid = Rc::clone(&get_vid_pid);
                    let status = Rc::clone(&status);
                    let brush_color = Rc::clone(&brush_color);
                    let live_switch = live_switch.clone();
                    let playing = Rc::clone(&playing);
                    let layout_order = layout_order.clone();
                    move |_| {
                        if playing.get() {
                            return;
                        }
                        let rgb = brush_color();
                        {
                            let mut st = state.borrow_mut();
                            if !st.backend.is_available() {
                                status("No per-key paint backend on this keyboard".into());
                                return;
                            }
                            st.anim.ensure_frame();
                            let idx = st.frame_idx;
                            if let Some(frame) = st.anim.frame_mut(idx) {
                                frame.set_cell(row, col, rgb);
                            }
                        }
                        for (r, c, b) in key_buttons.borrow().iter() {
                            if (*r, *c) == (row, col) {
                                tint_key(b, rgb.r, rgb.g, rgb.b);
                            }
                        }
                        if live_switch.is_active() {
                            if let Some((vid, pid)) = get_vid_pid() {
                                let backend = state.borrow().backend;
                                if let Some(index) = via::led_index_for_key(
                                    vid,
                                    pid,
                                    row,
                                    col,
                                    Some(&layout_order),
                                ) {
                                    let brightness = state
                                        .borrow()
                                        .lighting
                                        .as_ref()
                                        .map(|l| l.brightness)
                                        .unwrap_or(255);
                                    let _ = via::enter_paint_mode(vid, pid, backend, brightness);
                                    match via::set_led_rgb(
                                        vid, pid, backend, index, rgb.r, rgb.g, rgb.b,
                                    ) {
                                        Ok(()) => status(format!(
                                            "Painted LED {index} ← ({row},{col})"
                                        )),
                                        Err(e) => status(format!("Paint failed: {e}")),
                                    }
                                }
                            }
                        }
                    }
                });
                key_buttons.borrow_mut().push((key.row, key.col, btn));
            }
        })
    };

    // Fix sync_frame_chrome to use the shared suppress_delay
    let sync_frame_chrome = {
        let state = Rc::clone(&state);
        let frame_label = frame_label.clone();
        let delay_spin = delay_spin.clone();
        let loop_check = loop_check.clone();
        let preset_name = preset_name.clone();
        let suppress_delay = Rc::clone(&suppress_delay);
        Rc::new(move || {
            let st = state.borrow();
            let n = st.anim.frames.len().max(1);
            let idx = st.frame_idx.min(n.saturating_sub(1)) + 1;
            frame_label.set_text(&format!("Frame {idx} / {n}"));
            let delay = st
                .anim
                .frames
                .get(st.frame_idx)
                .map(|f| f.delay_ms)
                .unwrap_or(DEFAULT_DELAY_MS);
            suppress_delay.set(true);
            delay_spin.set_value(delay as f64);
            suppress_delay.set(false);
            loop_check.set_active(st.anim.looped);
            let name = st.anim.name.clone();
            drop(st);
            if preset_name.text().as_str() != name {
                // only seed if empty-ish
                if preset_name.text().is_empty() {
                    preset_name.set_text(&name);
                }
            }
        })
    };

    let apply_pattern = {
        let state = Rc::clone(&state);
        let rebuild_board = Rc::clone(&rebuild_board);
        let get_vid_pid = Rc::clone(&get_vid_pid);
        let status = Rc::clone(&status);
        let live_switch = live_switch.clone();
        Rc::new(move |map: std::collections::HashMap<(u8, u8), Rgb>| {
            {
                let mut st = state.borrow_mut();
                st.anim.ensure_frame();
                let idx = st.frame_idx;
                if let Some(frame) = st.anim.frame_mut(idx) {
                    frame.apply_map(map);
                }
            }
            rebuild_board();
            if live_switch.is_active() {
                if let Some((vid, pid)) = get_vid_pid() {
                    let (backend, frame, order, brightness) = {
                        let st = state.borrow();
                        let frame = st
                            .anim
                            .frames
                            .get(st.frame_idx)
                            .cloned()
                            .unwrap_or_default();
                        (
                            st.backend,
                            frame,
                            st.layout_order.clone(),
                            st.lighting
                                .as_ref()
                                .map(|l| l.brightness)
                                .unwrap_or(255),
                        )
                    };
                    if backend.is_available() {
                        let _ = via::enter_paint_mode(vid, pid, backend, brightness);
                        match via_studio::push_frame(vid, pid, backend, &frame, &order) {
                            Ok(()) => status("Pattern applied to keyboard".into()),
                            Err(e) => status(format!("Pattern push failed: {e}")),
                        }
                    }
                }
            }
        })
    };

    let stop_playback = {
        let state = Rc::clone(&state);
        let playing = Rc::clone(&playing);
        let play_btn = play_btn.clone();
        let stop_btn = stop_btn.clone();
        let get_vid_pid = Rc::clone(&get_vid_pid);
        let status = Rc::clone(&status);
        let rebuild_board = Rc::clone(&rebuild_board);
        Rc::new(move || {
            if !playing.get() {
                return;
            }
            playing.set(false);
            play_btn.set_sensitive(true);
            stop_btn.set_sensitive(false);
            if let Some((vid, pid)) = get_vid_pid() {
                let snap = state.borrow().lighting.clone().unwrap_or(LightingSnapshot {
                    label: "Lighting".into(),
                    brightness: 255,
                    effect: 1,
                    speed: 128,
                    hue: 0,
                    saturation: 255,
                });
                match via::exit_paint_mode(vid, pid, &snap) {
                    Ok(out) => {
                        state.borrow_mut().lighting = Some(out);
                        status("Playback stopped · Solid Color restored".into());
                    }
                    Err(e) => status(format!("Stop restore failed: {e}")),
                }
            }
            rebuild_board();
        })
    };

    // Playback scheduler
    let schedule_playback: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));
    {
        let state = Rc::clone(&state);
        let playing = Rc::clone(&playing);
        let get_vid_pid = Rc::clone(&get_vid_pid);
        let status = Rc::clone(&status);
        let stop_playback = Rc::clone(&stop_playback);
        let sync_frame_chrome = Rc::clone(&sync_frame_chrome);
        let rebuild_board = Rc::clone(&rebuild_board);
        let schedule_playback = Rc::clone(&schedule_playback);
        let schedule_slot = Rc::clone(&schedule_playback);
        *schedule_slot.borrow_mut() = Some(Rc::new(move || {
            if !playing.get() {
                return;
            }
            let Some((vid, pid)) = get_vid_pid() else {
                stop_playback();
                status("No device for playback".into());
                return;
            };
            let (backend, frame, order, delay, next_idx, looped, n_frames) = {
                let st = state.borrow();
                let n = st.anim.frames.len();
                if n == 0 {
                    return;
                }
                let idx = st.frame_idx.min(n - 1);
                let frame = st.anim.frames[idx].clone();
                let delay = frame.delay_ms.max(16);
                let next = if idx + 1 < n {
                    idx + 1
                } else if st.anim.looped {
                    0
                } else {
                    usize::MAX
                };
                (
                    st.backend,
                    frame,
                    st.layout_order.clone(),
                    delay,
                    next,
                    st.anim.looped,
                    n,
                )
            };
            if let Err(e) = via_studio::push_frame(vid, pid, backend, &frame, &order) {
                status(format!("Playback frame failed: {e}"));
                stop_playback();
                return;
            }
            sync_frame_chrome();
            rebuild_board();

            if next_idx == usize::MAX {
                stop_playback();
                status(format!("Playback finished ({n_frames} frames)"));
                return;
            }
            state.borrow_mut().frame_idx = next_idx;
            let _ = looped; // used via next_idx logic

            let schedule_playback = Rc::clone(&schedule_playback);
            let playing = Rc::clone(&playing);
            glib::timeout_add_local(Duration::from_millis(delay as u64), move || {
                if playing.get() {
                    if let Some(f) = schedule_playback.borrow().as_ref() {
                        f();
                    }
                }
                glib::ControlFlow::Break
            });
        }));
    }

    // ── Signal handlers ───────────────────────────────────────────────────

    for scale in [&hue_scale, &sat_scale, &val_scale] {
        scale.connect_value_changed({
            let update_brush_preview = Rc::clone(&update_brush_preview);
            move |_| update_brush_preview()
        });
    }
    update_brush_preview();

    fill_btn.connect_clicked({
        let state = Rc::clone(&state);
        let brush_color = Rc::clone(&brush_color);
        let apply_pattern = Rc::clone(&apply_pattern);
        move |_| {
            let keys = state.borrow().layout_order.clone();
            apply_pattern(via_studio::pattern_fill(&keys, brush_color()));
        }
    });
    clear_btn.connect_clicked({
        let state = Rc::clone(&state);
        let apply_pattern = Rc::clone(&apply_pattern);
        move |_| {
            let keys = state.borrow().layout_order.clone();
            apply_pattern(via_studio::pattern_clear(&keys));
        }
    });
    checker_btn.connect_clicked({
        let state = Rc::clone(&state);
        let brush_color = Rc::clone(&brush_color);
        let apply_pattern = Rc::clone(&apply_pattern);
        move |_| {
            let keys = state.borrow().layout_order.clone();
            let a = brush_color();
            let b = Rgb::BLACK;
            apply_pattern(via_studio::pattern_checkerboard(&keys, a, b));
        }
    });
    gradient_btn.connect_clicked({
        let state = Rc::clone(&state);
        let sat_scale = sat_scale.clone();
        let val_scale = val_scale.clone();
        let apply_pattern = Rc::clone(&apply_pattern);
        move |_| {
            let keys = state.borrow().layout_order.clone();
            apply_pattern(via_studio::pattern_gradient_h(
                &keys,
                sat_scale.value().round() as u8,
                val_scale.value().round() as u8,
            ));
        }
    });
    rainbow_btn.connect_clicked({
        let state = Rc::clone(&state);
        let sat_scale = sat_scale.clone();
        let val_scale = val_scale.clone();
        let apply_pattern = Rc::clone(&apply_pattern);
        move |_| {
            let keys = state.borrow().layout_order.clone();
            apply_pattern(via_studio::pattern_rainbow_rows(
                &keys,
                sat_scale.value().round() as u8,
                val_scale.value().round() as u8,
            ));
        }
    });

    delay_spin.connect_value_changed({
        let state = Rc::clone(&state);
        let suppress_delay = Rc::clone(&suppress_delay);
        move |spin| {
            if suppress_delay.get() {
                return;
            }
            let mut st = state.borrow_mut();
            let idx = st.frame_idx;
            if let Some(frame) = st.anim.frame_mut(idx) {
                frame.delay_ms = spin.value().round().clamp(16.0, 10_000.0) as u32;
            }
        }
    });

    loop_check.connect_toggled({
        let state = Rc::clone(&state);
        move |c| {
            state.borrow_mut().anim.looped = c.is_active();
        }
    });

    prev_btn.connect_clicked({
        let state = Rc::clone(&state);
        let sync_frame_chrome = Rc::clone(&sync_frame_chrome);
        let rebuild_board = Rc::clone(&rebuild_board);
        move |_| {
            let mut st = state.borrow_mut();
            if st.frame_idx > 0 {
                st.frame_idx -= 1;
            }
            drop(st);
            sync_frame_chrome();
            rebuild_board();
        }
    });
    next_btn.connect_clicked({
        let state = Rc::clone(&state);
        let sync_frame_chrome = Rc::clone(&sync_frame_chrome);
        let rebuild_board = Rc::clone(&rebuild_board);
        move |_| {
            let mut st = state.borrow_mut();
            let n = st.anim.frames.len();
            if n > 0 && st.frame_idx + 1 < n {
                st.frame_idx += 1;
            }
            drop(st);
            sync_frame_chrome();
            rebuild_board();
        }
    });
    add_btn.connect_clicked({
        let state = Rc::clone(&state);
        let sync_frame_chrome = Rc::clone(&sync_frame_chrome);
        let rebuild_board = Rc::clone(&rebuild_board);
        move |_| {
            let mut st = state.borrow_mut();
            st.anim.frames.push(AnimFrame::default());
            st.frame_idx = st.anim.frames.len() - 1;
            drop(st);
            sync_frame_chrome();
            rebuild_board();
        }
    });
    dup_btn.connect_clicked({
        let state = Rc::clone(&state);
        let sync_frame_chrome = Rc::clone(&sync_frame_chrome);
        let rebuild_board = Rc::clone(&rebuild_board);
        move |_| {
            let mut st = state.borrow_mut();
            st.anim.ensure_frame();
            let idx = st.frame_idx;
            if let Some(frame) = st.anim.frames.get(idx).cloned() {
                st.anim.frames.insert(idx + 1, frame);
                st.frame_idx = idx + 1;
            }
            drop(st);
            sync_frame_chrome();
            rebuild_board();
        }
    });
    del_btn.connect_clicked({
        let state = Rc::clone(&state);
        let sync_frame_chrome = Rc::clone(&sync_frame_chrome);
        let rebuild_board = Rc::clone(&rebuild_board);
        let status = Rc::clone(&status);
        move |_| {
            let mut st = state.borrow_mut();
            if st.anim.frames.len() <= 1 {
                status("Need at least one frame".into());
                return;
            }
            let idx = st.frame_idx;
            st.anim.frames.remove(idx);
            if st.frame_idx >= st.anim.frames.len() {
                st.frame_idx = st.anim.frames.len() - 1;
            }
            drop(st);
            sync_frame_chrome();
            rebuild_board();
        }
    });

    play_btn.connect_clicked({
        let state = Rc::clone(&state);
        let playing = Rc::clone(&playing);
        let play_btn = play_btn.clone();
        let stop_btn = stop_btn.clone();
        let get_vid_pid = Rc::clone(&get_vid_pid);
        let status = Rc::clone(&status);
        let schedule_playback = Rc::clone(&schedule_playback);
        let rebuild_board = Rc::clone(&rebuild_board);
        move |_| {
            if playing.get() {
                return;
            }
            let Some((vid, pid)) = get_vid_pid() else {
                status("No VIA device selected".into());
                return;
            };
            let (backend, brightness) = {
                let st = state.borrow();
                (
                    st.backend,
                    st.lighting
                        .as_ref()
                        .map(|l| l.brightness)
                        .unwrap_or(255),
                )
            };
            if !backend.is_available() {
                status("No per-key paint backend".into());
                return;
            }
            {
                let mut st = state.borrow_mut();
                st.anim.ensure_frame();
                st.frame_idx = 0;
            }
            if let Err(e) = via::enter_paint_mode(vid, pid, backend, brightness) {
                status(format!("Enter paint mode failed: {e}"));
                return;
            }
            playing.set(true);
            play_btn.set_sensitive(false);
            stop_btn.set_sensitive(true);
            rebuild_board();
            status(format!("Playing via {}", backend.label()));
            if let Some(f) = schedule_playback.borrow().as_ref() {
                f();
            }
        }
    });
    stop_btn.connect_clicked({
        let stop_playback = Rc::clone(&stop_playback);
        move |_| stop_playback()
    });

    save_preset_btn.connect_clicked({
        let state = Rc::clone(&state);
        let preset_name = preset_name.clone();
        let status = Rc::clone(&status);
        let refresh_presets_dd = Rc::clone(&refresh_presets_dd);
        move |_| {
            let name = preset_name.text().to_string();
            if name.trim().is_empty() {
                status("Enter a preset name".into());
                return;
            }
            let mut st = state.borrow_mut();
            st.anim.name = name;
            st.anim.ensure_frame();
            match via_studio::save_preset(&st.anim) {
                Ok(path) => {
                    status(format!("Saved preset → {}", path.display()));
                    drop(st);
                    refresh_presets_dd();
                }
                Err(e) => status(format!("Save preset failed: {e}")),
            }
        }
    });
    load_preset_btn.connect_clicked({
        let state = Rc::clone(&state);
        let preset_dd = preset_dd.clone();
        let preset_name = preset_name.clone();
        let status = Rc::clone(&status);
        let sync_frame_chrome = Rc::clone(&sync_frame_chrome);
        let rebuild_board = Rc::clone(&rebuild_board);
        move |_| {
            let Some(obj) = preset_dd.selected_item() else {
                return;
            };
            let Some(s) = obj.downcast_ref::<gtk4::StringObject>() else {
                return;
            };
            let name = s.string().to_string();
            if name == "(no presets)" {
                return;
            }
            match via_studio::load_preset(&name) {
                Ok(anim) => {
                    preset_name.set_text(&anim.name);
                    let mut st = state.borrow_mut();
                    st.anim = anim;
                    st.frame_idx = 0;
                    drop(st);
                    sync_frame_chrome();
                    rebuild_board();
                    status(format!("Loaded preset “{name}”"));
                }
                Err(e) => status(format!("Load preset failed: {e}")),
            }
        }
    });
    delete_preset_btn.connect_clicked({
        let preset_dd = preset_dd.clone();
        let status = Rc::clone(&status);
        let refresh_presets_dd = Rc::clone(&refresh_presets_dd);
        move |_| {
            let Some(obj) = preset_dd.selected_item() else {
                return;
            };
            let Some(s) = obj.downcast_ref::<gtk4::StringObject>() else {
                return;
            };
            let name = s.string().to_string();
            if name == "(no presets)" {
                return;
            }
            match via_studio::delete_preset(&name) {
                Ok(()) => {
                    status(format!("Deleted preset “{name}”"));
                    refresh_presets_dd();
                }
                Err(e) => status(format!("Delete failed: {e}")),
            }
        }
    });

    refresh_presets_dd();
    sync_frame_chrome();

    let sync = {
        let state = Rc::clone(&state);
        let info = info.clone();
        let rebuild_board = Rc::clone(&rebuild_board);
        let sync_frame_chrome = Rc::clone(&sync_frame_chrome);
        let body = body.clone();
        Rc::new(
            move |def: Option<&ViaDefinition>,
                  backend: PaintBackend,
                  lighting: Option<LightingSnapshot>| {
                {
                    let mut st = state.borrow_mut();
                    st.backend = backend;
                    st.lighting = lighting;
                    if let Some(d) = def {
                        st.keys = d.layout.keys.clone();
                        st.layout_order = d.layout.keys.iter().map(|k| (k.row, k.col)).collect();
                        st.layout_w = d.layout.width();
                        st.layout_h = d.layout.height();
                    } else {
                        st.keys.clear();
                        st.layout_order.clear();
                        st.layout_w = 0.0;
                        st.layout_h = 0.0;
                    }
                }
                let available = backend.is_available();
                body.set_sensitive(available || def.is_some());
                if available {
                    info.set_text(&format!(
                        "Backend: {} — paint keys, build frames with delays, Play to stream, Save as preset.",
                        backend.label()
                    ));
                } else {
                    info.set_text(
                        "Per-key Studio needs OpenRGB-offset (R75 Remaster) or VialRGB DirectFastSet firmware.",
                    );
                }
                sync_frame_chrome();
                rebuild_board();
            },
        ) as Rc<dyn Fn(Option<&ViaDefinition>, PaintBackend, Option<LightingSnapshot>)>
    };

    StudioTab { page, sync }
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

fn labeled_inline(label: &str, widget: &impl IsA<gtk4::Widget>) -> GtkBox {
    let row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .build();
    row.append(
        &Label::builder()
            .label(label)
            .css_classes(["dim-label", "caption"])
            .build(),
    );
    row.append(widget);
    row
}

fn set_swatch(frame: &Frame, r: u8, g: u8, b: u8) {
    let css = format!(
        ".hyprbinds-via-color-swatch {{ background-color: rgb({r},{g},{b}); border-radius: 6px; border: 1px solid alpha(currentColor, 0.2); }}"
    );
    let provider = gtk4::CssProvider::new();
    provider.load_from_string(&css);
    frame
        .style_context()
        .add_provider(&provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
}

fn tint_key(btn: &Button, r: u8, g: u8, b: u8) {
    let name = format!("hyprbinds-via-paint-{r}-{g}-{b}");
    btn.add_css_class(&name);
    let css = format!(
        ".{name} {{ background-image: none; background-color: rgb({r},{g},{b}); color: {}; }}",
        if (r as u16) + (g as u16) + (b as u16) > 380 {
            "#111"
        } else {
            "#eee"
        }
    );
    let provider = gtk4::CssProvider::new();
    provider.load_from_string(&css);
    btn.style_context()
        .add_provider(&provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
}
