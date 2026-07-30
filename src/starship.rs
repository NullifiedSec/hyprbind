//! Starship config I/O, shell integration, presets, and prompt preview.

use crate::backup;
use crate::starship_model::{ShellKind, ShellStatus, StarshipModel};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use thiserror::Error;

const MARK_START: &str = "# >>> hyprbinds:starship";
const MARK_END: &str = "# <<< hyprbinds:starship";

#[derive(Debug, Error)]
pub enum StarshipError {
    #[error("{0}")]
    Message(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("backup error: {0}")]
    Backup(#[from] backup::BackupError),
}

pub fn command_exists(bin: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {bin} >/dev/null 2>&1")])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn status_label() -> String {
    if command_exists("starship") {
        match starship_version() {
            Some(v) => format!("installed ({v})"),
            None => "installed".into(),
        }
    } else {
        "MISSING — install starship".into()
    }
}

pub fn install_hint() -> &'static str {
    "sudo pacman -S starship"
}

fn starship_version() -> Option<String> {
    let output = Command::new("starship")
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    // First line looks like: "starship 1.26.0"
    text.lines()
        .next()
        .map(|l| l.trim().trim_start_matches("starship ").to_string())
}

pub fn config_path() -> Option<PathBuf> {
    // Honor STARSHIP_CONFIG when set (same as starship itself).
    if let Ok(p) = std::env::var("STARSHIP_CONFIG") {
        if !p.is_empty() {
            return Some(PathBuf::from(p));
        }
    }
    let mut p = dirs::config_dir()?;
    p.push("starship.toml");
    Some(p)
}

pub fn load() -> Result<StarshipModel, StarshipError> {
    let config_path = config_path()
        .ok_or_else(|| StarshipError::Message("Could not resolve starship.toml path".into()))?;
    let mut model = StarshipModel::empty(config_path.clone());

    if config_path.is_file() {
        model.toml_text = fs::read_to_string(&config_path)?;
    } else {
        model
            .notes
            .push("No starship.toml yet — Apply will create one.".into());
    }

    if !command_exists("starship") {
        model.notes.push(format!(
            "starship binary missing — install with: {}",
            install_hint()
        ));
    }

    Ok(model)
}

pub fn apply(model: &StarshipModel) -> Result<String, StarshipError> {
    let path = &model.config_path;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path.is_file() {
        backup::snapshot_before_write(path)?;
    }
    fs::write(path, &model.toml_text)?;
    Ok(format!("Wrote {}", path.display()))
}

pub fn restore_files(model: &StarshipModel) -> Result<String, StarshipError> {
    let path = &model.config_path;
    let src = backup::restore_last_good(path)?;
    Ok(format!(
        "Restored {} from {}",
        path.display(),
        src.display()
    ))
}

/// Shells present on PATH (and always include common interactive shells when their RC exists).
pub fn detect_shells() -> Vec<ShellStatus> {
    ShellKind::ALL
        .iter()
        .copied()
        .filter_map(|kind| {
            let installed = command_exists(kind.binary());
            let rc_path = rc_path_for(kind)?;
            // Show if installed OR an RC file already exists (user may install later).
            if !installed && !rc_path.is_file() {
                return None;
            }
            let (enabled, managed, notes) = inspect_rc(kind, &rc_path);
            Some(ShellStatus {
                kind,
                installed,
                rc_path,
                enabled,
                managed,
                notes,
            })
        })
        .collect()
}

fn rc_path_for(kind: ShellKind) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    Some(match kind {
        ShellKind::Bash => home.join(".bashrc"),
        ShellKind::Zsh => home.join(".zshrc"),
        ShellKind::Fish => {
            let mut p = dirs::config_dir()?;
            p.push("fish");
            p.push("config.fish");
            p
        }
        ShellKind::Nu => {
            // Prefer XDG config; fall back to classic Nushell path.
            if let Some(mut p) = dirs::config_dir() {
                p.push("nushell");
                p.push("config.nu");
                if p.is_file() || !home.join(".config/nushell/config.nu").is_file() {
                    return Some(p);
                }
            }
            home.join(".config/nushell/config.nu")
        }
        ShellKind::Elvish => {
            let mut p = dirs::config_dir()?;
            p.push("elvish");
            p.push("rc.elv");
            p
        }
        ShellKind::Xonsh => home.join(".xonshrc"),
        ShellKind::Tcsh => home.join(".tcshrc"),
        ShellKind::Ion => {
            let mut p = dirs::config_dir()?;
            p.push("ion");
            p.push("initrc");
            p
        }
    })
}

