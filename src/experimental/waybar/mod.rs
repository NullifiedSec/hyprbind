//! Waybar studio (experimental).
//!
//! Config I/O, process control, and JSONC helpers.

pub mod model;
pub mod modules;
pub mod style;
pub mod themes;
mod ui;

pub use model::*;
pub use ui::*;

use crate::backup;
use crate::experimental::waybar::model::WaybarModel;
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WaybarError {
    #[error("{0}")]
    Message(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("backup error: {0}")]
    Backup(#[from] backup::BackupError),
}

pub fn waybar_dir() -> Option<PathBuf> {
    let mut p = dirs::config_dir()?;
    p.push("waybar");
    Some(p)
}

/// Discover primary config: config.jsonc → config → config.json.
pub fn discover_config_path() -> Option<PathBuf> {
    let dir = waybar_dir()?;
    for name in ["config.jsonc", "config", "config.json"] {
        let p = dir.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    // Prefer creating config.jsonc if directory exists or can be created.
    Some(dir.join("config.jsonc"))
}

pub fn style_path() -> Option<PathBuf> {
    Some(waybar_dir()?.join("style.css"))
}

pub fn command_exists(bin: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {bin} >/dev/null 2>&1")])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn pgrep_waybar_pids() -> Vec<i32> {
    let output = Command::new("pgrep")
        .args(["-x", "waybar"])
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
        .filter_map(|l| l.trim().parse::<i32>().ok())
        .collect()
}

/// Process state from `/proc/<pid>/stat` (third field after the command).
fn proc_state(pid: i32) -> Option<char> {
    let text = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let close = text.rfind(')')?;
    text[close + 1..]
        .split_whitespace()
        .next()
        .and_then(|s| s.chars().next())
}

fn is_zombie_pid(pid: i32) -> bool {
    matches!(proc_state(pid), Some('Z'))
}

/// True only if a live (non-zombie) waybar process exists.
pub fn is_running() -> bool {
    pgrep_waybar_pids()
        .into_iter()
        .any(|pid| !is_zombie_pid(pid))
}

/// True if only zombie waybar entries remain (blocks naive pgrep checks).
pub fn has_stale_zombies() -> bool {
    let pids = pgrep_waybar_pids();
    !pids.is_empty() && pids.iter().all(|pid| is_zombie_pid(*pid))
}

pub fn status_label() -> String {
    if is_running() {
        let n = live_waybar_pids().len();
        format!("running ({n})")
    } else if has_stale_zombies() {
        "stopped (stale zombie ignored)".into()
    } else {
        "stopped".into()
    }
}

fn live_waybar_pids() -> Vec<i32> {
    pgrep_waybar_pids()
        .into_iter()
        .filter(|pid| !is_zombie_pid(*pid))
        .collect()
}

pub fn reload() -> Result<String, WaybarError> {
    let pids = live_waybar_pids();
    if pids.is_empty() {
        return Err(WaybarError::Message(
            "Waybar is not running — use Start first".into(),
        ));
    }
    // Signal each live PID directly (more reliable than killall with zombies around).
    let mut ok = 0usize;
    for pid in &pids {
        let status = Command::new("kill")
            .args(["-USR2", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| WaybarError::Message(e.to_string()))?;
        if status.success() {
            ok += 1;
        }
    }
    if ok > 0 {
        Ok(format!("Sent SIGUSR2 reload to {ok} Waybar process(es)"))
    } else {
        Err(WaybarError::Message(
            "Failed to signal Waybar (SIGUSR2)".into(),
        ))
    }
}

fn waybar_log_path() -> Option<PathBuf> {
    let mut p = waybar_dir()?;
    p.push("hyprbinds-start.log");
    Some(p)
}

pub fn start() -> Result<String, WaybarError> {
    if is_running() {
        return Ok("Waybar already running".into());
    }
    if !command_exists("waybar") {
        return Err(WaybarError::Message(
            "waybar not found — install the waybar package".into(),
        ));
    }

    // Prefer setsid -f so Waybar is not a child of hyprbinds (avoids zombies when
    // Waybar exits and we never wait()). Fall back to a background shell launch.
    let log_path = waybar_log_path();
    if let Some(ref log) = log_path {
        if let Some(parent) = log.parent() {
            let _ = fs::create_dir_all(parent);
        }
    }

    let launch = if command_exists("setsid") {
        let mut cmd = Command::new("setsid");
        cmd.args(["-f", "waybar"]);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        if let Some(ref log) = log_path {
            match fs::File::create(log) {
                Ok(f) => {
                    cmd.stderr(Stdio::from(f));
                }
                Err(_) => {
                    cmd.stderr(Stdio::null());
                }
            }
        } else {
            cmd.stderr(Stdio::null());
        }
        cmd.spawn()
            .map_err(|e| WaybarError::Message(format!("failed to launch waybar via setsid: {e}")))?;
        "setsid -f waybar"
    } else {
        let log_redirect = log_path
            .as_ref()
            .map(|p| format!("\"{}\"", p.display()))
            .unwrap_or_else(|| "/dev/null".into());
        Command::new("sh")
            .args([
                "-c",
                &format!("waybar >/dev/null 2>{log_redirect} &"),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| WaybarError::Message(format!("failed to launch waybar: {e}")))?;
        "sh -c 'waybar &'"
    };

    // Give Waybar a moment to start (or fail).
    for _ in 0..10 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if is_running() {
            return Ok(format!("Started Waybar ({launch})"));
        }
    }

    let hint = log_path
        .as_ref()
        .and_then(|p| fs::read_to_string(p).ok())
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_default();
    if hint.is_empty() {
        Err(WaybarError::Message(
            "Launched Waybar but it did not stay running — check WAYLAND_DISPLAY / config".into(),
        ))
    } else {
        Err(WaybarError::Message(format!(
            "Waybar exited immediately: {hint}"
        )))
    }
}

pub fn stop() -> Result<String, WaybarError> {
    let pids = live_waybar_pids();
    if pids.is_empty() {
        // Still try killall to clear any odd cases; ignore zombies.
        let _ = Command::new("killall")
            .arg("waybar")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        return Ok("Waybar is not running".into());
    }
    let mut ok = 0usize;
    for pid in &pids {
        let status = Command::new("kill")
            .args([&pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| WaybarError::Message(e.to_string()))?;
        if status.success() {
            ok += 1;
        }
    }
    // Fallback
    let _ = Command::new("killall")
        .arg("waybar")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    std::thread::sleep(std::time::Duration::from_millis(150));
    if is_running() {
        Err(WaybarError::Message(
            "Tried to stop Waybar but a process is still running".into(),
        ))
    } else {
        Ok(format!("Stopped {ok} Waybar process(es)"))
    }
}

pub fn restart() -> Result<String, WaybarError> {
    let _ = stop();
    std::thread::sleep(std::time::Duration::from_millis(200));
    start()
}

/// Strip // and /* */ comments from JSONC (string-aware, UTF-8 safe).
pub fn strip_jsonc_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;
    let mut escape = false;

    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
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

        // Line comment
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

        // Block comment
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

pub fn parse_jsonc(text: &str) -> Result<Value, WaybarError> {
    let cleaned = strip_jsonc_comments(text);
    Ok(serde_json::from_str(&cleaned)?)
}

fn load_style(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|_| crate::experimental::waybar::style::DEFAULT_STYLE.to_string())
}

fn resolve_includes(dir: &Path, includes: &[Value]) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for v in includes {
        let Some(s) = v.as_str() else { continue };
        let p = PathBuf::from(s);
        let resolved = if p.is_absolute() {
            p
        } else {
            dir.join(p)
        };
        if resolved.is_file() {
            paths.push(resolved);
        }
    }
    paths
}

fn merge_include(
    bar: &mut Map<String, Value>,
    included_origins: &mut std::collections::HashMap<String, PathBuf>,
    primary_keys: &std::collections::HashSet<String>,
    include_path: &Path,
    notes: &mut Vec<String>,
) {
    let Ok(text) = fs::read_to_string(include_path) else {
        notes.push(format!(
            "Could not read include: {}",
            include_path.display()
        ));
        return;
    };
    let Ok(value) = parse_jsonc(&text) else {
        notes.push(format!(
            "Could not parse include: {}",
            include_path.display()
        ));
        return;
    };
    let Some(obj) = value.as_object() else {
        notes.push(format!(
            "Include is not an object: {}",
            include_path.display()
        ));
        return;
    };
    for (k, v) in obj {
        if primary_keys.contains(k) {
            // Primary wins; keep note only.
            continue;
        }
        if !bar.contains_key(k) {
            bar.insert(k.clone(), v.clone());
            included_origins.insert(k.clone(), include_path.to_path_buf());
        }
    }
}

pub fn load() -> Result<WaybarModel, WaybarError> {
    let config_path = discover_config_path()
        .ok_or_else(|| WaybarError::Message("Could not resolve Waybar config dir".into()))?;
    let style_path = style_path()
        .ok_or_else(|| WaybarError::Message("Could not resolve Waybar style path".into()))?;

    if !config_path.is_file() {
        let mut model = WaybarModel::empty(config_path, style_path);
        model.style_css = load_style(&model.style_path);
        return Ok(model);
    }

    let text = fs::read_to_string(&config_path)?;
    let root = parse_jsonc(&text)?;
    let mut notes = Vec::new();
    let mut root_is_array = false;

    let bar_value = match root {
        Value::Object(map) => Value::Object(map),
        Value::Array(arr) => {
            root_is_array = true;
            notes.push(
                "Config is a multi-bar array; editing bar [0] only.".into(),
            );
            arr.into_iter()
                .next()
                .unwrap_or_else(|| Value::Object(Map::new()))
        }
        other => {
            return Err(WaybarError::Message(format!(
                "Unexpected Waybar root type: {other}"
            )));
        }
    };

    let mut bar = match bar_value {
        Value::Object(m) => m,
        _ => {
            return Err(WaybarError::Message(
                "Waybar bar config must be a JSON object".into(),
            ));
        }
    };

    let primary_keys: std::collections::HashSet<String> = bar.keys().cloned().collect();
    let mut included_origins = std::collections::HashMap::new();

    if let Some(Value::Array(incs)) = bar.get("include").cloned() {
        let dir = config_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        for path in resolve_includes(&dir, &incs) {
            merge_include(
                &mut bar,
                &mut included_origins,
                &primary_keys,
                &path,
                &mut notes,
            );
        }
    }

    let style_css = load_style(&style_path);

    Ok(WaybarModel {
        config_path,
        style_path,
        root_is_array,
        primary_keys,
        included_origins,
        bar,
        style_css,
        notes,
    })
}

pub fn save(model: &WaybarModel) -> Result<String, WaybarError> {
    if let Some(parent) = model.config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if let Some(parent) = model.style_path.parent() {
        fs::create_dir_all(parent)?;
    }

    if model.config_path.is_file() {
        backup::snapshot_before_write(&model.config_path)?;
    }
    if model.style_path.is_file() {
        backup::snapshot_before_write(&model.style_path)?;
    }

    let primary = model.to_primary_value();
    let json = serde_json::to_string_pretty(&primary)?;
    // Atomic-ish write via temp file.
    let tmp_cfg = model.config_path.with_extension("hyprbinds.tmp");
    fs::write(&tmp_cfg, format!("{json}\n"))?;
    fs::rename(&tmp_cfg, &model.config_path)?;

    let tmp_css = model.style_path.with_extension("hyprbinds.tmp");
    fs::write(&tmp_css, &model.style_css)?;
    fs::rename(&tmp_css, &model.style_path)?;

    Ok(format!(
        "Saved {} and {}",
        model.config_path.display(),
        model.style_path.display()
    ))
}

pub fn apply(model: &WaybarModel) -> Result<String, WaybarError> {
    let mut msg = save(model)?;
    // Style/CSS changes are more reliable with a full restart than SIGUSR2 alone.
    match restart() {
        Ok(r) => {
            msg.push_str(" · ");
            msg.push_str(&r);
        }
        Err(e) => {
            msg.push_str(" · ");
            msg.push_str(&e.to_string());
            match reload() {
                Ok(r) => {
                    msg.push_str(" · ");
                    msg.push_str(&r);
                }
                Err(re) => {
                    msg.push_str(" · ");
                    msg.push_str(&re.to_string());
                }
            }
        }
    }
    Ok(msg)
}

pub fn restore_files(model: &WaybarModel) -> Result<String, WaybarError> {
    let mut parts = Vec::new();
    match backup::restore_last_good(&model.config_path) {
        Ok(p) => parts.push(format!("config ← {}", p.display())),
        Err(e) => parts.push(format!("config: {e}")),
    }
    match backup::restore_last_good(&model.style_path) {
        Ok(p) => parts.push(format!("style ← {}", p.display())),
        Err(e) => parts.push(format!("style: {e}")),
    }
    Ok(parts.join(" · "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_line_and_block_comments() {
        let input = r#"{
  // comment
  "a": 1,
  /* block */
  "b": "http://x"
}"#;
        let cleaned = strip_jsonc_comments(input);
        let v: Value = serde_json::from_str(&cleaned).unwrap();
        assert_eq!(v["a"], 1);
        assert_eq!(v["b"], "http://x");
    }

    #[test]
    fn strip_jsonc_preserves_unicode() {
        let input = r#"{
  // comment
  "format-icons": {
    "1": "一",
    "active": "●",
    "default": "○"
  }
}"#;
        let cleaned = strip_jsonc_comments(input);
        let v: Value = serde_json::from_str(&cleaned).unwrap();
        assert_eq!(v["format-icons"]["1"], "一");
        assert_eq!(v["format-icons"]["active"], "●");
        assert_eq!(v["format-icons"]["default"], "○");
    }

    #[test]
    fn empty_template_is_studio_ready() {
        let model = crate::experimental::waybar::model::WaybarModel::empty(
            PathBuf::from("/tmp/config.jsonc"),
            PathBuf::from("/tmp/style.css"),
        );
        assert!(model.included_origins.is_empty());
        assert!(!model.root_is_array);
        let left = model.zone_modules(crate::experimental::waybar::model::Zone::Left);
        assert!(left.contains(&"hyprland/workspaces".into()));
        assert!(left.contains(&"hyprland/window".into()));
        let primary = model.to_primary_value();
        assert!(primary.get("include").is_none());
        assert!(primary.get("clock").is_some());
        assert!(!model.style_css.is_empty());
    }
}
