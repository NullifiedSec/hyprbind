//! Full visual system for Hyprbinds.

use gtk4::prelude::*;
use gtk4::CssProvider;

pub fn apply() {
    let css = r#"
        @define-color hb_accent #00d4ff;
        @define-color hb_accent_hover #43e3ff;
        @define-color hb_accent_soft alpha(#00d4ff, 0.10);
        @define-color hb_accent_faint alpha(#00d4ff, 0.045);
        @define-color hb_surface alpha(currentColor, 0.024);
        @define-color hb_surface_2 alpha(currentColor, 0.040);
        @define-color hb_surface_3 alpha(currentColor, 0.060);
        @define-color hb_hover alpha(currentColor, 0.050);
        @define-color hb_border alpha(currentColor, 0.075);
        @define-color hb_border_soft alpha(currentColor, 0.045);
        @define-color hb_text_soft alpha(currentColor, 0.58);

        window { font-size: 1em; }

        button,
        entry,
        dropdown > button,
        dropdown button.toggle,
        spinbutton {
            min-height: 31px;
            border-radius: 10px;
            box-shadow: none;
        }

        button {
            min-width: 0;
            padding: 4px 11px;
            font-size: 0.88em;
            font-weight: 570;
            border: 1px solid transparent;
            transition: 120ms ease;
        }
        button:hover {
            background-color: @hb_hover;
            border-color: @hb_border;
        }

        entry {
            padding: 4px 10px;
            border: 1px solid @hb_border;
            background-color: @hb_surface_2;
        }
        entry:focus {
            border-color: alpha(@hb_accent, 0.58);
            box-shadow: 0 0 0 2px alpha(@hb_accent, 0.07);
            background-color: @hb_surface_3;
        }

        dropdown > button,
        dropdown button.toggle,
        spinbutton {
            border: 1px solid @hb_border;
            background-color: @hb_surface_2;
        }

        checkbutton { font-size: 0.88em; }
        separator { opacity: 0.28; }

        /* Shell */
        .hyprbinds-shell { background-color: @window_bg_color; }
        .hyprbinds-main { background-color: @window_bg_color; }

        .hyprbinds-header {
            min-height: 40px;
            padding: 5px 24px;
            border-bottom: 1px solid @hb_border_soft;
            background-color: alpha(currentColor, 0.008);
        }
        .hyprbinds-header-title {
            font-size: 0.80em;
            font-weight: 680;
            opacity: 0.38;
            letter-spacing: 0.02em;
        }
        .hyprbinds-path {
            font-size: 0.72em;
            opacity: 0.28;
            margin-right: 8px;
        }
        .hyprbinds-count {
            font-size: 0.72em;
            opacity: 0.70;
            padding: 3px 8px;
            border-radius: 999px;
            background-color: @hb_surface_2;
            border: 1px solid @hb_border_soft;
        }
        .hyprbinds-header-toggle {
            opacity: 0.58;
            font-size: 0.82em;
        }
        .hyprbinds-header button {
            min-height: 28px;
            padding: 2px 9px;
            border-radius: 8px;
        }
        .hyprbinds-content { padding-top: 14px; }

        /* Sidebar */
        .hyprbinds-sidebar {
            min-width: 276px;
            max-width: 276px;
            padding: 17px 12px 14px 12px;
            border-right: 1px solid @hb_border_soft;
            background-color: alpha(currentColor, 0.014);
        }
        .hyprbinds-brand {
            padding: 1px 8px 13px 8px;
        }
        .hyprbinds-brand-mark {
            min-width: 36px;
            min-height: 36px;
            border-radius: 11px;
            background-color: @hb_accent_soft;
            color: @hb_accent;
            font-size: 1.08em;
            font-weight: 820;
            border: 1px solid alpha(@hb_accent, 0.13);
        }
        .hyprbinds-brand-title {
            font-size: 1.04em;
            font-weight: 740;
            letter-spacing: -0.025em;
        }
        .hyprbinds-brand-sub {
            font-size: 0.70em;
            opacity: 0.35;
        }
        .hyprbinds-sidebar-filter {
            min-height: 34px;
            margin: 0 4px 9px 4px;
            border-radius: 11px;
            background-color: @hb_surface_2;
            border: 1px solid @hb_border_soft;
        }
        list.hyprbinds-sidebar-list,
        .hyprbinds-sidebar-scroll {
            background: transparent;
            border: none;
        }
        list.hyprbinds-sidebar-list > row {
            border-radius: 11px;
            margin: 2px 0;
            border: 1px solid transparent;
        }
        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-header-row {
            margin-top: 13px;
            margin-bottom: 3px;
            background: transparent;
            border: none;
        }
        .hyprbinds-sidebar-section {
            padding: 3px 10px;
            font-size: 0.62em;
            font-weight: 780;
            letter-spacing: 0.115em;
            opacity: 0.29;
        }
        .hyprbinds-sidebar-row-inner { padding: 8px 10px; }
        .hyprbinds-sidebar-icon {
            min-width: 1.7em;
            font-size: 1.00em;
            opacity: 0.54;
        }
        .hyprbinds-sidebar-label {
            font-size: 0.91em;
            font-weight: 630;
        }
        .hyprbinds-sidebar-subtitle {
            font-size: 0.69em;
            opacity: 0.34;
        }
        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-row:hover {
            background-color: @hb_hover;
            border-color: @hb_border_soft;
        }
        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-row:selected,
        list.hyprbinds-sidebar-list > row.hyprbinds-sidebar-row.nav-active {
            background-color: @hb_accent_soft;
            border-color: alpha(@hb_accent, 0.12);
            box-shadow: none;
        }
        list.hyprbinds-sidebar-list > row.nav-active .hyprbinds-sidebar-label {
            color: @hb_accent;
        }
        list.hyprbinds-sidebar-list > row.nav-active .hyprbinds-sidebar-subtitle,
        list.hyprbinds-sidebar-list > row.nav-active .hyprbinds-sidebar-icon {
            opacity: 0.82;
        }

        /* Page composition */
        .hyprbinds-page { padding: 5px 2px 2px 2px; }
        .hyprbinds-page-header { padding: 2px 2px 10px 2px; }
        .hyprbinds-page-eyebrow,
        .hyprbinds-hero-kicker {
            font-size: 0.61em;
            font-weight: 820;
            letter-spacing: 0.15em;
            color: @hb_accent;
            opacity: 0.72;
        }
        .hyprbinds-page-title {
            font-size: 1.94em;
            font-weight: 780;
            letter-spacing: -0.050em;
        }
        .hyprbinds-page-hint {
            font-size: 0.89em;
            opacity: 0.44;
            line-height: 1.45;
            max-width: 60em;
        }

        .hyprbinds-toolbar {
            padding: 7px;
            margin: 0 0 3px 0;
            border: 1px solid @hb_border_soft;
            border-radius: 13px;
            background-color: @hb_surface;
        }
        .hyprbinds-toolbar entry {
            min-height: 33px;
            background-color: @hb_surface_2;
        }
        .hyprbinds-toolbar button,
        .hyprbinds-toolbar dropdown > button {
            min-height: 33px;
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
            margin-bottom: 9px;
            border-bottom: 1px solid @hb_border_soft;
        }
        notebook.hyprbinds-hub > header tab,
        notebook.hyprbinds-bind-tabs > header tab {
            min-height: 29px;
            padding: 4px 11px 6px 11px;
            border: none;
            border-radius: 8px 8px 0 0;
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
            color: @hb_accent;
            box-shadow: inset 0 -2px 0 @hb_accent;
            background-color: @hb_accent_faint;
        }

        /* Lists */
        list.boxed-list {
            margin-top: 3px;
            border-radius: 13px;
            background-color: @hb_surface;
            border: 1px solid @hb_border_soft;
            box-shadow: none;
        }
        list.boxed-list > row {
            border-bottom: 1px solid @hb_border_soft;
        }
        list.boxed-list > row:hover { background-color: @hb_hover; }
        list.boxed-list > row:selected {
            background-color: @hb_accent_soft;
            box-shadow: none;
        }
        .hyprbinds-row-title {
            font-size: 0.96em;
            font-weight: 650;
            letter-spacing: -0.008em;
        }
        .hyprbinds-row-sub { font-size: 0.81em; opacity: 0.55; }
        .hyprbinds-row-body { font-size: 0.81em; opacity: 0.48; }
        .hyprbinds-row-meta { font-size: 0.71em; opacity: 0.31; }
        .hyprbinds-row-keys {
            padding: 3px 9px;
            border-radius: 8px;
            background-color: @hb_surface_3;
            border: 1px solid @hb_border;
            font-size: 0.78em;
            font-weight: 680;
        }
        list.boxed-list > row:selected .hyprbinds-row-keys {
            background-color: alpha(@hb_accent, 0.12);
            border-color: alpha(@hb_accent, 0.18);
        }
        .hyprbinds-section {
            margin-top: 15px;
            margin-bottom: 6px;
            font-size: 0.66em;
            font-weight: 780;
            letter-spacing: 0.11em;
            opacity: 0.34;
        }

        /* Dashboard */
        .hyprbinds-dashboard { padding: 4px 2px; }
        .hyprbinds-hero {
            padding: 22px 24px;
            border-radius: 16px;
            background-image: linear-gradient(
                115deg,
                alpha(@hb_accent, 0.085),
                alpha(@hb_accent, 0.028) 48%,
                alpha(currentColor, 0.012)
            );
            border: 1px solid alpha(@hb_accent, 0.12);
        }
        .hyprbinds-hero-title {
            font-size: 2.08em;
            font-weight: 800;
            letter-spacing: -0.055em;
        }
        .hyprbinds-hero-sub {
            max-width: 52em;
            font-size: 0.90em;
            opacity: 0.46;
        }
        .hyprbinds-metrics { margin-top: 3px; }
        .hyprbinds-metric {
            padding: 15px 16px;
            border-radius: 13px;
            border: 1px solid @hb_border_soft;
            background-color: @hb_surface;
        }
        .hyprbinds-metric-value {
            font-size: 1.48em;
            font-weight: 790;
            letter-spacing: -0.040em;
        }
        .hyprbinds-metric-label {
            font-size: 0.70em;
            opacity: 0.34;
            letter-spacing: 0.02em;
        }
        .hyprbinds-dashboard-panel {
            padding: 16px 17px;
            border-radius: 14px;
            border: 1px solid @hb_border_soft;
            background-color: @hb_surface;
        }
        .hyprbinds-dashboard-note {
            padding-top: 11px;
            font-size: 0.74em;
            opacity: 0.36;
        }
        .hyprbinds-speed-value {
            font-size: 1.08em;
            font-weight: 750;
            color: @hb_accent;
        }

        /* Settings surfaces */
        .hyprbinds-settings-card,
        .hyprbinds-dialog-section {
            background-color: @hb_surface;
            border: 1px solid @hb_border_soft;
            border-radius: 13px;
            box-shadow: none;
        }
        .hyprbinds-settings-card { padding: 6px 4px 7px 4px; }
        .hyprbinds-settings-card-title,
        .hyprbinds-dialog-section-title {
            font-size: 0.66em;
            font-weight: 780;
            letter-spacing: 0.10em;
            opacity: 0.34;
        }
        .hyprbinds-settings-row {
            min-height: 41px;
            padding: 10px 13px;
            border-radius: 9px;
        }
        .hyprbinds-settings-row:hover { background-color: @hb_hover; }
        .hyprbinds-settings-title { font-size: 0.94em; font-weight: 640; }
        .hyprbinds-settings-sub { font-size: 0.78em; opacity: 0.40; }

        /* Actions */
        button.suggested-action {
            min-height: 31px;
            padding: 4px 13px;
            border-radius: 10px;
            background-image: none;
            background-color: @hb_accent;
            color: #031218;
            font-weight: 690;
            border: 1px solid transparent;
            box-shadow: none;
        }
        button.suggested-action:hover {
            background-color: @hb_accent_hover;
            border-color: transparent;
        }
        button.destructive-action {
            min-height: 31px;
            border-radius: 10px;
            background-color: transparent;
            color: @error_color;
            border: 1px solid alpha(@error_color, 0.10);
        }
        button.destructive-action:hover {
            background-color: alpha(@error_color, 0.08);
            border-color: alpha(@error_color, 0.15);
        }

        /* Footer / palette / diagnostics */
        .hyprbinds-status-sep { margin-top: 5px; opacity: 0.20; }
        .hyprbinds-status {
            min-height: 18px;
            padding-top: 5px;
            font-size: 0.76em;
            opacity: 0.36;
        }
        .hyprbinds-sticky-footer {
            background-color: @window_bg_color;
            border-top: 1px solid @hb_border_soft;
        }
        .hyprbinds-palette-search {
            min-height: 38px;
            padding: 6px 12px;
            border-radius: 12px;
            font-size: 1em;
        }
        .hyprbinds-health-badge,
        .hyprbinds-conflict-badge {
            border-radius: 999px;
            padding: 3px 8px;
        }
        .curve-graph {
            border-radius: 12px;
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
