//! Look & Feel — curated, DE-style settings with live `hyprctl eval` preview.

use crate::bind::BindCollection;
use crate::debounce::Debouncer;
use crate::dialog;
use crate::settings_config;
use crate::writer;
use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GtkBox, Button, CheckButton, Entry, Label, Orientation, Scale,
    ScrolledWindow, Separator, Switch, Window,
};
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub type StatusFn = Rc<dyn Fn(String)>;
pub type ReloadFn = Rc<dyn Fn()>;

pub struct LookFeelPage {
    pub page: GtkBox,
}

struct LookFeelWidgets {
    gaps_in: Scale,
    gaps_out: Scale,
    border_size: Scale,
    rounding: Scale,
    active_opacity: Scale,
    inactive_opacity: Scale,
    blur_size: Scale,
    blur_enabled: Switch,
    shadow_enabled: Switch,
    animations_enabled: Switch,
    active_border: Entry,
    inactive_border: Entry,
}

fn int_scale(initial: f64, min: f64, max: f64, step: f64) -> Scale {
    let adj = Adjustment::new(initial, min, max, step, step * 5.0, 0.0);
    let scale = Scale::new(Orientation::Horizontal, Some(&adj));
    scale.set_draw_value(true);
    scale.set_hexpand(true);
    scale.set_value_pos(gtk4::PositionType::Right);
    scale.set_width_request(180);
    scale
}

fn opacity_scale(initial: f64) -> Scale {
    let adj = Adjustment::new(initial, 0.3, 1.0, 0.05, 0.1, 0.0);
    let scale = Scale::new(Orientation::Horizontal, Some(&adj));
    scale.set_draw_value(true);
    scale.set_hexpand(true);
    scale.set_digits(2);
    scale.set_value_pos(gtk4::PositionType::Right);
    scale.set_width_request(180);
    scale
}

/// GNOME/KDE-style row: title + subtitle on the left, control on the right.
fn settings_row(title: &str, subtitle: &str, control: &impl IsA<gtk4::Widget>) -> GtkBox {
    let text = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .valign(gtk4::Align::Center)
        .build();
    text.append(
        &Label::builder()
            .label(title)
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-settings-title"])
            .build(),
    );
    if !subtitle.is_empty() {
        text.append(
            &Label::builder()
                .label(subtitle)
                .halign(gtk4::Align::Start)
                .wrap(true)
                .xalign(0.0)
                .css_classes(["hyprbinds-settings-sub", "dim-label"])
                .build(),
        );
    }

    let row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(16)
        .css_classes(["hyprbinds-settings-row"])
        .build();
    row.append(&text);
    let control = control.clone().upcast::<gtk4::Widget>();
    control.set_valign(gtk4::Align::Center);
    control.set_halign(gtk4::Align::End);
    row.append(&control);
    row
}

fn settings_card(title: &str, rows: &[GtkBox]) -> GtkBox {
    let card = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .css_classes(["hyprbinds-settings-card"])
        .build();
    card.append(
        &Label::builder()
            .label(title)
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-settings-card-title"])
            .build(),
    );
    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            card.append(&Separator::new(Orientation::Horizontal));
        }
        card.append(row);
    }
    card
}

fn lua_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn hyprctl_eval(expr: &str) -> Result<(), String> {
    let output = std::process::Command::new("hyprctl")
        .args(["eval", expr])
        .output()
        .map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}").to_ascii_lowercase();
    if combined.contains("can't work") || combined.contains("error") || !output.status.success() {
        let msg = if !stderr.trim().is_empty() {
            stderr.trim().to_string()
        } else if !stdout.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            "hyprctl eval failed".into()
        };
        return Err(msg);
    }
    Ok(())
}

