//! CSS token extraction and rewriting for Waybar style.css.

pub const DEFAULT_STYLE: &str = r#"* {
    font-family: "JetBrainsMono Nerd Font", "Font Awesome 6 Free", sans-serif;
    font-size: 13px;
    font-weight: 600;
    border: none;
    border-radius: 0;
    min-height: 0;
}

window#waybar {
    background-color: transparent;
    color: #ffffff;
}

tooltip {
    background: rgba(0, 0, 0, 0.92);
    border: 1px solid #1a1a1a;
    border-radius: 10px;
}

tooltip label {
    color: #ffffff;
}

#window,
#clock,
#cpu,
#memory,
#pulseaudio,
#network,
#bluetooth,
#battery,
#tray {
    background-color: rgba(0, 0, 0, 0.75);
    color: #ffffff;
    padding: 4px 12px;
    margin: 0 4px;
    border-radius: 10px;
    border: 1px solid #1a1a1a;
}

#workspaces {
    background-color: rgba(0, 0, 0, 0.75);
    padding: 2px 6px;
    margin: 0 4px;
    border-radius: 10px;
    border: 1px solid #1a1a1a;
}

#workspaces button {
    padding: 0 8px;
    background: transparent;
    color: #777777;
    border-radius: 8px;
}

#workspaces button:hover {
    color: #ffffff;
    background: rgba(255, 255, 255, 0.08);
}

#workspaces button.active {
    color: #89b4fa;
    background: rgba(137, 180, 250, 0.15);
}

#workspaces button.urgent {
    color: #ff5555;
}

#window {
    color: #dddddd;
}

#battery.warning:not(.charging) {
    color: #f9e2af;
}

#battery.critical:not(.charging) {
    color: #f38ba8;
}

#network.disconnected,
#pulseaudio.muted {
    color: #a6adc8;
}
"#;

#[derive(Debug, Clone, Default)]
pub struct StyleTokens {
    pub font_family: String,
    pub font_size: String,
    pub bar_bg: String,
    pub bar_fg: String,
    pub pill_bg: String,
    pub pill_fg: String,
    pub border_color: String,
    pub border_radius: String,
    pub padding: String,
    pub margin: String,
    pub accent: String,
}

impl StyleTokens {
    pub fn defaults() -> Self {
        Self {
            font_family: "JetBrainsMono Nerd Font".into(),
            font_size: "13px".into(),
            bar_bg: "transparent".into(),
            bar_fg: "#ffffff".into(),
            pill_bg: "rgba(0, 0, 0, 0.75)".into(),
            pill_fg: "#ffffff".into(),
            border_color: "#1a1a1a".into(),
            border_radius: "10px".into(),
            padding: "4px 12px".into(),
            margin: "0 4px".into(),
            accent: "#89b4fa".into(),
        }
    }
}

/// Extract common tokens from CSS using simple regex-like scans.
pub fn extract_tokens(css: &str) -> StyleTokens {
    let mut t = StyleTokens::defaults();

    if let Some(v) = first_prop(css, "*", "font-family") {
        t.font_family = unquote(&v);
    }
    if let Some(v) = first_prop(css, "*", "font-size") {
        t.font_size = v;
    }
    if let Some(v) = first_prop(css, "window#waybar", "background-color")
        .or_else(|| first_prop(css, "window#waybar", "background"))
    {
        t.bar_bg = v;
    }
    if let Some(v) = first_prop(css, "window#waybar", "color") {
        t.bar_fg = v;
    }

    // Prefer common pill selectors.
    for sel in [
        "#clock",
        "#cpu",
        "#memory",
        "#workspaces",
        "#pulseaudio",
        "#battery",
    ] {
        if let Some(v) = first_prop(css, sel, "background-color") {
            t.pill_bg = v;
            break;
        }
    }
    for sel in ["#clock", "#cpu", "#memory", "#workspaces"] {
        if let Some(v) = first_prop(css, sel, "color") {
            t.pill_fg = v;
            break;
        }
    }
    for sel in ["#clock", "#workspaces", "tooltip"] {
        if let Some(v) = first_prop(css, sel, "border") {
            if let Some(color) = extract_color_from_border(&v) {
                t.border_color = color;
                break;
            }
        }
        if let Some(v) = first_prop(css, sel, "border-color") {
            t.border_color = v;
            break;
        }
    }
    for sel in ["#clock", "#workspaces", "#cpu"] {
        if let Some(v) = first_prop(css, sel, "border-radius") {
            t.border_radius = v;
            break;
        }
    }
    for sel in ["#clock", "#cpu", "#memory"] {
        if let Some(v) = first_prop(css, sel, "padding") {
            t.padding = v;
            break;
        }
    }
    for sel in ["#clock", "#cpu", "#memory", "#workspaces"] {
        if let Some(v) = first_prop(css, sel, "margin") {
            t.margin = v;
            break;
        }
    }
    if let Some(v) = first_prop(css, "#workspaces button.active", "color")
        .or_else(|| first_prop(css, "#clock:hover", "color"))
    {
        t.accent = v;
    }

    t
}

