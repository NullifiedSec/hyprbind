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
            "Start with the workspace and, optionally, a monitor. Gaps, layout, persistence, borders and shadows can be layered on only when you need them.",
        ),
        "Layer rules" => Some(
            "Pick a running layer namespace when possible, then enable only the effects you want. Regex, ordering and custom animation remain available for advanced cases.",
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
            "For most displays, pick the output and use preferred mode, automatic position and scale 1. Only set transform, mirror or a custom mode when the simple setup is not enough.",
        ),
        "Devices" => Some(
            "Pick the device first. Usually sensitivity plus natural scrolling/tap-to-click is enough; keyboard layout and low-level overrides can stay untouched unless that device needs them.",
        ),
        "Animations" => Some(
            "Choose what should animate, pick a curve, then tune speed. Style strings are optional and only needed for effects such as pop-in or slide variants.",
        ),
        "Curves" => Some(
            "Start from a preset and preview the graph. Drag Bézier handles for feel, or choose Spring when you specifically want physical bounce; exact values remain editable.",
        ),
        "Gestures" => Some(
            "Think in three steps: finger count, direction, action. Modifiers and scale are optional refinements, not required for a basic swipe gesture.",
        ),
        _ => None,
    }
}

fn complex_page_help(title: &str) -> Option<(&'static str, &'static [&'static str])> {
    match title {
        "Monitors" => Some((
            "Common setup",
            &[
                "Output → pick the connector reported by Hyprland (for example DP-1).",
                "Mode → preferred is the safest default; use 1920x1080@144 only when you need an exact mode.",
                "Position → auto lets Hyprland place it; use 0x0-style coordinates for manual layouts.",
                "Scale → 1 is normal size. Values above 1 enlarge the desktop on high-DPI displays.",
                "Transform 0 is normal orientation. Leave mirror and disabled alone unless you explicitly need them.",
            ],
        )),
        "Devices" => Some((
            "Common setup",
            &[
                "Pick the physical device instead of typing its internal name.",
                "Sensitivity is the main pointer-speed control; small changes are easier to tune than large jumps.",
                "Natural scroll, tap-to-click and disable-while-typing are ordinary on/off choices.",
                "Keyboard layout/options only affect this device, so leave them empty to inherit the global configuration.",
            ],
        )),
        "Animations" => Some((
            "Build an animation",
            &[
                "Leaf = what moves (windows, fade, workspaces, etc.). Use the provided leaf picker when possible.",
                "Curve = how motion accelerates. Existing curves are selectable; you do not need to type curve names.",
                "Speed controls Bézier animations. Spring curves intentionally ignore the speed field.",
                "Style is optional. Suggested style buttons appear for leaves that support variants.",
            ],
        )),
        "Curves" => Some((
            "Choose by feel",
            &[
                "Bézier is the normal choice for smooth UI motion; start from a preset and drag the handles.",
                "Spring is for bounce/physical motion and exposes mass, stiffness and dampening.",
                "The graph is the source of truth: tune visually first, then use exact numeric fields only when you need precision.",
            ],
        )),
        "Gestures" => Some((
            "Build a gesture",
            &[
                "Fingers → how many fingers must be on the trackpad.",
                "Direction → horizontal/vertical or the supported directional form for the action.",
                "Action → what the gesture actually does, such as changing workspace.",
                "Mods and Scale are optional advanced refinements; a useful gesture does not require either.",
            ],
        )),
        "Workspace rules" => Some((
            "Start simple",
            &[
                "Workspace identifies the target (number, name or selector).",
                "Monitor pins that workspace to an output; use the monitor picker rather than memorizing connector names.",
                "Gaps and layout are overrides. Leave them empty to inherit your normal desktop settings.",
                "Default, Persistent, No border and No shadow are independent switches—only enable the behavior you actually want.",
            ],
        )),
        "Layer rules" => Some((
            "Start simple",
            &[
                "Namespace identifies a bar, launcher or overlay. Pick from running layers whenever possible.",
                "Blur, No animation and Dim around are direct effects and can be combined.",
                "Order and custom Animation are advanced overrides; leave them empty unless layering or motion needs explicit control.",
                "Raw regex matching is still supported for rules that must cover several namespaces.",
            ],
        )),
        _ => None,
    }
}

fn build_complex_help(title: &str) -> Option<GtkBox> {
    let (heading, items) = complex_page_help(title)?;
    let card = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(SPACE_SM)
        .margin_top(SPACE_XS)
        .css_classes(["hyprbinds-settings-card", "hyprbinds-guided-help"])
        .build();
    card.append(
        &Label::builder()
            .label(heading)
            .halign(Align::Start)
            .xalign(0.0)
            .css_classes(["hyprbinds-settings-card-title"])
            .build(),
    );
    for item in items {
        card.append(
            &Label::builder()
                .label(format!("• {item}"))
                .halign(Align::Start)
                .xalign(0.0)
                .wrap(true)
                .css_classes(["hyprbinds-settings-sub"])
                .build(),
        );
    }
    Some(card)
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

    if let Some(help) = build_complex_help(title) {
        page.append(&help);
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
