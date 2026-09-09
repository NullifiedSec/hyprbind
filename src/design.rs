//! Production visual system for Hyprbinds.
//!
//! Keep this layer deliberately small and predictable. Feature modules own layout
//! and semantics; this file owns shared visual tokens, component states, and the
//! spacing/control conventions used across the application.

use gtk4::prelude::*;
use gtk4::CssProvider;

pub fn apply() {
    let css = r#"
        /*
         * Hyprbinds UI conventions
         * ------------------------
         * control height: 32px
         * compact control height: 28px
         * standard radius: 8px
         * surface radius: 10px
         * neutral hierarchy only; semantic color is reserved for errors
         */
        @define-color hb_surface alpha(currentColor, 0.022);
        @define-color hb_surface_raised alpha(currentColor, 0.040);
        @define-color hb_surface_strong alpha(currentColor, 0.065);
        @define-color hb_hover alpha(currentColor, 0.050);
        @define-color hb_selected alpha(currentColor, 0.080);
        @define-color hb_border alpha(currentColor, 0.080);
        @define-color hb_border_soft alpha(currentColor, 0.050);
        @define-color hb_focus alpha(currentColor, 0.260);

        window {
            font-size: 1em;
        }

        /* Base controls */
        button,
        entry,
        dropdown > button,
        dropdown button.toggle,
        spinbutton {
            min-height: 32px;
            border-radius: 8px;
            box-shadow: none;
        }

        button {
            min-width: 0;
            padding: 4px 10px;
            font-size: 0.88em;
            font-weight: 560;
            border: 1px solid transparent;
            background-image: none;
        }

        button:hover {
            background-color: @hb_hover;
            border-color: @hb_border;
        }

        button:focus-visible,
        entry:focus-visible,
        dropdown > button:focus-visible,
        spinbutton:focus-visible,
        checkbutton:focus-visible,
        switch:focus-visible {
            outline: 2px solid @hb_focus;
            outline-offset: 1px;
        }

        button:disabled,
        entry:disabled,
        dropdown:disabled,
        spinbutton:disabled,
        checkbutton:disabled,
        switch:disabled {
            opacity: 0.42;
        }

        entry {
            padding: 4px 10px;
            border: 1px solid @hb_border;
            background-color: @hb_surface_raised;
        }

        entry:focus {
            border-color: @hb_focus;
            background-color: @hb_surface_strong;
            box-shadow: none;
        }

        dropdown > button,
        dropdown button.toggle,
        spinbutton {
            border: 1px solid @hb_border;
            background-color: @hb_surface_raised;
        }

        checkbutton {
            font-size: 0.88em;
        }

        separator {
            opacity: 0.26;
        }

        /* Application shell */
        .hyprbinds-shell,
        .hyprbinds-main {
            background-color: @window_bg_color;
        }

        .hyprbinds-header {
            min-height: 40px;
            padding: 4px 22px;
            border-bottom: 1px solid @hb_border_soft;
            background-color: transparent;
        }

        .hyprbinds-header-title {
            font-size: 0.80em;
            font-weight: 650;
            opacity: 0.38;
        }

        .hyprbinds-path {
            font-size: 0.72em;
            opacity: 0.30;
            margin-right: 8px;
        }

        .hyprbinds-count {
            font-size: 0.72em;
            opacity: 0.62;
            padding: 2px 7px;
            border-radius: 6px;
            background-color: @hb_surface_raised;
            border: 1px solid @hb_border_soft;
        }

        .hyprbinds-header-toggle {
            opacity: 0.62;
            font-size: 0.82em;
        }

        .hyprbinds-header button {
            min-height: 28px;
            padding: 2px 8px;
            border-radius: 7px;
        }

        .hyprbinds-content {
            padding-top: 14px;
        }

        /* Sidebar */
        .hyprbinds-sidebar {
            min-width: 272px;
            max-width: 272px;
            padding: 16px 11px 12px 11px;
            border-right: 1px solid @hb_border_soft;
            background-color: alpha(currentColor, 0.012);
        }

        .hyprbinds-brand {
            padding: 1px 8px 12px 8px;
        }

        .hyprbinds-brand-mark {
            min-width: 34px;
            min-height: 34px;
            border-radius: 8px;
            background-color: @hb_surface_strong;
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
            opacity: 0.36;
        }

        .hyprbinds-sidebar-filter {
            min-height: 32px;
            margin: 0 4px 8px 4px;
            border-radius: 8px;
            background-color: @hb_surface_raised;
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
            opacity: 0.30;
        }

        .hyprbinds-sidebar-row-inner {
            padding: 8px 9px;
        }

        .hyprbinds-sidebar-icon {
            min-width: 1.65em;
            font-size: 0.98em;
            opacity: 0.52;
        }

        .hyprbinds-sidebar-label {
            font-size: 0.91em;
            font-weight: 610;
        }

        .hyprbinds-sidebar-subtitle {
            font-size: 0.69em;
            opacity: 0.36;
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
            opacity: 0.72;
        }

        /* Page hierarchy */
        .hyprbinds-page {
            padding: 4px 2px 2px 2px;
        }

        .hyprbinds-page-header {
            padding: 2px 2px 10px 2px;
        }

        .hyprbinds-page-eyebrow,
        .hyprbinds-hero-kicker {
            font-size: 0.61em;
            font-weight: 760;
            letter-spacing: 0.12em;
            color: inherit;
            opacity: 0.34;
        }

        .hyprbinds-page-title {
            font-size: 1.90em;
            font-weight: 740;
            letter-spacing: -0.045em;
        }

        .hyprbinds-page-hint {
            font-size: 0.89em;
            opacity: 0.46;
            line-height: 1.45;
            max-width: 60em;
        }

        .hyprbinds-toolbar {
            padding: 7px;
            margin: 0 0 4px 0;
            border: 1px solid @hb_border_soft;
            border-radius: 10px;
            background-color: @hb_surface;
        }

        .hyprbinds-toolbar entry,
        .hyprbinds-toolbar button,
        .hyprbinds-toolbar dropdown > button {
            min-height: 32px;
        }

        .hyprbinds-toolbar entry {
            background-color: @hb_surface_raised;
        }

        .hyprbinds-page-canvas {
            padding-top: 12px;
            background: transparent;
        }

        /* Tabs */
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
            opacity: 0.46;
        }

        notebook.hyprbinds-hub > header tab:hover,
        notebook.hyprbinds-bind-tabs > header tab:hover {
            opacity: 0.76;
            background-color: @hb_surface;
        }

        notebook.hyprbinds-hub > header tab:checked,
        notebook.hyprbinds-bind-tabs > header tab:checked {
            opacity: 1;
            color: inherit;
            box-shadow: inset 0 -1px 0 alpha(currentColor, 0.52);
            background: transparent;
        }

        /* Lists */
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

        list.boxed-list > row:last-child {
            border-bottom: none;
        }

        list.boxed-list > row:hover {
            background-color: @hb_hover;
        }

        list.boxed-list > row:selected {
            background-color: @hb_selected;
            box-shadow: none;
        }

        .hyprbinds-row-title {
            font-size: 0.96em;
            font-weight: 630;
        }

        .hyprbinds-row-sub {
            font-size: 0.81em;
            opacity: 0.56;
        }

        .hyprbinds-row-body {
            font-size: 0.81em;
            opacity: 0.49;
        }

        .hyprbinds-row-meta {
            font-size: 0.71em;
            opacity: 0.34;
        }

        .hyprbinds-row-keys {
            padding: 3px 8px;
            border-radius: 6px;
            background-color: @hb_surface_strong;
            border: 1px solid @hb_border;
            font-size: 0.78em;
            font-weight: 650;
        }

        list.boxed-list > row:selected .hyprbinds-row-keys {
            background-color: alpha(currentColor, 0.09);
            border-color: alpha(currentColor, 0.15);
        }

        .hyprbinds-section {
            margin-top: 16px;
            margin-bottom: 6px;
            font-size: 0.66em;
            font-weight: 740;
            letter-spacing: 0.10em;
            opacity: 0.34;
        }

        /* Overview dashboard */
        .hyprbinds-dashboard {
            padding: 4px 2px;
        }

        .hyprbinds-hero {
            padding: 20px 22px;
            border-radius: 10px;
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
            opacity: 0.46;
        }

        .hyprbinds-metrics {
            margin-top: 4px;
        }

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
            opacity: 0.36;
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
            opacity: 0.38;
        }

        .hyprbinds-speed-value {
            font-size: 1.06em;
            font-weight: 700;
            color: inherit;
        }

        /* Settings and dialogs */
        .hyprbinds-settings-card,
        .hyprbinds-dialog-section,
        .hyprbinds-via-frame,
        .hyprbinds-starship-preview,
        .hyprbinds-waybar-preview {
            background-color: @hb_surface;
            border: 1px solid @hb_border_soft;
            border-radius: 10px;
            box-shadow: none;
        }

        .hyprbinds-settings-card {
            padding: 6px 4px 7px 4px;
        }

        .hyprbinds-settings-card-title,
        .hyprbinds-dialog-section-title {
            font-size: 0.66em;
            font-weight: 740;
            letter-spacing: 0.09em;
            opacity: 0.34;
        }

        .hyprbinds-settings-row {
            min-height: 40px;
            padding: 10px 13px;
            border-radius: 8px;
        }

        .hyprbinds-settings-row:hover {
            background-color: @hb_hover;
        }

        .hyprbinds-settings-title {
            font-size: 0.94em;
            font-weight: 620;
        }

        .hyprbinds-settings-sub {
            font-size: 0.78em;
            opacity: 0.42;
        }

        /* Action hierarchy */
        button.suggested-action {
            min-height: 32px;
            padding: 4px 12px;
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
            min-height: 32px;
            padding: 4px 10px;
            border-radius: 8px;
            background-color: transparent;
            color: @error_color;
            border: 1px solid alpha(@error_color, 0.12);
        }

        button.destructive-action:hover {
            background-color: alpha(@error_color, 0.08);
            border-color: alpha(@error_color, 0.18);
        }

        /* Status, palette, diagnostics */
        .hyprbinds-status-sep {
            margin-top: 4px;
            opacity: 0.22;
        }

        .hyprbinds-status {
            min-height: 18px;
            padding-top: 5px;
            font-size: 0.77em;
            opacity: 0.42;
        }

        .hyprbinds-sticky-footer {
            background-color: @window_bg_color;
            border-top: 1px solid @hb_border_soft;
        }

        .hyprbinds-palette-root {
            background-color: @window_bg_color;
        }

        .hyprbinds-palette-search {
            min-height: 36px;
            padding: 6px 10px;
            font-size: 0.98em;
        }

        .hyprbinds-health-badge,
        .hyprbinds-conflict-badge {
            border-radius: 6px;
        }

        .curve-graph {
            border-radius: 10px;
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