fn build_live_eval(w: &LookFeelWidgets) -> String {
    let gaps_in = w.gaps_in.value().round() as i64;
    let gaps_out = w.gaps_out.value().round() as i64;
    let border = w.border_size.value().round() as i64;
    let rounding = w.rounding.value().round() as i64;
    let active_op = w.active_opacity.value();
    let inactive_op = w.inactive_opacity.value();
    let blur_size = w.blur_size.value().round() as i64;
    let blur = w.blur_enabled.is_active();
    let shadow = w.shadow_enabled.is_active();
    let anims = w.animations_enabled.is_active();

    let active = w.active_border.text().to_string();
    let inactive = w.inactive_border.text().to_string();
    let mut color_bits = Vec::new();
    if !active.trim().is_empty() {
        color_bits.push(format!(
            "active_border = \"{}\"",
            lua_escape(active.trim())
        ));
    }
    if !inactive.trim().is_empty() {
        color_bits.push(format!(
            "inactive_border = \"{}\"",
            lua_escape(inactive.trim())
        ));
    }

    let general = if color_bits.is_empty() {
        format!(
            "general = {{ gaps_in = {gaps_in}, gaps_out = {gaps_out}, border_size = {border} }}"
        )
    } else {
        format!(
            "general = {{ gaps_in = {gaps_in}, gaps_out = {gaps_out}, border_size = {border}, col = {{ {} }} }}",
            color_bits.join(", ")
        )
    };

    format!(
        "hl.config({{ {general}, decoration = {{ rounding = {rounding}, active_opacity = {active_op:.2}, inactive_opacity = {inactive_op:.2}, blur = {{ enabled = {blur}, size = {blur_size} }}, shadow = {{ enabled = {shadow} }} }}, animations = {{ enabled = {anims} }} }})"
    )
}

fn apply_live(w: &LookFeelWidgets) -> Result<(), String> {
    hyprctl_eval(&build_live_eval(w))
}

fn load_widgets(w: &LookFeelWidgets, merged: &Value, filling: &Cell<bool>) {
    filling.set(true);
    w.gaps_in
        .adjustment()
        .set_value(settings_config::get_i64(merged, &["general", "gaps_in"], 5) as f64);
    w.gaps_out
        .adjustment()
        .set_value(settings_config::get_i64(merged, &["general", "gaps_out"], 20) as f64);
    w.border_size
        .adjustment()
        .set_value(settings_config::get_i64(merged, &["general", "border_size"], 1) as f64);
    w.rounding
        .adjustment()
        .set_value(settings_config::get_i64(merged, &["decoration", "rounding"], 10) as f64);
    w.active_opacity.adjustment().set_value(settings_config::get_f64(
        merged,
        &["decoration", "active_opacity"],
        1.0,
    ));
    w.inactive_opacity.adjustment().set_value(settings_config::get_f64(
        merged,
        &["decoration", "inactive_opacity"],
        1.0,
    ));
    w.blur_size
        .adjustment()
        .set_value(settings_config::get_i64(merged, &["decoration", "blur", "size"], 3) as f64);
    w.blur_enabled.set_active(settings_config::get_bool(
        merged,
        &["decoration", "blur", "enabled"],
        true,
    ));
    w.shadow_enabled.set_active(settings_config::get_bool(
        merged,
        &["decoration", "shadow", "enabled"],
        true,
    ));
    w.animations_enabled.set_active(settings_config::get_bool(
        merged,
        &["animations", "enabled"],
        true,
    ));
    let active = settings_config::get_path(merged, &["general", "col", "active_border"])
        .map(settings_config::color_to_display)
        .unwrap_or_default();
    w.active_border.set_text(&active);
    let inactive = settings_config::get_path(merged, &["general", "col", "inactive_border"])
        .map(settings_config::color_to_display)
        .unwrap_or_default();
    w.inactive_border.set_text(&inactive);
    filling.set(false);
}

