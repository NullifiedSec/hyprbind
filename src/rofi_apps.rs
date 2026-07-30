//! Rofi Apps — managed launcher utilities (clipboard, power, wifi, custom scripts).

use crate::backup;
use crate::rofi;
use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RofiAppsError {
    #[error("{0}")]
    Message(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("backup error: {0}")]
    Backup(#[from] backup::BackupError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RofiAppsStore {
    #[serde(default)]
    pub apps: Vec<RofiApp>,
}

impl Default for RofiAppsStore {
    fn default() -> Self {
        Self { apps: Vec::new() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RofiApp {
    /// Stable id, e.g. `clipboard` or `custom-wifi`.
    pub id: String,
    pub label: String,
    /// Preset kind: clipboard, power, wifi, launcher, custom.
    pub kind: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Script basename under `~/.config/rofi/scripts/`.
    pub script_name: String,
    /// Optional override of the generated script body. Empty = use preset template.
    #[serde(default)]
    pub script_override: String,
    /// Binaries required for this app to work.
    #[serde(default)]
    pub requires: Vec<String>,
    /// Optional daemon / watcher started at Hyprland start.
    #[serde(default)]
    pub daemon: String,
    #[serde(default = "default_start")]
    pub daemon_when: String,
    /// Suggested bind keys shown in the UI (not auto-applied).
    #[serde(default)]
    pub bind_suggested: String,
    #[serde(default)]
    pub notes: String,
}

fn default_true() -> bool {
    true
}
fn default_start() -> String {
    "start".into()
}

#[derive(Debug, Clone, Copy)]
pub struct AppPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub kind: &'static str,
    pub description: &'static str,
    pub script_name: &'static str,
    pub requires: &'static [&'static str],
    pub daemon: &'static str,
    pub bind_suggested: &'static str,
}

pub fn catalog() -> Vec<AppPreset> {
    vec![
        AppPreset {
            id: "clipboard",
            label: "Clipboard history",
            kind: "clipboard",
            description: "cliphist + rofi with image thumbnails (needs wl-paste watcher)",
            script_name: "clipboard.sh",
            requires: &["rofi", "cliphist", "wl-copy", "wl-paste"],
            // Text + image watchers so screenshots / copied images are stored.
            daemon: "sh -c 'wl-paste --type text --watch cliphist store & exec wl-paste --type image --watch cliphist store'",
            bind_suggested: "SUPER + V",
        },
        AppPreset {
            id: "power",
            label: "Power menu",
            kind: "power",
            description: "Lock, logout, suspend, reboot, shutdown via rofi",
            script_name: "power.sh",
            requires: &["rofi", "systemctl", "loginctl"],
            daemon: "",
            bind_suggested: "SUPER + SHIFT + E",
        },
        AppPreset {
            id: "wifi",
            label: "Wi-Fi picker",
            kind: "wifi",
            description: "Scan and connect with nmcli + rofi",
            script_name: "wifi.sh",
            requires: &["rofi", "nmcli"],
            daemon: "",
            bind_suggested: "SUPER + SHIFT + W",
        },
        AppPreset {
            id: "launcher",
            label: "App launcher",
            kind: "launcher",
            description: "Thin wrapper around `rofi -show drun` (uses Rofi Studio theme)",
            script_name: "launcher.sh",
            requires: &["rofi"],
            daemon: "",
            bind_suggested: "SUPER + D",
        },
        AppPreset {
            id: "window",
            label: "Window switcher",
            kind: "launcher",
            description: "`rofi -show window`",
            script_name: "window.sh",
            requires: &["rofi"],
            daemon: "",
            bind_suggested: "SUPER + TAB",
        },
        AppPreset {
            id: "run",
            label: "Run command",
            kind: "launcher",
            description: "`rofi -show run`",
            script_name: "run.sh",
            requires: &["rofi"],
            daemon: "",
            bind_suggested: "SUPER + R",
        },
    ]
}

pub fn preset_by_id(id: &str) -> Option<AppPreset> {
    catalog().into_iter().find(|p| p.id == id)
}

pub fn store_path() -> Option<PathBuf> {
    let mut p = dirs::config_dir()?;
    p.push("hyprbinds");
    p.push("rofi-apps.json");
    Some(p)
}

pub fn scripts_dir() -> Option<PathBuf> {
    let mut p = rofi::rofi_dir()?;
    p.push("scripts");
    Some(p)
}

pub fn script_path(script_name: &str) -> Option<PathBuf> {
    Some(scripts_dir()?.join(script_name))
}

pub fn load() -> RofiAppsStore {
    let Some(path) = store_path() else {
        return RofiAppsStore::default();
    };
    let mut store = match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => RofiAppsStore::default(),
    };
    migrate_clipboard_apps(&mut store);
    store
}

/// Refresh clipboard presets that still use the old text-only watcher / dmenu script.
fn migrate_clipboard_apps(store: &mut RofiAppsStore) {
    let Some(preset) = preset_by_id("clipboard") else {
        return;
    };
    for app in &mut store.apps {
        if app.kind != "clipboard" {
            continue;
        }
        let old_text_only = app.daemon.contains("--type text")
            && !app.daemon.contains("--type image")
            && !app.daemon.contains("--watch cliphist store &");
        if old_text_only || app.daemon == "wl-paste --type text --watch cliphist store" {
            app.daemon = preset.daemon.to_string();
        }
        if app.notes.contains("dmenu picker") {
            app.notes = preset.description.to_string();
        }
        // Drop stale override so Apply regenerates the image-preview script,
        // unless the user clearly customized it.
        if !app.script_override.trim().is_empty()
            && !app.script_override.contains("MAX_THUMBS")
            && (app.script_override.contains("rofi -dmenu")
                || app.script_override.contains("ICON_SIZE=\"5em\"")
                || app.script_override.contains("size: 5em")
                || app.script_override.contains("rm -rf"))
        {
            app.script_override.clear();
        }
    }
}

pub fn save_store(store: &RofiAppsStore) -> Result<(), RofiAppsError> {
    let path = store_path()
        .ok_or_else(|| RofiAppsError::Message("Could not resolve hyprbinds config dir".into()))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(store)?;
    fs::write(path, format!("{text}\n"))?;
    Ok(())
}

pub fn app_from_preset(preset: &AppPreset) -> RofiApp {
    RofiApp {
        id: preset.id.to_string(),
        label: preset.label.to_string(),
        kind: preset.kind.to_string(),
        enabled: true,
        script_name: preset.script_name.to_string(),
        script_override: String::new(),
        requires: preset.requires.iter().map(|s| (*s).to_string()).collect(),
        daemon: preset.daemon.to_string(),
        daemon_when: "start".into(),
        bind_suggested: preset.bind_suggested.to_string(),
        notes: preset.description.to_string(),
    }
}

pub fn make_custom(label: &str, command: &str) -> RofiApp {
    let slug = slugify(label);
    let id = format!("custom-{slug}");
    let script_name = format!("{slug}.sh");
    let body = format!(
        r#"#!/usr/bin/env bash
# >>> hyprbinds:rofi-app:{id}
# Custom Rofi app — edit freely; Apply will keep this override.
set -euo pipefail
{command}
# <<< hyprbinds:rofi-app:{id}
"#
    );
    RofiApp {
        id,
        label: if label.trim().is_empty() {
            "Custom app".into()
        } else {
            label.trim().to_string()
        },
        kind: "custom".into(),
        enabled: true,
        script_name,
        script_override: body,
        requires: vec!["rofi".into()],
        daemon: String::new(),
        daemon_when: "start".into(),
        bind_suggested: String::new(),
        notes: "User-defined script".into(),
    }
}

fn slugify(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if (c == '-' || c == '_' || c.is_whitespace()) && !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "app".into()
    } else {
        out
    }
}

