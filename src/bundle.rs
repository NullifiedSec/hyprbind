//! Canonical hyprbinds-export JSON — snapshot and restore settings across apps.

use crate::bind::BindCollection;
use crate::config;
use crate::rofi;
use crate::rofi_apps::{self, RofiAppsStore};
use crate::ui_prefs::{self, UiPrefs};
use crate::via;
use crate::via_studio::{self, Animation};
use crate::wallpaper::{self, WallpaperPrefs};
use crate::waybar;
use crate::waybar_model::WaybarModel;
use crate::writer;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const FORMAT_ID: &str = "hyprbinds-export";
pub const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Error)]
pub enum BundleError {
    #[error("{0}")]
    Message(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("config error: {0}")]
    Config(#[from] config::ConfigError),
    #[error("write error: {0}")]
    Write(#[from] writer::WriteError),
    #[error("waybar error: {0}")]
    Waybar(#[from] waybar::WaybarError),
    #[error("rofi error: {0}")]
    Rofi(#[from] rofi::RofiError),
    #[error("rofi apps error: {0}")]
    RofiApps(#[from] rofi_apps::RofiAppsError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyprbindsExport {
    pub format: String,
    pub version: u32,
    #[serde(default)]
    pub exported_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hyprland: Option<BindCollection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waybar: Option<WaybarExport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rofi: Option<RofiExport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<AppExport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via: Option<ViaExport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<SystemExport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaybarExport {
    #[serde(default)]
    pub config: Value,
    #[serde(default)]
    pub style_css: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RofiExport {
    #[serde(default)]
    pub config_rasi: String,
    #[serde(default)]
    pub theme_rasi: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apps: Option<RofiAppsStore>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppExport {
    #[serde(default)]
    pub ui: UiPrefs,
    #[serde(default)]
    pub wallpaper: WallpaperPrefs,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ViaExport {
    #[serde(default)]
    pub definitions: Vec<ViaDefinitionExport>,
    #[serde(default)]
    pub rgb_presets: Vec<Animation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViaDefinitionExport {
    pub filename: String,
    pub body: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemExport {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hyprland_portals_conf: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct ImportOptions {
    pub hyprland: bool,
    pub waybar: bool,
    pub rofi: bool,
    pub app: bool,
    pub via: bool,
    pub system: bool,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            hyprland: true,
            waybar: true,
            rofi: true,
            app: true,
            via: true,
            system: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ImportReport {
    pub messages: Vec<String>,
    pub errors: Vec<String>,
}

impl ImportReport {
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if !self.messages.is_empty() {
            parts.push(self.messages.join("; "));
        }
        if !self.errors.is_empty() {
            parts.push(format!("errors: {}", self.errors.join("; ")));
        }
        if parts.is_empty() {
            "Nothing imported".into()
        } else {
            parts.join(" · ")
        }
    }

    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }
}

pub fn now_iso8601() -> String {
    // Avoid a chrono dependency — UTC-ish local wall clock is fine for metadata.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{secs}")
}

pub fn validate(bundle: &HyprbindsExport) -> Result<(), BundleError> {
    if bundle.format != FORMAT_ID {
        return Err(BundleError::Message(format!(
            "unsupported format {:?} (expected {FORMAT_ID})",
            bundle.format
        )));
    }
    if bundle.version == 0 || bundle.version > FORMAT_VERSION {
        return Err(BundleError::Message(format!(
            "unsupported export version {} (supported 1..{FORMAT_VERSION})",
            bundle.version
        )));
    }
    Ok(())
}

pub fn from_json_str(text: &str) -> Result<HyprbindsExport, BundleError> {
    let bundle: HyprbindsExport = serde_json::from_str(text)?;
    validate(&bundle)?;
    Ok(bundle)
}

pub fn to_pretty_json(bundle: &HyprbindsExport) -> Result<String, BundleError> {
    validate(bundle)?;
    let mut text = serde_json::to_string_pretty(bundle)?;
    text.push('\n');
    Ok(text)
}

pub fn export_all(collection: &BindCollection) -> Result<HyprbindsExport, BundleError> {
    let waybar = match waybar::load() {
        Ok(model) => Some(WaybarExport {
            config: model.to_primary_value(),
            style_css: model.style_css,
        }),
        Err(e) => {
            eprintln!("hyprbinds: waybar export skipped: {e}");
            None
        }
    };

    let apps_store = rofi_apps::load();
    let rofi_export = match rofi::load() {
        Ok(model) => {
            let (config_rasi, theme_rasi) = rofi::export_snapshot(&model);
            Some(RofiExport {
                config_rasi,
                theme_rasi,
                apps: Some(apps_store),
            })
        }
        Err(e) => {
            eprintln!("hyprbinds: rofi theme export skipped: {e}");
            Some(RofiExport {
                config_rasi: String::new(),
                theme_rasi: String::new(),
                apps: Some(apps_store),
            })
        }
    };

    let app = Some(AppExport {
        ui: ui_prefs::load(),
        wallpaper: wallpaper::load_prefs(),
    });

    let via = export_via().unwrap_or_else(|e| {
        eprintln!("hyprbinds: via export skipped: {e}");
        None
    });

    let system = Some(SystemExport {
        hyprland_portals_conf: read_portal_conf(),
    });

    Ok(HyprbindsExport {
        format: FORMAT_ID.into(),
        version: FORMAT_VERSION,
        exported_at: now_iso8601(),
        hyprland: Some(collection.clone()),
        waybar,
        rofi: rofi_export,
        app,
        via,
        system,
    })
}

fn export_via() -> Result<Option<ViaExport>, BundleError> {
    let mut definitions = Vec::new();
    let dir = via::definitions_dir();
    if dir.is_dir() {
        let rd = fs::read_dir(&dir)?;
        for entry in rd.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            let Some(filename) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let text = match fs::read_to_string(&path) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let body: Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(_) => continue,
            };
            definitions.push(ViaDefinitionExport {
                filename: filename.to_string(),
                body,
            });
        }
        definitions.sort_by(|a, b| a.filename.cmp(&b.filename));
    }

    let mut rgb_presets = Vec::new();
    if let Ok(names) = via_studio::list_presets() {
        for name in names {
            if let Ok(anim) = via_studio::load_preset(&name) {
                rgb_presets.push(anim);
            }
        }
    }

    if definitions.is_empty() && rgb_presets.is_empty() {
        return Ok(None);
    }
    Ok(Some(ViaExport {
        definitions,
        rgb_presets,
    }))
}

fn portal_conf_path() -> Option<PathBuf> {
    let mut p = dirs::config_dir()?;
    p.push("xdg-desktop-portal");
    p.push("hyprland-portals.conf");
    Some(p)
}

fn read_portal_conf() -> Option<String> {
    let path = portal_conf_path()?;
    fs::read_to_string(path).ok()
}

pub fn import_all(
    bundle: &HyprbindsExport,
    opts: ImportOptions,
) -> Result<ImportReport, BundleError> {
    validate(bundle)?;
    let mut report = ImportReport::default();

    if opts.hyprland {
        if let Some(hypr) = &bundle.hyprland {
            let path = config::default_config_path().ok_or_else(|| {
                BundleError::Message("could not resolve Hyprland config path".into())
            })?;
            match writer::apply_collection(&path, hypr) {
                Ok(wr) => report
                    .messages
                    .push(format!("hyprland → {}", wr.path)),
                Err(e) => report.errors.push(format!("hyprland: {e}")),
            }
        }
    }

    if opts.waybar {
        if let Some(wb) = &bundle.waybar {
            match import_waybar(wb) {
                Ok(msg) => report.messages.push(msg),
                Err(e) => report.errors.push(format!("waybar: {e}")),
            }
        }
    }

    if opts.rofi {
        if let Some(rf) = &bundle.rofi {
            match import_rofi(rf) {
                Ok(msg) => report.messages.push(msg),
                Err(e) => report.errors.push(format!("rofi: {e}")),
            }
        }
    }

    if opts.app {
        if let Some(app) = &bundle.app {
            ui_prefs::save(&app.ui);
            wallpaper::save_prefs(&app.wallpaper);
            report.messages.push("app prefs saved".into());
        }
    }

    if opts.via {
        if let Some(via_ex) = &bundle.via {
            match import_via(via_ex) {
                Ok(msg) => report.messages.push(msg),
                Err(e) => report.errors.push(format!("via: {e}")),
            }
        }
    }

    if opts.system {
        if let Some(sys) = &bundle.system {
            if let Some(conf) = &sys.hyprland_portals_conf {
                match write_portal_conf(conf) {
                    Ok(msg) => report.messages.push(msg),
                    Err(e) => report.errors.push(format!("portal: {e}")),
                }
            }
        }
    }

    Ok(report)
}

fn import_waybar(wb: &WaybarExport) -> Result<String, BundleError> {
    let config_path = waybar::discover_config_path()
        .ok_or_else(|| BundleError::Message("Could not resolve Waybar config dir".into()))?;
    let style_path = waybar::style_path()
        .ok_or_else(|| BundleError::Message("Could not resolve Waybar style path".into()))?;

    let bar = match &wb.config {
        Value::Object(map) => map.clone(),
        Value::Null if wb.style_css.is_empty() => {
            return Err(BundleError::Message("empty waybar section".into()));
        }
        Value::Null => Map::new(),
        other => {
            return Err(BundleError::Message(format!(
                "waybar.config must be a JSON object, got {other}"
            )));
        }
    };

    let mut model = WaybarModel::empty(config_path, style_path);
    model.bar = bar;
    model.primary_keys = model.bar.keys().cloned().collect();
    model.style_css = wb.style_css.clone();
    let msg = waybar::save(&model)?;
    Ok(msg)
}

fn import_rofi(rf: &RofiExport) -> Result<String, BundleError> {
    let mut parts = Vec::new();
    if !rf.config_rasi.trim().is_empty() || !rf.theme_rasi.trim().is_empty() {
        parts.push(rofi::import_snapshot(&rf.config_rasi, &rf.theme_rasi)?);
    }
    if let Some(apps) = &rf.apps {
        let msg = rofi_apps::apply(apps)?;
        parts.push(msg);
    }
    if parts.is_empty() {
        return Err(BundleError::Message("empty rofi section".into()));
    }
    Ok(parts.join(" · "))
}

fn import_via(via_ex: &ViaExport) -> Result<String, BundleError> {
    let mut n_defs = 0usize;
    let mut n_presets = 0usize;

    if !via_ex.definitions.is_empty() {
        let dir = via::ensure_definitions().map_err(|e| BundleError::Message(e.to_string()))?;
        for def in &via_ex.definitions {
            let name = Path::new(&def.filename)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("imported.json");
            if !name.ends_with(".json") {
                continue;
            }
            let path = dir.join(name);
            let text = serde_json::to_string_pretty(&def.body)?;
            fs::write(&path, format!("{text}\n"))?;
            n_defs += 1;
        }
    }

    for anim in &via_ex.rgb_presets {
        via_studio::save_preset(anim).map_err(|e| BundleError::Message(e.to_string()))?;
        n_presets += 1;
    }

    Ok(format!(
        "via: {n_defs} definition(s), {n_presets} RGB preset(s)"
    ))
}

fn write_portal_conf(contents: &str) -> Result<String, BundleError> {
    let path = portal_conf_path()
        .ok_or_else(|| BundleError::Message("no config dir for portal conf".into()))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, contents)?;
    Ok(format!("portal → {}", path.display()))
}

pub fn export_to_path(
    collection: &BindCollection,
    path: &Path,
) -> Result<(), BundleError> {
    let bundle = export_all(collection)?;
    let text = to_pretty_json(&bundle)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)?;
    Ok(())
}

pub fn import_from_path(path: &Path, opts: ImportOptions) -> Result<ImportReport, BundleError> {
    let text = fs::read_to_string(path)?;
    let bundle = from_json_str(&text)?;
    import_all(&bundle, opts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_wrong_format() {
        let raw = r#"{"format":"nope","version":1}"#;
        let err = from_json_str(raw).unwrap_err();
        assert!(err.to_string().contains("unsupported format"));
    }

    #[test]
    fn rejects_bad_version() {
        let raw = r#"{"format":"hyprbinds-export","version":99}"#;
        let err = from_json_str(raw).unwrap_err();
        assert!(err.to_string().contains("unsupported export version"));
    }

    #[test]
    fn round_trips_minimal_envelope() {
        let bundle = HyprbindsExport {
            format: FORMAT_ID.into(),
            version: FORMAT_VERSION,
            exported_at: "unix:0".into(),
            hyprland: None,
            waybar: Some(WaybarExport {
                config: json!({"layer": "top"}),
                style_css: "window#waybar { }".into(),
            }),
            rofi: Some(RofiExport {
                config_rasi: "configuration { }\n".into(),
                theme_rasi: "* { background: #111; }\n".into(),
                apps: None,
            }),
            app: Some(AppExport {
                ui: UiPrefs {
                    dark_mode: true,
                    developer_mode: false,
                },
                wallpaper: WallpaperPrefs::default(),
            }),
            via: None,
            system: None,
        };
        let text = to_pretty_json(&bundle).unwrap();
        let parsed = from_json_str(&text).unwrap();
        assert_eq!(parsed.format, FORMAT_ID);
        assert_eq!(parsed.version, 1);
        assert!(parsed.waybar.is_some());
        assert!(parsed.rofi.is_some());
        assert!(parsed.app.is_some());
    }
}
