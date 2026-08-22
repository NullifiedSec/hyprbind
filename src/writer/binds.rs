//! Keybind CRUD: `hl.bind(...)` edits.
//!
//! Two write paths:
//! - **In-place** when the bind is the sole call on its source line.
//! - **Materialized** into the managed binds section (writing `hl.unbind` +
//!   a fresh `hl.bind`) when several binds share one Lua call (e.g. inside a
//!   loop).

use std::path::Path;

use crate::bind::Keybind;
use crate::keys::lua_string_literal;
use crate::writer::error::{ok_result, write_config_atomic, WriteError, WriteMode, WriteResult};
use crate::writer::locate::{find_bind_call_span, statement_cut_range};
use crate::writer::lua_text::{format_action, format_keys, format_opts};
use crate::writer::section::{
    append_into_section, insert_into_managed_section, MANAGED_BINDS_BEGIN, MANAGED_BINDS_END,
};

pub fn save_bind(
    bind: &Keybind,
    name: &str,
    keys: &str,
    action: &str,
    shared_source: bool,
) -> Result<WriteResult, WriteError> {
    let name = name.trim();
    let keys = keys.trim();
    let action = action.trim();

    if name.is_empty() {
        return Err(WriteError::Invalid("name cannot be empty".into()));
    }
    if keys.is_empty() {
        return Err(WriteError::Invalid("keys cannot be empty".into()));
    }
    if action.is_empty() {
        return Err(WriteError::Invalid("action cannot be empty".into()));
    }
    if bind.source_file.is_empty() || bind.source_line == 0 {
        return Err(WriteError::Invalid(
            "bind has no source location to write back to".into(),
        ));
    }
    if action.starts_with('<') {
        return Err(WriteError::Invalid(
            "this bind uses a Lua function action; replace it with an hl.dsp.* call or shell command before saving".into(),
        ));
    }

    let path = Path::new(&bind.source_file);
    if !path.is_file() {
        return Err(WriteError::Missing(bind.source_file.clone()));
    }

    if shared_source {
        materialize_override(path, bind, name, keys, action)
    } else {
        update_in_place(path, bind, name, keys, action)
    }
}

/// Remove a bind from config.
///
/// When `shared_source` is true (several binds share one Lua call, e.g. a loop),
/// write `hl.unbind(...)` into the managed section instead of deleting the shared call.
pub fn delete_bind(bind: &Keybind, shared_source: bool) -> Result<WriteResult, WriteError> {
    if bind.source_file.is_empty() || bind.source_line == 0 {
        return Err(WriteError::Invalid(
            "bind has no source location to write back to".into(),
        ));
    }
    let path = Path::new(&bind.source_file);
    if !path.is_file() {
        return Err(WriteError::Missing(bind.source_file.clone()));
    }

    if shared_source {
        materialize_unbind(path, bind)
    } else {
        delete_bind_in_place(path, bind)
    }
}

/// Append a brand-new bind into the managed section of `hyprland.lua`.
/// When `submap` is non-empty, the bind is placed inside a managed `hl.define_submap` block.
pub fn add_bind(
    config_path: &Path,
    name: &str,
    keys: &str,
    action: &str,
    submap: &str,
) -> Result<WriteResult, WriteError> {
    let name = name.trim();
    let keys = keys.trim();
    let action = action.trim();
    let submap = submap.trim();

    if name.is_empty() {
        return Err(WriteError::Invalid("name cannot be empty".into()));
    }
    if keys.is_empty() {
        return Err(WriteError::Invalid("keys cannot be empty".into()));
    }
    if action.is_empty() {
        return Err(WriteError::Invalid("action cannot be empty".into()));
    }
    if action.starts_with('<') {
        return Err(WriteError::Invalid(
            "provide an hl.dsp.* action or shell command".into(),
        ));
    }
    if !config_path.is_file() {
        return Err(WriteError::Missing(config_path.display().to_string()));
    }

    let source = std::fs::read_to_string(config_path)?;
    let action_expr = format_action(action, "")?;
    let keys_expr = format_keys(keys)?;
    let opts = format_opts(&[], name);
    let line = format!("hl.bind({keys_expr}, {action_expr}, {opts})");

    if submap.is_empty() {
        append_into_section(
            config_path,
            MANAGED_BINDS_BEGIN,
            MANAGED_BINDS_END,
            &line,
            WriteMode::Appended,
        )
    } else {
        let updated = crate::writer::submaps::insert_bind_into_managed_submap(
            &source,
            submap,
            &line,
        )?;
        write_config_atomic(config_path, &updated)?;
        Ok(ok_result(
            config_path.display().to_string(),
            WriteMode::Appended,
        ))
    }
}

