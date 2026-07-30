//! Detect overlapping keybinds (same chord in the same submap).

use crate::bind::Keybind;
use crate::keys::normalize_keys_input;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct ConflictGroup {
    pub keys: String,
    pub submap: String,
    pub bind_ids: Vec<usize>,
    pub names: Vec<String>,
}

/// Groups binds that share a normalized key chord within the same submap.
pub fn find_conflicts(binds: &[Keybind]) -> Vec<ConflictGroup> {
    let mut map: BTreeMap<(String, String), Vec<&Keybind>> = BTreeMap::new();
    for bind in binds {
        let keys = normalize_conflict_keys(&bind.keys);
        if keys.is_empty() {
            continue;
        }
        let submap = if bind.submap.is_empty() {
            "global".to_string()
        } else {
            bind.submap.clone()
        };
        map.entry((keys, submap)).or_default().push(bind);
    }

    let mut out = Vec::new();
    for ((keys, submap), group) in map {
        if group.len() < 2 {
            continue;
        }
        // Same source line shared binds (materialized duplicates) still count as conflict
        // if they are distinct ids — user should know.
        out.push(ConflictGroup {
            keys,
            submap,
            bind_ids: group.iter().map(|b| b.id).collect(),
            names: group
                .iter()
                .map(|b| {
                    if b.name.is_empty() {
                        b.action.clone()
                    } else {
                        b.name.clone()
                    }
                })
                .collect(),
        });
    }
    out
}

pub fn conflicting_ids(binds: &[Keybind]) -> std::collections::BTreeSet<usize> {
    let mut set = std::collections::BTreeSet::new();
    for g in find_conflicts(binds) {
        set.extend(g.bind_ids);
    }
    set
}

pub fn normalize_conflict_keys(raw: &str) -> String {
    let normalized = normalize_keys_input(raw);
    // Sort modifiers so SUPER+SHIFT+A and SHIFT+SUPER+A match.
    let mut parts: Vec<&str> = normalized.split(" + ").collect();
    if parts.is_empty() {
        return String::new();
    }
    let key = parts.pop().unwrap_or("").to_string();
    let mut mods: Vec<String> = parts
        .into_iter()
        .map(|p| p.to_ascii_uppercase())
        .collect();
    mods.sort();
    mods.dedup();
    if mods.is_empty() {
        key
    } else {
        format!("{} + {}", mods.join(" + "), key)
    }
}

pub fn summary(groups: &[ConflictGroup]) -> String {
    if groups.is_empty() {
        "No bind conflicts".into()
    } else if groups.len() == 1 {
        format!("1 conflict group ({} binds)", groups[0].bind_ids.len())
    } else {
        let binds: usize = groups.iter().map(|g| g.bind_ids.len()).sum();
        format!("{} conflict groups · {} binds", groups.len(), binds)
    }
}
