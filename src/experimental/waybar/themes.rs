//! Built-in Waybar theme palettes (CSS token packs).

use crate::experimental::waybar::style::StyleTokens;

#[derive(Debug, Clone)]
pub struct WaybarTheme {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub tokens: StyleTokens,
}

pub fn all_themes() -> Vec<WaybarTheme> {
    vec![
        WaybarTheme {
            id: "stealth-pills",
            name: "Stealth Pills",
            description: "Dark translucent pills — matches common Hyprland setups",
            tokens: StyleTokens {
                font_family: "JetBrainsMono Nerd Font".into(),
                font_size: "14px".into(),
                bar_bg: "transparent".into(),
                bar_fg: "#ffffff".into(),
                pill_bg: "rgba(0, 0, 0, 0.75)".into(),
                pill_fg: "#ffffff".into(),
                border_color: "#1a1a1a".into(),
                border_radius: "12px".into(),
                padding: "4px 16px".into(),
                margin: "0 6px".into(),
                accent: "#ffffff".into(),
            },
        },
        WaybarTheme {
            id: "solid-dark",
            name: "Solid Dark",
            description: "Opaque dark bar with sharp modules",
            tokens: StyleTokens {
                font_family: "JetBrainsMono Nerd Font".into(),
                font_size: "13px".into(),
                bar_bg: "#11111b".into(),
                bar_fg: "#cdd6f4".into(),
                pill_bg: "#1e1e2e".into(),
                pill_fg: "#cdd6f4".into(),
                border_color: "#313244".into(),
                border_radius: "6px".into(),
                padding: "2px 10px".into(),
                margin: "4px 4px".into(),
                accent: "#89b4fa".into(),
            },
        },
        WaybarTheme {
            id: "catppuccin-mocha",
            name: "Catppuccin Mocha",
            description: "Mocha palette islands",
            tokens: StyleTokens {
                font_family: "JetBrainsMono Nerd Font".into(),
                font_size: "13px".into(),
                bar_bg: "transparent".into(),
                bar_fg: "#cdd6f4".into(),
                pill_bg: "#1e1e2e".into(),
                pill_fg: "#cdd6f4".into(),
                border_color: "#45475a".into(),
                border_radius: "10px".into(),
                padding: "4px 12px".into(),
                margin: "4px 6px".into(),
                accent: "#cba6f7".into(),
            },
        },
        WaybarTheme {
            id: "nord-frost",
            name: "Nord Frost",
            description: "Cool Nord-inspired bar",
            tokens: StyleTokens {
                font_family: "JetBrainsMono Nerd Font".into(),
                font_size: "13px".into(),
                bar_bg: "#2e3440".into(),
                bar_fg: "#eceff4".into(),
                pill_bg: "#3b4252".into(),
                pill_fg: "#eceff4".into(),
                border_color: "#4c566a".into(),
                border_radius: "8px".into(),
                padding: "3px 12px".into(),
                margin: "3px 4px".into(),
                accent: "#88c0d0".into(),
            },
        },
        WaybarTheme {
            id: "gruvbox-soft",
            name: "Gruvbox Soft",
            description: "Warm gruvbox modules",
            tokens: StyleTokens {
                font_family: "JetBrainsMono Nerd Font".into(),
                font_size: "13px".into(),
                bar_bg: "#282828".into(),
                bar_fg: "#ebdbb2".into(),
                pill_bg: "#3c3836".into(),
                pill_fg: "#ebdbb2".into(),
                border_color: "#504945".into(),
                border_radius: "8px".into(),
                padding: "3px 12px".into(),
                margin: "3px 4px".into(),
                accent: "#fabd2f".into(),
            },
        },
        WaybarTheme {
            id: "minimal-flat",
            name: "Minimal Flat",
            description: "Flat transparent bar, no pill borders",
            tokens: StyleTokens {
                font_family: "Inter".into(),
                font_size: "12px".into(),
                bar_bg: "rgba(0, 0, 0, 0.4)".into(),
                bar_fg: "#e0e0e0".into(),
                pill_bg: "transparent".into(),
                pill_fg: "#e0e0e0".into(),
                border_color: "transparent".into(),
                border_radius: "0".into(),
                padding: "0 8px".into(),
                margin: "0 2px".into(),
                accent: "#7aa2f7".into(),
            },
        },
    ]
}

pub fn apply_theme_to_css(css: &str, theme_id: &str) -> Option<String> {
    let theme = all_themes().into_iter().find(|t| t.id == theme_id)?;
    Some(crate::experimental::waybar::style::apply_tokens(css, &theme.tokens))
}
