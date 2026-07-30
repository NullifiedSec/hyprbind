//! Audio page — PipeWire/Pulse sinks, sources, app streams, and ALSA controls.

use crate::audio::{self, AlsaControl, AudioSnapshot, StreamInfo};
use crate::debounce::Debouncer;
use crate::dialog;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, DropDown, Label, ListBox, ListBoxRow, Orientation,
    PolicyType, Scale, ScrolledWindow, StringList,
};
use std::cell::Cell;
use std::rc::Rc;

pub struct AudioPage {
    pub page: GtkBox,
}

fn volume_scale(initial: f64, max: f64) -> Scale {
    let adj = gtk4::Adjustment::new(initial, 0.0, max, 1.0, 5.0, 0.0);
    let scale = Scale::new(Orientation::Horizontal, Some(&adj));
    scale.set_draw_value(true);
    scale.set_hexpand(true);
    scale
}

fn section_block(title: &str, body: &impl IsA<gtk4::Widget>) -> GtkBox {
    let block = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .build();
    block.append(&dialog::section_label(title));
    block.append(body);
    block
}

fn mute_label(muted: bool) -> String {
    if muted {
        "Muted".into()
    } else {
        "Unmuted".into()
    }
}

pub fn build_audio_page(status: Rc<dyn Fn(String)>) -> AudioPage {
    let refresh_btn = Button::builder()
        .label("Refresh")
        .css_classes(["suggested-action"])
        .build();
    let mute_out_btn = Button::builder().label("Mute output").build();
    let mute_in_btn = Button::builder().label("Mute input").build();
    let alsamixer_btn = Button::builder().label("Open alsamixer").build();

    let toolbar = GtkBox::new(Orientation::Horizontal, 8);
    toolbar.append(&refresh_btn);
    toolbar.append(&mute_out_btn);
    toolbar.append(&mute_in_btn);
    toolbar.append(&alsamixer_btn);

    let backend_label = Label::builder()
        .label("Backend: …")
        .halign(gtk4::Align::Start)
        .css_classes(["dim-label", "caption"])
        .build();

    let sink_scale = volume_scale(50.0, 150.0);
    let sink_mute_lbl = Label::builder()
        .label("Unmuted")
        .halign(gtk4::Align::Start)
        .css_classes(["dim-label", "caption"])
        .build();
    let sink_dd = DropDown::from_strings(&["…"]);

    let output_body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .build();
    output_body.append(&sink_scale);
    output_body.append(&sink_mute_lbl);
    output_body.append(&sink_dd);

    let source_scale = volume_scale(50.0, 150.0);
    let source_mute_lbl = Label::builder()
        .label("Unmuted")
        .halign(gtk4::Align::Start)
        .css_classes(["dim-label", "caption"])
        .build();
    let source_dd = DropDown::from_strings(&["…"]);

    let input_body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .build();
    input_body.append(&source_scale);
    input_body.append(&source_mute_lbl);
    input_body.append(&source_dd);

    let apps_list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    let alsa_list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    let content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(16)
        .margin_top(4)
        .margin_bottom(12)
        .build();
    content.append(&backend_label);
    content.append(&section_block("Output", &output_body));
    content.append(&section_block("Input", &input_body));
    content.append(&section_block("Applications", &apps_list));
    content.append(&section_block("ALSA", &alsa_list));

    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .child(&content)
        .build();

    let page = dialog::page_shell(
        "Audio",
        "Control output and input volume via PipeWire/Pulse (pactl/wpctl). ALSA mixer controls are listed when amixer is available.",
        &toolbar,
        &scroll,
    );

    let filling = Rc::new(Cell::new(false));
    let sink_names = Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
    let source_names = Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
    let sink_debouncer = Debouncer::new();
    let source_debouncer = Debouncer::new();

    let refresh = {
        let backend_label = backend_label.clone();
        let sink_scale = sink_scale.clone();
        let sink_mute_lbl = sink_mute_lbl.clone();
        let sink_dd = sink_dd.clone();
        let source_scale = source_scale.clone();
        let source_mute_lbl = source_mute_lbl.clone();
        let source_dd = source_dd.clone();
        let apps_list = apps_list.clone();
        let alsa_list = alsa_list.clone();
        let filling = Rc::clone(&filling);
        let sink_names = Rc::clone(&sink_names);
        let source_names = Rc::clone(&source_names);
        let status = Rc::clone(&status);
        Rc::new(move || {
            match audio::snapshot() {
                Ok(snap) => fill_audio_ui(
                    &snap,
                    &backend_label,
                    &sink_scale,
                    &sink_mute_lbl,
                    &sink_dd,
                    &source_scale,
                    &source_mute_lbl,
                    &source_dd,
                    &apps_list,
                    &alsa_list,
                    &filling,
                    &sink_names,
                    &source_names,
                    Rc::clone(&status),
                ),
                Err(e) => status(format!("Audio snapshot failed: {e}")),
            }
        })
    };

    sink_scale.connect_value_changed({
        let filling = Rc::clone(&filling);
        let debouncer = sink_debouncer.clone();
        let status = Rc::clone(&status);
        let refresh = Rc::clone(&refresh);
        move |scale| {
            if filling.get() {
                return;
            }
            let pct = scale.value().round().clamp(0.0, 150.0) as u8;
            let status = Rc::clone(&status);
            let refresh = Rc::clone(&refresh);
            debouncer.schedule(move || {
                match audio::set_sink_volume(pct) {
                    Ok(()) => status(format!("Sink volume {pct}%")),
                    Err(e) => status(format!("Sink volume failed: {e}")),
                }
                refresh();
            });
        }
    });

    source_scale.connect_value_changed({
        let filling = Rc::clone(&filling);
        let debouncer = source_debouncer.clone();
        let status = Rc::clone(&status);
        let refresh = Rc::clone(&refresh);
        move |scale| {
            if filling.get() {
                return;
            }
            let pct = scale.value().round().clamp(0.0, 150.0) as u8;
            let status = Rc::clone(&status);
            let refresh = Rc::clone(&refresh);
            debouncer.schedule(move || {
                match audio::set_source_volume(pct) {
                    Ok(()) => status(format!("Source volume {pct}%")),
                    Err(e) => status(format!("Source volume failed: {e}")),
                }
                refresh();
            });
        }
    });

    sink_dd.connect_selected_notify({
        let filling = Rc::clone(&filling);
        let sink_names = Rc::clone(&sink_names);
        let status = Rc::clone(&status);
        let refresh = Rc::clone(&refresh);
        move |dd| {
            if filling.get() {
                return;
            }
            let idx = dd.selected() as usize;
            let name = sink_names.borrow().get(idx).cloned();
            if let Some(name) = name {
                match audio::set_default_sink(&name) {
                    Ok(()) => status(format!("Default sink: {name}")),
                    Err(e) => status(format!("Set default sink failed: {e}")),
                }
                refresh();
            }
        }
    });

    source_dd.connect_selected_notify({
        let filling = Rc::clone(&filling);
        let source_names = Rc::clone(&source_names);
        let status = Rc::clone(&status);
        let refresh = Rc::clone(&refresh);
        move |dd| {
            if filling.get() {
                return;
            }
            let idx = dd.selected() as usize;
            let name = source_names.borrow().get(idx).cloned();
            if let Some(name) = name {
                match audio::set_default_source(&name) {
                    Ok(()) => status(format!("Default source: {name}")),
                    Err(e) => status(format!("Set default source failed: {e}")),
                }
                refresh();
            }
        }
    });

    refresh_btn.connect_clicked({
        let refresh = Rc::clone(&refresh);
        move |_| refresh()
    });

    mute_out_btn.connect_clicked({
        let status = Rc::clone(&status);
        let refresh = Rc::clone(&refresh);
        move |_| match audio::toggle_sink_mute() {
            Ok(()) => {
                status("Toggled output mute".into());
                refresh();
            }
            Err(e) => status(format!("Mute output failed: {e}")),
        }
    });

    mute_in_btn.connect_clicked({
        let status = Rc::clone(&status);
        let refresh = Rc::clone(&refresh);
        move |_| match audio::toggle_source_mute() {
            Ok(()) => {
                status("Toggled input mute".into());
                refresh();
            }
            Err(e) => status(format!("Mute input failed: {e}")),
        }
    });

    alsamixer_btn.connect_clicked({
        let status = Rc::clone(&status);
        move |_| match audio::open_alsamixer() {
            Ok(msg) => status(msg),
            Err(e) => status(e),
        }
    });

    refresh();

    AudioPage { page }
}

