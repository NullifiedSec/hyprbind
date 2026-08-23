//! Minimal modern presentation layer for Hyprbinds.
//! Keeps feature behavior untouched and only refines visual hierarchy.

use gtk4::prelude::*;
use gtk4::CssProvider;

pub fn apply() {
    let css = r#"
        @define-color hb_accent #00c8f8;
        @define-color hb_accent_soft alpha(#00c8f8, 0.10);
        @define-color hb_accent_hover alpha(#00c8f8, 0.16);
        @define-color hb_surface alpha(currentColor, 0.025);
        @define-color hb_surface_hover alpha(currentColor, 0.045);
        @define-color hb_border alpha(currentColor, 0.075);
        @define-color hb_border_strong alpha(currentColor, 0.12);

        /* General controls: compact, quiet, flat. */
        button,
        entry,
        dropdown > button,
        dropdown button.toggle,
        spinbutton {
            min-height: 28px;
            border-radius: 7px;
            box-shadow: none;
        }

        button {
            padding: 3px 10px;
            font-size: 0.88em;
            font-weight: 500;
        }

        button:hover {
            background-color: @hb_surface_hover;
        }

        entry,
        dropdown > button,
        dropdown button.toggle,
        spinbutton {
            background-color: @hb_surface;
            border: 1px solid @hb_border;
        }

        entry {
            padding: 3px 9px;
        }

        entry:focus {
            border-color: alpha(@hb_accent, 0.60);
            box-shadow: 0 0 0 1px alpha(@hb_accent, 0.13);
        }

        checkbutton {
            padding: 2px 0;
            font-size: 0.88em;
        }

        separator {
            opacity: 0.28;
        }

        /* Shell */
        .hyprbinds-shell,
        .hyprbinds-main,
        .hyprbinds-content {
            background-color: @window_bg_color;
        }

        .hyprbinds-header {
            min-height: 46px;
            padding: 8px 22px;
            background-color: @window_bg_color;
            border-bottom: 1px solid @hb_border;
        }

        .hyprbinds-header-title {
            font-size: 1.02em;
            font-weight: 650;
            letter-spacing: -0.01em;
        }

        .hyprbinds-path {
            opacity: 0.35;
            font-size: 0.76em;
            margin-right: 8px;
        }

        .hyprbinds-count {
            opacity: 0.52;
            font-size: 0.78em;
            margin-right: 5px;
        }

        .hyprbinds-header-toggle {
            opacity: 0.72;
            margin-left: 2px;
        }

        /* Sidebar: narrow, flat, restrained. */
        .hyprbinds-sidebar {
            min-width: 228px;
            max-width: 252px;
            padding: 12px 9px 12px 10px;
            background-color: alpha(currentColor, 0.012);
            border-right: 1px solid @hb_border;
        }

        .hyprbinds-sidebar-filter {
            min-height: 30px;
            margin: 0 2px 7px 2px;
            border-radius: 7px;
            background-color: alpha(currentColor, 0.025);
            border: 1px solid @hb_border;
        }

        list.hyprbinds-sidebar-list > row {
            border-radius: 7px;
            margin: 1px 0;
        }

        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-header-row {
            margin-top: 10px;
            margin-bottom: 1px;
        }

        .hyprbinds-sidebar-section {
            padding: 3px 8px;
            font-size: 0.65em;
            font-weight: 700;
            letter-spacing: 0.09em;
            opacity: 0.32;
        }

        .hyprbinds-sidebar-row-inner {
            padding: 7px 9px;
        }

        .hyprbinds-sidebar-icon {
            min-width: 1.35em;
            font-size: 0.96em;
            opacity: 0.58;
        }

        .hyprbinds-sidebar-label {
            font-size: 0.92em;
            font-weight: 520;
        }

        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-row:hover {
            background-color: @hb_surface_hover;
        }

        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-row:selected,
        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-row.nav-active {
            background-color: @hb_accent_soft;
            box-shadow: inset 2px 0 0 0 @hb_accent;
        }

        list.hyprbinds-sidebar-list > row.nav-active .hyprbinds-sidebar-label {
            font-weight: 620;
        }

        list.hyprbinds-sidebar-list > row.nav-active .hyprbinds-sidebar-icon {
            opacity: 0.9;
        }

        /* Page hierarchy */
        .hyprbinds-page {
            padding: 3px 0 0 0;
        }

        .hyprbinds-page-header {
            margin: 0 0 7px 0;
            padding: 1px 0;
        }

        .hyprbinds-page-title {
            font-size: 1.42em;
            font-weight: 680;
            letter-spacing: -0.025em;
        }

        .hyprbinds-page-hint {
            opacity: 0.44;
            font-size: 0.86em;
            line-height: 1.35;
            max-width: 60em;
        }

        /* Toolbars should feel integrated, not card-like. */
        .hyprbinds-toolbar {
            margin: 0 0 6px 0;
            padding: 0;
            background: transparent;
            border: none;
        }

        .hyprbinds-toolbar entry,
        .hyprbinds-toolbar button,
        .hyprbinds-toolbar dropdown > button {
            min-height: 29px;
        }

        /* Tabs: minimal underline, no pill container. */
        notebook.hyprbinds-hub > header,
        notebook.hyprbinds-bind-tabs > header {
            background: transparent;
            border: none;
        }

        notebook.hyprbinds-bind-tabs > header {
            margin-bottom: 6px;
        }

        notebook.hyprbinds-hub > header tabs,
        notebook.hyprbinds-bind-tabs > header tabs {
            background: transparent;
            border: none;
            padding: 0;
        }

        notebook.hyprbinds-hub > header tab,
        notebook.hyprbinds-bind-tabs > header tab {
            min-height: 26px;
            padding: 3px 10px 5px 10px;
            border-radius: 0;
            border: none;
            opacity: 0.55;
        }

        notebook.hyprbinds-hub > header tab:checked,
        notebook.hyprbinds-bind-tabs > header tab:checked {
            opacity: 1;
            background: transparent;
            box-shadow: inset 0 -2px 0 0 @hb_accent;
        }

        /* Lists: mostly flat, one thin outline around the group. */
        list.boxed-list {
            margin-top: 3px;
            border-radius: 9px;
            background-color: transparent;
            border: 1px solid @hb_border;
            box-shadow: none;
        }

        list.boxed-list > row {
            border-bottom: 1px solid alpha(currentColor, 0.045);
        }

        list.boxed-list > row:last-child {
            border-bottom: none;
        }

        list.boxed-list > row:hover {
            background-color: @hb_surface_hover;
        }

        list.boxed-list > row:selected {
            background-color: @hb_accent_soft;
            box-shadow: inset 2px 0 0 0 @hb_accent;
        }

        .hyprbinds-row-title {
            font-size: 0.96em;
            font-weight: 590;
            letter-spacing: -0.005em;
        }

        .hyprbinds-row-sub {
            opacity: 0.58;
            font-size: 0.82em;
        }

        .hyprbinds-row-body {
            opacity: 0.50;
            font-size: 0.81em;
        }

        .hyprbinds-row-meta {
            opacity: 0.34;
            font-size: 0.72em;
        }

        .hyprbinds-row-keys {
            padding: 2px 7px;
            border-radius: 5px;
            background-color: alpha(@hb_accent, 0.06);
            border: 1px solid alpha(@hb_accent, 0.11);
            font-size: 0.78em;
            font-weight: 600;
        }

        .hyprbinds-section {
            margin-top: 13px;
            margin-bottom: 4px;
            font-size: 0.68em;
            font-weight: 700;
            letter-spacing: 0.08em;
            opacity: 0.36;
        }

        /* Settings surfaces: no fake depth, no oversized cards. */
        .hyprbinds-settings-card,
        .hyprbinds-dialog-section,
        .hyprbinds-via-frame,
        .hyprbinds-starship-preview,
        .hyprbinds-waybar-preview {
            background-color: transparent;
            border: 1px solid @hb_border;
            border-radius: 9px;
            box-shadow: none;
        }

        .hyprbinds-settings-card {
            padding: 4px 2px 5px 2px;
        }

        .hyprbinds-settings-card-title,
        .hyprbinds-dialog-section-title {
            font-size: 0.68em;
            font-weight: 700;
            letter-spacing: 0.08em;
            opacity: 0.38;
        }

        .hyprbinds-settings-row {
            min-height: 38px;
            padding: 8px 12px;
            border-radius: 6px;
        }

        .hyprbinds-settings-row:hover {
            background-color: @hb_surface_hover;
        }

        .hyprbinds-settings-title {
            font-size: 0.94em;
            font-weight: 570;
        }

        .hyprbinds-settings-sub {
            font-size: 0.8em;
            opacity: 0.46;
        }

        /* Actions */
        button.suggested-action {
            min-height: 29px;
            padding: 3px 12px;
            border-radius: 7px;
            background-image: none;
            background-color: @hb_accent;
            color: #031116;
            font-weight: 620;
            border: none;
            box-shadow: none;
        }

        button.suggested-action:hover {
            background-color: #26d4fb;
        }

        button.destructive-action {
            min-height: 29px;
            padding: 3px 10px;
            border-radius: 7px;
            background: transparent;
            color: @error_color;
            border: 1px solid alpha(@error_color, 0.12);
        }

        button.destructive-action:hover {
            background-color: alpha(@error_color, 0.08);
        }

        /* Footer / status */
        .hyprbinds-status-sep {
            margin-top: 4px;
            opacity: 0.22;
        }

        .hyprbinds-status {
            min-height: 18px;
            padding: 4px 1px 0 1px;
            font-size: 0.79em;
            opacity: 0.43;
        }

        .hyprbinds-sticky-footer {
            padding-top: 6px;
            background-color: @window_bg_color;
            border-top: 1px solid @hb_border;
        }

        .hyprbinds-palette-search {
            min-height: 34px;
            border-radius: 8px;
            font-size: 0.98em;
        }

        .hyprbinds-health-badge,
        .hyprbinds-conflict-badge {
            border-radius: 999px;
            padding: 2px 7px;
        }

        .curve-graph {
            border-radius: 9px;
            border-color: @hb_border_strong;
            background-color: transparent;
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
