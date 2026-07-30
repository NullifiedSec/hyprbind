//! Rofi config I/O, theme apply, and demo launch.

use crate::backup;
use crate::rofi_model::RofiModel;
use crate::rofi_theme::{self, ThemeTokens};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use thiserror::Error;

const CONFIG_MARK_START: &str = "/* >>> hyprbinds:rofi-config */";
const CONFIG_MARK_END: &str = "/* <<< hyprbinds:rofi-config */";
const THEME_NAME: &str = "hyprbinds-theme";

#[derive(Debug, Error)]
pub enum RofiError {
    #[error("{0}")]
    Message(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("backup error: {0}")]
    Backup(#[from] backup::BackupError),
}

pub fn rofi_dir() -> Option<PathBuf> {
    let mut p = dirs::config_dir()?;
    p.push("rofi");
    Some(p)
}

pub fn discover_config_path() -> Option<PathBuf> {
    let dir = rofi_dir()?;
    for name in ["config.rasi", "config"] {
        let p = dir.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    Some(dir.join("config.rasi"))
}

pub fn theme_path() -> Option<PathBuf> {
    Some(rofi_dir()?.join(format!("{THEME_NAME}.rasi")))
}

pub fn command_exists(bin: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {bin} >/dev/null 2>&1")])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn status_label() -> String {
    if command_exists("rofi") {
        "installed".into()
    } else {
        "MISSING — install rofi".into()
    }
}

pub fn load() -> Result<RofiModel, RofiError> {
    let config_path = discover_config_path()
        .ok_or_else(|| RofiError::Message("Could not resolve Rofi config dir".into()))?;
    let theme_path = theme_path()
        .ok_or_else(|| RofiError::Message("Could not resolve Rofi theme path".into()))?;

    let mut model = RofiModel::empty(config_path.clone(), theme_path.clone());

    if config_path.is_file() {
        let text = fs::read_to_string(&config_path)?;
        parse_config_into(&text, &mut model);
    } else {
        model
            .notes
            .push("No config.rasi yet — Apply will create one.".into());
    }

    // Prefer our managed theme file; otherwise try @theme target or defaults.
    if theme_path.is_file() {
        let theme_text = fs::read_to_string(&theme_path)?;
        model.tokens = rofi_theme::extract_tokens(&theme_text);
    } else if let Some(external) = resolve_external_theme(&config_path) {
        if external.is_file() {
            match fs::read_to_string(&external) {
                Ok(theme_text) => {
                    model.tokens = rofi_theme::extract_tokens(&theme_text);
                    model.notes.push(format!(
                        "Imported colors from {}",
                        external.display()
                    ));
                }
                Err(e) => model.notes.push(format!("Could not read theme: {e}")),
            }
        } else {
            model.notes.push(format!(
                "Theme reference not found: {}",
                external.display()
            ));
        }
    }

    Ok(model)
}

fn resolve_external_theme(config_path: &Path) -> Option<PathBuf> {
    let text = fs::read_to_string(config_path).ok()?;
    let name = extract_theme_name(&text)?;
    if name == THEME_NAME {
        return theme_path();
    }
    // Absolute path or relative to config dir / share themes.
    let as_path = PathBuf::from(&name);
    if as_path.is_absolute() && as_path.is_file() {
        return Some(as_path);
    }
    let with_ext = if name.ends_with(".rasi") {
        name.clone()
    } else {
        format!("{name}.rasi")
    };
    if let Some(dir) = config_path.parent() {
        let local = dir.join(&with_ext);
        if local.is_file() {
            return Some(local);
        }
    }
    let share = PathBuf::from("/usr/share/rofi/themes").join(&with_ext);
    if share.is_file() {
        return Some(share);
    }
    None
}

fn extract_theme_name(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("@theme") {
            let rest = rest.trim().trim_end_matches(';').trim();
            let name = rest.trim_matches('"').trim_matches('\'').trim();
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

fn parse_config_into(text: &str, model: &mut RofiModel) {
    let cleaned = strip_rasi_comments(text);
    let body = configuration_body(&cleaned).unwrap_or(cleaned.as_str());

    if let Some(v) = prop_string(body, "modes").or_else(|| prop_string(body, "modi")) {
        model.modes = v
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    if let Some(v) = prop_string(body, "font") {
        model.font = v;
    }
    if let Some(v) = prop_bool(body, "show-icons") {
        model.show_icons = v;
    }
    if let Some(v) = prop_u8(body, "location") {
        model.location = v.min(8);
    }
    if let Some(v) = prop_i32(body, "xoffset") {
        model.xoffset = v;
    }
    if let Some(v) = prop_i32(body, "yoffset") {
        model.yoffset = v;
    }
    if let Some(v) = prop_string(body, "terminal") {
        model.terminal = v;
    }
    if let Some(v) = prop_string(body, "matching") {
        model.matching = v;
    }
    if let Some(v) = prop_bool(body, "case-sensitive") {
        model.case_sensitive = v;
    }
    if let Some(v) = prop_bool(body, "cycle") {
        model.cycle = v;
    }
    if let Some(v) = prop_bool(body, "sidebar-mode") {
        model.sidebar_mode = v;
    }
    if let Some(v) = prop_bool(body, "hover-select") {
        model.hover_select = v;
    }
    if let Some(v) = prop_bool(body, "disable-history") {
        model.disable_history = v;
    }
}

fn configuration_body(text: &str) -> Option<&str> {
    let start = text.find("configuration")?;
    let after = &text[start..];
    let open = after.find('{')?;
    let body_start = start + open + 1;
    let mut depth = 1i32;
    let bytes = text.as_bytes();
    let mut i = body_start;
    while i < bytes.len() {
        match bytes[i] as char {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[body_start..i]);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn prop_string(body: &str, key: &str) -> Option<String> {
    let needle = format!("{key}:");
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&needle) {
            let mut v = rest.trim().trim_end_matches(';').trim().to_string();
            if (v.starts_with('"') && v.ends_with('"') && v.len() >= 2)
                || (v.starts_with('\'') && v.ends_with('\'') && v.len() >= 2)
            {
                v = v[1..v.len() - 1].to_string();
            }
            return Some(v);
        }
    }
    None
}

fn prop_bool(body: &str, key: &str) -> Option<bool> {
    let v = prop_string(body, key)?;
    match v.to_lowercase().as_str() {
        "true" | "yes" | "1" | "on" => Some(true),
        "false" | "no" | "0" | "off" => Some(false),
        _ => None,
    }
}

fn prop_u8(body: &str, key: &str) -> Option<u8> {
    prop_string(body, key)?.parse().ok()
}

fn prop_i32(body: &str, key: &str) -> Option<i32> {
    prop_string(body, key)?.parse().ok()
}

fn strip_rasi_comments(input: &str) -> String {
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

fn managed_config_block(model: &RofiModel) -> String {
    let modes = model.modes_csv();
    let show_icons = if model.show_icons { "true" } else { "false" };
    let case_sensitive = if model.case_sensitive {
        "true"
    } else {
        "false"
    };
    let cycle = if model.cycle { "true" } else { "false" };
    let sidebar = if model.sidebar_mode { "true" } else { "false" };
    let hover = if model.hover_select { "true" } else { "false" };
    let history = if model.disable_history {
        "true"
    } else {
        "false"
    };

    format!(
        "{CONFIG_MARK_START}\n\
    modes: \"{modes}\";\n\
    font: \"{font}\";\n\
    show-icons: {show_icons};\n\
    location: {location};\n\
    xoffset: {xoffset};\n\
    yoffset: {yoffset};\n\
    terminal: \"{terminal}\";\n\
    matching: \"{matching}\";\n\
    case-sensitive: {case_sensitive};\n\
    cycle: {cycle};\n\
    sidebar-mode: {sidebar};\n\
    hover-select: {hover};\n\
    disable-history: {history};\n\
{CONFIG_MARK_END}",
        font = model.font,
        location = model.location,
        xoffset = model.xoffset,
        yoffset = model.yoffset,
        terminal = model.terminal,
        matching = model.matching,
    )
}

/// Build the full config.rasi text, preserving unmarked user content when possible.
pub fn render_config(model: &RofiModel) -> String {
    let managed = managed_config_block(model);
    let theme_line = format!("@theme \"{THEME_NAME}\"");

    if model.config_path.is_file() {
        if let Ok(existing) = fs::read_to_string(&model.config_path) {
            if existing.contains(CONFIG_MARK_START) && existing.contains(CONFIG_MARK_END) {
                let mut out = replace_marked_section(&existing, &managed);
                out = ensure_theme_line(&out, &theme_line);
                return out;
            }
            // No markers — wrap a fresh configuration + theme, keep a backup note.
            return format!(
                "/* Hyprbinds Rofi Studio — previous config was snapshotted on Apply. */\n\
configuration {{\n\
{managed}\n\
}}\n\
\n\
{theme_line}\n"
            );
        }
    }

    format!(
        "/* Hyprbinds Rofi Studio — managed config.rasi */\n\
configuration {{\n\
{managed}\n\
}}\n\
\n\
{theme_line}\n"
    )
}

fn replace_marked_section(existing: &str, managed: &str) -> String {
    let Some(start) = existing.find(CONFIG_MARK_START) else {
        return existing.to_string();
    };
    let Some(end_rel) = existing[start..].find(CONFIG_MARK_END) else {
        return existing.to_string();
    };
    let end = start + end_rel + CONFIG_MARK_END.len();
    let mut out = String::with_capacity(existing.len() + managed.len());
    out.push_str(&existing[..start]);
    out.push_str(managed);
    out.push_str(&existing[end..]);
    out
}

fn ensure_theme_line(text: &str, theme_line: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut replaced = false;
    for line in text.lines() {
        if line.trim().starts_with("@theme") {
            if !replaced {
                lines.push(theme_line.to_string());
                replaced = true;
            }
            // drop extra @theme lines
        } else {
            lines.push(line.to_string());
        }
    }
    if !replaced {
        lines.push(String::new());
        lines.push(theme_line.to_string());
    }
    let mut out = lines.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

pub fn save(model: &RofiModel) -> Result<String, RofiError> {
    if let Some(parent) = model.config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if let Some(parent) = model.theme_path.parent() {
        fs::create_dir_all(parent)?;
    }

    if model.config_path.is_file() {
        backup::snapshot_before_write(&model.config_path)?;
    }
    if model.theme_path.is_file() {
        backup::snapshot_before_write(&model.theme_path)?;
    }

    let config_text = render_config(model);
    let theme_text = rofi_theme::render_theme(&model.tokens, &model.font);

    write_atomic(&model.config_path, &config_text)?;
    write_atomic(&model.theme_path, &theme_text)?;

    Ok(format!(
        "Saved {} and {}",
        model.config_path.display(),
        model.theme_path.display()
    ))
}

fn write_atomic(path: &Path, contents: &str) -> Result<(), RofiError> {
    let tmp = path.with_extension("hyprbinds.tmp");
    fs::write(&tmp, contents)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn apply(model: &RofiModel) -> Result<String, RofiError> {
    save(model)
}

pub fn restore_files(model: &RofiModel) -> Result<String, RofiError> {
    let mut parts = Vec::new();
    match backup::restore_last_good(&model.config_path) {
        Ok(p) => parts.push(format!("config ← {}", p.display())),
        Err(e) => parts.push(format!("config: {e}")),
    }
    match backup::restore_last_good(&model.theme_path) {
        Ok(p) => parts.push(format!("theme ← {}", p.display())),
        Err(e) => parts.push(format!("theme: {e}")),
    }
    Ok(parts.join(" · "))
}

/// Launch a short-lived Rofi demo so the user can feel the theme.
pub fn launch_demo(mode: &str) -> Result<String, RofiError> {
    if !command_exists("rofi") {
        return Err(RofiError::Message(
            "rofi not found — install the rofi package".into(),
        ));
    }
    let mode = if mode.is_empty() { "drun" } else { mode };

    let launch = if command_exists("setsid") {
        Command::new("setsid")
            .args(["-f", "rofi", "-show", mode])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| RofiError::Message(format!("failed to launch rofi: {e}")))?;
        format!("setsid -f rofi -show {mode}")
    } else {
        Command::new("sh")
            .args(["-c", &format!("rofi -show {mode} >/dev/null 2>&1 &")])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| RofiError::Message(format!("failed to launch rofi: {e}")))?;
        format!("rofi -show {mode}")
    };

    Ok(format!("Launched demo: {launch}"))
}

/// Export-friendly snapshot of current studio state.
pub fn export_snapshot(model: &RofiModel) -> (String, String) {
    (
        render_config(model),
        rofi_theme::render_theme(&model.tokens, &model.font),
    )
}

pub fn import_snapshot(config_rasi: &str, theme_rasi: &str) -> Result<String, RofiError> {
    let config_path = discover_config_path()
        .ok_or_else(|| RofiError::Message("Could not resolve Rofi config dir".into()))?;
    let theme_path = theme_path()
        .ok_or_else(|| RofiError::Message("Could not resolve Rofi theme path".into()))?;

    let mut model = RofiModel::empty(config_path, theme_path);
    if !config_rasi.trim().is_empty() {
        parse_config_into(config_rasi, &mut model);
    }
    if !theme_rasi.trim().is_empty() {
        model.tokens = rofi_theme::extract_tokens(theme_rasi);
    } else {
        model.tokens = ThemeTokens::defaults();
    }
    // Prefer writing the exact provided theme text when present.
    if let Some(parent) = model.config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if model.config_path.is_file() {
        backup::snapshot_before_write(&model.config_path)?;
    }
    if model.theme_path.is_file() {
        backup::snapshot_before_write(&model.theme_path)?;
    }

    let config_out = if config_rasi.trim().is_empty() {
        render_config(&model)
    } else {
        config_rasi.to_string()
    };
    let theme_out = if theme_rasi.trim().is_empty() {
        rofi_theme::render_theme(&model.tokens, &model.font)
    } else {
        theme_rasi.to_string()
    };

    write_atomic(&model.config_path, &config_out)?;
    write_atomic(&model.theme_path, &theme_out)?;
    Ok(format!(
        "Imported Rofi → {} · {}",
        model.config_path.display(),
        model.theme_path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rofi_model::RofiModel;
    use std::path::PathBuf;

    #[test]
    fn render_config_contains_managed_block() {
        let model = RofiModel::empty(
            PathBuf::from("/tmp/rofi-config.rasi"),
            PathBuf::from("/tmp/hyprbinds-theme.rasi"),
        );
        let text = render_config(&model);
        assert!(text.contains(CONFIG_MARK_START));
        assert!(text.contains("modes:"));
        assert!(text.contains("@theme \"hyprbinds-theme\""));
    }

    #[test]
    fn parse_modes_from_config() {
        let text = r#"
configuration {
    modes: "drun,window";
    font: "mono 14";
    show-icons: true;
    location: 2;
}
"#;
        let mut model = RofiModel::empty(
            PathBuf::from("/tmp/c.rasi"),
            PathBuf::from("/tmp/t.rasi"),
        );
        parse_config_into(text, &mut model);
        assert_eq!(model.modes, vec!["drun".to_string(), "window".to_string()]);
        assert_eq!(model.font, "mono 14");
        assert!(model.show_icons);
        assert_eq!(model.location, 2);
    }
}
