//! Ctrl+P command palette — jump to pages and run common actions.

use crate::dialog;
use gtk4::gdk::Key;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Entry, EventControllerKey, Label, ListBox, ListBoxRow, Orientation,
    ScrolledWindow, Window,
};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct PaletteAction {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub keywords: String,
}

#[derive(Clone)]
pub struct PaletteHost {
    pub actions: Rc<RefCell<Vec<PaletteAction>>>,
}

impl PaletteHost {
    pub fn set_actions(&self, actions: Vec<PaletteAction>) {
        *self.actions.borrow_mut() = actions;
    }
}

pub fn attach_palette(
    parent: &impl IsA<Window>,
    on_run: Rc<dyn Fn(String)>,
) -> PaletteHost {
    let actions: Rc<RefCell<Vec<PaletteAction>>> = Rc::new(RefCell::new(Vec::new()));

    let palette = Window::builder()
        .title("Command palette")
        .transient_for(parent)
        .modal(true)
        .default_width(520)
        .default_height(420)
        .decorated(false)
        .resizable(false)
        .css_classes(["hyprbinds-palette"])
        .build();

    let search = Entry::builder()
        .placeholder_text("Go to page or run action…")
        .hexpand(true)
        .css_classes(["hyprbinds-palette-search"])
        .build();

    let list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["boxed-list", "hyprbinds-palette-list"])
        .build();
    let scroll = ScrolledWindow::builder()
        .vexpand(true)
        .child(&list)
        .build();

    let hint = Label::builder()
        .label("↑↓ navigate · Enter run · Esc close · Ctrl+P")
        .halign(gtk4::Align::Start)
        .css_classes(["dim-label", "caption"])
        .build();

    let root = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_top(14)
        .margin_bottom(14)
        .margin_start(14)
        .margin_end(14)
        .css_classes(["hyprbinds-palette-root"])
        .build();
    root.append(&search);
    root.append(&scroll);
    root.append(&hint);
    palette.set_child(Some(&root));

    let filtered_ids = Rc::new(RefCell::new(Vec::<String>::new()));

    let render = {
        let list = list.clone();
        let actions = Rc::clone(&actions);
        let filtered_ids = Rc::clone(&filtered_ids);
        Rc::new(move |query: String| {
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }
            let q = query.to_lowercase();
            let mut ids = Vec::new();
            for action in actions.borrow().iter() {
                if q.is_empty()
                    || action.title.to_lowercase().contains(&q)
                    || action.subtitle.to_lowercase().contains(&q)
                    || action.keywords.to_lowercase().contains(&q)
                    || action.id.to_lowercase().contains(&q)
                {
                    ids.push(action.id.clone());
                    list.append(&action_row(action));
                }
            }
            if let Some(row) = list.row_at_index(0) {
                list.select_row(Some(&row));
            }
            *filtered_ids.borrow_mut() = ids;
        })
    };

    let run_selected = {
        let list = list.clone();
        let filtered_ids = Rc::clone(&filtered_ids);
        let on_run = Rc::clone(&on_run);
        let palette = palette.clone();
        Rc::new(move || {
            let Some(row) = list.selected_row() else {
                return;
            };
            let idx = row.index() as usize;
            let id = filtered_ids.borrow().get(idx).cloned();
            if let Some(id) = id {
                palette.set_visible(false);
                on_run(id);
            }
        })
    };

    search.connect_changed({
        let render = Rc::clone(&render);
        move |e| render(e.text().to_string())
    });

    list.connect_row_activated({
        let run_selected = Rc::clone(&run_selected);
        move |_, _| run_selected()
    });

    let key = EventControllerKey::new();
    key.connect_key_pressed({
        let palette = palette.clone();
        let list = list.clone();
        let run_selected = Rc::clone(&run_selected);
        move |_, key, _, _| {
            if key == Key::Escape {
                palette.set_visible(false);
                return glib::Propagation::Stop;
            }
            if key == Key::Return || key == Key::KP_Enter {
                run_selected();
                return glib::Propagation::Stop;
            }
            if key == Key::Up {
                if let Some(row) = list.selected_row() {
                    let idx = row.index().saturating_sub(1);
                    if let Some(prev) = list.row_at_index(idx) {
                        list.select_row(Some(&prev));
                    }
                }
                return glib::Propagation::Stop;
            }
            if key == Key::Down {
                if let Some(row) = list.selected_row() {
                    let idx = row.index() + 1;
                    if let Some(next) = list.row_at_index(idx) {
                        list.select_row(Some(&next));
                    }
                } else if let Some(first) = list.row_at_index(0) {
                    list.select_row(Some(&first));
                }
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        }
    });
    palette.add_controller(key);

    let open = {
        let palette = palette.clone();
        let search = search.clone();
        let render = Rc::clone(&render);
        Rc::new(move || {
            search.set_text("");
            render(String::new());
            palette.present();
            search.grab_focus();
        })
    };

    let parent_key = EventControllerKey::new();
    parent_key.set_propagation_phase(gtk4::PropagationPhase::Capture);
    parent_key.connect_key_pressed({
        let open = Rc::clone(&open);
        move |_, key, _, modifiers| {
            let ctrl = modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK);
            if ctrl && (key == Key::p || key == Key::P) {
                open();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        }
    });
    parent.clone().upcast::<Window>().add_controller(parent_key);

    PaletteHost { actions }
}

