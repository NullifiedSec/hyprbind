//! Submap CRUD: `hl.define_submap("name", function() … end)` blocks.
//!
//! A new bind can either be inserted into an existing managed submap block or
//! create a new one. `find_define_submap_span` balances `function` / `end`
//! to find the closing `end)` even when binds span multiple lines.

use std::path::Path;

use crate::env::Submap;
use crate::keys::lua_string_literal;
use crate::writer::error::{ok_result, write_config_atomic, WriteError, WriteMode, WriteResult};
use crate::writer::section::{
    append_into_section, ensure_managed_section, insert_into_managed_section,
    MANAGED_SUBMAPS_BEGIN, MANAGED_SUBMAPS_END,
};

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
    let source = std::fs::read_to_string(config_path)?;
    if find_define_submap_span(&source, name).is_some() {
        return Err(WriteError::Invalid(format!(
            "submap '{name}' already exists"
        )));
    }
    let block = format!(
        "hl.define_submap({}, function()\n  -- binds for {name}\nend)",
        lua_string_literal(name)
    );
    append_into_section(
        config_path,
        MANAGED_SUBMAPS_BEGIN,
        MANAGED_SUBMAPS_END,
        &block,
        WriteMode::Appended,
    )
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
    let source = std::fs::read_to_string(path)?;
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
pub(crate) fn insert_bind_into_managed_submap(
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
