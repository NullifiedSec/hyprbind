//! Rules + Settings group pages (workspace/layer rules, config, monitors, devices, animations, gestures).

use crate::bind::BindCollection;
use crate::clients;
use crate::curve_editor::{
    self, BezierEditor, BezierPoints, SpringEditor, SpringParams,
};
use crate::debounce::Debouncer;
use crate::dialog;
use crate::settings_config;
use crate::spec::SpecItem;
use crate::variables::ConfigVariable;
use crate::window_rules::WindowRule;
use crate::writer;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DropDown, Entry, Label, ListBox, ListBoxRow, Orientation,
    PolicyType, ScrolledWindow, StringList, Window,
};
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;

pub type StatusFn = Rc<dyn Fn(String)>;
pub type ReloadFn = Rc<dyn Fn()>;

fn clear_list(list: &ListBox) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
}

fn row_id(row: &ListBoxRow, prefix: &str) -> usize {
    row.widget_name()
        .strip_prefix(prefix)
        .and_then(|s| s.parse().ok())
        .unwrap_or(usize::MAX)
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

/// Debounce `try_save(false)` while Realtime is on. Ignores the first idle tick so
/// initial widget population does not write.
pub(crate) fn wire_dialog_autosave(
    realtime: &Rc<Cell<bool>>,
    try_save: &Rc<dyn Fn(bool)>,
    entries: &[&Entry],
    checks: &[&CheckButton],
) {
    let ready = Rc::new(Cell::new(false));
    let debouncer = Debouncer::new();
    let schedule = {
        let ready = Rc::clone(&ready);
        let realtime = Rc::clone(realtime);
        let debouncer = debouncer.clone();
        let try_save = Rc::clone(try_save);
        Rc::new(move || {
            if !ready.get() || !realtime.get() {
                return;
            }
            let realtime = Rc::clone(&realtime);
            let try_save = Rc::clone(&try_save);
            debouncer.schedule(move || {
                if realtime.get() {
                    try_save(false);
                }
            });
        })
    };
    for entry in entries {
        entry.connect_changed({
            let schedule = Rc::clone(&schedule);
            move |_| schedule()
        });
    }
    for check in checks {
        check.connect_toggled({
            let schedule = Rc::clone(&schedule);
            move |_| schedule()
        });
    }
    gtk4::glib::idle_add_local_once({
        let ready = Rc::clone(&ready);
        move || ready.set(true)
    });
}

fn spec_row(item: &SpecItem, prefix: &str) -> ListBoxRow {
    let col = dialog::list_row_column();
    col.append(&dialog::row_title(&item.display_name));
    col.append(&dialog::row_sub(&item.fields_label()));
    if !item.source_file.is_empty() {
        col.append(&dialog::row_meta(&item.source_label()));
    }
    ListBoxRow::builder()
        .child(&col)
        .activatable(true)
        .name(format!("{prefix}{}", item.id))
        .build()
}

pub struct RulesExtra {
    pub workspace_page: GtkBox,
    pub workspace_list: ListBox,
    pub layer_page: GtkBox,
    pub layer_list: ListBox,
}

/// Build workspace + layer rule pages (mounted under the Rules hub notebook).
pub fn build_rules_extra(
    parent: &impl IsA<Window>,
    state: Rc<RefCell<Option<BindCollection>>>,
    reload: ReloadFn,
    status: StatusFn,
    realtime: Rc<Cell<bool>>,
) -> RulesExtra {
    let workspace_list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();
    let layer_list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();

    let ws_search = Entry::builder()
        .placeholder_text("Filter workspace rules…")
        .hexpand(true)
        .build();
    let ws_add = Button::builder().label("Add").build();
    let ws_edit = Button::builder().label("Edit").sensitive(false).build();
    let ws_del = Button::builder()
        .label("Delete")
        .sensitive(false)
        .css_classes(["destructive-action"])
        .build();
    let ws_tb = GtkBox::new(Orientation::Horizontal, 8);
    ws_tb.append(&ws_search);
    ws_tb.append(&ws_add);
    ws_tb.append(&ws_edit);
    ws_tb.append(&ws_del);
    let ws_scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .child(&workspace_list)
        .build();
    let workspace_page = dialog::page_shell(
        "Workspace rules",
        "Per-workspace gaps, layout, and monitor binding. New rules go in a managed section.",
        &ws_tb,
        &ws_scroll,
    );

    let ly_search = Entry::builder()
        .placeholder_text("Filter layer rules…")
        .hexpand(true)
        .build();
    let ly_add = Button::builder().label("Add").build();
    let ly_edit = Button::builder().label("Edit").sensitive(false).build();
    let ly_del = Button::builder()
        .label("Delete")
        .sensitive(false)
        .css_classes(["destructive-action"])
        .build();
    let ly_tb = GtkBox::new(Orientation::Horizontal, 8);
    ly_tb.append(&ly_search);
    ly_tb.append(&ly_add);
    ly_tb.append(&ly_edit);
    ly_tb.append(&ly_del);
    let ly_scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .child(&layer_list)
        .build();
    let layer_page = dialog::page_shell(
        "Layer rules",
        "Rules for bars and launchers. Pick namespaces from running layers.",
        &ly_tb,
        &ly_scroll,
    );

    workspace_list.connect_row_selected({
        let ws_edit = ws_edit.clone();
        let ws_del = ws_del.clone();
        move |_, row| {
            let on = row.is_some();
            ws_edit.set_sensitive(on);
            ws_del.set_sensitive(on);
        }
    });
    layer_list.connect_row_selected({
        let ly_edit = ly_edit.clone();
        let ly_del = ly_del.clone();
        move |_, row| {
            let on = row.is_some();
            ly_edit.set_sensitive(on);
            ly_del.set_sensitive(on);
        }
    });

    let ws_filter = Rc::new(RefCell::new(String::new()));
    ws_search.connect_changed({
        let state = Rc::clone(&state);
        let list = workspace_list.clone();
        let filter = Rc::clone(&ws_filter);
        move |e| {
            *filter.borrow_mut() = e.text().to_string();
            render_workspace_list(&state, &list, &filter.borrow());
        }
    });
    let ly_filter = Rc::new(RefCell::new(String::new()));
    ly_search.connect_changed({
        let state = Rc::clone(&state);
        let list = layer_list.clone();
        let filter = Rc::clone(&ly_filter);
        move |e| {
            *filter.borrow_mut() = e.text().to_string();
            render_layer_list(&state, &list, &filter.borrow());
        }
    });

    ws_add.connect_clicked({
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
            show_workspace_rule_dialog(&parent, None, Some(path), Rc::clone(&realtime), {
                let reload = Rc::clone(&reload);
                let status = Rc::clone(&status);
                move |msg| {
                    status(msg);
                    reload();
                }
            });
        }
    });
    ws_edit.connect_clicked({
        let parent = parent.clone().upcast::<Window>();
        let state = Rc::clone(&state);
        let list = workspace_list.clone();
        let reload = Rc::clone(&reload);
        let status = Rc::clone(&status);
        let realtime = Rc::clone(&realtime);
        move |_| {
            let Some(row) = list.selected_row() else {
                return;
            };
            let id = row_id(&row, "ws-");
            let item = state
                .borrow()
                .as_ref()
                .and_then(|c| c.workspace_rule_by_id(id).cloned());
            let Some(item) = item else {
                return;
            };
            show_workspace_rule_dialog(&parent, Some(item), None, Rc::clone(&realtime), {
                let reload = Rc::clone(&reload);
                let status = Rc::clone(&status);
                move |msg| {
                    status(msg);
                    reload();
                }
            });
        }
    });
    workspace_list.connect_row_activated({
        let ws_edit = ws_edit.clone();
        move |_, _| ws_edit.emit_clicked()
    });
    ws_del.connect_clicked({
        let state = Rc::clone(&state);
        let list = workspace_list.clone();
        let reload = Rc::clone(&reload);
        let status = Rc::clone(&status);
        move |_| {
            let Some(row) = list.selected_row() else {
                return;
            };
            let id = row_id(&row, "ws-");
            let item = state
                .borrow()
                .as_ref()
                .and_then(|c| c.workspace_rule_by_id(id).cloned());
            let Some(item) = item else {
                return;
            };
            match writer::delete_workspace_rule(&item) {
                Ok(r) => {
                    status(format!("Deleted workspace rule from {}", r.path));
                    reload();
                }
                Err(e) => status(format!("Delete failed: {e}")),
            }
        }
    });

    ly_add.connect_clicked({
        let parent = parent.clone().upcast::<Window>();
        let state = Rc::clone(&state);
        let reload = Rc::clone(&reload);
        let status = Rc::clone(&status);
        let realtime = Rc::clone(&realtime);
        move |_| {
            let (path, vars) = {
                let s = state.borrow();
                let Some(c) = s.as_ref() else {
                    status("Load a config first.".into());
                    return;
                };
                (c.config_path.clone(), c.variables.clone())
            };
            show_layer_rule_dialog(&parent, None, Some(path), vars, Rc::clone(&realtime), {
                let reload = Rc::clone(&reload);
                let status = Rc::clone(&status);
                move |msg| {
                    status(msg);
                    reload();
                }
            });
        }
    });
    ly_edit.connect_clicked({
        let parent = parent.clone().upcast::<Window>();
        let state = Rc::clone(&state);
        let list = layer_list.clone();
        let reload = Rc::clone(&reload);
        let status = Rc::clone(&status);
        let realtime = Rc::clone(&realtime);
        move |_| {
            let Some(row) = list.selected_row() else {
                return;
            };
            let id = row_id(&row, "layer-");
            let (rule, vars) = {
                let s = state.borrow();
                let Some(c) = s.as_ref() else {
                    return;
                };
                (c.layer_rule_by_id(id).cloned(), c.variables.clone())
            };
            let Some(rule) = rule else {
                return;
            };
            show_layer_rule_dialog(&parent, Some(rule), None, vars, Rc::clone(&realtime), {
                let reload = Rc::clone(&reload);
                let status = Rc::clone(&status);
                move |msg| {
                    status(msg);
                    reload();
                }
            });
        }
    });
    layer_list.connect_row_activated({
        let ly_edit = ly_edit.clone();
        move |_, _| ly_edit.emit_clicked()
    });
    ly_del.connect_clicked({
        let state = Rc::clone(&state);
        let list = layer_list.clone();
        let reload = Rc::clone(&reload);
        let status = Rc::clone(&status);
        move |_| {
            let Some(row) = list.selected_row() else {
                return;
            };
            let id = row_id(&row, "layer-");
            let rule = state
                .borrow()
                .as_ref()
                .and_then(|c| c.layer_rule_by_id(id).cloned());
            let Some(rule) = rule else {
                return;
            };
            match writer::delete_layer_rule(&rule) {
                Ok(r) => {
                    status(format!("Deleted layer rule from {}", r.path));
                    reload();
                }
                Err(e) => status(format!("Delete failed: {e}")),
            }
        }
    });

    RulesExtra {
        workspace_page,
        workspace_list,
        layer_page,
        layer_list,
    }
}