/// Apply token values into CSS. Rewrites known props when found; appends a managed block otherwise.
pub fn apply_tokens(css: &str, tokens: &StyleTokens) -> String {
    let mut out = css.to_string();

    out = set_or_note(
        &out,
        "*",
        "font-family",
        &format!("\"{}\"", tokens.font_family.trim_matches('"')),
    );
    out = set_or_note(&out, "*", "font-size", &tokens.font_size);
    out = set_or_note(&out, "window#waybar", "background-color", &tokens.bar_bg);
    out = set_or_note(&out, "window#waybar", "color", &tokens.bar_fg);

    let pill_sels = [
        "#workspaces",
        "#window",
        "#clock",
        "#cpu",
        "#memory",
        "#custom-multitool",
        "#bluetooth",
        "#pulseaudio",
        "#battery",
        "#tray",
        "#network",
        "#custom-power",
    ];

    for sel in pill_sels {
        if block_exists(&out, sel) {
            out = set_prop_in_block(&out, sel, "background-color", &tokens.pill_bg);
            out = set_prop_in_block(&out, sel, "color", &tokens.pill_fg);
            out = set_prop_in_block(&out, sel, "border-radius", &tokens.border_radius);
            out = set_prop_in_block(&out, sel, "padding", &tokens.padding);
            out = set_prop_in_block(&out, sel, "margin", &tokens.margin);
            // border shorthand if present
            if prop_exists_in_block(&out, sel, "border") {
                out = set_prop_in_block(
                    &out,
                    sel,
                    "border",
                    &format!("1px solid {}", tokens.border_color),
                );
            }
        }
    }

    // Ensure managed token block for preview / missing selectors.
    // NOTE: do not set `color` on `#workspaces` — it fights button/label colors in GTK.
    let managed = format!(
        "\n/* >>> hyprbinds:waybar-tokens */\n\
window#waybar {{ background-color: {bar_bg}; color: {bar_fg}; }}\n\
#clock, #cpu, #memory, #pulseaudio, #battery, #tray, #bluetooth, #network, #window {{\n\
    background-color: {pill_bg};\n\
    color: {pill_fg};\n\
    border-radius: {radius};\n\
    padding: {padding};\n\
    margin: {margin};\n\
    border: 1px solid {border};\n\
}}\n\
#workspaces {{\n\
    background-color: {pill_bg};\n\
    border-radius: {radius};\n\
    padding: {padding};\n\
    margin: {margin};\n\
    border: 1px solid {border};\n\
}}\n\
/* <<< hyprbinds:waybar-tokens */\n",
        bar_bg = tokens.bar_bg,
        bar_fg = tokens.bar_fg,
        pill_bg = tokens.pill_bg,
        pill_fg = tokens.pill_fg,
        radius = tokens.border_radius,
        padding = tokens.padding,
        margin = tokens.margin,
        border = tokens.border_color,
    );

    if let Some(start) = out.find("/* >>> hyprbinds:waybar-tokens */") {
        if let Some(end_rel) = out[start..].find("/* <<< hyprbinds:waybar-tokens */") {
            let end = start + end_rel + "/* <<< hyprbinds:waybar-tokens */".len();
            out.replace_range(start..end, managed.trim());
        } else {
            out.push_str(&managed);
        }
    } else {
        out.push_str(&managed);
    }

    out
}

