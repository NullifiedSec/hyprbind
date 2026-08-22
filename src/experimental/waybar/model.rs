//! In-memory Waybar bar model (primary JSON + include awareness).

use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    Left,
    Center,
    Right,
}

impl Zone {
    pub fn key(self) -> &'static str {
        match self {
            Self::Left => "modules-left",
            Self::Center => "modules-center",
            Self::Right => "modules-right",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Left => "Left",
            Self::Center => "Center",
            Self::Right => "Right",
        }
    }

    pub fn all() -> [Zone; 3] {
        [Self::Left, Self::Center, Self::Right]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleOrigin {
    Primary,
    Included(PathBuf),
}

#[derive(Debug, Clone)]
pub struct WaybarModel {
    pub config_path: PathBuf,
    pub style_path: PathBuf,
    pub root_is_array: bool,
    /// Keys present in the primary file before include merge.
    pub primary_keys: HashSet<String>,
    /// Module config keys that came only from includes (read-only configs).
    pub included_origins: HashMap<String, PathBuf>,
    pub bar: Map<String, Value>,
    pub style_css: String,
    pub notes: Vec<String>,
}

impl WaybarModel {
    pub fn empty(config_path: PathBuf, style_path: PathBuf) -> Self {
        let mut bar = Map::new();
        bar.insert("layer".into(), json!("top"));
        bar.insert("position".into(), json!("top"));
        bar.insert("height".into(), json!(34));
        bar.insert("spacing".into(), json!(4));
        bar.insert("margin-top".into(), json!(6));
        bar.insert("margin-left".into(), json!(8));
        bar.insert("margin-right".into(), json!(8));
        bar.insert("margin-bottom".into(), json!(0));
        bar.insert("exclusive".into(), json!(true));
        bar.insert("reload_style_on_change".into(), json!(true));
        bar.insert(
            "modules-left".into(),
            json!(["hyprland/workspaces", "hyprland/window"]),
        );
        bar.insert("modules-center".into(), json!(["clock"]));
        bar.insert(
            "modules-right".into(),
            json!([
                "cpu",
                "memory",
                "pulseaudio",
                "network",
                "bluetooth",
                "battery",
                "tray"
            ]),
        );
        bar.insert(
            "hyprland/workspaces".into(),
            json!({
                "disable-scroll": false,
                "all-outputs": true,
                "format": "{icon}",
                "on-click": "activate",
                "sort-by": "number",
                "format-icons": {
                    "1": "1",
                    "2": "2",
                    "3": "3",
                    "4": "4",
                    "5": "5",
                    "6": "6",
                    "7": "7",
                    "8": "8",
                    "9": "9",
                    "10": "10",
                    "urgent": "!",
                    "active": "●",
                    "default": "○",
                    "empty": "○"
                },
                "persistent-workspaces": { "*": 5 }
            }),
        );
        bar.insert(
            "hyprland/window".into(),
            json!({
                "format": "{title}",
                "max-length": 48,
                "tooltip": true
            }),
        );
        bar.insert(
            "clock".into(),
            json!({
                "format": "{:%H:%M}",
                "format-alt": "{:%a %d %b %Y}",
                "tooltip-format": "<tt><small>{calendar}</small></tt>",
                "interval": 60
            }),
        );
        bar.insert(
            "cpu".into(),
            json!({
                "format": "CPU {usage}%",
                "tooltip": true,
                "interval": 2
            }),
        );
        bar.insert(
            "memory".into(),
            json!({
                "format": "MEM {percentage}%",
                "tooltip": true,
                "interval": 2
            }),
        );
        bar.insert(
            "pulseaudio".into(),
            json!({
                "format": "{icon} {volume}%",
                "format-muted": "muted",
                "format-icons": {
                    "headphone": "♪",
                    "headset": "♪",
                    "default": ["♪", "♪", "♪"]
                },
                "on-click": "pavucontrol"
            }),
        );
        bar.insert(
            "network".into(),
            json!({
                "format-wifi": "WiFi {essid}",
                "format-ethernet": "ETH",
                "format-disconnected": "offline",
                "tooltip": true,
                "on-click": "nm-connection-editor"
            }),
        );
        bar.insert(
            "bluetooth".into(),
            json!({
                "format": "BT",
                "format-disabled": "BT off",
                "format-connected": "BT {device_alias}",
                "tooltip": true,
                "on-click": "blueman-manager",
                "max-length": 20
            }),
        );
        bar.insert(
            "battery".into(),
            json!({
                "states": { "warning": 30, "critical": 15 },
                "format": "{capacity}%",
                "format-charging": "⚡ {capacity}%",
                "format-plugged": "⚡ {capacity}%",
                "tooltip": true
            }),
        );
        bar.insert(
            "tray".into(),
            json!({
                "icon-size": 16,
                "spacing": 8
            }),
        );

        let primary_keys: HashSet<String> = bar.keys().cloned().collect();
        Self {
            config_path,
            style_path,
            root_is_array: false,
            primary_keys,
            included_origins: HashMap::new(),
            bar,
            style_css: crate::experimental::waybar::style::DEFAULT_STYLE.to_string(),
            notes: vec!["Created a Hyprland-native Waybar Studio template.".into()],
        }
    }

    pub fn zone_modules(&self, zone: Zone) -> Vec<String> {
        self.bar
            .get(zone.key())
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn set_zone_modules(&mut self, zone: Zone, modules: Vec<String>) {
        let arr: Vec<Value> = modules.into_iter().map(Value::String).collect();
        self.bar.insert(zone.key().into(), Value::Array(arr));
        self.primary_keys.insert(zone.key().into());
    }

    pub fn all_layout_modules(&self) -> Vec<String> {
        let mut out = Vec::new();
        for zone in Zone::all() {
            out.extend(self.zone_modules(zone));
        }
        out
    }

    pub fn module_origin(&self, name: &str) -> ModuleOrigin {
        if let Some(path) = self.included_origins.get(name) {
            if !self.primary_keys.contains(name) {
                return ModuleOrigin::Included(path.clone());
            }
        }
        ModuleOrigin::Primary
    }

    pub fn is_module_readonly(&self, name: &str) -> bool {
        matches!(self.module_origin(name), ModuleOrigin::Included(_))
    }

    pub fn module_config(&self, name: &str) -> Option<&Value> {
        self.bar.get(name)
    }

    pub fn set_module_config(&mut self, name: &str, value: Value) -> Result<(), String> {
        if self.is_module_readonly(name) {
            return Err(format!(
                "{name} is defined in an include file and is read-only here"
            ));
        }
        self.bar.insert(name.to_string(), value);
        self.primary_keys.insert(name.to_string());
        Ok(())
    }

    pub fn ensure_module_config(&mut self, name: &str) {
        if self.bar.contains_key(name) || self.is_module_readonly(name) {
            return;
        }
        let default = crate::experimental::waybar::modules::default_config_for(name);
        self.bar.insert(name.to_string(), default);
        self.primary_keys.insert(name.to_string());
    }

    pub fn remove_module_from_layout(&mut self, name: &str) {
        for zone in Zone::all() {
            let mut mods = self.zone_modules(zone);
            mods.retain(|m| m != name);
            self.set_zone_modules(zone, mods);
        }
    }

    pub fn move_module_in_zone(&mut self, zone: Zone, index: usize, delta: isize) {
        let mut mods = self.zone_modules(zone);
        if mods.is_empty() || index >= mods.len() {
            return;
        }
        let new_idx = index as isize + delta;
        if new_idx < 0 || new_idx as usize >= mods.len() {
            return;
        }
        mods.swap(index, new_idx as usize);
        self.set_zone_modules(zone, mods);
    }

    pub fn move_module_to_zone(&mut self, name: &str, from: Zone, to: Zone, to_index: Option<usize>) {
        let mut from_mods = self.zone_modules(from);
        from_mods.retain(|m| m != name);
        self.set_zone_modules(from, from_mods);

        let mut to_mods = self.zone_modules(to);
        to_mods.retain(|m| m != name);
        match to_index {
            Some(i) if i <= to_mods.len() => to_mods.insert(i, name.to_string()),
            _ => to_mods.push(name.to_string()),
        }
        self.set_zone_modules(to, to_mods);
        self.ensure_module_config(name);
    }

    pub fn add_module_to_zone(&mut self, zone: Zone, name: &str) {
        let mut mods = self.zone_modules(zone);
        if !mods.iter().any(|m| m == name) {
            mods.push(name.to_string());
            self.set_zone_modules(zone, mods);
        }
        self.ensure_module_config(name);
    }

    pub fn bar_string(&self, key: &str) -> Option<String> {
        self.bar.get(key).and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            _ => None,
        })
    }

    pub fn bar_i64(&self, key: &str) -> Option<i64> {
        self.bar.get(key).and_then(|v| v.as_i64())
    }

    pub fn bar_bool(&self, key: &str) -> Option<bool> {
        self.bar.get(key).and_then(|v| v.as_bool())
    }

    pub fn set_bar_value(&mut self, key: &str, value: Value) {
        self.bar.insert(key.to_string(), value);
        self.primary_keys.insert(key.to_string());
    }

    /// Build the Value to write back to the primary config file.
    pub fn to_primary_value(&self) -> Value {
        let mut out = Map::new();
        for (k, v) in &self.bar {
            // Skip module configs that exist only in includes.
            if self.included_origins.contains_key(k) && !self.primary_keys.contains(k) {
                continue;
            }
            // Always keep layout keys and bar options that were primary or edited.
            if is_layout_key(k) || is_bar_option(k) || self.primary_keys.contains(k) {
                out.insert(k.clone(), v.clone());
            }
        }
        // Preserve include array from primary if present.
        if let Some(inc) = self.bar.get("include") {
            out.insert("include".into(), inc.clone());
        }
        Value::Object(out)
    }

    pub fn preview_label_for(&self, module: &str) -> String {
        if let Some(cfg) = self.module_config(module) {
            if let Some(fmt) = cfg.get("format").and_then(|v| v.as_str()) {
                let simplified = simplify_format(fmt);
                if !simplified.is_empty() {
                    return simplified;
                }
            }
        }
        short_module_name(module)
    }
}

fn is_layout_key(k: &str) -> bool {
    matches!(
        k,
        "modules-left" | "modules-center" | "modules-right" | "include"
    )
}

fn is_bar_option(k: &str) -> bool {
    matches!(
        k,
        "layer"
            | "position"
            | "height"
            | "width"
            | "spacing"
            | "margin"
            | "margin-top"
            | "margin-bottom"
            | "margin-left"
            | "margin-right"
            | "exclusive"
            | "fixed-center"
            | "passthrough"
            | "gtk-layer-shell"
            | "mode"
            | "output"
            | "name"
            | "id"
            | "ipc"
            | "reload_style_on_change"
            | "on-sigusr1"
            | "on-sigusr2"
    )
}

fn short_module_name(name: &str) -> String {
    name.rsplit('/').next().unwrap_or(name).to_string()
}

fn simplify_format(fmt: &str) -> String {
    // Replace {…} tokens with placeholders for preview.
    let mut out = String::new();
    let mut chars = fmt.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            while let Some(n) = chars.next() {
                if n == '}' {
                    break;
                }
            }
            out.push('…');
        } else {
            out.push(c);
        }
    }
    out.trim().chars().take(24).collect()
}