fn update_in_place(
    path: &Path,
    bind: &Keybind,
    name: &str,
    keys: &str,
    action: &str,
) -> Result<WriteResult, WriteError> {
    let source = std::fs::read_to_string(path)?;
    let (start, end) = find_bind_call_span(&source, bind.source_line)?;
    let action_expr = format_action(action, &bind.action)?;
    let keys_expr = format_keys(keys)?;
    let opts = format_opts(&bind.flags, name);
    let replacement = format!("hl.bind({keys_expr}, {action_expr}, {opts})");

    let mut updated = String::with_capacity(source.len() + replacement.len());
    updated.push_str(&source[..start]);
    updated.push_str(&replacement);
    updated.push_str(&source[end..]);
    write_config_atomic(path, &updated)?;

    Ok(ok_result(path.display().to_string(), WriteMode::InPlace))
}

fn materialize_override(
    path: &Path,
    bind: &Keybind,
    name: &str,
    keys: &str,
    action: &str,
) -> Result<WriteResult, WriteError> {
    let source = std::fs::read_to_string(path)?;
    let opts = format_opts(&bind.flags, name);
    let action_expr = format_action(action, &bind.action)?;
    let keys_expr = format_keys(keys)?;
    let line = format!(
        "hl.unbind({old})\nhl.bind({keys_expr}, {action}, {opts})",
        old = lua_string_literal(&bind.keys),
        action = action_expr,
        opts = opts,
    );
    // Keep overrides in the managed binds section so user config above stays untouched.
    let comment = format!(
        "-- override for {} (was {}:{})",
        bind.keys,
        path.file_name().and_then(|n| n.to_str()).unwrap_or("config"),
        bind.source_line
    );
    let block = format!("{comment}\n{line}");
    let updated =
        insert_into_managed_section(&source, MANAGED_BINDS_BEGIN, MANAGED_BINDS_END, &block)?;
    write_config_atomic(path, &updated)?;

    Ok(ok_result(
        path.display().to_string(),
        WriteMode::Materialized,
    ))
}

fn delete_bind_in_place(path: &Path, bind: &Keybind) -> Result<WriteResult, WriteError> {
    let source = std::fs::read_to_string(path)?;
    let (start, end) = find_bind_call_span(&source, bind.source_line)?;
    let (cut_start, cut_end) = statement_cut_range(&source, start, end);

    let mut updated = String::with_capacity(source.len());
    updated.push_str(&source[..cut_start]);
    updated.push_str(&source[cut_end..]);
    write_config_atomic(path, &updated)?;

    Ok(ok_result(path.display().to_string(), WriteMode::InPlace))
}

fn materialize_unbind(path: &Path, bind: &Keybind) -> Result<WriteResult, WriteError> {
    let source = std::fs::read_to_string(path)?;
    let line = format!("hl.unbind({})", lua_string_literal(&bind.keys));
    let comment = format!(
        "-- unbound by hyprbinds (was {}:{})",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("config"),
        bind.source_line
    );
    let block = format!("{comment}\n{line}");
    let updated =
        insert_into_managed_section(&source, MANAGED_BINDS_BEGIN, MANAGED_BINDS_END, &block)?;
    write_config_atomic(path, &updated)?;

    Ok(ok_result(
        path.display().to_string(),
        WriteMode::Materialized,
    ))
}
