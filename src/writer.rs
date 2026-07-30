use crate::bind::Keybind;
use crate::env::{EnvVar, Submap};
use crate::keys::lua_string_literal;
use crate::settings_config;
use crate::spec::{self, SpecItem};
use crate::startup::StartupEntry;
use crate::variables::{substitute_action_template, template_to_lua_expr, ConfigVariable};
use crate::window_rules::{self, WindowRule};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WriteError {
    #[error("source file missing: {0}")]
    Missing(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not find call near line {line}")]
    CallNotFound { line: u32 },
    #[error("could not find variable declaration for {name}")]
    VariableNotFound { name: String },
    #[error("invalid edit: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone)]
pub struct WriteResult {
    pub path: String,
    pub mode: WriteMode,
    pub updated_files: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteMode {
    InPlace,
    Materialized,
    Appended,
}

const MANAGED_BINDS_BEGIN: &str = "-- >>> hyprbinds:managed-binds";
const MANAGED_BINDS_END: &str = "-- <<< hyprbinds:managed-binds";
const MANAGED_VARS_BEGIN: &str = "-- >>> hyprbinds:managed-variables";
const MANAGED_VARS_END: &str = "-- <<< hyprbinds:managed-variables";
const MANAGED_RULES_BEGIN: &str = "-- >>> hyprbinds:managed-window-rules";
const MANAGED_RULES_END: &str = "-- <<< hyprbinds:managed-window-rules";
const MANAGED_CONFIG_BEGIN: &str = "-- >>> hyprbinds:managed-config";
const MANAGED_CONFIG_END: &str = "-- <<< hyprbinds:managed-config";
const MANAGED_MONITORS_BEGIN: &str = "-- >>> hyprbinds:managed-monitors";
const MANAGED_MONITORS_END: &str = "-- <<< hyprbinds:managed-monitors";
const MANAGED_DEVICES_BEGIN: &str = "-- >>> hyprbinds:managed-devices";
const MANAGED_DEVICES_END: &str = "-- <<< hyprbinds:managed-devices";
const MANAGED_GESTURES_BEGIN: &str = "-- >>> hyprbinds:managed-gestures";
const MANAGED_GESTURES_END: &str = "-- <<< hyprbinds:managed-gestures";
const MANAGED_CURVES_BEGIN: &str = "-- >>> hyprbinds:managed-curves";
const MANAGED_CURVES_END: &str = "-- <<< hyprbinds:managed-curves";
const MANAGED_ANIMS_BEGIN: &str = "-- >>> hyprbinds:managed-animations";
const MANAGED_ANIMS_END: &str = "-- <<< hyprbinds:managed-animations";
const MANAGED_WS_RULES_BEGIN: &str = "-- >>> hyprbinds:managed-workspace-rules";
const MANAGED_WS_RULES_END: &str = "-- <<< hyprbinds:managed-workspace-rules";
const MANAGED_LAYER_RULES_BEGIN: &str = "-- >>> hyprbinds:managed-layer-rules";
const MANAGED_LAYER_RULES_END: &str = "-- <<< hyprbinds:managed-layer-rules";
const MANAGED_ENV_BEGIN: &str = "-- >>> hyprbinds:managed-env";
const MANAGED_ENV_END: &str = "-- <<< hyprbinds:managed-env";
const MANAGED_SUBMAPS_BEGIN: &str = "-- >>> hyprbinds:managed-submaps";
const MANAGED_SUBMAPS_END: &str = "-- <<< hyprbinds:managed-submaps";
const MANAGED_STARTUP_BEGIN: &str = "-- >>> hyprbinds:managed-startup";
const MANAGED_STARTUP_END: &str = "-- <<< hyprbinds:managed-startup";

fn ok_result(path: impl Into<String>, mode: WriteMode) -> WriteResult {
    let path = path.into();
    WriteResult {
        updated_files: vec![path.clone()],
        path,
        mode,
    }
}

fn ok_result_many(primary: impl Into<String>, mode: WriteMode, files: Vec<String>) -> WriteResult {
    WriteResult {
        path: primary.into(),
        mode,
        updated_files: files,
    }
}

/// Write config safely: snapshot last-good, then atomic replace via temp file.
fn write_config_atomic(path: &Path, contents: &str) -> Result<(), WriteError> {
    crate::backup::snapshot_before_write(path)
        .map_err(|e| WriteError::Invalid(format!("backup failed: {e}")))?;

    let tmp = path.with_extension("lua.hyprbinds.tmp");
    fs::write(&tmp, contents)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

fn ensure_managed_section(source: &str, begin: &str, end: &str) -> String {
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
fn join_section_entries(entries: &[String]) -> String {
    entries.join("\n\n")
}

/// Insert `line` just before the managed section end marker.
fn insert_into_managed_section(
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

fn update_in_place(
    path: &Path,
    bind: &Keybind,
    name: &str,
    keys: &str,
    action: &str,
) -> Result<WriteResult, WriteError> {
    let source = fs::read_to_string(path)?;
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
    let source = fs::read_to_string(path)?;
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

    let source = fs::read_to_string(config_path)?;
    let action_expr = format_action(action, "")?;
    let keys_expr = format_keys(keys)?;
    let opts = format_opts(&[], name);
    let line = format!("hl.bind({keys_expr}, {action_expr}, {opts})");

    let updated = if submap.is_empty() {
        insert_into_managed_section(&source, MANAGED_BINDS_BEGIN, MANAGED_BINDS_END, &line)?
    } else {
        insert_bind_into_managed_submap(&source, submap, &line)?
    };
    write_config_atomic(config_path, &updated)?;

    Ok(ok_result(
        config_path.display().to_string(),
        WriteMode::Appended,
    ))
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

fn delete_bind_in_place(path: &Path, bind: &Keybind) -> Result<WriteResult, WriteError> {
    let source = fs::read_to_string(path)?;
    let (start, end) = find_bind_call_span(&source, bind.source_line)?;
    let (cut_start, cut_end) = statement_cut_range(&source, start, end);

    let mut updated = String::with_capacity(source.len());
    updated.push_str(&source[..cut_start]);
    updated.push_str(&source[cut_end..]);
    write_config_atomic(path, &updated)?;

    Ok(ok_result(path.display().to_string(), WriteMode::InPlace))
}

fn materialize_unbind(path: &Path, bind: &Keybind) -> Result<WriteResult, WriteError> {
    let source = fs::read_to_string(path)?;
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

fn format_keys(keys: &str) -> Result<String, WriteError> {
    template_to_lua_expr(keys).map_err(WriteError::Invalid)
}

fn format_action(edited: &str, original: &str) -> Result<String, WriteError> {
    let edited = edited.trim();
    let substituted = substitute_action_template(edited).map_err(WriteError::Invalid)?;
    let substituted = substituted.trim();

    if substituted.starts_with("hl.") || substituted.starts_with("function") {
        return Ok(substituted.to_string());
    }
    if substituted == original && original.starts_with("hl.") {
        return Ok(original.to_string());
    }
    // Plain text → shell command (may still contain bare vars already substituted).
    if edited.contains("{{") {
        // substitute_action_template already turned vars into idents; if result isn't hl.*,
        // wrap as exec_cmd only when it's a bare identifier.
        if substituted.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Ok(format!("hl.dsp.exec_cmd({substituted})"));
        }
        return Ok(substituted.to_string());
    }
    Ok(format!(
        "hl.dsp.exec_cmd({})",
        lua_string_literal(substituted)
    ))
}

pub fn save_variable(
    var: &ConfigVariable,
    new_name: &str,
    new_value: &str,
    related_files: &[PathBuf],
    existing_names: &[String],
) -> Result<WriteResult, WriteError> {
    let new_name = new_name.trim();
    let new_value = new_value;
    if new_name.is_empty() {
        return Err(WriteError::Invalid("variable name cannot be empty".into()));
    }
    if !is_lua_ident(new_name) {
        return Err(WriteError::Invalid(format!(
            "invalid Lua identifier '{new_name}'"
        )));
    }
    if var.source_file.is_empty() {
        return Err(WriteError::Invalid(
            "variable has no source location".into(),
        ));
    }
    if new_name != var.name
        && existing_names
            .iter()
            .any(|n| n == new_name)
    {
        return Err(WriteError::Invalid(format!(
            "variable '{new_name}' already exists"
        )));
    }

    let decl_path = PathBuf::from(&var.source_file);
    if !decl_path.is_file() {
        return Err(WriteError::Missing(var.source_file.clone()));
    }

    // 1) Update the declaration (name and/or value) in its source file.
    let source = fs::read_to_string(&decl_path)?;
    let updated = rewrite_variable_declaration(&source, var, new_name, new_value)?;
    write_config_atomic(&decl_path, &updated)?;

    // 2) If renamed, rewrite every identifier occurrence across related config files.
    let mut touched = vec![decl_path.display().to_string()];
    if new_name != var.name {
        let mut files: Vec<PathBuf> = related_files.to_vec();
        if !files.iter().any(|f| f == &decl_path) {
            files.push(decl_path.clone());
        }
        files.sort();
        files.dedup();

        for path in files {
            if !path.is_file() {
                continue;
            }
            let text = fs::read_to_string(&path)?;
            let rewritten = rename_identifier_outside_strings(&text, &var.name, new_name);
            if rewritten != text {
                write_config_atomic(&path, &rewritten)?;
                let display = path.display().to_string();
                if !touched.contains(&display) {
                    touched.push(display);
                }
            }
        }
    }

    Ok(ok_result_many(
        decl_path.display().to_string(),
        WriteMode::InPlace,
        touched,
    ))
}

pub fn add_variable(
    config_path: &Path,
    name: &str,
    value: &str,
) -> Result<WriteResult, WriteError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(WriteError::Invalid("variable name cannot be empty".into()));
    }
    if !is_lua_ident(name) {
        return Err(WriteError::Invalid(format!(
            "invalid Lua identifier '{name}'"
        )));
    }
    if !config_path.is_file() {
        return Err(WriteError::Missing(config_path.display().to_string()));
    }

    let source = fs::read_to_string(config_path)?;
    if source.contains(&format!("local {name} =")) {
        return Err(WriteError::Invalid(format!(
            "variable '{name}' already exists"
        )));
    }

    let decl = format!("local {name} = {}", lua_string_literal(value));
    // New variables go into a managed section so existing locals/config stay untouched.
    let updated =
        insert_into_managed_section(&source, MANAGED_VARS_BEGIN, MANAGED_VARS_END, &decl)?;
    write_config_atomic(config_path, &updated)?;

    Ok(ok_result(
        config_path.display().to_string(),
        WriteMode::Appended,
    ))
}

pub fn delete_variable(var: &ConfigVariable) -> Result<WriteResult, WriteError> {
    if var.source_file.is_empty() || var.source_line == 0 {
        return Err(WriteError::Invalid(
            "variable has no source location".into(),
        ));
    }
    let path = Path::new(&var.source_file);
    if !path.is_file() {
        return Err(WriteError::Missing(var.source_file.clone()));
    }

    let source = fs::read_to_string(path)?;
    let starts = line_start_offsets(&source);
    let idx = var.source_line.saturating_sub(1) as usize;
    if idx >= starts.len() {
        return Err(WriteError::VariableNotFound {
            name: var.name.clone(),
        });
    }
    let line_start = starts[idx];
    let line_end = if idx + 1 < starts.len() {
        starts[idx + 1]
    } else {
        source.len()
    };
    let line = &source[line_start..line_end];
    if !line.contains(&format!("local {}", var.name)) {
        return Err(WriteError::VariableNotFound {
            name: var.name.clone(),
        });
    }

    let mut updated = String::new();
    updated.push_str(&source[..line_start]);
    updated.push_str(&source[line_end..]);
    write_config_atomic(path, &updated)?;

    Ok(ok_result(path.display().to_string(), WriteMode::InPlace))
}

// --- Environment variables (hl.env) ---

pub fn add_env(config_path: &Path, name: &str, value: &str) -> Result<WriteResult, WriteError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(WriteError::Invalid("env name cannot be empty".into()));
    }
    if !config_path.is_file() {
        return Err(WriteError::Missing(config_path.display().to_string()));
    }
    let source = fs::read_to_string(config_path)?;
    let needle = format!("hl.env(\"{}\"", name.replace('"', "\\\""));
    if source.contains(&needle) || source.contains(&format!("hl.env('{name}'")) {
        return Err(WriteError::Invalid(format!(
            "env '{name}' already appears in config"
        )));
    }
    let line = format!(
        "hl.env({}, {})",
        lua_string_literal(name),
        lua_string_literal(value)
    );
    let updated =
        insert_into_managed_section(&source, MANAGED_ENV_BEGIN, MANAGED_ENV_END, &line)?;
    write_config_atomic(config_path, &updated)?;
    Ok(ok_result(
        config_path.display().to_string(),
        WriteMode::Appended,
    ))
}

pub fn save_env(var: &EnvVar, name: &str, value: &str) -> Result<WriteResult, WriteError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(WriteError::Invalid("env name cannot be empty".into()));
    }
    if var.source_file.is_empty() || var.source_line == 0 {
        return Err(WriteError::Invalid("env has no source location".into()));
    }
    let path = Path::new(&var.source_file);
    if !path.is_file() {
        return Err(WriteError::Missing(var.source_file.clone()));
    }
    let source = fs::read_to_string(path)?;
    let (start, end) = find_call_span(&source, var.source_line, "hl.env")?;
    let replacement = format!(
        "hl.env({}, {})",
        lua_string_literal(name),
        lua_string_literal(value)
    );
    let mut updated = String::with_capacity(source.len() + replacement.len());
    updated.push_str(&source[..start]);
    updated.push_str(&replacement);
    updated.push_str(&source[end..]);
    write_config_atomic(path, &updated)?;
    Ok(ok_result(path.display().to_string(), WriteMode::InPlace))
}

pub fn delete_env(var: &EnvVar) -> Result<WriteResult, WriteError> {
    if var.source_file.is_empty() || var.source_line == 0 {
        return Err(WriteError::Invalid("env has no source location".into()));
    }
    let path = Path::new(&var.source_file);
    if !path.is_file() {
        return Err(WriteError::Missing(var.source_file.clone()));
    }
    let source = fs::read_to_string(path)?;
    let (start, end) = find_call_span(&source, var.source_line, "hl.env")?;
    // Drop through end of line
    let mut line_end = end;
    while line_end < source.len() && source.as_bytes()[line_end] != b'\n' {
        line_end += 1;
    }
    if line_end < source.len() {
        line_end += 1;
    }
    let mut updated = String::new();
    updated.push_str(&source[..start]);
    updated.push_str(&source[line_end..]);
    write_config_atomic(path, &updated)?;
    Ok(ok_result(path.display().to_string(), WriteMode::InPlace))
}

// --- Startup / autostart (hl.exec_cmd + hl.on) ---

fn normalize_when(when: &str) -> Result<&'static str, WriteError> {
    match when.trim().to_ascii_lowercase().as_str() {
        "start" | "once" | "exec-once" => Ok("start"),
        "reload" | "exec" => Ok("reload"),
        "shutdown" | "exit" => Ok("shutdown"),
        other => Err(WriteError::Invalid(format!(
            "unknown startup when '{other}' (use start, reload, or shutdown)"
        ))),
    }
}

