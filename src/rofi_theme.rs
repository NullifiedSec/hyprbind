//! Rasi theme tokens, rendering, and built-in palettes for Rofi Studio.

use serde::{Deserialize, Serialize};

pub const ORIENTATIONS: &[&str] = &["vertical", "horizontal"];

pub const LAYOUT_PRESET_LABELS: &[&str] = &[
    "Classic list",
    "Compact",
    "App grid",
    "Horizontal strip",
    "Fullscreen",
];

pub const LAYOUT_PRESET_IDS: &[&str] = &[
    "classic",
    "compact",
    "grid",
    "horizontal",
    "fullscreen",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeTokens {
    pub background: String,
    pub foreground: String,
    pub selected_bg: String,
    pub selected_fg: String,
    pub active: String,
    pub urgent: String,
    pub border_color: String,
    pub border: String,
    pub border_radius: String,
    pub width: String,
    #[serde(default)]
    pub height: String,
    pub padding: String,
    pub spacing: String,
    pub element_padding: String,
    pub lines: u32,
    #[serde(default = "default_columns")]
    pub columns: u32,
    #[serde(default = "default_orientation")]
    pub orientation: String,
    #[serde(default)]
    pub fullscreen: bool,
    #[serde(default = "default_true")]
    pub fixed_height: bool,
    #[serde(default)]
    pub scrollbar: bool,
    #[serde(default = "default_icon_size")]
    pub icon_size: String,
}

fn default_columns() -> u32 {
    1
}
fn default_orientation() -> String {
    "vertical".into()
}
fn default_true() -> bool {
    true
}
fn default_icon_size() -> String {
    "1.2em".into()
}

impl Default for ThemeTokens {
    fn default() -> Self {
        Self::defaults()
    }
}

impl ThemeTokens {
    pub fn defaults() -> Self {
        Self {
            background: "#1e1e2e".into(),
            foreground: "#cdd6f4".into(),
            selected_bg: "#89b4fa".into(),
            selected_fg: "#1e1e2e".into(),
            active: "#a6e3a1".into(),
            urgent: "#f38ba8".into(),
            border_color: "#45475a".into(),
            border: "2px".into(),
            border_radius: "12px".into(),
            width: "40em".into(),
            height: String::new(),
            padding: "12px".into(),
            spacing: "6px".into(),
            element_padding: "8px 12px".into(),
            lines: 10,
            columns: 1,
            orientation: "vertical".into(),
            fullscreen: false,
            fixed_height: true,
            scrollbar: false,
            icon_size: "1.2em".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RofiTheme {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub tokens: ThemeTokens,
}

fn palette(
    id: &'static str,
    name: &'static str,
    description: &'static str,
    background: &str,
    foreground: &str,
    selected_bg: &str,
    selected_fg: &str,
    active: &str,
    urgent: &str,
    border_color: &str,
) -> RofiTheme {
    RofiTheme {
        id,
        name,
        description,
        tokens: ThemeTokens {
            background: background.into(),
            foreground: foreground.into(),
            selected_bg: selected_bg.into(),
            selected_fg: selected_fg.into(),
            active: active.into(),
            urgent: urgent.into(),
            border_color: border_color.into(),
            ..ThemeTokens::defaults()
        },
    }
}

pub fn all_themes() -> Vec<RofiTheme> {
    vec![
        palette(
            "catppuccin-mocha",
            "Catppuccin Mocha",
            "Soft dark Mocha palette",
            "#1e1e2e",
            "#cdd6f4",
            "#cba6f7",
            "#1e1e2e",
            "#a6e3a1",
            "#f38ba8",
            "#45475a",
        ),
        palette(
            "nord-frost",
            "Nord Frost",
            "Cool Nord-inspired launcher",
            "#2e3440",
            "#eceff4",
            "#88c0d0",
            "#2e3440",
            "#a3be8c",
            "#bf616a",
            "#4c566a",
        ),
        palette(
            "gruvbox-dark",
            "Gruvbox Dark",
            "Warm gruvbox menu",
            "#282828",
            "#ebdbb2",
            "#fabd2f",
            "#282828",
            "#b8bb26",
            "#fb4934",
            "#504945",
        ),
        palette(
            "tokyo-night",
            "Tokyo Night",
            "Tokyo Night storm accents",
            "#1a1b26",
            "#c0caf5",
            "#7aa2f7",
            "#1a1b26",
            "#9ece6a",
            "#f7768e",
            "#414868",
        ),
        palette(
            "dracula",
            "Dracula",
            "Classic Dracula purple select",
            "#282a36",
            "#f8f8f2",
            "#bd93f9",
            "#282a36",
            "#50fa7b",
            "#ff5555",
            "#44475a",
        ),
        palette(
            "minimal-dark",
            "Minimal Dark",
            "Flat dark, sharp corners",
            "#111111",
            "#eeeeee",
            "#eeeeee",
            "#111111",
            "#7aa2f7",
            "#ff5555",
            "#333333",
        ),
    ]
}

pub fn theme_by_id(id: &str) -> Option<RofiTheme> {
    all_themes().into_iter().find(|t| t.id == id)
}

/// Apply a layout preset onto existing tokens (keeps colors).
pub fn apply_layout_preset(tokens: &mut ThemeTokens, preset_id: &str) {
    match preset_id {
        "compact" => {
            tokens.width = "32em".into();
            tokens.height.clear();
            tokens.lines = 8;
            tokens.columns = 1;
            tokens.orientation = "vertical".into();
            tokens.fullscreen = false;
            tokens.fixed_height = true;
            tokens.scrollbar = false;
            tokens.padding = "8px".into();
            tokens.spacing = "4px".into();
            tokens.element_padding = "6px 10px".into();
            tokens.border_radius = "8px".into();
            tokens.icon_size = "1.0em".into();
        }
        "grid" => {
            tokens.width = "52em".into();
            tokens.height = "28em".into();
            tokens.lines = 4;
            tokens.columns = 4;
            tokens.orientation = "vertical".into();
            tokens.fullscreen = false;
            tokens.fixed_height = true;
            tokens.scrollbar = true;
            tokens.padding = "14px".into();
            tokens.spacing = "10px".into();
            tokens.element_padding = "10px".into();
            tokens.border_radius = "14px".into();
            tokens.icon_size = "2.4em".into();
        }
        "horizontal" => {
            tokens.width = "70%".into();
            tokens.height = "4.5em".into();
            tokens.lines = 1;
            tokens.columns = 8;
            tokens.orientation = "horizontal".into();
            tokens.fullscreen = false;
            tokens.fixed_height = true;
            tokens.scrollbar = false;
            tokens.padding = "10px".into();
            tokens.spacing = "8px".into();
            tokens.element_padding = "8px 14px".into();
            tokens.border_radius = "16px".into();
            tokens.icon_size = "1.4em".into();
        }
        "fullscreen" => {
            tokens.width = "100%".into();
            tokens.height = "100%".into();
            tokens.lines = 14;
            tokens.columns = 1;
            tokens.orientation = "vertical".into();
            tokens.fullscreen = true;
            tokens.fixed_height = true;
            tokens.scrollbar = true;
            tokens.padding = "24px".into();
            tokens.spacing = "10px".into();
            tokens.element_padding = "10px 16px".into();
            tokens.border_radius = "0".into();
            tokens.icon_size = "1.4em".into();
        }
        _ => {
            // classic
            tokens.width = "40em".into();
            tokens.height.clear();
            tokens.lines = 10;
            tokens.columns = 1;
            tokens.orientation = "vertical".into();
            tokens.fullscreen = false;
            tokens.fixed_height = true;
            tokens.scrollbar = false;
            tokens.padding = "12px".into();
            tokens.spacing = "6px".into();
            tokens.element_padding = "8px 12px".into();
            tokens.border_radius = "12px".into();
            tokens.icon_size = "1.2em".into();
        }
    }
}

/// Render a complete managed theme file.
pub fn render_theme(tokens: &ThemeTokens, font: &str) -> String {
    let height_line = if tokens.height.trim().is_empty() {
        String::new()
    } else {
        format!("    height:           {};\n", tokens.height.trim())
    };
    let fullscreen = if tokens.fullscreen { "true" } else { "false" };
    let fixed_height = if tokens.fixed_height { "true" } else { "false" };
    let scrollbar = if tokens.scrollbar { "true" } else { "false" };
    let orientation = if tokens.orientation == "horizontal" {
        "horizontal"
    } else {
        "vertical"
    };
    let columns = tokens.columns.max(1);
    let lines = tokens.lines.max(1);

    format!(
        r#"/**
 * hyprbinds-theme.rasi — managed by Hyprbinds Rofi Studio.
 * Re-apply from the Studio to regenerate; manual edits may be overwritten.
 */
* {{
    background:                  {background};
    foreground:                  {foreground};
    selected-background:         {selected_bg};
    selected-foreground:         {selected_fg};
    active:                      {active};
    urgent:                      {urgent};
    border-color:                {border_color};
    background-color:            transparent;
    text-color:                  {foreground};
    separatorcolor:              {border_color};
    spacing:                     {spacing};
}}

window {{
    width:            {width};
{height_line}    border:           {border};
    border-color:     {border_color};
    border-radius:    {border_radius};
    padding:          {padding};
    background-color: {background};
    transparency:     "real";
    fullscreen:       {fullscreen};
}}

mainbox {{
    spacing:          {spacing};
    padding:          0;
    background-color: transparent;
    children:         [ "inputbar", "message", "listview" ];
}}

inputbar {{
    spacing:          {spacing};
    padding:          {element_padding};
    border-radius:    {border_radius};
    background-color: {background};
    text-color:       {foreground};
    children:         [ "prompt", "entry" ];
}}

prompt {{
    padding:          0 8px 0 0;
    background-color: transparent;
    text-color:       {selected_bg};
    font:             "{font}";
}}

entry {{
    background-color: transparent;
    text-color:       {foreground};
    placeholder:      "Search…";
    placeholder-color: {border_color};
    cursor:           text;
    font:             "{font}";
}}

listview {{
    lines:            {lines};
    columns:          {columns};
    orientation:      {orientation};
    fixed-height:     {fixed_height};
    spacing:          {spacing};
    scrollbar:        {scrollbar};
    background-color: transparent;
}}

element {{
    padding:          {element_padding};
    spacing:          8px;
    border-radius:    {border_radius};
    background-color: transparent;
    text-color:       {foreground};
    cursor:           pointer;
    orientation:      horizontal;
}}

element selected {{
    background-color: {selected_bg};
    text-color:       {selected_fg};
}}

element active {{
    text-color:       {active};
}}

element urgent {{
    text-color:       {urgent};
}}

element-text {{
    background-color: transparent;
    text-color:       inherit;
    font:             "{font}";
    highlight:        bold {selected_bg};
    vertical-align:   0.5;
}}

element-icon {{
    size:             {icon_size};
    background-color: transparent;
    text-color:       inherit;
}}

message {{
    padding:          {element_padding};
    border-radius:    {border_radius};
    background-color: {background};
    text-color:       {foreground};
}}

textbox {{
    background-color: transparent;
    text-color:       inherit;
}}
"#,
        background = tokens.background,
        foreground = tokens.foreground,
        selected_bg = tokens.selected_bg,
        selected_fg = tokens.selected_fg,
        active = tokens.active,
        urgent = tokens.urgent,
        border_color = tokens.border_color,
        spacing = tokens.spacing,
        width = tokens.width,
        border = tokens.border,
        border_radius = tokens.border_radius,
        padding = tokens.padding,
        element_padding = tokens.element_padding,
        icon_size = tokens.icon_size,
    )
}

/// Best-effort extraction of tokens from a rasi theme string.
pub fn extract_tokens(rasi: &str) -> ThemeTokens {
    let mut t = ThemeTokens::defaults();
    if let Some(v) = first_prop(rasi, "background") {
        t.background = v;
    }
    if let Some(v) = first_prop(rasi, "foreground") {
        t.foreground = v;
    }
    if let Some(v) = first_prop(rasi, "selected-background")
        .or_else(|| first_prop(rasi, "selected-normal-background"))
    {
        t.selected_bg = v;
    }
    if let Some(v) = first_prop(rasi, "selected-foreground")
        .or_else(|| first_prop(rasi, "selected-normal-foreground"))
    {
        t.selected_fg = v;
    }
    if let Some(v) = first_prop(rasi, "active").or_else(|| first_prop(rasi, "blue")) {
        t.active = v;
    }
    if let Some(v) = first_prop(rasi, "urgent").or_else(|| first_prop(rasi, "red")) {
        t.urgent = v;
    }
    if let Some(v) = first_prop(rasi, "border-color") {
        t.border_color = v;
    }
    if let Some(v) = block_prop(rasi, "window", "border") {
        t.border = v;
    }
    if let Some(v) = block_prop(rasi, "window", "border-radius") {
        t.border_radius = v;
    }
    if let Some(v) = block_prop(rasi, "window", "width") {
        t.width = v;
    }
    if let Some(v) = block_prop(rasi, "window", "height") {
        t.height = v;
    }
    if let Some(v) = block_prop(rasi, "window", "padding") {
        t.padding = v;
    }
    if let Some(v) = block_prop(rasi, "window", "fullscreen") {
        t.fullscreen = matches!(v.to_lowercase().as_str(), "true" | "1" | "yes");
    }
    if let Some(v) = first_prop(rasi, "spacing") {
        t.spacing = v;
    }
    if let Some(v) = block_prop(rasi, "element", "padding") {
        t.element_padding = v;
    }
    if let Some(v) = block_prop(rasi, "listview", "lines") {
        if let Ok(n) = v.trim().parse::<u32>() {
            t.lines = n;
        }
    }
    if let Some(v) = block_prop(rasi, "listview", "columns") {
        if let Ok(n) = v.trim().parse::<u32>() {
            t.columns = n.max(1);
        }
    }
    if let Some(v) = block_prop(rasi, "listview", "orientation") {
        t.orientation = v;
    }
    if let Some(v) = block_prop(rasi, "listview", "fixed-height") {
        t.fixed_height = matches!(v.to_lowercase().as_str(), "true" | "1" | "yes");
    }
    if let Some(v) = block_prop(rasi, "listview", "scrollbar") {
        t.scrollbar = matches!(v.to_lowercase().as_str(), "true" | "1" | "yes");
    }
    if let Some(v) = block_prop(rasi, "element-icon", "size") {
        t.icon_size = v;
    }
    t
}

fn strip_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if c == '\\' {
                if let Some(n) = chars.next() {
                    out.push(n);
                }
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        if c == '"' {
            in_string = true;
            out.push(c);
            continue;
        }
        if c == '/' && chars.peek() == Some(&'/') {
            chars.next();
            for nc in chars.by_ref() {
                if nc == '\n' {
                    out.push('\n');
                    break;
                }
            }
            continue;
        }
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            while let Some(nc) = chars.next() {
                if nc == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    break;
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

fn first_prop(rasi: &str, key: &str) -> Option<String> {
    let cleaned = strip_comments(rasi);
    let needle = format!("{key}:");
    for line in cleaned.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&needle) {
            return Some(normalize_value(rest));
        }
        if let Some(idx) = trimmed.find(&needle) {
            let before = &trimmed[..idx];
            if before.is_empty() || before.ends_with(' ') || before.ends_with('{') {
                return Some(normalize_value(&trimmed[idx + needle.len()..]));
            }
        }
    }
    None
}

fn block_prop(rasi: &str, block: &str, key: &str) -> Option<String> {
    let cleaned = strip_comments(rasi);
    let start_pat = format!("{block} {{");
    let start = cleaned.find(&start_pat)?;
    let after = &cleaned[start + start_pat.len()..];
    let end = after.find('}')?;
    let body = &after[..end];
    let needle = format!("{key}:");
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&needle) {
            return Some(normalize_value(rest));
        }
    }
    None
}

fn normalize_value(raw: &str) -> String {
    let mut v = raw.trim().trim_end_matches(';').trim().to_string();
    if v.starts_with('"') && v.ends_with('"') && v.len() >= 2 {
        v = v[1..v.len() - 1].to_string();
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_theme_extracts_core_colors() {
        let tokens = ThemeTokens::defaults();
        let rasi = render_theme(&tokens, "mono 12");
        let got = extract_tokens(&rasi);
        assert_eq!(got.background, tokens.background);
        assert_eq!(got.foreground, tokens.foreground);
        assert_eq!(got.selected_bg, tokens.selected_bg);
        assert_eq!(got.width, tokens.width);
        assert_eq!(got.lines, tokens.lines);
        assert_eq!(got.columns, tokens.columns);
        assert_eq!(got.orientation, tokens.orientation);
    }

    #[test]
    fn layout_preset_grid_sets_columns() {
        let mut t = ThemeTokens::defaults();
        apply_layout_preset(&mut t, "grid");
        assert_eq!(t.columns, 4);
        assert!(!t.fullscreen);
        apply_layout_preset(&mut t, "fullscreen");
        assert!(t.fullscreen);
    }
}
