//! Overview — quick global controls that keep the whole desktop consistent.
//!
//! The centrepiece is a single animation-speed slider that scales every bezier
//! animation leaf at once, so motion stays uniform without editing each leaf.

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
    /// Re-read widgets from the shared collection (call after a reload).
    pub refresh: Rc<dyn Fn()>,
}

const SPEED_MIN: f64 = 0.25;
const SPEED_MAX: f64 = 3.0;

fn card(title: &str) -> GtkBox {
    let card = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .css_classes(["hyprbinds-settings-card"])
        .build();
    card.append(
        &Label::builder()
            .label(title)
            .halign(Align::Start)
            .css_classes(["hyprbinds-settings-card-title"])
            .build(),
    );
    card
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
                .css_classes(["hyprbinds-settings-sub", "dim-label"])
                .build(),
        );
    }
    text
}

/// Collect the animation leaves that carry a numeric (bezier) speed.
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

    // --- Animations card ---
    let anims_card = card("Animations");

    let master_switch = Switch::builder().valign(Align::Center).halign(Align::End).build();
    let master_row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(16)
        .css_classes(["hyprbinds-settings-row"])
        .build();
    master_row.append(&text_block(
        "Enable animations",
        "Master switch for all Hyprland motion",
    ));
    master_row.append(&master_switch);
    anims_card.append(&master_row);
    anims_card.append(&Separator::new(Orientation::Horizontal));

    // Speed slider block (title + slider + value + buttons).
    let speed_block = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .css_classes(["hyprbinds-settings-row"])
        .build();
    speed_block.append(&text_block(
        "Global animation speed",
        "Scales every bezier leaf at once — right is faster (shorter durations), left is slower. Springs ignore speed.",
    ));

    let adj = Adjustment::new(1.0, SPEED_MIN, SPEED_MAX, 0.05, 0.25, 0.0);
    let speed_scale = Scale::new(Orientation::Horizontal, Some(&adj));
    speed_scale.set_draw_value(false);
    speed_scale.set_hexpand(true);
    speed_scale.set_digits(2);
    speed_scale.add_mark(0.5, gtk4::PositionType::Bottom, Some("0.5×"));
    speed_scale.add_mark(1.0, gtk4::PositionType::Bottom, Some("1×"));
    speed_scale.add_mark(2.0, gtk4::PositionType::Bottom, Some("2×"));
    speed_block.append(&speed_scale);

    let value_label = Label::builder()
        .label("1.00× speed")
        .halign(Align::Start)
        .css_classes(["hyprbinds-settings-sub", "dim-label"])
        .build();
    speed_block.append(&value_label);

    let reset_btn = Button::builder().label("Reset to 1×").build();
    let apply_btn = Button::builder()
        .label("Apply to all leaves")
        .css_classes(["suggested-action"])
        .build();
    let btn_row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .halign(Align::End)
        .build();
    btn_row.append(&reset_btn);
    btn_row.append(&apply_btn);
    speed_block.append(&btn_row);
    anims_card.append(&speed_block);

    let summary_label = Label::builder()
        .label("")
        .halign(Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    summary_label.set_margin_top(4);
    anims_card.append(&summary_label);

    // --- Body / page chrome ---
    let header = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .css_classes(["hyprbinds-page-header"])
        .build();
    header.append(
        &Label::builder()
            .label("Overview")
            .halign(Align::Start)
            .css_classes(["hyprbinds-page-title"])
            .build(),
    );
    header.append(
        &Label::builder()
            .label("Simple global controls for a consistent desktop. Fine-tune individual leaves under Animations.")
            .halign(Align::Start)
            .wrap(true)
            .xalign(0.0)
            .css_classes(["hyprbinds-page-hint"])
            .build(),
    );

    let form = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(16)
        .margin_top(4)
        .margin_bottom(8)
        .build();
    form.append(&header);
    form.append(&anims_card);

    let scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .child(&form)
        .build();

    let page = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .vexpand(true)
        .hexpand(true)
        .build();
    page.append(&scroll);

    // --- Behaviour ---
    speed_scale.connect_value_changed({
        let value_label = value_label.clone();
        move |s| value_label.set_text(&format!("{:.2}× speed", s.value()))
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
                Ok((count, _files)) => {
                    status(format!(
                        "Scaled {count} animation leaf(es) to {speed_mult:.2}× speed."
                    ));
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
                    status(format!(
                        "Animations {}.",
                        if sw.is_active() { "enabled" } else { "disabled" }
                    ));
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
        Rc::new(move || {
            let borrowed = state.borrow();
            let Some(collection) = borrowed.as_ref() else {
                baselines.borrow_mut().clear();
                summary_label.set_text("Load a config to adjust global settings.");
                speed_scale.set_sensitive(false);
                apply_btn.set_sensitive(false);
                reset_btn.set_sensitive(false);
                master_switch.set_sensitive(false);
                return;
            };

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
                let min = bases
                    .iter()
                    .map(|(_, s)| *s)
                    .fold(f64::INFINITY, f64::min);
                let max = bases
                    .iter()
                    .map(|(_, s)| *s)
                    .fold(f64::NEG_INFINITY, f64::max);
                Some((min, max))
            };
            *baselines.borrow_mut() = bases;

            // Loaded state is the new 1.00× reference.
            speed_scale.set_value(1.0);

            let has_any = count > 0;
            speed_scale.set_sensitive(has_any);
            apply_btn.set_sensitive(has_any);
            reset_btn.set_sensitive(has_any);

            let total = collection.animations.len();
            match range {
                Some((min, max)) if (max - min).abs() < f64::EPSILON => summary_label.set_text(
                    &format!("{count} of {total} leaves use speed {min} ds. Move the slider, then Apply."),
                ),
                Some((min, max)) => summary_label.set_text(&format!(
                    "{count} of {total} leaves are speed-scalable ({min}–{max} ds). Move the slider, then Apply."
                )),
                None => summary_label.set_text(
                    "No bezier animation leaves found — springs ignore speed, so there is nothing to scale.",
                ),
            }
        })
    };

    refresh();

    OverviewPage { page, refresh }
}
