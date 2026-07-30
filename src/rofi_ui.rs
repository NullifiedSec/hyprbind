//! Rofi Studio page — preview, config, style, and theme editors.

use crate::dialog;
use crate::rofi;
use crate::rofi_model::{RofiModel, KNOWN_MODES, LOCATION_LABELS, MATCHING_METHODS};
use crate::rofi_theme::{self, ThemeTokens, LAYOUT_PRESET_IDS, LAYOUT_PRESET_LABELS, ORIENTATIONS};
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, CssProvider, DropDown, Entry, Label, Notebook,
    Orientation, PolicyType, ScrolledWindow, SpinButton, StringList,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct RofiPage {
    pub page: GtkBox,
}

struct StudioState {
    model: RofiModel,
}

pub fn build_rofi_page(status: Rc<dyn Fn(String)>) -> RofiPage {
    let model = rofi::load().unwrap_or_else(|e| {
        let cfg = rofi::discover_config_path()
            .unwrap_or_else(|| std::path::PathBuf::from("~/.config/rofi/config.rasi"));
        let theme = rofi::theme_path()
            .unwrap_or_else(|| std::path::PathBuf::from("~/.config/rofi/hyprbinds-theme.rasi"));
        let mut m = RofiModel::empty(cfg, theme);
        m.notes.push(format!("Load warning: {e}"));
        m
    });

    let state = Rc::new(RefCell::new(StudioState { model }));

    let path_label = Label::builder()
        .halign(gtk4::Align::Start)
        .ellipsize(gtk4::pango::EllipsizeMode::Middle)
        .css_classes(["dim-label", "caption"])
        .build();
    let proc_label = Label::builder()
        .halign(gtk4::Align::Start)
        .css_classes(["caption"])
        .build();
    let note_label = Label::builder()
        .halign(gtk4::Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();

    let preview = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .hexpand(true)
        .css_classes(["hyprbinds-rofi-preview"])
        .build();

    let apply_btn = Button::builder()
        .label("Apply")
        .css_classes(["suggested-action"])
        .build();
    let demo_btn = Button::builder().label("Test Rofi").build();
    let restore_btn = Button::builder().label("Restore").build();
    let refresh_btn = Button::builder().label("Re-read").build();

    let toolbar = GtkBox::new(Orientation::Horizontal, 8);
    toolbar.append(&apply_btn);
    toolbar.append(&demo_btn);
    toolbar.append(&restore_btn);
    toolbar.append(&refresh_btn);

    let config_body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .build();
    let layout_body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .build();
    let style_body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .build();
    let themes_body = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .build();

    let notebook = Notebook::new();
    notebook.append_page(&wrap_scroll(&config_body), Some(&Label::new(Some("Config"))));
    notebook.append_page(&wrap_scroll(&layout_body), Some(&Label::new(Some("Layout"))));
    notebook.append_page(&wrap_scroll(&style_body), Some(&Label::new(Some("Style"))));
    notebook.append_page(&wrap_scroll(&themes_body), Some(&Label::new(Some("Themes"))));
    notebook.set_vexpand(true);
    notebook.set_hexpand(true);

    let refresh_ui: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));
    let refresh_preview: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));
    let refresh_busy = Rc::new(Cell::new(false));

    let do_preview = {
        let state = Rc::clone(&state);
        let preview = preview.clone();
        let path_label = path_label.clone();
        let proc_label = proc_label.clone();
        let note_label = note_label.clone();
        Rc::new(move || {
            let (cfg, theme, notes) = {
                let st = state.borrow();
                (
                    st.model.config_path.display().to_string(),
                    st.model.theme_path.display().to_string(),
                    st.model.notes.clone(),
                )
            };
            path_label.set_label(&format!("{cfg}  ·  {theme}"));
            proc_label.set_label(&format!("Rofi: {}", rofi::status_label()));
            note_label.set_label(&if notes.is_empty() {
                String::new()
            } else {
                notes.join(" · ")
            });
            rebuild_preview(&preview, &state);
        }) as Rc<dyn Fn()>
    };
    *refresh_preview.borrow_mut() = Some(Rc::clone(&do_preview));

    let do_refresh = {
        let state = Rc::clone(&state);
        let config_body = config_body.clone();
        let layout_body = layout_body.clone();
        let style_body = style_body.clone();
        let themes_body = themes_body.clone();
        let refresh_ui = Rc::clone(&refresh_ui);
        let refresh_preview = Rc::clone(&refresh_preview);
        let do_preview = Rc::clone(&do_preview);
        let refresh_busy = Rc::clone(&refresh_busy);

        Rc::new(move || {
            if refresh_busy.get() {
                return;
            }
            refresh_busy.set(true);
            do_preview();
            rebuild_config_tab(&config_body, &state, &refresh_ui, &refresh_preview);
            rebuild_layout_tab(&layout_body, &state, &refresh_ui, &refresh_preview);
            rebuild_style_tab(&style_body, &state, &refresh_preview);
            rebuild_themes_tab(&themes_body, &state, &refresh_ui);
            refresh_busy.set(false);
        }) as Rc<dyn Fn()>
    };
    *refresh_ui.borrow_mut() = Some(Rc::clone(&do_refresh));
    do_refresh();

    {
        let state = Rc::clone(&state);
        let status = Rc::clone(&status);
        let do_refresh = Rc::clone(&do_refresh);
        apply_btn.connect_clicked(move |_| {
            let model = state.borrow().model.clone();
            match rofi::apply(&model) {
                Ok(msg) => status(msg),
                Err(e) => status(format!("Apply failed: {e}")),
            }
            do_refresh();
        });
    }
    {
        let state = Rc::clone(&state);
        let status = Rc::clone(&status);
        demo_btn.connect_clicked(move |_| {
            let mode = state
                .borrow()
                .model
                .modes
                .first()
                .cloned()
                .unwrap_or_else(|| "drun".into());
            match rofi::launch_demo(&mode) {
                Ok(msg) => status(msg),
                Err(e) => status(e.to_string()),
            }
        });
    }
    {
        let state = Rc::clone(&state);
        let status = Rc::clone(&status);
        let do_refresh = Rc::clone(&do_refresh);
        restore_btn.connect_clicked(move |_| {
            let model = state.borrow().model.clone();
            match rofi::restore_files(&model) {
                Ok(m) => status(m),
                Err(e) => status(e.to_string()),
            }
            match rofi::load() {
                Ok(m) => state.borrow_mut().model = m,
                Err(e) => status(format!("Re-load after restore failed: {e}")),
            }
            do_refresh();
        });
    }
    {
        let state = Rc::clone(&state);
        let status = Rc::clone(&status);
        let do_refresh = Rc::clone(&do_refresh);
        refresh_btn.connect_clicked(move |_| {
            match rofi::load() {
                Ok(m) => {
                    state.borrow_mut().model = m;
                    status("Re-read Rofi config from disk".into());
                }
                Err(e) => status(format!("Load failed: {e}")),
            }
            do_refresh();
        });
    }

    let meta = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(2)
        .build();
    meta.append(&path_label);
    meta.append(&proc_label);
    meta.append(&note_label);

    let content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .vexpand(true)
        .build();
    content.append(&meta);
    content.append(
        &Label::builder()
            .label("Live preview")
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-settings-card-title"])
            .build(),
    );
    content.append(&preview);
    content.append(&notebook);

    let page = dialog::page_shell(
        "Rofi Studio",
        "Edit ~/.config/rofi config, layout, and theme — then Test Rofi to try it live.",
        &toolbar,
        &content,
    );

    RofiPage { page }
}