fn apply_to_merged(w: &LookFeelWidgets, merged: &mut Value) {
    settings_config::set_path(
        merged,
        &["general", "gaps_in"],
        json!(w.gaps_in.value().round() as i64),
    );
    settings_config::set_path(
        merged,
        &["general", "gaps_out"],
        json!(w.gaps_out.value().round() as i64),
    );
    settings_config::set_path(
        merged,
        &["general", "border_size"],
        json!(w.border_size.value().round() as i64),
    );
    settings_config::set_path(
        merged,
        &["decoration", "rounding"],
        json!(w.rounding.value().round() as i64),
    );
    settings_config::set_path(
        merged,
        &["decoration", "active_opacity"],
        json!((w.active_opacity.value() * 100.0).round() / 100.0),
    );
    settings_config::set_path(
        merged,
        &["decoration", "inactive_opacity"],
        json!((w.inactive_opacity.value() * 100.0).round() / 100.0),
    );
    settings_config::set_path(
        merged,
        &["decoration", "blur", "enabled"],
        json!(w.blur_enabled.is_active()),
    );
    settings_config::set_path(
        merged,
        &["decoration", "blur", "size"],
        json!(w.blur_size.value().round() as i64),
    );
    settings_config::set_path(
        merged,
        &["decoration", "shadow", "enabled"],
        json!(w.shadow_enabled.is_active()),
    );
    settings_config::set_path(
        merged,
        &["animations", "enabled"],
        json!(w.animations_enabled.is_active()),
    );
    if !w.active_border.text().trim().is_empty() {
        settings_config::set_path(
            merged,
            &["general", "col", "active_border"],
            settings_config::parse_color_input(&w.active_border.text()),
        );
    }
    if !w.inactive_border.text().trim().is_empty() {
        settings_config::set_path(
            merged,
            &["general", "col", "inactive_border"],
            settings_config::parse_color_input(&w.inactive_border.text()),
        );
    }
}

fn apply_preset(w: &LookFeelWidgets, name: &str, filling: &Cell<bool>) {
    filling.set(true);
    match name {
        "compact" => {
            w.gaps_in.adjustment().set_value(2.0);
            w.gaps_out.adjustment().set_value(4.0);
            w.border_size.adjustment().set_value(1.0);
            w.rounding.adjustment().set_value(4.0);
            w.active_opacity.adjustment().set_value(1.0);
            w.inactive_opacity.adjustment().set_value(1.0);
            w.blur_size.adjustment().set_value(2.0);
            w.blur_enabled.set_active(false);
            w.shadow_enabled.set_active(false);
            w.animations_enabled.set_active(true);
        }
        "comfortable" => {
            w.gaps_in.adjustment().set_value(5.0);
            w.gaps_out.adjustment().set_value(12.0);
            w.border_size.adjustment().set_value(2.0);
            w.rounding.adjustment().set_value(10.0);
            w.active_opacity.adjustment().set_value(1.0);
            w.inactive_opacity.adjustment().set_value(0.95);
            w.blur_size.adjustment().set_value(5.0);
            w.blur_enabled.set_active(true);
            w.shadow_enabled.set_active(true);
            w.animations_enabled.set_active(true);
        }
        "spacious" => {
            w.gaps_in.adjustment().set_value(12.0);
            w.gaps_out.adjustment().set_value(24.0);
            w.border_size.adjustment().set_value(2.0);
            w.rounding.adjustment().set_value(16.0);
            w.active_opacity.adjustment().set_value(0.98);
            w.inactive_opacity.adjustment().set_value(0.88);
            w.blur_size.adjustment().set_value(8.0);
            w.blur_enabled.set_active(true);
            w.shadow_enabled.set_active(true);
            w.animations_enabled.set_active(true);
        }
        _ => {}
    }
    filling.set(false);
}

