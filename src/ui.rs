use crate::clients;
use crate::debounce::Debouncer;
use crate::dialog;
use crate::dispatcher_ui::ActionBuilder;
use crate::bind::{BindCollection, Keybind};
use crate::config::{self, ConfigError};
use crate::extra_ui;
use crate::keys::{self, normalize_keys_input};
use crate::variables::{
    self, action_to_template, filter_variables, resolve_to_template, unfinished_var_prefix,
    ConfigVariable,
};
use crate::window_rules::{self, WindowRule, BOOL_EFFECTS, BOOL_MATCH_PROPS};
use crate::writer::{self, WriteMode};
use gtk4::gdk::Key;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, Button, CheckButton, CssProvider, DropDown,
    Entry, EventControllerKey, Label, ListBox, ListBoxRow, Notebook, Orientation, PolicyType,
    ScrolledWindow, Separator, Stack, StringList, Window,
};
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;

struct AppState {
    collection: Option<BindCollection>,
    filter: String,
    /// Empty = all; `"__global__"` = global only; else submap name.
    bind_submap_filter: String,
    /// When true, only show binds that conflict with another.
    conflicts_only: bool,
    var_filter: String,
    rule_filter: String,
    status: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BindTabKind {
    All,
    Category(crate::dispatchers::Category),
    Other,
}

impl BindTabKind {
    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Category(c) => c.label(),
            Self::Other => "Other",
        }
    }

    fn matches(self, action: &str) -> bool {
        match self {
            Self::All => true,
            Self::Category(want) => {
                crate::dispatchers::action_category(action) == Some(want)
            }
            Self::Other => crate::dispatchers::action_category(action).is_none(),
        }
    }
}

struct BindTabs {
    notebook: Notebook,
    pages: Vec<(BindTabKind, ListBox, Label)>,
}

impl BindTabs {
    fn build() -> Self {
        let notebook = Notebook::new();
        notebook.add_css_class("hyprbinds-hub");
        notebook.add_css_class("hyprbinds-bind-tabs");
        notebook.set_scrollable(true);
        notebook.set_vexpand(true);
        notebook.set_hexpand(true);

        let mut kinds = vec![BindTabKind::All];
        for cat in crate::dispatchers::Category::all() {
            kinds.push(BindTabKind::Category(*cat));
        }
        kinds.push(BindTabKind::Other);

        let mut pages = Vec::new();
        for kind in kinds {
            let list = ListBox::builder()
                .selection_mode(gtk4::SelectionMode::Single)
                .css_classes(["boxed-list"])
                .build();
            let scroll = ScrolledWindow::builder()
                .hscrollbar_policy(PolicyType::Automatic)
                .vscrollbar_policy(PolicyType::Automatic)
                .vexpand(true)
                .child(&list)
                .build();
            let tab_label = Label::new(Some(kind.label()));
            notebook.append_page(&scroll, Some(&tab_label));
            pages.push((kind, list, tab_label));
        }

        Self { notebook, pages }
    }

    fn clear_all(&self) {
        for (_, list, _) in &self.pages {
            clear_list(list);
        }
    }

    fn selected_row(&self) -> Option<ListBoxRow> {
        let page = self.notebook.current_page()? as usize;
        self.pages.get(page)?.1.selected_row()
    }

    fn unselect_all(&self) {
        for (_, list, _) in &self.pages {
            list.unselect_all();
        }
    }
}

