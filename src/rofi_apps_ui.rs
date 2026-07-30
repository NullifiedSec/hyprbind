//! Rofi Apps page — manage clipboard / power / wifi / custom rofi scripts.

use crate::bind::{BindCollection, Keybind};
use crate::dialog;
use crate::rofi_apps::{self, RofiApp, RofiAppsStore};
use crate::writer;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DropDown, Entry, Label, ListBox, ListBoxRow, Orientation,
    PolicyType, ScrolledWindow, StringList, TextBuffer, TextView, Window,
};
use std::cell::RefCell;
use std::rc::Rc;

pub type StatusFn = Rc<dyn Fn(String)>;
pub type ReloadFn = Rc<dyn Fn()>;

pub struct RofiAppsPage {
    pub page: GtkBox,
}

struct StudioState {
    store: RofiAppsStore,
    selected: Option<String>,
}

pub fn build_rofi_apps_page(
    parent: &impl IsA<Window>,
    collection: Rc<RefCell<Option<BindCollection>>>,
    reload: ReloadFn,
    status: StatusFn,
) -> RofiAppsPage {
    let parent = parent.clone().upcast::<Window>();
    let state = Rc::new(RefCell::new(StudioState {
        store: rofi_apps::load(),
        selected: None,
    }));

    let path_label = Label::builder()
        .halign(gtk4::Align::Start)
        .ellipsize(gtk4::pango::EllipsizeMode::Middle)
        .css_classes(["dim-label", "caption"])
        .build();
    let note_label = Label::builder()
        .halign(gtk4::Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();

    let apply_btn = Button::builder()
        .label("Apply")
        .css_classes(["suggested-action"])
        .build();
    let refresh_btn = Button::builder().label("Re-read").build();
    let test_btn = Button::builder().label("Test").sensitive(false).build();
    let edit_btn = Button::builder().label("Edit").sensitive(false).build();
    let remove_btn = Button::builder()
        .label("Remove")
        .sensitive(false)
        .css_classes(["destructive-action"])
        .build();
    let bind_btn = Button::builder()
        .label("Add bind…")
        .sensitive(false)
        .build();
    let daemon_btn = Button::builder()
        .label("Add daemon…")
        .sensitive(false)
        .build();

    let toolbar = GtkBox::new(Orientation::Horizontal, 8);
    toolbar.append(&apply_btn);
    toolbar.append(&refresh_btn);
    toolbar.append(&test_btn);
    toolbar.append(&edit_btn);
    toolbar.append(&remove_btn);
    toolbar.append(&bind_btn);
    toolbar.append(&daemon_btn);

    let list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();

    let presets = rofi_apps::catalog();
    let preset_labels: Vec<&str> = presets.iter().map(|p| p.label).collect();
    let mut add_labels = preset_labels;
    add_labels.push("Custom…");
    let add_list = StringList::new(&add_labels);
    let add_dd = DropDown::builder().model(&add_list).selected(0).build();
    let add_btn = Button::builder()
        .label("Add")
        .css_classes(["suggested-action"])
        .build();
    let add_row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .build();
    add_row.append(
        &Label::builder()
            .label("Add app")
            .css_classes(["dim-label"])
            .build(),
    );
    add_dd.set_hexpand(true);
    add_row.append(&add_dd);
    add_row.append(&add_btn);

    let refresh_ui: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));

    let do_refresh = {
        let state = Rc::clone(&state);
        let list = list.clone();
        let path_label = path_label.clone();
        let note_label = note_label.clone();
        let test_btn = test_btn.clone();
        let edit_btn = edit_btn.clone();
        let remove_btn = remove_btn.clone();
        let bind_btn = bind_btn.clone();
        let daemon_btn = daemon_btn.clone();
        let refresh_ui = Rc::clone(&refresh_ui);

        Rc::new(move || {
            let store_path = rofi_apps::store_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "~/.config/hyprbinds/rofi-apps.json".into());
            let scripts = rofi_apps::scripts_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "~/.config/rofi/scripts".into());
            path_label.set_label(&format!("{store_path}  ·  scripts → {scripts}"));

            let n = state.borrow().store.apps.len();
            let enabled = state
                .borrow()
                .store
                .apps
                .iter()
                .filter(|a| a.enabled)
                .count();
            note_label.set_label(&format!(
                "{n} app(s), {enabled} enabled. Apply writes executable scripts; Test launches the selected one."
            ));

            rebuild_list(&list, &state, &refresh_ui);

            let has = state.borrow().selected.is_some();
            let has_daemon = {
                let st = state.borrow();
                st.selected
                    .as_ref()
                    .and_then(|id| st.store.apps.iter().find(|a| &a.id == id))
                    .is_some_and(|a| !a.daemon.trim().is_empty())
            };
            test_btn.set_sensitive(has);
            edit_btn.set_sensitive(has);
            remove_btn.set_sensitive(has);
            bind_btn.set_sensitive(has);
            daemon_btn.set_sensitive(has && has_daemon);
        }) as Rc<dyn Fn()>
    };
    *refresh_ui.borrow_mut() = Some(Rc::clone(&do_refresh));
    do_refresh();

    list.connect_row_selected({
        let state = Rc::clone(&state);
        let test_btn = test_btn.clone();
        let edit_btn = edit_btn.clone();
        let remove_btn = remove_btn.clone();
        let bind_btn = bind_btn.clone();
        let daemon_btn = daemon_btn.clone();
        move |_, row| {
            {
                let mut st = state.borrow_mut();
                if let Some(row) = row {
                    let id = row.widget_name().to_string();
                    if !id.is_empty() {
                        st.selected = Some(id);
                    }
                } else {
                    st.selected = None;
                }
            }
            let (has, has_daemon) = {
                let st = state.borrow();
                let has = st.selected.is_some();
                let has_daemon = st
                    .selected
                    .as_ref()
                    .and_then(|id| st.store.apps.iter().find(|a| &a.id == id))
                    .is_some_and(|a| !a.daemon.trim().is_empty());
                (has, has_daemon)
            };
            test_btn.set_sensitive(has);
            edit_btn.set_sensitive(has);
            remove_btn.set_sensitive(has);
            bind_btn.set_sensitive(has);
            daemon_btn.set_sensitive(has && has_daemon);
        }
    });

    apply_btn.connect_clicked({
        let state = Rc::clone(&state);
        let status = Rc::clone(&status);
        let do_refresh = Rc::clone(&do_refresh);
        move |_| {
            let store = state.borrow().store.clone();
            match rofi_apps::apply(&store) {
                Ok(msg) => status(msg),
                Err(e) => status(format!("Apply failed: {e}")),
            }
            do_refresh();
        }
    });

    refresh_btn.connect_clicked({
        let state = Rc::clone(&state);
        let status = Rc::clone(&status);
        let do_refresh = Rc::clone(&do_refresh);
        move |_| {
            state.borrow_mut().store = rofi_apps::load();
            status("Re-read Rofi Apps from disk".into());
            do_refresh();
        }
    });

    test_btn.connect_clicked({
        let state = Rc::clone(&state);
        let status = Rc::clone(&status);
        move |_| {
            let Some(app) = selected_app(&state) else {
                return;
            };
            match rofi_apps::test_run(&app) {
                Ok(msg) => status(msg),
                Err(e) => status(e.to_string()),
            }
        }
    });

    remove_btn.connect_clicked({
        let state = Rc::clone(&state);
        let status = Rc::clone(&status);
        let do_refresh = Rc::clone(&do_refresh);
        move |_| {
            let Some(id) = state.borrow().selected.clone() else {
                return;
            };
            state.borrow_mut().store.apps.retain(|a| a.id != id);
            state.borrow_mut().selected = None;
            status(format!("Removed {id} (Apply to delete script on disk)"));
            do_refresh();
        }
    });

    add_btn.connect_clicked({
        let state = Rc::clone(&state);
        let status = Rc::clone(&status);
        let do_refresh = Rc::clone(&do_refresh);
        let parent = parent.clone();
        let add_dd = add_dd.clone();
        let presets = presets.clone();
        move |_| {
            let idx = add_dd.selected() as usize;
            if idx >= presets.len() {
                // Custom…
                show_custom_dialog(&parent, {
                    let state = Rc::clone(&state);
                    let status = Rc::clone(&status);
                    let do_refresh = Rc::clone(&do_refresh);
                    move |app| {
                        if state.borrow().store.apps.iter().any(|a| a.id == app.id) {
                            status(format!("App '{}' already exists", app.id));
                            return;
                        }
                        state.borrow_mut().selected = Some(app.id.clone());
                        state.borrow_mut().store.apps.push(app);
                        status("Added custom app — Apply to write script".into());
                        do_refresh();
                    }
                });
                return;
            }
            let preset = &presets[idx];
            if state.borrow().store.apps.iter().any(|a| a.id == preset.id) {
                status(format!("'{}' is already in the list", preset.label));
                return;
            }
            let app = rofi_apps::app_from_preset(preset);
            state.borrow_mut().selected = Some(app.id.clone());
            state.borrow_mut().store.apps.push(app);
            status(format!("Added {} — Apply to write script", preset.label));
            do_refresh();
        }
    });

    edit_btn.connect_clicked({
        let state = Rc::clone(&state);
        let parent = parent.clone();
        let status = Rc::clone(&status);
        let do_refresh = Rc::clone(&do_refresh);
        move |_| {
            let Some(app) = selected_app(&state) else {
                return;
            };
            show_edit_dialog(&parent, app, {
                let state = Rc::clone(&state);
                let status = Rc::clone(&status);
                let do_refresh = Rc::clone(&do_refresh);
                move |updated| {
                    let id = updated.id.clone();
                    if let Some(slot) = state
                        .borrow_mut()
                        .store
                        .apps
                        .iter_mut()
                        .find(|a| a.id == id)
                    {
                        *slot = updated;
                    }
                    status("Updated app — Apply to rewrite script".into());
                    do_refresh();
                }
            });
        }
    });

    bind_btn.connect_clicked({
        let state = Rc::clone(&state);
        let parent = parent.clone();
        let collection = Rc::clone(&collection);
        let reload = Rc::clone(&reload);
        let status = Rc::clone(&status);
        move |_| {
            let Some(app) = selected_app(&state) else {
                return;
            };
            let Some(exec) = rofi_apps::exec_path_for(&app) else {
                status("Could not resolve script path".into());
                return;
            };
            // Ensure script exists on disk for a useful bind.
            if !std::path::Path::new(&exec).is_file() {
                status("Apply first so the script exists on disk".into());
                return;
            }
            let config_path = collection
                .borrow()
                .as_ref()
                .map(|c| c.config_path.clone());
            let Some(config_path) = config_path else {
                status("Load a Hyprland config first".into());
                return;
            };
            show_bind_dialog(
                &parent,
                &app,
                &exec,
                config_path,
                Rc::clone(&collection),
                Rc::clone(&reload),
                Rc::clone(&status),
            );
        }
    });

    daemon_btn.connect_clicked({
        let state = Rc::clone(&state);
        let collection = Rc::clone(&collection);
        let reload = Rc::clone(&reload);
        let status = Rc::clone(&status);
        move |_| {
            let Some(app) = selected_app(&state) else {
                return;
            };
            if app.daemon.trim().is_empty() {
                status("This app has no daemon".into());
                return;
            }
            let config_path = collection
                .borrow()
                .as_ref()
                .map(|c| c.config_path.clone());
            let Some(config_path) = config_path else {
                status("Load a Hyprland config first".into());
                return;
            };
            // Avoid duplicates.
            if let Some(col) = collection.borrow().as_ref() {
                if col
                    .startup
                    .iter()
                    .any(|e| e.command.trim() == app.daemon.trim())
                {
                    status("Daemon already present in Startup".into());
                    return;
                }
            }
            match writer::add_startup(&config_path, &app.daemon, &app.daemon_when, "") {
                Ok(_) => {
                    status(format!(
                        "Added daemon to Startup ({}): {}",
                        app.daemon_when, app.daemon
                    ));
                    reload();
                }
                Err(e) => status(format!("Startup add failed: {e}")),
            }
        }
    });

    let list_scroll = ScrolledWindow::builder()
        .child(&list)
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .min_content_height(280)
        .build();

    let content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .vexpand(true)
        .build();
    content.append(&path_label);
    content.append(&note_label);
    content.append(&add_row);
    content.append(
        &Label::builder()
            .label("Installed apps")
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-settings-card-title"])
            .build(),
    );
    content.append(&list_scroll);

    let page = dialog::page_shell(
        "Rofi Apps",
        "Build clipboard, power, Wi-Fi, and custom menus as scripts under ~/.config/rofi/scripts. \
         Wire them to keybinds or Startup daemons.",
        &toolbar,
        &content,
    );

    RofiAppsPage { page }
}