pub fn build_lookfeel_page(
    _parent: &impl IsA<Window>,
    state: Rc<RefCell<Option<BindCollection>>>,
    reload: ReloadFn,
    status: StatusFn,
) -> LookFeelPage {
    let gaps_in = int_scale(5.0, 0.0, 40.0, 1.0);
    let gaps_out = int_scale(20.0, 0.0, 60.0, 1.0);
    let border_size = int_scale(1.0, 0.0, 10.0, 1.0);
    let rounding = int_scale(10.0, 0.0, 40.0, 1.0);
    let active_opacity = opacity_scale(1.0);
    let inactive_opacity = opacity_scale(1.0);
    let blur_size = int_scale(3.0, 0.0, 20.0, 1.0);
    let blur_enabled = Switch::builder().valign(gtk4::Align::Center).build();
    let shadow_enabled = Switch::builder().valign(gtk4::Align::Center).build();
    let animations_enabled = Switch::builder().valign(gtk4::Align::Center).build();
    let active_border = Entry::builder()
        .hexpand(true)
        .placeholder_text("rgba(88c0d0ee) or #88c0d0")
        .width_request(200)
        .build();
    let inactive_border = Entry::builder()
        .hexpand(true)
        .placeholder_text("rgba(4c566aaa)")
        .width_request(200)
        .build();

    let widgets = Rc::new(LookFeelWidgets {
        gaps_in: gaps_in.clone(),
        gaps_out: gaps_out.clone(),
        border_size: border_size.clone(),
        rounding: rounding.clone(),
        active_opacity: active_opacity.clone(),
        inactive_opacity: inactive_opacity.clone(),
        blur_size: blur_size.clone(),
        blur_enabled: blur_enabled.clone(),
        shadow_enabled: shadow_enabled.clone(),
        animations_enabled: animations_enabled.clone(),
        active_border: active_border.clone(),
        inactive_border: inactive_border.clone(),
    });

    let live_toggle = CheckButton::builder()
        .label("Live preview")
        .tooltip_text("Apply changes immediately with hyprctl eval (Lua-safe).")
        .active(true)
        .build();
    let refresh_btn = Button::builder().label("Reload values").build();
    let save_btn = Button::builder()
        .label("Save")
        .css_classes(["suggested-action"])
        .build();

    let preset_compact = Button::builder().label("Compact").css_classes(["flat"]).build();
    let preset_comfy = Button::builder()
        .label("Comfortable")
        .css_classes(["flat"])
        .build();
    let preset_space = Button::builder().label("Spacious").css_classes(["flat"]).build();

    let presets = GtkBox::new(Orientation::Horizontal, 6);
    presets.append(
        &Label::builder()
            .label("Presets")
            .css_classes(["dim-label", "caption"])
            .valign(gtk4::Align::Center)
            .build(),
    );
    presets.append(&preset_compact);
    presets.append(&preset_comfy);
    presets.append(&preset_space);

    let toolbar = GtkBox::new(Orientation::Horizontal, 10);
    toolbar.append(&live_toggle);
    toolbar.append(&presets);
    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    toolbar.append(&spacer);
    toolbar.append(&refresh_btn);
    toolbar.append(&save_btn);

    let spacing_card = settings_card(
        "Spacing & windows",
        &[
            settings_row(
                "Inner gaps",
                "Space between tiled windows",
                &gaps_in,
            ),
            settings_row(
                "Outer gaps",
                "Space between windows and screen edges",
                &gaps_out,
            ),
            settings_row(
                "Border size",
                "Thickness of window borders",
                &border_size,
            ),
            settings_row(
                "Corner rounding",
                "How round window corners appear",
                &rounding,
            ),
        ],
    );

    let opacity_card = settings_card(
        "Transparency",
        &[
            settings_row(
                "Active opacity",
                "Opacity of the focused window",
                &active_opacity,
            ),
            settings_row(
                "Inactive opacity",
                "Opacity of unfocused windows",
                &inactive_opacity,
            ),
        ],
    );

    let effects_card = settings_card(
        "Effects",
        &[
            settings_row("Blur", "Blur behind translucent windows", &blur_enabled),
            settings_row("Blur strength", "Blur kernel size", &blur_size),
            settings_row("Shadows", "Drop shadows under windows", &shadow_enabled),
            settings_row(
                "Animations",
                "Window open/close and workspace motion",
                &animations_enabled,
            ),
        ],
    );

    let colors_card = settings_card(
        "Colors",
        &[
            settings_row(
                "Active border",
                "Color of the focused window border",
                &active_border,
            ),
            settings_row(
                "Inactive border",
                "Color of unfocused window borders",
                &inactive_border,
            ),
        ],
    );

    let form = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(16)
        .margin_top(4)
        .margin_bottom(8)
        .css_classes(["hyprbinds-lookfeel"])
        .build();
    form.append(&spacing_card);
    form.append(&opacity_card);
    form.append(&effects_card);
    form.append(&colors_card);

    let scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .child(&form)
        .build();

    let footer_hint = Label::builder()
        .label("Live preview uses hyprctl eval with hl.config (works with Lua configs). Save writes a managed override into your hyprland.lua.")
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    let footer = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .build();
    footer.append(&footer_hint);

    let body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .build();
    let header = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .css_classes(["hyprbinds-page-header"])
        .build();
    header.append(
        &Label::builder()
            .label("Look & Feel")
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-page-title"])
            .build(),
    );
    header.append(
        &Label::builder()
            .label("Appearance controls for Hyprland — adjust, preview live, then save when it looks right.")
            .halign(gtk4::Align::Start)
            .wrap(true)
            .xalign(0.0)
            .css_classes(["hyprbinds-page-hint"])
            .build(),
    );
    toolbar.add_css_class("hyprbinds-toolbar");
    body.append(&header);
    body.append(&toolbar);
    body.append(&scroll);

    let page = dialog::mount_sticky_page(&body, &footer);

    let filling = Rc::new(Cell::new(false));
    let live_on = Rc::new(Cell::new(true));
    live_toggle.connect_toggled({
        let live_on = Rc::clone(&live_on);
        move |btn| live_on.set(btn.is_active())
    });

    let schedule_live = {
        let widgets = Rc::clone(&widgets);
        let filling = Rc::clone(&filling);
        let live_on = Rc::clone(&live_on);
        let status = Rc::clone(&status);
        let debouncer = Debouncer::new();
        Rc::new(move || {
            if filling.get() || !live_on.get() {
                return;
            }
            let widgets = Rc::clone(&widgets);
            let status = Rc::clone(&status);
            debouncer.schedule_after(120, move || match apply_live(&widgets) {
                Ok(()) => status("Live preview applied.".into()),
                Err(e) => status(format!("Live preview failed: {e}")),
            });
        })
    };

    let wire_scale = |scale: &Scale| {
        scale.connect_value_changed({
            let schedule_live = Rc::clone(&schedule_live);
            move |_| schedule_live()
        });
    };
    wire_scale(&gaps_in);
    wire_scale(&gaps_out);
    wire_scale(&border_size);
    wire_scale(&rounding);
    wire_scale(&active_opacity);
    wire_scale(&inactive_opacity);
    wire_scale(&blur_size);

    for sw in [&blur_enabled, &shadow_enabled, &animations_enabled] {
        sw.connect_active_notify({
            let schedule_live = Rc::clone(&schedule_live);
            move |_| schedule_live()
        });
    }

    for entry in [&active_border, &inactive_border] {
        entry.connect_activate({
            let schedule_live = Rc::clone(&schedule_live);
            move |_| schedule_live()
        });
    }

    let reload_form = {
        let state = Rc::clone(&state);
        let widgets = Rc::clone(&widgets);
        let filling = Rc::clone(&filling);
        let status = Rc::clone(&status);
        Rc::new(move || {
            let borrowed = state.borrow();
            let Some(collection) = borrowed.as_ref() else {
                status("Load a config first.".into());
                return;
            };
            load_widgets(&widgets, &collection.config_merged, &filling);
            status("Look & Feel values reloaded from config.".into());
        })
    };

    refresh_btn.connect_clicked({
        let reload_form = Rc::clone(&reload_form);
        move |_| reload_form()
    });

    let apply_preset_btn = |btn: &Button, name: &'static str| {
        btn.connect_clicked({
            let widgets = Rc::clone(&widgets);
            let filling = Rc::clone(&filling);
            let schedule_live = Rc::clone(&schedule_live);
            let status = Rc::clone(&status);
            move |_| {
                apply_preset(&widgets, name, &filling);
                schedule_live();
                status(format!("Applied {name} preset."));
            }
        });
    };
    apply_preset_btn(&preset_compact, "compact");
    apply_preset_btn(&preset_comfy, "comfortable");
    apply_preset_btn(&preset_space, "spacious");

    save_btn.connect_clicked({
        let state = Rc::clone(&state);
        let widgets = Rc::clone(&widgets);
        let reload = Rc::clone(&reload);
        let status = Rc::clone(&status);
        move |_| {
            let path = state.borrow().as_ref().map(|c| c.config_path.clone());
            let Some(path) = path else {
                status("Load a config first.".into());
                return;
            };
            let mut merged = state
                .borrow()
                .as_ref()
                .map(|c| c.config_merged.clone())
                .unwrap_or(json!({}));
            if !merged.is_object() {
                merged = json!({});
            }
            apply_to_merged(&widgets, &mut merged);
            // Also push live so session matches saved file.
            let _ = apply_live(&widgets);
            match writer::save_config_override(&path, &merged) {
                Ok(r) => {
                    if let Some(c) = state.borrow_mut().as_mut() {
                        c.config_merged = merged;
                    }
                    status(format!("Saved Look & Feel to {}", r.path));
                    reload();
                }
                Err(e) => status(format!("Save failed: {e}")),
            }
        }
    });

    reload_form();

    LookFeelPage { page }
}