pub fn render_workspace_list(
    state: &Rc<RefCell<Option<BindCollection>>>,
    list: &ListBox,
    filter: &str,
) {
    clear_list(list);
    let borrowed = state.borrow();
    let Some(c) = borrowed.as_ref() else {
        return;
    };
    for item in &c.workspace_rules {
        if item.matches(filter) {
            list.append(&spec_row(item, "ws-"));
        }
    }
}

pub fn render_layer_list(
    state: &Rc<RefCell<Option<BindCollection>>>,
    list: &ListBox,
    filter: &str,
) {
    clear_list(list);
    let borrowed = state.borrow();
    let Some(c) = borrowed.as_ref() else {
        return;
    };
    let q = filter.to_lowercase();
    for rule in &c.layer_rules {
        if q.is_empty() || rule.matches(&q) {
            list.append(&layer_row(rule));
        }
    }
}

fn layer_row(rule: &WindowRule) -> ListBoxRow {
    let col = dialog::list_row_column();
    col.append(&dialog::row_title(&rule.display_name));
    col.append(&dialog::row_sub(&format!("match  {}", rule.match_label())));
    col.append(&dialog::row_body(&format!("effects  {}", rule.effects_label())));
    if !rule.source_file.is_empty() {
        col.append(&dialog::row_meta(&rule.source_label()));
    }
    ListBoxRow::builder()
        .child(&col)
        .activatable(true)
        .name(format!("layer-{}", rule.id))
        .build()
}

