//! Screenshare doctor — guided portal / PipeWire checklist with apply actions.

use crate::dialog;
use crate::health::{self, CheckResult, Severity};
use gtk4::gdk;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, Label, ListBox, ListBoxRow, Orientation, PolicyType, ScrolledWindow,
};
use std::process::Command;
use std::rc::Rc;

pub struct ScreensharePage {
    pub page: GtkBox,
}

pub fn build_screenshare_page(status: Rc<dyn Fn(String)>) -> ScreensharePage {
    let summary = Label::builder()
        .label("Run the doctor to check portals and PipeWire for screenshare.")
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["hyprbinds-health-summary"])
        .build();

    let run_btn = Button::builder()
        .label("Run doctor")
        .css_classes(["suggested-action"])
        .build();
    let apply_portal_btn = Button::builder()
        .label("Apply portal preference")
        .tooltip_text("Write ~/.config/xdg-desktop-portal/hyprland-portals.conf and restart portals")
        .build();
    let restart_btn = Button::builder()
        .label("Restart portals")
        .build();
    let copy_btn = Button::builder()
        .label("Copy report")
        .sensitive(false)
        .build();

    let actions = GtkBox::new(Orientation::Horizontal, 8);
    actions.set_halign(gtk4::Align::End);
    actions.append(&run_btn);
    actions.append(&apply_portal_btn);
    actions.append(&restart_btn);
    actions.append(&copy_btn);

    let toolbar = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .build();
    let top = GtkBox::new(Orientation::Horizontal, 8);
    top.append(&summary);
    toolbar.append(&top);
    toolbar.append(&actions);

    let steps = Label::builder()
        .label(
            "1) Wayland session  ·  2) xdg-desktop-portal + hyprland backend  ·  3) PipeWire  ·  4) Portal preference  ·  5) Restart browser after fixes",
        )
        .halign(gtk4::Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    toolbar.append(&steps);

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
        "Screenshare",
        "Guided checks for screen sharing on Hyprland. Apply fixes here, then relaunch the browser/app that shares.",
        &toolbar,
        &scroll,
    );

    let last_report = Rc::new(std::cell::RefCell::new(String::new()));

    let refresh = {
        let list = list.clone();
        let summary = summary.clone();
        let copy_btn = copy_btn.clone();
        let status = Rc::clone(&status);
        let last_report = Rc::clone(&last_report);
        Rc::new(move || {
            let results: Vec<CheckResult> = health::run_all()
                .into_iter()
                .filter(|c| c.category == "Screenshare" || c.category == "Session" || c.category == "Audio")
                .collect();
            let (ok, warn, fail) = health::summarize(&results);
            summary.set_text(&format!(
                "Screenshare doctor: {ok} ok · {warn} warnings · {fail} failures"
            ));
            status(format!(
                "Screenshare doctor: {ok} ok, {warn} warnings, {fail} failures"
            ));
            *last_report.borrow_mut() = format_doctor_report(&results);
            copy_btn.set_sensitive(true);

            while let Some(child) = list.first_child() {
                list.remove(&child);
            }
            let mut last_cat = String::new();
            for check in &results {
                if check.category != last_cat {
                    last_cat = check.category.to_string();
                    list.append(&category_row(check.category));
                }
                list.append(&check_row(check, Rc::clone(&status)));
            }
        })
    };

    run_btn.connect_clicked({
        let refresh = Rc::clone(&refresh);
        move |_| refresh()
    });

    apply_portal_btn.connect_clicked({
        let status = Rc::clone(&status);
        let refresh = Rc::clone(&refresh);
        move |_| match apply_portal_preference() {
            Ok(msg) => {
                status(msg);
                refresh();
            }
            Err(e) => status(format!("Portal preference failed: {e}")),
        }
    });

    restart_btn.connect_clicked({
        let status = Rc::clone(&status);
        let refresh = Rc::clone(&refresh);
        move |_| match restart_portals() {
            Ok(msg) => {
                status(msg);
                refresh();
            }
            Err(e) => status(format!("Restart failed: {e}")),
        }
    });

    copy_btn.connect_clicked({
        let last_report = Rc::clone(&last_report);
        let status = Rc::clone(&status);
        move |_| {
            let text = last_report.borrow().clone();
            if text.is_empty() {
                return;
            }
            if let Some(display) = gdk::Display::default() {
                display.clipboard().set_text(&text);
                status("Copied screenshare doctor report.".into());
            }
        }
    });

    refresh();
    ScreensharePage { page }
}

pub fn apply_portal_preference() -> Result<String, String> {
    let home = dirs::home_dir().ok_or_else(|| "no home dir".to_string())?;
    let dir = home.join(".config/xdg-desktop-portal");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("hyprland-portals.conf");
    std::fs::write(&path, "[preferred]\ndefault=hyprland;gtk\n").map_err(|e| e.to_string())?;
    let _ = restart_portals();
    Ok(format!(
        "Wrote {} and restarted portals.",
        path.display()
    ))
}

pub fn restart_portals() -> Result<String, String> {
    let status = Command::new("systemctl")
        .args([
            "--user",
            "restart",
            "xdg-desktop-portal.service",
            "xdg-desktop-portal-hyprland.service",
        ])
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok("Restarted xdg-desktop-portal and xdg-desktop-portal-hyprland.".into())
    } else {
        Err(format!("systemctl exited with {status}"))
    }
}

fn category_row(title: &str) -> ListBoxRow {
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
    let text_col = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(3)
        .hexpand(true)
        .build();
    text_col.append(&dialog::row_title(&check.title));
    text_col.append(&dialog::row_body(&check.detail));
    if let Some(hint) = &check.fix_hint {
        text_col.append(&dialog::row_meta(hint));
    }
    let top = GtkBox::new(Orientation::Horizontal, 12);
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
                    status("Copied fix command.".into());
                }
            }
        });
        col.append(&copy);
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

fn format_doctor_report(results: &[CheckResult]) -> String {
    let (ok, warn, fail) = health::summarize(results);
    let mut out = format!("Hyprbinds screenshare doctor\n{ok} ok · {warn} warnings · {fail} failures\n\n");
    for r in results {
        out.push_str(&format!("[{}] {} — {}\n", r.severity.label(), r.title, r.detail));
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
