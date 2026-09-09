//! Command/search palettes.
//!
//! Ctrl+P is for commands and page navigation.
//! Ctrl+K is a focused fuzzy finder for individual settings.

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

fn fuzzy_score(query: &str, action: &PaletteAction) -> Option<i64> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Some(0);
    }

    let title = action.title.to_lowercase();
    let subtitle = action.subtitle.to_lowercase();
    let keywords = action.keywords.to_lowercase();
    let hay = format!("{title} {subtitle} {keywords}");

    if title == query {
        return Some(10_000);
    }
    if title.starts_with(&query) {
        return Some(8_000 - title.len() as i64);
    }
    if title.contains(&query) {
        return Some(6_000 - title.len() as i64);
    }
    if hay.contains(&query) {
        return Some(4_000 - hay.len().min(500) as i64);
    }

    // Loose subsequence matching: "tpad speed" can still find
    // "Touchpad pointer speed". Consecutive characters receive a bonus.
    let mut score = 0i64;
    let mut last_match: Option<usize> = None;
    let chars: Vec<char> = hay.chars().collect();
    let mut cursor = 0usize;
    for q in query.chars().filter(|c| !c.is_whitespace()) {
        let mut found = None;
        for i in cursor..chars.len() {
            if chars[i] == q {
                found = Some(i);
                break;
            }
        }
        let i = found?;
        score += 18;
        if last_match.is_some_and(|last| i == last + 1) {
            score += 14;
        }
        if i == 0 || chars.get(i.wrapping_sub(1)).is_some_and(|c| !c.is_alphanumeric()) {
            score += 10;
        }
        last_match = Some(i);
        cursor = i + 1;
    }
    Some(score - (cursor as i64 / 3))
}

fn attach_search_window(
    parent: &impl IsA<Window>,
    title: &str,
    placeholder: &str,
    hint_text: &str,
    trigger: Key,
    initial_actions: Vec<PaletteAction>,
    on_run: Rc<dyn Fn(String)>,
) -> PaletteHost {
    let actions: Rc<RefCell<Vec<PaletteAction>>> = Rc::new(RefCell::new(initial_actions));

    let palette = Window::builder()
        .title(title)
        .transient_for(parent)
        .modal(true)
        .default_width(560)
        .default_height(440)
        .decorated(false)
        .resizable(false)
        .css_classes(["hyprbinds-palette"])
        .build();

    let search = Entry::builder()
        .placeholder_text(placeholder)
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

    let empty = Label::builder()
        .label("No matches")
        .halign(gtk4::Align::Start)
        .visible(false)
        .css_classes(["dim-label", "caption"])
        .build();

    let hint = Label::builder()
        .label(hint_text)
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
    root.append(&empty);
    root.append(&hint);
    palette.set_child(Some(&root));

    let filtered_ids = Rc::new(RefCell::new(Vec::<String>::new()));

    let render = {
        let list = list.clone();
        let empty = empty.clone();
        let actions = Rc::clone(&actions);
        let filtered_ids = Rc::clone(&filtered_ids);
        Rc::new(move |query: String| {
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }

            let mut ranked: Vec<(i64, PaletteAction)> = actions
                .borrow()
                .iter()
                .filter_map(|action| fuzzy_score(&query, action).map(|score| (score, action.clone())))
                .collect();
            ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.title.cmp(&b.1.title)));

            let mut ids = Vec::new();
            for (_, action) in ranked.into_iter().take(80) {
                ids.push(action.id.clone());
                list.append(&action_row(&action));
            }
            empty.set_visible(ids.is_empty());
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
            if let Some(id) = filtered_ids.borrow().get(idx).cloned() {
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
            if ctrl && (key == trigger || key == trigger.to_upper()) {
                open();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        }
    });
    parent.clone().upcast::<Window>().add_controller(parent_key);

    PaletteHost { actions }
}

pub fn attach_palette(
    parent: &impl IsA<Window>,
    on_run: Rc<dyn Fn(String)>,
) -> PaletteHost {
    attach_search_window(
        parent,
        "Command palette",
        "Go to page or run action…",
        "↑↓ navigate · Enter run · Esc close · Ctrl+P",
        Key::p,
        Vec::new(),
        on_run,
    )
}

