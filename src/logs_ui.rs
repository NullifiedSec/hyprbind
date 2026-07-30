//! Journal log viewer page — user-session `journalctl` output with live filters.

use crate::dialog;
use crate::logs::{self, LogEntry, SeverityFilter};
use gtk4::gdk;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, DropDown, Entry, Label, ListBox, ListBoxRow, Orientation, PolicyType,
    ScrolledWindow, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

pub struct LogsPage {
    pub page: GtkBox,
}

const SEVERITY_LABELS: &[&str] = &["All", "Errors", "Warnings", "Info", "Debug"];

pub fn build_logs_page(status: Rc<dyn Fn(String)>) -> LogsPage {
    let search = Entry::builder()
        .placeholder_text("Query message, unit, tag…")
        .hexpand(true)
        .build();

    let severity_dd = DropDown::from_strings(SEVERITY_LABELS);
    severity_dd.set_tooltip_text(Some("Filter by severity"));

    let tag_dd = DropDown::from_strings(&["All tags"]);
    tag_dd.set_tooltip_text(Some("Filter by tag"));

    let refresh_btn = Button::builder()
        .label("Refresh")
        .css_classes(["suggested-action"])
        .build();
    let copy_btn = Button::builder()
        .label("Copy visible")
        .sensitive(false)
        .build();

    let toolbar = GtkBox::new(Orientation::Horizontal, 8);
    toolbar.append(&search);
    toolbar.append(&severity_dd);
    toolbar.append(&tag_dd);
    toolbar.append(&refresh_btn);
    toolbar.append(&copy_btn);

    let list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .css_classes(["boxed-list", "hyprbinds-logs-list"])
        .build();
    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .child(&list)
        .build();

    let page = dialog::page_shell(
        "Logs",
        "Recent user-session journal entries for the current boot (`journalctl --user -b`). Use filters to narrow Hyprland, portal, PipeWire, and related services.",
        &toolbar,
        &scroll,
    );

    let entries = Rc::new(RefCell::new(Vec::<LogEntry>::new()));
    let visible_report = Rc::new(RefCell::new(String::new()));

    let render = {
        let list = list.clone();
        let search = search.clone();
        let severity_dd = severity_dd.clone();
        let tag_dd = tag_dd.clone();
        let copy_btn = copy_btn.clone();
        let entries = Rc::clone(&entries);
        let visible_report = Rc::clone(&visible_report);
        Rc::new(move || {
            let query = search.text().to_string();
            let severity_filter = severity_from_index(severity_dd.selected() as usize);
            let tag_filter = selected_tag(&tag_dd);

            while let Some(child) = list.first_child() {
                list.remove(&child);
            }

            let all = entries.borrow();
            let mut visible: Vec<&LogEntry> = all
                .iter()
                .filter(|e| logs::matches(e, &query, severity_filter, &tag_filter))
                .collect();

            // Newest first (journalctl -n returns recent at bottom; reverse for UI).
            visible.reverse();

            let mut report = String::from("Hyprbinds log report (visible entries)\n\n");
            for entry in &visible {
                list.append(&log_row(entry));
                report.push_str(&format_log_line(entry));
                report.push('\n');
            }

            *visible_report.borrow_mut() = report;
            copy_btn.set_sensitive(!visible.is_empty());
        })
    };

    let fetch_and_render = {
        let entries = Rc::clone(&entries);
        let tag_dd = tag_dd.clone();
        let status = Rc::clone(&status);
        let render = Rc::clone(&render);
        Rc::new(move || {
            match logs::fetch(400) {
                Ok(fetched) => {
                    let count = fetched.len();
                    refresh_tag_dropdown(&tag_dd, &fetched);
                    *entries.borrow_mut() = fetched;
                    status(format!("Loaded {count} journal entries."));
                    render();
                }
                Err(e) => {
                    *entries.borrow_mut() = Vec::new();
                    status(format!("Log fetch failed: {e}"));
                    render();
                }
            }
        })
    };

    search.connect_changed({
        let render = Rc::clone(&render);
        move |_| render()
    });
    severity_dd.connect_selected_notify({
        let render = Rc::clone(&render);
        move |_| render()
    });
    tag_dd.connect_selected_notify({
        let render = Rc::clone(&render);
        move |_| render()
    });

    refresh_btn.connect_clicked({
        let fetch = Rc::clone(&fetch_and_render);
        move |_| fetch()
    });

    copy_btn.connect_clicked({
        let visible_report = Rc::clone(&visible_report);
        let status = Rc::clone(&status);
        move |_| {
            let text = visible_report.borrow().clone();
            if text.is_empty() {
                return;
            }
            if let Some(display) = gdk::Display::default() {
                display.clipboard().set_text(&text);
                status("Copied visible log entries to clipboard.".into());
            }
        }
    });

    fetch_and_render();

    LogsPage { page }
}

