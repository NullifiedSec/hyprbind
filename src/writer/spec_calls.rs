//! Generic CRUD for `SpecItem`-shaped calls (`hl.monitor`, `hl.device`,
//! `hl.gesture`, `hl.animation`, `hl.curve`, `hl.workspace_rule`).
//!
//! The five thin wrappers (`add_monitor`, `save_monitor`, …) each just call
//! `add_spec_call` / `save_spec_call` / `delete_spec_call` with the right
//! function name and managed-section markers.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

use crate::spec::{self, SpecItem};
use crate::writer::error::{ok_result, write_config_atomic, WriteError, WriteMode, WriteResult};
use crate::writer::locate::{find_call_span, statement_cut_range};
use crate::writer::section::{
    insert_into_managed_section, MANAGED_ANIMS_BEGIN, MANAGED_ANIMS_END, MANAGED_CURVES_BEGIN,
    MANAGED_CURVES_END, MANAGED_DEVICES_BEGIN, MANAGED_DEVICES_END, MANAGED_GESTURES_BEGIN,
    MANAGED_GESTURES_END, MANAGED_MONITORS_BEGIN, MANAGED_MONITORS_END, MANAGED_WS_RULES_BEGIN,
    MANAGED_WS_RULES_END,
};

pub fn add_spec_call(
    config_path: &Path,
    begin: &str,
    end: &str,
    fn_name: &str,
    fields: &BTreeMap<String, Value>,
    name: Option<&str>,
) -> Result<WriteResult, WriteError> {
    if fields.is_empty() {
        return Err(WriteError::Invalid("no fields to write".into()));
    }
    if !config_path.is_file() {
        return Err(WriteError::Missing(config_path.display().to_string()));
    }
    let source = std::fs::read_to_string(config_path)?;
    let block = spec::format_call(fn_name, fields, name);
    let updated = insert_into_managed_section(&source, begin, end, &block)?;
    write_config_atomic(config_path, &updated)?;
    Ok(ok_result(
        config_path.display().to_string(),
        WriteMode::Appended,
    ))
}

pub fn save_spec_call(
    item: &SpecItem,
    fn_name: &str,
    fields: &BTreeMap<String, Value>,
    name: Option<&str>,
) -> Result<WriteResult, WriteError> {
    if fields.is_empty() {
        return Err(WriteError::Invalid("no fields to write".into()));
    }
    if item.source_file.is_empty() || item.source_line == 0 {
        return Err(WriteError::Invalid("item has no source location".into()));
    }
    let path = Path::new(&item.source_file);
    if !path.is_file() {
        return Err(WriteError::Missing(item.source_file.clone()));
    }
    let source = std::fs::read_to_string(path)?;
    let (start, end) = find_call_span(&source, item.source_line, fn_name)?;
    let replacement = spec::format_call(fn_name, fields, name);
    let mut updated = String::with_capacity(source.len() + replacement.len());
    updated.push_str(&source[..start]);
    updated.push_str(&replacement);
    updated.push_str(&source[end..]);
    write_config_atomic(path, &updated)?;
    Ok(ok_result(path.display().to_string(), WriteMode::InPlace))
}

pub fn delete_spec_call(item: &SpecItem, fn_name: &str) -> Result<WriteResult, WriteError> {
    if item.source_file.is_empty() || item.source_line == 0 {
        return Err(WriteError::Invalid("item has no source location".into()));
    }
    let path = Path::new(&item.source_file);
    if !path.is_file() {
        return Err(WriteError::Missing(item.source_file.clone()));
    }
    let source = std::fs::read_to_string(path)?;
    let (start, end) = find_call_span(&source, item.source_line, fn_name)?;
    let (cut_start, cut_end) = statement_cut_range(&source, start, end);
    let mut updated = String::with_capacity(source.len());
    updated.push_str(&source[..cut_start]);
    updated.push_str(&source[cut_end..]);
    write_config_atomic(path, &updated)?;
    Ok(ok_result(path.display().to_string(), WriteMode::InPlace))
}

