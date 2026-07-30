//! Curated top Starship config options (schema-backed paths) + TOML get/set.

use toml_edit::{DocumentMut, Item, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionKind {
    Bool,
    String,
    Integer { min: i64, max: i64 },
}

#[derive(Debug, Clone, Copy)]
pub struct ConfigOption {
    /// Dotted path: `add_newline` or `character.success_symbol`.
    pub path: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub kind: OptionKind,
    pub group: &'static str,
}

/// Thirty high-traffic options from the official Starship schema / docs.
pub const TOP_OPTIONS: &[ConfigOption] = &[
    // —— Prompt globals ——
    ConfigOption {
        path: "format",
        label: "Format",
        hint: "Left prompt format string ($all expands to every module)",
        kind: OptionKind::String,
        group: "Prompt",
    },
    ConfigOption {
        path: "right_format",
        label: "Right format",
        hint: "Right-side prompt format (empty = unused)",
        kind: OptionKind::String,
        group: "Prompt",
    },
    ConfigOption {
        path: "continuation_prompt",
        label: "Continuation prompt",
        hint: "Shown for incomplete statements (multi-line input)",
        kind: OptionKind::String,
        group: "Prompt",
    },
    ConfigOption {
        path: "add_newline",
        label: "Add newline",
        hint: "Insert a blank line between shell prompts",
        kind: OptionKind::Bool,
        group: "Prompt",
    },
    ConfigOption {
        path: "scan_timeout",
        label: "Scan timeout (ms)",
        hint: "Max time to scan files for repo/tool detection",
        kind: OptionKind::Integer {
            min: 1,
            max: 10_000,
        },
        group: "Prompt",
    },
    ConfigOption {
        path: "command_timeout",
        label: "Command timeout (ms)",
        hint: "Max time for module commands before they are skipped",
        kind: OptionKind::Integer {
            min: 1,
            max: 30_000,
        },
        group: "Prompt",
    },
    ConfigOption {
        path: "follow_symlinks",
        label: "Follow symlinks",
        hint: "Follow symlinks when discovering repos and tool roots",
        kind: OptionKind::Bool,
        group: "Prompt",
    },
    ConfigOption {
        path: "palette",
        label: "Palette",
        hint: "Named palette from [palettes] (leave empty for default)",
        kind: OptionKind::String,
        group: "Prompt",
    },
    // —— Character ——
    ConfigOption {
        path: "character.success_symbol",
        label: "Success symbol",
        hint: "Prompt character after a successful command",
        kind: OptionKind::String,
        group: "Character",
    },
    ConfigOption {
        path: "character.error_symbol",
        label: "Error symbol",
        hint: "Prompt character after a failed command",
        kind: OptionKind::String,
        group: "Character",
    },
    ConfigOption {
        path: "character.disabled",
        label: "Disable character",
        hint: "Hide the character module",
        kind: OptionKind::Bool,
        group: "Character",
    },
    // —— Directory ——
    ConfigOption {
        path: "directory.truncation_length",
        label: "Truncation length",
        hint: "How many path components to keep before truncating",
        kind: OptionKind::Integer { min: 0, max: 20 },
        group: "Directory",
    },
    ConfigOption {
        path: "directory.truncate_to_repo",
        label: "Truncate to repo",
        hint: "Truncate path relative to the git root",
        kind: OptionKind::Bool,
        group: "Directory",
    },
    ConfigOption {
        path: "directory.style",
        label: "Directory style",
        hint: "ANSI style string for the path",
        kind: OptionKind::String,
        group: "Directory",
    },
    ConfigOption {
        path: "directory.home_symbol",
        label: "Home symbol",
        hint: "Replacement for $HOME in the path (e.g. ~)",
        kind: OptionKind::String,
        group: "Directory",
    },
    ConfigOption {
        path: "directory.disabled",
        label: "Disable directory",
        hint: "Hide the directory module",
        kind: OptionKind::Bool,
        group: "Directory",
    },
    // —— Git ——
    ConfigOption {
        path: "git_branch.symbol",
        label: "Branch symbol",
        hint: "Symbol shown before the branch name",
        kind: OptionKind::String,
        group: "Git",
    },
    ConfigOption {
        path: "git_branch.style",
        label: "Branch style",
        hint: "ANSI style for the branch module",
        kind: OptionKind::String,
        group: "Git",
    },
    ConfigOption {
        path: "git_branch.disabled",
        label: "Disable git branch",
        hint: "Hide the git_branch module",
        kind: OptionKind::Bool,
        group: "Git",
    },
    ConfigOption {
        path: "git_status.disabled",
        label: "Disable git status",
        hint: "Hide dirty/ahead/behind status indicators",
        kind: OptionKind::Bool,
        group: "Git",
    },
    // —— Duration / layout ——
    ConfigOption {
        path: "cmd_duration.min_time",
        label: "Min duration (ms)",
        hint: "Only show cmd_duration when the command took at least this long",
        kind: OptionKind::Integer {
            min: 0,
            max: 60_000,
        },
        group: "Duration",
    },
    ConfigOption {
        path: "cmd_duration.show_milliseconds",
        label: "Show milliseconds",
        hint: "Include ms precision in cmd_duration",
        kind: OptionKind::Bool,
        group: "Duration",
    },
    ConfigOption {
        path: "cmd_duration.disabled",
        label: "Disable cmd duration",
        hint: "Hide the cmd_duration module",
        kind: OptionKind::Bool,
        group: "Duration",
    },
    ConfigOption {
        path: "line_break.disabled",
        label: "Disable line break",
        hint: "Keep the prompt on a single line",
        kind: OptionKind::Bool,
        group: "Duration",
    },
    // —— Identity ——
    ConfigOption {
        path: "hostname.ssh_only",
        label: "Hostname SSH only",
        hint: "Only show hostname when connected over SSH",
        kind: OptionKind::Bool,
        group: "Identity",
    },
    ConfigOption {
        path: "hostname.disabled",
        label: "Disable hostname",
        hint: "Hide the hostname module",
        kind: OptionKind::Bool,
        group: "Identity",
    },
    ConfigOption {
        path: "username.show_always",
        label: "Username always",
        hint: "Show username even when not root / not SSH",
        kind: OptionKind::Bool,
        group: "Identity",
    },
    ConfigOption {
        path: "username.disabled",
        label: "Disable username",
        hint: "Hide the username module",
        kind: OptionKind::Bool,
        group: "Identity",
    },
    // —— Time ——
    ConfigOption {
        path: "time.disabled",
        label: "Disable time",
        hint: "Hide the time module (off by default)",
        kind: OptionKind::Bool,
        group: "Time",
    },
    ConfigOption {
        path: "time.use_12hr",
        label: "12-hour clock",
        hint: "Use 12-hour time formatting",
        kind: OptionKind::Bool,
        group: "Time",
    },
];

const _: () = assert!(TOP_OPTIONS.len() == 30);

#[derive(Debug, Clone)]
pub enum OptionValue {
    Bool(bool),
    String(String),
    Integer(i64),
    /// Key absent — UI shows schema default placeholder.
    Unset,
}

pub fn parse_document(toml_text: &str) -> Result<DocumentMut, String> {
    toml_text
        .parse::<DocumentMut>()
        .map_err(|e| format!("Invalid TOML: {e}"))
}

pub fn get_option(doc: &DocumentMut, opt: &ConfigOption) -> OptionValue {
    let Some(item) = resolve_item(doc, opt.path) else {
        return OptionValue::Unset;
    };
    match opt.kind {
        OptionKind::Bool => match item.as_bool() {
            Some(v) => OptionValue::Bool(v),
            None => OptionValue::Unset,
        },
        OptionKind::String => match item.as_str() {
            Some(v) => OptionValue::String(v.to_string()),
            None => OptionValue::Unset,
        },
        OptionKind::Integer { .. } => match item.as_integer() {
            Some(v) => OptionValue::Integer(v),
            None => OptionValue::Unset,
        },
    }
}

pub fn set_option(
    doc: &mut DocumentMut,
    opt: &ConfigOption,
    value: OptionValue,
) -> Result<(), String> {
    match (opt.kind, value) {
        (OptionKind::Bool, OptionValue::Bool(v)) => {
            set_item(doc, opt.path, Item::Value(Value::from(v)))
        }
        (OptionKind::String, OptionValue::String(v)) => {
            set_item(doc, opt.path, Item::Value(Value::from(v)))
        }
        (OptionKind::Integer { min, max }, OptionValue::Integer(v)) => {
            let v = v.clamp(min, max);
            set_item(doc, opt.path, Item::Value(Value::from(v)))
        }
        (_, OptionValue::Unset) => remove_item(doc, opt.path),
        _ => Err(format!("Type mismatch for {}", opt.path)),
    }
}

pub fn apply_option_to_toml(
    toml_text: &str,
    opt: &ConfigOption,
    value: OptionValue,
) -> Result<String, String> {
    let mut doc = parse_document(toml_text)?;
    set_option(&mut doc, opt, value)?;
    Ok(doc.to_string())
}

fn path_parts(path: &str) -> Vec<&str> {
    path.split('.').collect()
}

fn resolve_item<'a>(doc: &'a DocumentMut, path: &str) -> Option<&'a Item> {
    let parts = path_parts(path);
    let mut cur: &Item = doc.as_item();
    for part in parts {
        cur = cur.get(part)?;
    }
    Some(cur)
}

