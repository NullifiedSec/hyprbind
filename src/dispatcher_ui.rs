use crate::dispatchers::{
    self, by_category, find, generate, parse, Category, DispatcherDef, FieldDef, FieldKind,
};
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, CheckButton, DropDown, Entry, Expander, Label, Orientation, SpinButton,
    StringList,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const WORKSPACE_PRESETS: &[&str] = &[
    "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "e+1", "e-1", "m+1", "m-1", "previous",
    "empty", "special", "special:magic", "custom…",
];

const DIRECTIONS: &[&str] = &["left", "right", "up", "down", "l", "r", "u", "d"];
const ACTIONS: &[&str] = &["toggle", "enable", "disable", "on", "off", "set", "unset"];

#[derive(Clone)]
pub struct ActionBuilder {
    root: GtkBox,
    raw_entry: Entry,
    preview: Label,
    options: GtkBox,
    category: DropDown,
    dispatcher: DropDown,
    state: Rc<RefCell<BuilderState>>,
    suppress: Rc<RefCell<bool>>,
}

struct BuilderState {
    dispatcher_id: String,
    values: HashMap<String, String>,
    field_widgets: Vec<FieldWidget>,
}

enum FieldWidget {
    Text {
        key: String,
        entry: Entry,
    },
    Number {
        key: String,
        spin: SpinButton,
    },
    Choice {
        key: String,
        dropdown: DropDown,
        options: Vec<String>,
    },
    Bool {
        key: String,
        check: CheckButton,
    },
    Workspace {
        key: String,
        preset: DropDown,
        spin: SpinButton,
        custom: Entry,
    },
}

