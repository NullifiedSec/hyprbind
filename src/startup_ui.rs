//! Startup / autostart page — `hl.on("hyprland.start")` + `hl.exec_cmd`.

use crate::bind::BindCollection;
use crate::dialog;
use crate::startup::StartupEntry;
use crate::writer;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, DropDown, Entry, Label, ListBox, ListBoxRow, Orientation, PolicyType,
    ScrolledWindow, Window,
};
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

pub type StatusFn = Rc<dyn Fn(String)>;
pub type ReloadFn = Rc<dyn Fn()>;

pub struct StartupPage {
    pub page: GtkBox,
    pub list: ListBox,
}

const WHEN_LABELS: &[&str] = &["On start (once)", "On reload", "On shutdown"];

fn when_from_index(idx: u32) -> &'static str {
    match idx {
        1 => "reload",
        2 => "shutdown",
        _ => "start",
    }
}

fn index_from_when(when: &str) -> u32 {
    match when {
        "reload" => 1,
        "shutdown" => 2,
        _ => 0,
    }
}

pub fn build_startup_page(
    parent: &impl IsA<Window>,
    state: Rc<RefCell<Option<BindCollection>>>,
    reload: ReloadFn,
    status: StatusFn,
    realtime: Rc<Cell<bool>>,
) -> StartupPage {
    let list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();

    let search = Entry::builder()
        .placeholder_text("Filter startup commands…")
        .hexpand(true)
        .build();
    let add_btn = Button::builder()
        .label("Add")
        .css_classes(["suggested-action"])
        .build();
    let edit_btn = Button::builder().label("Edit").sensitive(false).build();
    let del_btn = Button::builder()
        .label("Delete")
        .sensitive(false)
        .css_classes(["destructive-action"])
        .build();

    let tb = GtkBox::new(Orientation::Horizontal, 8);
    tb.append(&search);
    tb.append(&add_btn);
    tb.append(&edit_btn);
    tb.append(&del_btn);

    let filter = Rc::new(RefCell::new(String::new()));
    search.connect_changed({
        let filter = Rc::clone(&filter);
        let state = Rc::clone(&state);
        let list = list.clone();
        move |e| {
            *filter.borrow_mut() = e.text().to_string();
            render_startup_list(&state, &list, &filter.borrow());
        }
    });

    list.connect_row_selected({
        let edit_btn = edit_btn.clone();
        let del_btn = del_btn.clone();
        move |_, row| {
            let on = row.is_some();
            edit_btn.set_sensitive(on);
            del_btn.set_sensitive(on);
        }
    });

    let open_editor = {
        let parent = parent.clone().upcast::<Window>();
        let state = Rc::clone(&state);
        let reload = Rc::clone(&reload);
        let status = Rc::clone(&status);
        let realtime = Rc::clone(&realtime);
        let list = list.clone();
        Rc::new(move |existing: Option<StartupEntry>| {
            let path = state.borrow().as_ref().map(|c| c.config_path.clone());
            let Some(path) = path else {
                status("Load a config first.".into());
                return;
            };
            show_startup_dialog(
                &parent,
                existing,
                path,
                Rc::clone(&realtime),
                {
                    let reload = Rc::clone(&reload);
                    let status = Rc::clone(&status);
                    move |msg| {
                        status(msg);
                        reload();
                    }
                },
            );
            let _ = &list;
        })
    };

    add_btn.connect_clicked({
        let open_editor = Rc::clone(&open_editor);
        move |_| open_editor(None)
    });

    edit_btn.connect_clicked({
        let state = Rc::clone(&state);
        let list = list.clone();
        let open_editor = Rc::clone(&open_editor);
        move |_| {
            let Some(row) = list.selected_row() else {
                return;
            };
            let id = row
                .widget_name()
                .strip_prefix("startup-")
                .and_then(|s| s.parse().ok())
                .unwrap_or(usize::MAX);
            let entry = state
                .borrow()
                .as_ref()
                .and_then(|c| c.startup_by_id(id).cloned());
            if let Some(entry) = entry {
                open_editor(Some(entry));
            }
        }
    });

    list.connect_row_activated({
        let state = Rc::clone(&state);
        let open_editor = Rc::clone(&open_editor);
        move |_, row| {
            let id = row
                .widget_name()
                .strip_prefix("startup-")
                .and_then(|s| s.parse().ok())
                .unwrap_or(usize::MAX);
            let entry = state
                .borrow()
                .as_ref()
                .and_then(|c| c.startup_by_id(id).cloned());
            if let Some(entry) = entry {
                open_editor(Some(entry));
            }
        }
    });

    del_btn.connect_clicked({
        let state = Rc::clone(&state);
        let list = list.clone();
        let reload = Rc::clone(&reload);
        let status = Rc::clone(&status);
        move |_| {
            let Some(row) = list.selected_row() else {
                return;
            };
            let id = row
                .widget_name()
                .strip_prefix("startup-")
                .and_then(|s| s.parse().ok())
                .unwrap_or(usize::MAX);
            let entry = state
                .borrow()
                .as_ref()
                .and_then(|c| c.startup_by_id(id).cloned());
            let Some(entry) = entry else {
                return;
            };
            match writer::delete_startup(&entry) {
                Ok(r) => {
                    status(format!(
                        "Deleted startup `{}` from {}",
                        entry.command, r.path
                    ));
                    reload();
                }
                Err(e) => status(format!("Delete failed: {e}")),
            }
        }
    });

    let scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .child(&list)
        .build();

    let page = dialog::page_shell(
        "Startup",
        "Autostart commands via hl.exec_cmd. “On start” runs once with hyprland.start (like exec-once). “On reload” re-runs when config reloads. “On shutdown” runs on exit.",
        &tb,
        &scroll,
    );

    StartupPage { page, list }
}