fn show_workspace_rule_dialog<F>(
    parent: &impl IsA<Window>,
    existing: Option<SpecItem>,
    config_path: Option<PathBuf>,
    realtime: Rc<Cell<bool>>,
    on_saved: F,
) where
    F: Fn(String) + 'static,
{
    let is_new = existing.is_none();
    let editor = Window::builder()
        .title(if existing.is_some() {
            "Edit workspace rule"
        } else {
            "Add workspace rule"
        })
        .transient_for(parent)
        .modal(true)
        .default_width(480)
        .build();

    let workspace = Entry::builder()
        .text(
            existing
                .as_ref()
                .and_then(|i| i.get_string("workspace"))
                .unwrap_or_default(),
        )
        .placeholder_text("2  or  name:coding  or  w[tv1]")
        .build();
    let monitor = Entry::builder()
        .text(
            existing
                .as_ref()
                .and_then(|i| i.get_string("monitor"))
                .unwrap_or_default(),
        )
        .placeholder_text("DP-1")
        .build();
    let pick_mon = Button::builder().label("Pick…").build();
    let mon_row = GtkBox::new(Orientation::Horizontal, 8);
    mon_row.append(&monitor);
    mon_row.append(&pick_mon);

    let gaps_in = Entry::builder()
        .text(
            existing
                .as_ref()
                .and_then(|i| i.get_string("gaps_in"))
                .unwrap_or_default(),
        )
        .placeholder_text("5")
        .build();
    let gaps_out = Entry::builder()
        .text(
            existing
                .as_ref()
                .and_then(|i| i.get_string("gaps_out"))
                .unwrap_or_default(),
        )
        .placeholder_text("20")
        .build();
    let layout = Entry::builder()
        .text(
            existing
                .as_ref()
                .and_then(|i| i.get_string("layout"))
                .unwrap_or_default(),
        )
        .placeholder_text("dwindle")
        .build();
    let default_cb = CheckButton::builder()
        .label("Default on monitor")
        .active(existing.as_ref().and_then(|i| i.get_bool("default")).unwrap_or(false))
        .build();
    let persistent_cb = CheckButton::builder()
        .label("Persistent")
        .active(
            existing
                .as_ref()
                .and_then(|i| i.get_bool("persistent"))
                .unwrap_or(false),
        )
        .build();
    let no_border = CheckButton::builder()
        .label("No border")
        .active(
            existing
                .as_ref()
                .and_then(|i| i.get_bool("no_border"))
                .unwrap_or(false),
        )
        .build();
    let no_shadow = CheckButton::builder()
        .label("No shadow")
        .active(
            existing
                .as_ref()
                .and_then(|i| i.get_bool("no_shadow"))
                .unwrap_or(false),
        )
        .build();

    let status = Label::builder()
        .label(if is_new {
            "New rules append to a managed section."
        } else {
            "Realtime autosaves ~0.8s after you stop typing."
        })
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
    form.append(&field_block("Workspace", &workspace));
    {
        let b = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .build();
        b.append(&Label::builder().label("Monitor").halign(gtk4::Align::Start).build());
        b.append(&mon_row);
        form.append(&b);
    }
    form.append(&field_block("gaps_in", &gaps_in));
    form.append(&field_block("gaps_out", &gaps_out));
    form.append(&field_block("layout", &layout));
    form.append(&default_cb);
    form.append(&persistent_cb);
    form.append(&no_border);
    form.append(&no_shadow);
    dialog::mount_sticky_dialog(&editor, &form, Some(&status), &actions);

    pick_mon.connect_clicked({
        let editor = editor.clone();
        let monitor = monitor.clone();
        move |_| show_string_picker(&editor, "Monitors", list_monitor_names(), {
            let monitor = monitor.clone();
            move |s| monitor.set_text(&s)
        })
    });
    cancel.connect_clicked({
        let editor = editor.clone();
        move |_| editor.close()
    });

    let on_saved = Rc::new(on_saved);
    let try_save: Rc<dyn Fn(bool)> = {
        let editor = editor.clone();
        let existing = existing.clone();
        let config_path = config_path.clone();
        let workspace = workspace.clone();
        let monitor = monitor.clone();
        let gaps_in = gaps_in.clone();
        let gaps_out = gaps_out.clone();
        let layout = layout.clone();
        let default_cb = default_cb.clone();
        let persistent_cb = persistent_cb.clone();
        let no_border = no_border.clone();
        let no_shadow = no_shadow.clone();
        let status = status.clone();
        let on_saved = Rc::clone(&on_saved);
        Rc::new(move |close_on_success: bool| {
            let mut fields = BTreeMap::new();
            let ws = workspace.text().to_string().trim().to_string();
            if ws.is_empty() {
                if close_on_success {
                    status.set_text("Workspace is required.");
                }
                return;
            }
            fields.insert("workspace".into(), json!(ws));
            push_opt_string(&mut fields, "monitor", &monitor.text());
            push_opt_number_or_string(&mut fields, "gaps_in", &gaps_in.text());
            push_opt_number_or_string(&mut fields, "gaps_out", &gaps_out.text());
            push_opt_string(&mut fields, "layout", &layout.text());
            if default_cb.is_active() {
                fields.insert("default".into(), json!(true));
            }
            if persistent_cb.is_active() {
                fields.insert("persistent".into(), json!(true));
            }
            if no_border.is_active() {
                fields.insert("no_border".into(), json!(true));
            }
            if no_shadow.is_active() {
                fields.insert("no_shadow".into(), json!(true));
            }
            let result = if let Some(item) = existing.as_ref() {
                writer::save_workspace_rule(item, &fields)
                    .map(|r| format!("Saved workspace rule in {}", r.path))
            } else if let Some(path) = config_path.as_ref() {
                writer::add_workspace_rule(path, &fields)
                    .map(|r| format!("Added workspace rule in {}", r.path))
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
        wire_dialog_autosave(
            &realtime,
            &try_save,
            &[
                &workspace, &monitor, &gaps_in, &gaps_out, &layout,
            ],
            &[&default_cb, &persistent_cb, &no_border, &no_shadow],
        );
    }

    editor.present();
}

fn show_layer_rule_dialog<F>(
    parent: &impl IsA<Window>,
    existing: Option<WindowRule>,
    config_path: Option<PathBuf>,
    _vars: Vec<ConfigVariable>,
    realtime: Rc<Cell<bool>>,
    on_saved: F,
) where
    F: Fn(String) + 'static,
{
    let is_new = existing.is_none();
    let editor = Window::builder()
        .title(if existing.is_some() {
            "Edit layer rule"
        } else {
            "Add layer rule"
        })
        .transient_for(parent)
        .modal(true)
        .default_width(480)
        .build();

    let name_entry = Entry::builder()
        .text(existing.as_ref().map(|r| r.name.as_str()).unwrap_or(""))
        .placeholder_text("optional name")
        .build();
    let ns_entry = Entry::builder()
        .text(
            existing
                .as_ref()
                .and_then(|r| r.match_string("namespace"))
                .unwrap_or_default(),
        )
        .placeholder_text("waybar")
        .hexpand(true)
        .build();
    let pick_ns = Button::builder().label("From layers…").build();
    let ns_row = GtkBox::new(Orientation::Horizontal, 8);
    ns_row.append(&ns_entry);
    ns_row.append(&pick_ns);

    let blur = CheckButton::builder()
        .label("Blur")
        .active(existing.as_ref().and_then(|r| r.effect_bool("blur")).unwrap_or(false))
        .build();
    let no_anim = CheckButton::builder()
        .label("No animation")
        .active(
            existing
                .as_ref()
                .and_then(|r| r.effect_bool("no_anim"))
                .unwrap_or(false),
        )
        .build();
    let dim = CheckButton::builder()
        .label("Dim around")
        .active(
            existing
                .as_ref()
                .and_then(|r| r.effect_bool("dim_around"))
                .unwrap_or(false),
        )
        .build();
    let order = Entry::builder()
        .text(
            existing
                .as_ref()
                .and_then(|r| r.effect_string("order"))
                .unwrap_or_default(),
        )
        .placeholder_text("0")
        .build();
    let animation = Entry::builder()
        .text(
            existing
                .as_ref()
                .and_then(|r| r.effect_string("animation"))
                .unwrap_or_default(),
        )
        .placeholder_text("fade")
        .build();

    let status = Label::builder()
        .label("")
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
    {
        let b = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .build();
        b.append(
            &Label::builder()
                .label("Namespace (regex)")
                .halign(gtk4::Align::Start)
                .build(),
        );
        b.append(&ns_row);
        form.append(&b);
    }
    form.append(&blur);
    form.append(&no_anim);
    form.append(&dim);
    form.append(&field_block("Order", &order));
    form.append(&field_block("Animation", &animation));
    dialog::mount_sticky_dialog(&editor, &form, Some(&status), &actions);

    pick_ns.connect_clicked({
        let editor = editor.clone();
        let ns_entry = ns_entry.clone();
        move |_| {
            let names = clients::list_layer_namespaces().unwrap_or_default();
            show_string_picker(&editor, "Layer namespaces", names, {
                let ns_entry = ns_entry.clone();
                move |s| ns_entry.set_text(&format!("^{s}$"))
            });
        }
    });
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
        let ns_entry = ns_entry.clone();
        let blur = blur.clone();
        let no_anim = no_anim.clone();
        let dim = dim.clone();
        let order = order.clone();
        let animation = animation.clone();
        let status = status.clone();
        let on_saved = Rc::clone(&on_saved);
        Rc::new(move |close_on_success: bool| {
            let mut match_props = BTreeMap::new();
            let ns = ns_entry.text().to_string().trim().to_string();
            if ns.is_empty() {
                if close_on_success {
                    status.set_text("Namespace is required.");
                }
                return;
            }
            match_props.insert("namespace".into(), json!(ns));
            let mut effects = BTreeMap::new();
            if blur.is_active() {
                effects.insert("blur".into(), json!(true));
            }
            if no_anim.is_active() {
                effects.insert("no_anim".into(), json!(true));
            }
            if dim.is_active() {
                effects.insert("dim_around".into(), json!(true));
            }
            push_opt_number_or_string(&mut effects, "order", &order.text());
            push_opt_string(&mut effects, "animation", &animation.text());
            if effects.is_empty() {
                if close_on_success {
                    status.set_text("Add at least one effect.");
                }
                return;
            }
            let name = name_entry.text().to_string();
            let result = if let Some(rule) = existing.as_ref() {
                writer::save_layer_rule(rule, &name, &match_props, &effects)
                    .map(|r| format!("Saved layer rule in {}", r.path))
            } else if let Some(path) = config_path.as_ref() {
                writer::add_layer_rule(path, &name, &match_props, &effects)
                    .map(|r| format!("Added layer rule in {}", r.path))
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
        wire_dialog_autosave(
            &realtime,
            &try_save,
            &[&name_entry, &ns_entry, &order, &animation],
            &[&blur, &no_anim, &dim],
        );
    }
    editor.present();
}

fn list_monitor_names() -> Vec<String> {
    clients::list_monitors()
        .unwrap_or_default()
        .into_iter()
        .map(|m| m.name)
        .collect()
}

fn show_string_picker<F>(parent: &impl IsA<Window>, title: &str, items: Vec<String>, on_pick: F)
where
    F: Fn(String) + 'static,
{
    let picker = Window::builder()
        .title(title)
        .transient_for(parent)
        .modal(true)
        .default_width(400)
        .default_height(360)
        .build();
    let list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();
    for item in &items {
        let lbl = Label::builder()
            .label(item)
            .halign(gtk4::Align::Start)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(10)
            .margin_end(10)
            .build();
        list.append(
            &ListBoxRow::builder()
                .child(&lbl)
                .activatable(true)
                .name(format!("pick-{item}"))
                .build(),
        );
    }
    let scroll = ScrolledWindow::builder()
        .vexpand(true)
        .child(&list)
        .build();
    let close = Button::builder().label("Close").build();
    let root = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    root.append(&scroll);
    root.append(&close);
    picker.set_child(Some(&root));
    close.connect_clicked({
        let picker = picker.clone();
        move |_| picker.close()
    });
    list.connect_row_activated({
        let picker = picker.clone();
        let on_pick = Rc::new(on_pick);
        move |_, row| {
            if let Some(s) = row.widget_name().strip_prefix("pick-") {
                on_pick(s.to_string());
                picker.close();
            }
        }
    });
    dialog::close_on_escape(&picker);
    picker.present();
}

fn push_opt_string(map: &mut BTreeMap<String, Value>, key: &str, raw: &str) {
    let v = raw.trim();
    if !v.is_empty() {
        map.insert(key.into(), json!(v));
    }
}

fn push_opt_number_or_string(map: &mut BTreeMap<String, Value>, key: &str, raw: &str) {
    let v = raw.trim();
    if v.is_empty() {
        return;
    }
    if let Ok(n) = v.parse::<i64>() {
        map.insert(key.into(), json!(n));
    } else if let Ok(n) = v.parse::<f64>() {
        map.insert(key.into(), json!(n));
    } else {
        map.insert(key.into(), json!(v));
    }
}

// --- Settings group ---

pub struct SettingsGroup {
    pub config_page: GtkBox,
    pub monitors_page: GtkBox,
    pub devices_page: GtkBox,
    pub animations_page: GtkBox,
    pub curves_page: GtkBox,
    pub gestures_page: GtkBox,
    pub monitor_list: ListBox,
    pub device_list: ListBox,
    pub gesture_list: ListBox,
    pub anim_list: ListBox,
    pub curve_list: ListBox,
    pub config_entries: Rc<RefCell<ConfigFormEntries>>,
    /// When true, Entry/Check changes from `fill_config_form` must not trigger autosave.
    pub config_filling: Rc<Cell<bool>>,
}

pub struct ConfigFormEntries {
    pub gaps_in: Entry,
    pub gaps_out: Entry,
    pub border_size: Entry,
    pub layout: Entry,
    pub active_border: Entry,
    pub inactive_border: Entry,
    pub resize_on_border: CheckButton,
    pub rounding: Entry,
    pub active_opacity: Entry,
    pub inactive_opacity: Entry,
    pub shadow_enabled: CheckButton,
    pub blur_enabled: CheckButton,
    pub blur_size: Entry,
    pub kb_layout: Entry,
    pub follow_mouse: Entry,
    pub sensitivity: Entry,
    pub natural_scroll: CheckButton,
    pub anims_enabled: CheckButton,
    pub force_wallpaper: Entry,
    pub disable_logo: CheckButton,
    pub vrr: Entry,
    pub dwindle_preserve: CheckButton,
    pub master_new_status: Entry,
}

pub fn build_settings_group(
    parent: &impl IsA<Window>,
    state: Rc<RefCell<Option<BindCollection>>>,
    reload: ReloadFn,
    status: StatusFn,
    realtime: Rc<Cell<bool>>,
) -> SettingsGroup {
    let config_filling = Rc::new(Cell::new(false));

    // Config page
    let (config_page, config_entries) = build_config_page(
        parent,
        Rc::clone(&state),
        Rc::clone(&reload),
        Rc::clone(&status),
        Rc::clone(&realtime),
        Rc::clone(&config_filling),
    );

    let monitor_list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();
    let device_list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();
    let gesture_list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();
    let anim_list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();
    let curve_list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();

    let monitors_page = build_spec_list_page(
        "Monitors",
        "hl.monitor — output, mode, position, scale. Pick outputs from hyprctl.",
        &monitor_list,
        parent,
        Rc::clone(&state),
        Rc::clone(&reload),
        Rc::clone(&status),
        SpecKind::Monitor,
        Rc::clone(&realtime),
    );
    let devices_page = build_spec_list_page(
        "Devices",
        "Per-device input overrides.",
        &device_list,
        parent,
        Rc::clone(&state),
        Rc::clone(&reload),
        Rc::clone(&status),
        SpecKind::Device,
        Rc::clone(&realtime),
    );
    let animations_page = build_spec_list_page(
        "Animations",
        "Animation leaves — pick curve + style, optional graph-backed curves under Curves.",
        &anim_list,
        parent,
        Rc::clone(&state),
        Rc::clone(&reload),
        Rc::clone(&status),
        SpecKind::Animation,
        Rc::clone(&realtime),
    );
    let curves_page = build_spec_list_page(
        "Curves",
        "Drag Bézier handles or tune spring physics — graphs preview the easing.",
        &curve_list,
        parent,
        Rc::clone(&state),
        Rc::clone(&reload),
        Rc::clone(&status),
        SpecKind::Curve,
        Rc::clone(&realtime),
    );
    let gestures_page = build_spec_list_page(
        "Gestures",
        "Trackpad gestures (string actions only).",
        &gesture_list,
        parent,
        Rc::clone(&state),
        Rc::clone(&reload),
        Rc::clone(&status),
        SpecKind::Gesture,
        Rc::clone(&realtime),
    );

    SettingsGroup {
        config_page,
        monitors_page,
        devices_page,
        animations_page,
        curves_page,
        gestures_page,
        monitor_list,
        device_list,
        gesture_list,
        anim_list,
        curve_list,
        config_entries,
        config_filling,
    }
}

#[derive(Clone, Copy)]
enum SpecKind {
    Monitor,
    Device,
    Gesture,
    Animation,
    Curve,
}

fn build_spec_list_page(
    title: &str,
    hint: &str,
    list: &ListBox,
    parent: &impl IsA<Window>,
    state: Rc<RefCell<Option<BindCollection>>>,
    reload: ReloadFn,
    status: StatusFn,
    kind: SpecKind,
    realtime: Rc<Cell<bool>>,
) -> GtkBox {
    let add = Button::builder().label("Add").build();
    let edit = Button::builder().label("Edit").sensitive(false).build();
    let del = Button::builder()
        .label("Delete")
        .sensitive(false)
        .css_classes(["destructive-action"])
        .build();
    let tb = GtkBox::new(Orientation::Horizontal, 8);
    tb.append(&Label::builder().label("").hexpand(true).build());
    tb.append(&add);
    tb.append(&edit);
    tb.append(&del);
    let scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .child(list)
        .build();
    let page = dialog::page_shell(title, hint, &tb, &scroll);

    list.connect_row_selected({
        let edit = edit.clone();
        let del = del.clone();
        move |_, row| {
            let on = row.is_some();
            edit.set_sensitive(on);
            del.set_sensitive(on);
        }
    });

    let prefix = match kind {
        SpecKind::Monitor => "mon-",
        SpecKind::Device => "dev-",
        SpecKind::Gesture => "ges-",
        SpecKind::Animation => "anim-",
        SpecKind::Curve => "curve-",
    };

    add.connect_clicked({
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
            show_kind_dialog(
                &parent,
                kind,
                None,
                Some(path),
                Rc::clone(&state),
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
        }
    });
    let open_edit = {
        let parent = parent.clone().upcast::<Window>();
        let state = Rc::clone(&state);
        let list = list.clone();
        let reload = Rc::clone(&reload);
        let status = Rc::clone(&status);
        let realtime = Rc::clone(&realtime);
        let prefix = prefix.to_string();
        Rc::new(move || {
            let Some(row) = list.selected_row() else {
                return;
            };
            let id = row_id(&row, &prefix);
            let item = {
                let s = state.borrow();
                let Some(c) = s.as_ref() else {
                    return;
                };
                match kind {
                    SpecKind::Monitor => c.monitor_by_id(id).cloned(),
                    SpecKind::Device => c.device_by_id(id).cloned(),
                    SpecKind::Gesture => c.gesture_by_id(id).cloned(),
                    SpecKind::Animation => c.animation_by_id(id).cloned(),
                    SpecKind::Curve => c.curve_by_id(id).cloned(),
                }
            };
            let Some(item) = item else {
                return;
            };
            show_kind_dialog(
                &parent,
                kind,
                Some(item),
                None,
                Rc::clone(&state),
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
        })
    };
    edit.connect_clicked({
        let open_edit = Rc::clone(&open_edit);
        move |_| open_edit()
    });
    list.connect_row_activated({
        let open_edit = Rc::clone(&open_edit);
        move |_, _| open_edit()
    });
    del.connect_clicked({
        let state = Rc::clone(&state);
        let list = list.clone();
        let reload = Rc::clone(&reload);
        let status = Rc::clone(&status);
        let prefix = prefix.to_string();
        move |_| {
            let Some(row) = list.selected_row() else {
                return;
            };
            let id = row_id(&row, &prefix);
            let item = {
                let s = state.borrow();
                let Some(c) = s.as_ref() else {
                    return;
                };
                match kind {
                    SpecKind::Monitor => c.monitor_by_id(id).cloned(),
                    SpecKind::Device => c.device_by_id(id).cloned(),
                    SpecKind::Gesture => c.gesture_by_id(id).cloned(),
                    SpecKind::Animation => c.animation_by_id(id).cloned(),
                    SpecKind::Curve => c.curve_by_id(id).cloned(),
                }
            };
            let Some(item) = item else {
                return;
            };
            let result = match kind {
                SpecKind::Monitor => writer::delete_monitor(&item),
                SpecKind::Device => writer::delete_device(&item),
                SpecKind::Gesture => writer::delete_gesture(&item),
                SpecKind::Animation => writer::delete_animation(&item),
                SpecKind::Curve => writer::delete_curve(&item),
            };
            match result {
                Ok(r) => {
                    status(format!("Deleted from {}", r.path));
                    reload();
                }
                Err(e) => status(format!("Delete failed: {e}")),
            }
        }
    });

    page
}

fn show_kind_dialog<F>(
    parent: &impl IsA<Window>,
    kind: SpecKind,
    existing: Option<SpecItem>,
    config_path: Option<PathBuf>,
    state: Rc<RefCell<Option<BindCollection>>>,
    realtime: Rc<Cell<bool>>,
    on_saved: F,
) where
    F: Fn(String) + 'static,
{
    match kind {
        SpecKind::Curve => show_curve_dialog(parent, existing, config_path, realtime, on_saved),
        SpecKind::Animation => {
            let curve_names = state
                .borrow()
                .as_ref()
                .map(|c| {
                    c.curves
                        .iter()
                        .map(|curve| curve.display_name.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            show_animation_dialog(parent, existing, config_path, curve_names, realtime, on_saved)
        }
        _ => show_spec_dialog(parent, kind, existing, config_path, realtime, on_saved),
    }
}

fn show_curve_dialog<F>(
    parent: &impl IsA<Window>,
    existing: Option<SpecItem>,
    config_path: Option<PathBuf>,
    realtime: Rc<Cell<bool>>,
    on_saved: F,
) where
    F: Fn(String) + 'static,
{
    let is_new = existing.is_none();
    let editor = Window::builder()
        .title(if is_new {
            "Add curve"
        } else {
            "Edit curve"
        })
        .transient_for(parent)
        .modal(true)
        .default_width(520)
        .default_height(720)
        .build();

    let initial_type = existing
        .as_ref()
        .and_then(|i| i.get_string("type"))
        .unwrap_or_else(|| "bezier".into());
    let is_spring = initial_type.eq_ignore_ascii_case("spring");

    let name_entry = Entry::builder()
        .text(existing.as_ref().map(|i| i.name.as_str()).unwrap_or(""))
        .placeholder_text("easeOutQuint")
        .build();

    let type_list = StringList::new(&["bezier", "spring"]);
    let type_dd = DropDown::builder().model(&type_list).build();
    type_dd.set_selected(if is_spring { 1 } else { 0 });

    let bezier = BezierEditor::new(
        existing
            .as_ref()
            .map(|i| BezierPoints::from_fields(&i.fields))
            .unwrap_or_default(),
    );
    let spring = SpringEditor::new(
        existing
            .as_ref()
            .map(|i| SpringParams::from_fields(&i.fields))
            .unwrap_or_default(),
    );

    let stack = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .build();
    stack.append(bezier.widget());
    stack.append(spring.widget());
    bezier.widget().set_visible(!is_spring);
    spring.widget().set_visible(is_spring);

    type_dd.connect_selected_notify({
        let bezier_w = bezier.widget().clone();
        let spring_w = spring.widget().clone();
        move |dd| {
            let spring_mode = dd.selected() == 1;
            bezier_w.set_visible(!spring_mode);
            spring_w.set_visible(spring_mode);
        }
    });

    let status = Label::builder()
        .label(if is_new {
            "Appends hl.curve to a managed section."
        } else {
            "Realtime autosaves ~0.8s after changes."
        })
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
    {
        let row = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .build();
        row.append(
            &Label::builder()
                .label("Type")
                .halign(gtk4::Align::Start)
                .build(),
        );
        row.append(&type_dd);
        form.append(&row);
    }
    form.append(&stack);
    dialog::mount_sticky_dialog(&editor, &form, Some(&status), &actions);

    cancel.connect_clicked({
        let editor = editor.clone();
        move |_| editor.close()
    });

    let bezier = Rc::new(bezier);
    let spring = Rc::new(spring);
    let on_saved = Rc::new(on_saved);
    let try_save: Rc<dyn Fn(bool)> = {
        let editor = editor.clone();
        let existing = existing.clone();
        let config_path = config_path.clone();
        let name_entry = name_entry.clone();
        let type_dd = type_dd.clone();
        let status = status.clone();
        let on_saved = Rc::clone(&on_saved);
        let bezier = Rc::clone(&bezier);
        let spring = Rc::clone(&spring);
        Rc::new(move |close_on_success: bool| {
            let name = name_entry.text().to_string().trim().to_string();
            if name.is_empty() {
                if close_on_success {
                    status.set_text("Name is required.");
                }
                return;
            }
            let mut fields = BTreeMap::new();
            if type_dd.selected() == 1 {
                let p = spring.params();
                fields.insert("type".into(), json!("spring"));
                fields.insert("mass".into(), json!(p.mass));
                fields.insert("stiffness".into(), json!(p.stiffness));
                fields.insert("dampening".into(), json!(p.dampening));
            } else {
                let p = bezier.points();
                fields.insert("type".into(), json!("bezier"));
                fields.insert("points".into(), p.to_json());
            }
            let result = if let Some(item) = existing.as_ref() {
                writer::save_curve(item, &name, &fields)
                    .map(|r| format!("Saved curve in {}", r.path))
            } else if let Some(path) = config_path.as_ref() {
                writer::add_curve(path, &name, &fields)
                    .map(|r| format!("Added curve in {}", r.path))
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
        wire_dialog_autosave(&realtime, &try_save, &[&name_entry], &[]);
        type_dd.connect_selected_notify({
            let try_save = Rc::clone(&try_save);
            let realtime = Rc::clone(&realtime);
            let debouncer = Debouncer::new();
            move |_| {
                if !realtime.get() {
                    return;
                }
                let realtime = Rc::clone(&realtime);
                let try_save = Rc::clone(&try_save);
                debouncer.schedule(move || {
                    if realtime.get() {
                        try_save(false);
                    }
                });
            }
        });
        let schedule = {
            let try_save = Rc::clone(&try_save);
            let realtime = Rc::clone(&realtime);
            let debouncer = Debouncer::new();
            Rc::new(move || {
                if !realtime.get() {
                    return;
                }
                let realtime = Rc::clone(&realtime);
                let try_save = Rc::clone(&try_save);
                debouncer.schedule(move || {
                    if realtime.get() {
                        try_save(false);
                    }
                });
            })
        };
        bezier.connect_changed({
            let schedule = Rc::clone(&schedule);
            move || schedule()
        });
        spring.connect_changed({
            let schedule = Rc::clone(&schedule);
            move || schedule()
        });
    }

    editor.present();
}

fn show_animation_dialog<F>(
    parent: &impl IsA<Window>,
    existing: Option<SpecItem>,
    config_path: Option<PathBuf>,
    curve_names: Vec<String>,
    realtime: Rc<Cell<bool>>,
    on_saved: F,
) where
    F: Fn(String) + 'static,
{
    let is_new = existing.is_none();
    let editor = Window::builder()
        .title(if is_new {
            "Add animation"
        } else {
            "Edit animation"
        })
        .transient_for(parent)
        .modal(true)
        .default_width(520)
        .default_height(640)
        .build();

    let leaf_entry = Entry::builder()
        .text(
            existing
                .as_ref()
                .and_then(|i| i.get_string("leaf"))
                .unwrap_or_default(),
        )
        .placeholder_text("windowsIn")
        .build();
    let leaf_hints: Vec<&str> = curve_editor::common_animation_leaves().to_vec();
    let leaf_list = StringList::new(
        &std::iter::once("leaf…")
            .chain(leaf_hints.iter().copied())
            .collect::<Vec<_>>(),
    );
    let leaf_dd = DropDown::builder().model(&leaf_list).build();
    leaf_dd.connect_selected_notify({
        let leaf_entry = leaf_entry.clone();
        move |dd| {
            let i = dd.selected() as usize;
            if i == 0 {
                return;
            }
            if let Some(s) = leaf_hints.get(i - 1) {
                leaf_entry.set_text(s);
            }
        }
    });

    let speed = Entry::builder()
        .text(
            existing
                .as_ref()
                .and_then(|i| i.get_string("speed"))
                .unwrap_or_else(|| "5".into()),
        )
        .placeholder_text("5")
        .build();
    let enabled = CheckButton::builder()
        .label("Enabled")
        .active(existing.as_ref().and_then(|i| i.get_bool("enabled")).unwrap_or(true))
        .build();

    let has_spring = existing
        .as_ref()
        .and_then(|i| i.get_string("spring"))
        .filter(|s| !s.is_empty())
        .is_some();
    let curve_kind = DropDown::from_strings(&["bezier", "spring"]);
    curve_kind.set_selected(if has_spring { 1 } else { 0 });

    let mut curve_options = vec!["(none)".to_string()];
    curve_options.extend(curve_names);
    let curve_refs: Vec<&str> = curve_options.iter().map(|s| s.as_str()).collect();
    let curve_dd = DropDown::from_strings(&curve_refs);
    let current_curve = existing
        .as_ref()
        .and_then(|i| i.get_string("spring").or_else(|| i.get_string("bezier")))
        .unwrap_or_default();
    if let Some(idx) = curve_options.iter().position(|c| c == &current_curve) {
        curve_dd.set_selected(idx as u32);
    }

    let style_entry = Entry::builder()
        .text(
            existing
                .as_ref()
                .and_then(|i| i.get_string("style"))
                .unwrap_or_default(),
        )
        .placeholder_text("popin 87%")
        .build();
    let style_box = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .build();
    style_box.append(&style_entry);

    let refresh_styles = {
        let style_box = style_box.clone();
        let style_entry = style_entry.clone();
        let leaf_entry = leaf_entry.clone();
        Rc::new(move || {
            while let Some(child) = style_box.first_child() {
                style_box.remove(&child);
            }
            style_box.append(&style_entry);
            for s in curve_editor::style_suggestions_for_leaf(&leaf_entry.text()) {
                if s.is_empty() {
                    continue;
                }
                let btn = Button::builder().label(*s).css_classes(["flat"]).build();
                btn.connect_clicked({
                    let style_entry = style_entry.clone();
                    let s = (*s).to_string();
                    move |_| style_entry.set_text(&s)
                });
                style_box.append(&btn);
            }
        })
    };
    refresh_styles();
    leaf_entry.connect_changed({
        let refresh_styles = Rc::clone(&refresh_styles);
        move |_| refresh_styles()
    });

    let status = Label::builder()
        .label("Curve names come from the Curves tab. Styles depend on the leaf.")
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
    form.append(&field_block("Leaf", &leaf_entry));
    form.append(&leaf_dd);
    form.append(&enabled);
    form.append(&field_block("Speed (bezier only; springs ignore speed)", &speed));
    {
        let row = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .build();
        row.append(
            &Label::builder()
                .label("Curve type")
                .halign(gtk4::Align::Start)
                .build(),
        );
        row.append(&curve_kind);
        form.append(&row);
    }
    {
        let row = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .build();
        row.append(
            &Label::builder()
                .label("Curve")
                .halign(gtk4::Align::Start)
                .build(),
        );
        row.append(&curve_dd);
        form.append(&row);
    }
    {
        let row = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .build();
        row.append(
            &Label::builder()
                .label("Style")
                .halign(gtk4::Align::Start)
                .build(),
        );
        row.append(&style_box);
        form.append(&row);
    }
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
        let leaf_entry = leaf_entry.clone();
        let speed = speed.clone();
        let enabled = enabled.clone();
        let curve_kind = curve_kind.clone();
        let curve_dd = curve_dd.clone();
        let style_entry = style_entry.clone();
        let status = status.clone();
        let on_saved = Rc::clone(&on_saved);
        let curve_options = curve_options.clone();
        Rc::new(move |close_on_success: bool| {
            let leaf = leaf_entry.text().to_string().trim().to_string();
            if leaf.is_empty() {
                if close_on_success {
                    status.set_text("Leaf is required.");
                }
                return;
            }
            let mut fields = BTreeMap::new();
            fields.insert("leaf".into(), json!(leaf));
            fields.insert("enabled".into(), json!(enabled.is_active()));
            push_opt_number_or_string(&mut fields, "speed", &speed.text());
            let idx = curve_dd.selected() as usize;
            if idx > 0 {
                if let Some(name) = curve_options.get(idx) {
                    if curve_kind.selected() == 1 {
                        fields.insert("spring".into(), json!(name));
                    } else {
                        fields.insert("bezier".into(), json!(name));
                    }
                }
            }
            let style = style_entry.text().to_string().trim().to_string();
            if !style.is_empty() {
                fields.insert("style".into(), json!(style));
            }
            let result = if let Some(item) = existing.as_ref() {
                writer::save_animation(item, &fields)
                    .map(|r| format!("Saved animation in {}", r.path))
            } else if let Some(path) = config_path.as_ref() {
                writer::add_animation(path, &fields)
                    .map(|r| format!("Added animation in {}", r.path))
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
        wire_dialog_autosave(
            &realtime,
            &try_save,
            &[&leaf_entry, &speed, &style_entry],
            &[&enabled],
        );
    }

    editor.present();
}

fn show_spec_dialog<F>(
    parent: &impl IsA<Window>,
    kind: SpecKind,
    existing: Option<SpecItem>,
    config_path: Option<PathBuf>,
    realtime: Rc<Cell<bool>>,
    on_saved: F,
) where
    F: Fn(String) + 'static,
{
    let is_new = existing.is_none();
    let title = match kind {
        SpecKind::Monitor => "monitor",
        SpecKind::Device => "device",
        SpecKind::Gesture => "gesture",
        SpecKind::Animation => "animation",
        SpecKind::Curve => "curve",
    };
    let editor = Window::builder()
        .title(format!(
            "{} {title}",
            if existing.is_some() { "Edit" } else { "Add" }
        ))
        .transient_for(parent)
        .modal(true)
        .default_width(520)
        .default_height(560)
        .build();

    let form = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .build();

    let entries: Rc<RefCell<BTreeMap<String, Entry>>> = Rc::new(RefCell::new(BTreeMap::new()));
    let checks: Rc<RefCell<BTreeMap<String, CheckButton>>> = Rc::new(RefCell::new(BTreeMap::new()));
    let name_entry = Entry::builder()
        .text(existing.as_ref().map(|i| i.name.as_str()).unwrap_or(""))
        .placeholder_text("name")
        .build();

    let fields: &[(&str, &str, bool)] = match kind {
        SpecKind::Monitor => &[
            ("output", "Output (DP-1 or empty fallback)", false),
            ("mode", "Mode (preferred / 1920x1080@144)", false),
            ("position", "Position (auto / 0x0)", false),
            ("scale", "Scale (1 / auto)", false),
            ("transform", "Transform 0-7", false),
            ("mirror", "Mirror output", false),
            ("disabled", "Disabled", true),
        ],
        SpecKind::Device => &[
            ("name", "Device name", false),
            ("sensitivity", "Sensitivity", false),
            ("kb_layout", "Keyboard layout", false),
            ("kb_options", "Keyboard options", false),
            ("natural_scroll", "Natural scroll", true),
            ("tap_to_click", "Tap to click", true),
            ("disable_while_typing", "Disable while typing", true),
            ("enabled", "Enabled", true),
        ],
        SpecKind::Gesture => &[
            ("fingers", "Fingers", false),
            ("direction", "Direction (horizontal/vertical/…)", false),
            ("action", "Action (workspace/move/…)", false),
            ("mods", "Mods (SUPER)", false),
            ("scale", "Scale", false),
        ],
        SpecKind::Animation => &[
            ("leaf", "Leaf (windows, fade, …)", false),
            ("speed", "Speed", false),
            ("bezier", "Bezier curve name", false),
            ("spring", "Spring curve name", false),
            ("style", "Style (popin 87%)", false),
            ("enabled", "Enabled", true),
        ],
        SpecKind::Curve => &[
            ("type", "Type (bezier / spring)", false),
            ("mass", "Spring mass", false),
            ("stiffness", "Spring stiffness", false),
            ("dampening", "Spring dampening", false),
        ],
    };

    if matches!(kind, SpecKind::Curve) {
        form.append(&field_block("Curve name", &name_entry));
    }

    for (key, label, is_bool) in fields {
        if *is_bool {
            let active = existing
                .as_ref()
                .and_then(|i| i.get_bool(key))
                .unwrap_or(matches!(*key, "enabled"));
            let cb = CheckButton::builder().label(*label).active(active).build();
            form.append(&cb);
            checks.borrow_mut().insert((*key).into(), cb);
        } else {
            let text = existing
                .as_ref()
                .and_then(|i| i.get_string(key))
                .unwrap_or_default();
            let entry = Entry::builder().text(&text).placeholder_text(*label).build();
            form.append(&field_block(label, &entry));
            entries.borrow_mut().insert((*key).into(), entry);
        }
    }

    if matches!(kind, SpecKind::Monitor) {
        let pick = Button::builder().label("Pick output…").build();
        form.append(&pick);
        pick.connect_clicked({
            let editor = editor.clone();
            let entries = Rc::clone(&entries);
            move |_| {
                show_string_picker(&editor, "Outputs", list_monitor_names(), {
                    let entries = Rc::clone(&entries);
                    move |s| {
                        if let Some(e) = entries.borrow().get("output") {
                            e.set_text(&s);
                        }
                    }
                });
            }
        });
    }
    if matches!(kind, SpecKind::Device) {
        let pick = Button::builder().label("Pick device…").build();
        form.append(&pick);
        pick.connect_clicked({
            let editor = editor.clone();
            let entries = Rc::clone(&entries);
            move |_| {
                let names: Vec<String> = clients::list_devices()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|d| format!("{} ({})", d.name, d.kind))
                    .collect();
                show_string_picker(&editor, "Devices", names, {
                    let entries = Rc::clone(&entries);
                    move |s| {
                        let name = s.split(" (").next().unwrap_or(&s);
                        if let Some(e) = entries.borrow().get("name") {
                            e.set_text(name);
                        }
                    }
                });
            }
        });
    }

    let status = Label::builder()
        .label(if is_new {
            "Appends to a managed section. Enable Realtime after first Save for autosave."
        } else {
            "Edits in place. Realtime autosaves ~0.8s after you stop typing."
        })
        .wrap(true)
        .css_classes(["dim-label"])
        .build();
    let cancel = Button::builder().label("Cancel").build();
    let save = Button::builder()
        .label("Save")
        .css_classes(["suggested-action"])
        .build();
    let actions = dialog::action_buttons(&cancel, &save);
    dialog::mount_sticky_dialog(&editor, &form, Some(&status), &actions);

    let on_saved = Rc::new(on_saved);
    let try_save: Rc<dyn Fn(bool)> = {
        let editor = editor.clone();
        let existing = existing.clone();
        let config_path = config_path.clone();
        let entries = Rc::clone(&entries);
        let checks = Rc::clone(&checks);
        let name_entry = name_entry.clone();
        let status = status.clone();
        let on_saved = Rc::clone(&on_saved);
        Rc::new(move |close_on_success: bool| {
            let mut fields = BTreeMap::new();
            for (k, e) in entries.borrow().iter() {
                push_opt_number_or_string(&mut fields, k, &e.text());
            }
            for (k, cb) in checks.borrow().iter() {
                if cb.is_active() || k == "enabled" || k == "disabled" {
                    fields.insert(k.clone(), json!(cb.is_active()));
                }
            }
            if matches!(kind, SpecKind::Curve) {
                if fields.get("type").and_then(|v| v.as_str()) == Some("bezier")
                    && !fields.contains_key("points")
                {
                    fields.insert(
                        "points".into(),
                        json!([[0.23, 1.0], [0.32, 1.0]]),
                    );
                }
            }
            if fields.is_empty() && !matches!(kind, SpecKind::Curve) {
                if close_on_success {
                    status.set_text("Fill at least one field.");
                }
                return;
            }
            let result = match kind {
                SpecKind::Monitor => {
                    if let Some(item) = existing.as_ref() {
                        writer::save_monitor(item, &fields).map(|r| format!("Saved monitor in {}", r.path))
                    } else if let Some(path) = config_path.as_ref() {
                        writer::add_monitor(path, &fields).map(|r| format!("Added monitor in {}", r.path))
                    } else {
                        Err(writer::WriteError::Invalid("missing path".into()))
                    }
                }
                SpecKind::Device => {
                    if let Some(item) = existing.as_ref() {
                        writer::save_device(item, &fields).map(|r| format!("Saved device in {}", r.path))
                    } else if let Some(path) = config_path.as_ref() {
                        writer::add_device(path, &fields).map(|r| format!("Added device in {}", r.path))
                    } else {
                        Err(writer::WriteError::Invalid("missing path".into()))
                    }
                }
                SpecKind::Gesture => {
                    if let Some(item) = existing.as_ref() {
                        writer::save_gesture(item, &fields).map(|r| format!("Saved gesture in {}", r.path))
                    } else if let Some(path) = config_path.as_ref() {
                        writer::add_gesture(path, &fields).map(|r| format!("Added gesture in {}", r.path))
                    } else {
                        Err(writer::WriteError::Invalid("missing path".into()))
                    }
                }
                SpecKind::Animation => {
                    if let Some(item) = existing.as_ref() {
                        writer::save_animation(item, &fields)
                            .map(|r| format!("Saved animation in {}", r.path))
                    } else if let Some(path) = config_path.as_ref() {
                        writer::add_animation(path, &fields)
                            .map(|r| format!("Added animation in {}", r.path))
                    } else {
                        Err(writer::WriteError::Invalid("missing path".into()))
                    }
                }
                SpecKind::Curve => {
                    let name = name_entry.text().to_string();
                    if let Some(item) = existing.as_ref() {
                        writer::save_curve(item, &name, &fields)
                            .map(|r| format!("Saved curve in {}", r.path))
                    } else if let Some(path) = config_path.as_ref() {
                        writer::add_curve(path, &name, &fields)
                            .map(|r| format!("Added curve in {}", r.path))
                    } else {
                        Err(writer::WriteError::Invalid("missing path".into()))
                    }
                }
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

    cancel.connect_clicked({
        let editor = editor.clone();
        move |_| editor.close()
    });
    save.connect_clicked({
        let try_save = Rc::clone(&try_save);
        move |_| try_save(true)
    });

    if !is_new {
        let entry_owned: Vec<Entry> = entries.borrow().values().cloned().collect();
        let check_owned: Vec<CheckButton> = checks.borrow().values().cloned().collect();
        let mut all_entries: Vec<&Entry> = vec![&name_entry];
        all_entries.extend(entry_owned.iter());
        let check_refs: Vec<&CheckButton> = check_owned.iter().collect();
        wire_dialog_autosave(&realtime, &try_save, &all_entries, &check_refs);
    }

    editor.present();
}

fn build_config_page(
    parent: &impl IsA<Window>,
    state: Rc<RefCell<Option<BindCollection>>>,
    reload: ReloadFn,
    status_fn: StatusFn,
    realtime: Rc<Cell<bool>>,
    filling: Rc<Cell<bool>>,
) -> (GtkBox, Rc<RefCell<ConfigFormEntries>>) {
    let gaps_in = Entry::new();
    let gaps_out = Entry::new();
    let border_size = Entry::new();
    let layout = Entry::new();
    let active_border = Entry::new();
    let inactive_border = Entry::new();
    let resize_on_border = CheckButton::builder().label("Resize on border").build();
    let rounding = Entry::new();
    let active_opacity = Entry::new();
    let inactive_opacity = Entry::new();
    let shadow_enabled = CheckButton::builder().label("Shadows").build();
    let blur_enabled = CheckButton::builder().label("Blur").build();
    let blur_size = Entry::new();
    let kb_layout = Entry::new();
    let follow_mouse = Entry::new();
    let sensitivity = Entry::new();
    let natural_scroll = CheckButton::builder().label("Natural scroll").build();
    let anims_enabled = CheckButton::builder().label("Animations enabled").build();
    let force_wallpaper = Entry::new();
    let disable_logo = CheckButton::builder().label("Disable Hyprland logo").build();
    let vrr = Entry::new();
    let dwindle_preserve = CheckButton::builder().label("dwindle preserve_split").build();
    let master_new_status = Entry::new();

    let entries = Rc::new(RefCell::new(ConfigFormEntries {
        gaps_in: gaps_in.clone(),
        gaps_out: gaps_out.clone(),
        border_size: border_size.clone(),
        layout: layout.clone(),
        active_border: active_border.clone(),
        inactive_border: inactive_border.clone(),
        resize_on_border: resize_on_border.clone(),
        rounding: rounding.clone(),
        active_opacity: active_opacity.clone(),
        inactive_opacity: inactive_opacity.clone(),
        shadow_enabled: shadow_enabled.clone(),
        blur_enabled: blur_enabled.clone(),
        blur_size: blur_size.clone(),
        kb_layout: kb_layout.clone(),
        follow_mouse: follow_mouse.clone(),
        sensitivity: sensitivity.clone(),
        natural_scroll: natural_scroll.clone(),
        anims_enabled: anims_enabled.clone(),
        force_wallpaper: force_wallpaper.clone(),
        disable_logo: disable_logo.clone(),
        vrr: vrr.clone(),
        dwindle_preserve: dwindle_preserve.clone(),
        master_new_status: master_new_status.clone(),
    }));

    let form = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .css_classes(["hyprbinds-page"])
        .build();
    form.append(
        &Label::builder()
            .label("Config")
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-page-title"])
            .build(),
    );
    form.append(
        &Label::builder()
            .label("Merged hl.config values. Save writes one managed override block.")
            .halign(gtk4::Align::Start)
            .wrap(true)
            .xalign(0.0)
            .css_classes(["hyprbinds-page-hint"])
            .build(),
    );
    form.append(&dialog::section_label("general"));
    form.append(&field_block("gaps_in", &gaps_in));
    form.append(&field_block("gaps_out", &gaps_out));
    form.append(&field_block("border_size", &border_size));
    form.append(&field_block("layout", &layout));
    form.append(&field_block("col.active_border", &active_border));
    form.append(&field_block("col.inactive_border", &inactive_border));
    form.append(&resize_on_border);
    form.append(&dialog::section_label("decoration"));
    form.append(&field_block("rounding", &rounding));
    form.append(&field_block("active_opacity", &active_opacity));
    form.append(&field_block("inactive_opacity", &inactive_opacity));
    form.append(&shadow_enabled);
    form.append(&blur_enabled);
    form.append(&field_block("blur.size", &blur_size));
    form.append(&dialog::section_label("input"));
    form.append(&field_block("kb_layout", &kb_layout));
    form.append(&field_block("follow_mouse", &follow_mouse));
    form.append(&field_block("sensitivity", &sensitivity));
    form.append(&natural_scroll);
    form.append(&dialog::section_label("animations / misc / layouts"));
    form.append(&anims_enabled);
    form.append(&field_block("misc.force_default_wallpaper", &force_wallpaper));
    form.append(&disable_logo);
    form.append(&field_block("misc.vrr", &vrr));
    form.append(&dwindle_preserve);
    form.append(&field_block("master.new_status", &master_new_status));

    let save = Button::builder()
        .label("Save config override")
        .css_classes(["suggested-action"])
        .hexpand(true)
        .build();
    let hint = Label::builder()
        .label("Shows merged hl.config values. Save writes one managed override block. With Realtime on, edits autosave ~0.8s after you stop typing (without resetting the form).")
        .wrap(true)
        .css_classes(["dim-label", "caption"])
        .build();
    let footer = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .build();
    footer.append(&hint);
    footer.append(&save);

    let persist_config: Rc<dyn Fn(bool)> = {
        let state = Rc::clone(&state);
        let entries = Rc::clone(&entries);
        let reload = Rc::clone(&reload);
        let status_fn = Rc::clone(&status_fn);
        let _parent = parent.clone().upcast::<Window>();
        Rc::new(move |full_reload: bool| {
            let path = state.borrow().as_ref().map(|c| c.config_path.clone());
            let Some(path) = path else {
                status_fn("Load a config first.".into());
                return;
            };
            let e = entries.borrow();
            let mut merged = state
                .borrow()
                .as_ref()
                .map(|c| c.config_merged.clone())
                .unwrap_or(json!({}));
            if !merged.is_object() {
                merged = json!({});
            }
            apply_config_form_to_merged(&e, &mut merged);
            match writer::save_config_override(&path, &merged) {
                Ok(r) => {
                    if let Some(c) = state.borrow_mut().as_mut() {
                        c.config_merged = merged;
                    }
                    status_fn(format!("Saved config override in {}", r.path));
                    if full_reload {
                        reload();
                    }
                }
                Err(err) => status_fn(format!("Config save failed: {err}")),
            }
        })
    };

    save.connect_clicked({
        let persist_config = Rc::clone(&persist_config);
        move |_| persist_config(true)
    });

    let debouncer = Debouncer::new();
    let schedule_autosave = {
        let realtime = Rc::clone(&realtime);
        let filling = Rc::clone(&filling);
        let persist_config = Rc::clone(&persist_config);
        let debouncer = debouncer.clone();
        Rc::new(move || {
            if filling.get() || !realtime.get() {
                return;
            }
            let realtime = Rc::clone(&realtime);
            let persist_config = Rc::clone(&persist_config);
            debouncer.schedule(move || {
                if realtime.get() {
                    persist_config(false);
                }
            });
        })
    };

    let text_entries = [
        &gaps_in,
        &gaps_out,
        &border_size,
        &layout,
        &active_border,
        &inactive_border,
        &rounding,
        &active_opacity,
        &inactive_opacity,
        &blur_size,
        &kb_layout,
        &follow_mouse,
        &sensitivity,
        &force_wallpaper,
        &vrr,
        &master_new_status,
    ];
    for entry in text_entries {
        entry.connect_changed({
            let schedule_autosave = Rc::clone(&schedule_autosave);
            move |_| schedule_autosave()
        });
    }
    let checks = [
        &resize_on_border,
        &shadow_enabled,
        &blur_enabled,
        &natural_scroll,
        &anims_enabled,
        &disable_logo,
        &dwindle_preserve,
    ];
    for check in checks {
        check.connect_toggled({
            let schedule_autosave = Rc::clone(&schedule_autosave);
            move |_| schedule_autosave()
        });
    }

    form.set_margin_top(0);
    form.set_margin_start(0);
    form.set_margin_end(0);
    form.set_margin_bottom(8);
    let page = dialog::mount_sticky_page(&form, &footer);
    (page, entries)
}

fn apply_config_form_to_merged(e: &ConfigFormEntries, merged: &mut Value) {
    set_num(merged, &["general", "gaps_in"], &e.gaps_in.text());
    set_num(merged, &["general", "gaps_out"], &e.gaps_out.text());
    set_num(merged, &["general", "border_size"], &e.border_size.text());
    set_str(merged, &["general", "layout"], &e.layout.text());
    if !e.active_border.text().trim().is_empty() {
        settings_config::set_path(
            merged,
            &["general", "col", "active_border"],
            settings_config::parse_color_input(&e.active_border.text()),
        );
    }
    if !e.inactive_border.text().trim().is_empty() {
        settings_config::set_path(
            merged,
            &["general", "col", "inactive_border"],
            settings_config::parse_color_input(&e.inactive_border.text()),
        );
    }
    settings_config::set_path(
        merged,
        &["general", "resize_on_border"],
        json!(e.resize_on_border.is_active()),
    );
    set_num(merged, &["decoration", "rounding"], &e.rounding.text());
    set_num(merged, &["decoration", "active_opacity"], &e.active_opacity.text());
    set_num(
        merged,
        &["decoration", "inactive_opacity"],
        &e.inactive_opacity.text(),
    );
    settings_config::set_path(
        merged,
        &["decoration", "shadow", "enabled"],
        json!(e.shadow_enabled.is_active()),
    );
    settings_config::set_path(
        merged,
        &["decoration", "blur", "enabled"],
        json!(e.blur_enabled.is_active()),
    );
    set_num(merged, &["decoration", "blur", "size"], &e.blur_size.text());
    set_str(merged, &["input", "kb_layout"], &e.kb_layout.text());
    set_num(merged, &["input", "follow_mouse"], &e.follow_mouse.text());
    set_num(merged, &["input", "sensitivity"], &e.sensitivity.text());
    settings_config::set_path(
        merged,
        &["input", "touchpad", "natural_scroll"],
        json!(e.natural_scroll.is_active()),
    );
    settings_config::set_path(
        merged,
        &["animations", "enabled"],
        json!(e.anims_enabled.is_active()),
    );
    set_num(
        merged,
        &["misc", "force_default_wallpaper"],
        &e.force_wallpaper.text(),
    );
    settings_config::set_path(
        merged,
        &["misc", "disable_hyprland_logo"],
        json!(e.disable_logo.is_active()),
    );
    set_num(merged, &["misc", "vrr"], &e.vrr.text());
    settings_config::set_path(
        merged,
        &["dwindle", "preserve_split"],
        json!(e.dwindle_preserve.is_active()),
    );
    set_str(merged, &["master", "new_status"], &e.master_new_status.text());
}

fn set_num(root: &mut Value, path: &[&str], raw: &str) {
    let v = raw.trim();
    if v.is_empty() {
        return;
    }
    if let Ok(n) = v.parse::<i64>() {
        settings_config::set_path(root, path, json!(n));
    } else if let Ok(n) = v.parse::<f64>() {
        settings_config::set_path(root, path, json!(n));
    } else {
        settings_config::set_path(root, path, json!(v));
    }
}

fn set_str(root: &mut Value, path: &[&str], raw: &str) {
    let v = raw.trim();
    if !v.is_empty() {
        settings_config::set_path(root, path, json!(v));
    }
}

pub fn fill_config_form(entries: &ConfigFormEntries, merged: &Value, filling: &Cell<bool>) {
    filling.set(true);
    entries
        .gaps_in
        .set_text(&settings_config::get_i64(merged, &["general", "gaps_in"], 5).to_string());
    entries
        .gaps_out
        .set_text(&settings_config::get_i64(merged, &["general", "gaps_out"], 20).to_string());
    entries.border_size.set_text(
        &settings_config::get_i64(merged, &["general", "border_size"], 1).to_string(),
    );
    entries
        .layout
        .set_text(&settings_config::get_string(merged, &["general", "layout"], "dwindle"));
    let active = settings_config::get_path(merged, &["general", "col", "active_border"])
        .map(settings_config::color_to_display)
        .unwrap_or_default();
    entries.active_border.set_text(&active);
    let inactive = settings_config::get_path(merged, &["general", "col", "inactive_border"])
        .map(settings_config::color_to_display)
        .unwrap_or_default();
    entries.inactive_border.set_text(&inactive);
    entries.resize_on_border.set_active(settings_config::get_bool(
        merged,
        &["general", "resize_on_border"],
        false,
    ));
    entries
        .rounding
        .set_text(&settings_config::get_i64(merged, &["decoration", "rounding"], 10).to_string());
    entries.active_opacity.set_text(
        &settings_config::get_f64(merged, &["decoration", "active_opacity"], 1.0).to_string(),
    );
    entries.inactive_opacity.set_text(
        &settings_config::get_f64(merged, &["decoration", "inactive_opacity"], 1.0).to_string(),
    );
    entries.shadow_enabled.set_active(settings_config::get_bool(
        merged,
        &["decoration", "shadow", "enabled"],
        true,
    ));
    entries.blur_enabled.set_active(settings_config::get_bool(
        merged,
        &["decoration", "blur", "enabled"],
        true,
    ));
    entries.blur_size.set_text(
        &settings_config::get_i64(merged, &["decoration", "blur", "size"], 3).to_string(),
    );
    entries
        .kb_layout
        .set_text(&settings_config::get_string(merged, &["input", "kb_layout"], "us"));
    entries.follow_mouse.set_text(
        &settings_config::get_i64(merged, &["input", "follow_mouse"], 1).to_string(),
    );
    entries.sensitivity.set_text(
        &settings_config::get_f64(merged, &["input", "sensitivity"], 0.0).to_string(),
    );
    entries.natural_scroll.set_active(settings_config::get_bool(
        merged,
        &["input", "touchpad", "natural_scroll"],
        false,
    ));
    entries.anims_enabled.set_active(settings_config::get_bool(
        merged,
        &["animations", "enabled"],
        true,
    ));
    entries.force_wallpaper.set_text(
        &settings_config::get_i64(merged, &["misc", "force_default_wallpaper"], -1).to_string(),
    );
    entries.disable_logo.set_active(settings_config::get_bool(
        merged,
        &["misc", "disable_hyprland_logo"],
        false,
    ));
    entries
        .vrr
        .set_text(&settings_config::get_i64(merged, &["misc", "vrr"], 0).to_string());
    entries.dwindle_preserve.set_active(settings_config::get_bool(
        merged,
        &["dwindle", "preserve_split"],
        false,
    ));
    entries.master_new_status.set_text(&settings_config::get_string(
        merged,
        &["master", "new_status"],
        "master",
    ));
    filling.set(false);
}

pub fn render_settings_lists(group: &SettingsGroup, collection: &BindCollection) {
    clear_list(&group.monitor_list);
    for item in &collection.monitors {
        group.monitor_list.append(&spec_row(item, "mon-"));
    }
    clear_list(&group.device_list);
    for item in &collection.devices {
        group.device_list.append(&spec_row(item, "dev-"));
    }
    clear_list(&group.gesture_list);
    for item in &collection.gestures {
        group.gesture_list.append(&spec_row(item, "ges-"));
    }
    clear_list(&group.anim_list);
    for item in &collection.animations {
        group.anim_list.append(&spec_row(item, "anim-"));
    }
    clear_list(&group.curve_list);
    for item in &collection.curves {
        group.curve_list.append(&spec_row(item, "curve-"));
    }
    fill_config_form(
        &group.config_entries.borrow(),
        &collection.config_merged,
        &group.config_filling,
    );
}