pub fn attach_setting_search(
    parent: &impl IsA<Window>,
    on_run: Rc<dyn Fn(String)>,
) -> PaletteHost {
    attach_search_window(
        parent,
        "Find a setting",
        "Search any setting…",
        "Fuzzy search · ↑↓ navigate · Enter open · Esc close · Ctrl+K",
        Key::k,
        setting_search_actions(),
        on_run,
    )
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

fn setting(id: &str, title: &str, page: &str, keywords: &str) -> PaletteAction {
    PaletteAction {
        id: format!("setting:{id}:{page}"),
        title: title.into(),
        subtitle: format!("Open {}", crate::nav::page_chrome_title(page)),
        keywords: keywords.into(),
    }
}

/// Search index for user-facing settings. These names intentionally include common
/// synonyms so Ctrl+K works with the words people actually think in.
pub fn setting_search_actions() -> Vec<PaletteAction> {
    vec![
        setting("gaps-in", "Inner gaps", "lookfeel", "gaps in inner spacing windows padding"),
        setting("gaps-out", "Outer gaps", "lookfeel", "gaps out outer screen edge spacing margin"),
        setting("border-size", "Border size", "lookfeel", "border width thickness window outline"),
        setting("rounding", "Window rounding", "lookfeel", "rounding radius rounded corners window"),
        setting("active-opacity", "Active window opacity", "lookfeel", "active opacity transparency focused window"),
        setting("inactive-opacity", "Inactive window opacity", "lookfeel", "inactive opacity transparency unfocused window"),
        setting("blur", "Background blur", "lookfeel", "blur enable background transparency decoration"),
        setting("blur-size", "Blur size", "lookfeel", "blur strength radius size decoration"),
        setting("shadow", "Window shadows", "lookfeel", "shadow enable window decoration"),
        setting("animations", "Animations", "settings-animations", "animations enable motion transitions"),
        setting("animation-speed", "Global animation speed", "overview", "animation speed duration motion global"),
        setting("active-border", "Active border color", "lookfeel", "active border color focused colour"),
        setting("inactive-border", "Inactive border color", "lookfeel", "inactive border color unfocused colour"),
        setting("layout", "Default layout", "settings-config", "layout dwindle master general tiling"),
        setting("resize-on-border", "Resize on border", "settings-config", "resize border mouse general"),
        setting("allow-tearing", "Allow tearing", "settings-config", "tearing gaming vsync render misc"),
        setting("focus-follows-mouse", "Focus follows mouse", "settings-config", "focus mouse cursor input follow"),
        setting("mouse-sensitivity", "Mouse sensitivity", "settings-devices", "mouse pointer sensitivity speed input device"),
        setting("touchpad-sensitivity", "Touchpad sensitivity", "settings-devices", "touchpad trackpad sensitivity pointer speed input"),
        setting("natural-scroll", "Natural scrolling", "settings-devices", "natural scroll inverted touchpad mouse"),
        setting("tap-to-click", "Tap to click", "settings-devices", "tap click touchpad trackpad"),
        setting("scroll-factor", "Scroll factor", "settings-devices", "scroll speed factor mouse touchpad"),
        setting("keyboard-layout", "Keyboard layout", "settings-devices", "keyboard layout kb_layout language input"),
        setting("keyboard-variant", "Keyboard variant", "settings-devices", "keyboard variant kb_variant input"),
        setting("keyboard-options", "Keyboard options", "settings-devices", "keyboard options xkb caps compose input"),
        setting("repeat-rate", "Keyboard repeat rate", "settings-devices", "keyboard key repeat rate delay input"),
        setting("monitor-resolution", "Monitor resolution", "settings-monitors", "monitor display resolution mode width height"),
        setting("monitor-refresh", "Monitor refresh rate", "settings-monitors", "monitor display refresh hz frequency"),
        setting("monitor-scale", "Monitor scaling", "settings-monitors", "monitor display scale scaling dpi"),
        setting("monitor-position", "Monitor position", "settings-monitors", "monitor display position x y layout arrangement"),
        setting("monitor-transform", "Monitor rotation", "settings-monitors", "monitor display transform rotate rotation orientation"),
        setting("workspace-monitor", "Workspace monitor", "rules-workspace", "workspace monitor output binding display"),
        setting("workspace-layout", "Workspace layout", "rules-workspace", "workspace layout dwindle master"),
        setting("workspace-gaps", "Workspace gaps", "rules-workspace", "workspace gaps spacing inner outer"),
        setting("workspace-persistent", "Persistent workspace", "rules-workspace", "workspace persistent always keep"),
        setting("window-match", "Window rule matching", "rules-window", "window rule class title app match regex"),
        setting("window-float", "Floating windows", "rules-window", "window rule float floating tiled"),
        setting("window-size", "Window size rule", "rules-window", "window rule size width height"),
        setting("window-position", "Window position rule", "rules-window", "window rule move position x y"),
        setting("window-workspace", "Send window to workspace", "rules-window", "window rule workspace move route open"),
        setting("layer-blur", "Layer blur", "rules-layer", "layer rule blur waybar launcher overlay"),
        setting("layer-animation", "Layer animation", "rules-layer", "layer rule animation no animation transition"),
        setting("layer-order", "Layer order", "rules-layer", "layer rule order z index overlay"),
        setting("gesture-workspace", "Workspace swipe gesture", "settings-gestures", "gesture swipe workspace touchpad fingers"),
        setting("gesture-direction", "Gesture direction", "settings-gestures", "gesture direction invert swipe"),
        setting("bezier", "Bézier curve", "settings-curves", "bezier easing curve animation control points"),
        setting("spring", "Spring animation curve", "settings-curves", "spring easing curve animation stiffness damping"),
        setting("startup", "Startup applications", "startup", "startup autostart login launch exec app command"),
        setting("env", "Environment variables", "environment", "environment env variable session value"),
        setting("submap", "Shortcut submaps", "submaps", "submap mode keybind shortcut layer"),
        setting("bind", "Keyboard shortcuts", "binds", "bind keybind keyboard shortcut hotkey chord"),
        setting("variables", "Config variables", "variables", "variable reusable value template config"),
        setting("backup", "Import / export settings", "import-export", "backup restore import export move settings json"),
        setting("health", "Session diagnostics", "health", "health diagnose issue broken check session"),
        setting("logs", "Session logs", "logs", "logs errors journal journalctl debug"),
    ]
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