impl ActionBuilder {
    pub fn new(initial_action: &str) -> Self {
        let root = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(10)
            .css_classes(["hyprbinds-action-builder"])
            .build();

        let category_labels: Vec<&str> = Category::all().iter().map(|c| c.label()).collect();
        let category = DropDown::from_strings(&category_labels);

        let dispatcher = DropDown::from_strings(&["Run command"]);
        let options = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(8)
            .css_classes(["hyprbinds-action-options"])
            .build();

        // Kept for sync plumbing; shown inside Advanced only (avoids duplicate preview).
        let preview = Label::builder()
            .halign(gtk4::Align::Start)
            .wrap(true)
            .selectable(true)
            .css_classes(["monospace", "dim-label", "caption", "hyprbinds-action-preview"])
            .build();

        let raw_entry = Entry::builder()
            .text(initial_action)
            .hexpand(true)
            .placeholder_text("hl.dsp…")
            .css_classes(["monospace"])
            .build();

        let parsed = parse(initial_action);
        let initial_id = parsed
            .as_ref()
            .map(|p| p.dispatcher_id.clone())
            .unwrap_or_else(|| "exec_cmd".into());
        let initial_values = parsed
            .as_ref()
            .map(|p| p.values.clone())
            .unwrap_or_default();
        let initial_cat = find(&initial_id)
            .map(|d| d.category)
            .unwrap_or(Category::General);

        if let Some(idx) = Category::all().iter().position(|c| *c == initial_cat) {
            category.set_selected(idx as u32);
        }

        let state = Rc::new(RefCell::new(BuilderState {
            dispatcher_id: initial_id,
            values: initial_values,
            field_widgets: Vec::new(),
        }));
        let suppress = Rc::new(RefCell::new(false));

        let picker_row = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .homogeneous(true)
            .build();
        picker_row.append(&labeled("Category", category.clone().upcast()));
        picker_row.append(&labeled("Dispatcher", dispatcher.clone().upcast()));
        root.append(&picker_row);
        root.append(&options);

        let advanced_body = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(6)
            .margin_top(2)
            .build();
        advanced_body.append(&preview);
        advanced_body.append(&raw_entry);
        let advanced = Expander::builder()
            .label("Advanced · raw Lua")
            .expanded(false)
            .child(&advanced_body)
            .build();
        root.append(&advanced);

        let builder = Self {
            root,
            raw_entry: raw_entry.clone(),
            preview: preview.clone(),
            options: options.clone(),
            category: category.clone(),
            dispatcher: dispatcher.clone(),
            state: Rc::clone(&state),
            suppress: Rc::clone(&suppress),
        };

        builder.refill_dispatchers();
        builder.rebuild_options();
        builder.sync_outputs();

        category.connect_selected_notify({
            let this_state = Rc::clone(&state);
            let this_suppress = Rc::clone(&suppress);
            let dispatcher = dispatcher.clone();
            let options = options.clone();
            let preview = preview.clone();
            let raw_entry = raw_entry.clone();
            let category = category.clone();
            move |_| {
                if *this_suppress.borrow() {
                    return;
                }
                refill_dispatcher_dropdown(&category, &dispatcher, &this_state);
                // select first dispatcher in category
                if let Some(def) = selected_dispatcher(&category, &dispatcher) {
                    this_state.borrow_mut().dispatcher_id = def.id.to_string();
                    this_state.borrow_mut().values = default_values(def);
                }
                rebuild_options_into(&options, &this_state, &this_suppress, &preview, &raw_entry);
                sync_outputs_into(&this_state, &preview, &raw_entry, &this_suppress);
            }
        });

        dispatcher.connect_selected_notify({
            let this_state = Rc::clone(&state);
            let this_suppress = Rc::clone(&suppress);
            let category = category.clone();
            let dispatcher = dispatcher.clone();
            let options = options.clone();
            let preview = preview.clone();
            let raw_entry = raw_entry.clone();
            move |_| {
                if *this_suppress.borrow() {
                    return;
                }
                if let Some(def) = selected_dispatcher(&category, &dispatcher) {
                    let mut s = this_state.borrow_mut();
                    if s.dispatcher_id != def.id {
                        s.dispatcher_id = def.id.to_string();
                        s.values = default_values(def);
                    }
                }
                rebuild_options_into(&options, &this_state, &this_suppress, &preview, &raw_entry);
                sync_outputs_into(&this_state, &preview, &raw_entry, &this_suppress);
            }
        });

        raw_entry.connect_changed({
            let this_state = Rc::clone(&state);
            let this_suppress = Rc::clone(&suppress);
            let category = category.clone();
            let dispatcher = dispatcher.clone();
            let options = options.clone();
            let preview = preview.clone();
            let raw_entry = raw_entry.clone();
            move |entry| {
                if *this_suppress.borrow() {
                    return;
                }
                let text = entry.text().to_string();
                if let Some(parsed) = parse(&text) {
                    *this_suppress.borrow_mut() = true;
                    let def = find(&parsed.dispatcher_id);
                    if let Some(def) = def {
                        if let Some(cat_idx) =
                            Category::all().iter().position(|c| *c == def.category)
                        {
                            category.set_selected(cat_idx as u32);
                        }
                        refill_dispatcher_dropdown(&category, &dispatcher, &this_state);
                        select_dispatcher(&dispatcher, def.id);
                        this_state.borrow_mut().dispatcher_id = def.id.to_string();
                        this_state.borrow_mut().values = parsed.values;
                    }
                    *this_suppress.borrow_mut() = false;
                    rebuild_options_into(&options, &this_state, &this_suppress, &preview, &raw_entry);
                    preview.set_text(&text);
                } else {
                    preview.set_text(&text);
                }
            }
        });

        builder
    }

    pub fn widget(&self) -> &GtkBox {
        &self.root
    }

    pub fn action_text(&self) -> String {
        self.raw_entry.text().to_string()
    }

    /// Fires when the composed action string changes (builder fields or raw entry).
    pub fn connect_changed<F: Fn() + 'static>(&self, f: F) {
        self.raw_entry.connect_changed(move |_| f());
    }

    fn refill_dispatchers(&self) {
        refill_dispatcher_dropdown(&self.category, &self.dispatcher, &self.state);
        let id = self.state.borrow().dispatcher_id.clone();
        select_dispatcher(&self.dispatcher, &id);
    }

    fn rebuild_options(&self) {
        rebuild_options_into(
            &self.options,
            &self.state,
            &self.suppress,
            &self.preview,
            &self.raw_entry,
        );
    }

    fn sync_outputs(&self) {
        sync_outputs_into(&self.state, &self.preview, &self.raw_entry, &self.suppress);
    }
}

