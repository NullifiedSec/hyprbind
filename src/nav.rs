//! Shared navigation taxonomy and persistent sidebar.

use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Entry, Label, ListBox, ListBoxRow, Orientation, PolicyType,
    ScrolledWindow, SelectionMode,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone, Copy)]
pub struct NavItem {
    pub id: &'static str,
    pub icon: &'static str,
    pub label: &'static str,
    pub subtitle: &'static str,
    pub keywords: &'static str,
    pub dev_only: bool,
}

#[derive(Clone, Copy)]
pub struct NavSection {
    pub title: &'static str,
    pub items: &'static [NavItem],
}

pub const NAV: &[NavSection] = &[
    NavSection {
        title: "Start here",
        items: &[NavItem {
            id: "overview",
            icon: "◎",
            label: "Overview",
            subtitle: "Global consistency controls",
            keywords: "overview global animation speed slider consistency simple",
            dev_only: false,
        }],
    },
    NavSection {
        title: "Keyboard & config",
        items: &[
            NavItem { id: "binds", icon: "⌨", label: "Binds", subtitle: "Keybinds and chords", keywords: "keyboard hotkey shortcut", dev_only: false },
            NavItem { id: "variables", icon: "⟨⟩", label: "Variables", subtitle: "String variables", keywords: "var template", dev_only: false },
            NavItem { id: "environment", icon: "⧉", label: "Environment", subtitle: "hl.env entries", keywords: "env environment", dev_only: false },
            NavItem { id: "submaps", icon: "▦", label: "Submaps", subtitle: "Bind modes", keywords: "mode submap", dev_only: false },
            NavItem { id: "startup", icon: "⏻", label: "Startup", subtitle: "Autostart commands", keywords: "exec autostart boot", dev_only: false },
        ],
    },
    NavSection {
        title: "Rules",
        items: &[
            NavItem { id: "rules-window", icon: "▣", label: "Window rules", subtitle: "Match class and title", keywords: "window rule", dev_only: false },
            NavItem { id: "rules-workspace", icon: "▤", label: "Workspace rules", subtitle: "Gaps, layout, monitors", keywords: "workspace", dev_only: false },
            NavItem { id: "rules-layer", icon: "▥", label: "Layer rules", subtitle: "Bars and overlays", keywords: "layer bar overlay", dev_only: false },
        ],
    },
    NavSection {
        title: "Appearance & input",
        items: &[
            NavItem { id: "lookfeel", icon: "◈", label: "Look & Feel", subtitle: "Gaps, blur, rounding", keywords: "appearance theme visual", dev_only: false },
            NavItem { id: "settings-config", icon: "⚙", label: "Config", subtitle: "General Hyprland settings", keywords: "general config", dev_only: false },
            NavItem { id: "settings-monitors", icon: "🖵", label: "Monitors", subtitle: "Layout and scale", keywords: "display output", dev_only: false },
            NavItem { id: "settings-devices", icon: "🖱", label: "Devices", subtitle: "Keyboards and mice", keywords: "input device", dev_only: false },
            NavItem { id: "settings-animations", icon: "✦", label: "Animations", subtitle: "Motion leaves", keywords: "animation motion", dev_only: false },
            NavItem { id: "settings-curves", icon: "∿", label: "Curves", subtitle: "Bézier and spring", keywords: "bezier spring easing", dev_only: false },
            NavItem { id: "settings-gestures", icon: "✋", label: "Gestures", subtitle: "Trackpad actions", keywords: "gesture touchpad", dev_only: false },
        ],
    },
    NavSection {
        title: "System",
        items: &[
            NavItem { id: "health", icon: "❤", label: "Health", subtitle: "Session diagnostics", keywords: "health diagnose check", dev_only: false },
            NavItem { id: "logs", icon: "☰", label: "Logs", subtitle: "journalctl session", keywords: "logs journal journalctl", dev_only: false },
            NavItem { id: "import-export", icon: "⇄", label: "Import / Export", subtitle: "Backup and restore as JSON", keywords: "import export backup restore json bundle settings", dev_only: false },
        ],
    },
    NavSection {
        title: "Experimental",
        items: &[
            NavItem { id: "wallpaper", icon: "🖼", label: "Wallpaper", subtitle: "Requires Developer mode · awww backgrounds", keywords: "wallpaper background image experimental", dev_only: true },
            NavItem { id: "waybar", icon: "━", label: "Waybar", subtitle: "Requires Developer mode · bar layout and style", keywords: "waybar status bar experimental", dev_only: true },
            NavItem { id: "starship", icon: "❯", label: "Starship", subtitle: "Requires Developer mode · shell prompt", keywords: "starship prompt shell zsh fish bash experimental", dev_only: true },
            NavItem { id: "via", icon: "⬡", label: "VIA keymap", subtitle: "Requires Developer mode · hardware remaps over USB", keywords: "via qmk vial hardware keymap keyboard definition experimental", dev_only: true },
            NavItem { id: "screenshare", icon: "⏺", label: "Screenshare", subtitle: "Requires Developer mode · portal and PipeWire", keywords: "screenshare portal pipewire experimental", dev_only: true },
            NavItem { id: "audio", icon: "♪", label: "Audio", subtitle: "Requires Developer mode · volume and devices", keywords: "audio sound volume pulse experimental", dev_only: true },
        ],
    },
];