fn unquote(s: &str) -> String {
    s.trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

fn extract_color_from_border(border: &str) -> Option<String> {
    // e.g. "1px solid #1a1a1a" or "1px solid rgba(...)"
    let parts: Vec<&str> = border.split_whitespace().collect();
    if parts.len() >= 3 {
        Some(parts[2..].join(" "))
    } else {
        None
    }
}

fn find_block<'a>(css: &'a str, selector: &str) -> Option<(usize, usize)> {
    // Find `selector { ... }` — first occurrence, brace-matched.
    let mut search = css;
    let mut offset = 0usize;
    loop {
        let Some(rel) = search.find(selector) else {
            return None;
        };
        let start = offset + rel;
        let after = &css[start + selector.len()..];
        let trimmed = after.trim_start();
        // Allow comma-separated selector lists containing our selector.
        let brace_rel = trimmed.find('{')?;
        // Heuristic: selector should appear before `{` without another `{`.
        let between = &css[start..start + selector.len() + (after.len() - trimmed.len()) + brace_rel];
        if between.contains('{') {
            offset = start + selector.len();
            search = &css[offset..];
            continue;
        }
        let open = start + selector.len() + (after.len() - trimmed.len()) + brace_rel;
        let mut depth = 0i32;
        let bytes = css.as_bytes();
        let mut i = open;
        while i < bytes.len() {
            match bytes[i] as char {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some((open, i));
                    }
                }
                _ => {}
            }
            i += 1;
        }
        return None;
    }
}

fn first_prop(css: &str, selector: &str, prop: &str) -> Option<String> {
    let (open, close) = find_block(css, selector)?;
    let body = &css[open + 1..close];
    for line in body.lines() {
        let line = line.trim();
        if line.starts_with("/*") {
            continue;
        }
        if let Some(rest) = line.strip_prefix(prop) {
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix(':') {
                let val = rest.trim().trim_end_matches(';').trim();
                // Strip trailing comments
                let val = val
                    .split("//")
                    .next()
                    .unwrap_or(val)
                    .split("/*")
                    .next()
                    .unwrap_or(val)
                    .trim();
                return Some(val.to_string());
            }
        }
    }
    None
}

fn block_exists(css: &str, selector: &str) -> bool {
    find_block(css, selector).is_some()
}

fn prop_exists_in_block(css: &str, selector: &str, prop: &str) -> bool {
    first_prop(css, selector, prop).is_some()
}

fn set_prop_in_block(css: &str, selector: &str, prop: &str, value: &str) -> String {
    let Some((open, close)) = find_block(css, selector) else {
        return css.to_string();
    };
    let body = &css[open + 1..close];
    let mut new_body = String::new();
    let mut replaced = false;
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(prop) {
            let rest = rest.trim_start();
            if rest.starts_with(':') {
                let indent = &line[..line.len() - line.trim_start().len()];
                new_body.push_str(indent);
                new_body.push_str(prop);
                new_body.push_str(": ");
                new_body.push_str(value);
                new_body.push(';');
                new_body.push('\n');
                replaced = true;
                continue;
            }
        }
        new_body.push_str(line);
        new_body.push('\n');
    }
    if !replaced {
        new_body.push_str("    ");
        new_body.push_str(prop);
        new_body.push_str(": ");
        new_body.push_str(value);
        new_body.push_str(";\n");
    }
    let mut out = String::new();
    out.push_str(&css[..open + 1]);
    out.push_str(&new_body);
    out.push_str(&css[close..]);
    out
}