fn labeled(title: &str, child: gtk4::Widget) -> GtkBox {
    let box_ = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .build();
    box_.append(
        &Label::builder()
            .label(title)
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-field-label"])
            .build(),
    );
    box_.append(&child);
    box_
}

fn selected_category(dropdown: &DropDown) -> Category {
    Category::all()
        .get(dropdown.selected() as usize)
        .copied()
        .unwrap_or(Category::General)
}

fn selected_dispatcher<'a>(
    category: &DropDown,
    dispatcher: &DropDown,
) -> Option<&'a DispatcherDef> {
    let cat = selected_category(category);
    let list = by_category(cat);
    list.get(dispatcher.selected() as usize).copied()
}

fn refill_dispatcher_dropdown(
    category: &DropDown,
    dispatcher: &DropDown,
    state: &Rc<RefCell<BuilderState>>,
) {
    let cat = selected_category(category);
    let list = by_category(cat);
    let labels: Vec<&str> = list.iter().map(|d| d.label).collect();
    let model = StringList::new(&labels);
    dispatcher.set_model(Some(&model));

    let current = state.borrow().dispatcher_id.clone();
    if let Some(idx) = list.iter().position(|d| d.id == current) {
        dispatcher.set_selected(idx as u32);
    } else if !list.is_empty() {
        dispatcher.set_selected(0);
        state.borrow_mut().dispatcher_id = list[0].id.to_string();
    }
}

fn select_dispatcher(dropdown: &DropDown, id: &str) {
    let Some(model) = dropdown.model() else {
        return;
    };
    let Ok(list) = model.downcast::<StringList>() else {
        return;
    };
    // We need index in current category list
    for cat in Category::all() {
        let defs = by_category(*cat);
        if let Some(idx) = defs.iter().position(|d| d.id == id) {
            // ensure category already set by caller
            if dropdown.selected() != idx as u32 {
                dropdown.set_selected(idx as u32);
            }
            let _ = list;
            return;
        }
    }
}

fn default_values(def: &DispatcherDef) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for field in def.fields {
        match field.kind {
            FieldKind::Number { default, .. } => {
                map.insert(field.key.to_string(), format_num(default));
            }
            FieldKind::Bool { default } => {
                map.insert(field.key.to_string(), default.to_string());
            }
            FieldKind::Choice { options } => {
                if let Some(first) = options.first() {
                    map.insert(field.key.to_string(), (*first).to_string());
                }
            }
            FieldKind::Direction => {
                map.insert(field.key.to_string(), "left".into());
            }
            FieldKind::Action => {
                map.insert(field.key.to_string(), "toggle".into());
            }
            FieldKind::Workspace => {
                map.insert(field.key.to_string(), "1".into());
            }
            FieldKind::Command | FieldKind::Text { .. } => {
                map.insert(field.key.to_string(), String::new());
            }
        }
    }
    map
}

fn rebuild_options_into(
    options: &GtkBox,
    state: &Rc<RefCell<BuilderState>>,
    suppress: &Rc<RefCell<bool>>,
    preview: &Label,
    raw_entry: &Entry,
) {
    while let Some(child) = options.first_child() {
        options.remove(&child);
    }

    let def = {
        let id = state.borrow().dispatcher_id.clone();
        find(&id).or_else(|| dispatchers::catalog().first())
    };
    let Some(def) = def else {
        return;
    };

    if def.fields.is_empty() {
        options.append(
            &Label::builder()
                .label("No options for this action")
                .halign(gtk4::Align::Start)
                .css_classes(["dim-label", "caption"])
                .build(),
        );
        state.borrow_mut().field_widgets = Vec::new();
        return;
    }

    let mut widgets = Vec::new();
    for field in def.fields {
        let row = build_field_row(field, &state.borrow().values, {
            let state = Rc::clone(state);
            let suppress = Rc::clone(suppress);
            let preview = preview.clone();
            let raw_entry = raw_entry.clone();
            move || {
                collect_values_from_state(&state);
                sync_outputs_into(&state, &preview, &raw_entry, &suppress);
            }
        });
        options.append(&row.0);
        widgets.push(row.1);
    }
    state.borrow_mut().field_widgets = widgets;
}

