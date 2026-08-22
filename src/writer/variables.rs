//! Variable CRUD: `local <name> = "<value>"` declarations.
//!
//! Saving renames the identifier across related files (string- and
//! comment-aware via `rename_identifier_outside_strings`).

use std::path::{Path, PathBuf};

use crate::keys::lua_string_literal;
use crate::variables::ConfigVariable;
use crate::writer::error::{
    ok_result, ok_result_many, write_config_atomic, WriteError, WriteMode, WriteResult,
};
use crate::writer::locate::line_start_offsets;
use crate::writer::lua_text::{
    is_lua_ident, leading_ws, line_ending, rename_identifier_outside_strings,
};
use crate::writer::section::{
    append_into_section, MANAGED_VARS_BEGIN, MANAGED_VARS_END,
};

pub fn save_variable(
    var: &ConfigVariable,
    new_name: &str,
    new_value: &str,
    related_files: &[PathBuf],
    existing_names: &[String],
) -> Result<WriteResult, WriteError> {
    let new_name = new_name.trim();
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
    let source = std::fs::read_to_string(&decl_path)?;
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
            let text = std::fs::read_to_string(&path)?;
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

    let source = std::fs::read_to_string(config_path)?;
    if source.contains(&format!("local {name} =")) {
        return Err(WriteError::Invalid(format!(
            "variable '{name}' already exists"
        )));
    }

    let decl = format!("local {name} = {}", lua_string_literal(value));
    append_into_section(
        config_path,
        MANAGED_VARS_BEGIN,
        MANAGED_VARS_END,
        &decl,
        WriteMode::Appended,
    )
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

    let source = std::fs::read_to_string(path)?;
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