fn format_exec_cmd_line(command: &str, workspace: &str, indent: &str) -> String {
    if workspace.trim().is_empty() {
        format!("{indent}hl.exec_cmd({})", lua_string_literal(command))
    } else {
        format!(
            "{indent}hl.exec_cmd({}, {{ workspace = {} }})",
            lua_string_literal(command),
            lua_string_literal(workspace.trim())
        )
    }
}

/// Ensure managed startup section exists with start/shutdown wrappers when needed.
fn ensure_startup_section(source: &str) -> String {
    let mut source = ensure_managed_section(source, MANAGED_STARTUP_BEGIN, MANAGED_STARTUP_END);
    let Some(begin_idx) = source.find(MANAGED_STARTUP_BEGIN) else {
        return source;
    };
    let Some(end_idx) = source.find(MANAGED_STARTUP_END) else {
        return source;
    };
    if end_idx <= begin_idx {
        return source;
    }
    let between = &source[begin_idx + MANAGED_STARTUP_BEGIN.len()..end_idx];
    if between.contains("hyprland.start") {
        return source;
    }
    // Seed wrappers so we can insert into them.
    let seed = "\nhl.on(\"hyprland.start\", function()\nend)\nhl.on(\"hyprland.shutdown\", function()\nend)\n";
    source.insert_str(end_idx, seed);
    source
}

