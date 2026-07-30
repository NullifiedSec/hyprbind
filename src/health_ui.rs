//! Health / diagnostics page — run system checks and copy suggested fixes.

use crate::dialog;
use crate::health::{self, CheckResult, Severity};
use crate::sysinfo;
use gtk4::gdk;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, Label, ListBox, ListBoxRow, Orientation, PolicyType,
    ScrolledWindow,
};
use std::rc::Rc;

pub struct HealthPage {
    pub page: GtkBox,
    pub developer_toggle: CheckButton,
}

pub fn build_health_page(
    status: Rc<dyn Fn(String)>,
    developer_mode: bool,
) -> HealthPage {
    let summary = Label::builder()
        .label("Run checks to inspect session, screenshare portals, and audio.")
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["hyprbinds-health-summary"])
        .build();

    let run_btn = Button::builder()
        .label("Run checks")
        .css_classes(["suggested-action"])
        .build();
    let copy_all_btn = Button::builder()
        .label("Copy report")
        .sensitive(false)
        .build();
    let copy_sys_btn = Button::builder().label("Copy system info").build();

    let actions = GtkBox::new(Orientation::Horizontal, 8);
    actions.set_halign(gtk4::Align::End);
    actions.append(&run_btn);
    actions.append(&copy_all_btn);
    actions.append(&copy_sys_btn);

    let developer_toggle = CheckButton::builder()
        .label("Developer mode")
        .tooltip_text(
            "Unlock experimental features such as the VIA keymap page. \
             These are unfinished and may change or break.",
        )
        .active(developer_mode)
        .halign(gtk4::Align::Start)
        .build();
    let developer_hint = Label::builder()
        .label("Experimental pages (VIA keymap) stay hidden until this is on.")
        .halign(gtk4::Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    let developer_box = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .css_classes(["hyprbinds-health-developer"])
        .build();
    developer_box.append(&developer_toggle);
    developer_box.append(&developer_hint);

    let toolbar = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .build();
    let top = GtkBox::new(Orientation::Horizontal, 8);
    top.append(&summary);
    toolbar.append(&top);
    toolbar.append(&actions);
    toolbar.append(&developer_box);

    let list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .css_classes(["boxed-list", "hyprbinds-health-list"])
        .build();
    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .child(&list)
        .build();

    let page = dialog::page_shell(
        "Health",
        "Diagnose Hyprland session issues — portals (screenshare), PipeWire (audio), and compositor reachability. Use Copy system info when asking for help online.",
        &toolbar,
        &scroll,
    );

    let last_report = Rc::new(std::cell::RefCell::new(String::new()));

    let refresh = {
        let list = list.clone();
        let summary = summary.clone();
        let copy_all_btn = copy_all_btn.clone();
        let status = Rc::clone(&status);
        let last_report = Rc::clone(&last_report);
        Rc::new(move || {
            let results = health::run_all();
            let (ok, warn, fail) = health::summarize(&results);
            summary.set_text(&format!(
                "{ok} ok · {warn} warnings · {fail} failures · {} checks",
                results.len()
            ));
            status(format!(
                "Health: {ok} ok, {warn} warnings, {fail} failures"
            ));

            *last_report.borrow_mut() = format_report(&results);
            copy_all_btn.set_sensitive(true);

            while let Some(child) = list.first_child() {
                list.remove(&child);
            }

            let mut last_category = String::new();
            for check in &results {
                if check.category != last_category {
                    last_category = check.category.to_string();
                    list.append(&category_header_row(check.category));
                }
                list.append(&check_row(check, Rc::clone(&status)));
            }
        })
    };

    run_btn.connect_clicked({
        let refresh = Rc::clone(&refresh);
        move |_| refresh()
    });

    copy_all_btn.connect_clicked({
        let last_report = Rc::clone(&last_report);
        let status = Rc::clone(&status);
        move |_| {
            let text = last_report.borrow().clone();
            if text.is_empty() {
                return;
            }
            if let Some(display) = gdk::Display::default() {
                display.clipboard().set_text(&text);
                status("Copied full health report to clipboard.".into());
            }
        }
    });

    copy_sys_btn.connect_clicked({
        let status = Rc::clone(&status);
        move |_| {
            let report = sysinfo::collect_report();
            if let Some(display) = gdk::Display::default() {
                display.clipboard().set_text(&report);
                status(format!(
                    "Copied system info ({} bytes) to clipboard.",
                    report.len()
                ));
            }
        }
    });

    // Auto-run once so the page is useful immediately.
    refresh();

    HealthPage {
        page,
        developer_toggle,
    }
}

