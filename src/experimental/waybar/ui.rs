//! Waybar Studio page — preview, bar/layout/module/style/theme editors.

use crate::dialog;
use crate::experimental::waybar;
use crate::experimental::waybar::model::{ModuleOrigin, WaybarModel, Zone};
use crate::experimental::waybar::modules::{self, FieldKind};
use crate::experimental::waybar::style::{self, StyleTokens};
use crate::experimental::waybar::themes;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DropDown, Entry, Label, ListBox, ListBoxRow, Notebook,
    Orientation, PolicyType, ScrolledWindow, SpinButton, TextBuffer, TextView, Window,
};
use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct WaybarPage {
    pub page: GtkBox,
}

struct StudioState {
    model: WaybarModel,
    selected_module: Option<String>,
}

pub fn build_waybar_page(
    parent: &impl IsA<Window>,
    status: Rc<dyn Fn(String)>,
) -> WaybarPage {
    let parent_window = parent.clone().upcast::<Window>();
    let model = waybar::load().unwrap_or_else(|e| {
        let cfg = waybar::discover_config_path()
            .unwrap_or_else(|| std::path::PathBuf::from("~/.config/waybar/config.jsonc"));
        let style = waybar::style_path()
            .unwrap_or_else(|| std::path::PathBuf::from("~/.config/waybar/style.css"));
        let mut m = WaybarModel::empty(cfg, style);
        m.notes.push(format!("Load warning: {e}"));
        m
    });

    let state = Rc::new(RefCell::new(StudioState {
        selected_module: model.all_layout_modules().first().cloned(),
        model,
    }));

    let path_label = Label::builder()
        .halign(gtk4::Align::Start)
        .ellipsize(gtk4::pango::EllipsizeMode::Middle)
        .css_classes(["dim-label", "caption"])
        .build();

    let proc_label = Label::builder()
        .halign(gtk4::Align::Start)
        .css_classes(["caption"])
        .build();

    let note_label = Label::builder()
        .halign(gtk4::Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();

    let preview = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .hexpand(true)
        .css_classes(["hyprbinds-waybar-preview"])
        .height_request(42)
        .build();

    let apply_btn = Button::builder()
        .label("Apply")
        .css_classes(["suggested-action"])
        .build();
    let reload_btn = Button::builder().label("Reload").build();
    let restore_btn = Button::builder().label("Restore").build();
    let start_btn = Button::builder().label("Start").build();
    let stop_btn = Button::builder().label("Stop").build();
    let restart_btn = Button::builder().label("Restart").build();
    let refresh_btn = Button::builder().label("Re-read").build();

    let toolbar = GtkBox::new(Orientation::Horizontal, 8);
    toolbar.append(&apply_btn);
    toolbar.append(&reload_btn);
    toolbar.append(&restore_btn);
    toolbar.append(&start_btn);
    toolbar.append(&stop_btn);
    toolbar.append(&restart_btn);
    toolbar.append(&refresh_btn);

    // --- Tab bodies (owned containers we rebuild into) ---
    let bar_body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .build();
    let layout_body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .build();
    let modules_body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .hexpand(true)
        .vexpand(true)
        .build();
    let style_body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .build();
    let themes_body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .build();

    let notebook = Notebook::new();
    notebook.append_page(
        &wrap_scroll(&bar_body),
        Some(&Label::new(Some("Bar"))),
    );
    notebook.append_page(
        &wrap_scroll(&layout_body),
        Some(&Label::new(Some("Layout"))),
    );
    notebook.append_page(
        &wrap_scroll(&modules_body),
        Some(&Label::new(Some("Modules"))),
    );
    notebook.append_page(
        &wrap_scroll(&style_body),
        Some(&Label::new(Some("Style"))),
    );
    notebook.append_page(
        &wrap_scroll(&themes_body),
        Some(&Label::new(Some("Themes"))),
    );
    notebook.set_vexpand(true);
    notebook.set_hexpand(true);

    let refresh_ui: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));
    let refresh_preview: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));
    let refresh_busy = Rc::new(Cell::new(false));

    let do_preview = {
        let state = Rc::clone(&state);
        let preview = preview.clone();
        let path_label = path_label.clone();
        let proc_label = proc_label.clone();
        let note_label = note_label.clone();
        Rc::new(move || {
            let (path_cfg, path_style, notes, root_is_array) = {
                let st = state.borrow();
                (
                    st.model.config_path.display().to_string(),
                    st.model.style_path.display().to_string(),
                    st.model.notes.clone(),
                    st.model.root_is_array,
                )
            };
            path_label.set_label(&format!("{path_cfg}  ·  {path_style}"));
            proc_label.set_label(&format!(
                "Waybar: {} · binary: {}",
                waybar::status_label(),
                if waybar::command_exists("waybar") {
                    "found"
                } else {
                    "MISSING — install waybar"
                }
            ));
            let mut n = notes;
            if root_is_array {
                n.push("Editing bar [0] of a multi-bar config.".into());
            }
            note_label.set_label(&if n.is_empty() {
                String::new()
            } else {
                n.join(" · ")
            });
            rebuild_preview(&preview, &state);
        }) as Rc<dyn Fn()>
    };
    *refresh_preview.borrow_mut() = Some(Rc::clone(&do_preview));

    let do_refresh = {
        let state = Rc::clone(&state);
        let bar_body = bar_body.clone();
        let layout_body = layout_body.clone();
        let modules_body = modules_body.clone();
        let style_body = style_body.clone();
        let themes_body = themes_body.clone();
        let refresh_ui = Rc::clone(&refresh_ui);
        let refresh_preview = Rc::clone(&refresh_preview);
        let status = Rc::clone(&status);
        let do_preview = Rc::clone(&do_preview);
        let refresh_busy = Rc::clone(&refresh_busy);
        let parent_window = parent_window.clone();

        Rc::new(move || {
            if refresh_busy.get() {
                return;
            }
            refresh_busy.set(true);
            do_preview();
            rebuild_bar_tab(
                &bar_body,
                &state,
                &refresh_ui,
                &refresh_preview,
                &parent_window,
            );
            rebuild_layout_tab(&layout_body, &state, &refresh_ui, &status);
            rebuild_modules_tab(
                &modules_body,
                &state,
                &refresh_ui,
                &status,
                &parent_window,
            );
            rebuild_style_tab(&style_body, &state, &refresh_ui, &status, &parent_window);
            rebuild_themes_tab(&themes_body, &state, &refresh_ui, &status);
            refresh_busy.set(false);
        }) as Rc<dyn Fn()>
    };
    *refresh_ui.borrow_mut() = Some(Rc::clone(&do_refresh));
    do_refresh();

    // Toolbar actions
    {
        let state = Rc::clone(&state);
        let status = Rc::clone(&status);
        let do_refresh = Rc::clone(&do_refresh);
        apply_btn.connect_clicked(move |_| {
            let model = state.borrow().model.clone();
            match waybar::apply(&model) {
                Ok(msg) => status(msg),
                Err(e) => status(format!("Apply failed: {e}")),
            }
            do_refresh();
        });
    }
    {
        let status = Rc::clone(&status);
        let do_refresh = Rc::clone(&do_refresh);
        reload_btn.connect_clicked(move |_| {
            match waybar::reload() {
                Ok(msg) => status(msg),
                Err(e) => status(e.to_string()),
            }
            do_refresh();
        });
    }
    {
        let state = Rc::clone(&state);
        let status = Rc::clone(&status);
        let do_refresh = Rc::clone(&do_refresh);
        restore_btn.connect_clicked(move |_| {
            let model = state.borrow().model.clone();
            match waybar::restore_files(&model) {
                Ok(m) => status(m),
                Err(e) => status(e.to_string()),
            }
            match waybar::load() {
                Ok(m) => {
                    let mut st = state.borrow_mut();
                    st.selected_module = m.all_layout_modules().first().cloned();
                    st.model = m;
                }
                Err(e) => status(format!("Re-load after restore failed: {e}")),
            }
            do_refresh();
        });
    }
    {
        let status = Rc::clone(&status);
        let do_refresh = Rc::clone(&do_refresh);
        start_btn.connect_clicked(move |_| {
            match waybar::start() {
                Ok(msg) => status(msg),
                Err(e) => status(e.to_string()),
            }
            do_refresh();
        });
    }
    {
        let status = Rc::clone(&status);
        let do_refresh = Rc::clone(&do_refresh);
        stop_btn.connect_clicked(move |_| {
            match waybar::stop() {
                Ok(msg) => status(msg),
                Err(e) => status(e.to_string()),
            }
            do_refresh();
        });
    }
    {
        let status = Rc::clone(&status);
        let do_refresh = Rc::clone(&do_refresh);
        restart_btn.connect_clicked(move |_| {
            match waybar::restart() {
                Ok(msg) => status(msg),
                Err(e) => status(e.to_string()),
            }
            do_refresh();
        });
    }
    {
        let state = Rc::clone(&state);
        let status = Rc::clone(&status);
        let do_refresh = Rc::clone(&do_refresh);
        refresh_btn.connect_clicked(move |_| {
            match waybar::load() {
                Ok(m) => {
                    let mut st = state.borrow_mut();
                    st.selected_module = m.all_layout_modules().first().cloned();
                    st.model = m;
                    status("Re-read Waybar config from disk".into());
                }
                Err(e) => status(format!("Load failed: {e}")),
            }
            do_refresh();
        });
    }

    let meta = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .build();
    meta.append(&path_label);
    meta.append(&proc_label);
    meta.append(&note_label);

    let content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .vexpand(true)
        .build();
    content.append(&meta);
    content.append(&Label::builder()
        .label("Live preview")
        .halign(gtk4::Align::Start)
        .css_classes(["hyprbinds-settings-card-title"])
        .build());
    content.append(&preview);
    content.append(&notebook);

    let page = dialog::page_shell(
        "Waybar Studio",
        "Edit ~/.config/waybar config and style, preview modules, apply themes, and reload Waybar.",
        &toolbar,
        &content,
    );

    WaybarPage { page }
}

fn wrap_scroll(child: &impl IsA<gtk4::Widget>) -> ScrolledWindow {
    ScrolledWindow::builder()
        .child(child)
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .hexpand(true)
        .min_content_height(280)
        .build()
}