fn fill_audio_ui(
    snap: &AudioSnapshot,
    backend_label: &Label,
    sink_scale: &Scale,
    sink_mute_lbl: &Label,
    sink_dd: &DropDown,
    source_scale: &Scale,
    source_mute_lbl: &Label,
    source_dd: &DropDown,
    apps_list: &ListBox,
    alsa_list: &ListBox,
    filling: &Cell<bool>,
    sink_names: &std::cell::RefCell<Vec<String>>,
    source_names: &std::cell::RefCell<Vec<String>>,
    status: Rc<dyn Fn(String)>,
) {
    filling.set(true);

    backend_label.set_text(&format!("Backend: {}", snap.backend));

    sink_scale
        .adjustment()
        .set_value(snap.sink_volume_pct as f64);
    sink_mute_lbl.set_text(&mute_label(snap.sink_muted));

    let sink_labels: Vec<String> = snap
        .sinks
        .iter()
        .map(|s| {
            if s.description != s.name {
                format!("{} ({})", s.description, s.name)
            } else {
                s.description.clone()
            }
        })
        .collect();
    *sink_names.borrow_mut() = snap.sinks.iter().map(|s| s.name.clone()).collect();
    let sink_refs: Vec<&str> = sink_labels.iter().map(|s| s.as_str()).collect();
    sink_dd.set_model(Some(&StringList::new(&sink_refs)));
    let sink_idx = snap
        .sinks
        .iter()
        .position(|s| s.name == snap.default_sink)
        .unwrap_or(0) as u32;
    if !snap.sinks.is_empty() {
        sink_dd.set_selected(sink_idx);
    }

    source_scale
        .adjustment()
        .set_value(snap.source_volume_pct as f64);
    source_mute_lbl.set_text(&mute_label(snap.source_muted));

    let source_labels: Vec<String> = snap
        .sources
        .iter()
        .map(|s| {
            if s.description != s.name {
                format!("{} ({})", s.description, s.name)
            } else {
                s.description.clone()
            }
        })
        .collect();
    *source_names.borrow_mut() = snap.sources.iter().map(|s| s.name.clone()).collect();
    let source_refs: Vec<&str> = source_labels.iter().map(|s| s.as_str()).collect();
    source_dd.set_model(Some(&StringList::new(&source_refs)));
    let source_idx = snap
        .sources
        .iter()
        .position(|s| s.name == snap.default_source)
        .unwrap_or(0) as u32;
    if !snap.sources.is_empty() {
        source_dd.set_selected(source_idx);
    }

    while let Some(child) = apps_list.first_child() {
        apps_list.remove(&child);
    }
    for stream in &snap.streams {
        apps_list.append(&stream_row(stream, Rc::clone(&status)));
    }
    if snap.streams.is_empty() {
        apps_list.append(&empty_row("No application streams"));
    }

    while let Some(child) = alsa_list.first_child() {
        alsa_list.remove(&child);
    }
    for ctrl in &snap.alsa {
        alsa_list.append(&alsa_row(ctrl, Rc::clone(&status)));
    }
    if snap.alsa.is_empty() {
        alsa_list.append(&empty_row("No ALSA controls (is amixer installed?)"));
    }

    filling.set(false);
}