/// Render script body for an app (override wins).
pub fn render_script(app: &RofiApp) -> String {
    if !app.script_override.trim().is_empty() {
        return ensure_shebang(&app.script_override);
    }
    match app.kind.as_str() {
        "clipboard" => clipboard_script(&app.id),
        "power" => power_script(&app.id),
        "wifi" => wifi_script(&app.id),
        "launcher" if app.id == "window" => simple_show_script(&app.id, "window"),
        "launcher" if app.id == "run" => simple_show_script(&app.id, "run"),
        "launcher" => simple_show_script(&app.id, "drun"),
        _ => format!(
            "#!/usr/bin/env bash\n# >>> hyprbinds:rofi-app:{}\necho 'No template for kind {}' >&2\nexit 1\n# <<< hyprbinds:rofi-app:{}\n",
            app.id, app.kind, app.id
        ),
    }
}

fn ensure_shebang(body: &str) -> String {
    let trimmed = body.trim_start();
    if trimmed.starts_with("#!") {
        if body.ends_with('\n') {
            body.to_string()
        } else {
            format!("{body}\n")
        }
    } else {
        format!("#!/usr/bin/env bash\n{body}\n")
    }
}

fn clipboard_script(id: &str) -> String {
    format!(
        r#"#!/usr/bin/env bash
# >>> hyprbinds:rofi-app:{id}
# Clipboard history via cliphist + rofi with image thumbnails
# (managed by Hyprbinds Rofi Apps).
#
# Performance notes:
# - Persistent thumbnail cache (no wipe on each open)
# - Cap listed rows + how many fresh thumbs we build
# - Missing thumbs are generated in the background for the *next* open
set -euo pipefail

if ! command -v cliphist >/dev/null 2>&1; then
  echo "cliphist not found" >&2
  exit 1
fi
if ! command -v rofi >/dev/null 2>&1; then
  echo "rofi not found" >&2
  exit 1
fi
if ! command -v wl-copy >/dev/null 2>&1; then
  echo "wl-copy not found — install wl-clipboard" >&2
  exit 1
fi

SCRIPT_PATH="$(readlink -f "${{BASH_SOURCE[0]}}" 2>/dev/null || realpath "${{BASH_SOURCE[0]}}" 2>/dev/null || echo "${{BASH_SOURCE[0]}}")"
CACHE_DIR="${{XDG_CACHE_HOME:-${{HOME}}/.cache}}/hyprbinds-cliphist"
ICON_SIZE="2em"
LIST_LINES="10"
THUMB_PX="64"
MAX_ITEMS="40"
MAX_THUMBS="8"

# Daemon helpers: store both text and images.
if [[ "${{1:-}}" == "--watch" ]]; then
  if ! command -v wl-paste >/dev/null 2>&1; then
    echo "wl-paste not found — install wl-clipboard" >&2
    exit 1
  fi
  wl-paste --type text --watch cliphist store &
  exec wl-paste --type image --watch cliphist store
fi

# Outside rofi → toggle clipboard mode (close if already open).
if [[ -z "${{ROFI_RETV:-}}" ]]; then
  # Match this script's clipboard instance only (not every rofi).
  if pgrep -f -- "rofi.*clipboard:${{SCRIPT_PATH}}" >/dev/null 2>&1; then
    pkill -f -- "rofi.*clipboard:${{SCRIPT_PATH}}" 2>/dev/null || true
    exit 0
  fi
  exec rofi -modi "clipboard:${{SCRIPT_PATH}}" -show clipboard -show-icons \
    -theme-str "listview {{ lines: ${{LIST_LINES}}; fixed-height: true; }}" \
    -theme-str "element {{ padding: 4px 8px; }}" \
    -theme-str "element-icon {{ size: ${{ICON_SIZE}}; }}"
fi

# Inside rofi script mode: selection → copy to clipboard.
if [[ -n "${{1:-}}" ]]; then
  cliphist decode <<<"$1" | wl-copy
  exit 0
fi

# Inside rofi script mode: list entries quickly.
mkdir -p "${{CACHE_DIR}}"

queue_thumb() {{
  # Build a missing thumbnail in the background so this open stays fast.
  local data="$1" thumb="$2"
  (
    [[ -s "${{thumb}}" ]] && exit 0
    local tmp="${{thumb}}.$$"
    cliphist decode <<<"${{data}}" >"${{tmp}}" 2>/dev/null || {{ rm -f "${{tmp}}"; exit 0; }}
    if command -v magick >/dev/null 2>&1; then
      magick "${{tmp}}" -thumbnail "${{THUMB_PX}}x${{THUMB_PX}}>" "${{thumb}}" 2>/dev/null \
        || mv -f "${{tmp}}" "${{thumb}}"
      rm -f "${{tmp}}"
    elif command -v convert >/dev/null 2>&1; then
      convert "${{tmp}}" -thumbnail "${{THUMB_PX}}x${{THUMB_PX}}>" "${{thumb}}" 2>/dev/null \
        || mv -f "${{tmp}}" "${{thumb}}"
      rm -f "${{tmp}}"
    else
      mv -f "${{tmp}}" "${{thumb}}"
    fi
  ) >/dev/null 2>&1 &
}}

items=0
thumbs=0
# Avoid pipefail aborting the whole script on SIGPIPE from head.
set +o pipefail
cliphist list | head -n "${{MAX_ITEMS}}" | while IFS= read -r data; do
  case "${{data}}" in
    *'<meta http-equiv="content-type"'*) continue ;;
  esac

  id="${{data%%$'\t'*}}"
  rest="${{data#*$'\t'}}"

  ext=""
  if [[ "${{rest}}" =~ [[:space:]](png|jpe?g|bmp|webp|gif)([[:space:]]|]]) ]]; then
    ext="${{BASH_REMATCH[1]}}"
    [[ "${{ext}}" == "jpeg" ]] && ext="jpg"
  fi

  if [[ -n "${{ext}}" && -n "${{id}}" && "${{thumbs}}" -lt "${{MAX_THUMBS}}" ]]; then
    thumbs=$((thumbs + 1))
    thumb="${{CACHE_DIR}}/${{id}}.${{ext}}"
    if [[ -s "${{thumb}}" ]]; then
      printf '%s\0icon\x1f%s\n' "${{data}}" "${{thumb}}"
      continue
    fi
    # Show the row now; bake the icon for next launch.
    queue_thumb "${{data}}" "${{thumb}}"
  fi

  printf '%s\n' "${{data}}"
