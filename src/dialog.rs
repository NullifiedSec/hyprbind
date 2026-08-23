//! Shared chrome — sticky dialogs, page shells, and list-row helpers.

use gtk4::gdk::Key;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, EventControllerKey, Label, Orientation, PolicyType,
    ScrolledWindow, Separator, Window,
};

pub fn close_on_escape(window: &Window) {
    let controller = EventControllerKey::new();
    controller.connect_key_pressed({
        let window = window.clone();
        move |_, key, _, _| {
            if key == Key::Escape {
                window.close();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        }
    });
    window.add_controller(controller);
}

pub fn action_buttons(cancel: &Button, primary: &Button) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 10);
    row.set_halign(Align::End);
    row.set_hexpand(true);
    cancel.set_hexpand(false);
    primary.set_hexpand(false);
    row.append(cancel);
    row.append(primary);
    row
}

pub fn mount_sticky_dialog(
    editor: &Window,
    body: &impl IsA<gtk4::Widget>,
    status: Option<&Label>,
    actions: &GtkBox,
) {
    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .hexpand(true)
        .child(body)
        .build();

    let footer = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_top(10)
        .margin_bottom(14)
        .margin_start(18)
        .margin_end(18)
        .css_classes(["hyprbinds-sticky-footer"])
        .build();
    if let Some(status) = status {
        status.set_halign(Align::Start);
        status.set_wrap(true);
        footer.append(status);
    }
    footer.append(actions);

    let root = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .build();
    root.append(&scroll);
    root.append(&Separator::new(Orientation::Horizontal));
    root.append(&footer);
    editor.set_child(Some(&root));
    close_on_escape(editor);
}

pub fn mount_sticky_page(body: &impl IsA<gtk4::Widget>, footer_actions: &GtkBox) -> GtkBox {
    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .hexpand(true)
        .child(body)
        .build();

    let footer = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_top(10)
        .margin_bottom(4)
        .css_classes(["hyprbinds-sticky-footer"])
        .build();
    footer.append(footer_actions);

    let page = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .build();
    page.append(&scroll);
    page.append(&Separator::new(Orientation::Horizontal));
    page.append(&footer);
    page
}

/// Modern workspace page: compact title rail, command surface, then content canvas.
pub fn page_shell(
    title: &str,
    hint: &str,
    toolbar: &GtkBox,
    content: &impl IsA<gtk4::Widget>,
) -> GtkBox {
    toolbar.add_css_class("hyprbinds-toolbar");

    let eyebrow = Label::builder()
        .label("HYPRLAND")
        .halign(Align::Start)
        .css_classes(["hyprbinds-page-eyebrow"])
        .build();
    let title_label = Label::builder()
        .label(title)
        .halign(Align::Start)
        .xalign(0.0)
        .css_classes(["hyprbinds-page-title"])
        .build();
    let hint_label = Label::builder()
        .label(hint)
        .halign(Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["hyprbinds-page-hint"])
        .build();

    let header_text = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(3)
        .hexpand(true)
        .build();
    header_text.append(&eyebrow);
    header_text.append(&title_label);
    header_text.append(&hint_label);

    let header = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(18)
        .css_classes(["hyprbinds-page-header"])
        .build();
    header.append(&header_text);

    let canvas = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .hexpand(true)
        .vexpand(true)
        .css_classes(["hyprbinds-page-canvas"])
        .build();
    canvas.append(content);

    let page = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .hexpand(true)
        .vexpand(true)
        .css_classes(["hyprbinds-page"])
        .build();
    page.append(&header);
    page.append(toolbar);
    page.append(&canvas);
    page
}

pub fn list_row_column() -> GtkBox {
    GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .margin_top(11)
        .margin_bottom(11)
        .margin_start(15)
        .margin_end(15)
        .css_classes(["hyprbinds-row"])
        .build()
}

pub fn list_row_column_compact() -> GtkBox {
    GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .margin_top(11)
        .margin_bottom(11)
        .margin_start(14)
        .margin_end(14)
        .css_classes(["hyprbinds-row"])
        .build()
}

pub fn row_title(text: &str) -> Label {
    Label::builder()
        .label(text)
        .halign(Align::Start)
        .hexpand(true)
        .ellipsize(gtk4::pango::EllipsizeMode::End)
        .selectable(true)
        .css_classes(["hyprbinds-row-title"])
        .build()
}

pub fn row_sub(text: &str) -> Label {
    Label::builder()
        .label(text)
        .halign(Align::Start)
        .ellipsize(gtk4::pango::EllipsizeMode::End)
        .selectable(true)
        .css_classes(["hyprbinds-row-sub", "monospace"])
        .build()
}

pub fn row_body(text: &str) -> Label {
    Label::builder()
        .label(text)
        .halign(Align::Start)
        .hexpand(true)
        .wrap(true)
        .xalign(0.0)
        .selectable(true)
        .css_classes(["hyprbinds-row-body"])
        .build()
}

pub fn row_meta(text: &str) -> Label {
    Label::builder()
        .label(text)
        .halign(Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["hyprbinds-row-meta"])
        .build()
}

pub fn row_keys(text: &str) -> Label {
    Label::builder()
        .label(text)
        .halign(Align::End)
        .valign(Align::Center)
        .selectable(true)
        .css_classes(["hyprbinds-row-keys", "monospace"])
        .build()
}

pub fn section_label(text: &str) -> Label {
    Label::builder()
        .label(text)
        .halign(Align::Start)
        .css_classes(["hyprbinds-section"])
        .build()
}