fn wrap_scroll(child: &impl IsA<gtk4::Widget>) -> ScrolledWindow {
    ScrolledWindow::builder()
        .child(child)
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .hexpand(true)
        .min_content_height(280)
        .build()
}

fn clear_box(b: &GtkBox) {
    while let Some(child) = b.first_child() {
        b.remove(&child);
    }
}

fn call_refresh(refresh_ui: &Rc<RefCell<Option<Rc<dyn Fn()>>>>) {
    let refresh_ui = Rc::clone(refresh_ui);
    gtk4::glib::idle_add_local_once(move || {
        if let Some(f) = refresh_ui.borrow().as_ref().cloned() {
            f();
        }
    });
}

fn call_preview(refresh_preview: &Rc<RefCell<Option<Rc<dyn Fn()>>>>) {
    let refresh_preview = Rc::clone(refresh_preview);
    gtk4::glib::idle_add_local_once(move || {
        if let Some(f) = refresh_preview.borrow().as_ref().cloned() {
            f();
        }
    });
}

fn rebuild_preview(preview: &GtkBox, state: &Rc<RefCell<StudioState>>) {
    clear_box(preview);
    let st = state.borrow();
    let t = &st.model.tokens;

    let window = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .css_classes(["hyprbinds-rofi-window"])
        .halign(gtk4::Align::Center)
        .width_request(360)
        .build();

    let css = format!(
        ".hyprbinds-rofi-window {{
            background-color: {bg};
            border: 2px solid {border};
            border-radius: 12px;
            padding: 12px;
        }}
        .hyprbinds-rofi-entry {{
            background-color: alpha(white, 0.06);
            color: {fg};
            border-radius: 8px;
            padding: 8px 12px;
            font-weight: 600;
        }}
        .hyprbinds-rofi-row {{
            color: {fg};
            border-radius: 8px;
            padding: 6px 12px;
        }}
        .hyprbinds-rofi-row-selected {{
            background-color: {sel_bg};
            color: {sel_fg};
            border-radius: 8px;
            padding: 6px 12px;
            font-weight: 600;
        }}",
        bg = css_color(&t.background),
        border = css_color(&t.border_color),
        fg = css_color(&t.foreground),
        sel_bg = css_color(&t.selected_bg),
        sel_fg = css_color(&t.selected_fg),
    );
    let provider = CssProvider::new();
    provider.load_from_string(&css);
    add_provider_to_widget(&window, &provider);

    let entry = Label::builder()
        .label("Search…")
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .css_classes(["hyprbinds-rofi-entry"])
        .build();
    add_provider_to_widget(&entry, &provider);
    window.append(&entry);

    let rows = [
        ("Terminal", false),
        ("Firefox", true),
        ("Code - OSS", false),
        ("Files", false),
    ];
    for (label, selected) in rows {
        let row = Label::builder()
            .label(label)
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .css_classes([if selected {
                "hyprbinds-rofi-row-selected"
            } else {
                "hyprbinds-rofi-row"
            }])
            .build();
        add_provider_to_widget(&row, &provider);
        window.append(&row);
    }

    let caption = Label::builder()
        .label(format!(
            "{} · {} · {} · {}×{} · {}",
            st.model.font,
            st.model.location_label(),
            st.model.modes_csv(),
            st.model.tokens.columns,
            st.model.tokens.lines,
            st.model.tokens.orientation
        ))
        .halign(gtk4::Align::Center)
        .css_classes(["dim-label", "caption"])
        .build();

    preview.append(&window);
    preview.append(&caption);
}

