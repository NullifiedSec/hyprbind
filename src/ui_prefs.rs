//! Persisted UI preferences (theme, etc.).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiPrefs {
    #[serde(default)]
    pub dark_mode: bool,
    /// Unlocks the Experimental sidebar section (Waybar, Wallpaper, VIA, Starship, Audio, Screenshare).
    #[serde(default)]
    pub developer_mode: bool,
}

impl Default for UiPrefs {
    fn default() -> Self {
        Self {
            dark_mode: true,
            developer_mode: false,
        }
    }
}

fn prefs_path() -> Option<PathBuf> {
    let mut dir = dirs::config_dir()?;
    dir.push("hyprbinds");
    Some(dir.join("ui.json"))
}

pub fn load() -> UiPrefs {
    let Some(path) = prefs_path() else {
        return UiPrefs::default();
    };
    match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => UiPrefs::default(),
    }
}

pub fn save(prefs: &UiPrefs) {
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

/// Load prefs, apply `update`, then save.
pub fn update(mutator: impl FnOnce(&mut UiPrefs)) {
    let mut prefs = load();
    mutator(&mut prefs);
    save(&prefs);
}

pub fn apply_dark_mode(enabled: bool) {
    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_application_prefer_dark_theme(enabled);
    }
}