pub fn render_startup_list(
    state: &Rc<RefCell<Option<BindCollection>>>,
    list: &ListBox,
    filter: &str,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let borrowed = state.borrow();
    let Some(collection) = borrowed.as_ref() else {
        return;
    };
    for entry in collection.startup.iter().filter(|e| e.matches(filter)) {
        list.append(&startup_row(entry));
    }
}

fn startup_row(entry: &StartupEntry) -> ListBoxRow {
    let col = dialog::list_row_column();
    col.append(&dialog::row_title(&entry.command));
    let mut sub = entry.when_label().to_string();
    if !entry.workspace.is_empty() {
        sub.push_str(" · workspace ");
        sub.push_str(&entry.workspace);
    }
    col.append(&dialog::row_sub(&sub));
    if !entry.source_file.is_empty() {
        let file = std::path::Path::new(&entry.source_file)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(entry.source_file.as_str());
        col.append(&dialog::row_meta(&format!("{file}:{}", entry.source_line)));
    }
    ListBoxRow::builder()
        .child(&col)
        .activatable(true)
        .name(format!("startup-{}", entry.id))
        .build()
}

fn field_block(label: &str, child: &impl IsA<gtk4::Widget>) -> GtkBox {
    let box_ = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .build();
    box_.append(
        &Label::builder()
            .label(label)
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-section"])
            .build(),
    );
    box_.append(child);
    box_
}

fn show_startup_dialog<F>(
    parent: &impl IsA<Window>,
    existing: Option<StartupEntry>,
    config_path: PathBuf,
    realtime: Rc<Cell<bool>>,
    on_saved: F,
) where
    F: Fn(String) + 'static,
{
    let is_new = existing.is_none();
    let editor = Window::builder()
        .title(if is_new {
            "Add startup command"
        } else {
            "Edit startup command"
        })
        .transient_for(parent)
        .modal(true)
        .default_width(560)
        .default_height(420)
        .build();

    let cmd_entry = Entry::builder()
        .text(existing.as_ref().map(|e| e.command.as_str()).unwrap_or(""))
        .placeholder_text("waybar")
        .hexpand(true)
        .build();
    let ws_entry = Entry::builder()
        .text(
            existing
                .as_ref()
                .map(|e| e.workspace.as_str())
                .unwrap_or(""),
        )
        .placeholder_text("optional — e.g. 2 silent")
        .hexpand(true)
        .build();
    let when_dd = DropDown::from_strings(WHEN_LABELS);
    when_dd.set_selected(index_from_when(
        existing.as_ref().map(|e| e.when.as_str()).unwrap_or("start"),
    ));

    let hint = Label::builder()
        .label("On start → hl.on(\"hyprland.start\", …). On reload → top-level hl.exec_cmd (runs again on hyprctl reload). Workspace is passed as exec opts when set.")
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
        .spacing(14)
        .margin_top(20)
        .margin_bottom(16)
        .margin_start(20)
        .margin_end(20)
        .build();
    form.append(&field_block("Command", &cmd_entry));
    form.append(&field_block("When", &when_dd));
    form.append(&field_block("Workspace (optional)", &ws_entry));
    form.append(&hint);
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
        let cmd_entry = cmd_entry.clone();
        let ws_entry = ws_entry.clone();
        let when_dd = when_dd.clone();
        let status = status.clone();
        let on_saved = Rc::clone(&on_saved);
        Rc::new(move |close_on_success: bool| {
            let command = cmd_entry.text().to_string();
            let workspace = ws_entry.text().to_string();
            let when = when_from_index(when_dd.selected());
            let result = if let Some(entry) = existing.as_ref() {
                writer::save_startup(entry, &command, when, &workspace)
                    .map(|r| format!("Saved startup `{command}` in {}", r.path))
            } else {
                writer::add_startup(&config_path, &command, when, &workspace)
                    .map(|r| format!("Added startup `{command}` in {}", r.path))
            };
            match result {
                Ok(msg) => {
                    on_saved(msg);
                    if close_on_success {
                        editor.close();
                    }
                }
                Err(e) => {
                    status.set_text(&format!("Save failed: {e}"));
                    status.set_css_classes(&["error"]);
                }
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
            &[&cmd_entry, &ws_entry],
            &[],
        );
    }

    editor.present();
}
