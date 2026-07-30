//! In-memory model for Rofi Studio (config.rasi + theme tokens).

use crate::rofi_theme::ThemeTokens;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Built-in mode toggles shown in the Studio UI.
pub const KNOWN_MODES: &[&str] = &[
    "drun",
    "run",
    "window",
    "ssh",
    "combi",
    "filebrowser",
    "keys",
];

pub const LOCATION_LABELS: &[&str] = &[
    "Center",
    "North-West",
    "North",
    "North-East",
    "East",
    "South-East",
    "South",
    "South-West",
    "West",
];

pub const MATCHING_METHODS: &[&str] = &["normal", "regex", "glob", "fuzzy", "prefix"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RofiModel {
    pub config_path: PathBuf,
    pub theme_path: PathBuf,
    /// Enabled modes, e.g. `["drun", "run", "window"]`.
    pub modes: Vec<String>,
    pub font: String,
    pub show_icons: bool,
    /// 0..=8 — see [`LOCATION_LABELS`].
    pub location: u8,
    #[serde(default)]
    pub xoffset: i32,
    #[serde(default)]
    pub yoffset: i32,
    pub terminal: String,
    pub matching: String,
    pub case_sensitive: bool,
    pub cycle: bool,
    pub sidebar_mode: bool,
    pub hover_select: bool,
    pub disable_history: bool,
    pub tokens: ThemeTokens,
    #[serde(default, skip_serializing)]
    pub notes: Vec<String>,
}

impl RofiModel {
    pub fn empty(config_path: PathBuf, theme_path: PathBuf) -> Self {
        Self {
            config_path,
            theme_path,
            modes: vec!["drun".into(), "run".into(), "window".into()],
            font: "JetBrainsMono Nerd Font 12".into(),
            show_icons: true,
            location: 0,
            xoffset: 0,
            yoffset: 0,
            terminal: "kitty".into(),
            matching: "fuzzy".into(),
            case_sensitive: false,
            cycle: true,
            sidebar_mode: false,
            hover_select: false,
            disable_history: false,
            tokens: ThemeTokens::defaults(),
            notes: Vec::new(),
        }
    }

    pub fn modes_csv(&self) -> String {
        self.modes.join(",")
    }

    pub fn set_mode_enabled(&mut self, mode: &str, enabled: bool) {
        if enabled {
            if !self.modes.iter().any(|m| m == mode) {
                // Keep a stable order matching KNOWN_MODES, then unknowns.
                let mut next: Vec<String> = KNOWN_MODES
                    .iter()
                    .filter(|m| self.modes.iter().any(|x| x == **m) || **m == mode)
                    .map(|m| (*m).to_string())
                    .collect();
                for m in &self.modes {
                    if !KNOWN_MODES.contains(&m.as_str()) && !next.iter().any(|x| x == m) {
                        next.push(m.clone());
                    }
                }
                if !next.iter().any(|m| m == mode) {
                    next.push(mode.to_string());
                }
                self.modes = next;
            }
        } else {
            self.modes.retain(|m| m != mode);
        }
    }

    pub fn location_label(&self) -> &'static str {
        LOCATION_LABELS
            .get(self.location as usize)
            .copied()
            .unwrap_or("Center")
    }
}
