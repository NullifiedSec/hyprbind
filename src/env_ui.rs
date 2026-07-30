//! Environment + Submaps list pages.

use crate::bind::BindCollection;
use crate::dialog;
use crate::env::{EnvVar, Submap};
use crate::writer;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, Entry, Label, ListBox, ListBoxRow, Orientation, PolicyType,
    ScrolledWindow, Window,
};
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

pub type StatusFn = Rc<dyn Fn(String)>;
pub type ReloadFn = Rc<dyn Fn()>;

pub struct EnvSubmapPages {
    pub env_page: GtkBox,
    pub env_list: ListBox,
    pub submap_page: GtkBox,
    pub submap_list: ListBox,
}

pub fn build_env_submap_pages(
    parent: &impl IsA<Window>,
    state: Rc<RefCell<Option<BindCollection>>>,
    reload: ReloadFn,
    status: StatusFn,
    realtime: Rc<Cell<bool>>,
) -> EnvSubmapPages {
    let env_list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();
    let submap_list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();

    // --- Environment ---
    let env_search = Entry::builder()
        .placeholder_text("Filter env…")
        .hexpand(true)
        .build();
    let env_add = Button::builder().label("Add").build();
    let env_edit = Button::builder().label("Edit").sensitive(false).build();
    let env_del = Button::builder()
        .label("Delete")
        .sensitive(false)
        .css_classes(["destructive-action"])
        .build();
    let env_tb = GtkBox::new(Orientation::Horizontal, 8);
    env_tb.append(&env_search);
    env_tb.append(&env_add);
    env_tb.append(&env_edit);
    env_tb.append(&env_del);
    let env_filter = Rc::new(RefCell::new(String::new()));
    env_search.connect_changed({
        let env_filter = Rc::clone(&env_filter);
        let state = Rc::clone(&state);
        let list = env_list.clone();
        move |e| {
            *env_filter.borrow_mut() = e.text().to_string();
            render_env_list(&state, &list, &env_filter.borrow());
        }
    });
    env_list.connect_row_selected({
        let env_edit = env_edit.clone();
        let env_del = env_del.clone();
        move |_, row| {
            let on = row.is_some();
            env_edit.set_sensitive(on);
            env_del.set_sensitive(on);
        }
    });

    env_add.connect_clicked({
        let parent = parent.clone().upcast::<Window>();
        let state = Rc::clone(&state);
        let reload = Rc::clone(&reload);
        let status = Rc::clone(&status);
        let realtime = Rc::clone(&realtime);
        move |_| {
            let path = state.borrow().as_ref().map(|c| c.config_path.clone());
            let Some(path) = path else {
                status("Load a config first.".into());
                return;
            };
            show_env_dialog(&parent, None, Some(path), Rc::clone(&realtime), {
                let reload = Rc::clone(&reload);
                let status = Rc::clone(&status);
                move |msg| {
                    status(msg);
                    reload();
                }
            });
        }
    });
    let open_env_edit = {
        let parent = parent.clone().upcast::<Window>();
        let state = Rc::clone(&state);
        let list = env_list.clone();
        let reload = Rc::clone(&reload);
        let status = Rc::clone(&status);
        let realtime = Rc::clone(&realtime);
        Rc::new(move || {
            let Some(row) = list.selected_row() else {
                return;
            };
            let name = row
                .widget_name()
                .strip_prefix("env-")
                .unwrap_or("")
                .to_string();
            let var = state
                .borrow()
                .as_ref()
                .and_then(|c| c.env.iter().find(|e| e.name == name).cloned());
            let Some(var) = var else {
                return;
            };
            show_env_dialog(&parent, Some(var), None, Rc::clone(&realtime), {
                let reload = Rc::clone(&reload);
                let status = Rc::clone(&status);
                move |msg| {
                    status(msg);
                    reload();
                }
            });
        })
    };
    env_edit.connect_clicked({
        let open = Rc::clone(&open_env_edit);
        move |_| open()
    });
    env_list.connect_row_activated({
        let open = Rc::clone(&open_env_edit);
        move |_, _| open()
    });
    env_del.connect_clicked({
        let state = Rc::clone(&state);
        let list = env_list.clone();
        let reload = Rc::clone(&reload);
        let status = Rc::clone(&status);
        move |_| {
            let Some(row) = list.selected_row() else {
                return;
            };
            let name = row
                .widget_name()
                .strip_prefix("env-")
                .unwrap_or("")
                .to_string();
            let var = state
                .borrow()
                .as_ref()
                .and_then(|c| c.env.iter().find(|e| e.name == name).cloned());
            let Some(var) = var else {
                return;
            };
            match writer::delete_env(&var) {
                Ok(r) => {
                    status(format!("Deleted env `{}` from {}", var.name, r.path));
                    reload();
                }
                Err(e) => status(format!("Delete failed: {e}")),
            }
        }
    });

    let env_scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .child(&env_list)
        .build();
    let env_page = dialog::page_shell(
        "Environment",
        "hl.env(\"KEY\", \"VALUE\") — process environment for Hyprland and child processes.",
        &env_tb,
        &env_scroll,
    );

    // --- Submaps ---
    let sm_search = Entry::builder()
        .placeholder_text("Filter submaps…")
        .hexpand(true)
        .build();
    let sm_add = Button::builder().label("Add").build();
    let sm_del = Button::builder()
        .label("Delete")
        .sensitive(false)
        .css_classes(["destructive-action"])
        .build();
    let sm_tb = GtkBox::new(Orientation::Horizontal, 8);
    sm_tb.append(&sm_search);
    sm_tb.append(&sm_add);
    sm_tb.append(&sm_del);
    let sm_filter = Rc::new(RefCell::new(String::new()));
    sm_search.connect_changed({
        let sm_filter = Rc::clone(&sm_filter);
        let state = Rc::clone(&state);
        let list = submap_list.clone();
        move |e| {
            *sm_filter.borrow_mut() = e.text().to_string();
            render_submap_list(&state, &list, &sm_filter.borrow());
        }
    });
    submap_list.connect_row_selected({
        let sm_del = sm_del.clone();
        move |_, row| sm_del.set_sensitive(row.is_some())
    });
    sm_add.connect_clicked({
        let parent = parent.clone().upcast::<Window>();
        let state = Rc::clone(&state);
        let reload = Rc::clone(&reload);
        let status = Rc::clone(&status);
        move |_| {
            let path = state.borrow().as_ref().map(|c| c.config_path.clone());
            let Some(path) = path else {
                status("Load a config first.".into());
                return;
            };
            show_add_submap_dialog(&parent, path, {
                let reload = Rc::clone(&reload);
                let status = Rc::clone(&status);
                move |msg| {
                    status(msg);
                    reload();
                }
            });
        }
    });
    sm_del.connect_clicked({
        let state = Rc::clone(&state);
        let list = submap_list.clone();
        let reload = Rc::clone(&reload);
        let status = Rc::clone(&status);
        move |_| {
            let Some(row) = list.selected_row() else {
                return;
            };
            let id = row
                .widget_name()
                .strip_prefix("submap-")
                .and_then(|s| s.parse().ok())
                .unwrap_or(usize::MAX);
            let item = state
                .borrow()
                .as_ref()
                .and_then(|c| c.submap_by_id(id).cloned());
            let Some(item) = item else {
                return;
            };
            match writer::delete_submap(&item) {
                Ok(r) => {
                    status(format!("Deleted submap `{}` from {}", item.name, r.path));
                    reload();
                }
                Err(e) => status(format!("Delete failed: {e}")),
            }
        }
    });

    let sm_scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .child(&submap_list)
        .build();
    let submap_page = dialog::page_shell(
        "Submaps",
        "hl.define_submap — create modes (e.g. resize). Add binds into a submap from Binds (filter + Add).",
        &sm_tb,
        &sm_scroll,
    );

    EnvSubmapPages {
        env_page,
        env_list,
        submap_page,
        submap_list,
    }
}

