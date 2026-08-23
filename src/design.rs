//! Full visual system for Hyprbinds.

use gtk4::prelude::*;
use gtk4::CssProvider;

pub fn apply() {
    let css = r#"
        @define-color hb_surface alpha(currentColor, 0.022);
        @define-color hb_surface_2 alpha(currentColor, 0.038);
        @define-color hb_surface_3 alpha(currentColor, 0.060);
        @define-color hb_hover alpha(currentColor, 0.050);
        @define-color hb_selected alpha(currentColor, 0.075);
        @define-color hb_border alpha(currentColor, 0.080);
        @define-color hb_border_soft alpha(currentColor, 0.050);

        window { font-size: 1em; }

        button,
        entry,
        dropdown > button,
        dropdown button.toggle,
        spinbutton {
            min-height: 30px;
            border-radius: 8px;
            box-shadow: none;
        }

        button {
            min-width: 0;
            padding: 3px 10px;
            font-size: 0.88em;
            font-weight: 560;
            border: 1px solid transparent;
        }
        button:hover {
            background-color: @hb_hover;
            border-color: @hb_border;
        }

        entry {
            padding: 3px 9px;
            border: 1px solid @hb_border;
            background-color: @hb_surface_2;
        }
        entry:focus {
            border-color: alpha(currentColor, 0.22);
            background-color: @hb_surface_3;
            box-shadow: none;
        }

        dropdown > button,
        dropdown button.toggle,
        spinbutton {
            border: 1px solid @hb_border;
            background-color: @hb_surface_2;
        }

        checkbutton { font-size: 0.88em; }
        separator { opacity: 0.26; }

        .hyprbinds-shell,
        .hyprbinds-main {
            background-color: @window_bg_color;
        }

        .hyprbinds-header {
            min-height: 39px;
            padding: 5px 22px;
            border-bottom: 1px solid @hb_border_soft;
            background-color: transparent;
        }
        .hyprbinds-header-title {
            font-size: 0.80em;
            font-weight: 650;
            opacity: 0.36;
        }
        .hyprbinds-path {
            font-size: 0.72em;
            opacity: 0.27;
            margin-right: 8px;
        }
        .hyprbinds-count {
            font-size: 0.72em;
            opacity: 0.58;
            padding: 2px 7px;
            border-radius: 6px;
            background-color: @hb_surface_2;
            border: 1px solid @hb_border_soft;
        }
        .hyprbinds-header-toggle {
            opacity: 0.56;
            font-size: 0.82em;
        }
        .hyprbinds-header button {
            min-height: 27px;
            padding: 2px 8px;
            border-radius: 7px;
        }
        .hyprbinds-content { padding-top: 14px; }

        .hyprbinds-sidebar {
            min-width: 272px;
            max-width: 272px;
            padding: 16px 11px 14px 11px;
            border-right: 1px solid @hb_border_soft;
            background-color: alpha(currentColor, 0.012);
        }
        .hyprbinds-brand { padding: 1px 8px 12px 8px; }
        .hyprbinds-brand-mark {
            min-width: 34px;
            min-height: 34px;
            border-radius: 9px;
            background-color: @hb_surface_3;
            color: inherit;
            font-size: 1.02em;
            font-weight: 760;
            border: 1px solid @hb_border;
        }
        .hyprbinds-brand-title {
            font-size: 1.02em;
            font-weight: 700;
            letter-spacing: -0.02em;
        }
        .hyprbinds-brand-sub {
            font-size: 0.70em;
            opacity: 0.33;
        }
        .hyprbinds-sidebar-filter {
            min-height: 33px;
            margin: 0 4px 9px 4px;
            border-radius: 8px;
            background-color: @hb_surface_2;
            border: 1px solid @hb_border_soft;
        }
        list.hyprbinds-sidebar-list,
        .hyprbinds-sidebar-scroll {
            background: transparent;
            border: none;
        }
        list.hyprbinds-sidebar-list > row {
            border-radius: 8px;
            margin: 1px 0;
            border: 1px solid transparent;
        }
        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-header-row {
            margin-top: 12px;
            margin-bottom: 2px;
            background: transparent;
            border: none;
        }
        .hyprbinds-sidebar-section {
            padding: 3px 9px;
            font-size: 0.62em;
            font-weight: 740;
            letter-spacing: 0.10em;
            opacity: 0.28;
        }
        .hyprbinds-sidebar-row-inner { padding: 8px 9px; }
        .hyprbinds-sidebar-icon {
            min-width: 1.65em;
            font-size: 0.98em;
            opacity: 0.50;
        }
        .hyprbinds-sidebar-label {
            font-size: 0.91em;
            font-weight: 610;
        }
        .hyprbinds-sidebar-subtitle {
            font-size: 0.69em;
            opacity: 0.33;
        }
        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-row:hover {
            background-color: @hb_hover;
            border-color: @hb_border_soft;
        }
        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-row:selected,
        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-row.nav-active {
            background-color: @hb_selected;
            border-color: @hb_border;
            box-shadow: none;
        }
        list.hyprbinds-sidebar-list > row.nav-active .hyprbinds-sidebar-label {
            color: inherit;
            font-weight: 660;
        }
        list.hyprbinds-sidebar-list > row.nav-active .hyprbinds-sidebar-subtitle,
        list.hyprbinds-sidebar-list > row.nav-active .hyprbinds-sidebar-icon {
            opacity: 0.70;
        }

        .hyprbinds-page { padding: 4px 2px 2px 2px; }
        .hyprbinds-page-header { padding: 2px 2px 10px 2px; }
        .hyprbinds-page-eyebrow,
        .hyprbinds-hero-kicker {
            font-size: 0.61em;
            font-weight: 760;
            letter-spacing: 0.12em;
            color: inherit;
            opacity: 0.32;
        }
        .hyprbinds-page-title {
            font-size: 1.90em;
            font-weight: 740;
            letter-spacing: -0.045em;
        }
        .hyprbinds-page-hint {
            font-size: 0.89em;
            opacity: 0.43;
            line-height: 1.45;
            max-width: 60em;
        }

        .hyprbinds-toolbar {
            padding: 7px;
            margin: 0 0 3px 0;
            border: 1px solid @hb_border_soft;
            border-radius: 10px;
            background-color: @hb_surface;
        }
        .hyprbinds-toolbar entry {
            min-height: 32px;
            background-color: @hb_surface_2;
        }
        .hyprbinds-toolbar button,
        .hyprbinds-toolbar dropdown > button {
            min-height: 32px;
        }
        .hyprbinds-page-canvas {
            padding-top: 12px;
            background: transparent;
        }

        notebook.hyprbinds-hub,
        notebook.hyprbinds-bind-tabs,
        notebook.hyprbinds-hub > header,
        notebook.hyprbinds-bind-tabs > header,
        notebook.hyprbinds-hub > stack,
        notebook.hyprbinds-bind-tabs > stack {
            background: transparent;
            border: none;
        }
        notebook.hyprbinds-bind-tabs > header {
            margin-bottom: 8px;
            border-bottom: 1px solid @hb_border_soft;
        }
        notebook.hyprbinds-hub > header tab,
        notebook.hyprbinds-bind-tabs > header tab {
            min-height: 28px;
            padding: 4px 10px 5px 10px;
            border: none;
            border-radius: 0;
            opacity: 0.42;
        }
        notebook.hyprbinds-hub > header tab:hover,
        notebook.hyprbinds-bind-tabs > header tab:hover {
            opacity: 0.72;
            background-color: @hb_surface;
        }
        notebook.hyprbinds-hub > header tab:checked,
        notebook.hyprbinds-bind-tabs > header tab:checked {
            opacity: 1;
            color: inherit;
            box-shadow: inset 0 -1px 0 alpha(currentColor, 0.52);
            background: transparent;
        }

        list.boxed-list {
            margin-top: 3px;
            border-radius: 10px;
            background-color: @hb_surface;
            border: 1px solid @hb_border_soft;
            box-shadow: none;
        }
        list.boxed-list > row {
            border-bottom: 1px solid @hb_border_soft;
        }
        list.boxed-list > row:hover { background-color: @hb_hover; }
        list.boxed-list > row:selected {
            background-color: @hb_selected;
            box-shadow: none;
        }
        .hyprbinds-row-title {
            font-size: 0.96em;
            font-weight: 630;
        }
        .hyprbinds-row-sub { font-size: 0.81em; opacity: 0.54; }
        .hyprbinds-row-body { font-size: 0.81em; opacity: 0.47; }
        .hyprbinds-row-meta { font-size: 0.71em; opacity: 0.30; }
        .hyprbinds-row-keys {
            padding: 3px 8px;
            border-radius: 6px;
            background-color: @hb_surface_3;
            border: 1px solid @hb_border;
            font-size: 0.78em;
            font-weight: 650;
        }
        list.boxed-list > row:selected .hyprbinds-row-keys {
            background-color: alpha(currentColor, 0.09);
            border-color: alpha(currentColor, 0.15);
        }
        .hyprbinds-section {
            margin-top: 15px;
            margin-bottom: 6px;
            font-size: 0.66em;
            font-weight: 740;
            letter-spacing: 0.10em;
            opacity: 0.33;
        }

        .hyprbinds-dashboard { padding: 4px 2px; }
        .hyprbinds-hero {
            padding: 20px 22px;
            border-radius: 12px;
            background-color: @hb_surface;
            background-image: none;
            border: 1px solid @hb_border;
        }
        .hyprbinds-hero-title {
            font-size: 2.02em;
            font-weight: 760;
            letter-spacing: -0.05em;
        }
        .hyprbinds-hero-sub {
            max-width: 52em;
            font-size: 0.90em;
            opacity: 0.45;
        }
        .hyprbinds-metrics { margin-top: 3px; }
        .hyprbinds-metric {
            padding: 14px 15px;
            border-radius: 10px;
            border: 1px solid @hb_border_soft;
            background-color: @hb_surface;
        }
        .hyprbinds-metric-value {
            font-size: 1.44em;
            font-weight: 750;
            letter-spacing: -0.035em;
        }
        .hyprbinds-metric-label {
            font-size: 0.70em;
            opacity: 0.33;
        }
        .hyprbinds-dashboard-panel {
            padding: 15px 16px;
            border-radius: 10px;
            border: 1px solid @hb_border_soft;
            background-color: @hb_surface;
        }
        .hyprbinds-dashboard-note {
            padding-top: 10px;
            font-size: 0.74em;
            opacity: 0.35;
        }
        .hyprbinds-speed-value {
            font-size: 1.06em;
            font-weight: 700;
            color: inherit;
        }

        .hyprbinds-settings-card,
        .hyprbinds-dialog-section {
            background-color: @hb_surface;
            border: 1px solid @hb_border_soft;
            border-radius: 10px;
            box-shadow: none;
        }
        .hyprbinds-settings-card { padding: 6px 4px 7px 4px; }
        .hyprbinds-settings-card-title,
        .hyprbinds-dialog-section-title {
            font-size: 0.66em;
            font-weight: 740;
            letter-spacing: 0.09em;
            opacity: 0.33;
        }
        .hyprbinds-settings-row {
            min-height: 40px;
            padding: 10px 13px;
            border-radius: 7px;
        }
        .hyprbinds-settings-row:hover { background-color: @hb_hover; }
        .hyprbinds-settings-title { font-size: 0.94em; font-weight: 620; }
        .hyprbinds-settings-sub { font-size: 0.78em; opacity: 0.39; }

        button.suggested-action {
            min-height: 30px;
            padding: 3px 12px;
            border-radius: 8px;
            background-image: none;
            background-color: alpha(currentColor, 0.13);
            color: inherit;
            font-weight: 650;
            border: 1px solid alpha(currentColor, 0.14);
            box-shadow: none;
        }
        button.suggested-action:hover {
            background-color: alpha(currentColor, 0.17);
            border-color: alpha(currentColor, 0.20);
        }
        button.destructive-action {
            min-height: 30px;
            border-radius: 8px;
            background-color: transparent;
            color: @error_color;
            border: 1px solid alpha(@error_color, 0.10);
        }
        button.destructive-action:hover {
            background-color: alpha(@error_color, 0.08);
        }

        .hyprbinds-status-sep { margin-top: 4px; opacity: 0.22; }
        .hyprbinds-status {
            min-height: 18px;
            padding-top: 5px;
            font-size: 0.77em;
            opacity: 0.40;
        }
        .hyprbinds-sticky-footer {
            background-color: @window_bg_color;
            border-top: 1px solid @hb_border_soft;
        }
        .hyprbinds-palette-search {
            min-height: 35px;
            padding: 5px 10px;
            font-size: 0.98em;
        }
        .hyprbinds-health-badge,
        .hyprbinds-conflict-badge { border-radius: 5px; }
        .curve-graph {
            border-radius: 9px;
            border-color: @hb_border;
            background-color: @hb_surface;
        }
    "#;

    let provider = CssProvider::new();
    provider.load_from_string(css);
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_USER + 1,
        );
    }
}