fn action_row(action: &PaletteAction) -> ListBoxRow {
    let col = dialog::list_row_column();
    col.append(&dialog::row_title(&action.title));
    if !action.subtitle.is_empty() {
        col.append(&dialog::row_meta(&action.subtitle));
    }
    ListBoxRow::builder()
        .child(&col)
        .activatable(true)
        .name(action.id.clone())
        .build()
}

pub fn default_nav_actions(developer_mode: bool) -> Vec<PaletteAction> {
    crate::nav::NAV
        .iter()
        .flat_map(|section| {
            section.items.iter().filter_map(|item| {
                if item.dev_only && !developer_mode {
                    return None;
                }
                let subtitle = if section.title.is_empty() {
                    item.subtitle.to_string()
                } else {
                    format!("{} › {}", section.title, item.subtitle)
                };
                Some(PaletteAction {
                    id: format!("goto:{}", item.id),
                    title: item.label.into(),
                    subtitle,
                    keywords: format!(
                        "go page {} {} {} {}",
                        item.label, item.subtitle, item.keywords, section.title
                    ),
                })
            })
        })
        .collect()
}

pub fn action_reload() -> PaletteAction {
    PaletteAction {
        id: "action:reload".into(),
        title: "Reload config".into(),
        subtitle: "Re-read hyprland.lua".into(),
        keywords: "reload refresh".into(),
    }
}

pub fn action_restore() -> PaletteAction {
    PaletteAction {
        id: "action:restore".into(),
        title: "Restore last good config".into(),
        subtitle: "Roll back to previous snapshot".into(),
        keywords: "restore backup undo last good".into(),
    }
}

pub fn action_conflicts() -> PaletteAction {
    PaletteAction {
        id: "action:conflicts".into(),
        title: "Show bind conflicts".into(),
        subtitle: "Filter binds that share the same chord".into(),
        keywords: "conflict duplicate keys".into(),
    }
}

pub fn action_sysinfo() -> PaletteAction {
    PaletteAction {
        id: "action:sysinfo".into(),
        title: "Copy system info".into(),
        subtitle: "Hardware + versions for debugging".into(),
        keywords: "system info clipboard hardware".into(),
    }
}

pub fn action_health() -> PaletteAction {
    PaletteAction {
        id: "action:health".into(),
        title: "Go to Health".into(),
        subtitle: "System diagnostics".into(),
        keywords: "health diagnose".into(),
    }
}
