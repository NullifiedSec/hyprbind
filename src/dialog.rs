//! Shared chrome — sticky dialogs, page shells, and list-row helpers.

use gtk4::gdk::Key;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, EventControllerKey, Label, Orientation, PolicyType,
    ScrolledWindow, Separator, Window,
};

const SPACE_XS: i32 = 4;
const SPACE_SM: i32 = 8;
const SPACE_MD: i32 = 12;
const SPACE_LG: i32 = 16;

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
    let row = GtkBox::new(Orientation::Horizontal, SPACE_SM);
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
        .spacing(SPACE_SM)
        .margin_top(SPACE_MD)
        .margin_bottom(SPACE_LG)
        .margin_start(SPACE_LG)
        .margin_end(SPACE_LG)
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
        .spacing(SPACE_SM)
        .margin_top(SPACE_MD)
        .margin_bottom(SPACE_XS)
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

fn page_guidance(title: &str) -> Option<&'static str> {
    match title {
        "Binds" => Some(
            "Search narrows the list instantly. Select a bind to edit or delete it; double-click opens the editor.",
        ),
        "Variables" => Some(
            "Search by variable name or value. Select a variable to edit or delete it; Add creates a new one.",
        ),
        "Window rules" => Some(
            "Search existing rules, then select one to edit or delete it. Add creates a rule for a new window match.",
        ),
        "Workspace rules" => Some(
            "Target a workspace first. Add monitor, gaps or layout overrides only when that workspace needs different behavior.",
        ),
        "Layer rules" => Some(
            "Pick a running layer first, choose the visible effect you want, and only use raw matching or ordering for edge cases.",
        ),
        "Environment" => Some(
            "Environment entries are key/value pairs exported to your Hyprland session. Select an entry to change it.",
        ),
        "Submaps" => Some(
            "Submaps are temporary keybind modes. Select one to inspect or edit its bindings.",
        ),
        "Startup" => Some(
            "Startup entries run when Hyprland starts. Keep one command per entry so they stay easy to manage.",
        ),
        "Monitors" => Some(
            "Start from the display you want to configure. Preferred mode, automatic placement and scale 1 cover most setups.",
        ),
        "Devices" => Some(
            "Pick the physical device first, tune the behavior you care about, and leave inherited settings untouched.",
        ),
        "Animations" => Some(
            "Choose what moves, choose how it feels, then adjust speed. Style is optional rather than part of the basic path.",
        ),
        "Curves" => Some(
            "Choose a feel first and tune visually. Exact Bézier or spring values stay available when precision matters.",
        ),
        "Gestures" => Some(
            "Build gestures as an intent: fingers → direction → action. Modifiers and scale are optional refinements.",
        ),
        _ => None,
    }
}

fn intent_flow(title: &str) -> Option<(&'static str, &'static str, &'static str, &'static str)> {
    match title {
        "Monitors" => Some((
            "Pick display",
            "Choose layout",
            "Fine-tune only if needed",
            "Exact modes, coordinates, transform, mirror and disable remain available.",
        )),
        "Devices" => Some((
            "Pick device",
            "Tune feel",
            "Add device-only overrides",
            "Raw device name, keyboard options and low-level values remain available.",
        )),
        "Animations" => Some((
            "Choose what moves",
            "Choose motion feel",
            "Tune speed or style",
            "Leaf names, exact curve references and style strings remain editable.",
        )),
        "Curves" => Some((
            "Start from a preset",
            "Preview the motion",
            "Adjust visually",
            "Exact Bézier points and spring physics remain editable.",
        )),
        "Gestures" => Some((
            "Choose fingers",
            "Choose direction",
            "Choose action",
            "Modifiers, scale and raw action values remain available.",
        )),
        "Workspace rules" => Some((
            "Choose workspace",
            "Optionally pin monitor",
            "Add only needed overrides",
            "Selectors, gaps, layout and rule flags remain fully available.",
        )),
        "Layer rules" => Some((
            "Pick layer",
            "Choose effect",
            "Add advanced ordering if needed",
            "Regex namespace matching and custom animation remain available.",
        )),
        _ => None,
    }
}

fn intent_step(number: u8, text: &str) -> GtkBox {
    let number = Label::builder()
        .label(number.to_string())
        .halign(Align::Center)
        .valign(Align::Center)
        .css_classes(["hyprbinds-intent-number"])
        .build();
    let label = Label::builder()
        .label(text)
        .halign(Align::Start)
        .valign(Align::Center)
        .xalign(0.0)
        .wrap(true)
        .hexpand(true)
        .css_classes(["hyprbinds-intent-label"])
        .build();
    let step = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(SPACE_SM)
        .hexpand(true)
        .css_classes(["hyprbinds-intent-step"])
        .build();
    step.append(&number);
    step.append(&label);
    step
}

fn build_intent_flow(title: &str) -> Option<GtkBox> {
    let (one, two, three, advanced) = intent_flow(title)?;

    let steps = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(SPACE_SM)
        .homogeneous(true)
        .hexpand(true)
        .build();
    steps.append(&intent_step(1, one));
    steps.append(&intent_step(2, two));
    steps.append(&intent_step(3, three));

    let advanced = Label::builder()
        .label(advanced)
        .halign(Align::Start)
        .xalign(0.0)
        .wrap(true)
        .css_classes(["dim-label", "caption", "hyprbinds-intent-advanced"])
        .build();

    let flow = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(SPACE_SM)
        .css_classes(["hyprbinds-intent-flow"])
        .build();
    flow.append(&steps);
    flow.append(&advanced);
    Some(flow)
}

/// Standard workspace page: title, concise explanation, actions, then content.
pub fn page_shell(
    title: &str,
    hint: &str,
    toolbar: &GtkBox,
    content: &impl IsA<gtk4::Widget>,
) -> GtkBox {
    toolbar.add_css_class("hyprbinds-toolbar");

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
        .spacing(SPACE_XS)
        .hexpand(true)
        .build();
    header_text.append(&title_label);
    header_text.append(&hint_label);

    let header = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(SPACE_LG)
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
        .spacing(SPACE_SM)
        .hexpand(true)
        .vexpand(true)
        .css_classes(["hyprbinds-page"])
        .build();
    page.append(&header);
    page.append(toolbar);

    if let Some(guidance) = page_guidance(title) {
        let guide = Label::builder()
            .label(guidance)
            .halign(Align::Start)
            .xalign(0.0)
            .wrap(true)
            .css_classes(["dim-label", "caption", "hyprbinds-page-guidance"])
            .build();
        page.append(&guide);
    }

    if let Some(flow) = build_intent_flow(title) {
        page.append(&flow);
    }

    page.append(&canvas);
    page
}

pub fn list_row_column() -> GtkBox {
    GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(SPACE_XS)
        .margin_top(10)
        .margin_bottom(10)
        .margin_start(14)
        .margin_end(14)
        .css_classes(["hyprbinds-row"])
        .build()
}

pub fn list_row_column_compact() -> GtkBox {
    GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .margin_top(SPACE_SM)
        .margin_bottom(SPACE_SM)
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