fn build_field_row<F>(
    field: &FieldDef,
    values: &HashMap<String, String>,
    on_change: F,
) -> (GtkBox, FieldWidget)
where
    F: Fn() + Clone + 'static,
{
    let row = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .css_classes(["hyprbinds-option-field"])
        .build();

    // Bool uses the check label itself — skip the duplicate caption.
    let is_bool = matches!(field.kind, FieldKind::Bool { .. });
    if !is_bool {
        row.append(
            &Label::builder()
                .label(field.label)
                .halign(gtk4::Align::Start)
                .css_classes(["hyprbinds-field-label"])
                .build(),
        );
    }

    let current = values.get(field.key).cloned().unwrap_or_default();

    let widget = match field.kind {
        FieldKind::Text { .. } | FieldKind::Command => {
            let entry = Entry::builder()
                .text(&current)
                .placeholder_text(placeholder_for(field))
                .hexpand(true)
                .build();
            entry.connect_changed({
                let on_change = on_change.clone();
                move |_| on_change()
            });
            row.append(&entry);
            FieldWidget::Text {
                key: field.key.to_string(),
                entry,
            }
        }
        FieldKind::Number {
            min, max, step, ..
        } => {
            let spin = SpinButton::with_range(min, max, step);
            let val = current.parse::<f64>().unwrap_or(min);
            spin.set_value(val);
            spin.connect_value_changed({
                let on_change = on_change.clone();
                move |_| on_change()
            });
            row.append(&spin);
            FieldWidget::Number {
                key: field.key.to_string(),
                spin,
            }
        }
        FieldKind::Choice { options } => {
            let dropdown = DropDown::from_strings(options);
            if let Some(idx) = options.iter().position(|o| *o == current) {
                dropdown.set_selected(idx as u32);
            }
            dropdown.connect_selected_notify({
                let on_change = on_change.clone();
                move |_| on_change()
            });
            row.append(&dropdown);
            FieldWidget::Choice {
                key: field.key.to_string(),
                dropdown,
                options: options.iter().map(|s| (*s).to_string()).collect(),
            }
        }
        FieldKind::Bool { default } => {
            let active = current.parse::<bool>().unwrap_or(default);
            let check = CheckButton::builder()
                .label(field.label)
                .active(active)
                .build();
            check.connect_toggled({
                let on_change = on_change.clone();
                move |_| on_change()
            });
            row.append(&check);
            FieldWidget::Bool {
                key: field.key.to_string(),
                check,
            }
        }
        FieldKind::Direction => {
            let dropdown = DropDown::from_strings(DIRECTIONS);
            if let Some(idx) = DIRECTIONS.iter().position(|o| *o == current) {
                dropdown.set_selected(idx as u32);
            }
            dropdown.connect_selected_notify({
                let on_change = on_change.clone();
                move |_| on_change()
            });
            row.append(&dropdown);
            FieldWidget::Choice {
                key: field.key.to_string(),
                dropdown,
                options: DIRECTIONS.iter().map(|s| (*s).to_string()).collect(),
            }
        }
        FieldKind::Action => {
            let dropdown = DropDown::from_strings(ACTIONS);
            if let Some(idx) = ACTIONS.iter().position(|o| *o == current) {
                dropdown.set_selected(idx as u32);
            }
            dropdown.connect_selected_notify({
                let on_change = on_change.clone();
                move |_| on_change()
            });
            row.append(&dropdown);
            FieldWidget::Choice {
                key: field.key.to_string(),
                dropdown,
                options: ACTIONS.iter().map(|s| (*s).to_string()).collect(),
            }
        }
        FieldKind::Workspace => {
            let controls = GtkBox::new(Orientation::Horizontal, 8);
            let preset = DropDown::from_strings(WORKSPACE_PRESETS);
            let spin = SpinButton::with_range(1.0, 50.0, 1.0);
            let custom = Entry::builder()
                .placeholder_text("custom selector")
                .hexpand(true)
                .build();

            if let Ok(n) = current.parse::<i64>() {
                spin.set_value(n as f64);
                if let Some(idx) = WORKSPACE_PRESETS.iter().position(|p| *p == current) {
                    preset.set_selected(idx as u32);
                } else {
                    // numeric not in first 10 maybe
                    preset.set_selected(0);
                    spin.set_value(n as f64);
                }
                custom.set_sensitive(false);
            } else if let Some(idx) = WORKSPACE_PRESETS.iter().position(|p| *p == current) {
                preset.set_selected(idx as u32);
                custom.set_sensitive(false);
            } else {
                // custom
                preset.set_selected((WORKSPACE_PRESETS.len() - 1) as u32);
                custom.set_text(&current);
                custom.set_sensitive(true);
            }

            let sync = {
                let preset = preset.clone();
                let spin = spin.clone();
                let custom = custom.clone();
                let on_change = on_change.clone();
                Rc::new(move || {
                    let idx = preset.selected() as usize;
                    if idx + 1 == WORKSPACE_PRESETS.len() {
                        custom.set_sensitive(true);
                        spin.set_sensitive(false);
                    } else if WORKSPACE_PRESETS[idx].parse::<i64>().is_ok() {
                        custom.set_sensitive(false);
                        spin.set_sensitive(true);
                        if let Ok(n) = WORKSPACE_PRESETS[idx].parse::<f64>() {
                            spin.set_value(n);
                        }
                    } else {
                        custom.set_sensitive(false);
                        spin.set_sensitive(false);
                    }
                    on_change();
                })
            };

            preset.connect_selected_notify({
                let sync = Rc::clone(&sync);
                move |_| sync()
            });
            spin.connect_value_changed({
                let on_change = on_change.clone();
                move |_| on_change()
            });
            custom.connect_changed({
                let on_change = on_change.clone();
                move |_| on_change()
            });

            controls.append(&preset);
            controls.append(&spin);
            controls.append(&custom);
            row.append(&controls);
            FieldWidget::Workspace {
                key: field.key.to_string(),
                preset,
                spin,
                custom,
            }
        }
    };

    (row, widget)
}