fn insert_into_startup_when(
    source: &str,
    when: &str,
    line: &str,
) -> Result<String, WriteError> {
    let mut source = ensure_startup_section(source);
    let Some(begin_idx) = source.find(MANAGED_STARTUP_BEGIN) else {
        return Err(WriteError::Invalid(
            "managed startup begin marker missing".into(),
        ));
    };
    let Some(end_idx) = source.find(MANAGED_STARTUP_END) else {
        return Err(WriteError::Invalid(
            "managed startup end marker missing".into(),
        ));
    };

    match when {
        "reload" => {
            // Insert just after the begin marker (top-level = re-runs on reload).
            let insert_at = begin_idx + MANAGED_STARTUP_BEGIN.len();
            let mut insertion = String::from("\n");
            insertion.push_str(line);
            if !line.ends_with('\n') {
                insertion.push('\n');
            }
            source.insert_str(insert_at, &insertion);
            Ok(source)
        }
        "start" | "shutdown" => {
            let needle = if when == "start" {
                "hl.on(\"hyprland.start\""
            } else {
                "hl.on(\"hyprland.shutdown\""
            };
            let search = &source[begin_idx..end_idx];
            let Some(rel) = search.find(needle) else {
                return Err(WriteError::Invalid(format!(
                    "managed startup missing {needle} wrapper"
                )));
            };
            let abs = begin_idx + rel;
            // Find the matching `end)` for this on() function — first lone `end)` after function().
            let after_fn = source[abs..]
                .find("function()")
                .map(|i| abs + i + "function()".len())
                .ok_or_else(|| WriteError::Invalid("startup wrapper missing function()".into()))?;
            let rest = &source[after_fn..end_idx];
            let Some(rel_end) = rest.find("\nend)") else {
                return Err(WriteError::Invalid(
                    "startup wrapper missing closing end)".into(),
                ));
            };
            let insert_at = after_fn + rel_end + 1; // before 'end)'
            let mut insertion = String::new();
            insertion.push_str(line);
            if !line.ends_with('\n') {
                insertion.push('\n');
            }
            source.insert_str(insert_at, &insertion);
            Ok(source)
        }
        _ => Err(WriteError::Invalid(format!("bad when '{when}'"))),
    }
}

pub fn add_startup(
    config_path: &Path,
    command: &str,
    when: &str,
    workspace: &str,
) -> Result<WriteResult, WriteError> {
    let command = command.trim();
    if command.is_empty() {
        return Err(WriteError::Invalid("command cannot be empty".into()));
    }
    let when = normalize_when(when)?;
    if !config_path.is_file() {
        return Err(WriteError::Missing(config_path.display().to_string()));
    }
    let source = fs::read_to_string(config_path)?;
    let indent = if when == "reload" { "" } else { "  " };
    let line = format_exec_cmd_line(command, workspace, indent);
    let updated = insert_into_startup_when(&source, when, &line)?;
    write_config_atomic(config_path, &updated)?;
    Ok(ok_result(
        config_path.display().to_string(),
        WriteMode::Appended,
    ))
}

pub fn save_startup(
    entry: &StartupEntry,
    command: &str,
    when: &str,
    workspace: &str,
) -> Result<WriteResult, WriteError> {
    let command = command.trim();
    if command.is_empty() {
        return Err(WriteError::Invalid("command cannot be empty".into()));
    }
    let new_when = normalize_when(when)?;
    if entry.source_file.is_empty() || entry.source_line == 0 {
        return Err(WriteError::Invalid("startup entry has no source location".into()));
    }
    let path = Path::new(&entry.source_file);
    if !path.is_file() {
        return Err(WriteError::Missing(entry.source_file.clone()));
    }

    // If `when` changes, delete + re-add into the right managed wrapper.
    let old_when = normalize_when(&entry.when).unwrap_or("start");
    if old_when != new_when {
        delete_startup(entry)?;
        return add_startup(path, command, new_when, workspace);
    }

    let source = fs::read_to_string(path)?;
    let (start, end) = find_call_span(&source, entry.source_line, "hl.exec_cmd")?;
    // Preserve existing indentation.
    let line_start = source[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let indent: String = source[line_start..start]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();
    let replacement = format_exec_cmd_line(command, workspace, &indent);
    let mut updated = String::with_capacity(source.len() + replacement.len());
    updated.push_str(&source[..start]);
    updated.push_str(&replacement);
    updated.push_str(&source[end..]);
    write_config_atomic(path, &updated)?;
    Ok(ok_result(path.display().to_string(), WriteMode::InPlace))
}

pub fn delete_startup(entry: &StartupEntry) -> Result<WriteResult, WriteError> {
    if entry.source_file.is_empty() || entry.source_line == 0 {
        return Err(WriteError::Invalid("startup entry has no source location".into()));
    }
    let path = Path::new(&entry.source_file);
    if !path.is_file() {
        return Err(WriteError::Missing(entry.source_file.clone()));
    }
    let source = fs::read_to_string(path)?;
    let (start, end) = find_call_span(&source, entry.source_line, "hl.exec_cmd")?;
    let line_start = source[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    // Include leading indent by deleting from line_start
    let mut line_end = end;
    while line_end < source.len() && source.as_bytes()[line_end] != b'\n' {
        line_end += 1;
    }
    if line_end < source.len() {
        line_end += 1;
    }
    let mut updated = String::new();
    updated.push_str(&source[..line_start]);
    updated.push_str(&source[line_end..]);
    write_config_atomic(path, &updated)?;
    Ok(ok_result(path.display().to_string(), WriteMode::InPlace))
}

// --- Submaps ---

pub fn add_submap(config_path: &Path, name: &str) -> Result<WriteResult, WriteError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(WriteError::Invalid("submap name cannot be empty".into()));
    }
    if name == "reset" || name == "global" {
        return Err(WriteError::Invalid(format!(
            "'{name}' is reserved"
        )));
    }
    if !config_path.is_file() {
        return Err(WriteError::Missing(config_path.display().to_string()));
    }
    let source = fs::read_to_string(config_path)?;
    if find_define_submap_span(&source, name).is_some() {
        return Err(WriteError::Invalid(format!(
            "submap '{name}' already exists"
        )));
    }
    let block = format!(
        "hl.define_submap({}, function()\n  -- binds for {name}\nend)",
        lua_string_literal(name)
    );
    let updated =
        insert_into_managed_section(&source, MANAGED_SUBMAPS_BEGIN, MANAGED_SUBMAPS_END, &block)?;
    write_config_atomic(config_path, &updated)?;
    Ok(ok_result(
        config_path.display().to_string(),
        WriteMode::Appended,
    ))
}

pub fn delete_submap(submap: &Submap) -> Result<WriteResult, WriteError> {
    let name = submap.name.trim();
    if name.is_empty() {
        return Err(WriteError::Invalid("submap name empty".into()));
    }
    if submap.bind_count > 0 {
        return Err(WriteError::Invalid(format!(
            "submap '{name}' still has {} bind(s); move or delete them first",
            submap.bind_count
        )));
    }
    if submap.source_file.is_empty() {
        return Err(WriteError::Invalid(
            "submap has no source location".into(),
        ));
    }
    let path = Path::new(&submap.source_file);
    if !path.is_file() {
        return Err(WriteError::Missing(submap.source_file.clone()));
    }
    let source = fs::read_to_string(path)?;
    let Some((start, end)) = find_define_submap_span(&source, name) else {
        return Err(WriteError::Invalid(format!(
            "could not find hl.define_submap(\"{name}\")"
        )));
    };
    let mut line_end = end;
    while line_end < source.len() && source.as_bytes()[line_end] != b'\n' {
        line_end += 1;
    }
    if line_end < source.len() {
        line_end += 1;
    }
    let mut updated = String::new();
    updated.push_str(&source[..start]);
    updated.push_str(&source[line_end..]);
    write_config_atomic(path, &updated)?;
    Ok(ok_result(path.display().to_string(), WriteMode::InPlace))
}