fn clear_box(b: &GtkBox) {
    while let Some(child) = b.first_child() {
        b.remove(&child);
    }
}

fn call_refresh(refresh_ui: &Rc<RefCell<Option<Rc<dyn Fn()>>>>) {
    // Defer so signal handlers can drop RefCell borrows before rebuild runs.
    let refresh_ui = Rc::clone(refresh_ui);
    gtk4::glib::idle_add_local_once(move || {
        if let Some(f) = refresh_ui.borrow().as_ref().cloned() {
            f();
        }
    });
}

fn call_preview(refresh_preview: &Rc<RefCell<Option<Rc<dyn Fn()>>>>) {
    let refresh_preview = Rc::clone(refresh_preview);
    gtk4::glib::idle_add_local_once(move || {
        if let Some(f) = refresh_preview.borrow().as_ref().cloned() {
            f();
        }
    });
}

fn rebuild_preview(preview: &GtkBox, state: &Rc<RefCell<StudioState>>) {
    clear_box(preview);
    let st = state.borrow();
    let tokens = super::style::extract_tokens(&st.model.style_css);

    let left = zone_preview_box("L", &st.model, Zone::Left, &tokens);
    let center = zone_preview_box("C", &st.model, Zone::Center, &tokens);
    let right = zone_preview_box("R", &st.model, Zone::Right, &tokens);

    left.set_hexpand(true);
    center.set_halign(gtk4::Align::Center);
    right.set_halign(gtk4::Align::End);
    right.set_hexpand(true);

    preview.append(&left);
    preview.append(&center);
    preview.append(&right);
}

fn zone_preview_box(
    _tag: &str,
    model: &WaybarModel,
    zone: Zone,
    tokens: &StyleTokens,
) -> GtkBox {
    let row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(4)
        .build();
    for m in model.zone_modules(zone) {
        let label = model.preview_label_for(&m);
        let pill = Label::builder()
            .label(&label)
            .css_classes(["hyprbinds-waybar-pill"])
            .tooltip_text(&m)
            .build();
        // Approximate colors via inline CSS provider is heavy; class is enough.
        let _ = tokens;
        row.append(&pill);
    }
    if model.zone_modules(zone).is_empty() {
        row.append(
            &Label::builder()
                .label("—")
                .css_classes(["dim-label"])
                .build(),
        );
    }
    row
}

fn settings_row(title: &str, subtitle: &str, control: &impl IsA<gtk4::Widget>) -> GtkBox {
    let text = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .valign(gtk4::Align::Center)
        .build();
    text.append(
        &Label::builder()
            .label(title)
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-settings-title"])
            .build(),
    );
    if !subtitle.is_empty() {
        text.append(
            &Label::builder()
                .label(subtitle)
                .halign(gtk4::Align::Start)
                .wrap(true)
                .xalign(0.0)
                .css_classes(["hyprbinds-settings-sub", "dim-label"])
                .build(),
        );
    }
    let row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(16)
        .css_classes(["hyprbinds-settings-row"])
        .build();
    row.append(&text);
    let control = control.clone().upcast::<gtk4::Widget>();
    control.set_valign(gtk4::Align::Center);
    control.set_halign(gtk4::Align::End);
    row.append(&control);
    row
}

fn settings_card(title: &str, rows: &[GtkBox]) -> GtkBox {
    let card = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .css_classes(["hyprbinds-settings-card"])
        .build();
    card.append(
        &Label::builder()
            .label(title)
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-settings-card-title"])
            .build(),
    );
    for r in rows {
        card.append(r);
    }
    card
}

fn rebuild_bar_tab(
    body: &GtkBox,
    state: &Rc<RefCell<StudioState>>,
    refresh_ui: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    refresh_preview: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    parent: &Window,
) {
    clear_box(body);
    let st = state.borrow();

    const POSITIONS: &[&str] = &["top", "bottom", "left", "right"];
    const LAYERS: &[&str] = &["top", "bottom", "overlay", "background"];
    const MODES: &[&str] = &["(default)", "dock", "hide", "invisible", "overlay"];
    const MODE_VALUES: &[&str] = &["", "dock", "hide", "invisible", "overlay"];

    let position = st.model.bar_string("position").unwrap_or_else(|| "top".into());
    let layer = st.model.bar_string("layer").unwrap_or_else(|| "top".into());
    let mode = st.model.bar_string("mode").unwrap_or_default();
    let height = st.model.bar_i64("height").unwrap_or(30);
    let spacing = st.model.bar_i64("spacing").unwrap_or(0);
    let margin_top = st.model.bar_i64("margin-top").unwrap_or(0);
    let margin_bottom = st.model.bar_i64("margin-bottom").unwrap_or(0);
    let margin_left = st.model.bar_i64("margin-left").unwrap_or(0);
    let margin_right = st.model.bar_i64("margin-right").unwrap_or(0);
    let exclusive = st.model.bar_bool("exclusive").unwrap_or(true);
    let reload_style = st
        .model
        .bar_bool("reload_style_on_change")
        .unwrap_or(false);
    let output = st.model.bar_string("output").unwrap_or_default();
    drop(st);

    let pos_idx = POSITIONS
        .iter()
        .position(|x| *x == position.as_str())
        .unwrap_or(0) as u32;
    let layer_idx = LAYERS
        .iter()
        .position(|x| *x == layer.as_str())
        .unwrap_or(0) as u32;
    let mode_idx = MODE_VALUES
        .iter()
        .position(|x| *x == mode.as_str())
        .unwrap_or(0) as u32;
    let mode_display = if mode.is_empty() {
        "(default)".to_string()
    } else {
        mode.clone()
    };

    let mut rows: Vec<GtkBox> = Vec::new();

    // Position
    {
        let (row, _, edit) =
            editable_value_row("Position", "Bar edge on the screen", &position, true);
        let state = Rc::clone(state);
        let refresh_ui = Rc::clone(refresh_ui);
        let refresh_preview = Rc::clone(refresh_preview);
        let parent = parent.clone();
        edit.connect_clicked(move |_| {
            let state = Rc::clone(&state);
            let refresh_ui = Rc::clone(&refresh_ui);
            let refresh_preview = Rc::clone(&refresh_preview);
            open_choice_edit_dialog(
                &parent,
                "Edit position",
                "Bar edge on the screen",
                POSITIONS,
                pos_idx,
                move |idx| {
                    if let Some(v) = POSITIONS.get(idx as usize) {
                        state
                            .borrow_mut()
                            .model
                            .set_bar_value("position", Value::String((*v).into()));
                        call_preview(&refresh_preview);
                        call_refresh(&refresh_ui);
                    }
                },
            );
        });
        rows.push(row);
    }

    // Layer
    {
        let (row, _, edit) =
            editable_value_row("Layer", "GTK layer-shell layer", &layer, true);
        let state = Rc::clone(state);
        let refresh_ui = Rc::clone(refresh_ui);
        let refresh_preview = Rc::clone(refresh_preview);
        let parent = parent.clone();
        edit.connect_clicked(move |_| {
            let state = Rc::clone(&state);
            let refresh_ui = Rc::clone(&refresh_ui);
            let refresh_preview = Rc::clone(&refresh_preview);
            open_choice_edit_dialog(
                &parent,
                "Edit layer",
                "GTK layer-shell layer",
                LAYERS,
                layer_idx,
                move |idx| {
                    if let Some(v) = LAYERS.get(idx as usize) {
                        state
                            .borrow_mut()
                            .model
                            .set_bar_value("layer", Value::String((*v).into()));
                        call_preview(&refresh_preview);
                        call_refresh(&refresh_ui);
                    }
                },
            );
        });
        rows.push(row);
    }

    // Mode
    {
        let (row, _, edit) =
            editable_value_row("Mode", "Optional sway-like bar mode", &mode_display, true);
        let state = Rc::clone(state);
        let refresh_ui = Rc::clone(refresh_ui);
        let refresh_preview = Rc::clone(refresh_preview);
        let parent = parent.clone();
        edit.connect_clicked(move |_| {
            let state = Rc::clone(&state);
            let refresh_ui = Rc::clone(&refresh_ui);
            let refresh_preview = Rc::clone(&refresh_preview);
            open_choice_edit_dialog(
                &parent,
                "Edit mode",
                "Optional sway-like bar mode. (default) removes the key.",
                MODES,
                mode_idx,
                move |idx| {
                    let v = MODE_VALUES.get(idx as usize).copied().unwrap_or("");
                    if v.is_empty() {
                        state.borrow_mut().model.bar.remove("mode");
                    } else {
                        state
                            .borrow_mut()
                            .model
                            .set_bar_value("mode", Value::String(v.into()));
                    }
                    call_preview(&refresh_preview);
                    call_refresh(&refresh_ui);
                },
            );
        });
        rows.push(row);
    }

    fn bar_int_row(
        title: &str,
        hint: &str,
        key: &'static str,
        value: i64,
        max: f64,
        state: &Rc<RefCell<StudioState>>,
        refresh_ui: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
        refresh_preview: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
        parent: &Window,
    ) -> GtkBox {
        let (row, _, edit) = editable_value_row(title, hint, &value.to_string(), true);
        let state = Rc::clone(state);
        let refresh_ui = Rc::clone(refresh_ui);
        let refresh_preview = Rc::clone(refresh_preview);
        let parent = parent.clone();
        let title = title.to_string();
        let hint = hint.to_string();
        edit.connect_clicked(move |_| {
            let initial = state.borrow().model.bar_i64(key).unwrap_or(value);
            let state = Rc::clone(&state);
            let refresh_ui = Rc::clone(&refresh_ui);
            let refresh_preview = Rc::clone(&refresh_preview);
            open_int_edit_dialog(
                &parent,
                &format!("Edit {title}"),
                &hint,
                initial as f64,
                0.0,
                max,
                move |v| {
                    state
                        .borrow_mut()
                        .model
                        .set_bar_value(key, Value::Number(v.into()));
                    call_preview(&refresh_preview);
                    call_refresh(&refresh_ui);
                },
            );
        });
        row
    }

    rows.push(bar_int_row(
        "Height",
        "Bar height in pixels",
        "height",
        height,
        128.0,
        state,
        refresh_ui,
        refresh_preview,
        parent,
    ));
    rows.push(bar_int_row(
        "Spacing",
        "Module spacing",
        "spacing",
        spacing,
        64.0,
        state,
        refresh_ui,
        refresh_preview,
        parent,
    ));
    rows.push(bar_int_row(
        "Margin top",
        "",
        "margin-top",
        margin_top,
        64.0,
        state,
        refresh_ui,
        refresh_preview,
        parent,
    ));
    rows.push(bar_int_row(
        "Margin bottom",
        "",
        "margin-bottom",
        margin_bottom,
        64.0,
        state,
        refresh_ui,
        refresh_preview,
        parent,
    ));
    rows.push(bar_int_row(
        "Margin left",
        "",
        "margin-left",
        margin_left,
        64.0,
        state,
        refresh_ui,
        refresh_preview,
        parent,
    ));
    rows.push(bar_int_row(
        "Margin right",
        "",
        "margin-right",
        margin_right,
        64.0,
        state,
        refresh_ui,
        refresh_preview,
        parent,
    ));

    // Exclusive
    {
        let (row, _, edit) = editable_value_row(
            "Exclusive",
            "Reserve screen space",
            if exclusive { "true" } else { "false" },
            true,
        );
        let state = Rc::clone(state);
        let refresh_ui = Rc::clone(refresh_ui);
        let refresh_preview = Rc::clone(refresh_preview);
        let parent = parent.clone();
        edit.connect_clicked(move |_| {
            let initial = state
                .borrow()
                .model
                .bar_bool("exclusive")
                .unwrap_or(true);
            let state = Rc::clone(&state);
            let refresh_ui = Rc::clone(&refresh_ui);
            let refresh_preview = Rc::clone(&refresh_preview);
            open_bool_edit_dialog(
                &parent,
                "Edit exclusive",
                "Reserve screen space",
                initial,
                move |active| {
                    state
                        .borrow_mut()
                        .model
                        .set_bar_value("exclusive", Value::Bool(active));
                    call_preview(&refresh_preview);
                    call_refresh(&refresh_ui);
                },
            );
        });
        rows.push(row);
    }

    // Reload style on change
    {
        let (row, _, edit) = editable_value_row(
            "Reload style on change",
            "Waybar watches style.css",
            if reload_style { "true" } else { "false" },
            true,
        );
        let state = Rc::clone(state);
        let refresh_ui = Rc::clone(refresh_ui);
        let refresh_preview = Rc::clone(refresh_preview);
        let parent = parent.clone();
        edit.connect_clicked(move |_| {
            let initial = state
                .borrow()
                .model
                .bar_bool("reload_style_on_change")
                .unwrap_or(false);
            let state = Rc::clone(&state);
            let refresh_ui = Rc::clone(&refresh_ui);
            let refresh_preview = Rc::clone(&refresh_preview);
            open_bool_edit_dialog(
                &parent,
                "Edit reload style on change",
                "Waybar watches style.css",
                initial,
                move |active| {
                    state.borrow_mut().model.set_bar_value(
                        "reload_style_on_change",
                        Value::Bool(active),
                    );
                    call_preview(&refresh_preview);
                    call_refresh(&refresh_ui);
                },
            );
        });
        rows.push(row);
    }

    // Output
    {
        let (row, _, edit) = editable_value_row(
            "Output",
            "Limit to one monitor (optional)",
            &output,
            true,
        );
        let state = Rc::clone(state);
        let refresh_ui = Rc::clone(refresh_ui);
        let parent = parent.clone();
        edit.connect_clicked(move |_| {
            let initial = state
                .borrow()
                .model
                .bar_string("output")
                .unwrap_or_default();
            let state = Rc::clone(&state);
            let refresh_ui = Rc::clone(&refresh_ui);
            open_text_edit_dialog(
                &parent,
                "Edit output",
                "Limit to one monitor. Leave blank to show on all.",
                &initial,
                "optional output name",
                move |value| {
                    {
                        let mut st = state.borrow_mut();
                        if value.trim().is_empty() {
                            st.model.bar.remove("output");
                        } else {
                            st.model
                                .set_bar_value("output", Value::String(value.trim().into()));
                        }
                    }
                    call_refresh(&refresh_ui);
                },
            );
        });
        rows.push(row);
    }

    body.append(&settings_card("Bar settings", &rows));
}