fn init_line(kind: ShellKind) -> String {
    match kind {
        ShellKind::Bash => r#"eval "$(starship init bash)""#.into(),
        ShellKind::Zsh => r#"eval "$(starship init zsh)""#.into(),
        ShellKind::Fish => "starship init fish | source".into(),
        ShellKind::Nu => {
            // Official docs: vendor autoload. We keep a single sourced line in config.nu
            // that writes/updates the vendor file on shell start when missing, then sources it.
            // Simpler managed approach: direct init pipe into a local file under config.
            "mkdir ($nu.data-dir | path join \"vendor/autoload\")\nstarship init nu | save -f ($nu.data-dir | path join \"vendor/autoload/starship.nu\")".into()
        }
        ShellKind::Elvish => "eval (starship init elvish)".into(),
        ShellKind::Xonsh => "execx($(starship init xonsh))".into(),
        ShellKind::Tcsh => "eval `starship init tcsh`".into(),
        ShellKind::Ion => "eval $(starship init ion)".into(),
    }
}

fn looks_like_starship_init(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("starship init")
        || lower.contains("starship.nu")
        || (lower.contains("starship") && lower.contains("prompt_starship"))
}

fn inspect_rc(kind: ShellKind, path: &Path) -> (bool, bool, Vec<String>) {
    let mut notes = Vec::new();
    if !path.is_file() {
        notes.push("RC file not created yet — Enable will create it.".into());
        return (false, false, notes);
    }
    let Ok(text) = fs::read_to_string(path) else {
        notes.push("Could not read RC file.".into());
        return (false, false, notes);
    };
    let managed = text.contains(MARK_START) && text.contains(MARK_END);
    let enabled = managed || looks_like_starship_init(&text);
    if enabled && !managed {
        notes.push(format!(
            "Starship init found in {} (not Hyprbinds-managed).",
            kind.label()
        ));
    }
    (enabled, managed, notes)
}

/// Append (or create) a Hyprbinds-managed starship init block in the shell RC.
pub fn enable_shell(kind: ShellKind) -> Result<String, StarshipError> {
    if !command_exists("starship") {
        return Err(StarshipError::Message(format!(
            "starship not found — install first ({})",
            install_hint()
        )));
    }
    if !command_exists(kind.binary()) {
        return Err(StarshipError::Message(format!(
            "{} is not installed on this system",
            kind.label()
        )));
    }
    let path = rc_path_for(kind)
        .ok_or_else(|| StarshipError::Message("Could not resolve shell RC path".into()))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let existing = if path.is_file() {
        fs::read_to_string(&path)?
    } else {
        String::new()
    };

    if existing.contains(MARK_START) && existing.contains(MARK_END) {
        return Ok(format!(
            "{} already has a Hyprbinds-managed Starship block",
            kind.label()
        ));
    }

    if path.is_file() {
        backup::snapshot_before_write(&path)?;
    }

    let block = format!(
        "\n{MARK_START}\n{}\n{MARK_END}\n",
        init_line(kind)
    );

    // If unmanaged init exists, leave it and still add managed block only when absent —
    // prefer wrapping: remove unmanaged? Safer to refuse and ask user to Remove first.
    if looks_like_starship_init(&existing) && !existing.contains(MARK_START) {
        // Still add managed block after commenting would be invasive; append and note.
        let mut next = existing;
        if !next.ends_with('\n') {
            next.push('\n');
        }
        next.push_str(&block);
        fs::write(&path, next)?;
        return Ok(format!(
            "Enabled Starship for {} (appended managed block; an existing init line was also present — open {} and remove duplicates if needed)",
            kind.label(),
            path.display()
        ));
    }

    let mut next = existing;
    if !next.is_empty() && !next.ends_with('\n') {
        next.push('\n');
    }
    next.push_str(&block);
    fs::write(&path, next)?;
    Ok(format!(
        "Enabled Starship for {} → {}",
        kind.label(),
        path.display()
    ))
}

