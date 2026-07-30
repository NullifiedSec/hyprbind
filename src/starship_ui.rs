//! Starship Studio page — shell install hooks, curated options, config editor, presets.

use crate::dialog;
use crate::starship;
use crate::starship_model::{ShellStatus, StarshipModel};
use crate::starship_options::{
    self, ConfigOption, OptionKind, OptionValue, TOP_OPTIONS,
};
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, Entry, Label, ListBox, ListBoxRow, Notebook, Orientation,
    PolicyType, ScrolledWindow, SpinButton, TextBuffer, TextView,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct StarshipPage {
    pub page: GtkBox,
}

struct StudioState {
    model: StarshipModel,
    shells: Vec<ShellStatus>,
}

pub fn build_starship_page(status: Rc<dyn Fn(String)>) -> StarshipPage {
    let model = starship::load().unwrap_or_else(|e| {
        let path = starship::config_path()
            .unwrap_or_else(|| std::path::PathBuf::from("~/.config/starship.toml"));
        let mut m = StarshipModel::empty(path);
        m.notes.push(format!("Load warning: {e}"));
        m
    });
    let shells = starship::detect_shells();
    let state = Rc::new(RefCell::new(StudioState { model, shells }));

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

    let preview = Label::builder()
        .halign(gtk4::Align::Start)
        .wrap(true)
        .xalign(0.0)
        .selectable(true)
        .css_classes(["hyprbinds-starship-preview-text"])
        .build();
    let preview_box = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .hexpand(true)
        .css_classes(["hyprbinds-starship-preview"])
        .build();
    preview_box.append(
        &Label::builder()
            .label("Prompt preview")
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-settings-card-title"])
            .build(),
    );
    preview_box.append(&preview);

    let apply_btn = Button::builder()
        .label("Apply")
        .css_classes(["suggested-action"])
        .build();
    let preview_btn = Button::builder().label("Refresh preview").build();
    let restore_btn = Button::builder().label("Restore").build();
    let refresh_btn = Button::builder().label("Re-read").build();

    let toolbar = GtkBox::new(Orientation::Horizontal, 8);
    toolbar.append(&apply_btn);
    toolbar.append(&preview_btn);
    toolbar.append(&restore_btn);
    toolbar.append(&refresh_btn);

    let shells_body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .build();
    let options_body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .build();
    let config_body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .build();
    let presets_body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .build();

    let notebook = Notebook::new();
    notebook.append_page(&wrap_scroll(&shells_body), Some(&Label::new(Some("Shells"))));
    notebook.append_page(
        &wrap_scroll(&options_body),
        Some(&Label::new(Some("Options"))),
    );
    notebook.append_page(&wrap_scroll(&config_body), Some(&Label::new(Some("Config"))));
    notebook.append_page(
        &wrap_scroll(&presets_body),
        Some(&Label::new(Some("Presets"))),
    );
    notebook.set_vexpand(true);
    notebook.set_hexpand(true);

    let refresh_ui: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));
    let refresh_busy = Rc::new(Cell::new(false));

    let sync_toml_from_editor: Rc<RefCell<Option<Rc<dyn Fn()>>>> =
        Rc::new(RefCell::new(None));

    let do_meta_preview = {
        let state = Rc::clone(&state);
        let path_label = path_label.clone();
        let proc_label = proc_label.clone();
        let note_label = note_label.clone();
        let preview = preview.clone();
        Rc::new(move || {
            let (path, notes, toml_text) = {
                let st = state.borrow();
                (
                    st.model.config_path.display().to_string(),
                    st.model.notes.clone(),
                    st.model.toml_text.clone(),
                )
            };
            path_label.set_label(&path);
            proc_label.set_label(&format!("Starship: {}", starship::status_label()));
            note_label.set_label(&if notes.is_empty() {
                String::new()
            } else {
                notes.join(" · ")
            });
            match starship::preview_prompt(&toml_text) {
                Ok(text) => preview.set_label(&if text.is_empty() {
                    "(empty prompt)".into()
                } else {
                    text
                }),
                Err(e) => preview.set_label(&format!("Preview unavailable: {e}")),
            }
        }) as Rc<dyn Fn()>
    };

    let do_refresh = {
        let state = Rc::clone(&state);
        let shells_body = shells_body.clone();
        let options_body = options_body.clone();
        let config_body = config_body.clone();
        let presets_body = presets_body.clone();
        let refresh_ui = Rc::clone(&refresh_ui);
        let sync_toml_from_editor = Rc::clone(&sync_toml_from_editor);
        let do_meta_preview = Rc::clone(&do_meta_preview);
        let refresh_busy = Rc::clone(&refresh_busy);
        let status = Rc::clone(&status);

        Rc::new(move || {
            if refresh_busy.get() {
                return;
            }
            refresh_busy.set(true);
            // Pull editor text into model before rebuild replaces the TextView.
            if let Some(f) = sync_toml_from_editor.borrow().as_ref() {
                f();
            }
            do_meta_preview();
            rebuild_shells_tab(&shells_body, &state, &refresh_ui, &status);
            rebuild_options_tab(&options_body, &state, &refresh_ui, &status);
            rebuild_config_tab(&config_body, &state, &sync_toml_from_editor);
            rebuild_presets_tab(&presets_body, &state, &refresh_ui, &status);
            refresh_busy.set(false);
        }) as Rc<dyn Fn()>
    };
    *refresh_ui.borrow_mut() = Some(Rc::clone(&do_refresh));
    do_refresh();

    {
        let state = Rc::clone(&state);
        let status = Rc::clone(&status);
        let do_refresh = Rc::clone(&do_refresh);
        let sync_toml_from_editor = Rc::clone(&sync_toml_from_editor);
        apply_btn.connect_clicked(move |_| {
            if let Some(f) = sync_toml_from_editor.borrow().as_ref() {
                f();
            }
            let model = state.borrow().model.clone();
            match starship::apply(&model) {
                Ok(msg) => status(msg),
                Err(e) => status(format!("Apply failed: {e}")),
            }
            do_refresh();
        });
    }
    {
        let do_meta_preview = Rc::clone(&do_meta_preview);
        let sync_toml_from_editor = Rc::clone(&sync_toml_from_editor);
        let status = Rc::clone(&status);
        preview_btn.connect_clicked(move |_| {
            if let Some(f) = sync_toml_from_editor.borrow().as_ref() {
                f();
            }
            do_meta_preview();
            status("Preview refreshed".into());
        });
    }
    {
        let state = Rc::clone(&state);
        let status = Rc::clone(&status);
        let do_refresh = Rc::clone(&do_refresh);
        restore_btn.connect_clicked(move |_| {
            let model = state.borrow().model.clone();
            match starship::restore_files(&model) {
                Ok(m) => status(m),
                Err(e) => status(e.to_string()),
            }
            match starship::load() {
                Ok(m) => state.borrow_mut().model = m,
                Err(e) => status(format!("Re-load after restore failed: {e}")),
            }
            do_refresh();
        });
    }
    {
        let state = Rc::clone(&state);
        let status = Rc::clone(&status);
        let do_refresh = Rc::clone(&do_refresh);
        refresh_btn.connect_clicked(move |_| {
            match starship::load() {
                Ok(m) => {
                    state.borrow_mut().model = m;
                    status("Re-read starship.toml from disk".into());
                }
                Err(e) => status(format!("Load failed: {e}")),
            }
            state.borrow_mut().shells = starship::detect_shells();
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
    content.append(&preview_box);
    content.append(&notebook);

    let page = dialog::page_shell(
        "Starship Studio",
        "Install Starship into shells, tweak the top 30 config options, edit starship.toml, and try presets.",
        &toolbar,
        &content,
    );

    StarshipPage { page }
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
    let refresh_ui = Rc::clone(refresh_ui);
    gtk4::glib::idle_add_local_once(move || {
        if let Some(f) = refresh_ui.borrow().as_ref().cloned() {
            f();
        }
    });
}

fn rebuild_shells_tab(
    body: &GtkBox,
    state: &Rc<RefCell<StudioState>>,
    refresh_ui: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    status: &Rc<dyn Fn(String)>,
) {
    clear_box(body);

    let installed = starship::command_exists("starship");
    let binary_row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .css_classes(["hyprbinds-settings-card"])
        .build();
    let binary_col = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .build();
    binary_col.append(
        &Label::builder()
            .label("Starship binary")
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-settings-card-title"])
            .build(),
    );
    binary_col.append(
        &Label::builder()
            .label(if installed {
                format!("Found on PATH — {}", starship::status_label())
            } else {
                format!(
                    "Not installed. Install with: {}",
                    starship::install_hint()
                )
            })
            .halign(gtk4::Align::Start)
            .wrap(true)
            .xalign(0.0)
            .css_classes(["hyprbinds-settings-sub"])
            .build(),
    );
    binary_row.append(&binary_col);
    if !installed {
        let copy_btn = Button::builder().label("Copy install").build();
        copy_btn.connect_clicked({
            let status = Rc::clone(status);
            move |_| {
                if let Some(display) = gtk4::gdk::Display::default() {
                    display.clipboard().set_text(starship::install_hint());
                    status("Copied install command to clipboard".into());
                } else {
                    status(format!("Install: {}", starship::install_hint()));
                }
            }
        });
        binary_row.append(&copy_btn);
    }
    body.append(&binary_row);

    body.append(
        &Label::builder()
            .label("Shells on this system")
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-settings-card-title"])
            .build(),
    );
    body.append(
        &Label::builder()
            .label(
                "Enable adds a managed init block to the shell RC. Disable removes only the Hyprbinds-managed block.",
            )
            .halign(gtk4::Align::Start)
            .wrap(true)
            .xalign(0.0)
            .css_classes(["hyprbinds-settings-sub"])
            .build(),
    );

    let shells = state.borrow().shells.clone();
    if shells.is_empty() {
        body.append(
            &Label::builder()
                .label("No supported shells detected.")
                .halign(gtk4::Align::Start)
                .css_classes(["dim-label"])
                .build(),
        );
        return;
    }

    let list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    for shell in shells {
        list.append(&shell_row(&shell, state, refresh_ui, status));
    }
    body.append(&list);
}

fn shell_row(
    shell: &ShellStatus,
    state: &Rc<RefCell<StudioState>>,
    refresh_ui: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    status: &Rc<dyn Fn(String)>,
) -> ListBoxRow {
    let col = dialog::list_row_column();
    let title = format!(
        "{}{}",
        shell.kind.label(),
        if shell.installed {
            ""
        } else {
            " (binary missing)"
        }
    );
    col.append(&dialog::row_title(&title));

    let status_text = if shell.enabled {
        if shell.managed {
            "Starship enabled (Hyprbinds-managed)"
        } else {
            "Starship enabled (existing init)"
        }
    } else {
        "Starship not enabled"
    };
    col.append(&dialog::row_sub(status_text));
    col.append(&dialog::row_meta(&shell.rc_path.display().to_string()));
    for n in &shell.notes {
        col.append(&dialog::row_meta(n));
    }

    let actions = GtkBox::new(Orientation::Horizontal, 8);
    actions.set_halign(gtk4::Align::Start);
    actions.set_margin_top(6);

    let kind = shell.kind;
    if shell.enabled && shell.managed {
        let disable = Button::builder().label("Disable").build();
        disable.connect_clicked({
            let status = Rc::clone(status);
            let state = Rc::clone(state);
            let refresh_ui = Rc::clone(refresh_ui);
            move |_| {
                match starship::disable_shell(kind) {
                    Ok(msg) => status(msg),
                    Err(e) => status(e.to_string()),
                }
                state.borrow_mut().shells = starship::detect_shells();
                call_refresh(&refresh_ui);
            }
        });
        actions.append(&disable);
    } else if !shell.enabled {
        let enable = Button::builder()
            .label("Enable for this shell")
            .css_classes(["suggested-action"])
            .sensitive(shell.installed && starship::command_exists("starship"))
            .build();
        enable.connect_clicked({
            let status = Rc::clone(status);
            let state = Rc::clone(state);
            let refresh_ui = Rc::clone(refresh_ui);
            move |_| {
                match starship::enable_shell(kind) {
                    Ok(msg) => status(msg),
                    Err(e) => status(e.to_string()),
                }
                state.borrow_mut().shells = starship::detect_shells();
                call_refresh(&refresh_ui);
            }
        });
        actions.append(&enable);
    } else {
        // Enabled but unmanaged — offer managed enable (append) + note.
        let enable = Button::builder()
            .label("Add managed block")
            .sensitive(shell.installed && starship::command_exists("starship"))
            .build();
        enable.connect_clicked({
            let status = Rc::clone(status);
            let state = Rc::clone(state);
            let refresh_ui = Rc::clone(refresh_ui);
            move |_| {
                match starship::enable_shell(kind) {
                    Ok(msg) => status(msg),
                    Err(e) => status(e.to_string()),
                }
                state.borrow_mut().shells = starship::detect_shells();
                call_refresh(&refresh_ui);
            }
        });
        actions.append(&enable);
    }

    col.append(&actions);

    ListBoxRow::builder()
        .child(&col)
        .activatable(false)
        .selectable(false)
        .build()
}

fn rebuild_options_tab(
    body: &GtkBox,
    state: &Rc<RefCell<StudioState>>,
    refresh_ui: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    status: &Rc<dyn Fn(String)>,
) {
    clear_box(body);

    body.append(
        &Label::builder()
            .label("Top options")
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-settings-card-title"])
            .build(),
    );
    body.append(
        &Label::builder()
            .label(format!(
                "{} curated settings from the official Starship schema. Changes update the Config tab; Apply writes disk. Unset keys show schema defaults.",
                TOP_OPTIONS.len()
            ))
            .halign(gtk4::Align::Start)
            .wrap(true)
            .xalign(0.0)
            .css_classes(["hyprbinds-settings-sub"])
            .build(),
    );

    let toml_text = state.borrow().model.toml_text.clone();
    let doc = match starship_options::parse_document(&toml_text) {
        Ok(d) => d,
        Err(e) => {
            body.append(
                &Label::builder()
                    .label(format!(
                        "Cannot parse starship.toml for Options UI:\n{e}\n\nFix the raw Config tab first."
                    ))
                    .halign(gtk4::Align::Start)
                    .wrap(true)
                    .xalign(0.0)
                    .css_classes(["dim-label"])
                    .build(),
            );
            return;
        }
    };

    let mut current_group = "";
    let mut list: Option<ListBox> = None;

    for opt in TOP_OPTIONS {
        if opt.group != current_group {
            current_group = opt.group;
            body.append(
                &Label::builder()
                    .label(opt.group)
                    .halign(gtk4::Align::Start)
                    .margin_top(8)
                    .css_classes(["hyprbinds-settings-card-title"])
                    .build(),
            );
            let lb = ListBox::builder()
                .selection_mode(gtk4::SelectionMode::None)
                .css_classes(["boxed-list"])
                .build();
            body.append(&lb);
            list = Some(lb);
        }
        if let Some(lb) = &list {
            lb.append(&option_row(opt, &doc, state, refresh_ui, status));
        }
    }
}

fn option_row(
    opt: &'static ConfigOption,
    doc: &toml_edit::DocumentMut,
    state: &Rc<RefCell<StudioState>>,
    refresh_ui: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    status: &Rc<dyn Fn(String)>,
) -> ListBoxRow {
    let col = dialog::list_row_column();
    col.append(&dialog::row_title(opt.label));
    col.append(&dialog::row_sub(opt.hint));
    col.append(&dialog::row_meta(&format!("`{}`", opt.path)));

    let unset = matches!(
        starship_options::get_option(doc, opt),
        OptionValue::Unset
    );
    if unset {
        col.append(&dialog::row_meta(&format!(
            "Using default: {}",
            starship_options::display_default(opt)
        )));
    }

    let controls = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .margin_top(6)
        .halign(gtk4::Align::Start)
        .build();

    match opt.kind {
        OptionKind::Bool => {
            let check = CheckButton::builder()
                .label(if opt.path.ends_with(".disabled") {
                    "Disabled"
                } else {
                    "On"
                })
                .active(starship_options::effective_bool(doc, opt))
                .build();
            check.connect_toggled({
                let state = Rc::clone(state);
                let refresh_ui = Rc::clone(refresh_ui);
                let status = Rc::clone(status);
                move |btn| {
                    write_option(
                        &state,
                        opt,
                        OptionValue::Bool(btn.is_active()),
                        &status,
                        &refresh_ui,
                    );
                }
            });
            controls.append(&check);
        }
        OptionKind::String => {
            let entry = Entry::builder()
                .text(starship_options::effective_string(doc, opt))
                .hexpand(true)
                .width_chars(28)
                .build();
            let apply = Button::builder().label("Set").build();
            apply.connect_clicked({
                let state = Rc::clone(state);
                let refresh_ui = Rc::clone(refresh_ui);
                let status = Rc::clone(status);
                let entry = entry.clone();
                move |_| {
                    write_option(
                        &state,
                        opt,
                        OptionValue::String(entry.text().to_string()),
                        &status,
                        &refresh_ui,
                    );
                }
            });
            entry.connect_activate({
                let state = Rc::clone(state);
                let refresh_ui = Rc::clone(refresh_ui);
                let status = Rc::clone(status);
                let entry = entry.clone();
                move |_| {
                    write_option(
                        &state,
                        opt,
                        OptionValue::String(entry.text().to_string()),
                        &status,
                        &refresh_ui,
                    );
                }
            });
            controls.append(&entry);
            controls.append(&apply);
        }
        OptionKind::Integer { min, max } => {
            let spin = SpinButton::with_range(min as f64, max as f64, 1.0);
            spin.set_value(starship_options::effective_integer(doc, opt) as f64);
            spin.connect_value_changed({
                let state = Rc::clone(state);
                let refresh_ui = Rc::clone(refresh_ui);
                let status = Rc::clone(status);
                let busy = Rc::new(Cell::new(false));
                move |spin| {
                    if busy.get() {
                        return;
                    }
                    busy.set(true);
                    write_option(
                        &state,
                        opt,
                        OptionValue::Integer(spin.value() as i64),
                        &status,
                        &refresh_ui,
                    );
                    busy.set(false);
                }
            });
            controls.append(&spin);
        }
    }

    if !unset {
        let reset = Button::builder().label("Reset").build();
        reset.set_tooltip_text(Some("Remove this key (fall back to Starship default)"));
        reset.connect_clicked({
            let state = Rc::clone(state);
            let refresh_ui = Rc::clone(refresh_ui);
            let status = Rc::clone(status);
            move |_| {
                write_option(&state, opt, OptionValue::Unset, &status, &refresh_ui);
            }
        });
        controls.append(&reset);
    }

    col.append(&controls);

    ListBoxRow::builder()
        .child(&col)
        .activatable(false)
        .selectable(false)
        .build()
}

fn write_option(
    state: &Rc<RefCell<StudioState>>,
    opt: &ConfigOption,
    value: OptionValue,
    status: &Rc<dyn Fn(String)>,
    refresh_ui: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
) {
    let toml_text = state.borrow().model.toml_text.clone();
    match starship_options::apply_option_to_toml(&toml_text, opt, value) {
        Ok(next) => {
            state.borrow_mut().model.toml_text = next;
            status(format!("Updated {}", opt.path));
            call_refresh(refresh_ui);
        }
        Err(e) => status(format!("Could not set {}: {e}", opt.path)),
    }
}

fn rebuild_config_tab(
    body: &GtkBox,
    state: &Rc<RefCell<StudioState>>,
    sync_toml_from_editor: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
) {
    clear_box(body);

    body.append(
        &Label::builder()
            .label("starship.toml (raw)")
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-settings-card-title"])
            .build(),
    );
    body.append(
        &Label::builder()
            .label("Full config editor. Options tab writes here; Apply saves to disk (with a backup).")
            .halign(gtk4::Align::Start)
            .wrap(true)
            .xalign(0.0)
            .css_classes(["hyprbinds-settings-sub"])
            .build(),
    );

    let buffer = TextBuffer::new(None::<&gtk4::TextTagTable>);
    buffer.set_text(&state.borrow().model.toml_text);
    let view = TextView::builder()
        .buffer(&buffer)
        .monospace(true)
        .wrap_mode(gtk4::WrapMode::WordChar)
        .hexpand(true)
        .vexpand(true)
        .top_margin(8)
        .bottom_margin(8)
        .left_margin(10)
        .right_margin(10)
        .css_classes(["hyprbinds-starship-editor"])
        .height_request(320)
        .build();

    *sync_toml_from_editor.borrow_mut() = Some({
        let state = Rc::clone(state);
        let buffer = buffer.clone();
        Rc::new(move || {
            let (start, end) = buffer.bounds();
            let text = buffer.text(&start, &end, false);
            state.borrow_mut().model.toml_text = text.to_string();
        }) as Rc<dyn Fn()>
    });

    // Keep model in sync as the user types (cheap for typical configs).
    buffer.connect_changed({
        let state = Rc::clone(state);
        let buffer = buffer.clone();
        move |_| {
            let (start, end) = buffer.bounds();
            let text = buffer.text(&start, &end, false);
            state.borrow_mut().model.toml_text = text.to_string();
        }
    });

    body.append(&view);
}

fn rebuild_presets_tab(
    body: &GtkBox,
    state: &Rc<RefCell<StudioState>>,
    refresh_ui: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    status: &Rc<dyn Fn(String)>,
) {
    clear_box(body);

    body.append(
        &Label::builder()
            .label("Built-in presets")
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-settings-card-title"])
            .build(),
    );
    body.append(
        &Label::builder()
            .label(
                "Loading a preset replaces the editor contents. Apply to write starship.toml.",
            )
            .halign(gtk4::Align::Start)
            .wrap(true)
            .xalign(0.0)
            .css_classes(["hyprbinds-settings-sub"])
            .build(),
    );

    let presets = starship::list_presets();
    if presets.is_empty() {
        body.append(
            &Label::builder()
                .label(if starship::command_exists("starship") {
                    "No presets returned by `starship preset --list`."
                } else {
                    "Install starship to browse presets."
                })
                .halign(gtk4::Align::Start)
                .css_classes(["dim-label"])
                .build(),
        );
        return;
    }

    let list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    for name in presets {
        let col = dialog::list_row_column();
        col.append(&dialog::row_title(&name));
        let btn = Button::builder()
            .label("Use preset")
            .halign(gtk4::Align::Start)
            .build();
        btn.set_margin_top(6);
        let name_clone = name.clone();
        btn.connect_clicked({
            let state = Rc::clone(state);
            let status = Rc::clone(status);
            let refresh_ui = Rc::clone(refresh_ui);
            move |_| {
                match starship::load_preset(&name_clone) {
                    Ok(text) => {
                        let mut st = state.borrow_mut();
                        st.model.toml_text = text;
                        st.model.selected_preset = Some(name_clone.clone());
                        status(format!(
                            "Loaded preset '{name_clone}' into editor — Apply to save"
                        ));
                    }
                    Err(e) => status(e.to_string()),
                }
                call_refresh(&refresh_ui);
            }
        });
        col.append(&btn);
        list.append(
            &ListBoxRow::builder()
                .child(&col)
                .activatable(false)
                .selectable(false)
                .build(),
        );
    }
    body.append(&list);
}