fn selected_app(state: &Rc<RefCell<StudioState>>) -> Option<RofiApp> {
    let st = state.borrow();
    let id = st.selected.as_ref()?;
    st.store.apps.iter().find(|a| &a.id == id).cloned()
}

fn call_refresh(refresh_ui: &Rc<RefCell<Option<Rc<dyn Fn()>>>>) {
    let refresh_ui = Rc::clone(refresh_ui);
    gtk4::glib::idle_add_local_once(move || {
        if let Some(f) = refresh_ui.borrow().as_ref().cloned() {
            f();
        }
    });
}

fn rebuild_list(
    list: &ListBox,
    state: &Rc<RefCell<StudioState>>,
    refresh_ui: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    let apps = state.borrow().store.apps.clone();
    let selected = state.borrow().selected.clone();

    for app in apps {
        let row_box = dialog::list_row_column();
        let title_row = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(10)
            .build();

        let enabled = CheckButton::builder().active(app.enabled).build();
        enabled.connect_toggled({
            let state = Rc::clone(state);
            let refresh_ui = Rc::clone(refresh_ui);
            let id = app.id.clone();
            move |btn| {
                if let Some(slot) = state
                    .borrow_mut()
                    .store
                    .apps
                    .iter_mut()
                    .find(|a| a.id == id)
                {
                    slot.enabled = btn.is_active();
                }
                call_refresh(&refresh_ui);
            }
        });

        let title = dialog::row_title(&app.label);
        title_row.append(&enabled);
        title_row.append(&title);
        row_box.append(&title_row);

        let deps = rofi_apps::deps_label(&app);
        let script = rofi_apps::script_path(&app.script_name)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| app.script_name.clone());
        let sub = format!(
            "{} · {} · {}{}",
            app.kind,
            deps,
            script,
            if app.bind_suggested.is_empty() {
                String::new()
            } else {
                format!(" · bind hint: {}", app.bind_suggested)
            }
        );
        row_box.append(&dialog::row_sub(&sub));
        if !app.notes.is_empty() {
            row_box.append(&dialog::row_meta(&app.notes));
        }
        if !app.daemon.is_empty() {
            row_box.append(&dialog::row_meta(&format!("daemon: {}", app.daemon)));
        }

        let row = ListBoxRow::builder()
            .child(&row_box)
            .activatable(true)
            .selectable(true)
            .name(&app.id)
            .build();
        list.append(&row);
        if selected.as_deref() == Some(app.id.as_str()) {
            list.select_row(Some(&row));
        }
    }
}