pub fn add_monitor(
    config_path: &Path,
    fields: &BTreeMap<String, Value>,
) -> Result<WriteResult, WriteError> {
    add_spec_call(
        config_path,
        MANAGED_MONITORS_BEGIN,
        MANAGED_MONITORS_END,
        "hl.monitor",
        fields,
        None,
    )
}

pub fn save_monitor(
    item: &SpecItem,
    fields: &BTreeMap<String, Value>,
) -> Result<WriteResult, WriteError> {
    save_spec_call(item, "hl.monitor", fields, None)
}

pub fn delete_monitor(item: &SpecItem) -> Result<WriteResult, WriteError> {
    delete_spec_call(item, "hl.monitor")
}

pub fn add_device(
    config_path: &Path,
    fields: &BTreeMap<String, Value>,
) -> Result<WriteResult, WriteError> {
    add_spec_call(
        config_path,
        MANAGED_DEVICES_BEGIN,
        MANAGED_DEVICES_END,
        "hl.device",
        fields,
        None,
    )
}

pub fn save_device(
    item: &SpecItem,
    fields: &BTreeMap<String, Value>,
) -> Result<WriteResult, WriteError> {
    save_spec_call(item, "hl.device", fields, None)
}

pub fn delete_device(item: &SpecItem) -> Result<WriteResult, WriteError> {
    delete_spec_call(item, "hl.device")
}

pub fn add_gesture(
    config_path: &Path,
    fields: &BTreeMap<String, Value>,
) -> Result<WriteResult, WriteError> {
    add_spec_call(
        config_path,
        MANAGED_GESTURES_BEGIN,
        MANAGED_GESTURES_END,
        "hl.gesture",
        fields,
        None,
    )
}

pub fn save_gesture(
    item: &SpecItem,
    fields: &BTreeMap<String, Value>,
) -> Result<WriteResult, WriteError> {
    save_spec_call(item, "hl.gesture", fields, None)
}

pub fn delete_gesture(item: &SpecItem) -> Result<WriteResult, WriteError> {
    delete_spec_call(item, "hl.gesture")
}

pub fn add_workspace_rule(
    config_path: &Path,
    fields: &BTreeMap<String, Value>,
) -> Result<WriteResult, WriteError> {
    if !fields.contains_key("workspace") {
        return Err(WriteError::Invalid(
            "workspace rule needs a workspace selector".into(),
        ));
    }
    add_spec_call(
        config_path,
        MANAGED_WS_RULES_BEGIN,
        MANAGED_WS_RULES_END,
        "hl.workspace_rule",
        fields,
        None,
    )
}

pub fn save_workspace_rule(
    item: &SpecItem,
    fields: &BTreeMap<String, Value>,
) -> Result<WriteResult, WriteError> {
    if !fields.contains_key("workspace") {
        return Err(WriteError::Invalid(
            "workspace rule needs a workspace selector".into(),
        ));
    }
    save_spec_call(item, "hl.workspace_rule", fields, None)
}

pub fn delete_workspace_rule(item: &SpecItem) -> Result<WriteResult, WriteError> {
    delete_spec_call(item, "hl.workspace_rule")
}

pub fn add_animation(
    config_path: &Path,
    fields: &BTreeMap<String, Value>,
) -> Result<WriteResult, WriteError> {
    add_spec_call(
        config_path,
        MANAGED_ANIMS_BEGIN,
        MANAGED_ANIMS_END,
        "hl.animation",
        fields,
        None,
    )
}

pub fn save_animation(
    item: &SpecItem,
    fields: &BTreeMap<String, Value>,
) -> Result<WriteResult, WriteError> {
    save_spec_call(item, "hl.animation", fields, None)
}

pub fn delete_animation(item: &SpecItem) -> Result<WriteResult, WriteError> {
    delete_spec_call(item, "hl.animation")
}

