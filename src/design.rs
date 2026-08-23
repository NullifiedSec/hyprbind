//! Visual design layer for Hyprbinds.
//!
//! This intentionally sits above the baseline UI CSS so visual refinement stays
//! decoupled from feature code. It changes presentation only: spacing, hierarchy,
//! surfaces, controls, navigation, and list treatment.

use gtk4::prelude::*;
use gtk4::CssProvider;

pub fn apply() {
    let css = r#"
        @define-color hb_accent #00d4ff;
        @define-color hb_accent_hover #35e0ff;
        @define-color hb_accent_soft alpha(#00d4ff, 0.12);
        @define-color hb_surface alpha(currentColor, 0.032);
        @define-color hb_surface_hover alpha(currentColor, 0.055);
        @define-color hb_border alpha(currentColor, 0.085);
        @define-color hb_border_strong alpha(currentColor, 0.14);

        /* Global rhythm */
        window {
            font-size: 1em;
        }

        button,
        entry,
        dropdown > button,
        dropdown button.toggle,
        spinbutton {
            min-height: 32px;
            border-radius: 9px;
        }

        button {
            padding: 4px 12px;
            font-size: 0.9em;
            font-weight: 550;
            transition: background-color 100ms ease, box-shadow 100ms ease;
        }

        button:hover {
            background-color: @hb_surface_hover;
        }

        entry {
            padding: 4px 10px;
            background-color: alpha(currentColor, 0.025);
            border: 1px solid @hb_border;
        }

        entry:focus {
            border-color: alpha(@hb_accent, 0.7);
            box-shadow: 0 0 0 2px alpha(@hb_accent, 0.10);
        }

        dropdown > button,
        dropdown button.toggle,
        spinbutton {
            border: 1px solid @hb_border;
            background-color: alpha(currentColor, 0.025);
        }

        checkbutton {
            padding: 3px 0;
            font-size: 0.9em;
        }

        separator {
            opacity: 0.45;
        }

        /* App shell */
        .hyprbinds-shell {
            background-color: @window_bg_color;
        }

        .hyprbinds-main {
            background-image: linear-gradient(
                to bottom,
                alpha(@hb_accent, 0.018),
                transparent 180px
            );
        }

        .hyprbinds-header {
            min-height: 54px;
            padding: 10px 26px;
            border-bottom: 1px solid @hb_border;
            background-color: alpha(@window_bg_color, 0.96);
        }

        .hyprbinds-header-title {
            font-size: 1.08em;
            font-weight: 650;
            letter-spacing: -0.015em;
        }

        .hyprbinds-path {
            opacity: 0.42;
            font-size: 0.78em;
            margin-right: 8px;
        }

        .hyprbinds-count {
            opacity: 0.52;
            font-size: 0.8em;
            padding: 3px 8px;
            border-radius: 999px;
            background-color: @hb_surface;
            border: 1px solid @hb_border;
        }

        .hyprbinds-header-toggle {
            margin-left: 2px;
            opacity: 0.82;
        }

        .hyprbinds-content {
            padding-top: 6px;
        }

        /* Sidebar */
        .hyprbinds-sidebar {
            min-width: 252px;
            max-width: 286px;
            padding: 16px 12px 14px 14px;
            border-right: 1px solid @hb_border;
            background-image: linear-gradient(
                to bottom,
                alpha(@hb_accent, 0.045),
                alpha(currentColor, 0.018) 160px,
                alpha(currentColor, 0.012)
            );
        }

        .hyprbinds-sidebar-filter {
            min-height: 34px;
            margin: 0 3px 8px 3px;
            border-radius: 10px;
            background-color: alpha(currentColor, 0.035);
            border: 1px solid @hb_border;
        }

        .hyprbinds-sidebar-filter:focus {
            border-color: alpha(@hb_accent, 0.55);
            box-shadow: 0 0 0 2px alpha(@hb_accent, 0.08);
        }

        list.hyprbinds-sidebar-list > row {
            border-radius: 10px;
            margin: 2px 0;
        }

        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-header-row {
            margin-top: 13px;
            margin-bottom: 3px;
        }

        .hyprbinds-sidebar-section {
            font-size: 0.66em;
            font-weight: 750;
            letter-spacing: 0.11em;
            opacity: 0.38;
            padding: 3px 10px;
        }

        .hyprbinds-sidebar-row-inner {
            padding: 9px 11px;
        }

        .hyprbinds-sidebar-icon {
            min-width: 1.55em;
            font-size: 1.02em;
            opacity: 0.72;
        }

        .hyprbinds-sidebar-label {
            font-size: 0.94em;
            font-weight: 570;
        }

        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-row:hover {
            background-color: @hb_surface_hover;
        }

        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-row:selected,
        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-row.nav-active {
            background-color: @hb_accent_soft;
            box-shadow: inset 3px 0 0 0 @hb_accent;
        }

        list.hyprbinds-sidebar-list > row.nav-active .hyprbinds-sidebar-icon,
        list.hyprbinds-sidebar-list > row.nav-active .hyprbinds-sidebar-label {
            opacity: 1;
        }

        /* Page hierarchy */
        .hyprbinds-page {
            padding: 6px 0 2px 0;
        }

        .hyprbinds-page-header {
            margin: 0 0 6px 0;
            padding: 3px 1px 1px 1px;
        }

        .hyprbinds-page-title {
            font-size: 1.68em;
            font-weight: 720;
            letter-spacing: -0.035em;
        }

        .hyprbinds-page-hint {
            opacity: 0.52;
            font-size: 0.9em;
            line-height: 1.45;
            max-width: 62em;
        }

        .hyprbinds-toolbar {
            padding: 8px;
            margin: 2px 0 7px 0;
            border-radius: 12px;
            border: 1px solid @hb_border;
            background-color: @hb_surface;
        }

        .hyprbinds-toolbar entry {
            min-height: 32px;
            background-color: alpha(@window_bg_color, 0.48);
        }

        .hyprbinds-toolbar button {
            min-height: 32px;
        }

        notebook.hyprbinds-hub > header tabs,
        notebook.hyprbinds-bind-tabs > header tabs {
            background-color: @hb_surface;
            border: 1px solid @hb_border;
            border-radius: 10px;
            padding: 3px;
        }

        notebook.hyprbinds-bind-tabs > header {
            margin-bottom: 10px;
        }

        notebook.hyprbinds-hub > header tab,
        notebook.hyprbinds-bind-tabs > header tab {
            min-height: 28px;
            padding: 3px 11px;
            border-radius: 7px;
            border: none;
        }

        notebook.hyprbinds-hub > header tab:checked,
        notebook.hyprbinds-bind-tabs > header tab:checked {
            background-color: @hb_accent_soft;
            box-shadow: inset 0 -2px 0 0 @hb_accent;
        }

        /* Lists and information surfaces */
        list.boxed-list {
            margin-top: 5px;
            border-radius: 14px;
            background-color: @hb_surface;
            border: 1px solid @hb_border;
            box-shadow: 0 1px 2px alpha(#000000, 0.12);
        }

        list.boxed-list > row {
            border-bottom: 1px solid alpha(currentColor, 0.05);
        }

        list.boxed-list > row:hover {
            background-color: @hb_surface_hover;
        }

        list.boxed-list > row:selected {
            background-color: @hb_accent_soft;
            box-shadow: inset 3px 0 0 0 @hb_accent;
        }

        .hyprbinds-row-title {
            font-size: 0.98em;
            font-weight: 620;
            letter-spacing: -0.008em;
        }

        .hyprbinds-row-sub {
            opacity: 0.64;
            font-size: 0.84em;
        }

        .hyprbinds-row-body {
            opacity: 0.56;
            font-size: 0.83em;
        }

        .hyprbinds-row-meta {
            opacity: 0.40;
            font-size: 0.74em;
        }

        .hyprbinds-row-keys {
            padding: 3px 9px;
            border-radius: 7px;
            background-color: alpha(@hb_accent, 0.075);
            border: 1px solid alpha(@hb_accent, 0.16);
            font-size: 0.8em;
            font-weight: 650;
            opacity: 0.95;
        }

        .hyprbinds-section {
            font-size: 0.7em;
            font-weight: 750;
            letter-spacing: 0.10em;
            opacity: 0.43;
            margin-top: 16px;
            margin-bottom: 5px;
        }

        /* Settings cards */
        .hyprbinds-settings-card,
        .hyprbinds-dialog-section,
        .hyprbinds-via-frame,
        .hyprbinds-starship-preview,
        .hyprbinds-waybar-preview {
            background-color: @hb_surface;
            border: 1px solid @hb_border;
            border-radius: 15px;
            box-shadow: 0 1px 2px alpha(#000000, 0.10);
        }

        .hyprbinds-settings-card {
            padding: 8px 6px 10px 6px;
        }

        .hyprbinds-settings-card-title,
        .hyprbinds-dialog-section-title {
            font-size: 0.7em;
            font-weight: 750;
            letter-spacing: 0.09em;
            opacity: 0.45;
        }

        .hyprbinds-settings-row {
            min-height: 44px;
            padding: 11px 14px;
            border-radius: 9px;
        }

        .hyprbinds-settings-row:hover {
            background-color: alpha(currentColor, 0.022);
        }

        .hyprbinds-settings-title {
            font-weight: 620;
        }

        .hyprbinds-settings-sub {
            opacity: 0.52;
        }

        /* Primary / destructive actions */
        button.suggested-action {
            min-height: 32px;
            padding: 4px 14px;
            border-radius: 9px;
            background-image: none;
            background-color: @hb_accent;
            color: #031218;
            font-weight: 650;
            box-shadow: 0 1px 2px alpha(#000000, 0.16);
        }

        button.suggested-action:hover {
            background-color: @hb_accent_hover;
            box-shadow: 0 2px 5px alpha(#000000, 0.18);
        }

        button.destructive-action {
            min-height: 32px;
            padding: 4px 12px;
            border-radius: 9px;
            background-color: alpha(@error_color, 0.055);
            border: 1px solid alpha(@error_color, 0.16);
        }

        button.destructive-action:hover {
            background-color: alpha(@error_color, 0.13);
        }

        /* Status / footer */
        .hyprbinds-status-sep {
            margin-top: 7px;
            opacity: 0.32;
        }

        .hyprbinds-status {
            min-height: 20px;
            padding: 6px 2px 1px 2px;
            font-size: 0.82em;
            opacity: 0.54;
        }

        .hyprbinds-sticky-footer {
            padding: 8px 2px 0 2px;
            background-color: alpha(@window_bg_color, 0.94);
            border-top: 1px solid @hb_border;
        }

        /* Command palette */
        .hyprbinds-palette-root {
            background-color: @window_bg_color;
        }

        .hyprbinds-palette-search {
            min-height: 38px;
            padding: 6px 12px;
            border-radius: 11px;
            font-size: 1.02em;
        }

        /* Diagnostics */
        .hyprbinds-health-badge,
        .hyprbinds-conflict-badge {
            border-radius: 999px;
            padding: 3px 8px;
        }

        .curve-graph {
            border-radius: 14px;
            border-color: @hb_border_strong;
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
