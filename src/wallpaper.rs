//! Wallpaper control via `awww` / `awww-daemon`.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct OutputWallpaper {
    pub output: String,
    pub size: String,
    pub image: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WallpaperPrefs {
    #[serde(default)]
    pub last_path: String,
    #[serde(default)]
    pub recent: Vec<String>,
    #[serde(default = "default_transition")]
    pub transition: String,
    #[serde(default = "default_resize")]
    pub resize: String,
}

fn default_transition() -> String {
    "fade".into()
}
fn default_resize() -> String {
    "crop".into()
}

fn prefs_path() -> Option<PathBuf> {
    let mut p = dirs::config_dir()?;
    p.push("hyprbinds");
    p.push("wallpaper.json");
    Some(p)
}

pub fn load_prefs() -> WallpaperPrefs {
    let Some(path) = prefs_path() else {
        return WallpaperPrefs::default();
    };
    fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save_prefs(prefs: &WallpaperPrefs) {
    let Some(path) = prefs_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string_pretty(prefs) {
        let _ = fs::write(path, text);
    }
}

pub fn daemon_running() -> bool {
    Command::new("pgrep")
        .args(["-x", "awww-daemon"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn ensure_daemon() -> Result<String, String> {
    if daemon_running() {
        return Ok("awww-daemon already running".into());
    }
    if !command_exists("awww-daemon") {
        return Err("awww-daemon not found — install package `awww`".into());
    }
    Command::new("awww-daemon")
        .spawn()
        .map_err(|e| e.to_string())?;
    // brief wait
    std::thread::sleep(std::time::Duration::from_millis(250));
    if daemon_running() {
        Ok("Started awww-daemon".into())
    } else {
        Ok("Launched awww-daemon (may still be starting)".into())
    }
}

pub fn query() -> Result<Vec<OutputWallpaper>, String> {
    let out = run_ok("awww", &["query"])?;
    let mut rows = Vec::new();
    // Typical: "HDMI-A-1: 1920x1080, scale: 1, Currently displaying: image: /path/to/img"
    for line in out.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (output, rest) = line
            .split_once(':')
            .map(|(a, b)| (a.trim().to_string(), b.trim().to_string()))
            .unwrap_or_else(|| (line.to_string(), String::new()));
        let size = rest
            .split(',')
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        let image = if let Some(idx) = rest.to_ascii_lowercase().find("image:") {
            rest[idx + 6..].trim().to_string()
        } else if let Some(idx) = rest.find('/') {
            // path somewhere
            rest[idx..].trim().to_string()
        } else {
            String::new()
        };
        rows.push(OutputWallpaper {
            output,
            size,
            image,
        });
    }
    Ok(rows)
}

pub fn set_image(
    path: &Path,
    transition: &str,
    resize: &str,
    output: Option<&str>,
) -> Result<String, String> {
    if !path.is_file() {
        return Err(format!("not a file: {}", path.display()));
    }
    ensure_daemon()?;
    let mut args = vec![
        "img".to_string(),
        "--transition-type".into(),
        transition.to_string(),
        "--resize".into(),
        resize.to_string(),
    ];
    if let Some(o) = output {
        if !o.is_empty() && o != "All outputs" {
            args.push("-o".into());
            args.push(o.to_string());
        }
    }
    args.push(path.display().to_string());
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run_ok("awww", &arg_refs)?;

    let mut prefs = load_prefs();
    prefs.last_path = path.display().to_string();
    prefs.transition = transition.to_string();
    prefs.resize = resize.to_string();
    prefs.recent.retain(|p| p != &prefs.last_path);
    prefs.recent.insert(0, prefs.last_path.clone());
    prefs.recent.truncate(12);
    save_prefs(&prefs);

    Ok(format!("Set wallpaper: {}", path.display()))
}

pub fn clear(color_hex: &str) -> Result<String, String> {
    ensure_daemon()?;
    let color = color_hex.trim().trim_start_matches('#');
    let color = if color.is_empty() { "000000ff" } else { color };
    run_ok("awww", &["clear", color])?;
    Ok(format!("Cleared wallpaper to {color}"))
}

pub fn restore() -> Result<String, String> {
    ensure_daemon()?;
    run_ok("awww", &["restore"])?;
    Ok("Restored last awww wallpaper(s)".into())
}

pub fn pictures_dir() -> PathBuf {
    dirs::picture_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn command_exists(bin: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {bin} >/dev/null 2>&1")])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn run_ok(bin: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(bin)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(if err.trim().is_empty() {
            format!("{bin} failed: {}", String::from_utf8_lossy(&output.stdout))
        } else {
            err.trim().to_string()
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
