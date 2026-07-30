use serde::Deserialize;
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct RunningWindow {
    pub class: String,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub name: String,
    pub description: String,
    pub width: i64,
    pub height: i64,
    pub refresh_rate: f64,
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Error)]
pub enum ClientsError {
    #[error("failed to run hyprctl: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("hyprctl failed: {0}")]
    Failed(String),
    #[error("failed to parse hyprctl JSON: {0}")]
    Parse(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize)]
struct RawClient {
    class: Option<String>,
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawMonitor {
    name: Option<String>,
    description: Option<String>,
    width: Option<i64>,
    height: Option<i64>,
    #[serde(rename = "refreshRate", default)]
    refresh_rate: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct RawDevices {
    #[serde(default)]
    keyboards: Vec<RawNamed>,
    #[serde(default)]
    mice: Vec<RawNamed>,
    #[serde(default)]
    touchpads: Option<Vec<RawNamed>>,
    #[serde(default)]
    tablets: Option<Vec<RawNamed>>,
}

#[derive(Debug, Deserialize)]
struct RawNamed {
    name: Option<String>,
}

fn hyprctl_json(args: &[&str]) -> Result<Vec<u8>, ClientsError> {
    let output = Command::new("hyprctl").args(args).output()?;
    if !output.status.success() {
        return Err(ClientsError::Failed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(output.stdout)
}

/// List mapped clients from the running Hyprland instance.
pub fn list_running_windows() -> Result<Vec<RunningWindow>, ClientsError> {
    let raw: Vec<RawClient> = serde_json::from_slice(&hyprctl_json(&["clients", "-j"])?)?;
    let mut windows = Vec::new();
    for c in raw {
        let class = c.class.unwrap_or_default();
        if class.is_empty() {
            continue;
        }
        windows.push(RunningWindow {
            class,
            title: c.title.unwrap_or_default(),
        });
    }
    windows.sort_by(|a, b| a.class.cmp(&b.class).then_with(|| a.title.cmp(&b.title)));
    Ok(windows)
}

pub fn list_monitors() -> Result<Vec<MonitorInfo>, ClientsError> {
    // Prefer all monitors (incl. disabled).
    let bytes = hyprctl_json(&["monitors", "all", "-j"]).or_else(|_| hyprctl_json(&["monitors", "-j"]))?;
    let raw: Vec<RawMonitor> = serde_json::from_slice(&bytes)?;
    Ok(raw
        .into_iter()
        .filter_map(|m| {
            let name = m.name.unwrap_or_default();
            if name.is_empty() {
                return None;
            }
            Some(MonitorInfo {
                name,
                description: m.description.unwrap_or_default(),
                width: m.width.unwrap_or(0),
                height: m.height.unwrap_or(0),
                refresh_rate: m.refresh_rate.unwrap_or(0.0),
            })
        })
        .collect())
}

pub fn list_devices() -> Result<Vec<DeviceInfo>, ClientsError> {
    let raw: RawDevices = serde_json::from_slice(&hyprctl_json(&["devices", "-j"])?)?;
    let mut out = Vec::new();
    for k in raw.keyboards {
        if let Some(n) = k.name {
            if !n.is_empty() {
                out.push(DeviceInfo {
                    name: n,
                    kind: "keyboard".into(),
                });
            }
        }
    }
    for m in raw.mice {
        if let Some(n) = m.name {
            if !n.is_empty() {
                out.push(DeviceInfo {
                    name: n,
                    kind: "mouse".into(),
                });
            }
        }
    }
    if let Some(pads) = raw.touchpads {
        for t in pads {
            if let Some(n) = t.name {
                if !n.is_empty() {
                    out.push(DeviceInfo {
                        name: n,
                        kind: "touchpad".into(),
                    });
                }
            }
        }
    }
    if let Some(tabs) = raw.tablets {
        for t in tabs {
            if let Some(n) = t.name {
                if !n.is_empty() {
                    out.push(DeviceInfo {
                        name: n,
                        kind: "tablet".into(),
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.name.cmp(&b.name)));
    out.dedup_by(|a, b| a.name == b.name);
    Ok(out)
}

pub fn list_layer_namespaces() -> Result<Vec<String>, ClientsError> {
    let value: serde_json::Value = serde_json::from_slice(&hyprctl_json(&["layers", "-j"])?)?;
    let mut names = Vec::new();
    collect_namespaces(&value, &mut names);
    names.sort();
    names.dedup();
    Ok(names)
}

fn collect_namespaces(v: &serde_json::Value, out: &mut Vec<String>) {
    match v {
        serde_json::Value::Object(map) => {
            if let Some(ns) = map.get("namespace").and_then(|x| x.as_str()) {
                if !ns.is_empty() {
                    out.push(ns.to_string());
                }
            }
            for val in map.values() {
                collect_namespaces(val, out);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                collect_namespaces(item, out);
            }
        }
        _ => {}
    }
}

pub fn list_layouts() -> Result<Vec<String>, ClientsError> {
    let value: serde_json::Value = serde_json::from_slice(&hyprctl_json(&["layouts", "-j"])?)?;
    let mut layouts = Vec::new();
    match value {
        serde_json::Value::Array(arr) => {
            for item in arr {
                if let Some(s) = item.as_str() {
                    layouts.push(s.to_string());
                } else if let Some(s) = item.get("name").and_then(|x| x.as_str()) {
                    layouts.push(s.to_string());
                }
            }
        }
        serde_json::Value::Object(map) => {
            for key in map.keys() {
                layouts.push(key.clone());
            }
        }
        _ => {}
    }
    if layouts.is_empty() {
        layouts.extend(
            ["dwindle", "master", "scrolling"]
                .iter()
                .map(|s| (*s).to_string()),
        );
    }
    layouts.sort();
    layouts.dedup();
    Ok(layouts)
}