fn set_or_note(css: &str, selector: &str, prop: &str, value: &str) -> String {
    if block_exists(css, selector) {
        set_prop_in_block(css, selector, prop, value)
    } else {
        css.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_prefers_managed_workspace_block() {
        let css = r#"
#workspaces button.active {
    color: #89b4fa;
    background-color: rgba(137, 180, 250, 0.15);
}
/* >>> hyprbinds:waybar-workspaces */
#workspaces button.active {
    color: black;
    background-color: white;
}
/* <<< hyprbinds:waybar-workspaces */
"#;
        let colors = extract_workspace_colors(css);
        assert_eq!(colors.active_fg, "black");
        assert_eq!(colors.active_bg, "white");
    }

    #[test]
    fn apply_workspace_colors_round_trips() {
        let css = r#"
#workspaces button {
    color: #777777;
    background: transparent;
}
/* >>> hyprbinds:waybar-tokens */
#workspaces button.active { color: #ffffff; }
/* <<< hyprbinds:waybar-tokens */
"#;
        let mut colors = WorkspaceStateColors::defaults();
        colors.active_fg = "#ff00ff".into();
        colors.active_bg = "rgb(1, 2, 3)".into();
        let out = apply_workspace_colors(css, &colors);
        let got = extract_workspace_colors(&out);
        assert_eq!(got.active_fg, "#ff00ff");
        assert_eq!(got.active_bg, "rgb(1, 2, 3)");
        assert!(!out.contains("#workspaces button.active { color: #ffffff; }"));
    }
}


/// Per-state colors for `#workspaces button…` (fg / bg).
#[derive(Debug, Clone)]
pub struct WorkspaceStateColors {
    pub default_fg: String,
    pub default_bg: String,
    pub hover_fg: String,
    pub hover_bg: String,
    pub active_fg: String,
    pub active_bg: String,
    pub urgent_fg: String,
    pub urgent_bg: String,
    pub empty_fg: String,
    pub empty_bg: String,
    pub visible_fg: String,
    pub visible_bg: String,
}

impl WorkspaceStateColors {
    pub fn defaults() -> Self {
        Self {
            default_fg: "#777777".into(),
            default_bg: "transparent".into(),
            hover_fg: "#ffffff".into(),
            hover_bg: "rgba(255, 255, 255, 0.08)".into(),
            active_fg: "#89b4fa".into(),
            active_bg: "rgba(137, 180, 250, 0.15)".into(),
            urgent_fg: "#ff5555".into(),
            urgent_bg: "rgba(255, 85, 85, 0.15)".into(),
            empty_fg: "#555555".into(),
            empty_bg: "transparent".into(),
            visible_fg: "#cdd6f4".into(),
            visible_bg: "rgba(205, 214, 244, 0.10)".into(),
        }
    }
}

const WS_MARK_START: &str = "/* >>> hyprbinds:waybar-workspaces */";
const WS_MARK_END: &str = "/* <<< hyprbinds:waybar-workspaces */";

fn managed_section<'a>(css: &'a str, start_mark: &str, end_mark: &str) -> Option<&'a str> {
    let start = css.find(start_mark)? + start_mark.len();
    let rest = &css[start..];
    let end = rest.find(end_mark)?;
    Some(rest[..end].trim())
}

fn prop_from_css(css: &str, selector: &str, prop: &str) -> Option<String> {
    first_prop(css, selector, prop)
}

