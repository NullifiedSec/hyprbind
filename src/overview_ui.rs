//! Overview — dashboard for high-level Hyprland controls.

use crate::bind::BindCollection;
use crate::settings_config;
use crate::spec::SpecItem;
use crate::writer;
use gtk4::prelude::*;
use gtk4::{
    Adjustment, Align, Box as GtkBox, Button, Label, Orientation, PolicyType, Scale,
    ScrolledWindow, Separator, Switch,
};
use serde_json::json;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub type StatusFn = Rc<dyn Fn(String)>;
pub type ReloadFn = Rc<dyn Fn()>;

pub struct OverviewPage {
    pub page: GtkBox,
    pub refresh: Rc<dyn Fn()>,
}

const SPEED_MIN: f64 = 0.25;
const SPEED_MAX: f64 = 3.0;

fn metric(label: &str, value: &str) -> GtkBox {
    let value = Label::builder()
        .label(value)
        .halign(Align::Start)
        .css_classes(["hyprbinds-metric-value"])
        .build();
    let label = Label::builder()
        .label(label)
        .halign(Align::Start)
        .css_classes(["hyprbinds-metric-label"])
        .build();
    let box_ = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .css_classes(["hyprbinds-metric"])
        .build();
    box_.append(&value);
    box_.append(&label);
    box_
}

fn text_block(title: &str, subtitle: &str) -> GtkBox {
    let text = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .valign(Align::Center)
        .build();
    text.append(
        &Label::builder()
            .label(title)
            .halign(Align::Start)
            .css_classes(["hyprbinds-settings-title"])
            .build(),
    );
    if !subtitle.is_empty() {
        text.append(
            &Label::builder()
                .label(subtitle)
                .halign(Align::Start)
                .wrap(true)
                .xalign(0.0)
                .css_classes(["hyprbinds-settings-sub"])
                .build(),
        );
    }
    text
}

fn scalable_baselines(collection: &BindCollection) -> Vec<(SpecItem, f64)> {
    collection
        .animations
        .iter()
        .filter_map(|a| a.get_f64("speed").map(|s| (a.clone(), s)))
        .collect()
}