done
set -o pipefail
# <<< hyprbinds:rofi-app:{id}
"#
    )
}

fn power_script(id: &str) -> String {
    format!(
        r#"#!/usr/bin/env bash
# >>> hyprbinds:rofi-app:{id}
# Power menu (managed by Hyprbinds Rofi Apps).
set -euo pipefail

choice="$(printf '%s\n' \
  "Lock" \
  "Logout" \
  "Suspend" \
  "Reboot" \
  "Shutdown" \
  | rofi -dmenu -i -p "Power")"

case "${{choice}}" in
  Lock)
    if command -v hyprlock >/dev/null 2>&1; then
      hyprlock
    elif command -v loginctl >/dev/null 2>&1; then
      loginctl lock-session
    else
      echo "No lock command found" >&2
      exit 1
    fi
    ;;
  Logout)
    if command -v hyprctl >/dev/null 2>&1; then
      hyprctl dispatch exit
    else
      loginctl terminate-user ""
    fi
    ;;
  Suspend) systemctl suspend ;;
  Reboot) systemctl reboot ;;
  Shutdown) systemctl poweroff ;;
  *) exit 0 ;;
esac
# <<< hyprbinds:rofi-app:{id}
"#
    )
}

fn wifi_script(id: &str) -> String {
    format!(
        r#"#!/usr/bin/env bash
# >>> hyprbinds:rofi-app:{id}
# Wi-Fi picker via nmcli + rofi (managed by Hyprbinds Rofi Apps).
set -euo pipefail

if ! command -v nmcli >/dev/null 2>&1; then
  echo "nmcli not found — install NetworkManager" >&2
  exit 1
fi

notify() {{
  if command -v notify-send >/dev/null 2>&1; then
    notify-send "Wi-Fi" "$1"
  else
    echo "$1" >&2
  fi
}}

choice="$(printf '%s\n' \
  "Scan & connect" \
  "Disconnect" \
  "Enable Wi-Fi" \
  "Disable Wi-Fi" \
  | rofi -dmenu -i -p "Wi-Fi")"

case "${{choice}}" in
  "Scan & connect")
    notify "Scanning…"
    nmcli device wifi rescan >/dev/null 2>&1 || true
    sleep 1
    ssid="$(nmcli -t -f SSID,SIGNAL,SECURITY device wifi list \
      | awk -F: 'NF && $1 != "" {{ printf "%s  (%s%%)  %s\n", $1, $2, $3 }}' \
      | rofi -dmenu -i -p "Network" \
      | awk '{{print $1}}')"
    [[ -z "${{ssid}}" ]] && exit 0
    if nmcli --ask device wifi connect "${{ssid}}"; then
      notify "Connected to ${{ssid}}"
    else
      notify "Failed to connect to ${{ssid}}"
    fi
    ;;
  Disconnect)
    device="$(nmcli -t -f DEVICE,TYPE device status | awk -F: '$2=="wifi"{{print $1; exit}}')"
    [[ -n "${{device}}" ]] && nmcli device disconnect "${{device}}"
    ;;
  "Enable Wi-Fi") nmcli radio wifi on ;;
  "Disable Wi-Fi") nmcli radio wifi off ;;
  *) exit 0 ;;