pub fn page_chrome_title(page_id: &str) -> &'static str {
    if page_id == "home" || page_id == "settings" || page_id == "rules" || page_id == "system" {
        return match page_id {
            "home" => "Overview",
            "settings" => "Look & Feel",
            "rules" => "Window rules",
            "system" => "Health",
            _ => "Settings",
        };
    }
    for section in NAV {
        for item in section.items {
            if item.id == page_id {
                return if item.dev_only && item.id == "via" {
                    "VIA keymap (experimental)"
                } else {
                    item.label
                };
            }
        }
    }
    "Settings"
}

pub fn find_item(id: &str) -> Option<&'static NavItem> {
    NAV.iter().flat_map(|section| section.items.iter()).find(|item| item.id == id)
}

pub struct Sidebar {
    pub widget: GtkBox,
    pub filter: Entry,
    list: ListBox,
    developer_mode: Rc<Cell<bool>>,
    selecting: Rc<Cell<bool>>,
    section_rows: Rc<RefCell<Vec<ListBoxRow>>>,
    item_rows: Rc<RefCell<Vec<(ListBoxRow, &'static NavItem)>>>,
}

impl Sidebar {
    pub fn select(&self, id: &str) {
        self.selecting.set(true);
        let mut found = false;
        for (row, item) in self.item_rows.borrow().iter() {
            if item.id == id {
                self.list.select_row(Some(row));
                found = true;
                break;
            }
        }
        if !found {
            self.list.unselect_all();
        }
        self.selecting.set(false);
        self.apply_active_class(id);
    }

    pub fn set_developer_mode(&self, enabled: bool) {
        self.developer_mode.set(enabled);
        apply_sidebar_filter(
            &self.section_rows.borrow(),
            &self.item_rows.borrow(),
            &self.filter.text(),
            enabled,
        );
    }