fn empty_row(text: &str) -> ListBoxRow {
    let col = dialog::list_row_column();
    col.append(&dialog::row_body(text));
    ListBoxRow::builder()
        .child(&col)
        .activatable(false)
        .selectable(false)
        .build()
}

fn stream_row(stream: &StreamInfo, status: Rc<dyn Fn(String)>) -> ListBoxRow {
    let col = dialog::list_row_column();
    col.append(&dialog::row_title(&stream.name));
    if !stream.sink.is_empty() {
        col.append(&dialog::row_sub(&format!("sink: {}", stream.sink)));
    }

    let scale = volume_scale(stream.volume_pct as f64, 150.0);
    let mute_btn = Button::builder()
        .label(if stream.muted { "Unmute" } else { "Mute" })
        .halign(gtk4::Align::Start)
        .build();

    let stream_id = stream.id;
    let stream_name = stream.name.clone();
    let debouncer = Debouncer::new();
    let status_vol = Rc::clone(&status);
    scale.connect_value_changed(move |s| {
        let pct = s.value().round().clamp(0.0, 150.0) as u8;
        let status = Rc::clone(&status_vol);
        let name = stream_name.clone();
        debouncer.schedule(move || {
            match audio::set_stream_volume(stream_id, pct) {
                Ok(()) => status(format!("{name} volume {pct}%")),
                Err(e) => status(format!("Stream volume failed: {e}")),
            }
        });
    });

    let status_mute = Rc::clone(&status);
    let name = stream.name.clone();
    mute_btn.connect_clicked(move |_| {
        match audio::toggle_stream_mute(stream_id) {
            Ok(()) => status_mute(format!("Toggled mute for {name}")),
            Err(e) => status_mute(format!("Stream mute failed: {e}")),
        }
    });

    col.append(&scale);
    col.append(&mute_btn);

    ListBoxRow::builder()
        .child(&col)
        .activatable(false)
        .selectable(false)
        .build()
}