fn add_provider_to_widget(widget: &impl IsA<gtk4::Widget>, provider: &CssProvider) {
    widget
        .style_context()
        .add_provider(provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
}

fn css_color(value: &str) -> String {
    let v = value.trim();
    if v.starts_with('#') || v.starts_with("rgb") || v.starts_with("rgba") {
        v.to_string()
    } else {
        "#888888".into()
    }
}

fn section_title(text: &str) -> Label {
    Label::builder()
        .label(text)
        .halign(gtk4::Align::Start)
        .css_classes(["hyprbinds-settings-card-title"])
        .build()
}

fn field_row(label: &str, widget: &impl IsA<gtk4::Widget>) -> GtkBox {
    let row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .build();
    row.append(
        &Label::builder()
            .label(label)
            .halign(gtk4::Align::Start)
            .width_chars(16)
            .css_classes(["dim-label"])
            .build(),
    );
    let w = widget.clone().upcast::<gtk4::Widget>();
    w.set_hexpand(true);
    row.append(&w);
    row
}

fn rebuild_config_tab(
    body: &GtkBox,
    state: &Rc<RefCell<StudioState>>,
    refresh_ui: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    refresh_preview: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
) {
    clear_box(body);
    body.append(&section_title("Modes"));
    body.append(
        &Label::builder()
            .label("Enabled Rofi modes (at least one). First enabled mode is used by Test Rofi.")
            .halign(gtk4::Align::Start)
            .wrap(true)
            .xalign(0.0)
            .css_classes(["dim-label", "caption"])
            .build(),
    );

    let modes_box = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .build();
    for mode in KNOWN_MODES {
        let enabled = state.borrow().model.modes.iter().any(|m| m == mode);
        let cb = CheckButton::builder()
            .label(*mode)
            .active(enabled)
            .build();
        cb.connect_toggled({
            let state = Rc::clone(state);
            let refresh_ui = Rc::clone(refresh_ui);
            let mode = (*mode).to_string();
            move |btn| {
                {
                    let mut st = state.borrow_mut();
                    st.model.set_mode_enabled(&mode, btn.is_active());
                    if st.model.modes.is_empty() {
                        st.model.modes.push("drun".into());
                    }
                }
                call_refresh(&refresh_ui);
            }
        });
        modes_box.append(&cb);
    }
    body.append(&modes_box);

    body.append(&section_title("Behavior"));

    let font = Entry::builder()
        .text(&state.borrow().model.font)
        .placeholder_text("JetBrainsMono Nerd Font 12")
        .build();
    font.connect_changed({
        let state = Rc::clone(state);
        let refresh_preview = Rc::clone(refresh_preview);
        move |entry| {
            state.borrow_mut().model.font = entry.text().to_string();
            call_preview(&refresh_preview);
        }
    });
    body.append(&field_row("Font", &font));

    let terminal = Entry::builder()
        .text(&state.borrow().model.terminal)
        .placeholder_text("kitty")
        .build();
    terminal.connect_changed({
        let state = Rc::clone(state);
        move |entry| {
            state.borrow_mut().model.terminal = entry.text().to_string();
        }
    });
    body.append(&field_row("Terminal", &terminal));

    let match_list = StringList::new(MATCHING_METHODS);
    let matching_idx = MATCHING_METHODS
        .iter()
        .position(|m| *m == state.borrow().model.matching.as_str())
        .unwrap_or(3) as u32;
    let matching = DropDown::builder()
        .model(&match_list)
        .selected(matching_idx)
        .build();
    matching.connect_selected_notify({
        let state = Rc::clone(state);
        move |dd| {
            let idx = dd.selected() as usize;
            if let Some(m) = MATCHING_METHODS.get(idx) {
                state.borrow_mut().model.matching = (*m).to_string();
            }
        }
    });
    body.append(&field_row("Matching", &matching));

    let toggles = [
        ("Show icons", state.borrow().model.show_icons, "show_icons"),
        ("Cycle", state.borrow().model.cycle, "cycle"),
        (
            "Sidebar mode",
            state.borrow().model.sidebar_mode,
            "sidebar",
        ),
        (
            "Hover select",
            state.borrow().model.hover_select,
            "hover",
        ),
        (
            "Case sensitive",
            state.borrow().model.case_sensitive,
            "case",
        ),
        (
            "Disable history",
            state.borrow().model.disable_history,
            "history",
        ),
    ];
    for (label, active, key) in toggles {
        let cb = CheckButton::builder().label(label).active(active).build();
        cb.connect_toggled({
            let state = Rc::clone(state);
            move |btn| {
                let mut st = state.borrow_mut();
                match key {
                    "show_icons" => st.model.show_icons = btn.is_active(),
                    "cycle" => st.model.cycle = btn.is_active(),
                    "sidebar" => st.model.sidebar_mode = btn.is_active(),
                    "hover" => st.model.hover_select = btn.is_active(),
                    "case" => st.model.case_sensitive = btn.is_active(),
                    "history" => st.model.disable_history = btn.is_active(),
                    _ => {}
                }
            }
        });
        body.append(&cb);
    }
}

fn rebuild_style_tab(
    body: &GtkBox,
    state: &Rc<RefCell<StudioState>>,
    refresh_preview: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
) {
    clear_box(body);
    body.append(&section_title("Colors"));
    body.append(
        &Label::builder()
            .label("Hex colors written into ~/.config/rofi/hyprbinds-theme.rasi")
            .halign(gtk4::Align::Start)
            .css_classes(["dim-label", "caption"])
            .build(),
    );

    let colors: &[(&str, fn(&mut ThemeTokens) -> &mut String)] = &[
        ("Background", |t| &mut t.background),
        ("Foreground", |t| &mut t.foreground),
        ("Selected bg", |t| &mut t.selected_bg),
        ("Selected fg", |t| &mut t.selected_fg),
        ("Active", |t| &mut t.active),
        ("Urgent", |t| &mut t.urgent),
        ("Border", |t| &mut t.border_color),
    ];

    for (label, accessor) in colors {
        let value = accessor(&mut state.borrow_mut().model.tokens).clone();
        let entry = Entry::builder().text(&value).build();
        let swatch = GtkBox::builder()
            .width_request(28)
            .height_request(28)
            .css_classes(["hyprbinds-rofi-swatch"])
            .build();
        paint_swatch(&swatch, &value);

        entry.connect_changed({
            let state = Rc::clone(state);
            let refresh_preview = Rc::clone(refresh_preview);
            let swatch = swatch.clone();
            let accessor = *accessor;
            move |entry| {
                let v = entry.text().to_string();
                paint_swatch(&swatch, &v);
                *accessor(&mut state.borrow_mut().model.tokens) = v;
                call_preview(&refresh_preview);
            }
        });

        let row = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(10)
            .build();
        row.append(
            &Label::builder()
                .label(*label)
                .width_chars(14)
                .halign(gtk4::Align::Start)
                .css_classes(["dim-label"])
                .build(),
        );
        row.append(&swatch);
        entry.set_hexpand(true);
        row.append(&entry);
        body.append(&row);
    }
}

fn rebuild_layout_tab(
    body: &GtkBox,
    state: &Rc<RefCell<StudioState>>,
    refresh_ui: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    refresh_preview: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
) {
    clear_box(body);

    body.append(&section_title("Presets"));
    body.append(
        &Label::builder()
            .label("One-click layouts. Colors stay on the Style / Themes tabs.")
            .halign(gtk4::Align::Start)
            .wrap(true)
            .xalign(0.0)
            .css_classes(["dim-label", "caption"])
            .build(),
    );

    let preset_list = StringList::new(LAYOUT_PRESET_LABELS);
    let preset_dd = DropDown::builder().model(&preset_list).selected(0).build();
    let apply_preset = Button::builder()
        .label("Apply preset")
        .css_classes(["suggested-action"])
        .build();
    let preset_row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .build();
    preset_dd.set_hexpand(true);
    preset_row.append(&preset_dd);
    preset_row.append(&apply_preset);
    body.append(&preset_row);

    apply_preset.connect_clicked({
        let state = Rc::clone(state);
        let refresh_ui = Rc::clone(refresh_ui);
        let preset_dd = preset_dd.clone();
        move |_| {
            let idx = preset_dd.selected() as usize;
            if let Some(id) = LAYOUT_PRESET_IDS.get(idx) {
                rofi_theme::apply_layout_preset(&mut state.borrow_mut().model.tokens, id);
                if *id == "fullscreen" {
                    state.borrow_mut().model.location = 0;
                    state.borrow_mut().model.xoffset = 0;
                    state.borrow_mut().model.yoffset = 0;
                }
                call_refresh(&refresh_ui);
            }
        }
    });

    body.append(&section_title("Position"));

    let loc_list = StringList::new(LOCATION_LABELS);
    let location = DropDown::builder()
        .model(&loc_list)
        .selected(u32::from(state.borrow().model.location.min(8)))
        .build();
    location.connect_selected_notify({
        let state = Rc::clone(state);
        let refresh_preview = Rc::clone(refresh_preview);
        move |dd| {
            state.borrow_mut().model.location = dd.selected().min(8) as u8;
            call_preview(&refresh_preview);
        }
    });
    body.append(&field_row("Anchor", &location));

    let xoff = SpinButton::with_range(-4000.0, 4000.0, 1.0);
    xoff.set_value(f64::from(state.borrow().model.xoffset));
    xoff.connect_value_changed({
        let state = Rc::clone(state);
        move |spin| {
            state.borrow_mut().model.xoffset = spin.value() as i32;
        }
    });
    body.append(&field_row("X offset", &xoff));

    let yoff = SpinButton::with_range(-4000.0, 4000.0, 1.0);
    yoff.set_value(f64::from(state.borrow().model.yoffset));
    yoff.connect_value_changed({
        let state = Rc::clone(state);
        move |spin| {
            state.borrow_mut().model.yoffset = spin.value() as i32;
        }
    });
    body.append(&field_row("Y offset", &yoff));

    body.append(&section_title("Window & list"));

    let width = Entry::builder()
        .text(&state.borrow().model.tokens.width)
        .placeholder_text("40em")
        .build();
    width.connect_changed({
        let state = Rc::clone(state);
        let refresh_preview = Rc::clone(refresh_preview);
        move |entry| {
            state.borrow_mut().model.tokens.width = entry.text().to_string();
            call_preview(&refresh_preview);
        }
    });
    body.append(&field_row("Width", &width));

    let height = Entry::builder()
        .text(&state.borrow().model.tokens.height)
        .placeholder_text("auto (leave empty)")
        .build();
    height.connect_changed({
        let state = Rc::clone(state);
        move |entry| {
            state.borrow_mut().model.tokens.height = entry.text().to_string();
        }
    });
    body.append(&field_row("Height", &height));

    let lines = SpinButton::with_range(1.0, 40.0, 1.0);
    lines.set_value(f64::from(state.borrow().model.tokens.lines.max(1)));
    lines.connect_value_changed({
        let state = Rc::clone(state);
        let refresh_preview = Rc::clone(refresh_preview);
        move |spin| {
            state.borrow_mut().model.tokens.lines = spin.value().max(1.0) as u32;
            call_preview(&refresh_preview);
        }
    });
    body.append(&field_row("Lines", &lines));

    let columns = SpinButton::with_range(1.0, 12.0, 1.0);
    columns.set_value(f64::from(state.borrow().model.tokens.columns.max(1)));
    columns.connect_value_changed({
        let state = Rc::clone(state);
        let refresh_preview = Rc::clone(refresh_preview);
        move |spin| {
            state.borrow_mut().model.tokens.columns = spin.value().max(1.0) as u32;
            call_preview(&refresh_preview);
        }
    });
    body.append(&field_row("Columns", &columns));

    let orient_idx = ORIENTATIONS
        .iter()
        .position(|o| *o == state.borrow().model.tokens.orientation.as_str())
        .unwrap_or(0) as u32;
    let orient_list = StringList::new(ORIENTATIONS);
    let orientation = DropDown::builder()
        .model(&orient_list)
        .selected(orient_idx)
        .build();
    orientation.connect_selected_notify({
        let state = Rc::clone(state);
        let refresh_preview = Rc::clone(refresh_preview);
        move |dd| {
            let idx = dd.selected() as usize;
            if let Some(o) = ORIENTATIONS.get(idx) {
                state.borrow_mut().model.tokens.orientation = (*o).to_string();
                call_preview(&refresh_preview);
            }
        }
    });
    body.append(&field_row("Orientation", &orientation));

    let icon_size = Entry::builder()
        .text(&state.borrow().model.tokens.icon_size)
        .placeholder_text("1.2em")
        .build();
    icon_size.connect_changed({
        let state = Rc::clone(state);
        move |entry| {
            state.borrow_mut().model.tokens.icon_size = entry.text().to_string();
        }
    });
    body.append(&field_row("Icon size", &icon_size));

    let radius = Entry::builder()
        .text(&state.borrow().model.tokens.border_radius)
        .placeholder_text("12px")
        .build();
    radius.connect_changed({
        let state = Rc::clone(state);
        move |entry| {
            state.borrow_mut().model.tokens.border_radius = entry.text().to_string();
        }
    });
    body.append(&field_row("Radius", &radius));

    let border = Entry::builder()
        .text(&state.borrow().model.tokens.border)
        .placeholder_text("2px")
        .build();
    border.connect_changed({
        let state = Rc::clone(state);
        move |entry| {
            state.borrow_mut().model.tokens.border = entry.text().to_string();
        }
    });
    body.append(&field_row("Border width", &border));

    let padding = Entry::builder()
        .text(&state.borrow().model.tokens.padding)
        .placeholder_text("12px")
        .build();
    padding.connect_changed({
        let state = Rc::clone(state);
        move |entry| {
            state.borrow_mut().model.tokens.padding = entry.text().to_string();
        }
    });
    body.append(&field_row("Padding", &padding));

    let spacing = Entry::builder()
        .text(&state.borrow().model.tokens.spacing)
        .placeholder_text("6px")
        .build();
    spacing.connect_changed({
        let state = Rc::clone(state);
        move |entry| {
            state.borrow_mut().model.tokens.spacing = entry.text().to_string();
        }
    });
    body.append(&field_row("Spacing", &spacing));

    let element_padding = Entry::builder()
        .text(&state.borrow().model.tokens.element_padding)
        .placeholder_text("8px 12px")
        .build();
    element_padding.connect_changed({
        let state = Rc::clone(state);
        move |entry| {
            state.borrow_mut().model.tokens.element_padding = entry.text().to_string();
        }
    });
    body.append(&field_row("Row padding", &element_padding));

    let toggles = [
        (
            "Fullscreen",
            state.borrow().model.tokens.fullscreen,
            "fullscreen",
        ),
        (
            "Fixed list height",
            state.borrow().model.tokens.fixed_height,
            "fixed_height",
        ),
        (
            "Scrollbar",
            state.borrow().model.tokens.scrollbar,
            "scrollbar",
        ),
    ];
    for (label, active, key) in toggles {
        let cb = CheckButton::builder().label(label).active(active).build();
        cb.connect_toggled({
            let state = Rc::clone(state);
            let refresh_preview = Rc::clone(refresh_preview);
            move |btn| {
                {
                    let mut st = state.borrow_mut();
                    match key {
                        "fullscreen" => st.model.tokens.fullscreen = btn.is_active(),
                        "fixed_height" => st.model.tokens.fixed_height = btn.is_active(),
                        "scrollbar" => st.model.tokens.scrollbar = btn.is_active(),
                        _ => {}
                    }
                }
                call_preview(&refresh_preview);
            }
        });
        body.append(&cb);
    }
}

fn paint_swatch(swatch: &GtkBox, color: &str) {
    let css = format!(
        ".hyprbinds-rofi-swatch {{
            background-color: {};
            border-radius: 6px;
            border: 1px solid alpha(currentColor, 0.25);
            min-width: 28px;
            min-height: 28px;
        }}",
        css_color(color)
    );
    let provider = CssProvider::new();
    provider.load_from_string(&css);
    add_provider_to_widget(swatch, &provider);
}

