//! Managed-section marker constants and helpers.
//!
//! All edits land inside `-- >>> hyprbinds:managed-<name>` / `-- <<< hyprbinds:managed-<name>`
//! blocks. The ensure/insert/replace primitives preserve any user-written Lua
//! outside the markers.

use crate::writer::error::{ok_result, write_config_atomic, WriteError, WriteResult};

pub const MANAGED_BINDS_BEGIN: &str = "-- >>> hyprbinds:managed-binds";
pub const MANAGED_BINDS_END: &str = "-- <<< hyprbinds:managed-binds";
pub const MANAGED_VARS_BEGIN: &str = "-- >>> hyprbinds:managed-variables";
pub const MANAGED_VARS_END: &str = "-- <<< hyprbinds:managed-variables";
pub const MANAGED_RULES_BEGIN: &str = "-- >>> hyprbinds:managed-window-rules";
pub const MANAGED_RULES_END: &str = "-- <<< hyprbinds:managed-window-rules";
pub const MANAGED_CONFIG_BEGIN: &str = "-- >>> hyprbinds:managed-config";
pub const MANAGED_CONFIG_END: &str = "-- <<< hyprbinds:managed-config";
pub const MANAGED_MONITORS_BEGIN: &str = "-- >>> hyprbinds:managed-monitors";
pub const MANAGED_MONITORS_END: &str = "-- <<< hyprbinds:managed-monitors";
pub const MANAGED_DEVICES_BEGIN: &str = "-- >>> hyprbinds:managed-devices";
pub const MANAGED_DEVICES_END: &str = "-- <<< hyprbinds:managed-devices";
pub const MANAGED_GESTURES_BEGIN: &str = "-- >>> hyprbinds:managed-gestures";
pub const MANAGED_GESTURES_END: &str = "-- <<< hyprbinds:managed-gestures";
pub const MANAGED_CURVES_BEGIN: &str = "-- >>> hyprbinds:managed-curves";
pub const MANAGED_CURVES_END: &str = "-- <<< hyprbinds:managed-curves";
pub const MANAGED_ANIMS_BEGIN: &str = "-- >>> hyprbinds:managed-animations";
pub const MANAGED_ANIMS_END: &str = "-- <<< hyprbinds:managed-animations";
pub const MANAGED_WS_RULES_BEGIN: &str = "-- >>> hyprbinds:managed-workspace-rules";
pub const MANAGED_WS_RULES_END: &str = "-- <<< hyprbinds:managed-workspace-rules";
pub const MANAGED_LAYER_RULES_BEGIN: &str = "-- >>> hyprbinds:managed-layer-rules";
pub const MANAGED_LAYER_RULES_END: &str = "-- <<< hyprbinds:managed-layer-rules";
pub const MANAGED_ENV_BEGIN: &str = "-- >>> hyprbinds:managed-env";
pub const MANAGED_ENV_END: &str = "-- <<< hyprbinds:managed-env";
pub const MANAGED_SUBMAPS_BEGIN: &str = "-- >>> hyprbinds:managed-submaps";
pub const MANAGED_SUBMAPS_END: &str = "-- <<< hyprbinds:managed-submaps";
pub const MANAGED_STARTUP_BEGIN: &str = "-- >>> hyprbinds:managed-startup";
pub const MANAGED_STARTUP_END: &str = "-- <<< hyprbinds:managed-startup";

/// Ensure a managed section exists (creates begin+end markers if missing).
pub(crate) fn ensure_managed_section(source: &str, begin: &str, end: &str) -> String {
    if source.contains(begin) && source.contains(end) {
        return source.to_string();
    }

    let mut out = source.to_string();
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push('\n');
    out.push_str(begin);
    out.push('\n');
    out.push_str(end);
    out.push('\n');
    out
}

/// Join managed-section entries with a blank line for hand-edit readability.
pub(crate) fn join_section_entries(entries: &[String]) -> String {
    entries.join("\n\n")
}

/// Insert `line` just before the managed section end marker.
pub(crate) fn insert_into_managed_section(
    source: &str,
    begin: &str,
    end: &str,
    line: &str,
) -> Result<String, WriteError> {
    let mut source = ensure_managed_section(source, begin, end);
    let Some(begin_idx) = source.find(begin) else {
        return Err(WriteError::Invalid(
            "managed section begin marker missing".into(),
        ));
    };
    let Some(end_idx) = source.find(end) else {
        return Err(WriteError::Invalid(
            "managed section end marker missing".into(),
        ));
    };

    let after_begin = begin_idx + begin.len();
    let has_content = end_idx > after_begin && !source[after_begin..end_idx].trim().is_empty();

    let mut insertion = String::new();
    // Keep a trailing newline before the end marker.
    if end_idx > 0 && !source[..end_idx].ends_with('\n') {
        insertion.push('\n');
    } else if has_content && !source[..end_idx].ends_with("\n\n") {
        // Separate from the previous entry with a blank line.
        insertion.push('\n');
    }
    insertion.push_str(line);
    if !line.ends_with('\n') {
        insertion.push('\n');
    }

    source.insert_str(end_idx, &insertion);
    Ok(source)
}

/// Ensure section exists, then replace everything between begin and end markers.
pub(crate) fn replace_managed_section_contents(
    source: &str,
    begin: &str,
    end: &str,
    contents: &str,
) -> Result<String, WriteError> {
    let source = ensure_managed_section(source, begin, end);
    let Some(begin_idx) = source.find(begin) else {
        return Err(WriteError::Invalid(
            "managed section begin marker missing".into(),
        ));
    };
    let after_begin = begin_idx + begin.len();
    let Some(rel_end) = source[after_begin..].find(end) else {
        return Err(WriteError::Invalid(
            "managed section end marker missing".into(),
        ));
    };
    let end_idx = after_begin + rel_end;

    let mut body = String::new();
    body.push('\n');
    body.push_str(contents.trim());
    body.push('\n');

    let mut out = String::with_capacity(source.len() + body.len());
    out.push_str(&source[..after_begin]);
    out.push_str(&body);
    out.push_str(&source[end_idx..]);
    Ok(out)
}

/// Append `line` into a managed section and write the file atomically.
pub(crate) fn append_into_section(
    path: &std::path::Path,
    begin: &str,
    end: &str,
    line: &str,
    mode: crate::writer::error::WriteMode,
) -> Result<WriteResult, WriteError> {
    let source = std::fs::read_to_string(path)?;
    let updated = insert_into_managed_section(&source, begin, end, line)?;
    write_config_atomic(path, &updated)?;
    Ok(ok_result(path.display().to_string(), mode))
}
