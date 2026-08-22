//! In-memory model for Starship Studio (`starship.toml` + shell hooks).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShellKind {
    Bash,
    Zsh,
    Fish,
    Nu,
    Elvish,
    Xonsh,
    Tcsh,
    Ion,
}

impl ShellKind {
    pub const ALL: &'static [ShellKind] = &[
        Self::Bash,
        Self::Zsh,
        Self::Fish,
        Self::Nu,
        Self::Elvish,
        Self::Xonsh,
        Self::Tcsh,
        Self::Ion,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Fish => "fish",
            Self::Nu => "nu",
            Self::Elvish => "elvish",
            Self::Xonsh => "xonsh",
            Self::Tcsh => "tcsh",
            Self::Ion => "ion",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Bash => "Bash",
            Self::Zsh => "Zsh",
            Self::Fish => "Fish",
            Self::Nu => "Nushell",
            Self::Elvish => "Elvish",
            Self::Xonsh => "Xonsh",
            Self::Tcsh => "Tcsh",
            Self::Ion => "Ion",
        }
    }

    pub fn binary(self) -> &'static str {
        self.id()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellStatus {
    pub kind: ShellKind,
    pub installed: bool,
    pub rc_path: PathBuf,
    /// Starship init present (managed markers or a known init line).
    pub enabled: bool,
    /// Enabled via Hyprbinds-managed block.
    pub managed: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarshipModel {
    pub config_path: PathBuf,
    /// Full `starship.toml` text (source of truth for Apply).
    pub toml_text: String,
    pub selected_preset: Option<String>,
    #[serde(default, skip_serializing)]
    pub notes: Vec<String>,
}

impl StarshipModel {
    pub fn empty(config_path: PathBuf) -> Self {
        Self {
            config_path,
            toml_text: default_toml(),
            selected_preset: None,
            notes: Vec::new(),
        }
    }
}

pub fn default_toml() -> String {
    concat!(
        "# https://starship.rs/config/\n",
        "# Managed by Hyprbinds Starship Studio — Apply writes this file.\n",
        "\n",
        "\"$schema\" = 'https://starship.rs/config-schema.json'\n",
        "\n",
        "add_newline = true\n",
        "\n",
        "[character]\n",
        "success_symbol = \"[❯](bold green)\"\n",
        "error_symbol = \"[❯](bold red)\"\n",
    )
    .to_string()
}