fn alsa_row(ctrl: &AlsaControl, status: Rc<dyn Fn(String)>) -> ListBoxRow {
    let col = dialog::list_row_column();
    col.append(&dialog::row_title(&ctrl.name));

    if let Some(pct) = ctrl.volume_pct {
        let scale = volume_scale(pct as f64, 100.0);
        let name = ctrl.name.clone();
        let debouncer = Debouncer::new();
        let status_vol = Rc::clone(&status);
        scale.connect_value_changed(move |s| {
            let v = s.value().round().clamp(0.0, 100.0) as u8;
            let status = Rc::clone(&status_vol);
            let name = name.clone();
            debouncer.schedule(move || {
                match audio::set_alsa_volume(&name, v) {
                    Ok(()) => status(format!("ALSA {name} {v}%")),
                    Err(e) => status(format!("ALSA volume failed: {e}")),
                }
            });
        });
        col.append(&scale);
    }

    if ctrl.muted.is_some() {
        let mute_btn = Button::builder()
            .label(if ctrl.muted == Some(true) {
                "Unmute"
            } else {
                "Mute"
            })
            .halign(gtk4::Align::Start)
            .build();
        let name = ctrl.name.clone();
        let status_mute = Rc::clone(&status);
        mute_btn.connect_clicked(move |_| match audio::toggle_alsa_mute(&name) {
            Ok(()) => status_mute(format!("Toggled ALSA {name}")),
            Err(e) => status_mute(format!("ALSA mute failed: {e}")),
        });
        col.append(&mute_btn);
    }

    ListBoxRow::builder()
        .child(&col)
        .activatable(false)
        .selectable(false)
        .build()
}