pub fn render_env_list(
    state: &Rc<RefCell<Option<BindCollection>>>,
    list: &ListBox,
    filter: &str,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let borrowed = state.borrow();
    let Some(c) = borrowed.as_ref() else {
        return;
    };
    for var in &c.env {
        if var.matches(filter) {
            list.append(&env_row(var));
        }
    }
}

pub fn render_submap_list(
    state: &Rc<RefCell<Option<BindCollection>>>,
    list: &ListBox,
    filter: &str,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let borrowed = state.borrow();
    let Some(c) = borrowed.as_ref() else {
        return;
    };
    for s in &c.submaps {
        if s.matches(filter) {
            list.append(&submap_row(s));
        }
    }
}

fn env_row(var: &EnvVar) -> ListBoxRow {
    let col = dialog::list_row_column();
    col.append(&dialog::row_title(&var.name));
    col.append(&dialog::row_sub(&format!("\"{}\"", var.value)));
    col.append(&dialog::row_meta(&format!(
        "{}:{}",
        std::path::Path::new(&var.source_file)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&var.source_file),
        var.source_line
    )));
    ListBoxRow::builder()
        .child(&col)
        .activatable(true)
        .name(format!("env-{}", var.name))
        .build()
}

fn submap_row(s: &Submap) -> ListBoxRow {
    let col = dialog::list_row_column();
    col.append(&dialog::row_title(&s.name));
    col.append(&dialog::row_body(&s.label()));
    if !s.source_file.is_empty() {
        col.append(&dialog::row_meta(&format!(
            "{}:{}",
            std::path::Path::new(&s.source_file)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&s.source_file),
            s.source_line
        )));
    }
    ListBoxRow::builder()
        .child(&col)
        .activatable(true)
        .name(format!("submap-{}", s.id))
        .build()
}