fn rebuild_layout_tab(
    body: &GtkBox,
    state: &Rc<RefCell<StudioState>>,
    refresh_ui: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    status: &Rc<dyn Fn(String)>,
) {
    clear_box(body);

    for zone in Zone::all() {
        body.append(&build_zone_editor(zone, state, refresh_ui, status));
    }

    // Add module controls
    let addables = super::modules::addable_modules();
    let labels: Vec<&str> = addables.iter().copied().collect();
    let add_dd = DropDown::from_strings(&labels);
    let zone_dd = DropDown::from_strings(&["Left", "Center", "Right"]);
    let add_btn = Button::builder()
        .label("Add module")
        .css_classes(["suggested-action"])
        .build();
    let custom_entry = Entry::builder()
        .placeholder_text("or custom id e.g. custom/foo")
        .width_chars(22)
        .build();

    {
        let state = Rc::clone(state);
        let refresh_ui = Rc::clone(refresh_ui);
        let status = Rc::clone(status);
        let add_dd = add_dd.clone();
        let zone_dd = zone_dd.clone();
        let custom_entry = custom_entry.clone();
        add_btn.connect_clicked(move |_| {
            let custom = custom_entry.text().to_string();
            let name = if !custom.trim().is_empty() {
                custom.trim().to_string()
            } else {
                let idx = add_dd.selected() as usize;
                addables
                    .get(idx)
                    .copied()
                    .unwrap_or("clock")
                    .to_string()
            };
            let zone = match zone_dd.selected() {
                1 => Zone::Center,
                2 => Zone::Right,
                _ => Zone::Left,
            };
            state.borrow_mut().model.add_module_to_zone(zone, &name);
            status(format!("Added {name} to {}", zone.label()));
            call_refresh(&refresh_ui);
        });
    }

    let add_row = GtkBox::new(Orientation::Horizontal, 8);
    add_row.append(&Label::new(Some("Add:")));
    add_row.append(&add_dd);
    add_row.append(&custom_entry);
    add_row.append(&Label::new(Some("to")));
    add_row.append(&zone_dd);
    add_row.append(&add_btn);
    body.append(&settings_card("Add module", &[add_row]));
}

fn build_zone_editor(
    zone: Zone,
    state: &Rc<RefCell<StudioState>>,
    refresh_ui: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    status: &Rc<dyn Fn(String)>,
) -> GtkBox {
    let card = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .css_classes(["hyprbinds-settings-card"])
        .build();
    card.append(
        &Label::builder()
            .label(&format!("{} modules", zone.label()))
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-settings-card-title"])
            .build(),
    );

    let list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    let modules = state.borrow().model.zone_modules(zone);
    for (idx, name) in modules.iter().enumerate() {
        let row = ListBoxRow::new();
        let box_row = GtkBox::new(Orientation::Horizontal, 8);
        box_row.set_margin_top(6);
        box_row.set_margin_bottom(6);
        box_row.set_margin_start(10);
        box_row.set_margin_end(10);

        let origin = state.borrow().model.module_origin(name);
        let suffix = match &origin {
            ModuleOrigin::Included(p) => {
                format!(
                    "  (include: {})",
                    p.file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("…")
                )
            }
            ModuleOrigin::Primary => String::new(),
        };
        let title = Label::builder()
            .label(&format!("{name}{suffix}"))
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();

        let up = Button::builder().label("↑").build();
        let down = Button::builder().label("↓").build();
        let to_l = Button::builder().label("⟵ L").build();
        let to_c = Button::builder().label("C").build();
        let to_r = Button::builder().label("R ⟶").build();
        let remove = Button::builder().label("Remove").build();

        {
            let state = Rc::clone(state);
            let refresh_ui = Rc::clone(refresh_ui);
            up.connect_clicked(move |_| {
                state.borrow_mut().model.move_module_in_zone(zone, idx, -1);
                call_refresh(&refresh_ui);
            });
        }
        {
            let state = Rc::clone(state);
            let refresh_ui = Rc::clone(refresh_ui);
            down.connect_clicked(move |_| {
                state.borrow_mut().model.move_module_in_zone(zone, idx, 1);
                call_refresh(&refresh_ui);
            });
        }
        for (btn, target) in [(&to_l, Zone::Left), (&to_c, Zone::Center), (&to_r, Zone::Right)]
        {
            let state = Rc::clone(state);
            let refresh_ui = Rc::clone(refresh_ui);
            let status = Rc::clone(status);
            let name = name.clone();
            btn.connect_clicked(move |_| {
                if target != zone {
                    state
                        .borrow_mut()
                        .model
                        .move_module_to_zone(&name, zone, target, None);
                    status(format!("Moved {name} to {}", target.label()));
                    call_refresh(&refresh_ui);
                }
            });
        }
        {
            let state = Rc::clone(state);
            let refresh_ui = Rc::clone(refresh_ui);
            let status = Rc::clone(status);
            let name = name.clone();
            remove.connect_clicked(move |_| {
                state.borrow_mut().model.remove_module_from_layout(&name);
                status(format!("Removed {name} from layout"));
                call_refresh(&refresh_ui);
            });
        }

        box_row.append(&title);
        box_row.append(&up);
        box_row.append(&down);
        box_row.append(&to_l);
        box_row.append(&to_c);
        box_row.append(&to_r);
        box_row.append(&remove);
        row.set_child(Some(&box_row));
        list.append(&row);
    }

    if modules.is_empty() {
        card.append(
            &Label::builder()
                .label("No modules in this zone")
                .css_classes(["dim-label"])
                .halign(gtk4::Align::Start)
                .build(),
        );
    } else {
        card.append(&list);
    }
    card
}

