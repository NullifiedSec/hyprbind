//! Call/span finding in Lua source.
//!
//! Given a source location (line number) and a needle like `hl.bind`, locate
//! the byte-range of the matching call. Also handles Lua bracket balancing,
//! line-start offsets, and full-statement cut ranges for in-place deletes.

use crate::writer::error::WriteError;

pub(crate) fn line_start_offsets(source: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, b) in source.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

pub(crate) fn find_bind_call_span(
    source: &str,
    source_line: u32,
) -> Result<(usize, usize), WriteError> {
    find_call_span(source, source_line, "hl.bind")
}

pub(crate) fn find_call_span(
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

/// Expand a call span to the full statement when the call owns its line
/// (leading whitespace only, or an assignment like `local x = hl.bind(...)`).
pub(crate) fn statement_cut_range(source: &str, start: usize, end: usize) -> (usize, usize) {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
