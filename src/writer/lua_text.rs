//! Lua text utilities used across the writer modules.
//!
//! - `format_keys` / `format_action` build Lua expressions for `hl.bind`
//! - `format_opts` builds the `{ description = "…", … }` table
//! - `rename_identifier_outside_strings` rewrites an identifier, skipping
//!   strings and line comments
//! - `is_lua_ident` / `leading_ws` / `line_ending` are small helpers

use crate::keys::lua_string_literal;
use crate::variables::{substitute_action_template, template_to_lua_expr};
use crate::writer::error::WriteError;

pub(crate) fn format_keys(keys: &str) -> Result<String, WriteError> {
    template_to_lua_expr(keys).map_err(WriteError::Invalid)
}

pub(crate) fn format_action(edited: &str, original: &str) -> Result<String, WriteError> {
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

pub(crate) fn format_opts(flags: &[String], name: &str) -> String {
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

pub(crate) fn is_lua_ident(s: &str) -> bool {
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

pub(crate) fn leading_ws(line: &str) -> &str {
    let trimmed = line.trim_start_matches([' ', '\t']);
    &line[..line.len() - trimmed.len()]
}

pub(crate) fn line_ending(line: &str) -> &str {
    if line.ends_with("\r\n") {
        "\r\n"
    } else if line.ends_with('\n') {
        "\n"
    } else {
        ""
    }
}

pub(crate) fn rename_identifier_outside_strings(source: &str, old: &str, new: &str) -> String {
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