fn rebuild_modules_tab(
    body: &GtkBox,
    state: &Rc<RefCell<StudioState>>,
    refresh_ui: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    status: &Rc<dyn Fn(String)>,
    parent: &Window,
) {
    clear_box(body);

    let modules = {
        let mut m = state.borrow().model.all_layout_modules();
        // Also include configs present but not in layout (e.g. group children)
        for k in state.borrow().model.bar.keys() {
            if k.starts_with("modules-") || k == "include" || is_bar_option_key(k) {
                continue;
            }
            if looks_like_module_key(k) && !m.iter().any(|x| x == k) {
                m.push(k.clone());
            }
        }
        m.sort();
        m.dedup();
        m
    };

    if modules.is_empty() {
        body.append(&Label::new(Some("No modules — add some in Layout.")));
        return;
    }

    let labels: Vec<&str> = modules.iter().map(|s| s.as_str()).collect();
    let picker = DropDown::from_strings(&labels);
    if let Some(sel) = state.borrow().selected_module.as_ref() {
        if let Some(i) = modules.iter().position(|m| m == sel) {
            picker.set_selected(i as u32);
        }
    }

    {
        let state = Rc::clone(state);
        let refresh_ui = Rc::clone(refresh_ui);
        let modules = modules.clone();
        picker.connect_selected_notify(move |dd| {
            let idx = dd.selected() as usize;
            if let Some(name) = modules.get(idx).cloned() {
                let changed = {
                    let mut st = state.borrow_mut();
                    let changed = st.selected_module.as_ref() != Some(&name);
                    if changed {
                        st.selected_module = Some(name);
                    }
                    changed
                };
                if changed {
                    call_refresh(&refresh_ui);
                }
            }
        });
    }

    body.append(&settings_row(
        "Module",
        "Select a module to edit its config",
        &picker,
    ));

    let selected = state
        .borrow()
        .selected_module
        .clone()
        .or_else(|| modules.first().cloned());

    let Some(name) = selected else {
        return;
    };
    state.borrow_mut().selected_module = Some(name.clone());

    let readonly = state.borrow().model.is_module_readonly(&name);
    if readonly {
        let origin = state.borrow().model.module_origin(&name);
        let msg = match origin {
            ModuleOrigin::Included(p) => format!(
                "Read-only: defined in include {}. Layout can reference it; edit that file to change options.",
                p.display()
            ),
            ModuleOrigin::Primary => String::new(),
        };
        body.append(
            &Label::builder()
                .label(&msg)
                .wrap(true)
                .xalign(0.0)
                .css_classes(["dim-label"])
                .build(),
        );
    }

    let cfg = state
        .borrow()
        .model
        .module_config(&name)
        .cloned()
        .unwrap_or_else(|| super::modules::default_config_for(&name));

    let schema = super::modules::schema_for(&name);
    let form = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .css_classes(["hyprbinds-settings-card"])
        .build();
    form.append(
        &Label::builder()
            .label(&format!(
                "{} — {name}",
                schema
                    .as_ref()
                    .map(|s| s.label)
                    .unwrap_or("Module")
            ))
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-settings-card-title"])
            .build(),
    );

    if let Some(ref schema) = schema {
        form.append(
            &Label::builder()
                .label(&format!("Category: {}", schema.category))
                .halign(gtk4::Align::Start)
                .css_classes(["dim-label", "caption"])
                .build(),
        );
        for field in schema.fields {
            let row = build_field_row(
                &name,
                field,
                &cfg,
                readonly,
                state,
                refresh_ui,
                status,
                parent,
            );
            form.append(&row);
        }
    } else {
        form.append(
            &Label::builder()
                .label("No schema for this module — use raw JSON below.")
                .css_classes(["dim-label"])
                .build(),
        );
    }
    body.append(&form);

    if name == "hyprland/workspaces" && !readonly {
        body.append(&build_workspaces_icons_panel(
            state,
            refresh_ui,
            status,
            parent,
        ));
    }

    // Raw JSON editor
    let pretty = serde_json::to_string_pretty(&cfg).unwrap_or_else(|_| "{}".into());
    let preview = if pretty.len() > 48 {
        format!("{}…", pretty.chars().take(48).collect::<String>())
    } else {
        pretty.clone()
    };
    let (raw_row, _raw_value, raw_edit) = editable_value_row(
        "Raw JSON",
        "Full module object — Cancel discards, Save commits",
        &preview,
        !readonly,
    );
    {
        let state = Rc::clone(state);
        let refresh_ui = Rc::clone(refresh_ui);
        let status = Rc::clone(status);
        let parent = parent.clone();
        let name = name.clone();
        raw_edit.connect_clicked(move |_| {
            let initial = state
                .borrow()
                .model
                .module_config(&name)
                .map(|c| serde_json::to_string_pretty(c).unwrap_or_else(|_| "{}".into()))
                .unwrap_or_else(|| "{}".into());
            let state = Rc::clone(&state);
            let refresh_ui = Rc::clone(&refresh_ui);
            let status = Rc::clone(&status);
            let name = name.clone();
            open_json_edit_dialog(
                &parent,
                &format!("Edit raw JSON — {name}"),
                "Full module object. Cancel discards changes.",
                &initial,
                move |text| {
                    match serde_json::from_str::<Value>(&text) {
                        Ok(v) => {
                            let result = state.borrow_mut().model.set_module_config(&name, v);
                            match result {
                                Ok(()) => {
                                    status(format!("Updated {name} from raw JSON"));
                                    call_refresh(&refresh_ui);
                                }
                                Err(e) => status(e),
                            }
                        }
                        Err(e) => status(format!("Invalid JSON: {e}")),
                    }
                },
            );
        });
    }
    body.append(&settings_card("Advanced", &[raw_row]));
}

fn ensure_module_object<'a>(
    st: &'a mut StudioState,
    module: &str,
) -> Option<&'a mut serde_json::Map<String, Value>> {
    let cfg = st.model.bar.get_mut(module)?;
    if !cfg.is_object() {
        *cfg = Value::Object(serde_json::Map::new());
    }
    st.model.primary_keys.insert(module.to_string());
    cfg.as_object_mut()
}

fn format_icons_map(cfg: &Value) -> serde_json::Map<String, Value> {
    cfg.get("format-icons")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default()
}

fn icon_value(map: &serde_json::Map<String, Value>, key: &str) -> String {
    map.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn set_format_icon(state: &Rc<RefCell<StudioState>>, key: &str, value: &str) {
    let mut st = state.borrow_mut();
    let Some(obj) = ensure_module_object(&mut st, "hyprland/workspaces") else {
        return;
    };
    let icons = obj
        .entry("format-icons".to_string())
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    if !icons.is_object() {
        *icons = Value::Object(serde_json::Map::new());
    }
    if let Some(map) = icons.as_object_mut() {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            map.remove(key);
        } else {
            map.insert(key.to_string(), Value::String(trimmed.to_string()));
        }
    }
}

fn open_text_edit_dialog(
    parent: &Window,
    title: &str,
    hint: &str,
    initial: &str,
    placeholder: &str,
    on_save: impl Fn(String) + 'static,
) {
    let editor = Window::builder()
        .title(title)
        .transient_for(parent)
        .modal(true)
        .default_width(420)
        .default_height(220)
        .build();

    let entry = Entry::builder()
        .text(initial)
        .placeholder_text(placeholder)
        .hexpand(true)
        .build();
    let hint_lbl = Label::builder()
        .label(hint)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    let status = Label::builder()
        .label("")
        .wrap(true)
        .css_classes(["dim-label", "caption"])
        .build();

    let cancel = Button::builder().label("Cancel").build();
    let save = Button::builder()
        .label("Save")
        .css_classes(["suggested-action"])
        .build();
    let actions = dialog::action_buttons(&cancel, &save);

    let form = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(18)
        .margin_bottom(12)
        .margin_start(18)
        .margin_end(18)
        .build();
    form.append(&hint_lbl);
    form.append(&entry);
    dialog::mount_sticky_dialog(&editor, &form, Some(&status), &actions);

    cancel.connect_clicked({
        let editor = editor.clone();
        move |_| editor.close()
    });
    save.connect_clicked({
        let editor = editor.clone();
        let entry = entry.clone();
        move |_| {
            on_save(entry.text().to_string());
            editor.close();
        }
    });
    editor.present();
}

fn open_int_edit_dialog(
    parent: &Window,
    title: &str,
    hint: &str,
    initial: f64,
    min: f64,
    max: f64,
    on_save: impl Fn(i64) + 'static,
) {
    let editor = Window::builder()
        .title(title)
        .transient_for(parent)
        .modal(true)
        .default_width(360)
        .default_height(220)
        .build();

    let spin = SpinButton::with_range(min, max, 1.0);
    spin.set_value(initial);
    let hint_lbl = Label::builder()
        .label(hint)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    let status = Label::builder()
        .label("")
        .wrap(true)
        .css_classes(["dim-label", "caption"])
        .build();
    let cancel = Button::builder().label("Cancel").build();
    let save = Button::builder()
        .label("Save")
        .css_classes(["suggested-action"])
        .build();
    let actions = dialog::action_buttons(&cancel, &save);
    let form = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(18)
        .margin_bottom(12)
        .margin_start(18)
        .margin_end(18)
        .build();
    form.append(&hint_lbl);
    form.append(&spin);
    dialog::mount_sticky_dialog(&editor, &form, Some(&status), &actions);
    cancel.connect_clicked({
        let editor = editor.clone();
        move |_| editor.close()
    });
    save.connect_clicked({
        let editor = editor.clone();
        let spin = spin.clone();
        move |_| {
            on_save(spin.value() as i64);
            editor.close();
        }
    });
    editor.present();
}

fn open_choice_edit_dialog(
    parent: &Window,
    title: &str,
    hint: &str,
    labels: &[&str],
    selected: u32,
    on_save: impl Fn(u32) + 'static,
) {
    let editor = Window::builder()
        .title(title)
        .transient_for(parent)
        .modal(true)
        .default_width(420)
        .default_height(220)
        .build();

    let dd = DropDown::from_strings(labels);
    dd.set_selected(selected);
    let hint_lbl = Label::builder()
        .label(hint)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    let status = Label::builder()
        .label("")
        .wrap(true)
        .css_classes(["dim-label", "caption"])
        .build();
    let cancel = Button::builder().label("Cancel").build();
    let save = Button::builder()
        .label("Save")
        .css_classes(["suggested-action"])
        .build();
    let actions = dialog::action_buttons(&cancel, &save);
    let form = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(18)
        .margin_bottom(12)
        .margin_start(18)
        .margin_end(18)
        .build();
    form.append(&hint_lbl);
    form.append(&dd);
    dialog::mount_sticky_dialog(&editor, &form, Some(&status), &actions);
    cancel.connect_clicked({
        let editor = editor.clone();
        move |_| editor.close()
    });
    save.connect_clicked({
        let editor = editor.clone();
        let dd = dd.clone();
        move |_| {
            on_save(dd.selected());
            editor.close();
        }
    });
    editor.present();
}