fn rebuild_themes_tab(
    body: &GtkBox,
    state: &Rc<RefCell<StudioState>>,
    refresh_ui: &Rc<RefCell<Option<Rc<dyn Fn()>>>>,
) {
    clear_box(body);
    body.append(&section_title("Built-in palettes"));
    body.append(
        &Label::builder()
            .label("Apply a palette to Style tokens, then hit Apply to write files.")
            .halign(gtk4::Align::Start)
            .wrap(true)
            .xalign(0.0)
            .css_classes(["dim-label", "caption"])
            .build(),
    );

    for theme in rofi_theme::all_themes() {
        let row = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(10)
            .css_classes(["hyprbinds-rofi-theme-row"])
            .build();

        let swatches = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(4)
            .build();
        for color in [
            &theme.tokens.background,
            &theme.tokens.foreground,
            &theme.tokens.selected_bg,
            &theme.tokens.active,
        ] {
            let s = GtkBox::builder()
                .width_request(18)
                .height_request(18)
                .css_classes(["hyprbinds-rofi-swatch"])
                .build();
            paint_swatch(&s, color);
            swatches.append(&s);
        }

        let text = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .build();
        text.append(
            &Label::builder()
                .label(theme.name)
                .halign(gtk4::Align::Start)
                .css_classes(["hyprbinds-settings-title"])
                .build(),
        );
        text.append(
            &Label::builder()
                .label(theme.description)
                .halign(gtk4::Align::Start)
                .css_classes(["hyprbinds-settings-sub"])
                .build(),
        );

        let apply = Button::builder().label("Use").build();
        apply.connect_clicked({
            let state = Rc::clone(state);
            let refresh_ui = Rc::clone(refresh_ui);
            let id = theme.id;
            move |_| {
                if let Some(t) = rofi_theme::theme_by_id(id) {
                    // Keep layout geometry; only swap the color palette.
                    let keep = state.borrow().model.tokens.clone();
                    let mut tokens = t.tokens;
                    tokens.width = keep.width;
                    tokens.height = keep.height;
                    tokens.lines = keep.lines;
                    tokens.columns = keep.columns;
                    tokens.orientation = keep.orientation;
                    tokens.fullscreen = keep.fullscreen;
                    tokens.fixed_height = keep.fixed_height;
                    tokens.scrollbar = keep.scrollbar;
                    tokens.icon_size = keep.icon_size;
                    tokens.padding = keep.padding;
                    tokens.spacing = keep.spacing;
                    tokens.element_padding = keep.element_padding;
                    tokens.border = keep.border;
                    tokens.border_radius = keep.border_radius;
                    state.borrow_mut().model.tokens = tokens;
                    call_refresh(&refresh_ui);
                }
            }
        });

        row.append(&swatches);
        row.append(&text);
        row.append(&apply);
        body.append(&row);
    }
}