fn set_item(doc: &mut DocumentMut, path: &str, item: Item) -> Result<(), String> {
    let parts = path_parts(path);
    if parts.is_empty() {
        return Err("empty option path".into());
    }
    if parts.len() == 1 {
        doc[parts[0]] = item;
        return Ok(());
    }
    // Ensure intermediate tables exist as tables (not dotted inline surprises).
    ensure_table(doc, &parts[..parts.len() - 1])?;
    let key = parts[parts.len() - 1];
    let mut cur = doc.as_table_mut();
    for part in &parts[..parts.len() - 1] {
        cur = cur[part]
            .as_table_mut()
            .ok_or_else(|| format!("expected table at {part}"))?;
    }
    cur[key] = item;
    Ok(())
}

fn ensure_table(doc: &mut DocumentMut, parts: &[&str]) -> Result<(), String> {
    let mut cur = doc.as_table_mut();
    for part in parts {
        if cur.get(part).is_none() {
            cur.insert(part, Item::Table(toml_edit::Table::new()));
        } else if cur[part].is_value() {
            return Err(format!(
                "cannot create table '{part}' — a value already exists there"
            ));
        } else if !cur[part].is_table() {
            // Inline table → promote by replacing with a normal table copying values is complex;
            // treat as error and ask user to fix in raw editor.
            if cur[part].as_inline_table().is_some() {
                return Err(format!(
                    "'{part}' is an inline table — edit this key in the raw Config tab"
                ));
            }
        }
        cur = cur[part]
            .as_table_mut()
            .ok_or_else(|| format!("expected table at {part}"))?;
    }
    Ok(())
}