fn show_custom_dialog(parent: &Window, on_save: impl Fn(RofiApp) + 'static) {
    let editor = Window::builder()
        .transient_for(parent)
        .modal(true)
        .title("New custom Rofi app")
        .default_width(480)
        .default_height(320)
        .build();

    let body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .margin_top(16)
        .margin_bottom(8)
        .margin_start(16)
        .margin_end(16)
        .build();

    let name = Entry::builder()
        .placeholder_text("Name (e.g. Emoji picker)")
        .build();
    let cmd = Entry::builder()
        .placeholder_text("Command or pipeline to run")
        .hexpand(true)
        .build();
    body.append(
        &Label::builder()
            .label("Label")
            .halign(gtk4::Align::Start)
            .css_classes(["dim-label"])
            .build(),
    );
    body.append(&name);
    body.append(
        &Label::builder()
            .label("Command")
            .halign(gtk4::Align::Start)
            .css_classes(["dim-label"])
            .build(),
    );
    body.append(&cmd);

    let cancel = Button::builder().label("Cancel").build();
    let save = Button::builder()
        .label("Add")
        .css_classes(["suggested-action"])
        .build();
    let actions = dialog::action_buttons(&cancel, &save);
    dialog::mount_sticky_dialog(&editor, &body, None, &actions);

    cancel.connect_clicked({
        let editor = editor.clone();
        move |_| editor.close()
    });
    save.connect_clicked({
        let editor = editor.clone();
        let name = name.clone();
        let cmd = cmd.clone();
        move |_| {
            let label = name.text().to_string();
            let command = cmd.text().to_string();
            if command.trim().is_empty() {
                return;
            }
            on_save(rofi_apps::make_custom(&label, &command));
            editor.close();
        }
    });
    editor.present();
}