/// Insert `bind_line` inside a managed `hl.define_submap("name", function() … end)`.
fn insert_bind_into_managed_submap(
    source: &str,
    submap: &str,
    bind_line: &str,
) -> Result<String, WriteError> {
    let source = ensure_managed_section(source, MANAGED_SUBMAPS_BEGIN, MANAGED_SUBMAPS_END);
    if let Some((start, end)) = find_define_submap_span(&source, submap) {
        // Insert before the final `end)` of the block.
        let body = &source[start..end];
        let Some(rel) = body.rfind("end)") else {
            return Err(WriteError::Invalid(format!(
                "malformed define_submap for '{submap}'"
            )));
        };
        let insert_at = start + rel;
        let mut out = String::with_capacity(source.len() + bind_line.len() + 4);
        out.push_str(&source[..insert_at]);
        out.push_str("  ");
        out.push_str(bind_line);
        out.push('\n');
        out.push_str(&source[insert_at..]);
        return Ok(out);
    }
    // Create a new submap block with this bind inside.
    let block = format!(
        "hl.define_submap({}, function()\n  {}\nend)",
        lua_string_literal(submap),
        bind_line
    );
    insert_into_managed_section(&source, MANAGED_SUBMAPS_BEGIN, MANAGED_SUBMAPS_END, &block)
}

/// Find span of `hl.define_submap("name", … end)` including the closing `end)`.
fn find_define_submap_span(source: &str, name: &str) -> Option<(usize, usize)> {
    let patterns = [
        format!("hl.define_submap(\"{}\"", name.replace('"', "\\\"")),
        format!("hl.define_submap('{}'", name.replace('\'', "\\'")),
    ];
    let mut start = None;
    for pat in &patterns {
        if let Some(i) = source.find(pat) {
            start = Some(i);
            break;
        }
    }
    let start = start?;
    // Balance from first `function` after start to matching end)
    let after = &source[start..];
    let fn_rel = after.find("function")?;
    let mut depth = 0i32;
    let bytes = after.as_bytes();
    let mut i = fn_rel;
    while i < after.len() {
        // crude scan for "function" / "end"
        if after[i..].starts_with("function")
            && (i + 8 == after.len()
                || !after.as_bytes()[i + 8].is_ascii_alphanumeric() && after.as_bytes()[i + 8] != b'_')
        {
            depth += 1;
            i += 8;
            continue;
        }
        if after[i..].starts_with("end")
            && (i + 3 == after.len()
                || !after.as_bytes()[i + 3].is_ascii_alphanumeric() && after.as_bytes()[i + 3] != b'_')
        {
            depth -= 1;
            if depth == 0 {
                // include optional `)` after end
                let mut end = start + i + 3;
                while end < source.len() && source.as_bytes()[end].is_ascii_whitespace() {
                    end += 1;
                }
                if end < source.len() && source.as_bytes()[end] == b')' {
                    end += 1;
                }
                let _ = bytes;
                return Some((start, end));
            }
            i += 3;
            continue;
        }
        i += 1;
    }
    None
}

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

    let source = fs::read_to_string(config_path)?;
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

    let source = fs::read_to_string(path)?;
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

    let source = fs::read_to_string(path)?;
    let (start, end) = find_call_span(&source, rule.source_line, "hl.window_rule")?;
    let (cut_start, cut_end) = statement_cut_range(&source, start, end);

    let mut updated = String::with_capacity(source.len());
    updated.push_str(&source[..cut_start]);
    updated.push_str(&source[cut_end..]);
    write_config_atomic(path, &updated)?;

    Ok(ok_result(path.display().to_string(), WriteMode::InPlace))
}

// --- Generic SpecItem CRUD (monitors, devices, gestures, workspace rules, animations) ---

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
    let source = fs::read_to_string(config_path)?;
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
    let source = fs::read_to_string(path)?;
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
    let source = fs::read_to_string(path)?;
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
    let source = fs::read_to_string(config_path)?;
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
    let source = fs::read_to_string(path)?;
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
    let source = fs::read_to_string(path)?;
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
        let source = fs::read_to_string(path)?;
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
        let source = fs::read_to_string(path)?;
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
    let source = fs::read_to_string(config_path)?;
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
    let source = fs::read_to_string(path)?;
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

/// Rewrite the single managed-config override block with MVP-filtered merged settings.
pub fn save_config_override(
    config_path: &Path,
    merged: &Value,
) -> Result<WriteResult, WriteError> {
    if !config_path.is_file() {
        return Err(WriteError::Missing(config_path.display().to_string()));
    }
    let filtered = settings_config::filter_mvp(merged);
    let call = spec::format_config_call(&filtered);
    let source = fs::read_to_string(config_path)?;
    let updated = replace_managed_section_contents(
        &source,
        MANAGED_CONFIG_BEGIN,
        MANAGED_CONFIG_END,
        &call,
    )?;
    write_config_atomic(config_path, &updated)?;
    Ok(ok_result(
        config_path.display().to_string(),
        WriteMode::Appended,
    ))
}

