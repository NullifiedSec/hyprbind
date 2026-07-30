//! Environment variables from `hl.env("KEY", "VALUE")`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub source_file: String,
    #[serde(default)]
    pub source_line: u32,
}

impl EnvVar {
    pub fn matches(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        let q = query.to_lowercase();
        self.name.to_lowercase().contains(&q) || self.value.to_lowercase().contains(&q)
    }
}

/// Named keybind submap from `hl.define_submap(...)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Submap {
    pub name: String,
    #[serde(default)]
    pub reset: String,
    #[serde(default)]
    pub source_file: String,
    #[serde(default)]
    pub source_line: u32,
    #[serde(default)]
    pub bind_count: usize,
    #[serde(default)]
    pub id: usize,
}

impl Submap {
    pub fn matches(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        let q = query.to_lowercase();
        self.name.to_lowercase().contains(&q) || self.reset.to_lowercase().contains(&q)
    }

    pub fn label(&self) -> String {
        if self.bind_count == 1 {
            format!("{} · 1 bind", self.name)
        } else {
            format!("{} · {} binds", self.name, self.bind_count)
        }
    }
}