fn fill_workspace_colors_from(css: &str, c: &mut WorkspaceStateColors) {
    if let Some(v) = prop_from_css(css, "#workspaces button", "color") {
        c.default_fg = v;
    }
    if let Some(v) = prop_from_css(css, "#workspaces button", "background-color")
        .or_else(|| prop_from_css(css, "#workspaces button", "background"))
    {
        c.default_bg = v;
    }
    if let Some(v) = prop_from_css(css, "#workspaces button:hover", "color") {
        c.hover_fg = v;
    }
    if let Some(v) = prop_from_css(css, "#workspaces button:hover", "background-color")
        .or_else(|| prop_from_css(css, "#workspaces button:hover", "background"))
    {
        c.hover_bg = v;
    }
    if let Some(v) = prop_from_css(css, "#workspaces button.active", "color") {
        c.active_fg = v;
    }
    if let Some(v) = prop_from_css(css, "#workspaces button.active", "background-color")
        .or_else(|| prop_from_css(css, "#workspaces button.active", "background"))
    {
        c.active_bg = v;
    }
    if let Some(v) = prop_from_css(css, "#workspaces button.urgent", "color") {
        c.urgent_fg = v;
    }
    if let Some(v) = prop_from_css(css, "#workspaces button.urgent", "background-color")
        .or_else(|| prop_from_css(css, "#workspaces button.urgent", "background"))
    {
        c.urgent_bg = v;
    }
    if let Some(v) = prop_from_css(css, "#workspaces button.empty", "color") {
        c.empty_fg = v;
    }
    if let Some(v) = prop_from_css(css, "#workspaces button.empty", "background-color")
        .or_else(|| prop_from_css(css, "#workspaces button.empty", "background"))
    {
        c.empty_bg = v;
    }
    if let Some(v) = prop_from_css(css, "#workspaces button.visible", "color") {
        c.visible_fg = v;
    }
    if let Some(v) = prop_from_css(css, "#workspaces button.visible", "background-color")
        .or_else(|| prop_from_css(css, "#workspaces button.visible", "background"))
    {
        c.visible_bg = v;
    }
}

pub fn extract_workspace_colors(css: &str) -> WorkspaceStateColors {
    let mut c = WorkspaceStateColors::defaults();
    // Prefer the managed block (written by Studio) — it is the source of truth and
    // may come after older duplicate selectors in the file.
    if let Some(section) = managed_section(css, WS_MARK_START, WS_MARK_END) {
        fill_workspace_colors_from(section, &mut c);
        return c;
    }
    fill_workspace_colors_from(css, &mut c);
    c
}

fn replace_managed_block(css: &str, start_mark: &str, end_mark: &str, body: &str) -> String {
    let mut out = css.to_string();
    if let Some(start) = out.find(start_mark) {
        if let Some(end_rel) = out[start..].find(end_mark) {
            let end = start + end_rel + end_mark.len();
            out.replace_range(start..end, body.trim());
            return out;
        }
    }
    out.push('\n');
    out.push_str(body);
    out
}

/// Remove `#workspaces button.active { … }` lines from the tokens managed block so
/// they cannot override workspace colors.
fn scrub_workspace_rules_from_tokens(css: &str) -> String {
    let Some(section) = managed_section(css, "/* >>> hyprbinds:waybar-tokens */", "/* <<< hyprbinds:waybar-tokens */")
    else {
        return css.to_string();
    };
    let mut cleaned = String::new();
    let mut skip_depth = 0i32;
    let mut skipping_ws_active = false;
    for line in section.lines() {
        let trimmed = line.trim();
        if skip_depth > 0 {
            skip_depth += trimmed.matches('{').count() as i32;
            skip_depth -= trimmed.matches('}').count() as i32;
            if skip_depth <= 0 {
                skipping_ws_active = false;
                skip_depth = 0;
            }
            continue;
        }
        if trimmed.starts_with("#workspaces button") {
            // Drop any workspace button rules from the tokens block.
            if trimmed.contains('{') && trimmed.contains('}') {
                continue; // single-line rule
            }
            skipping_ws_active = true;
            skip_depth = trimmed.matches('{').count() as i32;
            skip_depth -= trimmed.matches('}').count() as i32;
            if skip_depth <= 0 {
                skip_depth = 0;
            }
            let _ = skipping_ws_active;
            continue;
        }
        cleaned.push_str(line);
        cleaned.push('\n');
    }
    let body = format!(
        "/* >>> hyprbinds:waybar-tokens */\n{}\n/* <<< hyprbinds:waybar-tokens */",
        cleaned.trim()
    );
    replace_managed_block(
        css,
        "/* >>> hyprbinds:waybar-tokens */",
        "/* <<< hyprbinds:waybar-tokens */",
        &body,
    )
}