fn category_header_row(title: &str) -> ListBoxRow {
    let label = Label::builder()
        .label(title)
        .halign(gtk4::Align::Start)
        .css_classes(["hyprbinds-section"])
        .margin_top(8)
        .margin_bottom(2)
        .margin_start(14)
        .margin_end(14)
        .build();
    ListBoxRow::builder()
        .child(&label)
        .activatable(false)
        .selectable(false)
        .css_classes(["hyprbinds-health-category"])
        .build()
}

fn check_row(check: &CheckResult, status: Rc<dyn Fn(String)>) -> ListBoxRow {
    let badge = Label::builder()
        .label(check.severity.label())
        .css_classes(["hyprbinds-health-badge", check.severity.css_class()])
        .valign(gtk4::Align::Start)
        .build();

    let title = dialog::row_title(&check.title);
    let detail = dialog::row_body(&check.detail);

    let text_col = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(3)
        .hexpand(true)
        .build();
    text_col.append(&title);
    text_col.append(&detail);
    if let Some(hint) = &check.fix_hint {
        text_col.append(&dialog::row_meta(hint));
    }

    let top = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .build();
    top.append(&badge);
    top.append(&text_col);

    let col = dialog::list_row_column();
    col.append(&top);

    if let Some(cmd) = &check.fix_command {
        let copy = Button::builder()
            .label("Copy fix")
            .halign(gtk4::Align::Start)
            .css_classes(["flat"])
            .build();
        let cmd_for_clip = cmd.clone();
        copy.connect_clicked({
            let status = Rc::clone(&status);
            move |_| {
                if let Some(display) = gdk::Display::default() {
                    display.clipboard().set_text(&cmd_for_clip);
                    status("Copied fix command to clipboard.".into());
                }
            }
        });
        col.append(&copy);

        let cmd_preview = Label::builder()
            .label(cmd.as_str())
            .halign(gtk4::Align::Start)
            .wrap(true)
            .xalign(0.0)
            .selectable(true)
            .css_classes(["hyprbinds-health-cmd", "monospace", "dim-label", "caption"])
            .build();
        col.append(&cmd_preview);
    }

    let row = ListBoxRow::builder()
        .child(&col)
        .activatable(false)
        .selectable(false)
        .css_classes(["hyprbinds-health-row"])
        .build();
    match check.severity {
        Severity::Fail => row.add_css_class("hyprbinds-health-row-fail"),
        Severity::Warn => row.add_css_class("hyprbinds-health-row-warn"),
        _ => {}
    }
    row
}

fn format_report(results: &[CheckResult]) -> String {
    let (ok, warn, fail) = health::summarize(results);
    let mut out = format!("Hyprbinds health report\n{ok} ok · {warn} warnings · {fail} failures\n\n");
    let mut last = "";
    for r in results {
        if r.category != last {
            last = r.category;
            out.push_str(&format!("## {}\n", r.category));
        }
        out.push_str(&format!("[{}] {}\n  {}\n", r.severity.label(), r.title, r.detail));
        if let Some(h) = &r.fix_hint {
            out.push_str(&format!("  hint: {h}\n"));
        }
        if let Some(c) = &r.fix_command {
            out.push_str(&format!("  fix: {c}\n"));
        }
        out.push('\n');
    }
    out
}