pub fn build_overview_page(
    state: Rc<RefCell<Option<BindCollection>>>,
    reload: ReloadFn,
    status: StatusFn,
) -> OverviewPage {
    let filling = Rc::new(Cell::new(false));
    let baselines: Rc<RefCell<Vec<(SpecItem, f64)>>> = Rc::new(RefCell::new(Vec::new()));

    let hero_kicker = Label::builder()
        .label("HYPRBINDS")
        .halign(Align::Start)
        .css_classes(["hyprbinds-hero-kicker"])
        .build();
    let hero_title = Label::builder()
        .label("Shape your Hyprland setup")
        .halign(Align::Start)
        .xalign(0.0)
        .wrap(true)
        .css_classes(["hyprbinds-hero-title"])
        .build();
    let hero_sub = Label::builder()
        .label("A focused control center for binds, rules, motion and system configuration.")
        .halign(Align::Start)
        .xalign(0.0)
        .wrap(true)
        .css_classes(["hyprbinds-hero-sub"])
        .build();
    let hero_text = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(5)
        .hexpand(true)
        .build();
    hero_text.append(&hero_kicker);
    hero_text.append(&hero_title);
    hero_text.append(&hero_sub);

    let hero = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(18)
        .css_classes(["hyprbinds-hero"])
        .build();
    hero.append(&hero_text);

    let bind_metric = metric("Binds", "—");
    let var_metric = metric("Variables", "—");
    let rule_metric = metric("Window rules", "—");
    let anim_metric = metric("Animations", "—");
    let metrics = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(10)
        .css_classes(["hyprbinds-metrics"])
        .build();
    metrics.append(&bind_metric);
    metrics.append(&var_metric);
    metrics.append(&rule_metric);
    metrics.append(&anim_metric);

    let master_switch = Switch::builder().valign(Align::Center).halign(Align::End).build();
    let master_row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(16)
        .css_classes(["hyprbinds-settings-row"])
        .build();
    master_row.append(&text_block(
        "Animations",
        "Enable or disable Hyprland motion globally",
    ));
    master_row.append(&master_switch);

    let adj = Adjustment::new(1.0, SPEED_MIN, SPEED_MAX, 0.05, 0.25, 0.0);
    let speed_scale = Scale::new(Orientation::Horizontal, Some(&adj));
    speed_scale.set_draw_value(false);
    speed_scale.set_hexpand(true);
    speed_scale.set_digits(2);
    speed_scale.add_mark(0.5, gtk4::PositionType::Bottom, Some("0.5×"));
    speed_scale.add_mark(1.0, gtk4::PositionType::Bottom, Some("1×"));
    speed_scale.add_mark(2.0, gtk4::PositionType::Bottom, Some("2×"));

    let value_label = Label::builder()
        .label("1.00×")
        .halign(Align::Start)
        .css_classes(["hyprbinds-speed-value"])
        .build();
    let reset_btn = Button::builder().label("Reset").build();
    let apply_btn = Button::builder()
        .label("Apply speed")
        .css_classes(["suggested-action"])
        .build();
    let speed_actions = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .halign(Align::End)
        .build();
    speed_actions.append(&reset_btn);
    speed_actions.append(&apply_btn);

    let speed_header = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .build();
    speed_header.append(&text_block(
        "Animation speed",
        "Scale every bezier animation leaf from one control",
    ));
    speed_header.append(&value_label);

    let speed_panel = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .css_classes(["hyprbinds-dashboard-panel"])
        .build();
    speed_panel.append(&speed_header);
    speed_panel.append(&speed_scale);
    speed_panel.append(&speed_actions);

    let summary_label = Label::builder()
        .label("Load a config to inspect animation leaves.")
        .halign(Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["hyprbinds-dashboard-note"])
        .build();

    let motion_panel = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .css_classes(["hyprbinds-dashboard-panel"])
        .build();
    motion_panel.append(&master_row);
    motion_panel.append(&Separator::new(Orientation::Horizontal));
    motion_panel.append(&summary_label);

    let controls = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .homogeneous(true)
        .build();
    controls.append(&motion_panel);
    controls.append(&speed_panel);

    let content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(14)
        .margin_top(2)
        .margin_bottom(12)
        .css_classes(["hyprbinds-dashboard"])
        .build();
    content.append(&hero);
    content.append(&metrics);
    content.append(&controls);

    let scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .child(&content)
        .build();

    let page = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .vexpand(true)
        .hexpand(true)
        .build();
    page.append(&scroll);

    speed_scale.connect_value_changed({
        let value_label = value_label.clone();
        move |s| value_label.set_text(&format!("{:.2}×", s.value()))
    });

    reset_btn.connect_clicked({
        let speed_scale = speed_scale.clone();
        move |_| speed_scale.set_value(1.0)
    });

    apply_btn.connect_clicked({
        let baselines = Rc::clone(&baselines);
        let speed_scale = speed_scale.clone();
        let reload = Rc::clone(&reload);
        let status = Rc::clone(&status);
        move |_| {
            let bases = baselines.borrow();
            if bases.is_empty() {
                status("No bezier animation leaves to scale.".into());
                return;
            }
            let speed_mult = speed_scale.value().max(SPEED_MIN);
            let duration_factor = 1.0 / speed_mult;
            let items: Vec<SpecItem> = bases.iter().map(|(item, _)| item.clone()).collect();
            let base_speeds: Vec<f64> = bases.iter().map(|(_, s)| *s).collect();
            drop(bases);
            match writer::scale_animation_speeds(&items, &base_speeds, duration_factor) {
                Ok((count, _)) => {
                    status(format!("Scaled {count} animation leaf(es) to {speed_mult:.2}× speed."));
                    reload();
                }
                Err(e) => status(format!("Animation speed change failed: {e}")),
            }
        }
    });

    master_switch.connect_active_notify({
        let filling = Rc::clone(&filling);
        let state = Rc::clone(&state);
        let reload = Rc::clone(&reload);
        let status = Rc::clone(&status);
        move |sw| {
            if filling.get() {
                return;
            }
            let path = state.borrow().as_ref().map(|c| c.config_path.clone());
            let Some(path) = path else {
                status("Load a config first.".into());
                return;
            };
            let mut merged = state
                .borrow()
                .as_ref()
                .map(|c| c.config_merged.clone())
                .unwrap_or_else(|| json!({}));
            if !merged.is_object() {
                merged = json!({});
            }
            settings_config::set_path(&mut merged, &["animations", "enabled"], json!(sw.is_active()));
            match writer::save_config_override(&path, &merged) {
                Ok(_) => {
                    if let Some(c) = state.borrow_mut().as_mut() {
                        c.config_merged = merged;
                    }
                    status(format!("Animations {}.", if sw.is_active() { "enabled" } else { "disabled" }));
                    reload();
                }
                Err(e) => status(format!("Could not update animations: {e}")),
            }
        }
    });

    let refresh: Rc<dyn Fn()> = {
        let state = Rc::clone(&state);
        let baselines = Rc::clone(&baselines);
        let filling = Rc::clone(&filling);
        let master_switch = master_switch.clone();
        let speed_scale = speed_scale.clone();
        let summary_label = summary_label.clone();
        let apply_btn = apply_btn.clone();
        let reset_btn = reset_btn.clone();
        let bind_metric = bind_metric.clone();
        let var_metric = var_metric.clone();
        let rule_metric = rule_metric.clone();
        let anim_metric = anim_metric.clone();
        Rc::new(move || {
            let borrowed = state.borrow();
            let Some(collection) = borrowed.as_ref() else {
                baselines.borrow_mut().clear();
                summary_label.set_text("Load a config to inspect animation leaves.");
                speed_scale.set_sensitive(false);
                apply_btn.set_sensitive(false);
                reset_btn.set_sensitive(false);
                master_switch.set_sensitive(false);
                return;
            };

            if let Some(label) = bind_metric.first_child().and_then(|w| w.downcast::<Label>().ok()) {
                label.set_text(&collection.binds.len().to_string());
            }
            if let Some(label) = var_metric.first_child().and_then(|w| w.downcast::<Label>().ok()) {
                label.set_text(&collection.variables.len().to_string());
            }
            if let Some(label) = rule_metric.first_child().and_then(|w| w.downcast::<Label>().ok()) {
                label.set_text(&collection.window_rules.len().to_string());
            }
            if let Some(label) = anim_metric.first_child().and_then(|w| w.downcast::<Label>().ok()) {
                label.set_text(&collection.animations.len().to_string());
            }

            filling.set(true);
            master_switch.set_active(settings_config::get_bool(
                &collection.config_merged,
                &["animations", "enabled"],
                true,
            ));
            master_switch.set_sensitive(true);
            filling.set(false);

            let bases = scalable_baselines(collection);
            let count = bases.len();
            let range = if count == 0 {
                None
            } else {
                let min = bases.iter().map(|(_, s)| *s).fold(f64::INFINITY, f64::min);
                let max = bases.iter().map(|(_, s)| *s).fold(f64::NEG_INFINITY, f64::max);
                Some((min, max))
            };
            *baselines.borrow_mut() = bases;
            speed_scale.set_value(1.0);
            let has_any = count > 0;
            speed_scale.set_sensitive(has_any);
            apply_btn.set_sensitive(has_any);
            reset_btn.set_sensitive(has_any);

            let total = collection.animations.len();
            match range {
                Some((min, max)) if (max - min).abs() < f64::EPSILON => summary_label.set_text(
                    &format!("{count} of {total} animation leaves use speed {min} ds."),
                ),
                Some((min, max)) => summary_label.set_text(&format!(
                    "{count} of {total} leaves are speed-scalable ({min}–{max} ds)."
                )),
                None => summary_label.set_text("No bezier animation leaves found; springs ignore speed."),
            }
        })
    };

    refresh();
    OverviewPage { page, refresh }
}