fn open_bool_edit_dialog(
    parent: &Window,
    title: &str,
    hint: &str,
    initial: bool,
    on_save: impl Fn(bool) + 'static,
) {
    let editor = Window::builder()
        .title(title)
        .transient_for(parent)
        .modal(true)
        .default_width(360)
        .default_height(200)
        .build();

    let check = CheckButton::builder()
        .label(title)
        .active(initial)
        .build();
    let hint_lbl = Label::builder()
        .label(hint)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    let status = Label::builder()
        .label("")
        .wrap(true)
        .css_classes(["dim-label", "caption"])
        .build();
    let cancel = Button::builder().label("Cancel").build();
    let save = Button::builder()
        .label("Save")
        .css_classes(["suggested-action"])
        .build();
    let actions = dialog::action_buttons(&cancel, &save);
    let form = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(18)
        .margin_bottom(12)
        .margin_start(18)
        .margin_end(18)
        .build();
    form.append(&hint_lbl);
    form.append(&check);
    dialog::mount_sticky_dialog(&editor, &form, Some(&status), &actions);
    cancel.connect_clicked({
        let editor = editor.clone();
        move |_| editor.close()
    });
    save.connect_clicked({
        let editor = editor.clone();
        let check = check.clone();
        move |_| {
            on_save(check.is_active());
            editor.close();
        }
    });
    editor.present();
}

fn open_multiline_edit_dialog(
    parent: &Window,
    title: &str,
    hint: &str,
    initial: &str,
    monospace: bool,
    validate: Option<Rc<dyn Fn(&str) -> Result<(), String>>>,
    on_save: impl Fn(String) + 'static,
) {
    let editor = Window::builder()
        .title(title)
        .transient_for(parent)
        .modal(true)
        .default_width(560)
        .default_height(420)
        .build();

    let buffer = TextBuffer::builder().text(initial).build();
    let view = TextView::builder()
        .buffer(&buffer)
        .monospace(monospace)
        .wrap_mode(gtk4::WrapMode::WordChar)
        .hexpand(true)
        .vexpand(true)
        .build();
    let scroll = ScrolledWindow::builder()
        .child(&view)
        .vexpand(true)
        .min_content_height(220)
        .build();
    let hint_lbl = Label::builder()
        .label(hint)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    let status = Label::builder()
        .label("")
        .wrap(true)
        .css_classes(["dim-label", "caption"])
        .build();
    let cancel = Button::builder().label("Cancel").build();
    let save = Button::builder()
        .label("Save")
        .css_classes(["suggested-action"])
        .build();
    let actions = dialog::action_buttons(&cancel, &save);
    let form = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(18)
        .margin_bottom(12)
        .margin_start(18)
        .margin_end(18)
        .build();
    form.append(&hint_lbl);
    form.append(&scroll);
    dialog::mount_sticky_dialog(&editor, &form, Some(&status), &actions);
    cancel.connect_clicked({
        let editor = editor.clone();
        move |_| editor.close()
    });
    save.connect_clicked({
        let editor = editor.clone();
        let buffer = buffer.clone();
        let status = status.clone();
        move |_| {
            let (start, end) = buffer.bounds();
            let text = buffer.text(&start, &end, false).to_string();
            if let Some(ref validate) = validate {
                if let Err(err) = validate(&text) {
                    status.set_label(&err);
                    return;
                }
            }
            on_save(text);
            editor.close();
        }
    });
    editor.present();
}

fn open_json_edit_dialog(
    parent: &Window,
    title: &str,
    hint: &str,
    initial: &str,
    on_save: impl Fn(String) + 'static,
) {
    let validate: Rc<dyn Fn(&str) -> Result<(), String>> = Rc::new(|text: &str| {
        if text.trim().is_empty() {
            return Ok(());
        }
        serde_json::from_str::<Value>(text)
            .map(|_| ())
            .map_err(|err| format!("Invalid JSON: {err}"))
    });
    open_multiline_edit_dialog(parent, title, hint, initial, true, Some(validate), on_save);
}

fn open_color_pair_edit_dialog(
    parent: &Window,
    title: &str,
    hint: &str,
    fg_init: &str,
    bg_init: &str,
    on_save: impl Fn(String, String) + 'static,
) {
    let editor = Window::builder()
        .title(title)
        .transient_for(parent)
        .modal(true)
        .default_width(460)
        .default_height(280)
        .build();

    let fg = Entry::builder()
        .text(fg_init)
        .placeholder_text("#rrggbb / rgba()")
        .hexpand(true)
        .build();
    let bg = Entry::builder()
        .text(bg_init)
        .placeholder_text("transparent / rgba()")
        .hexpand(true)
        .build();
    let hint_lbl = Label::builder()
        .label(hint)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    let status = Label::builder()
        .label("")
        .wrap(true)
        .css_classes(["dim-label", "caption"])
        .build();

    let cancel = Button::builder().label("Cancel").build();
    let save = Button::builder()
        .label("Save")
        .css_classes(["suggested-action"])
        .build();
    let actions = dialog::action_buttons(&cancel, &save);

    let form = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(18)
        .margin_bottom(12)
        .margin_start(18)
        .margin_end(18)
        .build();
    form.append(&hint_lbl);
    form.append(
        &Label::builder()
            .label("Foreground")
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-settings-title"])
            .build(),
    );
    form.append(&fg);
    form.append(
        &Label::builder()
            .label("Background")
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-settings-title"])
            .build(),
    );
    form.append(&bg);
    dialog::mount_sticky_dialog(&editor, &form, Some(&status), &actions);

    cancel.connect_clicked({
        let editor = editor.clone();
        move |_| editor.close()
    });
    save.connect_clicked({
        let editor = editor.clone();
        let fg = fg.clone();
        let bg = bg.clone();
        move |_| {
            on_save(fg.text().to_string(), bg.text().to_string());
            editor.close();
        }
    });
    editor.present();
}

fn editable_value_row(
    title: &str,
    hint: &str,
    display: &str,
    edit_sensitive: bool,
) -> (GtkBox, Label, Button) {
    let value = Label::builder()
        .label(if display.is_empty() { "(empty)" } else { display })
        .halign(gtk4::Align::End)
        .ellipsize(gtk4::pango::EllipsizeMode::End)
        .max_width_chars(28)
        .selectable(true)
        .css_classes(["hyprbinds-settings-title"])
        .build();
    let edit = Button::builder()
        .label("Edit")
        .sensitive(edit_sensitive)
        .build();
    let controls = GtkBox::new(Orientation::Horizontal, 8);
    controls.append(&value);
    controls.append(&edit);
    (settings_row(title, hint, &controls), value, edit)
}

fn current_workspace_colors(state: &Rc<RefCell<StudioState>>) -> super::style::WorkspaceStateColors {
    super::style::extract_workspace_colors(&state.borrow().model.style_css)
}

fn save_workspace_colors(
    state: &Rc<RefCell<StudioState>>,
    colors: &super::style::WorkspaceStateColors,
    status: &Rc<dyn Fn(String)>,
) {
    {
        let mut st = state.borrow_mut();
        st.model.style_css = super::style::apply_workspace_colors(&st.model.style_css, colors);
    }
    let model = state.borrow().model.clone();
    match waybar::apply(&model) {
        Ok(msg) => status(format!("Saved workspace colors · {msg}")),
        Err(e) => status(format!("Colors updated but apply failed: {e}")),
    }
}

