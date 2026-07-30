//! Autostart entries from `hl.exec_cmd` / `hl.on("hyprland.start", …)`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupEntry {
    pub command: String,
    /// `start` (exec-once), `reload` (every config reload), `shutdown`, or `event:…`.
    #[serde(default = "default_when")]
    pub when: String,
    #[serde(default)]
    pub workspace: String,
    #[serde(default)]
    pub source_file: String,
    #[serde(default)]
    pub source_line: u32,
    #[serde(default)]
    pub id: usize,
}

fn default_when() -> String {
    "start".into()
}

impl StartupEntry {
    pub fn matches(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        let q = query.to_lowercase();
        self.command.to_lowercase().contains(&q)
            || self.when.to_lowercase().contains(&q)
            || self.workspace.to_lowercase().contains(&q)
    }

    pub fn when_label(&self) -> &str {
        match self.when.as_str() {
            "start" => "on start",
            "reload" => "on reload",
            "shutdown" => "on shutdown",
            other => other,
        }
    }
}

pub fn assign_ids(entries: &mut [StartupEntry]) {
    for (idx, e) in entries.iter_mut().enumerate() {
        e.id = idx;
    }
}
