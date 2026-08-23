//! Full visual system for Hyprbinds.

use gtk4::prelude::*;
use gtk4::CssProvider;

pub fn apply() {
    let css = r#"
        @define-color hb_accent #00d4ff;
        @define-color hb_accent_soft alpha(#00d4ff, 0.11);
        @define-color hb_surface alpha(currentColor, 0.028);
        @define-color hb_surface_2 alpha(currentColor, 0.045);
        @define-color hb_hover alpha(currentColor, 0.055);
        @define-color hb_border alpha(currentColor, 0.085);
        @define-color hb_border_soft alpha(currentColor, 0.055);

        window { font-size: 1em; }

        button,
        entry,
        dropdown > button,
        dropdown button.toggle,
        spinbutton {
            min-height: 29px;
            border-radius: 8px;
            box-shadow: none;
        }

        button {
            min-width: 0;
            padding: 3px 10px;
            font-size: 0.88em;
            font-weight: 550;
        }
        button:hover { background-color: @hb_hover; }

        entry {
            padding: 3px 9px;
            border: 1px solid @hb_border;
            background-color: alpha(currentColor, 0.022);
        }
        entry:focus {
            border-color: alpha(@hb_accent, 0.62);
            box-shadow: 0 0 0 1px alpha(@hb_accent, 0.09);
        }

        dropdown > button,
        dropdown button.toggle,
        spinbutton {
            border: 1px solid @hb_border;
            background-color: alpha(currentColor, 0.022);
        }
        separator { opacity: 0.36; }

        .hyprbinds-shell { background-color: @window_bg_color; }
        .hyprbinds-main { background-color: @window_bg_color; }
        .hyprbinds-header {
            min-height: 42px;
            padding: 6px 22px;
            border-bottom: 1px solid @hb_border_soft;
            background-color: alpha(currentColor, 0.012);
        }
        .hyprbinds-header-title { font-size: 0.86em; font-weight: 650; opacity: 0.54; }
        .hyprbinds-path { font-size: 0.74em; opacity: 0.34; margin-right: 6px; }
        .hyprbinds-count {
            font-size: 0.74em;
            opacity: 0.48;
            padding: 2px 7px;
            border-radius: 6px;
            background-color: @hb_surface_2;
        }
        .hyprbinds-header-toggle { opacity: 0.7; font-size: 0.86em; }
        .hyprbinds-content { padding-top: 10px; }

        .hyprbinds-sidebar {
            min-width: 270px;
            max-width: 270px;
            padding: 15px 11px 13px 11px;
            border-right: 1px solid @hb_border;
            background-color: alpha(currentColor, 0.018);
        }
        .hyprbinds-brand { padding: 2px 7px 10px 7px; }
        .hyprbinds-brand-mark {
            min-width: 34px;
            min-height: 34px;
            border-radius: 10px;
            background-color: @hb_accent_soft;
            color: @hb_accent;
            font-size: 1.05em;
            font-weight: 800;
        }
        .hyprbinds-brand-title { font-size: 1.03em; font-weight: 720; letter-spacing: -0.02em; }
        .hyprbinds-brand-sub { font-size: 0.72em; opacity: 0.42; }
        .hyprbinds-sidebar-filter {
            min-height: 32px;
            margin: 0 4px 7px 4px;
            border-radius: 8px;
            background-color: alpha(currentColor, 0.028);
        }
        list.hyprbinds-sidebar-list,
        .hyprbinds-sidebar-scroll { background: transparent; border: none; }
        list.hyprbinds-sidebar-list > row { border-radius: 9px; margin: 1px 0; border: none; }
        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-header-row {
            margin-top: 11px;
            margin-bottom: 2px;
            background: transparent;
        }
        .hyprbinds-sidebar-section {
            padding: 3px 9px;
            font-size: 0.64em;
            font-weight: 760;
            letter-spacing: 0.105em;
            opacity: 0.33;
        }
        .hyprbinds-sidebar-row-inner { padding: 7px 9px; }
        .hyprbinds-sidebar-icon { min-width: 1.6em; font-size: 1em; opacity: 0.62; }
        .hyprbinds-sidebar-label { font-size: 0.9em; font-weight: 610; }
        .hyprbinds-sidebar-subtitle { font-size: 0.70em; opacity: 0.38; }
        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-row:hover { background-color: @hb_hover; }
        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-row:selected,
        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-row.nav-active {
            background-color: @hb_accent_soft;
            box-shadow: inset 2px 0 0 @hb_accent;
        }
        list.hyprbinds-sidebar-list > row.nav-active .hyprbinds-sidebar-label { color: @hb_accent; }
        list.hyprbinds-sidebar-list > row.nav-active .hyprbinds-sidebar-subtitle,
        list.hyprbinds-sidebar-list > row.nav-active .hyprbinds-sidebar-icon { opacity: 0.78; }

        .hyprbinds-page { padding: 3px 0 0 0; }
        .hyprbinds-page-header { padding: 2px 1px 7px 1px; }
        .hyprbinds-page-eyebrow,
        .hyprbinds-hero-kicker {
            font-size: 0.62em;
            font-weight: 800;
            letter-spacing: 0.14em;
            color: @hb_accent;
            opacity: 0.82;
        }
        .hyprbinds-page-title {
            font-size: 1.82em;
            font-weight: 760;
            letter-spacing: -0.045em;
        }
        .hyprbinds-page-hint {
            font-size: 0.88em;
            opacity: 0.46;
            line-height: 1.4;
            max-width: 62em;
        }
        .hyprbinds-toolbar {
            padding: 6px 0 9px 0;
            margin: 0;
            border: none;
            border-bottom: 1px solid @hb_border_soft;
            background: transparent;
        }
        .hyprbinds-toolbar entry,
        .hyprbinds-toolbar button { min-height: 31px; }
        .hyprbinds-page-canvas { padding-top: 10px; background: transparent; }

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
            margin-bottom: 7px;
            border-bottom: 1px solid @hb_border_soft;
        }
        notebook.hyprbinds-hub > header tab,
        notebook.hyprbinds-bind-tabs > header tab {
            min-height: 27px;
            padding: 3px 10px 5px 10px;
            border: none;
            border-radius: 0;
            opacity: 0.48;
        }
        notebook.hyprbinds-hub > header tab:checked,
        notebook.hyprbinds-bind-tabs > header tab:checked {
            opacity: 1;
            color: @hb_accent;
            box-shadow: inset 0 -2px 0 @hb_accent;
            background: transparent;
        }

        list.boxed-list {
            margin-top: 2px;
            border-radius: 10px;
            background-color: alpha(currentColor, 0.015);
            border: 1px solid @hb_border_soft;
            box-shadow: none;
        }
        list.boxed-list > row { border-bottom: 1px solid @hb_border_soft; }
        list.boxed-list > row:hover { background-color: @hb_hover; }
        list.boxed-list > row:selected {
            background-color: @hb_accent_soft;
            box-shadow: inset 2px 0 0 @hb_accent;
        }
        .hyprbinds-row-title { font-size: 0.95em; font-weight: 630; }
        .hyprbinds-row-sub { font-size: 0.81em; opacity: 0.58; }
        .hyprbinds-row-body { font-size: 0.81em; opacity: 0.52; }
        .hyprbinds-row-meta { font-size: 0.72em; opacity: 0.34; }
        .hyprbinds-row-keys {
            padding: 2px 8px;
            border-radius: 6px;
            background-color: @hb_accent_soft;
            border: 1px solid alpha(@hb_accent, 0.13);
            font-size: 0.78em;
            font-weight: 650;
        }
        .hyprbinds-section {
            margin-top: 14px;
            margin-bottom: 5px;
            font-size: 0.67em;
            font-weight: 760;
            letter-spacing: 0.10em;
            opacity: 0.38;
        }

        .hyprbinds-dashboard { padding: 4px 1px; }
        .hyprbinds-hero {
            padding: 18px 20px;
            border-radius: 14px;
            background-color: alpha(@hb_accent, 0.055);
            border: 1px solid alpha(@hb_accent, 0.13);
        }
        .hyprbinds-hero-title {
            font-size: 1.9em;
            font-weight: 780;
            letter-spacing: -0.05em;
        }
        .hyprbinds-hero-sub { max-width: 52em; font-size: 0.88em; opacity: 0.48; }
        .hyprbinds-metrics { margin-top: 1px; }
        .hyprbinds-metric {
            padding: 13px 15px;
            border-radius: 11px;
            border: 1px solid @hb_border_soft;
            background-color: alpha(currentColor, 0.016);
        }
        .hyprbinds-metric-value {
            font-size: 1.36em;
            font-weight: 760;
            letter-spacing: -0.035em;
        }
        .hyprbinds-metric-label { font-size: 0.72em; opacity: 0.4; }
        .hyprbinds-dashboard-panel {
            padding: 13px 15px;
            border-radius: 12px;
            border: 1px solid @hb_border_soft;
            background-color: alpha(currentColor, 0.016);
        }
        .hyprbinds-dashboard-note { padding-top: 10px; font-size: 0.76em; opacity: 0.4; }
        .hyprbinds-speed-value { font-size: 1.05em; font-weight: 720; color: @hb_accent; }

        .hyprbinds-settings-card,
        .hyprbinds-dialog-section {
            background-color: alpha(currentColor, 0.016);
            border: 1px solid @hb_border_soft;
            border-radius: 11px;
            box-shadow: none;
        }
        .hyprbinds-settings-card { padding: 5px 3px 6px 3px; }
        .hyprbinds-settings-card-title,
        .hyprbinds-dialog-section-title {
            font-size: 0.67em;
            font-weight: 760;
            letter-spacing: 0.09em;
            opacity: 0.38;
        }
        .hyprbinds-settings-row { min-height: 39px; padding: 9px 12px; }
        .hyprbinds-settings-row:hover { background-color: alpha(currentColor, 0.018); }
        .hyprbinds-settings-title { font-size: 0.93em; font-weight: 620; }
        .hyprbinds-settings-sub { font-size: 0.78em; opacity: 0.43; }

        button.suggested-action {
            min-height: 29px;
            padding: 3px 12px;
            border-radius: 8px;
            background-image: none;
            background-color: @hb_accent;
            color: #031218;
            font-weight: 660;
            border: none;
            box-shadow: none;
        }
        button.suggested-action:hover { background-color: #35e0ff; }
        button.destructive-action {
            min-height: 29px;
            border-radius: 8px;
            background-color: transparent;
            color: @error_color;
            border: 1px solid alpha(@error_color, 0.11);
        }
        button.destructive-action:hover { background-color: alpha(@error_color, 0.09); }

        .hyprbinds-status-sep { margin-top: 4px; opacity: 0.24; }
        .hyprbinds-status { min-height: 18px; padding-top: 5px; font-size: 0.77em; opacity: 0.42; }
        .hyprbinds-sticky-footer {
            background-color: @window_bg_color;
            border-top: 1px solid @hb_border_soft;
        }
        .hyprbinds-palette-search { min-height: 36px; padding: 5px 11px; font-size: 1em; }
        .hyprbinds-health-badge,
        .hyprbinds-conflict-badge { border-radius: 6px; }
        .curve-graph {
            border-radius: 10px;
            border-color: @hb_border;
            background-color: alpha(currentColor, 0.015);
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
