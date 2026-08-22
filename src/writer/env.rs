//! Environment variable CRUD: `hl.env("NAME", "VALUE")` calls.

use std::path::Path;

use crate::env::EnvVar;
use crate::keys::lua_string_literal;
use crate::writer::error::{ok_result, write_config_atomic, WriteError, WriteMode, WriteResult};
use crate::writer::locate::find_call_span;
use crate::writer::section::{append_into_section, MANAGED_ENV_BEGIN, MANAGED_ENV_END};

pub fn add_env(config_path: &Path, name: &str, value: &str) -> Result<WriteResult, WriteError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(WriteError::Invalid("env name cannot be empty".into()));
    }
    if !config_path.is_file() {
        return Err(WriteError::Missing(config_path.display().to_string()));
    }
    let source = std::fs::read_to_string(config_path)?;
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
    append_into_section(
        config_path,
        MANAGED_ENV_BEGIN,
        MANAGED_ENV_END,
        &line,
        WriteMode::Appended,
    )
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
    let source = std::fs::read_to_string(path)?;
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
    let source = std::fs::read_to_string(path)?;
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