pub fn build_ui(app: &Application) {
    let prefs = crate::ui_prefs::load();
    crate::ui_prefs::apply_dark_mode(prefs.dark_mode);

    let state = Rc::new(RefCell::new(AppState {
        collection: None,
        filter: String::new(),
        bind_submap_filter: String::new(),
        conflicts_only: false,
        var_filter: String::new(),
        rule_filter: String::new(),
        status: String::new(),
    }));

    let path_label = Label::builder()
        .label("No config loaded")
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .ellipsize(gtk4::pango::EllipsizeMode::Middle)
        .css_classes(["hyprbinds-path", "dim-label"])
        .build();

    let count_label = Label::builder()
        .label("0 binds")
        .halign(gtk4::Align::End)
        .css_classes(["hyprbinds-count", "dim-label"])
        .build();

    let status_label = Label::builder()
        .label("")
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .wrap(true)
        .css_classes(["hyprbinds-status", "dim-label"])
        .build();

    let realtime_toggle = CheckButton::builder()
        .label("Realtime")
        .tooltip_text("Autosave ~0.8s after you stop typing (Config and open editors).")
        .active(false)
        .build();

    let prefs = crate::ui_prefs::load();
    let dark_toggle = CheckButton::builder()
        .label("Dark mode")
        .tooltip_text("Prefer a dark GTK theme for Hyprbinds.")
        .active(prefs.dark_mode)
        .build();
    dark_toggle.connect_toggled(move |btn| {
        let enabled = btn.is_active();
        crate::ui_prefs::apply_dark_mode(enabled);
        crate::ui_prefs::update(|prefs| prefs.dark_mode = enabled);
    });

    // --- Binds page ---
    let search = Entry::builder()
        .placeholder_text("Filter by name, key, action…")
        .hexpand(true)
        .build();
    let submap_filter_dd = DropDown::from_strings(&["All submaps"]);
    submap_filter_dd.set_tooltip_text(Some("Filter binds by submap"));
    let edit_bind_btn = Button::builder().label("Edit").sensitive(false).build();
    let delete_bind_btn = Button::builder()
        .label("Delete")
        .sensitive(false)
        .css_classes(["destructive-action"])
        .build();
    let add_bind_btn = Button::builder()
        .label("Add")
        .css_classes(["suggested-action"])
        .build();
    let conflicts_btn = Button::builder()
        .label("Conflicts")
        .tooltip_text("Show only binds that share the same chord in a submap")
        .build();
    let refresh = Button::builder().label("Reload").build();
    let restore_btn = Button::builder()
        .label("Restore")
        .tooltip_text("Restore last good config snapshot")
        .build();
    let bind_tabs = Rc::new(BindTabs::build());

    let bind_toolbar = GtkBox::new(Orientation::Horizontal, 8);
    bind_toolbar.append(&search);
    bind_toolbar.append(&submap_filter_dd);
    bind_toolbar.append(&add_bind_btn);
    bind_toolbar.append(&edit_bind_btn);
    bind_toolbar.append(&delete_bind_btn);
    bind_toolbar.append(&conflicts_btn);

    let binds_page = dialog::page_shell(
        "Binds",
        "Double-click to edit · Ctrl+P for commands",
        &bind_toolbar,
        &bind_tabs.notebook,
    );

    // --- Variables page ---
    let var_search = Entry::builder()
        .placeholder_text("Filter variables…")
        .hexpand(true)
        .build();
    let add_var_btn = Button::builder().label("Add").build();
    let edit_var_btn = Button::builder().label("Edit").sensitive(false).build();
    let delete_var_btn = Button::builder()
        .label("Delete")
        .sensitive(false)
        .css_classes(["destructive-action"])
        .build();
    let var_list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();
    let var_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .child(&var_list)
        .build();

    let var_toolbar = GtkBox::new(Orientation::Horizontal, 8);
    var_toolbar.append(&var_search);
    var_toolbar.append(&add_var_btn);
    var_toolbar.append(&edit_var_btn);
    var_toolbar.append(&delete_var_btn);

    let vars_page = dialog::page_shell(
        "Variables",
        "Local string variables. Renaming updates every identifier use across your Hyprland Lua files.",
        &var_toolbar,
        &var_scroll,
    );

    // --- Window rules page ---
    let rule_search = Entry::builder()
        .placeholder_text("Filter window rules…")
        .hexpand(true)
        .build();
    let add_rule_btn = Button::builder().label("Add").build();
    let edit_rule_btn = Button::builder().label("Edit").sensitive(false).build();
    let delete_rule_btn = Button::builder()
        .label("Delete")
        .sensitive(false)
        .css_classes(["destructive-action"])
        .build();
    let rule_list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();
    let rule_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .child(&rule_list)
        .build();

    let rule_toolbar = GtkBox::new(Orientation::Horizontal, 8);
    rule_toolbar.append(&rule_search);
    rule_toolbar.append(&add_rule_btn);
    rule_toolbar.append(&edit_rule_btn);
    rule_toolbar.append(&delete_rule_btn);

    let rules_page = dialog::page_shell(
        "Window rules",
        "Match by class/title. Use {{var}} in string fields. New rules append to a managed section.",
        &rule_toolbar,
        &rule_scroll,
    );

    // Pages assembled into sidebar stack after the window exists.
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Hyprbinds")
        .default_width(1120)
        .default_height(760)
        .build();

    apply_app_css();

    let shared_collection: Rc<RefCell<Option<BindCollection>>> = Rc::new(RefCell::new(None));
    let realtime = Rc::new(Cell::new(false));
    let current_page = Rc::new(RefCell::new(String::from("overview")));
    let developer_mode = Rc::new(Cell::new(prefs.developer_mode));

    realtime_toggle.connect_toggled({
        let realtime = Rc::clone(&realtime);
        let status_label = status_label.clone();
        move |btn| {
            let on = btn.is_active();
            realtime.set(on);
            status_label.set_text(if on {
                "Realtime on — changes autosave 0.8s after you stop typing."
            } else {
                "Realtime off — use Save / Add buttons."
            });
        }
    });

    let status_cb: extra_ui::StatusFn = {
        let status_label = status_label.clone();
        Rc::new(move |msg: String| status_label.set_text(&msg))
    };

    // Placeholder reload until real one is built.
    let reload_holder: Rc<RefCell<Option<extra_ui::ReloadFn>>> = Rc::new(RefCell::new(None));
    let reload_proxy: extra_ui::ReloadFn = {
        let reload_holder = Rc::clone(&reload_holder);
        Rc::new(move || {
            if let Some(f) = reload_holder.borrow().as_ref() {
                f();
            }
        })
    };

    let rules_extra = extra_ui::build_rules_extra(
        &window,
        Rc::clone(&shared_collection),
        Rc::clone(&reload_proxy),
        Rc::clone(&status_cb),
        Rc::clone(&realtime),
    );

    let settings_group = extra_ui::build_settings_group(
        &window,
        Rc::clone(&shared_collection),
        Rc::clone(&reload_proxy),
        Rc::clone(&status_cb),
        Rc::clone(&realtime),
    );

    let env_submaps = crate::env_ui::build_env_submap_pages(
        &window,
        Rc::clone(&shared_collection),
        Rc::clone(&reload_proxy),
        Rc::clone(&status_cb),
        Rc::clone(&realtime),
    );

    let health = crate::health_ui::build_health_page(Rc::clone(&status_cb), prefs.developer_mode);
    let logs = crate::logs_ui::build_logs_page(Rc::clone(&status_cb));
    let screenshare = crate::screenshare_ui::build_screenshare_page(Rc::clone(&status_cb));
    let audio_page = crate::audio_ui::build_audio_page(Rc::clone(&status_cb));
    let bundle_page = crate::bundle_ui::build_bundle_page(
        Rc::clone(&shared_collection),
        Rc::clone(&reload_proxy),
        Rc::clone(&status_cb),
        &window,
    );
    let wallpaper_page =
        crate::wallpaper_ui::build_wallpaper_page(&window, Rc::clone(&status_cb));
    let via_page = crate::via_ui::build_via_page(&window, Rc::clone(&status_cb));
    let waybar_page = crate::waybar_ui::build_waybar_page(&window, Rc::clone(&status_cb));
    let rofi_page = crate::rofi_ui::build_rofi_page(Rc::clone(&status_cb));
    let rofi_apps_page = crate::rofi_apps_ui::build_rofi_apps_page(
        &window,
        Rc::clone(&shared_collection),
        Rc::clone(&reload_proxy),
        Rc::clone(&status_cb),
    );
    let starship_page = crate::starship_ui::build_starship_page(Rc::clone(&status_cb));
    let lookfeel_page = crate::lookfeel_ui::build_lookfeel_page(
        &window,
        Rc::clone(&shared_collection),
        Rc::clone(&reload_proxy),
        Rc::clone(&status_cb),
    );
    let startup_page = crate::startup_ui::build_startup_page(
        &window,
        Rc::clone(&shared_collection),
        Rc::clone(&reload_proxy),
        Rc::clone(&status_cb),
        Rc::clone(&realtime),
    );
    let overview_page = crate::overview_ui::build_overview_page(
        Rc::clone(&shared_collection),
        Rc::clone(&reload_proxy),
        Rc::clone(&status_cb),
    );

    let rules_hub = hub_notebook(&[
        (&rules_page, "Window"),
        (&rules_extra.workspace_page, "Workspace"),
        (&rules_extra.layer_page, "Layer"),
    ]);
    let settings_hub = hub_notebook(&[
        (&lookfeel_page.page, "Look & Feel"),
        (&settings_group.config_page, "Config"),
        (&settings_group.monitors_page, "Monitors"),
        (&settings_group.devices_page, "Devices"),
        (&settings_group.animations_page, "Animations"),
        (&settings_group.curves_page, "Curves"),
        (&settings_group.gestures_page, "Gestures"),
    ]);
    let system_hub = hub_notebook(&[
        (&health.page, "Health"),
        (&screenshare.page, "Screenshare"),
        (&audio_page.page, "Audio"),
        (&logs.page, "Logs"),
        (&bundle_page.page, "Import / Export"),
    ]);
    rules_hub.set_show_tabs(false);
    settings_hub.set_show_tabs(false);
    system_hub.set_show_tabs(false);

    let stack = Stack::builder()
        .vexpand(true)
        .hexpand(true)
        .transition_type(gtk4::StackTransitionType::Crossfade)
        .transition_duration(160)
        .build();

    // Header chrome — title like a DE Settings app (sidebar replaces back button).
    let header_title = Label::builder()
        .label("Overview")
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .css_classes(["hyprbinds-header-title"])
        .build();

    // navigate_to is filled in after sidebar + chrome widgets exist.
    let navigate_holder: Rc<RefCell<Option<Rc<dyn Fn(&str)>>>> = Rc::new(RefCell::new(None));
    let navigate_proxy: Rc<dyn Fn(&str)> = {
        let navigate_holder = Rc::clone(&navigate_holder);
        Rc::new(move |id: &str| {
            if let Some(f) = navigate_holder.borrow().as_ref() {
                f(id);
            }
        })
    };

    let sidebar = Rc::new(crate::nav::build_sidebar(
        Rc::clone(&navigate_proxy),
        prefs.developer_mode,
    ));

    stack.add_named(&overview_page.page, Some("overview"));
    stack.add_named(&binds_page, Some("binds"));
    stack.add_named(&vars_page, Some("variables"));
    stack.add_named(&env_submaps.env_page, Some("environment"));
    stack.add_named(&env_submaps.submap_page, Some("submaps"));
    stack.add_named(&startup_page.page, Some("startup"));
    stack.add_named(&via_page.page, Some("via"));
    stack.add_named(&rules_hub, Some("rules"));
    stack.add_named(&settings_hub, Some("settings"));
    stack.add_named(&wallpaper_page.page, Some("wallpaper"));
    stack.add_named(&waybar_page.page, Some("waybar"));
    stack.add_named(&rofi_page.page, Some("rofi"));
    stack.add_named(&rofi_apps_page.page, Some("rofi-apps"));
    stack.add_named(&starship_page.page, Some("starship"));
    stack.add_named(&system_hub, Some("system"));
    stack.set_visible_child_name("overview");

    let navigate_to: Rc<dyn Fn(&str)> = {
        let stack = stack.clone();
        let current_page = Rc::clone(&current_page);
        let developer_mode = Rc::clone(&developer_mode);
        let rules_hub = rules_hub.clone();
        let settings_hub = settings_hub.clone();
        let system_hub = system_hub.clone();
        let header_title = header_title.clone();
        let sidebar = Rc::clone(&sidebar);
        Rc::new(move |id: &str| {
            let id = if id == "via" && !developer_mode.get() {
                "overview"
            } else if id == "home" {
                "overview"
            } else {
                id
            };
            let dest = resolve_nav_destination(id);
            stack.set_visible_child_name(dest.stack_id);
            let page_id = if let Some(tab) = dest.tab {
                match dest.stack_id {
                    "rules" => rules_hub.set_current_page(Some(tab)),
                    "settings" => settings_hub.set_current_page(Some(tab)),
                    "system" => system_hub.set_current_page(Some(tab)),
                    _ => {}
                }
                dest.page_id
            } else {
                match dest.stack_id {
                    "rules" => hub_page_id(
                        &["rules-window", "rules-workspace", "rules-layer"],
                        rules_hub.current_page(),
                        dest.page_id,
                    ),
                    "settings" => hub_page_id(
                        &[
                            "lookfeel",
                            "settings-config",
                            "settings-monitors",
                            "settings-devices",
                            "settings-animations",
                            "settings-curves",
                            "settings-gestures",
                        ],
                        settings_hub.current_page(),
                        dest.page_id,
                    ),
                    "system" => hub_page_id(
                        &["health", "screenshare", "audio", "logs", "import-export"],
                        system_hub.current_page(),
                        dest.page_id,
                    ),
                    _ => dest.page_id,
                }
            };
            *current_page.borrow_mut() = page_id.to_string();
            header_title.set_text(crate::nav::page_chrome_title(page_id));
            sidebar.select(page_id);
        })
    };
    *navigate_holder.borrow_mut() = Some(Rc::clone(&navigate_to));
    sidebar.select("overview");

    wire_hub_page_tracking(
        &rules_hub,
        &["rules-window", "rules-workspace", "rules-layer"],
        &current_page,
        &header_title,
        &sidebar,
    );
    wire_hub_page_tracking(
        &settings_hub,
        &[
            "lookfeel",
            "settings-config",
            "settings-monitors",
            "settings-devices",
            "settings-animations",
            "settings-curves",
            "settings-gestures",
        ],
        &current_page,
        &header_title,
        &sidebar,
    );
    wire_hub_page_tracking(
        &system_hub,
        &["health", "screenshare", "audio", "logs", "import-export"],
        &current_page,
        &header_title,
        &sidebar,
    );

    let header = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .css_classes(["hyprbinds-header"])
        .build();
    header.append(&header_title);
    path_label.set_halign(gtk4::Align::End);
    header.append(&path_label);
    header.append(&count_label);
    dark_toggle.add_css_class("hyprbinds-header-toggle");
    realtime_toggle.add_css_class("hyprbinds-header-toggle");
    header.append(&dark_toggle);
    header.append(&realtime_toggle);
    restore_btn.add_css_class("hyprbinds-header-btn");
    refresh.add_css_class("hyprbinds-reload");
    header.append(&restore_btn);
    header.append(&refresh);

    let main_col = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .hexpand(true)
        .vexpand(true)
        .css_classes(["hyprbinds-main"])
        .build();
    main_col.append(&header);
    let content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .hexpand(true)
        .vexpand(true)
        .margin_top(8)
        .margin_bottom(12)
        .margin_start(28)
        .margin_end(28)
        .css_classes(["hyprbinds-content"])
        .build();
    content.append(&stack);
    let status_sep = Separator::new(Orientation::Horizontal);
    status_sep.add_css_class("hyprbinds-status-sep");
    content.append(&status_sep);
    content.append(&status_label);
    main_col.append(&content);

    let body = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(0)
        .css_classes(["hyprbinds-shell"])
        .build();
    body.append(&sidebar.widget);
    body.append(&main_col);

    window.set_child(Some(&body));

    let settings_group = Rc::new(settings_group);
    let rules_extra = Rc::new(rules_extra);
    let env_submaps = Rc::new(env_submaps);
    let startup_page = Rc::new(startup_page);
    let reload = {
        let state = Rc::clone(&state);
        let shared_collection = Rc::clone(&shared_collection);
        let bind_tabs = Rc::clone(&bind_tabs);
        let var_list = var_list.clone();
        let rule_list = rule_list.clone();
        let rules_extra = Rc::clone(&rules_extra);
        let settings_group = Rc::clone(&settings_group);
        let env_submaps = Rc::clone(&env_submaps);
        let startup_page = Rc::clone(&startup_page);
        let overview_refresh = Rc::clone(&overview_page.refresh);
        let submap_filter_dd = submap_filter_dd.clone();
        let path_label = path_label.clone();
        let count_label = count_label.clone();
        let status_label = status_label.clone();
        let edit_bind_btn = edit_bind_btn.clone();
        let delete_bind_btn = delete_bind_btn.clone();
        let edit_var_btn = edit_var_btn.clone();
        let delete_var_btn = delete_var_btn.clone();
        let edit_rule_btn = edit_rule_btn.clone();
        let delete_rule_btn = delete_rule_btn.clone();
        Rc::new(move || match config::load_binds(None) {
            Ok(mut collection) => {
                collection.finalize();
                let path = collection.config_path.display().to_string();
                let err = collection.error.clone();
                let total = collection.binds.len();
                let vars = collection.variables.len();
                let rules = collection.window_rules.len();
                let envs = collection.env.len();
                let submaps = collection.submaps.len();
                let startups = collection.startup.len();
                {
                    let mut s = state.borrow_mut();
                    s.status = match &err {
                        Some(e) => format!("Loaded with errors: {e}"),
                        None => format!(
                            "Loaded {total} binds, {vars} vars, {rules} rules, {envs} env, {submaps} submaps, {startups} startup"
                        ),
                    };
                    *shared_collection.borrow_mut() = Some(collection.clone());
                    s.collection = Some(collection);
                }
                path_label.set_text(&path);
                status_label.set_text(&state.borrow().status);
                edit_bind_btn.set_sensitive(false);
                delete_bind_btn.set_sensitive(false);
                edit_var_btn.set_sensitive(false);
                delete_var_btn.set_sensitive(false);
                edit_rule_btn.set_sensitive(false);
                delete_rule_btn.set_sensitive(false);
                refresh_submap_filter_dropdown(&submap_filter_dd, &state);
                render_bind_list(&state, &bind_tabs, &count_label);
                render_var_list(&state, &var_list);
                render_rule_list(&state, &rule_list);
                crate::env_ui::render_env_list(&shared_collection, &env_submaps.env_list, "");
                crate::env_ui::render_submap_list(&shared_collection, &env_submaps.submap_list, "");
                crate::startup_ui::render_startup_list(&shared_collection, &startup_page.list, "");
                extra_ui::render_workspace_list(&shared_collection, &rules_extra.workspace_list, "");
                extra_ui::render_layer_list(&shared_collection, &rules_extra.layer_list, "");
                if let Some(c) = shared_collection.borrow().as_ref() {
                    extra_ui::render_settings_lists(&settings_group, c);
                }
                overview_refresh();
            }
            Err(err) => {
                state.borrow_mut().collection = None;
                *shared_collection.borrow_mut() = None;
                state.borrow_mut().status = format_error(&err);
                path_label.set_text(
                    &config::default_config_path()
                        .unwrap_or_else(|| PathBuf::from("~/.config/hypr/hyprland.lua"))
                        .display()
                        .to_string(),
                );
                count_label.set_text("0 binds");
                status_label.set_text(&state.borrow().status);
                edit_bind_btn.set_sensitive(false);
                delete_bind_btn.set_sensitive(false);
                edit_var_btn.set_sensitive(false);
                delete_var_btn.set_sensitive(false);
                edit_rule_btn.set_sensitive(false);
                delete_rule_btn.set_sensitive(false);
                bind_tabs.clear_all();
                clear_list(&var_list);
                clear_list(&rule_list);
                clear_list(&env_submaps.env_list);
                clear_list(&env_submaps.submap_list);
                clear_list(&startup_page.list);
                clear_list(&rules_extra.workspace_list);
                clear_list(&rules_extra.layer_list);
                overview_refresh();
            }
        })
    };
    *reload_holder.borrow_mut() = Some(Rc::clone(&reload) as extra_ui::ReloadFn);

    refresh.connect_clicked({
        let reload = Rc::clone(&reload);
        move |_| reload()
    });

    restore_btn.connect_clicked({
        let reload = Rc::clone(&reload);
        let status_label = status_label.clone();
        let shared_collection = Rc::clone(&shared_collection);
        move |_| {
            let path = shared_collection
                .borrow()
                .as_ref()
                .map(|c| c.config_path.clone())
                .or_else(config::default_config_path);
            let Some(path) = path else {
                status_label.set_text("No config path to restore.");
                return;
            };
            match crate::backup::restore_last_good(&path) {
                Ok(from) => {
                    status_label.set_text(&format!(
                        "Restored {} from {}",
                        path.display(),
                        from.display()
                    ));
                    reload();
                }
                Err(e) => status_label.set_text(&format!("Restore failed: {e}")),
            }
        }
    });

    conflicts_btn.connect_clicked({
        let state = Rc::clone(&state);
        let bind_tabs = Rc::clone(&bind_tabs);
        let count_label = count_label.clone();
        let status_label = status_label.clone();
        let conflicts_btn = conflicts_btn.clone();
        let navigate_to = Rc::clone(&navigate_to);
        move |btn| {
            let mut s = state.borrow_mut();
            s.conflicts_only = !s.conflicts_only;
            let on = s.conflicts_only;
            drop(s);
            if on {
                btn.add_css_class("suggested-action");
                conflicts_btn.set_label("Conflicts ✓");
            } else {
                btn.remove_css_class("suggested-action");
                conflicts_btn.set_label("Conflicts");
            }
            navigate_to("binds");
            render_bind_list(&state, &bind_tabs, &count_label);
            if let Some(c) = state.borrow().collection.as_ref() {
                let groups = crate::conflicts::find_conflicts(&c.binds);
                status_label.set_text(&crate::conflicts::summary(&groups));
            }
        }
    });

    // Command palette (Ctrl+P)
    let palette = crate::palette::attach_palette(
        &window,
        Rc::new({
            let navigate_to = Rc::clone(&navigate_to);
            let reload = Rc::clone(&reload);
            let restore_btn = restore_btn.clone();
            let conflicts_btn = conflicts_btn.clone();
            let status_label = status_label.clone();
            move |id: String| {
                if let Some(page) = id.strip_prefix("goto:") {
                    navigate_to(page);
                    return;
                }
                match id.as_str() {
                    "action:reload" => reload(),
                    "action:restore" => restore_btn.emit_by_name::<()>("clicked", &[]),
                    "action:conflicts" => conflicts_btn.emit_by_name::<()>("clicked", &[]),
                    "action:sysinfo" => {
                        let report = crate::sysinfo::collect_report();
                        if let Some(display) = gtk4::gdk::Display::default() {
                            display.clipboard().set_text(&report);
                            status_label.set_text(&format!(
                                "Copied system info ({} bytes).",
                                report.len()
                            ));
                        }
                    }
                    "action:health" => navigate_to("health"),
                    _ => {}
                }
            }
        }),
    );
    {
        let mut actions = crate::palette::default_nav_actions(developer_mode.get());
        actions.insert(0, crate::palette::action_reload());
        actions.insert(1, crate::palette::action_restore());
        actions.insert(2, crate::palette::action_conflicts());
        actions.insert(3, crate::palette::action_sysinfo());
        actions.insert(4, crate::palette::action_health());
        palette.set_actions(actions);
    }

    health.developer_toggle.connect_toggled({
        let sidebar = Rc::clone(&sidebar);
        let palette = palette.clone();
        let developer_mode = Rc::clone(&developer_mode);
        let current_page = Rc::clone(&current_page);
        let navigate_to = Rc::clone(&navigate_to);
        let status_label = status_label.clone();
        move |btn| {
            let enabled = btn.is_active();
            developer_mode.set(enabled);
            crate::ui_prefs::update(|prefs| prefs.developer_mode = enabled);
            sidebar.set_developer_mode(enabled);

            let mut actions = crate::palette::default_nav_actions(enabled);
            actions.insert(0, crate::palette::action_reload());
            actions.insert(1, crate::palette::action_restore());
            actions.insert(2, crate::palette::action_conflicts());
            actions.insert(3, crate::palette::action_sysinfo());
            actions.insert(4, crate::palette::action_health());
            palette.set_actions(actions);

            if !enabled && current_page.borrow().as_str() == "via" {
                navigate_to("overview");
            }
            status_label.set_text(if enabled {
                "Developer mode on — experimental pages unlocked."
            } else {
                "Developer mode off — experimental pages hidden."
            });
        }
    });

    search.connect_changed({
        let state = Rc::clone(&state);
        let bind_tabs = Rc::clone(&bind_tabs);
        let count_label = count_label.clone();
        let edit_bind_btn = edit_bind_btn.clone();
        let delete_bind_btn = delete_bind_btn.clone();
        move |entry| {
            state.borrow_mut().filter = entry.text().to_string();
            edit_bind_btn.set_sensitive(false);
            delete_bind_btn.set_sensitive(false);
            render_bind_list(&state, &bind_tabs, &count_label);
        }
    });

    submap_filter_dd.connect_selected_notify({
        let state = Rc::clone(&state);
        let bind_tabs = Rc::clone(&bind_tabs);
        let count_label = count_label.clone();
        let edit_bind_btn = edit_bind_btn.clone();
        let delete_bind_btn = delete_bind_btn.clone();
        let submap_filter_dd = submap_filter_dd.clone();
        move |_| {
            let selected = submap_filter_dd.selected();
            let filter = match selected {
                0 => String::new(),
                1 => "__global__".into(),
                n => submap_filter_dd
                    .model()
                    .and_then(|m| m.item(n).and_then(|o| o.downcast::<gtk4::StringObject>().ok()))
                    .map(|o| o.string().to_string())
                    .unwrap_or_default(),
            };
            state.borrow_mut().bind_submap_filter = filter;
            edit_bind_btn.set_sensitive(false);
            delete_bind_btn.set_sensitive(false);
            render_bind_list(&state, &bind_tabs, &count_label);
        }
    });

    var_search.connect_changed({
        let state = Rc::clone(&state);
        let var_list = var_list.clone();
        let edit_var_btn = edit_var_btn.clone();
        let delete_var_btn = delete_var_btn.clone();
        move |entry| {
            state.borrow_mut().var_filter = entry.text().to_string();
            edit_var_btn.set_sensitive(false);
            delete_var_btn.set_sensitive(false);
            render_var_list(&state, &var_list);
        }
    });

    rule_search.connect_changed({
        let state = Rc::clone(&state);
        let rule_list = rule_list.clone();
        let edit_rule_btn = edit_rule_btn.clone();
        let delete_rule_btn = delete_rule_btn.clone();
        move |entry| {
            state.borrow_mut().rule_filter = entry.text().to_string();
            edit_rule_btn.set_sensitive(false);
            delete_rule_btn.set_sensitive(false);
            render_rule_list(&state, &rule_list);
        }
    });

    for (_, list, _) in &bind_tabs.pages {
        list.connect_row_selected({
            let edit_bind_btn = edit_bind_btn.clone();
            let delete_bind_btn = delete_bind_btn.clone();
            let bind_tabs = Rc::clone(&bind_tabs);
            move |this, row| {
                if row.is_some() {
                    for (_, other, _) in &bind_tabs.pages {
                        if other != this {
                            other.unselect_all();
                        }
                    }
                }
                let on = row.is_some_and(|r| r.widget_name().starts_with("bind-"));
                edit_bind_btn.set_sensitive(on);
                delete_bind_btn.set_sensitive(on);
            }
        });
    }

    bind_tabs.notebook.connect_switch_page({
        let edit_bind_btn = edit_bind_btn.clone();
        let delete_bind_btn = delete_bind_btn.clone();
        let bind_tabs = Rc::clone(&bind_tabs);
        move |_, _, _| {
            bind_tabs.unselect_all();
            edit_bind_btn.set_sensitive(false);
            delete_bind_btn.set_sensitive(false);
        }
    });

    var_list.connect_row_selected({
        let edit_var_btn = edit_var_btn.clone();
        let delete_var_btn = delete_var_btn.clone();
        move |_, row| {
            let on = row.is_some();
            edit_var_btn.set_sensitive(on);
            delete_var_btn.set_sensitive(on);
        }
    });

    rule_list.connect_row_selected({
        let edit_rule_btn = edit_rule_btn.clone();
        let delete_rule_btn = delete_rule_btn.clone();
        move |_, row| {
            let on = row.is_some();
            edit_rule_btn.set_sensitive(on);
            delete_rule_btn.set_sensitive(on);
        }
    });

    let open_bind_editor = {
        let state = Rc::clone(&state);
        let bind_tabs = Rc::clone(&bind_tabs);
        let window = window.clone();
        let reload = Rc::clone(&reload);
        let status_label = status_label.clone();
        let realtime = Rc::clone(&realtime);
        Rc::new(move || {
            let Some(row) = bind_tabs.selected_row() else {
                return;
            };
            let id = row_id(&row, "bind-");
            let (bind, shared, variables, submap_names) = {
                let s = state.borrow();
                let Some(collection) = s.collection.as_ref() else {
                    return;
                };
                let Some(bind) = collection.bind_by_id(id).cloned() else {
                    return;
                };
                let shared = collection.source_share_count(&bind) > 1;
                (
                    bind,
                    shared,
                    collection.variables.clone(),
                    collection.submap_names(),
                )
            };
            show_edit_dialog(
                &window,
                Some(bind),
                shared,
                None,
                variables,
                submap_names,
                String::new(),
                Rc::clone(&realtime),
                {
                    let reload = Rc::clone(&reload);
                    let status_label = status_label.clone();
                    move |msg| {
                        status_label.set_text(&msg);
                        reload();
                    }
                },
            );
        })
    };

    let open_new_bind = {
        let state = Rc::clone(&state);
        let window = window.clone();
        let reload = Rc::clone(&reload);
        let status_label = status_label.clone();
        let realtime = Rc::clone(&realtime);
        Rc::new(move || {
            let (config_path, variables, submap_names, preferred_submap) = {
                let s = state.borrow();
                let Some(collection) = s.collection.as_ref() else {
                    status_label.set_text("Load a config before adding binds.");
                    return;
                };
                (
                    collection.config_path.clone(),
                    collection.variables.clone(),
                    collection.submap_names(),
                    s.bind_submap_filter.clone(),
                )
            };
            show_edit_dialog(
                &window,
                None,
                false,
                Some(config_path),
                variables,
                submap_names,
                preferred_submap,
                Rc::clone(&realtime),
                {
                    let reload = Rc::clone(&reload);
                    let status_label = status_label.clone();
                    move |msg| {
                        status_label.set_text(&msg);
                        reload();
                    }
                },
            );
        })
    };

    let open_var_editor = {
        let state = Rc::clone(&state);
        let var_list = var_list.clone();
        let window = window.clone();
        let reload = Rc::clone(&reload);
        let status_label = status_label.clone();
        let realtime = Rc::clone(&realtime);
        Rc::new(move || {
            let Some(row) = var_list.selected_row() else {
                return;
            };
            let name = row
                .widget_name()
                .strip_prefix("var-")
                .unwrap_or("")
                .to_string();
            let (var, related_files, existing_names) = {
                let s = state.borrow();
                let Some(collection) = s.collection.as_ref() else {
                    return;
                };
                let Some(var) = collection
                    .variables
                    .iter()
                    .find(|v| v.name == name)
                    .cloned()
                else {
                    return;
                };
                (
                    var,
                    collection.related_files(),
                    collection.variable_names(),
                )
            };
            show_variable_dialog(
                &window,
                Some(var),
                None,
                related_files,
                existing_names,
                Rc::clone(&realtime),
                {
                    let reload = Rc::clone(&reload);
                    let status_label = status_label.clone();
                    move |msg| {
                        status_label.set_text(&msg);
                        reload();
                    }
                },
            );
        })
    };

    edit_bind_btn.connect_clicked({
        let open_bind_editor = Rc::clone(&open_bind_editor);
        move |_| open_bind_editor()
    });
    add_bind_btn.connect_clicked({
        let open_new_bind = Rc::clone(&open_new_bind);
        move |_| open_new_bind()
    });
    delete_bind_btn.connect_clicked({
        let state = Rc::clone(&state);
        let bind_tabs = Rc::clone(&bind_tabs);
        let reload = Rc::clone(&reload);
        let status_label = status_label.clone();
        move |_| {
            let Some(row) = bind_tabs.selected_row() else {
                return;
            };
            let id = row_id(&row, "bind-");
            let (bind, shared) = {
                let s = state.borrow();
                let Some(collection) = s.collection.as_ref() else {
                    return;
                };
                let Some(bind) = collection.bind_by_id(id).cloned() else {
                    return;
                };
                let shared = collection.source_share_count(&bind) > 1;
                (bind, shared)
            };
            match writer::delete_bind(&bind, shared) {
                Ok(result) => {
                    let how = if shared {
                        format!("unbound “{}” via {}", bind.keys, result.path)
                    } else {
                        format!("Deleted bind “{}” from {}", bind.name, result.path)
                    };
                    status_label.set_text(&how);
                    reload();
                }
                Err(err) => status_label.set_text(&format!("Delete failed: {err}")),
            }
        }
    });
    for (_, list, _) in &bind_tabs.pages {
        list.connect_row_activated({
            let open_bind_editor = Rc::clone(&open_bind_editor);
            move |_, _| open_bind_editor()
        });
    }

    edit_var_btn.connect_clicked({
        let open_var_editor = Rc::clone(&open_var_editor);
        move |_| open_var_editor()
    });
    var_list.connect_row_activated({
        let open_var_editor = Rc::clone(&open_var_editor);
        move |_, _| open_var_editor()
    });

    add_var_btn.connect_clicked({
        let state = Rc::clone(&state);
        let window = window.clone();
        let reload = Rc::clone(&reload);
        let status_label = status_label.clone();
        let realtime = Rc::clone(&realtime);
        move |_| {
            let (config_path, related_files, existing_names) = {
                let s = state.borrow();
                let Some(collection) = s.collection.as_ref() else {
                    status_label.set_text("Load a config before adding variables.");
                    return;
                };
                (
                    collection.config_path.clone(),
                    collection.related_files(),
                    collection.variable_names(),
                )
            };
            show_variable_dialog(
                &window,
                None,
                Some(config_path),
                related_files,
                existing_names,
                Rc::clone(&realtime),
                {
                    let reload = Rc::clone(&reload);
                    let status_label = status_label.clone();
                    move |msg| {
                        status_label.set_text(&msg);
                        reload();
                    }
                },
            );
        }
    });

    delete_var_btn.connect_clicked({
        let state = Rc::clone(&state);
        let var_list = var_list.clone();
        let reload = Rc::clone(&reload);
        let status_label = status_label.clone();
        move |_| {
            let Some(row) = var_list.selected_row() else {
                return;
            };
            let name = row
                .widget_name()
                .strip_prefix("var-")
                .unwrap_or("")
                .to_string();
            let var = {
                let s = state.borrow();
                s.collection.as_ref().and_then(|c| {
                    c.variables
                        .iter()
                        .find(|v| v.name == name)
                        .cloned()
                })
            };
            let Some(var) = var else {
                return;
            };
            match writer::delete_variable(&var) {
                Ok(result) => {
                    status_label.set_text(&format!(
                        "Deleted variable “{}” from {}",
                        var.name, result.path
                    ));
                    reload();
                }
                Err(err) => status_label.set_text(&format!("Delete failed: {err}")),
            }
        }
    });

    let open_rule_editor = {
        let state = Rc::clone(&state);
        let rule_list = rule_list.clone();
        let window = window.clone();
        let reload = Rc::clone(&reload);
        let status_label = status_label.clone();
        let realtime = Rc::clone(&realtime);
        Rc::new(move || {
            let Some(row) = rule_list.selected_row() else {
                return;
            };
            let id = row_id(&row, "rule-");
            let (rule, variables) = {
                let s = state.borrow();
                let Some(collection) = s.collection.as_ref() else {
                    return;
                };
                let Some(rule) = collection.rule_by_id(id).cloned() else {
                    return;
                };
                (rule, collection.variables.clone())
            };
            show_rule_dialog(
                &window,
                Some(rule),
                None,
                variables,
                Rc::clone(&realtime),
                {
                    let reload = Rc::clone(&reload);
                    let status_label = status_label.clone();
                    move |msg| {
                        status_label.set_text(&msg);
                        reload();
                    }
                },
            );
        })
    };

    edit_rule_btn.connect_clicked({
        let open_rule_editor = Rc::clone(&open_rule_editor);
        move |_| open_rule_editor()
    });
    rule_list.connect_row_activated({
        let open_rule_editor = Rc::clone(&open_rule_editor);
        move |_, _| open_rule_editor()
    });

    add_rule_btn.connect_clicked({
        let state = Rc::clone(&state);
        let window = window.clone();
        let reload = Rc::clone(&reload);
        let status_label = status_label.clone();
        let realtime = Rc::clone(&realtime);
        move |_| {
            let (config_path, variables) = {
                let s = state.borrow();
                let Some(collection) = s.collection.as_ref() else {
                    status_label.set_text("Load a config before adding window rules.");
                    return;
                };
                (
                    collection.config_path.clone(),
                    collection.variables.clone(),
                )
            };
            show_rule_dialog(
                &window,
                None,
                Some(config_path),
                variables,
                Rc::clone(&realtime),
                {
                    let reload = Rc::clone(&reload);
                    let status_label = status_label.clone();
                    move |msg| {
                        status_label.set_text(&msg);
                        reload();
                    }
                },
            );
        }
    });

    delete_rule_btn.connect_clicked({
        let state = Rc::clone(&state);
        let rule_list = rule_list.clone();
        let reload = Rc::clone(&reload);
        let status_label = status_label.clone();
        move |_| {
            let Some(row) = rule_list.selected_row() else {
                return;
            };
            let id = row_id(&row, "rule-");
            let rule = {
                let s = state.borrow();
                s.collection
                    .as_ref()
                    .and_then(|c| c.rule_by_id(id).cloned())
            };
            let Some(rule) = rule else {
                return;
            };
            match writer::delete_window_rule(&rule) {
                Ok(result) => {
                    status_label.set_text(&format!(
                        "Deleted window rule “{}” from {}",
                        rule.display_name, result.path
                    ));
                    reload();
                }
                Err(err) => status_label.set_text(&format!("Delete failed: {err}")),
            }
        }
    });

    let key_controller = EventControllerKey::new();
    key_controller.connect_key_pressed({
        let search = search.clone();
        let open_bind_editor = Rc::clone(&open_bind_editor);
        let current_page = Rc::clone(&current_page);
        let sidebar_filter = sidebar.filter.clone();
        move |_, key, _, _| {
            if key == Key::slash {
                let page = current_page.borrow().clone();
                if page == "binds" {
                    search.grab_focus();
                    return glib::Propagation::Stop;
                }
                sidebar_filter.grab_focus();
                return glib::Propagation::Stop;
            }
            if key == Key::Return || key == Key::KP_Enter {
                if current_page.borrow().as_str() == "binds" {
                    open_bind_editor();
                }
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        }
    });
    window.add_controller(key_controller);

    reload();
    sidebar.filter.grab_focus();
    window.present();
}

fn apply_app_css() {
    // Theme `:selected` often only recolors text for list rows. Nav fill uses
    // an explicit `.nav-active` class. Cyan rail + key chips give a Hyprland feel
    // without fighting the host Adwaita/system palette.
    //
    // We ship Adwaita-like utility classes (dim-label, caption, …) ourselves so
    // typography works without depending on libadwaita.
    let css = r#"
        @define-color hypr_cyan #00d4ff;
        @define-color hypr_cyan_dim alpha(#00d4ff, 0.18);
        @define-color hypr_cyan_soft alpha(#00d4ff, 0.10);
        @define-color hypr_ok #2ec27e;
        @define-color hypr_warn #e5a50a;

        /* Adwaita utility shims (plain GTK4 has no libadwaita styles) */
        .dim-label {
            opacity: 0.55;
        }
        .caption {
            font-size: 0.82em;
            opacity: 0.72;
        }
        .dim-label.caption {
            opacity: 0.58;
        }
        .monospace {
            font-family: monospace;
        }
        .title-4 {
            font-size: 1.05em;
            font-weight: 700;
            letter-spacing: -0.01em;
        }
        .hyprbinds-field-label {
            font-size: 0.78em;
            font-weight: 600;
            letter-spacing: 0.02em;
            opacity: 0.62;
        }

        /* Compact controls — override chunky default GTK sizing */
        button {
            min-height: 26px;
            min-width: 0;
            padding: 2px 10px;
            border-radius: 6px;
            font-size: 0.88em;
            font-weight: 500;
            box-shadow: none;
            text-shadow: none;
            outline-offset: -2px;
        }
        button.flat {
            padding: 2px 8px;
        }
        button.circular {
            min-width: 26px;
            min-height: 26px;
            padding: 0;
        }

        entry {
            min-height: 26px;
            padding: 2px 8px;
            border-radius: 6px;
            box-shadow: none;
            font-size: 0.92em;
        }
        entry.flat {
            min-height: 26px;
        }

        dropdown {
            font-size: 0.88em;
        }
        dropdown > button,
        dropdown button.toggle {
            min-height: 26px;
            padding: 2px 8px;
            border-radius: 6px;
            font-weight: 500;
            box-shadow: none;
            text-shadow: none;
        }
        dropdown arrow {
            opacity: 0.55;
            margin-left: 4px;
        }
        dropdown popover contents {
            padding: 4px;
        }
        dropdown popover listview > row {
            min-height: 26px;
            padding: 2px 8px;
            border-radius: 5px;
        }
        dropdown popover listview > row > box {
            padding: 0;
        }

        spinbutton {
            min-height: 26px;
            border-radius: 6px;
            box-shadow: none;
            font-size: 0.9em;
        }
        spinbutton > text {
            min-height: 24px;
            padding: 1px 6px;
        }
        spinbutton > button {
            min-height: 22px;
            min-width: 20px;
            padding: 0;
            border-radius: 4px;
        }

        checkbutton {
            min-height: 0;
            padding: 1px 0;
            font-size: 0.9em;
        }
        checkbutton check,
        checkbutton radio {
            min-width: 14px;
            min-height: 14px;
            margin-right: 6px;
        }

        expander {
            font-size: 0.88em;
        }
        expander title {
            min-height: 24px;
            padding: 2px 0;
        }

        popover.menu contents {
            padding: 4px;
        }
        popover.menu contents modelbutton,
        popover.menu contents > list > row {
            min-height: 26px;
            padding: 2px 8px;
            border-radius: 5px;
        }

        .hyprbinds-shell {
            background-color: @window_bg_color;
        }

        .hyprbinds-header {
            padding: 14px 28px 12px 28px;
            border-bottom: 1px solid alpha(currentColor, 0.08);
            background-color: alpha(currentColor, 0.025);
        }
        .hyprbinds-header-title {
            font-size: 1.15em;
            font-weight: 700;
            letter-spacing: -0.02em;
        }
        .hyprbinds-header-toggle {
            margin-left: 4px;
        }
        .hyprbinds-header-btn {
            margin-left: 2px;
        }

        .hyprbinds-main {
            background-color: transparent;
        }
        .hyprbinds-content {
            background-color: transparent;
        }
        .hyprbinds-path {
            opacity: 0.55;
            font-size: 0.82em;
            font-family: monospace;
            margin-right: 6px;
        }
        .hyprbinds-count {
            opacity: 0.5;
            font-size: 0.82em;
            margin-right: 8px;
        }

        .hyprbinds-sidebar {
            min-width: 240px;
            max-width: 280px;
            padding: 12px 10px 12px 12px;
            border-right: 1px solid alpha(currentColor, 0.08);
            background-color: alpha(currentColor, 0.02);
        }
        .hyprbinds-sidebar-filter {
            margin: 0 2px 4px 2px;
            min-height: 28px;
        }
        .hyprbinds-sidebar-scroll {
            background: transparent;
        }
        list.hyprbinds-sidebar-list {
            background: transparent;
            border: none;
            box-shadow: none;
        }
        list.hyprbinds-sidebar-list > row {
            min-height: 0;
            padding: 0;
            border-radius: 8px;
            margin: 1px 0;
            border: none;
            transition: background-color 80ms ease;
        }
        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-header-row {
            margin-top: 10px;
            margin-bottom: 2px;
            background: transparent;
        }
        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-header-row:hover,
        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-header-row:selected {
            background: transparent;
            box-shadow: none;
        }
        .hyprbinds-sidebar-section {
            font-size: 0.68em;
            font-weight: 700;
            text-transform: uppercase;
            letter-spacing: 0.1em;
            opacity: 0.45;
            padding: 4px 8px 2px 8px;
        }
        .hyprbinds-sidebar-row-inner {
            padding: 8px 10px;
        }
        .hyprbinds-sidebar-icon {
            font-size: 1.05em;
            min-width: 1.4em;
            opacity: 0.85;
        }
        .hyprbinds-sidebar-label {
            font-size: 0.95em;
            font-weight: 550;
        }
        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-row:hover {
            background-color: alpha(currentColor, 0.05);
        }
        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-row:selected,
        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-row.nav-active {
            background-color: @hypr_cyan_soft;
            box-shadow: inset 3px 0 0 0 @hypr_cyan;
        }

        notebook.hyprbinds-hub {
            background-color: transparent;
            border: none;
        }
        notebook.hyprbinds-hub > header {
            background-color: transparent;
            border: none;
        }
        notebook.hyprbinds-hub > stack {
            background-color: transparent;
        }

        .hyprbinds-reload {
            min-width: 72px;
        }
        .hyprbinds-status-sep {
            margin-top: 4px;
            opacity: 0.55;
        }
        .hyprbinds-status {
            padding-top: 6px;
            font-size: 0.86em;
            opacity: 0.68;
        }
        .hyprbinds-status.error {
            color: @error_color;
            opacity: 1;
            font-weight: 600;
        }

        .hyprbinds-page {
            margin: 0;
        }
        .hyprbinds-page-header {
            margin: 0 0 4px 0;
            padding-bottom: 2px;
        }
        .hyprbinds-page-title {
            font-size: 1.5em;
            font-weight: 700;
            letter-spacing: -0.03em;
        }
        .hyprbinds-page-hint {
            opacity: 0.52;
            font-size: 0.88em;
            line-height: 1.4;
            max-width: 58em;
        }
        .hyprbinds-toolbar {
            margin: 2px 0 4px 0;
        }
        .hyprbinds-toolbar entry {
            min-height: 28px;
        }
        .hyprbinds-toolbar button,
        .hyprbinds-header button {
            min-height: 26px;
            padding: 2px 9px;
        }
        .hyprbinds-header-toggle checkbutton {
            font-size: 0.86em;
        }

        list.boxed-list {
            margin-top: 4px;
            border-radius: 12px;
            background-color: alpha(currentColor, 0.025);
            border: 1px solid alpha(currentColor, 0.08);
            box-shadow: none;
        }
        list.boxed-list > row {
            min-height: 0;
            padding: 0;
            border-bottom: 1px solid alpha(currentColor, 0.05);
            transition: background-color 80ms ease;
        }
        list.boxed-list > row:last-child {
            border-bottom: none;
        }
        list.boxed-list > row:hover {
            background-color: alpha(currentColor, 0.045);
        }
        list.boxed-list > row:selected {
            background-color: @hypr_cyan_soft;
            box-shadow: inset 3px 0 0 0 @hypr_cyan;
        }

        .hyprbinds-row {
            padding: 0;
        }
        .hyprbinds-row-title {
            font-weight: 600;
            font-size: 1.0em;
            letter-spacing: -0.01em;
        }
        .hyprbinds-row-keys {
            font-weight: 600;
            font-size: 0.82em;
            letter-spacing: 0.02em;
            opacity: 0.9;
            padding: 2px 8px;
            border-radius: 6px;
            background-color: alpha(currentColor, 0.06);
            border: 1px solid alpha(currentColor, 0.1);
        }
        .hyprbinds-row-sub {
            opacity: 0.74;
            font-size: 0.88em;
        }
        .hyprbinds-row-body {
            opacity: 0.62;
            font-size: 0.86em;
            font-family: monospace;
            letter-spacing: -0.01em;
        }
        .hyprbinds-row-meta {
            opacity: 0.42;
            font-size: 0.76em;
            margin-top: 2px;
            letter-spacing: 0.01em;
        }

        .hyprbinds-section {
            font-size: 0.74em;
            font-weight: 700;
            text-transform: uppercase;
            letter-spacing: 0.08em;
            opacity: 0.48;
            margin-top: 12px;
            margin-bottom: 4px;
        }
        notebook.hyprbinds-bind-tabs {
            margin-top: 2px;
        }
        notebook.hyprbinds-bind-tabs > header {
            margin-bottom: 8px;
        }
        .hyprbinds-sticky-footer {
            background-color: alpha(currentColor, 0.028);
            border-top: 1px solid alpha(currentColor, 0.06);
        }

        /* Bind / form dialogs */
        window.hyprbinds-dialog {
            background-color: @window_bg_color;
        }
        .hyprbinds-dialog-section {
            background-color: alpha(currentColor, 0.03);
            border: 1px solid alpha(currentColor, 0.08);
            border-radius: 12px;
            padding: 14px 14px 16px 14px;
        }
        .hyprbinds-dialog-section-title {
            font-size: 0.72em;
            font-weight: 700;
            letter-spacing: 0.08em;
            text-transform: uppercase;
            opacity: 0.5;
            margin-bottom: 2px;
        }
        .hyprbinds-action-preview {
            padding: 6px 8px;
            border-radius: 8px;
            background-color: alpha(currentColor, 0.04);
            border: 1px solid alpha(currentColor, 0.07);
            font-size: 0.8em;
        }
        .hyprbinds-keys-preview {
            margin-top: 2px;
        }
        .hyprbinds-record-hint {
            color: @hypr_cyan;
            opacity: 0.9;
        }

        button.suggested-action {
            background-image: none;
            background-color: @hypr_cyan;
            color: #041018;
            font-weight: 600;
            border: none;
            text-shadow: none;
            box-shadow: none;
            min-height: 26px;
            padding: 2px 12px;
        }
        button.suggested-action:hover {
            background-color: #33ddff;
            color: #041018;
        }
        button.suggested-action:disabled {
            opacity: 0.45;
        }
        button.destructive-action {
            background-image: none;
            color: @error_color;
            min-height: 26px;
            padding: 2px 10px;
        }
        button.destructive-action:hover {
            background-color: alpha(@error_color, 0.12);
        }
        .error {
            color: @error_color;
        }
        .curve-graph {
            border-radius: 10px;
            border: 1px solid alpha(currentColor, 0.1);
            background-color: alpha(currentColor, 0.03);
        }

        .hyprbinds-health-summary {
            font-weight: 600;
            font-size: 0.92em;
            opacity: 0.85;
        }
        .hyprbinds-health-badge {
            font-size: 0.72em;
            font-weight: 700;
            letter-spacing: 0.04em;
            padding: 3px 8px;
            border-radius: 6px;
            min-width: 3.2em;
        }
        .hyprbinds-health-ok {
            background-color: alpha(@hypr_ok, 0.18);
            color: @hypr_ok;
        }
        .hyprbinds-health-warn {
            background-color: alpha(@hypr_warn, 0.18);
            color: @hypr_warn;
        }
        .hyprbinds-health-fail {
            background-color: alpha(@error_color, 0.18);
            color: @error_color;
        }
        .hyprbinds-health-info {
            background-color: @hypr_cyan_dim;
            color: @hypr_cyan;
            opacity: 0.95;
        }
        .hyprbinds-health-cmd {
            margin-top: 2px;
            opacity: 0.55;
        }
        list.hyprbinds-health-list > row.hyprbinds-health-category {
            background-color: transparent;
            border-bottom: none;
        }
        list.hyprbinds-health-list > row.hyprbinds-health-row-fail {
            box-shadow: inset 3px 0 0 0 alpha(@error_color, 0.85);
        }
        list.hyprbinds-health-list > row.hyprbinds-health-row-warn {
            box-shadow: inset 3px 0 0 0 alpha(@hypr_warn, 0.85);
        }

        .hyprbinds-log-error {
            background-color: alpha(@error_color, 0.18);
            color: @error_color;
        }
        .hyprbinds-log-warn {
            background-color: alpha(@hypr_warn, 0.18);
            color: @hypr_warn;
        }
        .hyprbinds-log-info {
            background-color: @hypr_cyan_dim;
            color: @hypr_cyan;
        }
        .hyprbinds-log-debug {
            background-color: alpha(currentColor, 0.08);
            opacity: 0.75;
        }

        .hyprbinds-conflict-badge {
            font-size: 0.68em;
            font-weight: 700;
            letter-spacing: 0.02em;
            padding: 1px 6px;
            border-radius: 4px;
            background-color: alpha(@error_color, 0.18);
            color: @error_color;
            text-transform: lowercase;
        }
        list.boxed-list > row.hyprbinds-conflict-row {
            box-shadow: inset 3px 0 0 0 alpha(@error_color, 0.85);
        }

        .hyprbinds-palette-root {
            background-color: @window_bg_color;
        }
        .hyprbinds-palette-search {
            min-height: 30px;
            font-size: 1.0em;
        }

        .hyprbinds-lookfeel {
            margin-end: 4px;
        }
        .hyprbinds-settings-card {
            background-color: alpha(currentColor, 0.035);
            border: 1px solid alpha(currentColor, 0.08);
            border-radius: 14px;
            padding: 6px 4px 8px 4px;
        }
        .hyprbinds-settings-card-title {
            font-size: 0.78em;
            font-weight: 700;
            letter-spacing: 0.06em;
            text-transform: uppercase;
            opacity: 0.55;
            margin: 8px 14px 6px 14px;
        }
        .hyprbinds-settings-row {
            padding: 10px 14px;
            min-height: 40px;
        }
        .hyprbinds-settings-title {
            font-weight: 600;
            font-size: 0.98em;
        }
        .hyprbinds-settings-sub {
            font-size: 0.82em;
            opacity: 0.58;
            line-height: 1.3;
        }

        .hyprbinds-waybar-preview {
            background-color: alpha(currentColor, 0.06);
            border: 1px solid alpha(currentColor, 0.1);
            border-radius: 12px;
            padding: 6px 10px;
            min-height: 42px;
        }
        .hyprbinds-waybar-pill {
            background-color: alpha(currentColor, 0.12);
            border-radius: 10px;
            padding: 4px 12px;
            font-size: 0.85em;
            font-weight: 600;
        }

        .hyprbinds-rofi-preview {
            background-color: alpha(currentColor, 0.04);
            border: 1px solid alpha(currentColor, 0.1);
            border-radius: 12px;
            padding: 16px;
            min-height: 160px;
        }
        .hyprbinds-rofi-theme-row {
            padding: 8px 4px;
        }
        .hyprbinds-rofi-swatch {
            border-radius: 6px;
            border: 1px solid alpha(currentColor, 0.25);
            min-width: 18px;
            min-height: 18px;
        }

        .hyprbinds-starship-preview {
            background-color: alpha(currentColor, 0.04);
            border: 1px solid alpha(currentColor, 0.1);
            border-radius: 12px;
            padding: 12px 14px;
            min-height: 64px;
        }
        .hyprbinds-starship-preview-text {
            font-family: monospace;
            font-size: 0.95em;
        }
        .hyprbinds-starship-editor {
            background-color: alpha(currentColor, 0.04);
            border: 1px solid alpha(currentColor, 0.1);
            border-radius: 10px;
            font-family: monospace;
            font-size: 0.9em;
        }

        .hyprbinds-via-frame {
            background-color: alpha(currentColor, 0.04);
            border: 1px solid alpha(currentColor, 0.1);
            border-radius: 12px;
            padding: 10px;
        }
        .hyprbinds-via-board {
            min-height: 200px;
        }
        button.hyprbinds-via-key {
            font-size: 0.72em;
            font-weight: 600;
            padding: 2px;
            border-radius: 7px;
            min-width: 0;
            min-height: 0;
        }
        button.hyprbinds-via-key-selected {
            outline: 2px solid @accent_color;
            outline-offset: -2px;
        }
        .hyprbinds-via-lighting {
            max-width: 720px;
        }
        .hyprbinds-via-color-swatch {
            min-width: 48px;
            min-height: 28px;
            border-radius: 6px;
            border: 1px solid alpha(currentColor, 0.2);
            background-color: #888;
        }
    "#;
    let provider = CssProvider::new();
    provider.load_from_string(css);
    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().expect("display"),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_USER,
    );
}

struct NavDestination {
    /// Gtk Stack child name.
    stack_id: &'static str,
    /// Canonical page id tracked in `current_page` (deep-link friendly).
    page_id: &'static str,
    /// Optional notebook tab within a hub page.
    tab: Option<u32>,
}

fn resolve_nav_destination(id: &str) -> NavDestination {
    match id {
        "home" | "overview" => NavDestination {
            stack_id: "overview",
            page_id: "overview",
            tab: None,
        },
        "rules" => NavDestination {
            stack_id: "rules",
            page_id: "rules-window",
            tab: None,
        },
        "rules-window" => NavDestination {
            stack_id: "rules",
            page_id: "rules-window",
            tab: Some(0),
        },
        "rules-workspace" => NavDestination {
            stack_id: "rules",
            page_id: "rules-workspace",
            tab: Some(1),
        },
        "rules-layer" => NavDestination {
            stack_id: "rules",
            page_id: "rules-layer",
            tab: Some(2),
        },
        "settings" => NavDestination {
            stack_id: "settings",
            page_id: "lookfeel",
            tab: None,
        },
        "lookfeel" => NavDestination {
            stack_id: "settings",
            page_id: "lookfeel",
            tab: Some(0),
        },
        "settings-config" => NavDestination {
            stack_id: "settings",
            page_id: "settings-config",
            tab: Some(1),
        },
        "settings-monitors" => NavDestination {
            stack_id: "settings",
            page_id: "settings-monitors",
            tab: Some(2),
        },
        "settings-devices" => NavDestination {
            stack_id: "settings",
            page_id: "settings-devices",
            tab: Some(3),
        },
        "settings-animations" => NavDestination {
            stack_id: "settings",
            page_id: "settings-animations",
            tab: Some(4),
        },
        "settings-curves" => NavDestination {
            stack_id: "settings",
            page_id: "settings-curves",
            tab: Some(5),
        },
        "settings-gestures" => NavDestination {
            stack_id: "settings",
            page_id: "settings-gestures",
            tab: Some(6),
        },
        "system" => NavDestination {
            stack_id: "system",
            page_id: "health",
            tab: None,
        },
        "health" => NavDestination {
            stack_id: "system",
            page_id: "health",
            tab: Some(0),
        },
        "screenshare" => NavDestination {
            stack_id: "system",
            page_id: "screenshare",
            tab: Some(1),
        },
        "audio" => NavDestination {
            stack_id: "system",
            page_id: "audio",
            tab: Some(2),
        },
        "logs" => NavDestination {
            stack_id: "system",
            page_id: "logs",
            tab: Some(3),
        },
        "import-export" => NavDestination {
            stack_id: "system",
            page_id: "import-export",
            tab: Some(4),
        },
        "binds" => NavDestination {
            stack_id: "binds",
            page_id: "binds",
            tab: None,
        },
        "via" => NavDestination {
            stack_id: "via",
            page_id: "via",
            tab: None,
        },
        "variables" => NavDestination {
            stack_id: "variables",
            page_id: "variables",
            tab: None,
        },
        "environment" => NavDestination {
            stack_id: "environment",
            page_id: "environment",
            tab: None,
        },
        "submaps" => NavDestination {
            stack_id: "submaps",
            page_id: "submaps",
            tab: None,
        },
        "startup" => NavDestination {
            stack_id: "startup",
            page_id: "startup",
            tab: None,
        },
        "wallpaper" => NavDestination {
            stack_id: "wallpaper",
            page_id: "wallpaper",
            tab: None,
        },
        "waybar" => NavDestination {
            stack_id: "waybar",
            page_id: "waybar",
            tab: None,
        },
        "rofi" => NavDestination {
            stack_id: "rofi",
            page_id: "rofi",
            tab: None,
        },
        "rofi-apps" => NavDestination {
            stack_id: "rofi-apps",
            page_id: "rofi-apps",
            tab: None,
        },
        "starship" => NavDestination {
            stack_id: "starship",
            page_id: "starship",
            tab: None,
        },
        _ => NavDestination {
            stack_id: "overview",
            page_id: "overview",
            tab: None,
        },
    }
}

fn hub_notebook(pages: &[(&GtkBox, &str)]) -> Notebook {
    let notebook = Notebook::new();
    notebook.add_css_class("hyprbinds-hub");
    notebook.set_scrollable(true);
    notebook.set_vexpand(true);
    notebook.set_hexpand(true);
    for (page, title) in pages {
        notebook.append_page(*page, Some(&Label::new(Some(title))));
    }
    notebook
}

fn hub_page_id(ids: &[&'static str], page: Option<u32>, fallback: &'static str) -> &'static str {
    page.and_then(|n| ids.get(n as usize).copied())
        .unwrap_or(fallback)
}

fn wire_hub_page_tracking(
    notebook: &Notebook,
    page_ids: &[&'static str],
    current_page: &Rc<RefCell<String>>,
    header_title: &Label,
    sidebar: &Rc<crate::nav::Sidebar>,
) {
    let page_ids: Vec<&'static str> = page_ids.to_vec();
    let current_page = Rc::clone(current_page);
    let header_title = header_title.clone();
    let sidebar = Rc::clone(sidebar);
    notebook.connect_switch_page(move |_, _, page_num| {
        if let Some(id) = page_ids.get(page_num as usize) {
            *current_page.borrow_mut() = (*id).to_string();
            header_title.set_text(crate::nav::page_chrome_title(id));
            sidebar.select(id);
        }
    });
}

fn clear_list(list: &ListBox) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
}

fn row_id(row: &ListBoxRow, prefix: &str) -> usize {
    row.widget_name()
        .strip_prefix(prefix)
        .and_then(|s| s.parse().ok())
        .unwrap_or(usize::MAX)
}

fn refresh_submap_filter_dropdown(dd: &DropDown, state: &Rc<RefCell<AppState>>) {
    let mut labels = vec!["All submaps".to_string(), "global".to_string()];
    if let Some(c) = state.borrow().collection.as_ref() {
        for s in &c.submaps {
            labels.push(s.name.clone());
        }
    }
    let refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
    let model = StringList::new(&refs);
    let prev = state.borrow().bind_submap_filter.clone();
    dd.set_model(Some(&model));
    let idx = match prev.as_str() {
        "" => 0u32,
        "__global__" => 1,
        name => labels
            .iter()
            .position(|l| l == name)
            .unwrap_or(0) as u32,
    };
    dd.set_selected(idx);
}

fn render_bind_list(state: &Rc<RefCell<AppState>>, tabs: &BindTabs, count_label: &Label) {
    tabs.clear_all();

    let state = state.borrow();
    let Some(collection) = state.collection.as_ref() else {
        count_label.set_text("0 binds");
        for (kind, _, label) in &tabs.pages {
            label.set_text(kind.label());
        }
        return;
    };

    let conflict_ids = crate::conflicts::conflicting_ids(&collection.binds);
    let conflict_count = conflict_ids.len();

    let filtered: Vec<&Keybind> = collection
        .binds
        .iter()
        .filter(|b| b.matches(&state.filter))
        .filter(|b| match state.bind_submap_filter.as_str() {
            "" => true,
            "__global__" => b.submap.is_empty(),
            name => b.submap == name,
        })
        .filter(|b| !state.conflicts_only || conflict_ids.contains(&b.id))
        .collect();

    let conflict_note = if conflict_count > 0 {
        format!(" · {conflict_count} conflicting")
    } else {
        String::new()
    };
    count_label.set_text(&format!(
        "{} / {} binds{conflict_note}",
        filtered.len(),
        collection.binds.len()
    ));

    for (kind, list, tab_label) in &tabs.pages {
        let binds: Vec<&Keybind> = filtered
            .iter()
            .copied()
            .filter(|b| kind.matches(&b.action))
            .collect();
        let n = binds.len();
        if n == 0 {
            tab_label.set_text(kind.label());
        } else {
            tab_label.set_text(&format!("{} ({n})", kind.label()));
        }
        for bind in binds {
            let keys_display = resolve_to_template(&bind.keys, &collection.variables);
            let is_conflict = conflict_ids.contains(&bind.id);
            list.append(&bind_row(bind, &keys_display, is_conflict));
        }
    }
}

fn render_var_list(state: &Rc<RefCell<AppState>>, list: &ListBox) {
    clear_list(list);
    let state = state.borrow();
    let Some(collection) = state.collection.as_ref() else {
        return;
    };

    let q = state.var_filter.to_lowercase();
    for var in &collection.variables {
        if !q.is_empty()
            && !var.name.to_lowercase().contains(&q)
            && !var.value.to_lowercase().contains(&q)
        {
            continue;
        }
        list.append(&variable_row(var));
    }
}

fn render_rule_list(state: &Rc<RefCell<AppState>>, list: &ListBox) {
    clear_list(list);
    let state = state.borrow();
    let Some(collection) = state.collection.as_ref() else {
        return;
    };

    for rule in &collection.window_rules {
        if rule.matches(&state.rule_filter) {
            list.append(&rule_row(rule, &collection.variables));
        }
    }
}

fn rule_row(rule: &WindowRule, vars: &[ConfigVariable]) -> ListBoxRow {
    let col = dialog::list_row_column();
    col.append(&dialog::row_title(&rule.display_name));
    col.append(&dialog::row_sub(&format!(
        "match  {}",
        rule.match_label_templated(vars)
    )));
    col.append(&dialog::row_body(&format!(
        "effects  {}",
        rule.effects_label_templated(vars)
    )));
    if !rule.source_file.is_empty() {
        col.append(&dialog::row_meta(&rule.source_label()));
    }

    ListBoxRow::builder()
        .child(&col)
        .activatable(true)
        .name(format!("rule-{}", rule.id))
        .build()
}

fn bind_row(bind: &Keybind, keys_display: &str, is_conflict: bool) -> ListBoxRow {
    let top = GtkBox::new(Orientation::Horizontal, 10);
    top.append(&dialog::row_title(&bind.name));
    if is_conflict {
        top.append(
            &Label::builder()
                .label("conflict")
                .tooltip_text("Conflicting chord in this submap")
                .css_classes(["hyprbinds-conflict-badge"])
                .valign(gtk4::Align::Center)
                .build(),
        );
    }
    let submap = bind.submap_label();
    if submap != "global" {
        top.append(
            &Label::builder()
                .label(&submap)
                .css_classes(["hyprbinds-row-meta"])
                .valign(gtk4::Align::Center)
                .build(),
        );
    }
    top.append(&dialog::row_keys(keys_display));

    let col = dialog::list_row_column_compact();
    col.append(&top);

    let mut tip_parts = Vec::new();
    if !bind.action.is_empty() {
        tip_parts.push(bind.action.clone());
    }
    let flags = bind.flags_label();
    if !flags.is_empty() {
        tip_parts.push(format!("flags: {flags}"));
    }
    let source = bind.source_label();
    if !source.is_empty() {
        tip_parts.push(source);
    }
    if !tip_parts.is_empty() {
        col.set_tooltip_text(Some(&tip_parts.join("\n")));
    }

    let row = ListBoxRow::builder()
        .child(&col)
        .activatable(true)
        .name(format!("bind-{}", bind.id))
        .build();
    if is_conflict {
        row.add_css_class("hyprbinds-conflict-row");
    }
    row
}

fn variable_row(var: &ConfigVariable) -> ListBoxRow {
    let col = dialog::list_row_column();
    col.append(&dialog::row_title(&format!("{{{{{}}}}}", var.name)));
    col.append(&dialog::row_sub(&format!("\"{}\"", var.value)));

    if !var.source_file.is_empty() {
        let file = std::path::Path::new(&var.source_file)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(var.source_file.as_str());
        col.append(&dialog::row_meta(&format!("{file}:{}", var.source_line)));
    }

    ListBoxRow::builder()
        .child(&col)
        .activatable(true)
        .name(format!("var-{}", var.name))
        .build()
}

fn show_variable_dialog<F>(
    parent: &impl IsA<Window>,
    existing: Option<ConfigVariable>,
    config_path_for_add: Option<PathBuf>,
    related_files: Vec<PathBuf>,
    existing_names: Vec<String>,
    realtime: Rc<Cell<bool>>,
    on_saved: F,
) where
    F: Fn(String) + 'static,
{
    let is_new = existing.is_none();
    let editor = Window::builder()
        .title(if is_new {
            "Add variable"
        } else {
            "Edit variable"
        })
        .transient_for(parent)
        .modal(true)
        .default_width(520)
        .default_height(380)
        .resizable(true)
        .build();

    let name_entry = Entry::builder()
        .text(existing.as_ref().map(|v| v.name.as_str()).unwrap_or(""))
        .placeholder_text("mainMod")
        .hexpand(true)
        .build();
    let value_entry = Entry::builder()
        .text(existing.as_ref().map(|v| v.value.as_str()).unwrap_or(""))
        .placeholder_text("SUPER")
        .hexpand(true)
        .build();

    let hint = Label::builder()
        .label(if is_new {
            "Adds a new variable into a managed section at the end of hyprland.lua. Existing config is left unchanged."
        } else {
            "Saving updates the declaration. Renaming rewrites all identifier occurrences in related config files (including keybinds). Realtime autosaves ~0.8s after typing."
        })
        .halign(gtk4::Align::Start)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();

    let status = Label::builder()
        .label("")
        .halign(gtk4::Align::Start)
        .wrap(true)
        .css_classes(["dim-label", "caption"])
        .build();

    let cancel = Button::builder().label("Cancel").build();
    let save = Button::builder()
        .label("Save")
        .css_classes(["suggested-action"])
        .build();
    let buttons = dialog::action_buttons(&cancel, &save);

    let content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(14)
        .margin_top(20)
        .margin_bottom(16)
        .margin_start(20)
        .margin_end(20)
        .build();
    content.append(&field_block("Name", &name_entry));
    content.append(&field_block("Value", &value_entry));
    content.append(&hint);
    dialog::mount_sticky_dialog(&editor, &content, Some(&status), &buttons);

    cancel.connect_clicked({
        let editor = editor.clone();
        move |_| editor.close()
    });

    let on_saved = Rc::new(on_saved);
    let try_save: Rc<dyn Fn(bool)> = {
        let editor = editor.clone();
        let status = status.clone();
        let name_entry = name_entry.clone();
        let value_entry = value_entry.clone();
        let existing = existing.clone();
        let config_path_for_add = config_path_for_add.clone();
        let related_files = related_files.clone();
        let existing_names = existing_names.clone();
        let on_saved = Rc::clone(&on_saved);
        Rc::new(move |close_on_success: bool| {
            let name = name_entry.text().to_string();
            let value = value_entry.text().to_string();
            let result = if let Some(var) = existing.as_ref() {
                writer::save_variable(var, &name, &value, &related_files, &existing_names).map(
                    |r| {
                        let extra = if r.updated_files.len() > 1 {
                            format!(" (updated {} files)", r.updated_files.len())
                        } else {
                            String::new()
                        };
                        let msg = format!(
                            "Saved variable `{name}` = \"{value}\" in {}{extra}",
                            r.path
                        );
                        (r, msg)
                    },
                )
            } else if let Some(path) = config_path_for_add.as_ref() {
                writer::add_variable(path, &name, &value).map(|r| {
                    let msg = format!("Added variable `{name}` = \"{value}\" to {}", r.path);
                    (r, msg)
                })
            } else {
                Err(writer::WriteError::Invalid(
                    "missing config path for new variable".into(),
                ))
            };

            match result {
                Ok((_r, msg)) => {
                    on_saved(msg);
                    if close_on_success {
                        editor.close();
                    }
                }
                Err(err) => {
                    status.set_text(&format!("Save failed: {err}"));
                    status.set_css_classes(&["error"]);
                }
            }
        })
    };
    save.connect_clicked({
        let try_save = Rc::clone(&try_save);
        move |_| try_save(true)
    });
    if !is_new {
        extra_ui::wire_dialog_autosave(&realtime, &try_save, &[&name_entry, &value_entry], &[]);
    }

    editor.present();
}

fn show_edit_dialog<F>(
    parent: &impl IsA<Window>,
    bind: Option<Keybind>,
    shared_source: bool,
    config_path_for_add: Option<PathBuf>,
    variables: Vec<ConfigVariable>,
    submap_names: Vec<String>,
    preferred_submap: String,
    realtime: Rc<Cell<bool>>,
    on_saved: F,
) where
    F: Fn(String) + 'static,
{
    let is_new = bind.is_none();
    let editor = Window::builder()
        .title(if is_new { "Add bind" } else { "Edit bind" })
        .transient_for(parent)
        .modal(true)
        .default_width(760)
        .default_height(480)
        .resizable(true)
        .build();
    editor.add_css_class("hyprbinds-dialog");

    let default_name = if is_new {
        "untitled-new".to_string()
    } else {
        bind.as_ref().map(|b| b.name.clone()).unwrap_or_default()
    };
    let keys_template = bind
        .as_ref()
        .map(|b| resolve_to_template(&b.keys, &variables))
        .unwrap_or_else(|| "{{mainMod}} + ".to_string());
    let action_template = bind
        .as_ref()
        .map(|b| action_to_template(&b.action, &variables))
        .unwrap_or_else(|| "hl.dsp.exec_cmd(\"\")".to_string());

    let name_entry = Entry::builder().text(&default_name).hexpand(true).build();
    let keys_entry = Entry::builder()
        .text(&keys_template)
        .hexpand(true)
        .placeholder_text("SUPER + Q")
        .build();

    let current_submap = bind
        .as_ref()
        .map(|b| b.submap.clone())
        .unwrap_or_else(|| {
            if preferred_submap == "__global__" || preferred_submap.is_empty() {
                String::new()
            } else {
                preferred_submap
            }
        });
    let mut submap_labels = vec!["global (default)".to_string()];
    submap_labels.extend(submap_names);
    if !current_submap.is_empty() && !submap_labels.iter().any(|s| s == &current_submap) {
        submap_labels.push(current_submap.clone());
    }
    let submap_refs: Vec<&str> = submap_labels.iter().map(|s| s.as_str()).collect();
    let submap_dd = DropDown::from_strings(&submap_refs);
    if let Some(idx) = submap_labels.iter().position(|s| {
        if current_submap.is_empty() {
            s.starts_with("global")
        } else {
            s == &current_submap
        }
    }) {
        submap_dd.set_selected(idx as u32);
    }
    // Existing binds keep their source submap (in-place edit); only new binds pick a target.
    submap_dd.set_sensitive(is_new);
    if !is_new {
        submap_dd.set_tooltip_text(Some("Submap is fixed for existing binds"));
    }

    let action_builder = ActionBuilder::new(&action_template);

    let recording = Rc::new(Cell::new(false));
    let record_btn = Button::builder()
        .label("Record")
        .tooltip_text("Click, then press the shortcut (Esc cancels)")
        .css_classes(["hyprbinds-record-btn"])
        .build();
    let record_hint = Label::builder()
        .label("Listening — press the shortcut (Esc cancels)")
        .halign(gtk4::Align::Start)
        .wrap(true)
        .visible(false)
        .css_classes(["dim-label", "caption", "hyprbinds-record-hint"])
        .build();

    let preview = Label::builder()
        .label(&preview_text(&keys_template, &variables))
        .halign(gtk4::Align::Start)
        .wrap(true)
        .css_classes(["dim-label", "caption", "hyprbinds-keys-preview"])
        .build();
    preview.set_visible(!preview_text(&keys_template, &variables).is_empty());

    keys_entry.connect_changed({
        let variables = variables.clone();
        let preview = preview.clone();
        move |entry| {
            let text = preview_text(&entry.text(), &variables);
            preview.set_text(&text);
            preview.set_visible(!text.is_empty());
        }
    });

    let (status_text, status_tooltip) = if is_new {
        (
            "Saved to managed binds / submaps".to_string(),
            Some(
                "New binds go into managed-binds (global) or managed-submaps when a submap is selected."
                    .to_string(),
            ),
        )
    } else if shared_source {
        (
            "Shared source — save writes a managed override".to_string(),
            Some(
                "This bind comes from a shared/looped source line. Saving will append an unbind+bind override in the managed section."
                    .to_string(),
            ),
        )
    } else {
        (
            format!(
                "{} · {}",
                bind.as_ref().map(|b| b.source_label()).unwrap_or_default(),
                if current_submap.is_empty() {
                    "global"
                } else {
                    current_submap.as_str()
                }
            ),
            None,
        )
    };

    let status = Label::builder()
        .label(&status_text)
        .halign(gtk4::Align::Start)
        .wrap(true)
        .ellipsize(gtk4::pango::EllipsizeMode::End)
        .css_classes(["dim-label", "caption"])
        .build();
    if let Some(tip) = status_tooltip.as_deref() {
        status.set_tooltip_text(Some(tip));
    }

    let cancel = Button::builder().label("Cancel").build();
    let save = Button::builder()
        .label(if is_new { "Add" } else { "Save" })
        .css_classes(["suggested-action"])
        .build();

    let buttons = dialog::action_buttons(&cancel, &save);

    let content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .margin_top(14)
        .margin_bottom(6)
        .margin_start(14)
        .margin_end(14)
        .css_classes(["hyprbinds-dialog-body"])
        .build();

    let columns = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(14)
        .vexpand(true)
        .build();

    let left = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .hexpand(true)
        .width_request(260)
        .css_classes(["hyprbinds-dialog-section"])
        .build();
    left.append(
        &Label::builder()
            .label("Binding")
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-dialog-section-title"])
            .build(),
    );
    left.append(&field_block("Name", &name_entry));
    {
        let row = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .build();
        row.append(
            &Label::builder()
                .label("Submap")
                .halign(gtk4::Align::Start)
                .css_classes(["hyprbinds-field-label"])
                .build(),
        );
        row.append(&submap_dd);
        left.append(&row);
    }

    let keys_row = GtkBox::new(Orientation::Horizontal, 8);
    keys_row.append(&keys_entry);
    keys_row.append(&record_btn);
    let keys_block = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .build();
    keys_block.append(
        &Label::builder()
            .label("Keys")
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-field-label"])
            .build(),
    );
    keys_block.append(&keys_row);
    keys_block.append(&record_hint);
    keys_block.append(&preview);
    left.append(&keys_block);

    let right = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .hexpand(true)
        .vexpand(true)
        .width_request(380)
        .css_classes(["hyprbinds-dialog-section"])
        .build();
    right.append(
        &Label::builder()
            .label("Action")
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-dialog-section-title"])
            .build(),
    );
    let action_widget = action_builder.widget();
    action_widget.set_hexpand(true);
    action_widget.set_vexpand(true);
    right.append(action_widget);

    columns.append(&left);
    columns.append(&right);
    content.append(&columns);
    dialog::mount_sticky_dialog(&editor, &content, Some(&status), &buttons);

    let key_controller = EventControllerKey::new();
    key_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    key_controller.connect_key_pressed({
        let keys_entry = keys_entry.clone();
        let record_btn = record_btn.clone();
        let recording = Rc::clone(&recording);
        let record_hint = record_hint.clone();
        let preview = preview.clone();
        let variables = variables.clone();
        move |_, key, _, modifiers| {
            if !recording.get() {
                return glib::Propagation::Proceed;
            }
            if key == Key::Escape {
                recording.set(false);
                record_btn.set_label("Record");
                record_hint.set_visible(false);
                return glib::Propagation::Stop;
            }
            if let Some(chord) = keys::event_to_hypr_keys(key, modifiers) {
                let templated = resolve_to_template(&chord, &variables);
                keys_entry.set_text(&templated);
                let text = preview_text(&templated, &variables);
                preview.set_text(&text);
                preview.set_visible(!text.is_empty());
                recording.set(false);
                record_btn.set_label("Record");
                record_hint.set_visible(false);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Stop
        }
    });
    editor.add_controller(key_controller);

    record_btn.connect_clicked({
        let recording = Rc::clone(&recording);
        let record_hint = record_hint.clone();
        move |btn| {
            let now = !recording.get();
            recording.set(now);
            if now {
                btn.set_label("Listening…");
                record_hint.set_visible(true);
            } else {
                btn.set_label("Record");
                record_hint.set_visible(false);
            }
        }
    });

    cancel.connect_clicked({
        let editor = editor.clone();
        move |_| editor.close()
    });

    let on_saved = Rc::new(on_saved);
    let try_save: Rc<dyn Fn(bool)> = {
        let editor = editor.clone();
        let status = status.clone();
        let variables = variables.clone();
        let name_entry = name_entry.clone();
        let keys_entry = keys_entry.clone();
        let action_builder = action_builder.clone();
        let bind = bind.clone();
        let config_path_for_add = config_path_for_add.clone();
        let on_saved = Rc::clone(&on_saved);
        let submap_dd = submap_dd.clone();
        let submap_labels = submap_labels.clone();
        Rc::new(move |close_on_success: bool| {
            let name = name_entry.text().to_string();
            let keys_raw = keys_entry.text().to_string();
            let keys = normalize_keys_input(&keys_raw);
            let action = action_builder.action_text();

            if let Err(err) = variables::expand_template(&keys, &variables) {
                if close_on_success {
                    status.set_text(&format!("Keys error: {err}"));
                    status.set_css_classes(&["error"]);
                }
                return;
            }

            let submap = {
                let idx = submap_dd.selected() as usize;
                if idx == 0 {
                    String::new()
                } else {
                    submap_labels.get(idx).cloned().unwrap_or_default()
                }
            };

            let result = if let Some(bind) = bind.as_ref() {
                writer::save_bind(bind, &name, &keys, &action, shared_source).map(|r| {
                    let msg = match r.mode {
                        WriteMode::InPlace => format!("Saved bind “{name}” in {}", r.path),
                        WriteMode::Materialized => {
                            format!("Saved bind “{name}” as managed override in {}", r.path)
                        }
                        WriteMode::Appended => format!("Added bind “{name}” in {}", r.path),
                    };
                    (r, msg)
                })
            } else if let Some(path) = config_path_for_add.as_ref() {
                writer::add_bind(path, &name, &keys, &action, &submap).map(|r| {
                    let where_ = if submap.is_empty() {
                        "managed-binds".to_string()
                    } else {
                        format!("submap `{submap}`")
                    };
                    let msg = format!(
                        "Added bind “{name}” to {where_} in {} (backup: {}.hyprbinds.bak)",
                        r.path, r.path
                    );
                    (r, msg)
                })
            } else {
                Err(writer::WriteError::Invalid(
                    "missing config path for new bind".into(),
                ))
            };

            match result {
                Ok((_r, msg)) => {
                    on_saved(msg);
                    if close_on_success {
                        editor.close();
                    }
                }
                Err(err) => {
                    status.set_text(&format!("Save failed: {err}"));
                    status.set_css_classes(&["error"]);
                }
            }
        })
    };
    save.connect_clicked({
        let try_save = Rc::clone(&try_save);
        move |_| try_save(true)
    });

    if !is_new {
        extra_ui::wire_dialog_autosave(
            &realtime,
            &try_save,
            &[&name_entry, &keys_entry],
            &[],
        );
        // Action builder changes (dropdowns / fields) also count as typing.
        {
            let ready = Rc::new(Cell::new(false));
            let debouncer = Debouncer::new();
            let schedule = {
                let ready = Rc::clone(&ready);
                let realtime = Rc::clone(&realtime);
                let debouncer = debouncer.clone();
                let try_save = Rc::clone(&try_save);
                Rc::new(move || {
                    if !ready.get() || !realtime.get() {
                        return;
                    }
                    let realtime = Rc::clone(&realtime);
                    let try_save = Rc::clone(&try_save);
                    debouncer.schedule(move || {
                        if realtime.get() {
                            try_save(false);
                        }
                    });
                })
            };
            action_builder.connect_changed({
                let schedule = Rc::clone(&schedule);
                move || schedule()
            });
            glib::idle_add_local_once(move || ready.set(true));
        }
    }

    editor.present();
}

fn attach_variable_helpers(keys_entry: &Entry, variables: &[ConfigVariable]) {
    if variables.is_empty() {
        return;
    }
    keys_entry.set_tooltip_text(Some(
        "Type {{ to start a variable, then click a suggestion chip.",
    ));
}

fn rebuild_suggestions<F>(container: &GtkBox, variables: &[ConfigVariable], prefix: &str, on_pick: F)
where
    F: Fn(String) + Clone + 'static,
{
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    for var in filter_variables(variables, prefix) {
        let label = format!("{} (“{}”)", var.token(), var.value);
        let btn = Button::builder()
            .label(&label)
            .tooltip_text(&var.suggestion_label())
            .css_classes(["flat"])
            .build();
        let token = var.token();
        let on_pick = on_pick.clone();
        btn.connect_clicked(move |_| on_pick(token.clone()));
        container.append(&btn);
    }
}

fn insert_at_cursor(entry: &Entry, token: &str) {
    let text = entry.text().to_string();
    let pos = entry.position().max(0) as usize;
    let pos = pos.min(text.len());
    let mut next = String::new();
    next.push_str(&text[..pos]);
    next.push_str(token);
    next.push_str(&text[pos..]);
    entry.set_text(&next);
    entry.set_position((pos + token.len()) as i32);
}

fn complete_or_insert_var(entry: &Entry, token: &str) {
    let text = entry.text().to_string();
    if let Some(start) = text.rfind("{{") {
        let after = &text[start + 2..];
        if !after.contains("}}") {
            let mut next = String::new();
            next.push_str(&text[..start]);
            next.push_str(token);
            entry.set_text(&next);
            entry.set_position(next.len() as i32);
            return;
        }
    }
    insert_at_cursor(entry, token);
}

fn preview_text(keys_template: &str, variables: &[ConfigVariable]) -> String {
    match variables::expand_template(keys_template, variables) {
        Ok(expanded) if expanded != keys_template => format!("Resolves to {expanded}"),
        Ok(_) => String::new(),
        Err(err) => format!("Preview: {err}"),
    }
}

fn field_block(title: &str, entry: &Entry) -> GtkBox {
    let block = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .build();
    block.append(
        &Label::builder()
            .label(title)
            .halign(gtk4::Align::Start)
            .css_classes(["hyprbinds-field-label"])
            .build(),
    );
    block.append(entry);
    block
}

fn format_error(err: &ConfigError) -> String {
    match err {
        ConfigError::NotFound(path) => format!(
            "Config not found at {}. Create ~/.config/hypr/hyprland.lua first.",
            path.display()
        ),
        ConfigError::Spawn(_) => {
            "Failed to run `lua`. Install Lua to collect binds from your config.".to_string()
        }
        other => other.to_string(),
    }
}

fn show_rule_dialog<F>(
    parent: &impl IsA<Window>,
    existing: Option<WindowRule>,
    config_path_for_add: Option<PathBuf>,
    variables: Vec<ConfigVariable>,
    realtime: Rc<Cell<bool>>,
    on_saved: F,
) where
    F: Fn(String) + 'static,
{
    let is_new = existing.is_none();
    let editor = Window::builder()
        .title(if is_new {
            "Add window rule"
        } else {
            "Edit window rule"
        })
        .transient_for(parent)
        .modal(true)
        .default_width(640)
        .default_height(720)
        .build();

    let name_entry = Entry::builder()
        .text(existing.as_ref().map(|r| r.name.as_str()).unwrap_or(""))
        .placeholder_text("optional-rule-name")
        .hexpand(true)
        .build();

    let class_entry = Entry::builder()
        .text(
            existing
                .as_ref()
                .and_then(|r| r.match_string_template("class", &variables))
                .unwrap_or_default(),
        )
        .placeholder_text("^{{terminal}}$  or  ^firefox$")
        .hexpand(true)
        .build();
    let pick_class_btn = Button::builder().label("From running…").build();
    let class_row = GtkBox::new(Orientation::Horizontal, 8);
    class_row.append(&class_entry);
    class_row.append(&pick_class_btn);

    let title_entry = Entry::builder()
        .text(
            existing
                .as_ref()
                .and_then(|r| r.match_string_template("title", &variables))
                .unwrap_or_default(),
        )
        .placeholder_text("optional title regex / {{var}}")
        .hexpand(true)
        .build();

    let match_bools: Vec<(String, CheckButton)> = BOOL_MATCH_PROPS
        .iter()
        .map(|(key, label)| {
            let active = existing
                .as_ref()
                .and_then(|r| r.match_bool(key))
                .unwrap_or(false);
            let cb = CheckButton::builder().label(*label).active(active).build();
            ((*key).to_string(), cb)
        })
        .collect();

    let match_bool_box = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .homogeneous(false)
        .build();
    for (_, cb) in &match_bools {
        match_bool_box.append(cb);
    }

    let effect_bools: Vec<(String, CheckButton)> = BOOL_EFFECTS
        .iter()
        .map(|(key, label)| {
            let active = existing
                .as_ref()
                .and_then(|r| r.effect_bool(key))
                .unwrap_or(false);
            let cb = CheckButton::builder().label(*label).active(active).build();
            ((*key).to_string(), cb)
        })
        .collect();

    let effect_bool_box = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .build();
    let mut row = GtkBox::new(Orientation::Horizontal, 12);
    for (i, (_, cb)) in effect_bools.iter().enumerate() {
        if i > 0 && i % 3 == 0 {
            effect_bool_box.append(&row);
            row = GtkBox::new(Orientation::Horizontal, 12);
        }
        row.append(cb);
    }
    effect_bool_box.append(&row);

    let workspace_entry = Entry::builder()
        .text(
            existing
                .as_ref()
                .and_then(|r| r.effect_string_template("workspace", &variables))
                .unwrap_or_default(),
        )
        .placeholder_text("2  or  {{workspaceVar}}")
        .hexpand(true)
        .build();
    let monitor_entry = Entry::builder()
        .text(
            existing
                .as_ref()
                .and_then(|r| r.effect_string_template("monitor", &variables))
                .unwrap_or_default(),
        )
        .placeholder_text("DP-1")
        .hexpand(true)
        .build();
    let move_entry = Entry::builder()
        .text(
            existing
                .as_ref()
                .and_then(|r| r.effect_string_template("move", &variables))
                .unwrap_or_default(),
        )
        .placeholder_text("100 200  or  20 monitor_h-120")
        .hexpand(true)
        .build();
    let size_entry = Entry::builder()
        .text(
            existing
                .as_ref()
                .and_then(|r| r.effect_string_template("size", &variables))
                .unwrap_or_default(),
        )
        .placeholder_text("800 600")
        .hexpand(true)
        .build();
    let opacity_entry = Entry::builder()
        .text(
            existing
                .as_ref()
                .and_then(|r| r.effect_string_template("opacity", &variables))
                .unwrap_or_default(),
        )
        .placeholder_text("0.9 0.7")
        .hexpand(true)
        .build();
    let suppress_entry = Entry::builder()
        .text(
            existing
                .as_ref()
                .and_then(|r| r.effect_string_template("suppress_event", &variables))
                .unwrap_or_default(),
        )
        .placeholder_text("maximize")
        .hexpand(true)
        .build();
    let border_entry = Entry::builder()
        .text(
            existing
                .as_ref()
                .and_then(|r| r.effect_string("border_size"))
                .unwrap_or_default(),
        )
        .placeholder_text("2")
        .hexpand(true)
        .build();
    let rounding_entry = Entry::builder()
        .text(
            existing
                .as_ref()
                .and_then(|r| r.effect_string("rounding"))
                .unwrap_or_default(),
        )
        .placeholder_text("10")
        .hexpand(true)
        .build();

    let var_hint = Label::builder()
        .label(if variables.is_empty() {
            "No config variables loaded. String fields accept plain values."
        } else {
            "Type {{ to insert a variable, or click a chip below. Saves as Lua concat/idents (e.g. {{terminal}} → terminal)."
        })
        .halign(gtk4::Align::Start)
        .wrap(true)
        .css_classes(["dim-label", "caption"])
        .build();
    let suggestions = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .build();
    let active_entry: Rc<RefCell<Entry>> = Rc::new(RefCell::new(class_entry.clone()));
    let preview = Label::builder()
        .label("")
        .halign(gtk4::Align::Start)
        .wrap(true)
        .css_classes(["dim-label", "caption"])
        .build();

    let status = Label::builder()
        .label(if is_new {
            "New rules are appended under a managed section at the end of hyprland.lua. Your existing rules stay untouched."
        } else {
            "Saving replaces this hl.window_rule(…) call in place. A .hyprbinds.bak backup is written first."
        })
        .halign(gtk4::Align::Start)
        .wrap(true)
        .css_classes(["dim-label"])
        .build();

    let cancel = Button::builder().label("Cancel").build();
    let save = Button::builder()
        .label(if is_new { "Add" } else { "Save" })
        .css_classes(["suggested-action"])
        .build();
    let actions = dialog::action_buttons(&cancel, &save);

    let form = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .build();
    form.append(&field_block("Name (optional)", &name_entry));
    form.append(
        &Label::builder()
            .label("Match")
            .halign(gtk4::Align::Start)
            .css_classes(["heading"])
            .build(),
    );
    {
        let block = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .build();
        block.append(
            &Label::builder()
                .label("Class (regex)")
                .halign(gtk4::Align::Start)
                .build(),
        );
        block.append(&class_row);
        form.append(&block);
    }
    form.append(&field_block("Title (regex, optional)", &title_entry));
    form.append(
        &Label::builder()
            .label("Also match when…")
            .halign(gtk4::Align::Start)
            .build(),
    );
    form.append(&match_bool_box);
    form.append(
        &Label::builder()
            .label("Effects")
            .halign(gtk4::Align::Start)
            .css_classes(["heading"])
            .build(),
    );
    form.append(&effect_bool_box);
    form.append(&field_block("Workspace", &workspace_entry));
    form.append(&field_block("Monitor", &monitor_entry));
    form.append(&field_block("Move", &move_entry));
    form.append(&field_block("Size", &size_entry));
    form.append(&field_block("Opacity", &opacity_entry));
    form.append(&field_block("Suppress event", &suppress_entry));
    form.append(&field_block("Border size", &border_entry));
    form.append(&field_block("Rounding", &rounding_entry));
    form.append(&var_hint);
    form.append(&suggestions);
    form.append(&preview);
    dialog::mount_sticky_dialog(&editor, &form, Some(&status), &actions);

    // Variable chips + preview for string fields.
    let wire_var_entry = |entry: &Entry| {
        attach_variable_helpers(entry, &variables);
        entry.connect_notify_local(Some("has-focus"), {
            let active_entry = Rc::clone(&active_entry);
            let entry = entry.clone();
            move |e, _| {
                if e.has_focus() {
                    *active_entry.borrow_mut() = entry.clone();
                }
            }
        });
        entry.connect_changed({
            let variables = variables.clone();
            let suggestions = suggestions.clone();
            let preview = preview.clone();
            let active_entry = Rc::clone(&active_entry);
            let entry = entry.clone();
            move |e| {
                *active_entry.borrow_mut() = entry.clone();
                let text = e.text().to_string();
                let prefix = unfinished_var_prefix(&text).unwrap_or_default();
                rebuild_suggestions(&suggestions, &variables, &prefix, {
                    let active_entry = Rc::clone(&active_entry);
                    let preview = preview.clone();
                    let variables = variables.clone();
                    move |token| {
                        let entry = active_entry.borrow().clone();
                        complete_or_insert_var(&entry, &token);
                        preview.set_text(&rule_field_preview(&entry.text(), &variables));
                    }
                });
                preview.set_text(&rule_field_preview(&text, &variables));
            }
        });
    };
    for entry in [
        &class_entry,
        &title_entry,
        &workspace_entry,
        &monitor_entry,
        &move_entry,
        &size_entry,
        &opacity_entry,
        &suppress_entry,
    ] {
        wire_var_entry(entry);
    }
    rebuild_suggestions(&suggestions, &variables, "", {
        let active_entry = Rc::clone(&active_entry);
        let preview = preview.clone();
        let variables = variables.clone();
        move |token| {
            let entry = active_entry.borrow().clone();
            complete_or_insert_var(&entry, &token);
            preview.set_text(&rule_field_preview(&entry.text(), &variables));
        }
    });
    preview.set_text(&rule_field_preview(&class_entry.text(), &variables));

    pick_class_btn.connect_clicked({
        let editor = editor.clone();
        let class_entry = class_entry.clone();
        move |_| {
            show_class_picker(&editor, {
                let class_entry = class_entry.clone();
                move |class| {
                    class_entry.set_text(&window_rules::exact_class_regex(&class));
                }
            });
        }
    });

    cancel.connect_clicked({
        let editor = editor.clone();
        move |_| editor.close()
    });

    let on_saved = Rc::new(on_saved);
    let try_save: Rc<dyn Fn(bool)> = {
        let editor = editor.clone();
        let status = status.clone();
        let existing = existing.clone();
        let config_path_for_add = config_path_for_add.clone();
        let match_bools = match_bools.clone();
        let effect_bools = effect_bools.clone();
        let variables = variables.clone();
        let name_entry = name_entry.clone();
        let class_entry = class_entry.clone();
        let title_entry = title_entry.clone();
        let workspace_entry = workspace_entry.clone();
        let monitor_entry = monitor_entry.clone();
        let move_entry = move_entry.clone();
        let size_entry = size_entry.clone();
        let opacity_entry = opacity_entry.clone();
        let suppress_entry = suppress_entry.clone();
        let border_entry = border_entry.clone();
        let rounding_entry = rounding_entry.clone();
        let on_saved = Rc::clone(&on_saved);
        Rc::new(move |close_on_success: bool| {
            let name = name_entry.text().to_string();
            let mut match_props = BTreeMap::new();

            for (key, raw) in [
                ("class", class_entry.text().to_string()),
                ("title", title_entry.text().to_string()),
            ] {
                match push_templated_field(&mut match_props, key, &raw, &variables) {
                    Ok(()) => {}
                    Err(err) => {
                        if close_on_success {
                            status.set_text(&format!("{key}: {err}"));
                            status.set_css_classes(&["error"]);
                        }
                        return;
                    }
                }
            }

            for (key, cb) in &match_bools {
                if cb.is_active() {
                    match_props.insert(key.clone(), json!(true));
                } else if let Some(rule) = existing.as_ref() {
                    if rule.match_bool(key) == Some(false) {
                        match_props.insert(key.clone(), json!(false));
                    }
                }
            }

            let mut effects = BTreeMap::new();
            for (key, cb) in &effect_bools {
                if cb.is_active() {
                    effects.insert(key.clone(), json!(true));
                }
            }
            if let Some(rule) = existing.as_ref() {
                for (key, val) in &rule.effects {
                    if val.as_bool() == Some(false)
                        && BOOL_EFFECTS.iter().any(|(k, _)| *k == key.as_str())
                        && !effects.contains_key(key)
                        && effect_bools
                            .iter()
                            .any(|(k, cb)| k == key && !cb.is_active())
                    {
                        effects.insert(key.clone(), json!(false));
                    }
                    if !BOOL_EFFECTS.iter().any(|(k, _)| *k == key.as_str())
                        && ![
                            "workspace",
                            "monitor",
                            "move",
                            "size",
                            "opacity",
                            "suppress_event",
                            "border_size",
                            "rounding",
                        ]
                        .contains(&key.as_str())
                    {
                        effects.insert(key.clone(), val.clone());
                    }
                }
            }

            for (key, raw) in [
                ("workspace", workspace_entry.text().to_string()),
                ("monitor", monitor_entry.text().to_string()),
                ("move", move_entry.text().to_string()),
                ("size", size_entry.text().to_string()),
                ("opacity", opacity_entry.text().to_string()),
                ("suppress_event", suppress_entry.text().to_string()),
            ] {
                match push_templated_field(&mut effects, key, &raw, &variables) {
                    Ok(()) => {}
                    Err(err) => {
                        if close_on_success {
                            status.set_text(&format!("{key}: {err}"));
                            status.set_css_classes(&["error"]);
                        }
                        return;
                    }
                }
            }
            push_int_or_string_effect(&mut effects, "border_size", &border_entry.text());
            push_int_or_string_effect(&mut effects, "rounding", &rounding_entry.text());

            if match_props.is_empty() {
                if close_on_success {
                    status.set_text("Add at least one match property (e.g. class).");
                    status.set_css_classes(&["error"]);
                }
                return;
            }
            if effects.is_empty() {
                if close_on_success {
                    status.set_text("Add at least one effect.");
                    status.set_css_classes(&["error"]);
                }
                return;
            }

            let result = if let Some(rule) = existing.as_ref() {
                writer::save_window_rule(rule, &name, &match_props, &effects).map(|r| {
                    format!("Saved window rule “{}” in {}", rule.display_name, r.path)
                })
            } else if let Some(path) = config_path_for_add.as_ref() {
                writer::add_window_rule(path, &name, &match_props, &effects).map(|r| {
                    format!("Added window rule to managed section in {}", r.path)
                })
            } else {
                Err(writer::WriteError::Invalid(
                    "missing config path for new window rule".into(),
                ))
            };

            match result {
                Ok(msg) => {
                    on_saved(msg);
                    if close_on_success {
                        editor.close();
                    }
                }
                Err(err) => {
                    status.set_text(&format!("Save failed: {err}"));
                    status.set_css_classes(&["error"]);
                }
            }
        })
    };
    save.connect_clicked({
        let try_save = Rc::clone(&try_save);
        move |_| try_save(true)
    });

    if !is_new {
        let text_entries = [
            &name_entry,
            &class_entry,
            &title_entry,
            &workspace_entry,
            &monitor_entry,
            &move_entry,
            &size_entry,
            &opacity_entry,
            &suppress_entry,
            &border_entry,
            &rounding_entry,
        ];
        let match_cbs: Vec<&CheckButton> = match_bools.iter().map(|(_, cb)| cb).collect();
        let effect_cbs: Vec<&CheckButton> = effect_bools.iter().map(|(_, cb)| cb).collect();
        let mut all_checks = match_cbs;
        all_checks.extend(effect_cbs);
        extra_ui::wire_dialog_autosave(&realtime, &try_save, &text_entries, &all_checks);
    }

    editor.present();
}

fn push_templated_field(
    map: &mut BTreeMap<String, Value>,
    key: &str,
    raw: &str,
    variables: &[ConfigVariable],
) -> Result<(), String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(());
    }
    if raw.contains("{{") {
        variables::expand_template(raw, variables)?;
    }
    if let Some(value) = window_rules::field_to_value(raw)? {
        map.insert(key.into(), value);
    }
    Ok(())
}

fn rule_field_preview(template: &str, variables: &[ConfigVariable]) -> String {
    if template.trim().is_empty() {
        return String::new();
    }
    match variables::expand_template(template, variables) {
        Ok(expanded) if expanded != template => format!("Resolves to: {expanded}"),
        Ok(_) => String::new(),
        Err(err) => format!("Preview: {err}"),
    }
}

fn push_int_or_string_effect(effects: &mut BTreeMap<String, Value>, key: &str, raw: &str) {
    let v = raw.trim();
    if v.is_empty() {
        return;
    }
    if let Ok(n) = v.parse::<i64>() {
        effects.insert(key.into(), json!(n));
    } else {
        effects.insert(key.into(), json!(v));
    }
}

fn show_class_picker<F>(parent: &impl IsA<Window>, on_pick: F)
where
    F: Fn(String) + 'static,
{
    let picker = Window::builder()
        .title("Running window classes")
        .transient_for(parent)
        .modal(true)
        .default_width(520)
        .default_height(420)
        .build();

    let hint = Label::builder()
        .label("Click a class to use it as an exact match (^class$). Refresh reads hyprctl clients.")
        .halign(gtk4::Align::Start)
        .wrap(true)
        .css_classes(["dim-label", "caption"])
        .build();

    let refresh = Button::builder().label("Refresh").build();
    let close = Button::builder().label("Close").build();
    let toolbar = GtkBox::new(Orientation::Horizontal, 8);
    toolbar.append(&refresh);
    toolbar.append(&close);

    let list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();
    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .child(&list)
        .build();

    let status = Label::builder()
        .label("")
        .halign(gtk4::Align::Start)
        .css_classes(["dim-label"])
        .build();

    let root = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .margin_top(14)
        .margin_bottom(14)
        .margin_start(14)
        .margin_end(14)
        .build();
    root.append(&hint);
    root.append(&toolbar);
    root.append(&scroll);
    root.append(&status);
    picker.set_child(Some(&root));

    let refill = {
        let list = list.clone();
        let status = status.clone();
        Rc::new(move || {
            clear_list(&list);
            match clients::list_running_windows() {
                Ok(windows) => {
                    if windows.is_empty() {
                        status.set_text("No windows reported by hyprctl.");
                        return;
                    }
                    status.set_text(&format!("{} windows", windows.len()));
                    for w in windows {
                        let title = if w.title.is_empty() {
                            "(no title)".into()
                        } else {
                            w.title.clone()
                        };
                        let primary = Label::builder()
                            .label(&w.class)
                            .halign(gtk4::Align::Start)
                            .css_classes(["title-4", "monospace"])
                            .selectable(true)
                            .build();
                        let secondary = Label::builder()
                            .label(&title)
                            .halign(gtk4::Align::Start)
                            .css_classes(["dim-label", "caption"])
                            .ellipsize(gtk4::pango::EllipsizeMode::End)
                            .build();
                        let col = GtkBox::builder()
                            .orientation(Orientation::Vertical)
                            .spacing(2)
                            .margin_top(8)
                            .margin_bottom(8)
                            .margin_start(10)
                            .margin_end(10)
                            .build();
                        col.append(&primary);
                        col.append(&secondary);
                        let row = ListBoxRow::builder()
                            .child(&col)
                            .activatable(true)
                            .name(format!("class-{}", w.class))
                            .build();
                        list.append(&row);
                    }
                }
                Err(err) => {
                    status.set_text(&format!("Could not read clients: {err}"));
                }
            }
        })
    };

    refill();

    refresh.connect_clicked({
        let refill = Rc::clone(&refill);
        move |_| refill()
    });
    close.connect_clicked({
        let picker = picker.clone();
        move |_| picker.close()
    });

    list.connect_row_activated({
        let picker = picker.clone();
        let on_pick = Rc::new(on_pick);
        move |_, row| {
            let name = row.widget_name();
            if let Some(class) = name.strip_prefix("class-") {
                on_pick(class.to_string());
                picker.close();
            }
        }
    });

    dialog::close_on_escape(&picker);
    picker.present();
}