fn build_workspaces_icons_panel(
    state: &Rc<RefCell<StudioState>>,
    refresh_ui: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    status: &Rc<dyn Fn(String)>,
    parent: &Window,
) -> GtkBox {
    let cfg = state
        .borrow()
        .model
        .module_config("hyprland/workspaces")
        .cloned()
        .unwrap_or_else(|| super::modules::default_config_for("hyprland/workspaces"));
    let icons = format_icons_map(&cfg);

    let card = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .css_classes(["hyprbinds-settings-card"])
        .build();
    card.append(
        &Label::builder()
            .label("Workspace labels & icons")
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-settings-card-title"])
            .build(),
    );
    card.append(
        &Label::builder()
            .label("Shown when Format includes {icon}. Leave blank to remove an entry.")
            .halign(gtk4::Align::Start)
            .wrap(true)
            .xalign(0.0)
            .css_classes(["dim-label", "caption"])
            .build(),
    );

    // Format presets — {icon} is required for format-icons to appear.
    const PRESETS: &[(&str, &str)] = &[
        ("Custom", ""),
        ("Icons (use format-icons below)", "{icon}"),
        ("Id number (no icons)", "{id}"),
        ("Name (no icons)", "{name}"),
        ("Icon + id", "{icon} {id}"),
        ("Name + icon", "{name} {icon}"),
    ];
    let current_fmt = cfg
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("{icon}");
    let preset_idx = PRESETS
        .iter()
        .position(|(_, f)| *f == current_fmt)
        .unwrap_or(0);
    let preset_display = if PRESETS[preset_idx].1.is_empty() {
        format!("Custom ({current_fmt})")
    } else {
        PRESETS[preset_idx].0.to_string()
    };
    let (preset_row, _preset_value, preset_edit) = editable_value_row(
        "Format preset",
        "Must include {icon} for workspace labels/icons below to show on the bar",
        &preset_display,
        true,
    );
    {
        let state = Rc::clone(state);
        let status = Rc::clone(status);
        let refresh_ui = Rc::clone(refresh_ui);
        let parent = parent.clone();
        preset_edit.connect_clicked(move |_| {
            let labels: Vec<&str> = PRESETS.iter().map(|(l, _)| *l).collect();
            let state = Rc::clone(&state);
            let status = Rc::clone(&status);
            let refresh_ui = Rc::clone(&refresh_ui);
            open_choice_edit_dialog(
                &parent,
                "Edit format preset",
                "Must include {icon} for workspace labels/icons to show on the bar",
                &labels,
                preset_idx as u32,
                move |idx| {
                    let Some((label, fmt)) = PRESETS.get(idx as usize) else {
                        return;
                    };
                    if fmt.is_empty() {
                        status("Custom format — edit the Format field in the module schema".into());
                        return;
                    }
                    {
                        let mut st = state.borrow_mut();
                        if let Some(obj) = ensure_module_object(&mut st, "hyprland/workspaces") {
                            obj.insert("format".into(), Value::String((*fmt).into()));
                        }
                    }
                    if !fmt.contains("{icon}") {
                        status(format!(
                            "Format → {fmt} (note: format-icons are ignored without {{icon}})"
                        ));
                    } else {
                        status(format!("Format → {label}: {fmt}"));
                    }
                    call_refresh(&refresh_ui);
                },
            );
        });
    }
    card.append(&preset_row);

    // sort-by (valid Waybar values)
    const SORTS: &[&str] = &["number", "name", "id", "default", "special-centered"];
    let current_sort = cfg
        .get("sort-by")
        .and_then(|v| v.as_str())
        .unwrap_or("number");
    let sort_idx = SORTS
        .iter()
        .position(|s| *s == current_sort)
        .unwrap_or(0);
    let (sort_row, _sort_value, sort_edit) = editable_value_row(
        "Sort by",
        "Waybar sort-by (use number for 1,2,3…)",
        current_sort,
        true,
    );
    {
        let state = Rc::clone(state);
        let status = Rc::clone(status);
        let refresh_ui = Rc::clone(refresh_ui);
        let parent = parent.clone();
        sort_edit.connect_clicked(move |_| {
            let state = Rc::clone(&state);
            let status = Rc::clone(&status);
            let refresh_ui = Rc::clone(&refresh_ui);
            open_choice_edit_dialog(
                &parent,
                "Edit sort-by",
                "Waybar sort-by (use number for 1,2,3…)",
                SORTS,
                sort_idx as u32,
                move |idx| {
                    if let Some(v) = SORTS.get(idx as usize) {
                        {
                            let mut st = state.borrow_mut();
                            if let Some(obj) = ensure_module_object(&mut st, "hyprland/workspaces") {
                                obj.insert("sort-by".into(), Value::String((*v).into()));
                                obj.remove("sort-by-number");
                            }
                        }
                        status(format!("Sort by → {v}"));
                        call_refresh(&refresh_ui);
                    }
                },
            );
        });
    }
    card.append(&sort_row);

    // Persistent count
    let persistent = cfg
        .get("persistent-workspaces")
        .and_then(|v| v.get("*"))
        .and_then(|v| v.as_i64())
        .unwrap_or(5);
    let (persist_row, _persist_value, persist_edit) = editable_value_row(
        "Persistent count",
        "Always show this many workspaces on every monitor (*). 0 clears the option.",
        &persistent.to_string(),
        true,
    );
    {
        let state = Rc::clone(state);
        let status = Rc::clone(status);
        let refresh_ui = Rc::clone(refresh_ui);
        let parent = parent.clone();
        persist_edit.connect_clicked(move |_| {
            let state = Rc::clone(&state);
            let status = Rc::clone(&status);
            let refresh_ui = Rc::clone(&refresh_ui);
            open_int_edit_dialog(
                &parent,
                "Edit persistent count",
                "Always show this many workspaces on every monitor (*). Set 0 to clear.",
                persistent as f64,
                0.0,
                20.0,
                move |n| {
                    {
                        let mut st = state.borrow_mut();
                        if let Some(obj) = ensure_module_object(&mut st, "hyprland/workspaces") {
                            if n <= 0 {
                                obj.remove("persistent-workspaces");
                                status("Cleared persistent workspaces".into());
                            } else {
                                obj.insert(
                                    "persistent-workspaces".into(),
                                    serde_json::json!({ "*": n }),
                                );
                                status(format!("Persistent workspaces: {n}"));
                            }
                        }
                    }
                    call_refresh(&refresh_ui);
                },
            );
        });
    }
    card.append(&persist_row);

    // Numbered workspace labels 1–10
    let numbers = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .build();
    numbers.append(
        &Label::builder()
            .label("Workspace numbers (1–10)")
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-settings-title"])
            .build(),
    );
    for n in 1..=10 {
        let key = n.to_string();
        let current = icon_value(&icons, &key);
        let (row, _value, edit) = editable_value_row(
            &format!("Workspace {n}"),
            "Text/icon for this workspace id",
            &current,
            true,
        );
        {
            let state = Rc::clone(state);
            let status = Rc::clone(status);
            let refresh_ui = Rc::clone(refresh_ui);
            let parent = parent.clone();
            let key = key.clone();
            edit.connect_clicked(move |_| {
                let state = Rc::clone(&state);
                let status = Rc::clone(&status);
                let refresh_ui = Rc::clone(&refresh_ui);
                let key = key.clone();
                let initial = {
                    let cfg = state
                        .borrow()
                        .model
                        .module_config("hyprland/workspaces")
                        .cloned()
                        .unwrap_or_else(|| {
                            super::modules::default_config_for("hyprland/workspaces")
                        });
                    icon_value(&format_icons_map(&cfg), &key)
                };
                open_text_edit_dialog(
                    &parent,
                    &format!("Edit workspace {key} icon"),
                    "Text/icon for this workspace id. Leave blank to remove.",
                    &initial,
                    &format!("label for {key}"),
                    move |value| {
                        set_format_icon(&state, &key, &value);
                        status(format!("Workspace {key} icon saved"));
                        call_refresh(&refresh_ui);
                    },
                );
            });
        }
        numbers.append(&row);
    }
    card.append(&numbers);

    // State icons
    let states = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .build();
    states.append(
        &Label::builder()
            .label("State icons")
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-settings-title"])
            .build(),
    );
    for (key, title, hint, fallback) in [
        ("default", "Default", "Inactive / fallback icon", "○"),
        ("active", "Active", "Currently focused workspace", "●"),
        ("urgent", "Urgent", "Workspace with urgent window", "!"),
        ("empty", "Empty", "Empty persistent workspace (optional)", ""),
    ] {
        let initial = {
            let v = icon_value(&icons, key);
            if v.is_empty() {
                fallback.to_string()
            } else {
                v
            }
        };
        let (row, _value, edit) = editable_value_row(title, hint, &initial, true);
        {
            let state = Rc::clone(state);
            let status = Rc::clone(status);
            let refresh_ui = Rc::clone(refresh_ui);
            let parent = parent.clone();
            let key = key.to_string();
            let title = title.to_string();
            let hint = hint.to_string();
            let fallback = fallback.to_string();
            edit.connect_clicked(move |_| {
                let state = Rc::clone(&state);
                let status = Rc::clone(&status);
                let refresh_ui = Rc::clone(&refresh_ui);
                let key = key.clone();
                let initial = {
                    let cfg = state
                        .borrow()
                        .model
                        .module_config("hyprland/workspaces")
                        .cloned()
                        .unwrap_or_else(|| {
                            super::modules::default_config_for("hyprland/workspaces")
                        });
                    let v = icon_value(&format_icons_map(&cfg), &key);
                    if v.is_empty() {
                        fallback.clone()
                    } else {
                        v
                    }
                };
                open_text_edit_dialog(
                    &parent,
                    &format!("Edit {title} icon"),
                    &hint,
                    &initial,
                    &fallback,
                    {
                        let title = title.clone();
                        move |value| {
                            set_format_icon(&state, &key, &value);
                            status(format!("{title} icon saved"));
                            call_refresh(&refresh_ui);
                        }
                    },
                );
            });
        }
        states.append(&row);
    }
    card.append(&states);

    // State colors (CSS — foreground / background)
    let ws_colors = current_workspace_colors(state);
    let colors_box = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .build();
    colors_box.append(
        &Label::builder()
            .label("State colors (CSS)")
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-settings-title"])
            .build(),
    );
    colors_box.append(
        &Label::builder()
            .label("Each color pair has Edit → Cancel/Save. Save writes CSS and restarts Waybar.")
            .halign(gtk4::Align::Start)
            .wrap(true)
            .xalign(0.0)
            .css_classes(["dim-label", "caption"])
            .build(),
    );

    fn color_display(fg: &str, bg: &str) -> String {
        format!("FG {fg}  ·  BG {bg}")
    }

    for (title, hint, fg_key, bg_key) in [
        (
            "Default",
            "Inactive workspace button",
            "default_fg",
            "default_bg",
        ),
        ("Hover", "Pointer over a workspace", "hover_fg", "hover_bg"),
        ("Active", "Focused workspace", "active_fg", "active_bg"),
        (
            "Urgent",
            "Workspace with urgent window",
            "urgent_fg",
            "urgent_bg",
        ),
        (
            "Empty",
            "Empty persistent workspace",
            "empty_fg",
            "empty_bg",
        ),
        (
            "Visible",
            "Visible on another monitor (not focused)",
            "visible_fg",
            "visible_bg",
        ),
    ] {
        let (fg_init, bg_init) = match fg_key {
            "default_fg" => (ws_colors.default_fg.clone(), ws_colors.default_bg.clone()),
            "hover_fg" => (ws_colors.hover_fg.clone(), ws_colors.hover_bg.clone()),
            "active_fg" => (ws_colors.active_fg.clone(), ws_colors.active_bg.clone()),
            "urgent_fg" => (ws_colors.urgent_fg.clone(), ws_colors.urgent_bg.clone()),
            "empty_fg" => (ws_colors.empty_fg.clone(), ws_colors.empty_bg.clone()),
            _ => (ws_colors.visible_fg.clone(), ws_colors.visible_bg.clone()),
        };
        let (row, _value, edit) = editable_value_row(
            title,
            hint,
            &color_display(&fg_init, &bg_init),
            true,
        );
        {
            let state = Rc::clone(state);
            let status = Rc::clone(status);
            let refresh_ui = Rc::clone(refresh_ui);
            let parent = parent.clone();
            let title = title.to_string();
            let hint = hint.to_string();
            let fg_key = fg_key.to_string();
            edit.connect_clicked(move |_| {
                let colors = current_workspace_colors(&state);
                let (fg, bg) = match fg_key.as_str() {
                    "default_fg" => (colors.default_fg, colors.default_bg),
                    "hover_fg" => (colors.hover_fg, colors.hover_bg),
                    "active_fg" => (colors.active_fg, colors.active_bg),
                    "urgent_fg" => (colors.urgent_fg, colors.urgent_bg),
                    "empty_fg" => (colors.empty_fg, colors.empty_bg),
                    _ => (colors.visible_fg, colors.visible_bg),
                };
                let state = Rc::clone(&state);
                let status = Rc::clone(&status);
                let refresh_ui = Rc::clone(&refresh_ui);
                let fg_key = fg_key.clone();
                open_color_pair_edit_dialog(
                    &parent,
                    &format!("Edit {title} colors"),
                    &hint,
                    &fg,
                    &bg,
                    move |fg, bg| {
                        let mut colors = current_workspace_colors(&state);
                        match fg_key.as_str() {
                            "default_fg" => {
                                colors.default_fg = fg;
                                colors.default_bg = bg;
                            }
                            "hover_fg" => {
                                colors.hover_fg = fg;
                                colors.hover_bg = bg;
                            }
                            "active_fg" => {
                                colors.active_fg = fg;
                                colors.active_bg = bg;
                            }
                            "urgent_fg" => {
                                colors.urgent_fg = fg;
                                colors.urgent_bg = bg;
                            }
                            "empty_fg" => {
                                colors.empty_fg = fg;
                                colors.empty_bg = bg;
                            }
                            _ => {
                                colors.visible_fg = fg;
                                colors.visible_bg = bg;
                            }
                        }
                        save_workspace_colors(&state, &colors, &status);
                        call_refresh(&refresh_ui);
                    },
                );
            });
        }
        colors_box.append(&row);
        let _ = bg_key;
    }

    let reset_colors = Button::builder().label("Reset color defaults").build();
    {
        let state = Rc::clone(state);
        let status = Rc::clone(status);
        let refresh_ui = Rc::clone(refresh_ui);
        reset_colors.connect_clicked(move |_| {
            let colors = super::style::WorkspaceStateColors::defaults();
            save_workspace_colors(&state, &colors, &status);
            call_refresh(&refresh_ui);
        });
    }
    colors_box.append(&reset_colors);
    card.append(&colors_box);

    // Quick fill buttons
    let actions = GtkBox::new(Orientation::Horizontal, 8);
    let fill_numbers = Button::builder().label("Fill 1–10 as numbers").build();
    let fill_dots = Button::builder().label("Use ● / ○ / !").build();
    let fill_circles = Button::builder().label("Use ➊–➓").build();
    {
        let state = Rc::clone(state);
        let status = Rc::clone(status);
        let refresh_ui = Rc::clone(refresh_ui);
        fill_dots.connect_clicked(move |_| {
            // State-only icons: clear numbered overrides so active/default apply.
            {
                let mut st = state.borrow_mut();
                if let Some(obj) = ensure_module_object(&mut st, "hyprland/workspaces") {
                    obj.insert("format".into(), Value::String("{icon}".into()));
                    let mut map = serde_json::Map::new();
                    map.insert("default".into(), Value::String("○".into()));
                    map.insert("active".into(), Value::String("●".into()));
                    map.insert("urgent".into(), Value::String("!".into()));
                    map.insert("empty".into(), Value::String("○".into()));
                    obj.insert("format-icons".into(), Value::Object(map));
                }
            }
            status("Dots mode: format={icon}, only state icons (●/○/!)".into());
            call_refresh(&refresh_ui);
        });
    }
    {
        let state = Rc::clone(state);
        let status = Rc::clone(status);
        let refresh_ui = Rc::clone(refresh_ui);
        fill_numbers.connect_clicked(move |_| {
            {
                let mut st = state.borrow_mut();
                if let Some(obj) = ensure_module_object(&mut st, "hyprland/workspaces") {
                    obj.insert("format".into(), Value::String("{icon}".into()));
                }
            }
            for n in 1..=10 {
                set_format_icon(&state, &n.to_string(), &n.to_string());
            }
            set_format_icon(&state, "default", "○");
            set_format_icon(&state, "active", "●");
            set_format_icon(&state, "urgent", "!");
            status("Numbers mode: format={icon} with digit icons 1–10".into());
            call_refresh(&refresh_ui);
        });
    }
    {
        let state = Rc::clone(state);
        let status = Rc::clone(status);
        let refresh_ui = Rc::clone(refresh_ui);
        fill_circles.connect_clicked(move |_| {
            {
                let mut st = state.borrow_mut();
                if let Some(obj) = ensure_module_object(&mut st, "hyprland/workspaces") {
                    obj.insert("format".into(), Value::String("{icon}".into()));
                }
            }
            const CIRCLED: &[&str] = &["➊", "➋", "➌", "➍", "➎", "➏", "➐", "➑", "➒", "➓"];
            for (i, glyph) in CIRCLED.iter().enumerate() {
                set_format_icon(&state, &(i + 1).to_string(), glyph);
            }
            status("Circled digits mode: format={icon}".into());
            call_refresh(&refresh_ui);
        });
    }
    actions.append(&fill_numbers);
    actions.append(&fill_dots);
    actions.append(&fill_circles);
    card.append(&actions);

    card
}

