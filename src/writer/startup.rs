//! Startup entry CRUD: `hl.exec_cmd(...)` inside `hl.on("hyprland.start" | "shutdown", ...)`
//! wrappers, or top-level for `exec`/`exec-once` (`when == "reload"`).
//!
//! When `when` changes between save calls, the entry is deleted and re-added
//! into the right wrapper.

use std::path::Path;

use crate::keys::lua_string_literal;
use crate::startup::StartupEntry;
use crate::writer::error::{ok_result, write_config_atomic, WriteError, WriteMode, WriteResult};
use crate::writer::locate::find_call_span;
use crate::writer::section::{
    ensure_managed_section, MANAGED_STARTUP_BEGIN, MANAGED_STARTUP_END,
};

pub(crate) fn normalize_when(when: &str) -> Result<&'static str, WriteError> {
    match when.trim().to_ascii_lowercase().as_str() {
        "start" | "once" | "exec-once" => Ok("start"),
        "reload" | "exec" => Ok("reload"),
        "shutdown" | "exit" => Ok("shutdown"),
        other => Err(WriteError::Invalid(format!(
            "unknown startup when '{other}' (use start, reload, or shutdown)"
        ))),
    }
}

pub(crate) fn format_exec_cmd_line(command: &str, workspace: &str, indent: &str) -> String {
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
    let source = std::fs::read_to_string(config_path)?;
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

    let source = std::fs::read_to_string(path)?;
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
    let source = std::fs::read_to_string(path)?;
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