/// Scale the `speed` field of every supplied animation leaf by `duration_factor`
/// in one pass per source file. `base_speeds` gives the reference speed for each
/// item (matched by index) so repeated scaling stays consistent. Springs / leaves
/// without a numeric speed are skipped. Returns the number of leaves rewritten.
pub fn scale_animation_speeds(
    items: &[SpecItem],
    base_speeds: &[f64],
    duration_factor: f64,
) -> Result<(usize, Vec<String>), WriteError> {
    use std::collections::BTreeMap as StdBTreeMap;

    // Bucket edits per source file so each file is read/written once.
    let mut per_file: StdBTreeMap<String, Vec<(usize, usize, String)>> = StdBTreeMap::new();

    for (item, base) in items.iter().zip(base_speeds.iter()) {
        if item.source_file.is_empty() || item.source_line == 0 {
            continue;
        }
        // Only scale bezier leaves that carry a numeric speed.
        if item.get_f64("speed").is_none() {
            continue;
        }
        let scaled = (base * duration_factor).round();
        let new_speed = if scaled < 1.0 { 1 } else { scaled as i64 };

        let mut fields: BTreeMap<String, Value> = item.fields.clone();
        fields.insert("speed".into(), Value::from(new_speed));

        let path = Path::new(&item.source_file);
        if !path.is_file() {
            return Err(WriteError::Missing(item.source_file.clone()));
        }
        let source = std::fs::read_to_string(path)?;
        let (start, end) = find_call_span(&source, item.source_line, "hl.animation")?;
        let replacement = spec::format_call("hl.animation", &fields, None);
        per_file
            .entry(item.source_file.clone())
            .or_default()
            .push((start, end, replacement));
    }

    let mut count = 0usize;
    let mut files = Vec::new();
    for (file, mut edits) in per_file {
        let path = Path::new(&file);
        let source = std::fs::read_to_string(path)?;
        // Apply from the end backwards so earlier byte offsets stay valid.
        edits.sort_by(|a, b| b.0.cmp(&a.0));
        let mut updated = source;
        let mut last_start = usize::MAX;
        for (start, end, replacement) in edits {
            if end > last_start {
                // Overlapping span (ambiguous match) — skip rather than corrupt.
                continue;
            }
            updated.replace_range(start..end, &replacement);
            last_start = start;
            count += 1;
        }
        write_config_atomic(path, &updated)?;
        files.push(file);
    }

    Ok((count, files))
}

pub fn add_curve(
    config_path: &Path,
    name: &str,
    fields: &BTreeMap<String, Value>,
) -> Result<WriteResult, WriteError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(WriteError::Invalid("curve name required".into()));
    }
    if !config_path.is_file() {
        return Err(WriteError::Missing(config_path.display().to_string()));
    }
    let source = std::fs::read_to_string(config_path)?;
    let block = spec::format_curve_call(name, fields);
    let updated = insert_into_managed_section(&source, MANAGED_CURVES_BEGIN, MANAGED_CURVES_END, &block)?;
    write_config_atomic(config_path, &updated)?;
    Ok(ok_result(
        config_path.display().to_string(),
        WriteMode::Appended,
    ))
}

pub fn save_curve(
    item: &SpecItem,
    name: &str,
    fields: &BTreeMap<String, Value>,
) -> Result<WriteResult, WriteError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(WriteError::Invalid("curve name required".into()));
    }
    if item.source_file.is_empty() || item.source_line == 0 {
        return Err(WriteError::Invalid("curve has no source location".into()));
    }
    let path = Path::new(&item.source_file);
    if !path.is_file() {
        return Err(WriteError::Missing(item.source_file.clone()));
    }
    let source = std::fs::read_to_string(path)?;
    let (start, end) = find_call_span(&source, item.source_line, "hl.curve")?;
    let replacement = spec::format_curve_call(name, fields);
    let mut updated = String::with_capacity(source.len() + replacement.len());
    updated.push_str(&source[..start]);
    updated.push_str(&replacement);
    updated.push_str(&source[end..]);
    write_config_atomic(path, &updated)?;
    Ok(ok_result(path.display().to_string(), WriteMode::InPlace))
}

pub fn delete_curve(item: &SpecItem) -> Result<WriteResult, WriteError> {
    delete_spec_call(item, "hl.curve")
}