fn sync_selector_colors(css: &str, selector: &str, fg: &str, bg: &str) -> String {
    let mut out = css.to_string();
    if !block_exists(&out, selector) {
        // Insert a small rule before the managed workspaces block (or at end).
        let rule = format!(
            "\n{selector} {{\n    color: {fg};\n    background-color: {bg};\n}}\n"
        );
        if let Some(idx) = out.find(WS_MARK_START) {
            out.insert_str(idx, &rule);
        } else {
            out.push_str(&rule);
        }
        return out;
    }
    out = set_prop_in_block(&out, selector, "color", fg);
    out = set_prop_in_block(&out, selector, "background-color", bg);
    // Clear shorthand `background:` if present so it cannot override background-color.
    if prop_exists_in_block(&out, selector, "background") {
        out = set_prop_in_block(&out, selector, "background", bg);
    }
    out
}

pub fn apply_workspace_colors(css: &str, colors: &WorkspaceStateColors) -> String {
    let mut out = scrub_workspace_rules_from_tokens(css);

    // Keep legacy selectors in sync so the file isn't confusing and extract fallbacks work.
    // Order of sync doesn't matter here; the managed block is the authoritative cascade.
    out = sync_selector_colors(&out, "#workspaces button", &colors.default_fg, &colors.default_bg);
    out = sync_selector_colors(
        &out,
        "#workspaces button:hover",
        &colors.hover_fg,
        &colors.hover_bg,
    );
    out = sync_selector_colors(
        &out,
        "#workspaces button.persistent",
        &colors.default_fg,
        &colors.default_bg,
    );
    out = sync_selector_colors(
        &out,
        "#workspaces button.empty",
        &colors.empty_fg,
        &colors.empty_bg,
    );
    out = sync_selector_colors(
        &out,
        "#workspaces button.visible",
        &colors.visible_fg,
        &colors.visible_bg,
    );
    out = sync_selector_colors(
        &out,
        "#workspaces button.urgent",
        &colors.urgent_fg,
        &colors.urgent_bg,
    );
    out = sync_selector_colors(
        &out,
        "#workspaces button.active",
        &colors.active_fg,
        &colors.active_bg,
    );

    // IMPORTANT: cascade order. Hyprland buttons often have multiple classes at once
    // (persistent + empty + visible + active). Later rules win — active must be last.
    let block = format!(
        "\n{WS_MARK_START}\n\
#workspaces button,\n\
#workspaces button label {{\n\
    color: {default_fg};\n\
    background-color: {default_bg};\n\
    border: none;\n\
    box-shadow: none;\n\
    text-shadow: none;\n\
    border-radius: 8px;\n\
    padding: 0 8px;\n\
    min-width: 0;\n\
    transition: none;\n\
}}\n\
#workspaces button:hover,\n\
#workspaces button:hover label {{\n\
    color: {hover_fg};\n\
    background-color: {hover_bg};\n\
}}\n\
#workspaces button.persistent,\n\
#workspaces button.persistent label {{\n\
    color: {default_fg};\n\
    background-color: {default_bg};\n\
}}\n\
#workspaces button.empty,\n\
#workspaces button.empty label {{\n\
    color: {empty_fg};\n\
    background-color: {empty_bg};\n\
}}\n\
#workspaces button.visible,\n\
#workspaces button.visible label {{\n\
    color: {visible_fg};\n\
    background-color: {visible_bg};\n\
}}\n\
#workspaces button.urgent,\n\
#workspaces button.urgent label {{\n\
    color: {urgent_fg};\n\
    background-color: {urgent_bg};\n\
}}\n\
#workspaces button.active,\n\
#workspaces button.active label {{\n\
    color: {active_fg};\n\
    background-color: {active_bg};\n\
}}\n\
{WS_MARK_END}\n",
        default_fg = colors.default_fg,
        default_bg = colors.default_bg,
        hover_fg = colors.hover_fg,
        hover_bg = colors.hover_bg,
        active_fg = colors.active_fg,
        active_bg = colors.active_bg,
        urgent_fg = colors.urgent_fg,
        urgent_bg = colors.urgent_bg,
        empty_fg = colors.empty_fg,
        empty_bg = colors.empty_bg,
        visible_fg = colors.visible_fg,
        visible_bg = colors.visible_bg,
    );
    replace_managed_block(&out, WS_MARK_START, WS_MARK_END, &block)
}

