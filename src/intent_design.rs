use gtk4::prelude::*;
use gtk4::CssProvider;

pub fn apply() {
    let css = r#"
        .hyprbinds-intent-flow {
            padding: 10px 0 4px 0;
            background: transparent;
        }

        .hyprbinds-intent-step {
            min-height: 38px;
            padding: 7px 9px;
            border-radius: 8px;
            border: 1px solid alpha(currentColor, 0.065);
            background-color: alpha(currentColor, 0.018);
        }

        .hyprbinds-intent-number {
            min-width: 22px;
            min-height: 22px;
            border-radius: 11px;
            background-color: alpha(currentColor, 0.065);
            font-size: 0.72em;
            font-weight: 700;
            opacity: 0.72;
        }

        .hyprbinds-intent-label {
            font-size: 0.82em;
            font-weight: 610;
            opacity: 0.76;
        }

        .hyprbinds-intent-advanced {
            margin-left: 2px;
            opacity: 0.38;
        }
    "#;

    let provider = CssProvider::new();
    provider.load_from_string(css);
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_USER + 2,
        );
    }
}
