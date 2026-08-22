//! Window rule and layer rule CRUD: `hl.window_rule(...)` and `hl.layer_rule(...)`.
//!
//! Both reuse the same formatter (`window_rules::format_lua_rule`); layer
//! rules are emitted by `replacen("hl.window_rule", "hl.layer_rule", 1)`.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

use crate::window_rules::{self, WindowRule};
use crate::writer::error::{ok_result, write_config_atomic, WriteError, WriteMode, WriteResult};
use crate::writer::locate::{find_call_span, statement_cut_range};
use crate::writer::section::{
    insert_into_managed_section, MANAGED_LAYER_RULES_BEGIN, MANAGED_LAYER_RULES_END,
    MANAGED_RULES_BEGIN, MANAGED_RULES_END,
};

/// Append a new window rule into the managed section (existing config untouched).
pub fn add_window_rule(
    config_path: &Path,
    name: &str,
    match_props: &BTreeMap<String, Value>,
    effects: &BTreeMap<String, Value>,
) -> Result<WriteResult, WriteError> {
    if match_props.is_empty() {
        return Err(WriteError::Invalid(
            "window rule needs at least one match property".into(),
        ));
    }
    if effects.is_empty() {
        return Err(WriteError::Invalid(
            "window rule needs at least one effect".into(),
        ));
    }
    if !config_path.is_file() {
        return Err(WriteError::Missing(config_path.display().to_string()));
    }

    let source = std::fs::read_to_string(config_path)?;
    let block = window_rules::format_lua_rule(name, match_props, effects);
    let updated =
        insert_into_managed_section(&source, MANAGED_RULES_BEGIN, MANAGED_RULES_END, &block)?;
    write_config_atomic(config_path, &updated)?;

    Ok(ok_result(
        config_path.display().to_string(),
        WriteMode::Appended,
    ))
}

/// Update an existing window rule in place.
pub fn save_window_rule(
    rule: &WindowRule,
    name: &str,
    match_props: &BTreeMap<String, Value>,
    effects: &BTreeMap<String, Value>,
) -> Result<WriteResult, WriteError> {
    if match_props.is_empty() {
        return Err(WriteError::Invalid(
            "window rule needs at least one match property".into(),
        ));
    }
    if effects.is_empty() {
        return Err(WriteError::Invalid(
            "window rule needs at least one effect".into(),
        ));
    }
    if rule.source_file.is_empty() || rule.source_line == 0 {
        return Err(WriteError::Invalid(
            "window rule has no source location".into(),
        ));
    }

    let path = Path::new(&rule.source_file);
    if !path.is_file() {
        return Err(WriteError::Missing(rule.source_file.clone()));
    }

    let source = std::fs::read_to_string(path)?;
    let (start, end) = find_call_span(&source, rule.source_line, "hl.window_rule")?;
    let replacement = window_rules::format_lua_rule(name, match_props, effects);

    let mut updated = String::with_capacity(source.len() + replacement.len());
    updated.push_str(&source[..start]);
    updated.push_str(&replacement);
    updated.push_str(&source[end..]);
    write_config_atomic(path, &updated)?;

    Ok(ok_result(path.display().to_string(), WriteMode::InPlace))
}

pub fn delete_window_rule(rule: &WindowRule) -> Result<WriteResult, WriteError> {
    if rule.source_file.is_empty() || rule.source_line == 0 {
        return Err(WriteError::Invalid(
            "window rule has no source location".into(),
        ));
    }
    let path = Path::new(&rule.source_file);
    if !path.is_file() {
        return Err(WriteError::Missing(rule.source_file.clone()));
    }

    let source = std::fs::read_to_string(path)?;
    let (start, end) = find_call_span(&source, rule.source_line, "hl.window_rule")?;
    let (cut_start, cut_end) = statement_cut_range(&source, start, end);

    let mut updated = String::with_capacity(source.len());
    updated.push_str(&source[..cut_start]);
    updated.push_str(&source[cut_end..]);
    write_config_atomic(path, &updated)?;

    Ok(ok_result(path.display().to_string(), WriteMode::InPlace))
}

pub fn add_layer_rule(
    config_path: &Path,
    name: &str,
    match_props: &BTreeMap<String, Value>,
    effects: &BTreeMap<String, Value>,
) -> Result<WriteResult, WriteError> {
    if match_props.is_empty() {
        return Err(WriteError::Invalid(
            "layer rule needs at least one match property".into(),
        ));
    }
    if effects.is_empty() {
        return Err(WriteError::Invalid(
            "layer rule needs at least one effect".into(),
        ));
    }
    if !config_path.is_file() {
        return Err(WriteError::Missing(config_path.display().to_string()));
    }
    let source = std::fs::read_to_string(config_path)?;
    // Reuse window-rule formatter but rename the function.
    let block = window_rules::format_lua_rule(name, match_props, effects)
        .replacen("hl.window_rule", "hl.layer_rule", 1);
    let updated =
        insert_into_managed_section(&source, MANAGED_LAYER_RULES_BEGIN, MANAGED_LAYER_RULES_END, &block)?;
    write_config_atomic(config_path, &updated)?;
    Ok(ok_result(
        config_path.display().to_string(),
        WriteMode::Appended,
    ))
}

pub fn save_layer_rule(
    rule: &WindowRule,
    name: &str,
    match_props: &BTreeMap<String, Value>,
    effects: &BTreeMap<String, Value>,
) -> Result<WriteResult, WriteError> {
    if match_props.is_empty() || effects.is_empty() {
        return Err(WriteError::Invalid(
            "layer rule needs match and effects".into(),
        ));
    }
    if rule.source_file.is_empty() || rule.source_line == 0 {
        return Err(WriteError::Invalid(
            "layer rule has no source location".into(),
        ));
    }
    let path = Path::new(&rule.source_file);
    if !path.is_file() {
        return Err(WriteError::Missing(rule.source_file.clone()));
    }
    let source = std::fs::read_to_string(path)?;
    let (start, end) = find_call_span(&source, rule.source_line, "hl.layer_rule")?;
    let replacement = window_rules::format_lua_rule(name, match_props, effects)
        .replacen("hl.window_rule", "hl.layer_rule", 1);
    let mut updated = String::with_capacity(source.len() + replacement.len());
    updated.push_str(&source[..start]);
    updated.push_str(&replacement);
    updated.push_str(&source[end..]);
    write_config_atomic(path, &updated)?;
    Ok(ok_result(path.display().to_string(), WriteMode::InPlace))
}

pub fn delete_layer_rule(rule: &WindowRule) -> Result<WriteResult, WriteError> {
    if rule.source_file.is_empty() || rule.source_line == 0 {
        return Err(WriteError::Invalid(
            "layer rule has no source location".into(),
        ));
    }
    let path = Path::new(&rule.source_file);
    if !path.is_file() {
        return Err(WriteError::Missing(rule.source_file.clone()));
    }
    let source = std::fs::read_to_string(path)?;
    let (start, end) = find_call_span(&source, rule.source_line, "hl.layer_rule")?;
    let mut cut_end = end;
    if source.as_bytes().get(cut_end) == Some(&b'\n') {
        cut_end += 1;
    }
    let mut updated = String::with_capacity(source.len());
    updated.push_str(&source[..start]);
    updated.push_str(&source[cut_end..]);
    write_config_atomic(path, &updated)?;
    Ok(ok_result(path.display().to_string(), WriteMode::InPlace))
}