fn placeholder_for(field: &FieldDef) -> &str {
    match field.kind {
        FieldKind::Text { placeholder } => placeholder,
        FieldKind::Command => "kitty  or  {{terminal}}",
        _ => "",
    }
}

fn collect_values_from_state(state: &Rc<RefCell<BuilderState>>) {
    let mut s = state.borrow_mut();
    let mut values = HashMap::new();
    for widget in &s.field_widgets {
        match widget {
            FieldWidget::Text { key, entry } => {
                values.insert(key.clone(), entry.text().to_string());
            }
            FieldWidget::Number { key, spin } => {
                values.insert(key.clone(), format_num(spin.value()));
            }
            FieldWidget::Choice {
                key,
                dropdown,
                options,
            } => {
                let idx = dropdown.selected() as usize;
                if let Some(v) = options.get(idx) {
                    values.insert(key.clone(), v.clone());
                }
            }
            FieldWidget::Bool { key, check } => {
                values.insert(key.clone(), check.is_active().to_string());
            }
            FieldWidget::Workspace {
                key,
                preset,
                spin,
                custom,
            } => {
                let idx = preset.selected() as usize;
                let value = if idx + 1 == WORKSPACE_PRESETS.len() {
                    custom.text().to_string()
                } else if WORKSPACE_PRESETS[idx].parse::<i64>().is_ok() {
                    format_num(spin.value())
                } else {
                    WORKSPACE_PRESETS[idx].to_string()
                };
                values.insert(key.clone(), value);
            }
        }
    }
    s.values = values;
}

fn sync_outputs_into(
    state: &Rc<RefCell<BuilderState>>,
    preview: &Label,
    raw_entry: &Entry,
    suppress: &Rc<RefCell<bool>>,
) {
    collect_values_from_state(state);
    let (id, values) = {
        let s = state.borrow();
        (s.dispatcher_id.clone(), s.values.clone())
    };
    let Some(def) = find(&id) else {
        return;
    };
    let action = generate(def, &values);
    preview.set_text(&action);
    *suppress.borrow_mut() = true;
    raw_entry.set_text(&action);
    *suppress.borrow_mut() = false;
}

fn format_num(v: f64) -> String {
    if (v - v.round()).abs() < f64::EPSILON {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}