fn severity_from_index(idx: usize) -> SeverityFilter {
    match idx {
        1 => SeverityFilter::Errors,
        2 => SeverityFilter::Warnings,
        3 => SeverityFilter::InfoAndUp,
        4 => SeverityFilter::Debug,
        _ => SeverityFilter::All,
    }
}

fn selected_tag(tag_dd: &DropDown) -> String {
    let idx = tag_dd.selected() as usize;
    if idx == 0 {
        return String::new();
    }
    tag_dd
        .model()
        .and_then(|m| m.downcast::<StringList>().ok())
        .and_then(|list| list.string(idx as u32).map(|s| s.to_string()))
        .unwrap_or_default()
}

fn refresh_tag_dropdown(tag_dd: &DropDown, entries: &[LogEntry]) {
    let prev = selected_tag(tag_dd);
    let mut labels = vec!["All tags".to_string()];
    labels.extend(logs::known_tags(entries));
    let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
    let model = StringList::new(&refs);
    tag_dd.set_model(Some(&model));
    if prev.is_empty() {
        tag_dd.set_selected(0);
        return;
    }
    if let Some(idx) = labels.iter().position(|t| t == &prev) {
        tag_dd.set_selected(idx as u32);
    } else {
        tag_dd.set_selected(0);
    }
}

fn log_row(entry: &LogEntry) -> ListBoxRow {
    let badge = Label::builder()
        .label(logs::severity_label(entry.severity))
        .css_classes([
            "hyprbinds-log-badge",
            logs::css_class(entry.severity),
        ])
        .valign(gtk4::Align::Start)
        .build();

    let who = if entry.identifier.is_empty() {
        if entry.unit.is_empty() {
            "unknown".to_string()
        } else {
            entry.unit.clone()
        }
    } else if entry.unit.is_empty() {
        entry.identifier.clone()
    } else {
        format!("{} · {}", entry.identifier, entry.unit)
    };

    let header = Label::builder()
        .label(format!("{} · {}", who, entry.timestamp_display))
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .ellipsize(gtk4::pango::EllipsizeMode::End)
        .selectable(true)
        .css_classes(["hyprbinds-row-title"])
        .build();

    let top = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .build();
    top.append(&badge);
    top.append(&header);

    let col = dialog::list_row_column();
    col.append(&top);
    col.append(&dialog::row_body(&entry.message));
    if !entry.tags.is_empty() {
        col.append(&dialog::row_meta(&entry.tags.join(" · ")));
    }

    ListBoxRow::builder()
        .child(&col)
        .activatable(false)
        .selectable(false)
        .css_classes(["hyprbinds-log-row"])
        .build()
}

fn format_log_line(entry: &LogEntry) -> String {
    let who = if entry.identifier.is_empty() {
        entry.unit.as_str()
    } else {
        entry.identifier.as_str()
    };
    format!(
        "[{}] {} · {}\n  {}\n  tags: {}",
        logs::severity_label(entry.severity),
        who,
        entry.timestamp_display,
        entry.message,
        entry.tags.join(", ")
    )
}