fn build_field_row(
    module: &str,
    field: &super::modules::ModuleField,
    cfg: &Value,
    readonly: bool,
    state: &Rc<RefCell<StudioState>>,
    refresh_ui: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    status: &Rc<dyn Fn(String)>,
    parent: &Window,
) -> GtkBox {
    let key = field.key;
    let module = module.to_string();
    let label = field.label.to_string();
    let hint = field.hint.to_string();

    match field.kind {
        FieldKind::String => {
            let current = cfg
                .get(key)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let (row, _value, edit) =
                editable_value_row(&label, &hint, &current, !readonly);
            {
                let state = Rc::clone(state);
                let status = Rc::clone(status);
                let refresh_ui = Rc::clone(refresh_ui);
                let parent = parent.clone();
                let module = module.clone();
                let label = label.clone();
                let hint = hint.clone();
                edit.connect_clicked(move |_| {
                    let initial = state
                        .borrow()
                        .model
                        .module_config(&module)
                        .and_then(|c| c.get(key).and_then(|v| v.as_str().map(|s| s.to_string())))
                        .unwrap_or_default();
                    let state = Rc::clone(&state);
                    let status = Rc::clone(&status);
                    let refresh_ui = Rc::clone(&refresh_ui);
                    let module = module.clone();
                    open_text_edit_dialog(
                        &parent,
                        &format!("Edit {label}"),
                        &hint,
                        &initial,
                        "",
                        move |value| {
                            {
                                let mut st = state.borrow_mut();
                                let Some(cfg) = st.model.bar.get_mut(&module) else {
                                    return;
                                };
                                if !cfg.is_object() {
                                    *cfg = Value::Object(serde_json::Map::new());
                                }
                                if let Some(obj) = cfg.as_object_mut() {
                                    if value.trim().is_empty() {
                                        obj.remove(key);
                                    } else {
                                        obj.insert(key.into(), Value::String(value));
                                    }
                                    st.model.primary_keys.insert(module.clone());
                                }
                            }
                            status(format!("Saved {module}.{key}"));
                            call_refresh(&refresh_ui);
                        },
                    );
                });
            }
            row
        }
        FieldKind::Int => {
            let current = cfg.get(key).and_then(|v| v.as_i64()).unwrap_or(0);
            let (row, _value, edit) =
                editable_value_row(&label, &hint, &current.to_string(), !readonly);
            {
                let state = Rc::clone(state);
                let status = Rc::clone(status);
                let refresh_ui = Rc::clone(refresh_ui);
                let parent = parent.clone();
                let module = module.clone();
                let label = label.clone();
                let hint = hint.clone();
                edit.connect_clicked(move |_| {
                    let initial = state
                        .borrow()
                        .model
                        .module_config(&module)
                        .and_then(|c| c.get(key).and_then(|v| v.as_i64()))
                        .unwrap_or(0);
                    let state = Rc::clone(&state);
                    let status = Rc::clone(&status);
                    let refresh_ui = Rc::clone(&refresh_ui);
                    let module = module.clone();
                    open_int_edit_dialog(
                        &parent,
                        &format!("Edit {label}"),
                        &hint,
                        initial as f64,
                        0.0,
                        99999.0,
                        move |v| {
                            {
                                let mut st = state.borrow_mut();
                                let Some(cfg) = st.model.bar.get_mut(&module) else {
                                    return;
                                };
                                if !cfg.is_object() {
                                    *cfg = Value::Object(serde_json::Map::new());
                                }
                                if let Some(obj) = cfg.as_object_mut() {
                                    obj.insert(key.into(), Value::Number(v.into()));
                                    st.model.primary_keys.insert(module.clone());
                                }
                            }
                            status(format!("Saved {module}.{key} = {v}"));
                            call_refresh(&refresh_ui);
                        },
                    );
                });
            }
            row
        }
        FieldKind::Bool => {
            let current = cfg.get(key).and_then(|v| v.as_bool()).unwrap_or(false);
            let (row, _value, edit) = editable_value_row(
                &label,
                &hint,
                if current { "true" } else { "false" },
                !readonly,
            );
            {
                let state = Rc::clone(state);
                let status = Rc::clone(status);
                let refresh_ui = Rc::clone(refresh_ui);
                let parent = parent.clone();
                let module = module.clone();
                let label = label.clone();
                let hint = hint.clone();
                edit.connect_clicked(move |_| {
                    let initial = state
                        .borrow()
                        .model
                        .module_config(&module)
                        .and_then(|c| c.get(key).and_then(|v| v.as_bool()))
                        .unwrap_or(false);
                    let state = Rc::clone(&state);
                    let status = Rc::clone(&status);
                    let refresh_ui = Rc::clone(&refresh_ui);
                    let module = module.clone();
                    open_bool_edit_dialog(
                        &parent,
                        &format!("Edit {label}"),
                        &hint,
                        initial,
                        move |active| {
                            {
                                let mut st = state.borrow_mut();
                                let Some(cfg) = st.model.bar.get_mut(&module) else {
                                    return;
                                };
                                if !cfg.is_object() {
                                    *cfg = Value::Object(serde_json::Map::new());
                                }
                                if let Some(obj) = cfg.as_object_mut() {
                                    obj.insert(key.into(), Value::Bool(active));
                                    st.model.primary_keys.insert(module.clone());
                                }
                            }
                            status(format!("Saved {module}.{key} = {active}"));
                            call_refresh(&refresh_ui);
                        },
                    );
                });
            }
            row
        }
        FieldKind::Json => {
            let text = cfg
                .get(key)
                .map(|v| serde_json::to_string_pretty(v).unwrap_or_default())
                .unwrap_or_default();
            let display = if text.is_empty() {
                "(empty)".to_string()
            } else {
                text.chars().take(40).collect::<String>()
                    + if text.len() > 40 { "…" } else { "" }
            };
            let (row, _value, edit) =
                editable_value_row(&label, &hint, &display, !readonly);
            {
                let state = Rc::clone(state);
                let status = Rc::clone(status);
                let refresh_ui = Rc::clone(refresh_ui);
                let parent = parent.clone();
                let module = module.clone();
                let label = label.clone();
                let hint = hint.clone();
                edit.connect_clicked(move |_| {
                    let initial = state
                        .borrow()
                        .model
                        .module_config(&module)
                        .and_then(|c| c.get(key).cloned())
                        .map(|v| serde_json::to_string_pretty(&v).unwrap_or_default())
                        .unwrap_or_default();
                    let state = Rc::clone(&state);
                    let status = Rc::clone(&status);
                    let refresh_ui = Rc::clone(&refresh_ui);
                    let module = module.clone();
                    open_json_edit_dialog(
                        &parent,
                        &format!("Edit {label}"),
                        &format!("{hint} — leave empty to remove"),
                        &initial,
                        move |text| {
                            {
                                let mut st = state.borrow_mut();
                                let Some(cfg) = st.model.bar.get_mut(&module) else {
                                    return;
                                };
                                if !cfg.is_object() {
                                    *cfg = Value::Object(serde_json::Map::new());
                                }
                                if let Some(obj) = cfg.as_object_mut() {
                                    if text.trim().is_empty() {
                                        obj.remove(key);
                                    } else if let Ok(v) = serde_json::from_str::<Value>(&text) {
                                        obj.insert(key.into(), v);
                                        st.model.primary_keys.insert(module.clone());
                                    }
                                }
                            }
                            status(format!("Saved {module}.{key}"));
                            call_refresh(&refresh_ui);
                        },
                    );
                });
            }
            row
        }
    }
}