/// Rewrite every hyprbinds managed section from a collected model.
///
/// Unmanaged Lua outside markers is preserved. Entries that cannot be
/// translated (e.g. Lua-function bind actions) are skipped.
pub fn apply_collection(
    config_path: &Path,
    collection: &crate::bind::BindCollection,
) -> Result<WriteResult, WriteError> {
    use crate::bind::Keybind;
    use std::collections::BTreeMap as Map;

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if !config_path.is_file() {
        fs::write(
            config_path,
            "-- Generated by hyprbinds import\nlocal hl = require(\"hyprland\")\n",
        )?;
    }

    let mut source = fs::read_to_string(config_path)?;

    // --- Variables ---
    {
        let mut lines = Vec::new();
        for var in &collection.variables {
            let name = var.name.trim();
            if name.is_empty() || !is_lua_ident(name) {
                continue;
            }
            lines.push(format!(
                "local {name} = {}",
                lua_string_literal(&var.value)
            ));
        }
        source = replace_managed_section_contents(
            &source,
            MANAGED_VARS_BEGIN,
            MANAGED_VARS_END,
            &join_section_entries(&lines),
        )?;
    }

    // --- Root binds (empty submap) ---
    {
        let mut lines = Vec::new();
        for bind in collection.binds.iter().filter(|b| b.submap.trim().is_empty()) {
            if let Some(line) = format_bind_line(bind) {
                lines.push(line);
            }
        }
        source = replace_managed_section_contents(
            &source,
            MANAGED_BINDS_BEGIN,
            MANAGED_BINDS_END,
            &join_section_entries(&lines),
        )?;
    }

    // --- Submaps (+ their binds) ---
    {
        let mut by_submap: Map<String, Vec<&Keybind>> = Map::new();
        for bind in &collection.binds {
            let sm = bind.submap.trim();
            if sm.is_empty() {
                continue;
            }
            by_submap.entry(sm.to_string()).or_default().push(bind);
        }
        for sub in &collection.submaps {
            let name = sub.name.trim();
            if name.is_empty() {
                continue;
            }
            by_submap.entry(name.to_string()).or_default();
        }
        let mut blocks = Vec::new();
        for (name, binds) in &by_submap {
            let mut body = Vec::new();
            for bind in binds {
                if let Some(line) = format_bind_line(bind) {
                    body.push(format!("  {line}"));
                }
            }
            if body.is_empty() {
                body.push(format!("  -- binds for {name}"));
            }
            blocks.push(format!(
                "hl.define_submap({}, function()\n{}\nend)",
                lua_string_literal(name),
                body.join("\n")
            ));
        }
        source = replace_managed_section_contents(
            &source,
            MANAGED_SUBMAPS_BEGIN,
            MANAGED_SUBMAPS_END,
            &join_section_entries(&blocks),
        )?;
    }

    // --- Env ---
    {
        let mut lines = Vec::new();
        for env in &collection.env {
            let name = env.name.trim();
            if name.is_empty() {
                continue;
            }
            lines.push(format!(
                "hl.env({}, {})",
                lua_string_literal(name),
                lua_string_literal(&env.value)
            ));
        }
        source = replace_managed_section_contents(
            &source,
            MANAGED_ENV_BEGIN,
            MANAGED_ENV_END,
            &join_section_entries(&lines),
        )?;
    }

    // --- Startup ---
    {
        let mut reload = Vec::new();
        let mut start = Vec::new();
        let mut shutdown = Vec::new();
        for entry in &collection.startup {
            let cmd = entry.command.trim();
            if cmd.is_empty() {
                continue;
            }
            let when = normalize_when(&entry.when).unwrap_or("start");
            match when {
                "reload" => reload.push(format_exec_cmd_line(cmd, &entry.workspace, "")),
                "shutdown" => shutdown.push(format_exec_cmd_line(cmd, &entry.workspace, "  ")),
                _ => start.push(format_exec_cmd_line(cmd, &entry.workspace, "  ")),
            }
        }
        let mut body = String::new();
        if !reload.is_empty() {
            body.push_str(&reload.join("\n"));
            body.push('\n');
        }
        body.push_str("hl.on(\"hyprland.start\", function()\n");
        if start.is_empty() {
            body.push_str("end)\n");
        } else {
            body.push_str(&start.join("\n"));
            body.push_str("\nend)\n");
        }
        body.push_str("hl.on(\"hyprland.shutdown\", function()\n");
        if shutdown.is_empty() {
            body.push_str("end)");
        } else {
            body.push_str(&shutdown.join("\n"));
            body.push_str("\nend)");
        }
        source = replace_managed_section_contents(
            &source,
            MANAGED_STARTUP_BEGIN,
            MANAGED_STARTUP_END,
            &body,
        )?;
    }

    // --- Window rules ---
    {
        let mut blocks = Vec::new();
        for rule in &collection.window_rules {
            if rule.match_props.is_empty() && rule.effects.is_empty() {
                continue;
            }
            blocks.push(window_rules::format_lua_rule(
                &rule.name,
                &rule.match_props,
                &rule.effects,
            ));
        }
        source = replace_managed_section_contents(
            &source,
            MANAGED_RULES_BEGIN,
            MANAGED_RULES_END,
            &join_section_entries(&blocks),
        )?;
    }

    // --- Layer rules ---
    {
        let mut blocks = Vec::new();
        for rule in &collection.layer_rules {
            if rule.match_props.is_empty() && rule.effects.is_empty() {
                continue;
            }
            let block = window_rules::format_lua_rule(
                &rule.name,
                &rule.match_props,
                &rule.effects,
            )
            .replacen("hl.window_rule", "hl.layer_rule", 1);
            blocks.push(block);
        }
        source = replace_managed_section_contents(
            &source,
            MANAGED_LAYER_RULES_BEGIN,
            MANAGED_LAYER_RULES_END,
            &join_section_entries(&blocks),
        )?;
    }

    // --- Spec sections ---
    let spec_sections = [
        (
            MANAGED_WS_RULES_BEGIN,
            MANAGED_WS_RULES_END,
            "hl.workspace_rule",
            &collection.workspace_rules,
            true,
        ),
        (
            MANAGED_MONITORS_BEGIN,
            MANAGED_MONITORS_END,
            "hl.monitor",
            &collection.monitors,
            true,
        ),
        (
            MANAGED_DEVICES_BEGIN,
            MANAGED_DEVICES_END,
            "hl.device",
            &collection.devices,
            true,
        ),
        (
            MANAGED_GESTURES_BEGIN,
            MANAGED_GESTURES_END,
            "hl.gesture",
            &collection.gestures,
            false,
        ),
        (
            MANAGED_ANIMS_BEGIN,
            MANAGED_ANIMS_END,
            "hl.animation",
            &collection.animations,
            false,
        ),
    ];
    for (begin, end, fn_name, items, use_name) in spec_sections {
        let mut blocks = Vec::new();
        for item in items {
            if item.fields.is_empty() {
                continue;
            }
            let name = if use_name {
                let n = item.name.trim();
                if n.is_empty() {
                    None
                } else {
                    Some(n)
                }
            } else {
                None
            };
            blocks.push(spec::format_call(fn_name, &item.fields, name));
        }
        source = replace_managed_section_contents(
            &source,
            begin,
            end,
            &join_section_entries(&blocks),
        )?;
    }

    // --- Curves ---
    {
        let mut blocks = Vec::new();
        for item in &collection.curves {
            let name = if item.name.trim().is_empty() {
                item.display_name.trim()
            } else {
                item.name.trim()
            };
            if name.is_empty() {
                continue;
            }
            blocks.push(spec::format_curve_call(name, &item.fields));
        }
        source = replace_managed_section_contents(
            &source,
            MANAGED_CURVES_BEGIN,
            MANAGED_CURVES_END,
            &join_section_entries(&blocks),
        )?;
    }

    // --- Config overrides ---
    {
        let filtered = settings_config::filter_mvp(&collection.config_merged);
        let call = spec::format_config_call(&filtered);
        source = replace_managed_section_contents(
            &source,
            MANAGED_CONFIG_BEGIN,
            MANAGED_CONFIG_END,
            &call,
        )?;
    }

    write_config_atomic(config_path, &source)?;
    Ok(ok_result(
        config_path.display().to_string(),
        WriteMode::Materialized,
    ))
}

fn format_bind_line(bind: &crate::bind::Keybind) -> Option<String> {
    let keys = bind.keys.trim();
    let action = bind.action.trim();
    let name = if bind.name.trim().is_empty() {
        bind.description.trim()
    } else {
        bind.name.trim()
    };
    if keys.is_empty() || action.is_empty() {
        return None;
    }
    if action.starts_with('<') {
        return None;
    }
    let keys_expr = format_keys(keys).ok()?;
    let action_expr = format_action(action, "").ok()?;
    let opts = format_opts(&bind.flags, if name.is_empty() { "untitled" } else { name });
    Some(format!("hl.bind({keys_expr}, {action_expr}, {opts})"))
}

/// Ensure section exists, then replace everything between begin and end markers.
fn replace_managed_section_contents(
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

fn rewrite_variable_declaration(
    source: &str,
    var: &ConfigVariable,
    new_name: &str,
    new_value: &str,
) -> Result<String, WriteError> {
    let starts = line_start_offsets(source);
    let idx = var.source_line.saturating_sub(1) as usize;
    if idx >= starts.len() {
        return Err(WriteError::VariableNotFound {
            name: var.name.clone(),
        });
    }
    let line_start = starts[idx];
    let line_end = if idx + 1 < starts.len() {
        starts[idx + 1]
    } else {
        source.len()
    };
    let line = &source[line_start..line_end];
    let trimmed = line.trim_end_matches(['\r', '\n']);
    let ending = &line[trimmed.len()..];

    if !trimmed.contains(&format!("local {}", var.name)) {
        // Fallback: search nearby lines for the declaration.
        if let Some((s, e, found)) = find_local_decl_near(source, &starts, idx, &var.name) {
            let new_line = format!(
                "{}local {new_name} = {}{}",
                leading_ws(found),
                lua_string_literal(new_value),
                line_ending(found)
            );
            let mut out = String::new();
            out.push_str(&source[..s]);
            out.push_str(&new_line);
            out.push_str(&source[e..]);
            return Ok(out);
        }
        return Err(WriteError::VariableNotFound {
            name: var.name.clone(),
        });
    }

    let new_line = format!(
        "{}local {new_name} = {}{}",
        leading_ws(trimmed),
        lua_string_literal(new_value),
        ending
    );
    let mut out = String::new();
    out.push_str(&source[..line_start]);
    out.push_str(&new_line);
    out.push_str(&source[line_end..]);
    Ok(out)
}

fn find_local_decl_near<'a>(
    source: &'a str,
    starts: &[usize],
    around: usize,
    name: &str,
) -> Option<(usize, usize, &'a str)> {
    let from = around.saturating_sub(3);
    let to = (around + 4).min(starts.len());
    for i in from..to {
        let s = starts[i];
        let e = if i + 1 < starts.len() {
            starts[i + 1]
        } else {
            source.len()
        };
        let line = &source[s..e];
        if line.contains(&format!("local {name}")) {
            return Some((s, e, line));
        }
    }
    None
}