esac
# <<< hyprbinds:rofi-app:{id}
"#
    )
}

fn simple_show_script(id: &str, mode: &str) -> String {
    format!(
        r#"#!/usr/bin/env bash
# >>> hyprbinds:rofi-app:{id}
# Managed by Hyprbinds Rofi Apps.
exec rofi -show {mode}
# <<< hyprbinds:rofi-app:{id}
"#
    )
}

/// Write enabled app scripts + JSON store. Disabled apps keep JSON but scripts are removed.
pub fn apply(store: &RofiAppsStore) -> Result<String, RofiAppsError> {
    let scripts = scripts_dir()
        .ok_or_else(|| RofiAppsError::Message("Could not resolve ~/.config/rofi/scripts".into()))?;
    fs::create_dir_all(&scripts)?;

    let mut written = 0usize;
    let mut removed = 0usize;

    for app in &store.apps {
        let path = scripts.join(&app.script_name);
        if app.enabled {
            if path.is_file() {
                backup::snapshot_before_write(&path)?;
            }
            let body = render_script(app);
            write_executable(&path, &body)?;
            written += 1;
        } else if path.is_file() && is_managed_script(&path, &app.id) {
            backup::snapshot_before_write(&path)?;
            fs::remove_file(&path)?;
            removed += 1;
        }
    }

    save_store(store)?;
    Ok(format!(
        "Rofi Apps: wrote {written} script(s), removed {removed}, saved {}",
        store_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "rofi-apps.json".into())
    ))
}