fn show_edit_dialog(parent: &Window, app: RofiApp, on_save: impl Fn(RofiApp) + 'static) {
    let editor = Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(format!("Edit {}", app.label))
        .default_width(560)
        .default_height(480)
        .build();

    let body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .margin_top(16)
        .margin_bottom(8)
        .margin_start(16)
        .margin_end(16)
        .build();

    let label = Entry::builder().text(&app.label).build();
    let bind_hint = Entry::builder()
        .text(&app.bind_suggested)
        .placeholder_text("SUPER + V")
        .build();
    let daemon = Entry::builder()
        .text(&app.daemon)
        .placeholder_text("optional daemon command")
        .build();

    let script_text = if app.script_override.trim().is_empty() {
        rofi_apps::render_script(&app)
    } else {
        app.script_override.clone()
    };
    let buffer = TextBuffer::builder().text(&script_text).build();
    let view = TextView::builder()
        .buffer(&buffer)
        .monospace(true)
        .wrap_mode(gtk4::WrapMode::WordChar)
        .vexpand(true)
        .hexpand(true)
        .build();
    let script_scroll = ScrolledWindow::builder()
        .child(&view)
        .min_content_height(200)
        .vexpand(true)
        .build();

    body.append(&labeled("Label", &label));
    body.append(&labeled("Bind hint", &bind_hint));
    body.append(&labeled("Daemon", &daemon));
    body.append(
        &Label::builder()
            .label("Script (saved as override when Apply runs)")
            .halign(gtk4::Align::Start)
            .css_classes(["dim-label"])
            .build(),
    );
    body.append(&script_scroll);

    let cancel = Button::builder().label("Cancel").build();
    let save = Button::builder()
        .label("Save")
        .css_classes(["suggested-action"])
        .build();
    let actions = dialog::action_buttons(&cancel, &save);
    dialog::mount_sticky_dialog(&editor, &body, None, &actions);

    cancel.connect_clicked({
        let editor = editor.clone();
        move |_| editor.close()
    });
    save.connect_clicked({
        let editor = editor.clone();
        let app = app;
        let label = label.clone();
        let bind_hint = bind_hint.clone();
        let daemon = daemon.clone();
        let buffer = buffer.clone();
        move |_| {
            let mut updated = app.clone();
            updated.label = label.text().to_string();
            updated.bind_suggested = bind_hint.text().to_string();
            updated.daemon = daemon.text().to_string();
            let (start, end) = buffer.bounds();
            updated.script_override = buffer.text(&start, &end, false).to_string();
            on_save(updated);
            editor.close();
        }
    });
    editor.present();
}