fn rename_identifier_outside_strings(source: &str, old: &str, new: &str) -> String {
    if old == new {
        return source.to_string();
    }
    let bytes = source.as_bytes();
    let old_b = old.as_bytes();
    let mut out = String::with_capacity(source.len());
    let mut i = 0usize;
    let mut in_string: Option<u8> = None;
    let mut in_line_comment = false;

    while i < bytes.len() {
        let b = bytes[i];
        if in_line_comment {
            out.push(b as char);
            if b == b'\n' {
                in_line_comment = false;
            }
            i += 1;
            continue;
        }
        if let Some(quote) = in_string {
            out.push(b as char);
            if b == b'\\' && i + 1 < bytes.len() {
                out.push(bytes[i + 1] as char);
                i += 2;
                continue;
            }
            if b == quote {
                in_string = None;
            }
            i += 1;
            continue;
        }
        if b == b'-' && i + 1 < bytes.len() && bytes[i + 1] == b'-' {
            out.push('-');
            out.push('-');
            in_line_comment = true;
            i += 2;
            continue;
        }
        if b == b'\'' || b == b'"' {
            in_string = Some(b);
            out.push(b as char);
            i += 1;
            continue;
        }

        if i + old_b.len() <= bytes.len() && &bytes[i..i + old_b.len()] == old_b {
            let before_ok = i == 0 || !is_ident_byte(bytes[i - 1]);
            let after_ok = i + old_b.len() >= bytes.len() || !is_ident_byte(bytes[i + old_b.len()]);
            if before_ok && after_ok {
                out.push_str(new);
                i += old_b.len();
                continue;
            }
        }

        out.push(b as char);
        i += 1;
    }
    out
}

fn is_lua_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn leading_ws(line: &str) -> &str {
    let trimmed = line.trim_start_matches([' ', '\t']);
    &line[..line.len() - trimmed.len()]
}

fn line_ending(line: &str) -> &str {
    if line.ends_with("\r\n") {
        "\r\n"
    } else if line.ends_with('\n') {
        "\n"
    } else {
        ""
    }
}

fn format_opts(flags: &[String], name: &str) -> String {
    let mut parts = Vec::new();
    for flag in flags {
        if flag == "device" {
            continue;
        }
        parts.push(format!("{flag} = true"));
    }
    parts.push(format!("description = {}", lua_string_literal(name)));
    format!("{{ {} }}", parts.join(", "))
}

fn find_bind_call_span(source: &str, source_line: u32) -> Result<(usize, usize), WriteError> {
    find_call_span(source, source_line, "hl.bind")
}

/// Expand a call span to the full statement when the call owns its line
/// (leading whitespace only, or an assignment like `local x = hl.bind(...)`).
fn statement_cut_range(source: &str, start: usize, end: usize) -> (usize, usize) {
    let mut cut_end = end.min(source.len());
    if source.as_bytes().get(cut_end) == Some(&b'\n') {
        cut_end += 1;
    }

    let line_start = source[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let mut cut_start = start;
    if line_start <= start {
        let prefix = source[line_start..start].trim();
        if prefix.is_empty() || prefix.contains('=') {
            cut_start = line_start;
        }
    }
    (cut_start.min(cut_end), cut_end)
}

fn find_call_span(
    source: &str,
    source_line: u32,
    needle: &str,
) -> Result<(usize, usize), WriteError> {
    let line_starts = line_start_offsets(source);
    let idx = source_line.saturating_sub(1) as usize;
    if idx >= line_starts.len() {
        return Err(WriteError::CallNotFound { line: source_line });
    }

    let line_start = line_starts[idx];
    let line_end = if idx + 1 < line_starts.len() {
        line_starts[idx + 1]
    } else {
        source.len()
    };

    let search_from = line_starts[idx.saturating_sub(2)];
    // Window rules are often multi-line; search farther ahead.
    let ahead = if needle.contains("window_rule") { 40 } else { 5 };
    let search_to = if idx + ahead < line_starts.len() {
        line_starts[idx + ahead]
    } else {
        source.len()
    };

    let mut candidates = Vec::new();
    let mut pos = search_from;
    while let Some(rel) = source[pos..search_to].find(needle) {
        let start = pos + rel;
        let after_needle = start + needle.len();
        let Some(open_rel) = source[after_needle..].find('(') else {
            break;
        };
        let open = after_needle + open_rel;
        if !source[after_needle..open]
            .chars()
            .all(|c| c.is_whitespace())
        {
            pos = after_needle;
            continue;
        }
        if let Ok(close) = find_matching_paren(source, open) {
            candidates.push((start, close + 1));
        }
        pos = after_needle;
    }

    // Prefer a call that starts on the reported line.
    if let Some(&(start, end)) = candidates
        .iter()
        .find(|(s, _)| *s >= line_start && *s < line_end)
    {
        return Ok((start, end));
    }
    // Then a multi-line call that spans the reported line.
    if let Some(&(start, end)) = candidates
        .iter()
        .find(|(s, e)| *s < line_end && *e > line_start)
    {
        return Ok((start, end));
    }
    // Nearest call at or before the reported line (within the lookback window).
    if let Some(&(start, end)) = candidates.iter().rev().find(|(s, _)| *s <= line_start) {
        return Ok((start, end));
    }
    // Last resort: first candidate in the window.
    if let Some(&(start, end)) = candidates.first() {
        return Ok((start, end));
    }

    Err(WriteError::CallNotFound { line: source_line })
}

fn find_matching_paren(source: &str, open_idx: usize) -> Result<usize, WriteError> {
    let bytes = source.as_bytes();
    if bytes.get(open_idx) != Some(&b'(') {
        return Err(WriteError::CallNotFound { line: 0 });
    }

    let mut depth = 0i32;
    let mut in_string: Option<u8> = None;
    let mut i = open_idx;
    while i < bytes.len() {
        let b = bytes[i];
        if let Some(quote) = in_string {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == quote {
                in_string = None;
            }
            i += 1;
            continue;
        }
        match b {
            b'\'' | b'"' => in_string = Some(b),
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(i);
                }
            }
            _ => {}
        }
        i += 1;
    }

    Err(WriteError::CallNotFound { line: 0 })
}