fn is_managed_script(path: &Path, id: &str) -> bool {
    fs::read_to_string(path)
        .map(|t| t.contains(&format!("hyprbinds:rofi-app:{id}")))
        .unwrap_or(false)
}

fn write_executable(path: &Path, body: &str) -> Result<(), RofiAppsError> {
    let tmp = path.with_extension("hyprbinds.tmp");
    fs::write(&tmp, body)?;
    let mut perms = fs::metadata(&tmp)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&tmp, perms)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn missing_requires(app: &RofiApp) -> Vec<String> {
    app.requires
        .iter()
        .filter(|bin| !rofi::command_exists(bin))
        .cloned()
        .collect()
}

pub fn deps_label(app: &RofiApp) -> String {
    let missing = missing_requires(app);
    if missing.is_empty() {
        "deps ok".into()
    } else {
        format!("missing: {}", missing.join(", "))
    }
}

pub fn launch_command(app: &RofiApp) -> Result<String, RofiAppsError> {
    let path = script_path(&app.script_name).ok_or_else(|| {
        RofiAppsError::Message("Could not resolve script path".into())
    })?;
    if !path.is_file() {
        return Err(RofiAppsError::Message(format!(
            "Script not found — Apply first: {}",
            path.display()
        )));
    }
    Ok(path.display().to_string())
}

pub fn test_run(app: &RofiApp) -> Result<String, RofiAppsError> {
    let path = launch_command(app)?;
    let missing = missing_requires(app);
    if !missing.is_empty() {
        return Err(RofiAppsError::Message(format!(
            "Missing dependencies: {}",
            missing.join(", ")
        )));
    }

    if rofi::command_exists("setsid") {
        Command::new("setsid")
            .args(["-f", &path])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| RofiAppsError::Message(format!("failed to launch: {e}")))?;
    } else {
        Command::new("sh")
            .args(["-c", &format!("\"{path}\" >/dev/null 2>&1 &")])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| RofiAppsError::Message(format!("failed to launch: {e}")))?;
    }
    Ok(format!("Launched {}", path))
}

/// Absolute path suitable for `hl.dsp.exec_cmd(...)`.
pub fn exec_path_for(app: &RofiApp) -> Option<String> {
    script_path(&app.script_name).map(|p| p.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_script_mentions_cliphist() {
        let app = app_from_preset(&preset_by_id("clipboard").unwrap());
        let body = render_script(&app);
        assert!(body.contains("cliphist"));
        assert!(body.contains("show-icons"));
        assert!(body.contains("MAX_THUMBS"));
        assert!(body.contains("\\0icon\\x1f"));
        assert!(body.starts_with("#!"));
    }

    #[test]
    fn custom_slug() {
        let app = make_custom("My Cool App", "rofi -show drun");
        assert_eq!(app.id, "custom-my-cool-app");
        assert!(app.script_name.ends_with(".sh"));
        assert!(app.script_override.contains("rofi -show drun"));
    }

    #[test]
    fn catalog_has_clipboard() {
        assert!(catalog().iter().any(|p| p.id == "clipboard"));
    }
}
