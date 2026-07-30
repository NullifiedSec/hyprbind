use crate::env::{EnvVar, Submap};
use crate::spec::{self, SpecItem};
use crate::startup::{self, StartupEntry};
use crate::variables::ConfigVariable;
use crate::window_rules::{self, WindowRule};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindCollection {
    pub config_path: PathBuf,
    #[serde(default)]
    #[allow(dead_code)]
    pub config_dir: PathBuf,
    pub binds: Vec<Keybind>,
    #[serde(default)]
    pub window_rules: Vec<WindowRule>,
    #[serde(default)]
    pub workspace_rules: Vec<SpecItem>,
    #[serde(default)]
    pub layer_rules: Vec<WindowRule>,
    #[serde(default)]
    pub monitors: Vec<SpecItem>,
    #[serde(default)]
    pub devices: Vec<SpecItem>,
    #[serde(default)]
    pub gestures: Vec<SpecItem>,
    #[serde(default)]
    pub curves: Vec<SpecItem>,
    #[serde(default)]
    pub animations: Vec<SpecItem>,
    #[serde(default)]
    pub env: Vec<EnvVar>,
    #[serde(default)]
    pub submaps: Vec<Submap>,
    #[serde(default)]
    pub startup: Vec<StartupEntry>,
    #[serde(default)]
    pub config_merged: Value,
    #[serde(default)]
    pub variables: Vec<ConfigVariable>,
    /// All Lua files loaded while collecting (main + require() modules).
    #[serde(default)]
    pub files: Vec<PathBuf>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybind {
    pub keys: String,
    pub action: String,
    pub flags: Vec<String>,
    pub description: String,
    pub submap: String,
    pub source_file: String,
    pub source_line: u32,
    /// Display name: description if set, otherwise untitled-N.
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub id: usize,
}

impl BindCollection {
    pub fn finalize(&mut self) {
        assign_names(&mut self.binds);
        window_rules::assign_rule_names(&mut self.window_rules);
        window_rules::assign_rule_names(&mut self.layer_rules);
        startup::assign_ids(&mut self.startup);
        spec::assign_spec_names(&mut self.workspace_rules, "workspace");
        spec::assign_spec_names(&mut self.monitors, "output");
        spec::assign_spec_names(&mut self.devices, "name");
        spec::assign_spec_names(&mut self.gestures, "action");
        spec::assign_spec_names(&mut self.curves, "name");
        // Curves already have name on SpecItem.name
        for (idx, c) in self.curves.iter_mut().enumerate() {
            c.id = idx;
            if c.display_name.is_empty() {
                c.display_name = if c.name.is_empty() {
                    format!("curve-{idx}")
                } else {
                    c.name.clone()
                };
            }
        }
        spec::assign_spec_names(&mut self.animations, "leaf");
        self.synthesize_submaps_from_binds();
        let counts: Vec<(String, usize)> = {
            let mut map = std::collections::BTreeMap::<String, usize>::new();
            for b in &self.binds {
                if !b.submap.is_empty() {
                    *map.entry(b.submap.clone()).or_default() += 1;
                }
            }
            map.into_iter().collect()
        };
        for (idx, s) in self.submaps.iter_mut().enumerate() {
            s.id = idx;
            s.bind_count = counts
                .iter()
                .find(|(n, _)| n == &s.name)
                .map(|(_, c)| *c)
                .unwrap_or(0);
        }
    }

    /// Ensure every bind.submap appears as a Submap entry (even if define_submap failed to record).
    fn synthesize_submaps_from_binds(&mut self) {
        let mut known: BTreeSet<String> = self.submaps.iter().map(|s| s.name.clone()).collect();
        for bind in &self.binds {
            let name = bind.submap.trim();
            if name.is_empty() || known.contains(name) {
                continue;
            }
            known.insert(name.to_string());
            self.submaps.push(Submap {
                name: name.to_string(),
                reset: String::new(),
                source_file: bind.source_file.clone(),
                source_line: bind.source_line,
                bind_count: 0,
                id: 0,
            });
        }
        self.submaps.sort_by(|a, b| a.name.cmp(&b.name));
    }

    pub fn submap_names(&self) -> Vec<String> {
        self.submaps.iter().map(|s| s.name.clone()).collect()
    }

    pub fn env_by_name(&self, name: &str) -> Option<&EnvVar> {
        self.env.iter().find(|e| e.name == name)
    }

    pub fn startup_by_id(&self, id: usize) -> Option<&StartupEntry> {
        self.startup.iter().find(|e| e.id == id)
    }

    pub fn submap_by_id(&self, id: usize) -> Option<&Submap> {
        self.submaps.iter().find(|s| s.id == id)
    }

    pub fn submap_by_name(&self, name: &str) -> Option<&Submap> {
        self.submaps.iter().find(|s| s.name == name)
    }

    pub fn bind_by_id(&self, id: usize) -> Option<&Keybind> {
        self.binds.iter().find(|b| b.id == id)
    }

    pub fn rule_by_id(&self, id: usize) -> Option<&WindowRule> {
        self.window_rules.iter().find(|r| r.id == id)
    }

    pub fn layer_rule_by_id(&self, id: usize) -> Option<&WindowRule> {
        self.layer_rules.iter().find(|r| r.id == id)
    }

    pub fn workspace_rule_by_id(&self, id: usize) -> Option<&SpecItem> {
        self.workspace_rules.iter().find(|r| r.id == id)
    }

    pub fn monitor_by_id(&self, id: usize) -> Option<&SpecItem> {
        self.monitors.iter().find(|r| r.id == id)
    }

    pub fn device_by_id(&self, id: usize) -> Option<&SpecItem> {
        self.devices.iter().find(|r| r.id == id)
    }

    pub fn gesture_by_id(&self, id: usize) -> Option<&SpecItem> {
        self.gestures.iter().find(|r| r.id == id)
    }

    pub fn curve_by_id(&self, id: usize) -> Option<&SpecItem> {
        self.curves.iter().find(|r| r.id == id)
    }

    pub fn animation_by_id(&self, id: usize) -> Option<&SpecItem> {
        self.animations.iter().find(|r| r.id == id)
    }

    pub fn source_share_count(&self, bind: &Keybind) -> usize {
        self.binds
            .iter()
            .filter(|b| b.source_file == bind.source_file && b.source_line == bind.source_line)
            .count()
    }

    /// Files that may contain references to config variables / binds.
    pub fn related_files(&self) -> Vec<PathBuf> {
        let mut files = self.files.clone();
        if !files.iter().any(|f| f == &self.config_path) {
            files.push(self.config_path.clone());
        }
        for var in &self.variables {
            let p = PathBuf::from(&var.source_file);
            if !p.as_os_str().is_empty() && !files.contains(&p) {
                files.push(p);
            }
        }
        for bind in &self.binds {
            let p = PathBuf::from(&bind.source_file);
            if !p.as_os_str().is_empty() && !files.contains(&p) {
                files.push(p);
            }
        }
        for rule in &self.window_rules {
            let p = PathBuf::from(&rule.source_file);
            if !p.as_os_str().is_empty() && !files.contains(&p) {
                files.push(p);
            }
        }
        files.sort();
        files.dedup();
        files
    }

    pub fn variable_names(&self) -> Vec<String> {
        self.variables.iter().map(|v| v.name.clone()).collect()
    }
}

impl Keybind {
    pub fn matches(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        let q = query.to_lowercase();
        self.name.to_lowercase().contains(&q)
            || self.keys.to_lowercase().contains(&q)
            || self.action.to_lowercase().contains(&q)
            || self.description.to_lowercase().contains(&q)
            || self.submap.to_lowercase().contains(&q)
            || self.flags.iter().any(|f| f.to_lowercase().contains(&q))
            || self.source_file.to_lowercase().contains(&q)
    }

    pub fn flags_label(&self) -> String {
        if self.flags.is_empty() {
            String::new()
        } else {
            self.flags.join(", ")
        }
    }

    pub fn source_label(&self) -> String {
        if self.source_file.is_empty() {
            String::new()
        } else {
            let name = std::path::Path::new(&self.source_file)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(self.source_file.as_str());
            format!("{name}:{}", self.source_line)
        }
    }

    pub fn submap_label(&self) -> String {
        if self.submap.is_empty() {
            "global".to_string()
        } else {
            self.submap.clone()
        }
    }
}

pub fn assign_names(binds: &mut [Keybind]) {
    let mut untitled = 0usize;
    // Auto-generated names: (bind index, current exec-arg depth)
    let mut auto: Vec<(usize, usize)> = Vec::new();

    for (idx, bind) in binds.iter_mut().enumerate() {
        bind.id = idx;
        let desc = bind.description.trim();
        if !desc.is_empty() {
            bind.name = desc.to_string();
        } else if let Some(generated) = crate::dispatchers::describe_action(&bind.action) {
            bind.name = generated;
            auto.push((idx, 1));
        } else {
            untitled += 1;
            bind.name = format!("untitled-{untitled}");
        }
    }

    // When several binds share a generated label (e.g. all "Run ags…"), include
    // more command arguments until each label is unique (or we can't deepen).
    loop {
        let mut counts = std::collections::HashMap::<String, usize>::new();
        for &(idx, _) in &auto {
            *counts.entry(binds[idx].name.clone()).or_default() += 1;
        }
        if !counts.values().any(|&c| c > 1) {
            break;
        }

        let mut progressed = false;
        for entry in &mut auto {
            let (idx, depth) = *entry;
            if counts.get(&binds[idx].name).copied().unwrap_or(0) <= 1 {
                continue;
            }
            let next = depth + 1;
            let Some(deeper) =
                crate::dispatchers::describe_action_at_depth(&binds[idx].action, next)
            else {
                continue;
            };
            if deeper != binds[idx].name {
                binds[idx].name = deeper;
                entry.1 = next;
                progressed = true;
            }
        }
        if !progressed {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bind(action: &str) -> Keybind {
        Keybind {
            keys: String::new(),
            action: action.into(),
            flags: vec![],
            description: String::new(),
            submap: String::new(),
            source_file: String::new(),
            source_line: 0,
            name: String::new(),
            id: 0,
        }
    }

    #[test]
    fn deepens_duplicate_exec_descriptions() {
        let mut binds = vec![
            bind("hl.dsp.exec_cmd(\"ags -b\")"),
            bind("hl.dsp.exec_cmd(\"ags request toggle\")"),
            bind("hl.dsp.exec_cmd(\"ags -t media\")"),
            bind("hl.dsp.exec_cmd(\"kitty\")"),
        ];
        assign_names(&mut binds);
        assert_eq!(binds[0].name, "Run ags -b");
        assert_eq!(binds[1].name, "Run ags request…");
        assert_eq!(binds[2].name, "Run ags -t…");
        assert_eq!(binds[3].name, "Run kitty");
        // Names for ags binds should be unique among themselves.
        let ags: Vec<_> = binds.iter().take(3).map(|b| b.name.as_str()).collect();
        assert_eq!(ags.len(), ags.iter().collect::<std::collections::BTreeSet<_>>().len());
    }

    #[test]
    fn keeps_explicit_descriptions() {
        let mut binds = vec![
            {
                let mut b = bind("hl.dsp.exec_cmd(\"ags -b\")");
                b.description = "Open bar".into();
                b
            },
            bind("hl.dsp.exec_cmd(\"ags -t media\")"),
        ];
        assign_names(&mut binds);
        assert_eq!(binds[0].name, "Open bar");
        assert_eq!(binds[1].name, "Run ags…");
    }
}