/// Remove the Hyprbinds-managed starship block from the shell RC.
pub fn disable_shell(kind: ShellKind) -> Result<String, StarshipError> {
    let path = rc_path_for(kind)
        .ok_or_else(|| StarshipError::Message("Could not resolve shell RC path".into()))?;
    if !path.is_file() {
        return Ok(format!("{} RC not found — nothing to remove", kind.label()));
    }
    let text = fs::read_to_string(&path)?;
    if !text.contains(MARK_START) {
        if looks_like_starship_init(&text) {
            return Err(StarshipError::Message(format!(
                "{} has Starship init that was not added by Hyprbinds — edit {} manually",
                kind.label(),
                path.display()
            )));
        }
        return Ok(format!("Starship was not enabled for {}", kind.label()));
    }

    backup::snapshot_before_write(&path)?;
    let next = strip_managed_block(&text);
    fs::write(&path, next)?;
    Ok(format!(
        "Disabled Starship for {} → {}",
        kind.label(),
        path.display()
    ))
}

fn strip_managed_block(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut skipping = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == MARK_START {
            skipping = true;
            continue;
        }
        if skipping {
            if trimmed == MARK_END {
                skipping = false;
            }
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    // Trim extra trailing blank lines introduced by removal, keep a single trailing newline.
    let trimmed = out.trim_end_matches('\n');
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{trimmed}\n")
    }
}

pub fn list_presets() -> Vec<String> {
    if !command_exists("starship") {
        return Vec::new();
    }
    let output = Command::new("starship")
        .args(["preset", "--list"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok();
    let Some(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

pub fn load_preset(name: &str) -> Result<String, StarshipError> {
    if !command_exists("starship") {
        return Err(StarshipError::Message(
            "starship not found — cannot load presets".into(),
        ));
    }
    let output = Command::new("starship")
        .args(["preset", name])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| StarshipError::Message(e.to_string()))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(StarshipError::Message(format!(
            "preset '{name}' failed: {}",
            err.trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Render a prompt preview (ANSI stripped) using the given config text via a temp file.
pub fn preview_prompt(toml_text: &str) -> Result<String, StarshipError> {
    if !command_exists("starship") {
        return Err(StarshipError::Message(
            "starship not found — install to preview".into(),
        ));
    }
    let dir = std::env::temp_dir().join("hyprbinds-starship");
    fs::create_dir_all(&dir)?;
    let cfg = dir.join("preview.toml");
    fs::write(&cfg, toml_text)?;

    let output = Command::new("starship")
        .arg("prompt")
        .env("STARSHIP_CONFIG", &cfg)
        .env("TERM", "xterm-256color")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| StarshipError::Message(e.to_string()))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(StarshipError::Message(format!(
            "starship prompt failed: {}",
            err.trim()
        )));
    }
    let raw = String::from_utf8_lossy(&output.stdout);
    Ok(strip_ansi(&raw).trim_end().to_string())
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // CSI / OSC sequences — skip until letter terminator or BEL.
            if chars.peek() == Some(&'[') {
                chars.next();
                for ch in chars.by_ref() {
                    if ch.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else if chars.peek() == Some(&']') {
                chars.next();
                for ch in chars.by_ref() {
                    if ch == '\u{7}' || ch == '\u{1b}' {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_managed_block_removes_markers() {
        let input = "alias x=y\n# >>> hyprbinds:starship\neval \"$(starship init zsh)\"\n# <<< hyprbinds:starship\nalias z=w\n";
        let out = strip_managed_block(input);
        assert!(!out.contains("starship"));
        assert!(out.contains("alias x=y"));
        assert!(out.contains("alias z=w"));
    }

    #[test]
    fn strip_ansi_removes_csi() {
        let s = "\u{1b}[1;36mhello\u{1b}[0m world";
        assert_eq!(strip_ansi(s), "hello world");
    }
}