fn labeled(title: &str, widget: &impl IsA<gtk4::Widget>) -> GtkBox {
    let col = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .build();
    col.append(
        &Label::builder()
            .label(title)
            .halign(gtk4::Align::Start)
            .css_classes(["dim-label"])
            .build(),
    );
    col.append(widget);
    col
}

fn show_bind_dialog(
    parent: &Window,
    app: &RofiApp,
    exec: &str,
    config_path: std::path::PathBuf,
    collection: Rc<RefCell<Option<BindCollection>>>,
    reload: ReloadFn,
    status: StatusFn,
) {
    // Fast path: script already wired to a bind.
    if let Some(col) = collection.borrow().as_ref() {
        if let Some(existing) = find_bind_for_script(&col.binds, exec) {
            status(format!(
                "Already bound as {} ({}) — remove it in Binds first",
                if existing.name.is_empty() {
                    "untitled"
                } else {
                    &existing.name
                },
                existing.keys
            ));
            return;
        }
    }

    let editor = Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(format!("Bind · {}", app.label))
        .default_width(420)
        .default_height(240)
        .build();

    let body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .margin_top(16)
        .margin_bottom(8)
        .margin_start(16)
        .margin_end(16)
        .build();

    let keys = Entry::builder()
        .text(if app.bind_suggested.is_empty() {
            "SUPER + V"
        } else {
            &app.bind_suggested
        })
        .placeholder_text("SUPER + V")
        .build();
    let name = Entry::builder()
        .text(&format!("Rofi · {}", app.label))
        .build();
    let warn = Label::builder()
        .halign(gtk4::Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();

    body.append(
        &Label::builder()
            .label(format!("Command: {exec}"))
            .halign(gtk4::Align::Start)
            .wrap(true)
            .xalign(0.0)
            .css_classes(["dim-label", "caption"])
            .build(),
    );
    body.append(&labeled("Keys", &keys));
    body.append(&labeled("Bind name", &name));
    body.append(&warn);

    let cancel = Button::builder().label("Cancel").build();
    let save = Button::builder()
        .label("Add bind")
        .css_classes(["suggested-action"])
        .build();
    let actions = dialog::action_buttons(&cancel, &save);
    dialog::mount_sticky_dialog(&editor, &body, None, &actions);

    // Live feedback when the chord is already taken.
    {
        let collection = Rc::clone(&collection);
        let warn = warn.clone();
        let save = save.clone();
        let update = Rc::new(move |keys_text: &str| {
            if let Some(col) = collection.borrow().as_ref() {
                if let Some(existing) =
                    find_bind_with_keys(&col.binds, keys_text)
                {
                    warn.set_label(&format!(
                        "Keys already used by {} — choose a different chord.",
                        if existing.name.is_empty() {
                            existing.action.as_str()
                        } else {
                            existing.name.as_str()
                        }
                    ));
                    save.set_sensitive(false);
                    return;
                }
            }
            warn.set_label("");
            save.set_sensitive(true);
        });
        update(&keys.text());
        keys.connect_changed({
            let update = Rc::clone(&update);
            move |entry| update(&entry.text())
        });
    }

    cancel.connect_clicked({
        let editor = editor.clone();
        move |_| editor.close()
    });
    save.connect_clicked({
        let editor = editor.clone();
        let keys = keys.clone();
        let name = name.clone();
        let exec = exec.to_string();
        let collection = Rc::clone(&collection);
        let warn = warn.clone();
        move |_| {
            let keys = keys.text().to_string();
            let name = name.text().to_string();
            if keys.trim().is_empty() {
                return;
            }
            if let Some(col) = collection.borrow().as_ref() {
                if let Some(existing) = find_bind_for_script(&col.binds, &exec) {
                    warn.set_label(&format!(
                        "Script already bound as {} ({})",
                        existing.name, existing.keys
                    ));
                    status("Bind already exists for this script".into());
                    return;
                }
                if let Some(existing) = find_bind_with_keys(&col.binds, &keys) {
                    warn.set_label(&format!(
                        "Keys already used by {}",
                        if existing.name.is_empty() {
                            existing.action.as_str()
                        } else {
                            existing.name.as_str()
                        }
                    ));
                    status(format!("Keys {} already bound", keys.trim()));
                    return;
                }
            }
            match writer::add_bind(&config_path, &name, &keys, &exec, "") {
                Ok(_) => {
                    status(format!("Added bind {keys} → {exec}"));
                    reload();
                    editor.close();
                }
                Err(e) => status(format!("Bind failed: {e}")),
            }
        }
    });
    editor.present();
}

fn find_bind_for_script<'a>(binds: &'a [Keybind], exec: &str) -> Option<&'a Keybind> {
    let exec = exec.trim();
    if exec.is_empty() {
        return None;
    }
    binds.iter().find(|b| action_runs_script(&b.action, exec))
}

fn find_bind_with_keys<'a>(binds: &'a [Keybind], keys: &str) -> Option<&'a Keybind> {
    let want = crate::conflicts::normalize_conflict_keys(keys);
    if want.is_empty() {
        return None;
    }
    binds.iter().find(|b| {
        // Only compare global (empty) submap — Rofi Apps always adds there.
        b.submap.is_empty()
            && crate::conflicts::normalize_conflict_keys(&b.keys) == want
    })
}

fn action_runs_script(action: &str, exec: &str) -> bool {
    if action.contains(exec) {
        return true;
    }
    // Tolerate path vs quoted path differences.
    let needle = exec.trim_matches('"').trim_matches('\'');
    action.contains(needle)
}