fn line_start_offsets(source: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, b) in source.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bind::Keybind;
    use std::io::Write;

    #[test]
    fn finds_multiline_bind() {
        let src = r#"
local mainMod = "SUPER"
hl.bind(
    mainMod .. " + M",
    hl.dsp.exec_cmd("exit")
)
"#;
        let (start, end) = find_bind_call_span(src, 3).unwrap();
        assert!(src[start..end].starts_with("hl.bind("));
        assert!(src[start..end].ends_with(')'));
    }

    #[test]
    fn updates_unique_bind_in_place() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("hyprbinds-writer-{}.lua", std::process::id()));
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(
            file,
            "hl.bind(\"SUPER + Q\", hl.dsp.window.close())\nhl.bind(\"SUPER + E\", hl.dsp.exec_cmd(\"thunar\"), {{ locked = true }})"
        )
        .unwrap();
        drop(file);

        let bind = Keybind {
            keys: "SUPER + Q".into(),
            action: "hl.dsp.window.close()".into(),
            flags: vec![],
            description: String::new(),
            submap: String::new(),
            source_file: path.display().to_string(),
            source_line: 1,
            name: "untitled-1".into(),
            id: 0,
        };

        let result = save_bind(
            &bind,
            "Close window",
            "SUPER + SHIFT + Q",
            "hl.dsp.window.close()",
            false,
        )
        .unwrap();
        assert_eq!(result.mode, WriteMode::InPlace);

        let updated = std::fs::read_to_string(&path).unwrap();
        assert!(updated.contains("description = \"Close window\""));
        assert!(updated.contains("\"SUPER + SHIFT + Q\""));
        assert!(updated.contains("hl.dsp.window.close()"));
        assert!(updated.contains("SUPER + E"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn deletes_bind_in_place() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("hyprbinds-writer-del-{}.lua", std::process::id()));
        std::fs::write(
            &path,
            "hl.bind(\"SUPER + Q\", hl.dsp.window.close())\nhl.bind(\"SUPER + E\", hl.dsp.exec_cmd(\"thunar\"))\n",
        )
        .unwrap();

        let bind = Keybind {
            keys: "SUPER + Q".into(),
            action: "hl.dsp.window.close()".into(),
            flags: vec![],
            description: String::new(),
            submap: String::new(),
            source_file: path.display().to_string(),
            source_line: 1,
            name: "untitled-1".into(),
            id: 0,
        };

        let result = delete_bind(&bind, false).unwrap();
        assert_eq!(result.mode, WriteMode::InPlace);

        let updated = std::fs::read_to_string(&path).unwrap();
        assert!(!updated.contains("SUPER + Q"));
        assert!(updated.contains("SUPER + E"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn deletes_second_of_adjacent_binds() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("hyprbinds-writer-del2-{}.lua", std::process::id()));
        std::fs::write(
            &path,
            "hl.bind(\"SUPER + Q\", hl.dsp.window.close())\nhl.bind(\"SUPER + E\", hl.dsp.exec_cmd(\"thunar\"))\nhl.bind(\"SUPER + R\", hl.dsp.exec_cmd(\"rofi\"))\n",
        )
        .unwrap();

        let bind = Keybind {
            keys: "SUPER + E".into(),
            action: "hl.dsp.exec_cmd(\"thunar\")".into(),
            flags: vec![],
            description: String::new(),
            submap: String::new(),
            source_file: path.display().to_string(),
            source_line: 2,
            name: "untitled-2".into(),
            id: 1,
        };

        delete_bind(&bind, false).unwrap();

        let updated = std::fs::read_to_string(&path).unwrap();
        assert!(updated.contains("SUPER + Q"));
        assert!(!updated.contains("SUPER + E"));
        assert!(updated.contains("SUPER + R"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn unbinds_shared_bind() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("hyprbinds-writer-unbind-{}.lua", std::process::id()));
        std::fs::write(
            &path,
            "for i = 1, 2 do\n  hl.bind(\"SUPER + \" .. i, hl.dsp.focus({ workspace = i }))\nend\n",
        )
        .unwrap();

        let bind = Keybind {
            keys: "SUPER + 1".into(),
            action: "hl.dsp.focus({ workspace = 1 })".into(),
            flags: vec![],
            description: String::new(),
            submap: String::new(),
            source_file: path.display().to_string(),
            source_line: 2,
            name: "untitled-1".into(),
            id: 0,
        };

        let result = delete_bind(&bind, true).unwrap();
        assert_eq!(result.mode, WriteMode::Materialized);

        let updated = std::fs::read_to_string(&path).unwrap();
        assert!(updated.contains("-- >>> hyprbinds:managed-binds"));
        assert!(updated.contains("hl.unbind(\"SUPER + 1\")"));
        assert!(updated.contains("for i = 1, 2 do"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn materializes_shared_bind() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("hyprbinds-writer-shared-{}.lua", std::process::id()));
        std::fs::write(
            &path,
            "for i = 1, 2 do\n  hl.bind(\"SUPER + \" .. i, hl.dsp.focus({ workspace = i }))\nend\n",
        )
        .unwrap();

        let bind = Keybind {
            keys: "SUPER + 1".into(),
            action: "hl.dsp.focus({ workspace = 1 })".into(),
            flags: vec![],
            description: String::new(),
            submap: String::new(),
            source_file: path.display().to_string(),
            source_line: 2,
            name: "untitled-1".into(),
            id: 0,
        };

        let result = save_bind(
            &bind,
            "Workspace 1",
            "SUPER + 1",
            "hl.dsp.focus({ workspace = 1 })",
            true,
        )
        .unwrap();
        assert_eq!(result.mode, WriteMode::Materialized);

        let updated = std::fs::read_to_string(&path).unwrap();
        assert!(updated.contains("-- >>> hyprbinds:managed-binds"));
        assert!(updated.contains("hl.unbind(\"SUPER + 1\")"));
        assert!(updated.contains("description = \"Workspace 1\""));
        // Original loop must remain intact.
        assert!(updated.contains("for i = 1, 2 do"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn renames_variable_across_related_files() {
        let dir = std::env::temp_dir().join(format!("hyprbinds-var-multi-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let main = dir.join("hyprland.lua");
        let side = dir.join("sidebar.lua");
        std::fs::write(
            &main,
            "local mainMod = \"SUPER\"\nlocal setup = require(\"sidebar\")\nsetup(mainMod)\nhl.bind(mainMod .. \" + Q\", hl.dsp.window.close())\n",
        )
        .unwrap();
        std::fs::write(
            &side,
            "return function(mainMod)\n  hl.bind(mainMod .. \" + B\", hl.dsp.exec_cmd(\"ags\"))\nend\n",
        )
        .unwrap();

        let var = ConfigVariable {
            name: "mainMod".into(),
            value: "SUPER".into(),
            source_file: main.display().to_string(),
            source_line: 1,
        };
        let result = save_variable(
            &var,
            "modKey",
            "SUPER",
            &[main.clone(), side.clone()],
            &["mainMod".into()],
        )
        .unwrap();
        assert!(result.updated_files.len() >= 2);

        let main_txt = std::fs::read_to_string(&main).unwrap();
        let side_txt = std::fs::read_to_string(&side).unwrap();
        assert!(main_txt.contains("local modKey = \"SUPER\""));
        assert!(main_txt.contains("setup(modKey)"));
        assert!(main_txt.contains("modKey .. \" + Q\""));
        assert!(!main_txt.contains("mainMod"));
        assert!(side_txt.contains("function(modKey)"));
        assert!(side_txt.contains("modKey .. \" + B\""));
        assert!(!side_txt.contains("mainMod"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn updates_and_renames_variable() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("hyprbinds-var-{}.lua", std::process::id()));
        std::fs::write(
            &path,
            "local mainMod = \"SUPER\"\nhl.bind(mainMod .. \" + Q\", hl.dsp.window.close())\n",
        )
        .unwrap();

        let var = ConfigVariable {
            name: "mainMod".into(),
            value: "SUPER".into(),
            source_file: path.display().to_string(),
            source_line: 1,
        };

        save_variable(&var, "mainMod", "ALT", &[path.clone()], &["mainMod".into()]).unwrap();
        let updated = std::fs::read_to_string(&path).unwrap();
        assert!(updated.contains("local mainMod = \"ALT\""));
        assert!(updated.contains("mainMod .. \" + Q\""));

        let var = ConfigVariable {
            name: "mainMod".into(),
            value: "ALT".into(),
            source_file: path.display().to_string(),
            source_line: 1,
        };
        save_variable(&var, "modKey", "ALT", &[path.clone()], &["mainMod".into()]).unwrap();
        let renamed = std::fs::read_to_string(&path).unwrap();
        assert!(renamed.contains("local modKey = \"ALT\""));
        assert!(renamed.contains("modKey .. \" + Q\""));
        assert!(!renamed.contains("mainMod"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn adds_bind_in_managed_section_without_touching_prefix() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("hyprbinds-add-bind-{}.lua", std::process::id()));
        let original = "-- user config\nlocal mainMod = \"SUPER\"\nhl.bind(mainMod .. \" + Q\", hl.dsp.window.close())\n";
        std::fs::write(&path, original).unwrap();

        add_bind(
            &path,
            "Open terminal",
            "{{mainMod}} + Return",
            "hl.dsp.exec_cmd(terminal)",
            "",
        )
        .unwrap();

        let updated = std::fs::read_to_string(&path).unwrap();
        assert!(updated.starts_with("-- user config\nlocal mainMod = \"SUPER\""));
        assert!(updated.contains("-- >>> hyprbinds:managed-binds"));
        assert!(updated.contains("description = \"Open terminal\""));
        assert!(updated.contains("mainMod .. \" + Return\""));
        assert!(updated.contains("-- <<< hyprbinds:managed-binds"));
        assert!(path.with_extension("lua.hyprbinds.bak").exists() || {
            // backup uses path + ".hyprbinds.bak"
            let mut bak = path.as_os_str().to_owned();
            bak.push(".hyprbinds.bak");
            PathBuf::from(bak).is_file()
        });
        let _ = std::fs::remove_file(&path);
        let mut bak = path.as_os_str().to_owned();
        bak.push(".hyprbinds.bak");
        let _ = std::fs::remove_file(PathBuf::from(bak));
    }

    #[test]
    fn adds_variable_in_managed_section() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("hyprbinds-add-var-{}.lua", std::process::id()));
        std::fs::write(&path, "local mainMod = \"SUPER\"\n").unwrap();
        add_variable(&path, "browser", "firefox").unwrap();
        let updated = std::fs::read_to_string(&path).unwrap();
        assert!(updated.contains("local mainMod = \"SUPER\""));
        assert!(updated.contains("-- >>> hyprbinds:managed-variables"));
        assert!(updated.contains("local browser = \"firefox\""));
        let _ = std::fs::remove_file(&path);
        let mut bak = path.as_os_str().to_owned();
        bak.push(".hyprbinds.bak");
        let _ = std::fs::remove_file(PathBuf::from(bak));
    }

    #[test]
    fn adds_window_rule_in_managed_section() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("hyprbinds-add-rule-{}.lua", std::process::id()));
        let original =
            "-- user rules\nhl.window_rule({ name = \"keep\", match = { class = \".*\" }, float = false })\n";
        std::fs::write(&path, original).unwrap();

        let mut match_props = BTreeMap::new();
        match_props.insert("class".into(), serde_json::json!("^firefox$"));
        let mut effects = BTreeMap::new();
        effects.insert("workspace".into(), serde_json::json!("2"));
        effects.insert("float".into(), serde_json::json!(true));

        add_window_rule(&path, "browser-ws", &match_props, &effects).unwrap();
        let updated = std::fs::read_to_string(&path).unwrap();
        assert!(updated.starts_with("-- user rules"));
        assert!(updated.contains("name = \"keep\""));
        assert!(updated.contains("-- >>> hyprbinds:managed-window-rules"));
        assert!(updated.contains("name = \"browser-ws\""));
        assert!(updated.contains("class = \"^firefox$\""));
        assert!(updated.contains("-- <<< hyprbinds:managed-window-rules"));
        let _ = std::fs::remove_file(&path);
        let mut bak = path.as_os_str().to_owned();
        bak.push(".hyprbinds.bak");
        let _ = std::fs::remove_file(PathBuf::from(bak));
    }

    #[test]
    fn adds_monitor_and_config_override() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("hyprbinds-settings-{}.lua", std::process::id()));
        std::fs::write(&path, "hl.config({ general = { gaps_in = 5 } })\n").unwrap();

        let mut fields = BTreeMap::new();
        fields.insert("output".into(), serde_json::json!("DP-1"));
        fields.insert("mode".into(), serde_json::json!("preferred"));
        add_monitor(&path, &fields).unwrap();

        let merged = serde_json::json!({
            "general": { "gaps_in": 8, "layout": "dwindle" },
            "decoration": { "rounding": 12 },
            "cursor": { "zoom_factor": 1.0 }
        });
        save_config_override(&path, &merged).unwrap();

        let updated = std::fs::read_to_string(&path).unwrap();
        assert!(updated.contains("hl.config({ general = { gaps_in = 5 } })"));
        assert!(updated.contains("-- >>> hyprbinds:managed-monitors"));
        assert!(updated.contains("output = \"DP-1\""));
        assert!(updated.contains("-- >>> hyprbinds:managed-config"));
        assert!(
            updated.contains("  general = {\n    gaps_in = 8,\n    layout = \"dwindle\",\n  },"),
            "managed config should nest tables:\n{updated}"
        );
        assert!(updated.contains("  decoration = {\n    rounding = 12,\n  },"));
        assert!(!updated.contains("zoom_factor")); // filtered out of MVP
        let _ = std::fs::remove_file(&path);
        let mut bak = path.as_os_str().to_owned();
        bak.push(".hyprbinds.bak");
        let _ = std::fs::remove_file(PathBuf::from(bak));
    }

    #[test]
    fn adds_env_and_submap_bind() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("hyprbinds-env-submap-{}.lua", std::process::id()));
        std::fs::write(&path, "-- prefix\n").unwrap();

        add_env(&path, "XCURSOR_SIZE", "24").unwrap();
        add_submap(&path, "resize").unwrap();
        add_bind(
            &path,
            "grow right",
            "right",
            "hl.dsp.window.resize({ x = 10, y = 0, relative = true })",
            "resize",
        )
        .unwrap();

        let updated = std::fs::read_to_string(&path).unwrap();
        assert!(updated.starts_with("-- prefix\n"));
        assert!(updated.contains("-- >>> hyprbinds:managed-env"));
        assert!(updated.contains("hl.env(\"XCURSOR_SIZE\", \"24\")"));
        assert!(updated.contains("-- >>> hyprbinds:managed-submaps"));
        assert!(updated.contains("hl.define_submap(\"resize\""));
        assert!(updated.contains("description = \"grow right\""));
        assert!(updated.contains("hl.bind("));
        // bind lives inside the submap function, before end)
        let sub_idx = updated.find("hl.define_submap(\"resize\"").unwrap();
        let bind_idx = updated.find("grow right").unwrap();
        let end_idx = updated[sub_idx..].find("end)").unwrap() + sub_idx;
        assert!(bind_idx > sub_idx && bind_idx < end_idx);

        let _ = std::fs::remove_file(&path);
        let mut bak = path.as_os_str().to_owned();
        bak.push(".hyprbinds.bak");
        let _ = std::fs::remove_file(PathBuf::from(bak));
    }

    #[test]
    fn apply_collection_rewrites_managed_sections() {
        use crate::bind::{BindCollection, Keybind};
        use crate::env::EnvVar;
        use crate::variables::ConfigVariable;
        use std::collections::BTreeMap;

        let dir = std::env::temp_dir();
        let path = dir.join(format!("hyprbinds-apply-{}.lua", std::process::id()));
        std::fs::write(
            &path,
            "-- keep me\nlocal hl = require(\"hyprland\")\nhl.bind(\"SUPER + X\", hl.dsp.window.close())\n",
        )
        .unwrap();

        let collection = BindCollection {
            config_path: path.clone(),
            config_dir: dir.clone(),
            binds: vec![Keybind {
                keys: "SUPER + Q".into(),
                action: "hl.dsp.window.close()".into(),
                flags: vec![],
                description: "close".into(),
                submap: String::new(),
                source_file: String::new(),
                source_line: 0,
                name: "close".into(),
                id: 0,
            }],
            window_rules: vec![],
            workspace_rules: vec![],
            layer_rules: vec![],
            monitors: {
                let mut fields = BTreeMap::new();
                fields.insert("output".into(), serde_json::json!("DP-1"));
                fields.insert("mode".into(), serde_json::json!("preferred"));
                vec![crate::spec::SpecItem {
                    name: String::new(),
                    fields,
                    source_file: String::new(),
                    source_line: 0,
                    id: 0,
                    display_name: "DP-1".into(),
                }]
            },
            devices: vec![],
            gestures: vec![],
            curves: vec![],
            animations: vec![],
            env: vec![EnvVar {
                name: "XCURSOR_SIZE".into(),
                value: "24".into(),
                source_file: String::new(),
                source_line: 0,
            }],
            submaps: vec![],
            startup: vec![],
            config_merged: serde_json::json!({
                "general": { "gaps_in": 10 }
            }),
            variables: vec![ConfigVariable {
                name: "mainMod".into(),
                value: "SUPER".into(),
                source_file: String::new(),
                source_line: 0,
            }],
            files: vec![],
            error: None,
        };

        apply_collection(&path, &collection).unwrap();
        let updated = std::fs::read_to_string(&path).unwrap();
        assert!(updated.contains("-- keep me"));
        assert!(updated.contains("-- >>> hyprbinds:managed-binds"));
        assert!(updated.contains("SUPER + Q") || updated.contains("\"SUPER + Q\""));
        assert!(updated.contains("hl.env(\"XCURSOR_SIZE\", \"24\")"));
        assert!(updated.contains("local mainMod = \"SUPER\""));
        assert!(updated.contains("output = \"DP-1\""));
        assert!(updated.contains("gaps_in = 10"));

        let _ = std::fs::remove_file(&path);
        let mut bak = path.as_os_str().to_owned();
        bak.push(".hyprbinds.bak");
        let _ = std::fs::remove_file(PathBuf::from(bak));
    }

    #[test]
    fn managed_section_entries_separated_by_blank_line() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("hyprbinds-blank-{}.lua", std::process::id()));
        std::fs::write(&path, "-- prefix\n").unwrap();

        add_env(&path, "XCURSOR_SIZE", "24").unwrap();
        add_env(&path, "XCURSOR_THEME", "Adwaita").unwrap();

        let updated = std::fs::read_to_string(&path).unwrap();
        assert!(
            updated.contains(
                "hl.env(\"XCURSOR_SIZE\", \"24\")\n\nhl.env(\"XCURSOR_THEME\", \"Adwaita\")"
            ),
            "expected blank line between env entries:\n{updated}"
        );

        let _ = std::fs::remove_file(&path);
        let mut bak = path.as_os_str().to_owned();
        bak.push(".hyprbinds.bak");
        let _ = std::fs::remove_file(PathBuf::from(bak));
    }
}