fn rebuild_style_tab(
    body: &GtkBox,
    state: &Rc<RefCell<StudioState>>,
    refresh_ui: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    status: &Rc<dyn Fn(String)>,
    parent: &Window,
) {
    clear_box(body);
    let tokens = super::style::extract_tokens(&state.borrow().model.style_css);

    const FIELDS: &[(&str, &str, &str)] = &[
        ("font_family", "Font family", ""),
        ("font_size", "Font size", ""),
        ("bar_bg", "Bar background", ""),
        ("bar_fg", "Bar foreground", ""),
        ("pill_bg", "Pill background", ""),
        ("pill_fg", "Pill foreground", ""),
        ("border_color", "Border color", ""),
        ("border_radius", "Border radius", ""),
        ("padding", "Padding", ""),
        ("margin", "Margin", ""),
        ("accent", "Accent", "Active / hover accent"),
    ];

    fn token_get(tokens: &StyleTokens, key: &str) -> String {
        match key {
            "font_family" => tokens.font_family.clone(),
            "font_size" => tokens.font_size.clone(),
            "bar_bg" => tokens.bar_bg.clone(),
            "bar_fg" => tokens.bar_fg.clone(),
            "pill_bg" => tokens.pill_bg.clone(),
            "pill_fg" => tokens.pill_fg.clone(),
            "border_color" => tokens.border_color.clone(),
            "border_radius" => tokens.border_radius.clone(),
            "padding" => tokens.padding.clone(),
            "margin" => tokens.margin.clone(),
            _ => tokens.accent.clone(),
        }
    }

    fn token_set(tokens: &mut StyleTokens, key: &str, value: String) {
        match key {
            "font_family" => tokens.font_family = value,
            "font_size" => tokens.font_size = value,
            "bar_bg" => tokens.bar_bg = value,
            "bar_fg" => tokens.bar_fg = value,
            "pill_bg" => tokens.pill_bg = value,
            "pill_fg" => tokens.pill_fg = value,
            "border_color" => tokens.border_color = value,
            "border_radius" => tokens.border_radius = value,
            "padding" => tokens.padding = value,
            "margin" => tokens.margin = value,
            _ => tokens.accent = value,
        }
    }

    let mut rows: Vec<GtkBox> = Vec::new();
    for &(key, title, hint) in FIELDS {
        let current = token_get(&tokens, key);
        let (row, _, edit) = editable_value_row(title, hint, &current, true);
        let state = Rc::clone(state);
        let status = Rc::clone(status);
        let refresh_ui = Rc::clone(refresh_ui);
        let parent = parent.clone();
        let dialog_hint = if hint.is_empty() {
            "Cancel discards; Save updates in-memory CSS.".to_string()
        } else {
            hint.to_string()
        };
        edit.connect_clicked(move |_| {
            let initial = token_get(
                &super::style::extract_tokens(&state.borrow().model.style_css),
                key,
            );
            let state = Rc::clone(&state);
            let status = Rc::clone(&status);
            let refresh_ui = Rc::clone(&refresh_ui);
            open_text_edit_dialog(
                &parent,
                &format!("Edit {title}"),
                &dialog_hint,
                &initial,
                "",
                move |value| {
                    {
                        let mut st = state.borrow_mut();
                        let mut tokens = super::style::extract_tokens(&st.model.style_css);
                        token_set(&mut tokens, key, value);
                        st.model.style_css =
                            super::style::apply_tokens(&st.model.style_css, &tokens);
                    }
                    status(format!(
                        "Saved {title} to CSS (click Apply to write + reload)"
                    ));
                    call_refresh(&refresh_ui);
                },
            );
        });
        rows.push(row);
    }

    body.append(&settings_card("Style tokens", &rows));

    let css = state.borrow().model.style_css.clone();
    let css_preview = if css.is_empty() {
        "(empty)".to_string()
    } else if css.len() > 48 {
        format!("{}…", css.chars().take(48).collect::<String>())
    } else {
        css.clone()
    };
    let (css_row, _, css_edit) = editable_value_row(
        "Raw CSS",
        "Full style.css — Cancel discards, Save commits in memory",
        &css_preview,
        true,
    );
    {
        let state = Rc::clone(state);
        let refresh_ui = Rc::clone(refresh_ui);
        let status = Rc::clone(status);
        let parent = parent.clone();
        css_edit.connect_clicked(move |_| {
            let initial = state.borrow().model.style_css.clone();
            let state = Rc::clone(&state);
            let refresh_ui = Rc::clone(&refresh_ui);
            let status = Rc::clone(&status);
            open_multiline_edit_dialog(
                &parent,
                "Edit raw CSS",
                "Cancel discards. Save updates in-memory CSS (toolbar Apply writes + reloads).",
                &initial,
                true,
                None,
                move |text| {
                    state.borrow_mut().model.style_css = text;
                    status("Updated in-memory CSS (click Apply to write + reload)".into());
                    call_refresh(&refresh_ui);
                },
            );
        });
    }
    body.append(&settings_card("Advanced", &[css_row]));
}

fn is_bar_option_key(k: &str) -> bool {
    matches!(
        k,
        "layer"
            | "position"
            | "height"
            | "width"
            | "spacing"
            | "margin"
            | "margin-top"
            | "margin-bottom"
            | "margin-left"
            | "margin-right"
            | "exclusive"
            | "fixed-center"
            | "passthrough"
            | "gtk-layer-shell"
            | "mode"
            | "output"
            | "name"
            | "id"
            | "ipc"
            | "reload_style_on_change"
            | "on-sigusr1"
            | "on-sigusr2"
    )
}

fn looks_like_module_key(k: &str) -> bool {
    k.contains('/')
        || matches!(
            k,
            "clock"
                | "cpu"
                | "memory"
                | "battery"
                | "pulseaudio"
                | "bluetooth"
                | "network"
                | "tray"
                | "backlight"
                | "idle_inhibitor"
        )
}

fn rebuild_themes_tab(
    body: &GtkBox,
    state: &Rc<RefCell<StudioState>>,
    refresh_ui: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    status: &Rc<dyn Fn(String)>,
) {
    clear_box(body);
    body.append(
        &Label::builder()
            .label("Pick a theme to rewrite CSS tokens. Then click Apply in the toolbar to write files and reload Waybar.")
            .wrap(true)
            .xalign(0.0)
            .css_classes(["dim-label"])
            .build(),
    );

    for theme in super::themes::all_themes() {
        let row = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .css_classes(["hyprbinds-settings-row"])
            .build();
        let text = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .build();
        text.append(
            &Label::builder()
                .label(theme.name)
                .halign(gtk4::Align::Start)
                .css_classes(["hyprbinds-settings-title"])
                .build(),
        );
        text.append(
            &Label::builder()
                .label(theme.description)
                .halign(gtk4::Align::Start)
                .css_classes(["hyprbinds-settings-sub", "dim-label"])
                .build(),
        );
        let btn = Button::builder().label("Use theme").build();
        {
            let state = Rc::clone(state);
            let refresh_ui = Rc::clone(refresh_ui);
            let status = Rc::clone(status);
            let id = theme.id;
            btn.connect_clicked(move |_| {
                let mut st = state.borrow_mut();
                if let Some(css) = super::themes::apply_theme_to_css(&st.model.style_css, id) {
                    st.model.style_css = css;
                    status(format!(
                        "Applied theme '{id}' to CSS — click Apply to write + reload"
                    ));
                }
                drop(st);
                call_refresh(&refresh_ui);
            });
        }
        row.append(&text);
        row.append(&btn);
        body.append(&row);
    }
}