fn remove_item(doc: &mut DocumentMut, path: &str) -> Result<(), String> {
    let parts = path_parts(path);
    if parts.is_empty() {
        return Ok(());
    }
    if parts.len() == 1 {
        doc.as_table_mut().remove(parts[0]);
        return Ok(());
    }
    let mut cur = doc.as_table_mut();
    for part in &parts[..parts.len() - 1] {
        match cur.get_mut(part).and_then(|i| i.as_table_mut()) {
            Some(t) => cur = t,
            None => return Ok(()),
        }
    }
    cur.remove(parts[parts.len() - 1]);
    Ok(())
}

/// Schema defaults used when a key is unset (for UI display).
pub fn display_default(opt: &ConfigOption) -> String {
    match opt.path {
        "format" => "$all".into(),
        "right_format" => String::new(),
        "continuation_prompt" => "[∙](bright-black) ".into(),
        "add_newline" => "true".into(),
        "scan_timeout" => "30".into(),
        "command_timeout" => "500".into(),
        "follow_symlinks" => "true".into(),
        "palette" => String::new(),
        "character.success_symbol" => "[❯](bold green)".into(),
        "character.error_symbol" => "[❯](bold red)".into(),
        "character.disabled" => "false".into(),
        "directory.truncation_length" => "3".into(),
        "directory.truncate_to_repo" => "true".into(),
        "directory.style" => "cyan bold dimmed".into(),
        "directory.home_symbol" => "~".into(),
        "directory.disabled" => "false".into(),
        "git_branch.symbol" => " ".into(),
        "git_branch.style" => "bold purple".into(),
        "git_branch.disabled" => "false".into(),
        "git_status.disabled" => "false".into(),
        "cmd_duration.min_time" => "2_000".into(),
        "cmd_duration.show_milliseconds" => "false".into(),
        "cmd_duration.disabled" => "false".into(),
        "line_break.disabled" => "false".into(),
        "hostname.ssh_only" => "true".into(),
        "hostname.disabled" => "false".into(),
        "username.show_always" => "false".into(),
        "username.disabled" => "false".into(),
        "time.disabled" => "true".into(),
        "time.use_12hr" => "false".into(),
        _ => String::new(),
    }
}

pub fn effective_bool(doc: &DocumentMut, opt: &ConfigOption) -> bool {
    match get_option(doc, opt) {
        OptionValue::Bool(v) => v,
        _ => display_default(opt) == "true",
    }
}

pub fn effective_string(doc: &DocumentMut, opt: &ConfigOption) -> String {
    match get_option(doc, opt) {
        OptionValue::String(v) => v,
        _ => display_default(opt),
    }
}

pub fn effective_integer(doc: &DocumentMut, opt: &ConfigOption) -> i64 {
    match get_option(doc, opt) {
        OptionValue::Integer(v) => v,
        _ => display_default(opt)
            .replace('_', "")
            .parse()
            .unwrap_or(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_nested_and_read_back() {
        let toml = "add_newline = true\n";
        let out = apply_option_to_toml(
            toml,
            &TOP_OPTIONS
                .iter()
                .find(|o| o.path == "character.success_symbol")
                .unwrap(),
            OptionValue::String("[>](bold green)".into()),
        )
        .unwrap();
        let doc = parse_document(&out).unwrap();
        let opt = TOP_OPTIONS
            .iter()
            .find(|o| o.path == "character.success_symbol")
            .unwrap();
        match get_option(&doc, opt) {
            OptionValue::String(s) => assert_eq!(s, "[>](bold green)"),
            other => panic!("unexpected {other:?}"),
        }
        assert!(out.contains("add_newline = true"));
    }

    #[test]
    fn top_options_count() {
        assert_eq!(TOP_OPTIONS.len(), 30);
    }
}