fn field_block(title: &str, entry: &Entry) -> GtkBox {
    let block = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .build();
    block.append(
        &Label::builder()
            .label(title)
            .halign(gtk4::Align::Start)
            .css_classes(["dim-label", "caption"])
            .build(),
    );
    block.append(entry);
    block
}

fn show_env_dialog<F>(
    parent: &impl IsA<Window>,
    existing: Option<EnvVar>,
    config_path: Option<PathBuf>,
    realtime: Rc<Cell<bool>>,
    on_saved: F,
) where
    F: Fn(String) + 'static,
{
    let is_new = existing.is_none();
    let editor = Window::builder()
        .title(if is_new { "Add env" } else { "Edit env" })
        .transient_for(parent)
        .modal(true)
        .default_width(480)
        .build();

    let name_entry = Entry::builder()
        .text(existing.as_ref().map(|v| v.name.as_str()).unwrap_or(""))
        .placeholder_text("XCURSOR_SIZE")
        .build();
    let value_entry = Entry::builder()
        .text(existing.as_ref().map(|v| v.value.as_str()).unwrap_or(""))
        .placeholder_text("24")
        .build();

    let status = Label::builder()
        .label("Writes hl.env(\"KEY\", \"VALUE\") into a managed section (or in place).")
        .wrap(true)
        .css_classes(["dim-label"])
        .build();
    let cancel = Button::builder().label("Cancel").build();
    let save = Button::builder()
        .label("Save")
        .css_classes(["suggested-action"])
        .build();
    let actions = dialog::action_buttons(&cancel, &save);

    let form = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .margin_top(16)
        .margin_bottom(8)
        .margin_start(16)
        .margin_end(16)
        .build();
    form.append(&field_block("Name", &name_entry));
    form.append(&field_block("Value", &value_entry));
    dialog::mount_sticky_dialog(&editor, &form, Some(&status), &actions);

    cancel.connect_clicked({
        let editor = editor.clone();
        move |_| editor.close()
    });

    let on_saved = Rc::new(on_saved);
    let try_save: Rc<dyn Fn(bool)> = {
        let editor = editor.clone();
        let existing = existing.clone();
        let config_path = config_path.clone();
        let name_entry = name_entry.clone();
        let value_entry = value_entry.clone();
        let status = status.clone();
        let on_saved = Rc::clone(&on_saved);
        Rc::new(move |close_on_success: bool| {
            let name = name_entry.text().to_string();
            let value = value_entry.text().to_string();
            let result = if let Some(var) = existing.as_ref() {
                writer::save_env(var, &name, &value)
                    .map(|r| format!("Saved env `{name}` in {}", r.path))
            } else if let Some(path) = config_path.as_ref() {
                writer::add_env(path, &name, &value)
                    .map(|r| format!("Added env `{name}` in {}", r.path))
            } else {
                Err(writer::WriteError::Invalid("missing path".into()))
            };
            match result {
                Ok(msg) => {
                    on_saved(msg);
                    if close_on_success {
                        editor.close();
                    }
                }
                Err(e) => status.set_text(&format!("Save failed: {e}")),
            }
        })
    };
    save.connect_clicked({
        let try_save = Rc::clone(&try_save);
        move |_| try_save(true)
    });
    if !is_new {
        crate::extra_ui::wire_dialog_autosave(
            &realtime,
            &try_save,
            &[&name_entry, &value_entry],
            &[],
        );
    }
    editor.present();
}

fn show_add_submap_dialog<F>(parent: &impl IsA<Window>, config_path: PathBuf, on_saved: F)
where
    F: Fn(String) + 'static,
{
    let editor = Window::builder()
        .title("Add submap")
        .transient_for(parent)
        .modal(true)
        .default_width(420)
        .build();
    let name_entry = Entry::builder()
        .placeholder_text("resize")
        .build();
    let status = Label::builder()
        .label("Creates an empty hl.define_submap in the managed-submaps section. Then Add binds with that submap selected.")
        .wrap(true)
        .css_classes(["dim-label"])
        .build();
    let cancel = Button::builder().label("Cancel").build();
    let save = Button::builder()
        .label("Create")
        .css_classes(["suggested-action"])
        .build();
    let actions = dialog::action_buttons(&cancel, &save);
    let form = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .margin_top(16)
        .margin_bottom(8)
        .margin_start(16)
        .margin_end(16)
        .build();
    form.append(&field_block("Name", &name_entry));
    dialog::mount_sticky_dialog(&editor, &form, Some(&status), &actions);
    cancel.connect_clicked({
        let editor = editor.clone();
        move |_| editor.close()
    });
    save.connect_clicked({
        let editor = editor.clone();
        let status = status.clone();
        move |_| {
            let name = name_entry.text().to_string();
            match writer::add_submap(&config_path, &name) {
                Ok(r) => {
                    on_saved(format!("Created submap `{name}` in {}", r.path));
                    editor.close();
                }
                Err(e) => status.set_text(&format!("Failed: {e}")),
            }
        }
    });
    editor.present();
}