    fn apply_active_class(&self, id: &str) {
        for (row, item) in self.item_rows.borrow().iter() {
            if item.id == id {
                row.add_css_class("nav-active");
            } else {
                row.remove_css_class("nav-active");
            }
        }
    }
}

pub fn build_sidebar(on_navigate: Rc<dyn Fn(&str)>, developer_mode: bool) -> Sidebar {
    let brand_mark = Label::builder()
        .label("H")
        .halign(Align::Start)
        .css_classes(["hyprbinds-brand-mark"])
        .build();
    let brand_title = Label::builder()
        .label("Hyprbinds")
        .halign(Align::Start)
        .css_classes(["hyprbinds-brand-title"])
        .build();
    let brand_sub = Label::builder()
        .label("Hyprland control center")
        .halign(Align::Start)
        .css_classes(["hyprbinds-brand-sub"])
        .build();
    let brand_text = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .build();
    brand_text.append(&brand_title);
    brand_text.append(&brand_sub);
    let brand = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .css_classes(["hyprbinds-brand"])
        .build();
    brand.append(&brand_mark);
    brand.append(&brand_text);

    let filter = Entry::builder()
        .placeholder_text("Search settings")
        .hexpand(true)
        .css_classes(["hyprbinds-sidebar-filter"])
        .build();

    let list = ListBox::builder()
        .selection_mode(SelectionMode::Single)
        .css_classes(["hyprbinds-sidebar-list"])
        .build();

    let section_rows: Rc<RefCell<Vec<ListBoxRow>>> = Rc::new(RefCell::new(Vec::new()));
    let item_rows: Rc<RefCell<Vec<(ListBoxRow, &'static NavItem)>>> = Rc::new(RefCell::new(Vec::new()));
    let developer_mode = Rc::new(Cell::new(developer_mode));
    let selecting = Rc::new(Cell::new(false));

    for section in NAV {
        let header = Label::builder()
            .label(section.title)
            .halign(Align::Start)
            .xalign(0.0)
            .css_classes(["hyprbinds-sidebar-section"])
            .build();
        let header_row = ListBoxRow::builder()
            .child(&header)
            .activatable(false)
            .selectable(false)
            .css_classes(["hyprbinds-sidebar-header-row"])
            .build();
        list.append(&header_row);
        section_rows.borrow_mut().push(header_row);

        for item in section.items {
            let icon = Label::builder()
                .label(item.icon)
                .valign(Align::Center)
                .css_classes(["hyprbinds-sidebar-icon"])
                .build();
            let title = Label::builder()
                .label(item.label)
                .halign(Align::Start)
                .ellipsize(gtk4::pango::EllipsizeMode::End)
                .hexpand(true)
                .css_classes(["hyprbinds-sidebar-label"])
                .build();
            let subtitle = Label::builder()
                .label(item.subtitle)
                .halign(Align::Start)
                .ellipsize(gtk4::pango::EllipsizeMode::End)
                .hexpand(true)
                .css_classes(["hyprbinds-sidebar-subtitle"])
                .build();
            let text = GtkBox::builder()
                .orientation(Orientation::Vertical)
                .spacing(1)
                .hexpand(true)
                .build();
            text.append(&title);
            text.append(&subtitle);
            let row_box = GtkBox::builder()
                .orientation(Orientation::Horizontal)
                .spacing(10)
                .css_classes(["hyprbinds-sidebar-row-inner"])
                .build();
            row_box.append(&icon);
            row_box.append(&text);

            let row = ListBoxRow::builder()
                .child(&row_box)
                .activatable(true)
                .selectable(true)
                .name(item.id)
                .css_classes(["hyprbinds-sidebar-row"])
                .build();
            if item.dev_only {
                row.add_css_class("hyprbinds-dev-only");
                if !developer_mode.get() {
                    row.set_visible(false);
                }
            }
            list.append(&row);
            item_rows.borrow_mut().push((row, item));
        }
    }

    apply_sidebar_filter(
        &section_rows.borrow(),
        &item_rows.borrow(),
        "",
        developer_mode.get(),
    );

    list.connect_row_selected({
        let on_navigate = Rc::clone(&on_navigate);
        let selecting = Rc::clone(&selecting);
        let item_rows = Rc::clone(&item_rows);
        move |_, row| {
            if selecting.get() {
                return;
            }
            let Some(row) = row else { return; };
            let id = row.widget_name().to_string();
            if id.is_empty() || find_item(&id).is_none() {
                return;
            }
            for (r, item) in item_rows.borrow().iter() {
                if item.id == id {
                    r.add_css_class("nav-active");
                } else {
                    r.remove_css_class("nav-active");
                }
            }
            on_navigate(&id);
        }
    });

    filter.connect_changed({
        let section_rows = Rc::clone(&section_rows);
        let item_rows = Rc::clone(&item_rows);
        let developer_mode = Rc::clone(&developer_mode);
        move |entry| {
            apply_sidebar_filter(
                &section_rows.borrow(),
                &item_rows.borrow(),
                &entry.text(),
                developer_mode.get(),
            );
        }
    });

    let scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .child(&list)
        .css_classes(["hyprbinds-sidebar-scroll"])
        .build();

    let widget = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .css_classes(["hyprbinds-sidebar"])
        .build();
    widget.append(&brand);
    widget.append(&filter);
    widget.append(&scroll);
    widget.set_size_request(270, -1);

    Sidebar {
        widget,
        filter,
        list,
        developer_mode,
        selecting,
        section_rows,
        item_rows,
    }
}

fn apply_sidebar_filter(
    section_rows: &[ListBoxRow],
    item_rows: &[(ListBoxRow, &'static NavItem)],
    query: &str,
    developer_mode: bool,
) {
    let q = query.trim().to_lowercase();
    let mut offset = 0usize;

    for (s_i, section) in NAV.iter().enumerate() {
        let mut any_visible = false;
        for item in section.items {
            let Some((row, stored)) = item_rows.get(offset) else { break; };
            debug_assert_eq!(stored.id, item.id);
            let hay = format!("{} {} {} {}", item.id, item.label, item.subtitle, item.keywords).to_lowercase();
            let unlocked = developer_mode || !item.dev_only;
            let show = unlocked && (q.is_empty() || hay.contains(&q));
            row.set_visible(show);
            any_visible |= show;
            offset += 1;
        }
        if let Some(header) = section_rows.get(s_i) {
            header.set_visible(any_visible);
        }
    }
}
