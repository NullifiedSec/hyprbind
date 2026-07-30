use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    General,
    Window,
    Workspace,
    Group,
    Cursor,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Window => "Window",
            Self::Workspace => "Workspace",
            Self::Group => "Group",
            Self::Cursor => "Cursor",
        }
    }

    pub fn all() -> &'static [Category] {
        &[
            Self::General,
            Self::Window,
            Self::Workspace,
            Self::Group,
            Self::Cursor,
        ]
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FieldKind {
    Text {
        placeholder: &'static str,
    },
    /// Freeform that often holds {{var}} or a command.
    Command,
    Number {
        min: f64,
        max: f64,
        step: f64,
        default: f64,
    },
    Choice {
        options: &'static [&'static str],
    },
    Bool {
        default: bool,
    },
    /// Workspace selector helper: numeric ID + common relative presets.
    Workspace,
    Direction,
    /// toggle / enable / disable
    Action,
}

#[derive(Debug, Clone, Copy)]
pub struct FieldDef {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: FieldKind,
    pub required: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct DispatcherDef {
    pub id: &'static str,
    pub label: &'static str,
    pub category: Category,
    /// Lua path without trailing args, e.g. `hl.dsp.focus` or `hl.dsp.window.close`
    pub path: &'static str,
    pub fields: &'static [FieldDef],
    /// First field is a single positional argument (string/number), not a table.
    pub positional: bool,
}

#[derive(Debug, Clone)]
pub struct ParsedAction {
    pub dispatcher_id: String,
    pub values: HashMap<String, String>,
    #[allow(dead_code)]
    pub raw: String,
}

pub fn catalog() -> &'static [DispatcherDef] {
    &CATALOG
}

pub fn find(id: &str) -> Option<&'static DispatcherDef> {
    CATALOG.iter().find(|d| d.id == id)
}

pub fn by_category(category: Category) -> Vec<&'static DispatcherDef> {
    CATALOG.iter().filter(|d| d.category == category).collect()
}

pub fn generate(def: &DispatcherDef, values: &HashMap<String, String>) -> String {
    if def.id == "playerctl" {
        return generate_playerctl(values);
    }
    if def.positional {
        let first = def.fields.first().map(|f| f.key).unwrap_or("value");
        let value = values.get(first).map(String::as_str).unwrap_or("");
        if def.fields.len() <= 1 {
            return format!("{}({})", def.path, format_positional(value));
        }
        // positional + optional trailing table fields
        let mut table = Vec::new();
        for field in def.fields.iter().skip(1) {
            if let Some(v) = values.get(field.key) {
                if !v.is_empty() {
                    table.push(format!("{} = {}", field.key, format_table_value(v, field)));
                }
            }
        }
        if table.is_empty() {
            format!("{}({})", def.path, format_positional(value))
        } else {
            format!(
                "{}({}, {{ {} }})",
                def.path,
                format_positional(value),
                table.join(", ")
            )
        }
    } else if def.fields.is_empty() {
        format!("{}()", def.path)
    } else {
        let mut table = Vec::new();
        for field in def.fields {
            if let Some(v) = values.get(field.key) {
                if v.is_empty() && !field.required {
                    continue;
                }
                if !v.is_empty() || field.required {
                    if v.is_empty() {
                        continue;
                    }
                    table.push(format!("{} = {}", field.key, format_table_value(v, field)));
                }
            } else if field.required {
                // skip missing required → still emit what we have
            }
        }
        if table.is_empty() {
            format!("{}()", def.path)
        } else {
            format!("{}({{ {} }})", def.path, table.join(", "))
        }
    }
}

pub fn parse(action: &str) -> Option<ParsedAction> {
    let action = action.trim();
    if !action.starts_with("hl.dsp.") {
        return None;
    }

    let open = action.find('(')?;
    let path = &action[..open];
    let close = action.rfind(')')?;
    let inside = action[open + 1..close].trim();

    // Prefer Playerctl when exec_cmd wraps a playerctl invocation.
    if path == "hl.dsp.exec_cmd" {
        if let Some(cmd) = first_positional_string(inside)
            .or_else(|| {
                if !inside.is_empty() && !inside.starts_with('{') {
                    Some(strip_quotes(inside).to_string())
                } else {
                    None
                }
            })
        {
            if let Some(parsed) = parse_playerctl_cmd(&cmd) {
                return Some(ParsedAction {
                    dispatcher_id: "playerctl".into(),
                    values: parsed,
                    raw: action.to_string(),
                });
            }
        }
    }

    // Match exact path + best field signature.
    let candidates: Vec<_> = CATALOG
        .iter()
        .filter(|d| d.path == path && d.id != "playerctl")
        .collect();
    if candidates.is_empty() {
        return None;
    }

    let values = parse_args(inside);
    let def = pick_candidate(&candidates, &values)?;
    let mut map = HashMap::new();

    if def.positional {
        if let Some(FieldKind::Number { .. }) = def.fields.first().map(|f| f.kind) {
            if let Some(v) = first_positional_number(inside) {
                map.insert(def.fields[0].key.to_string(), v);
            }
        } else if let Some(v) = first_positional_string(inside) {
            map.insert(def.fields[0].key.to_string(), v);
        } else if !inside.is_empty() && !inside.starts_with('{') {
            map.insert(
                def.fields[0].key.to_string(),
                strip_quotes(inside).to_string(),
            );
        }
        // also merge table if present after comma
        for (k, v) in values {
            if def.fields.iter().any(|f| f.key == k) {
                map.insert(k, v);
            }
        }
    } else {
        for (k, v) in values {
            if def.fields.iter().any(|f| f.key == k) {
                map.insert(k, v);
            }
        }
    }

    Some(ParsedAction {
        dispatcher_id: def.id.to_string(),
        values: map,
        raw: action.to_string(),
    })
}

/// Human-readable summary of a dispatcher action, e.g. "Move focus right".
/// Returns `None` for non-`hl.dsp.*` actions (Lua functions, unknown paths).
pub fn describe_action(action: &str) -> Option<String> {
    describe_action_at_depth(action, 1)
}

/// Like [`describe_action`], but for `exec_*` commands includes up to `depth`
/// argument tokens so colliding labels can be disambiguated.
pub fn describe_action_at_depth(action: &str, depth: usize) -> Option<String> {
    let action = action.trim();
    if action.is_empty() || action.starts_with('<') {
        return None;
    }
    let parsed = parse(action)?;
    Some(describe_parsed_at_depth(&parsed, depth))
}

/// Dispatcher category for an action, if it parses as a known `hl.dsp.*` call.
pub fn action_category(action: &str) -> Option<Category> {
    let parsed = parse(action.trim())?;
    find(&parsed.dispatcher_id).map(|d| d.category)
}

fn describe_parsed_at_depth(parsed: &ParsedAction, depth: usize) -> String {
    let v = |key: &str| parsed.values.get(key).map(String::as_str);
    let dir = || v("direction").map(pretty_direction).unwrap_or_else(|| "…".into());
    let ws = || v("workspace").map(pretty_workspace).unwrap_or_else(|| "workspace".into());
    let mon = || v("monitor").unwrap_or("monitor");
    let win = || v("window");
    let act = || v("action").map(pretty_action);
    let follow = || v("follow").is_some_and(is_truthy);

    match parsed.dispatcher_id.as_str() {
        "exec_cmd" | "exec_raw" => match v("cmd").map(|c| short_command_at_depth(c, depth)) {
            Some(cmd) if !cmd.is_empty() => format!("Run {cmd}"),
            _ => "Run command".into(),
        },
        "playerctl" => {
            let action = v("action").unwrap_or("Play/Pause");
            let player = v("player").filter(|p| !p.is_empty());
            let all = v("all_players").is_some_and(is_truthy);
            match (all, player) {
                (true, _) => format!("Playerctl {action} (all)"),
                (_, Some(p)) => format!("Playerctl {action} ({p})"),
                _ => format!("Playerctl {action}"),
            }
        },
        "focus.direction" => format!("Move focus {}", dir()),
        "focus.workspace" => {
            let base = format!("Focus {}", ws());
            if v("on_current_monitor").is_some_and(is_truthy) {
                format!("{base} (this monitor)")
            } else {
                base
            }
        }
        "focus.monitor" => format!("Focus monitor {}", mon()),
        "focus.window" => match win() {
            Some(w) if !w.is_empty() => format!("Focus window {w}"),
            _ => "Focus window".into(),
        },
        "focus.last" => "Focus last window".into(),
        "exit" => "Exit Hyprland".into(),
        "submap" => match v("name") {
            Some(name) if !name.is_empty() => format!("Enter submap “{name}”"),
            _ => "Enter submap".into(),
        },
        "layout" => match v("message") {
            Some(msg) if !msg.is_empty() => format!("Layout: {msg}"),
            _ => "Layout message".into(),
        },
        "dpms" => match act() {
            Some(a) => format!("DPMS {a}"),
            None => "Toggle DPMS".into(),
        },
        "pass" => match win() {
            Some(w) if !w.is_empty() => format!("Pass shortcut to {w}"),
            _ => "Pass shortcut".into(),
        },
        "global" => match v("name") {
            Some(name) if !name.is_empty() => format!("Global shortcut “{name}”"),
            _ => "Global shortcut".into(),
        },
        "no_op" => "No-op".into(),
        "send_shortcut" => {
            let mods = v("mods").unwrap_or("");
            let key = v("key").unwrap_or("?");
            let chord = if mods.is_empty() {
                key.to_string()
            } else {
                format!("{mods} + {key}")
            };
            match win() {
                Some(w) if !w.is_empty() => format!("Send {chord} to {w}"),
                _ => format!("Send {chord}"),
            }
        }
        "force_idle" => match v("seconds") {
            Some(s) if !s.is_empty() => format!("Force idle ({s}s)"),
            _ => "Force idle".into(),
        },
        "event" => match v("name") {
            Some(name) if !name.is_empty() => format!("Emit event “{name}”"),
            _ => "Emit event".into(),
        },
        "window.close" => "Close window".into(),
        "window.kill" => "Kill window".into(),
        "window.center" => "Center window".into(),
        "window.float" => match act() {
            Some(a) => format!("{} floating", capitalize(&a)),
            None => "Toggle floating".into(),
        },
        "window.pseudo" => match act() {
            Some(a) => format!("{} pseudo-tile", capitalize(&a)),
            None => "Toggle pseudo-tile".into(),
        },
        "window.pin" => match act() {
            Some(a) => format!("{} pin", capitalize(&a)),
            None => "Toggle pin".into(),
        },
        "window.fullscreen" => {
            let mode = v("mode").unwrap_or("");
            let mode_bit = if mode.is_empty() {
                String::new()
            } else {
                format!(" ({mode})")
            };
            match act() {
                Some(a) => format!("{} fullscreen{mode_bit}", capitalize(&a)),
                None => format!("Toggle fullscreen{mode_bit}"),
            }
        }
        "window.move.direction" => format!("Move window {}", dir()),
        "window.move.workspace" => {
            let base = format!("Move window to {}", ws());
            if follow() {
                format!("{base} and follow")
            } else {
                base
            }
        }
        "window.move.monitor" => {
            let base = format!("Move window to monitor {}", mon());
            if follow() {
                format!("{base} and follow")
            } else {
                base
            }
        }
        "window.swap.direction" => format!("Swap window {}", dir()),
        "window.cycle_next" => {
            let next = v("next").map(is_truthy).unwrap_or(true);
            let mut parts = vec![if next { "Cycle next window" } else { "Cycle previous window" }
                .to_string()];
            if v("tiled").is_some_and(is_truthy) {
                parts.push("tiled".into());
            }
            if v("floating").is_some_and(is_truthy) {
                parts.push("floating".into());
            }
            if parts.len() == 1 {
                parts[0].clone()
            } else {
                format!("{} ({})", parts[0], parts[1..].join(", "))
            }
        }
        "window.drag" => "Drag window".into(),
        "window.resize.mouse" => "Resize window (mouse)".into(),
        "window.resize.amount" => {
            let x = v("x").unwrap_or("0");
            let y = v("y").unwrap_or("0");
            if v("relative").is_some_and(is_truthy) || v("relative").is_none() {
                format!("Resize window by {x}×{y}")
            } else {
                format!("Resize window to {x}×{y}")
            }
        }
        "window.tag" => match v("tag") {
            Some(tag) if !tag.is_empty() => format!("Tag window “{tag}”"),
            _ => "Tag window".into(),
        },
        "workspace.toggle_special" => match v("name") {
            Some(name) if !name.is_empty() => format!("Toggle special workspace “{name}”"),
            _ => "Toggle special workspace".into(),
        },
        "workspace.rename" => match (v("workspace"), v("name")) {
            (Some(ws), Some(name)) if !name.is_empty() => {
                format!("Rename workspace {ws} to “{name}”")
            }
            (Some(ws), _) => format!("Rename workspace {ws}"),
            _ => "Rename workspace".into(),
        },
        "workspace.move" => match v("workspace") {
            Some(ws) if !ws.is_empty() => {
                format!("Move workspace {ws} to monitor {}", mon())
            }
            _ => format!("Move workspace to monitor {}", mon()),
        },
        "group.toggle" => "Toggle group".into(),
        "group.next" => "Next window in group".into(),
        "group.prev" => "Previous window in group".into(),
        "group.lock" => match act() {
            Some(a) => format!("{} group lock", capitalize(&a)),
            None => "Toggle group lock".into(),
        },
        "cursor.move" => {
            let x = v("x").unwrap_or("0");
            let y = v("y").unwrap_or("0");
            format!("Move cursor by {x}, {y}")
        }
        "cursor.move_to_corner" => match v("corner") {
            Some(c) => format!("Move cursor to corner {c}"),
            None => "Move cursor to corner".into(),
        },
        _ => find(&parsed.dispatcher_id)
            .map(|d| d.label.to_string())
            .unwrap_or_else(|| parsed.dispatcher_id.clone()),
    }
}

fn pretty_direction(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "l" | "left" => "left".into(),
        "r" | "right" => "right".into(),
        "u" | "up" => "up".into(),
        "d" | "down" => "down".into(),
        other => other.to_string(),
    }
}

fn pretty_workspace(raw: &str) -> String {
    let t = raw.trim();
    match t.to_ascii_lowercase().as_str() {
        "m+1" | "+1" => "next workspace".into(),
        "m-1" | "-1" => "previous workspace".into(),
        "r+1" => "next empty workspace".into(),
        "r-1" => "previous empty workspace".into(),
        "e+1" => "next existing workspace".into(),
        "e-1" => "previous existing workspace".into(),
        "special" => "special workspace".into(),
        "previous" | "prev" => "previous workspace".into(),
        other if other.chars().all(|c| c.is_ascii_digit()) => format!("workspace {other}"),
        other => format!("workspace {other}"),
    }
}

fn pretty_action(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "toggle" => "toggle".into(),
        "enable" | "on" | "set" => "enable".into(),
        "disable" | "off" | "unset" => "disable".into(),
        other => other.to_string(),
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn is_truthy(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "true" | "1" | "yes" | "on"
    )
}

/// Summarize a command using the first `depth` tokens (min 1).
/// The first token (and any path-like token) uses its basename; if more
/// tokens remain, a trailing `…` is appended.
fn short_command_at_depth(cmd: &str, depth: usize) -> String {
    let cmd = cmd.trim().trim_matches('"').trim_matches('\'');
    if cmd.is_empty() {
        return String::new();
    }
    // Bare Lua ident / {{var}} → keep readable
    if cmd.starts_with("{{") && cmd.ends_with("}}") {
        return cmd[2..cmd.len() - 2].trim().to_string();
    }
    let tokens: Vec<&str> = cmd.split_whitespace().collect();
    if tokens.is_empty() {
        return String::new();
    }
    let depth = depth.max(1).min(tokens.len());
    let mut parts = Vec::with_capacity(depth);
    for (i, tok) in tokens.iter().take(depth).enumerate() {
        if i == 0 || tok.contains('/') {
            let base = std::path::Path::new(tok)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(tok);
            parts.push(base);
        } else {
            parts.push(tok);
        }
    }
    let joined = parts.join(" ");
    if depth < tokens.len() {
        format!("{joined}…")
    } else {
        joined
    }
}

fn pick_candidate<'a>(
    candidates: &[&'a DispatcherDef],
    values: &HashMap<String, String>,
) -> Option<&'a DispatcherDef> {
    if candidates.len() == 1 {
        return Some(candidates[0]);
    }
    // Prefer the candidate whose required/primary keys are present.
    let mut best: Option<(&DispatcherDef, usize)> = None;
    for def in candidates {
        let score = def
            .fields
            .iter()
            .filter(|f| values.contains_key(f.key))
            .count();
        if best.is_none_or(|(_, s)| score > s) {
            best = Some((def, score));
        }
    }
    best.map(|(d, _)| d).or_else(|| candidates.first().copied())
}

fn parse_args(inside: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let table = if let Some(start) = inside.find('{') {
        let end = inside.rfind('}').unwrap_or(inside.len());
        inside[start + 1..end].trim()
    } else {
        return map;
    };

    for part in split_top_level(table) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((k, v)) = part.split_once('=') {
            let key = k.trim().to_string();
            let value = strip_quotes(v.trim()).to_string();
            map.insert(key, value);
        }
    }
    map
}

fn split_top_level(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    let mut in_str: Option<char> = None;
    for ch in s.chars() {
        if let Some(q) = in_str {
            cur.push(ch);
            if ch == q {
                in_str = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' => {
                in_str = Some(ch);
                cur.push(ch);
            }
            '{' | '(' => {
                depth += 1;
                cur.push(ch);
            }
            '}' | ')' => {
                depth -= 1;
                cur.push(ch);
            }
            ',' if depth == 0 => {
                parts.push(std::mem::take(&mut cur));
            }
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() {
        parts.push(cur);
    }
    parts
}

fn first_positional_string(inside: &str) -> Option<String> {
    let trimmed = inside.trim();
    if trimmed.starts_with('{') {
        return None;
    }
    if let Some(rest) = trimmed.strip_prefix('"') {
        let end = rest.find('"')?;
        return Some(rest[..end].to_string());
    }
    if let Some(rest) = trimmed.strip_prefix('\'') {
        let end = rest.find('\'')?;
        return Some(rest[..end].to_string());
    }
    // bare identifier / {{var}}
    let token = trimmed.split(',').next()?.trim();
    if token.starts_with("{{") || token.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Some(token.to_string());
    }
    None
}

fn first_positional_number(inside: &str) -> Option<String> {
    let token = inside.split(',').next()?.trim();
    if token.parse::<f64>().is_ok() {
        Some(token.to_string())
    } else {
        None
    }
}

fn strip_quotes(s: &str) -> &str {
    if (s.starts_with('"') && s.ends_with('"') && s.len() >= 2)
        || (s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2)
    {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

fn format_positional(value: &str) -> String {
    let v = value.trim();
    if v.starts_with("{{") && v.ends_with("}}") {
        // {{terminal}} → terminal
        let inner = &v[2..v.len() - 2];
        return inner.trim().to_string();
    }
    if v.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && !v.is_empty() {
        // bare variable identifier
        if v.parse::<f64>().is_ok() {
            return v.to_string();
        }
        // could be variable or command word; quote if it looks like a command with spaces already handled
        // Prefer quoting unless it's a known-looking ident used as var - for exec_cmd, bare idents are vars.
        return v.to_string();
    }
    if v.parse::<f64>().is_ok() {
        return v.to_string();
    }
    format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\""))
}

fn format_table_value(value: &str, field: &FieldDef) -> String {
    let v = value.trim();
    match field.kind {
        FieldKind::Number { .. } => {
            if v.parse::<f64>().is_ok() {
                v.to_string()
            } else {
                format!("\"{}\"", v.replace('"', "\\\""))
            }
        }
        FieldKind::Bool { .. } => {
            if matches!(v, "true" | "false") {
                v.to_string()
            } else {
                format!("\"{v}\"")
            }
        }
        FieldKind::Workspace => {
            if v.parse::<i64>().is_ok() {
                v.to_string()
            } else if v.starts_with("{{") {
                format_positional(v)
            } else {
                format!("\"{}\"", v.replace('"', "\\\""))
            }
        }
        _ => {
            if v.starts_with("{{") {
                format_positional(v)
            } else if matches!(v, "true" | "false") || v.parse::<f64>().is_ok() {
                v.to_string()
            } else {
                format!("\"{}\"", v.replace('"', "\\\""))
            }
        }
    }
}

macro_rules! fields {
    ($($item:expr),* $(,)?) => {{
        const FIELDS: &[FieldDef] = &[$($item),*];
        FIELDS
    }};
}

const DIR: FieldDef = FieldDef {
    key: "direction",
    label: "Direction",
    kind: FieldKind::Direction,
    required: true,
};
const ACTION: FieldDef = FieldDef {
    key: "action",
    label: "Action",
    kind: FieldKind::Action,
    required: false,
};
const FOLLOW: FieldDef = FieldDef {
    key: "follow",
    label: "Follow",
    kind: FieldKind::Bool { default: true },
    required: false,
};
const WINDOW: FieldDef = FieldDef {
    key: "window",
    label: "Window",
    kind: FieldKind::Text {
        placeholder: "activewindow / class:…",
    },
    required: false,
};

/// Dropdown labels for Playerctl actions (mapped to CLI args in generate/parse).
const PLAYERCTL_ACTION_OPTIONS: &[&str] = &[
    "Play/Pause",
    "Play",
    "Pause",
    "Stop",
    "Next",
    "Previous",
    "Volume up",
    "Volume down",
    "Seek forward",
    "Seek back",
    "Shuffle on",
    "Shuffle off",
    "Loop none",
    "Loop track",
    "Loop playlist",
];

fn playerctl_cli_for_action(action: &str) -> &'static str {
    match action {
        "Play/Pause" => "play-pause",
        "Play" => "play",
        "Pause" => "pause",
        "Stop" => "stop",
        "Next" => "next",
        "Previous" => "previous",
        "Volume up" => "volume 0.05+",
        "Volume down" => "volume 0.05-",
        "Seek forward" => "position 5+",
        "Seek back" => "position 5-",
        "Shuffle on" => "shuffle On",
        "Shuffle off" => "shuffle Off",
        "Loop none" => "loop None",
        "Loop track" => "loop Track",
        "Loop playlist" => "loop Playlist",
        _ => "play-pause",
    }
}

fn playerctl_action_for_cli(cli: &str) -> Option<&'static str> {
    let cli = cli.trim();
    for label in PLAYERCTL_ACTION_OPTIONS {
        if playerctl_cli_for_action(label) == cli {
            return Some(*label);
        }
    }
    // Accept raw playerctl tokens as well.
    Some(match cli {
        "play-pause" => "Play/Pause",
        "play" => "Play",
        "pause" => "Pause",
        "stop" => "Stop",
        "next" => "Next",
        "previous" | "prev" => "Previous",
        "volume 0.05+" | "volume 0.1+" => "Volume up",
        "volume 0.05-" | "volume 0.1-" => "Volume down",
        "position 5+" | "position 10+" => "Seek forward",
        "position 5-" | "position 10-" => "Seek back",
        "shuffle On" | "shuffle on" => "Shuffle on",
        "shuffle Off" | "shuffle off" => "Shuffle off",
        "loop None" | "loop none" => "Loop none",
        "loop Track" | "loop track" => "Loop track",
        "loop Playlist" | "loop playlist" => "Loop playlist",
        _ => return None,
    })
}

fn generate_playerctl(values: &HashMap<String, String>) -> String {
    let action = values
        .get("action")
        .map(String::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("Play/Pause");
    let cli = playerctl_cli_for_action(action);
    let mut parts = vec!["playerctl".to_string()];
    if values.get("all_players").is_some_and(|v| is_truthy(v)) {
        parts.push("--all-players".into());
    } else if let Some(player) = values.get("player").map(String::as_str).filter(|p| !p.is_empty())
    {
        parts.push(format!("--player={player}"));
    }
    parts.push(cli.to_string());
    let cmd = parts.join(" ");
    format!("hl.dsp.exec_cmd({})", format_positional(&cmd))
}

fn parse_playerctl_cmd(cmd: &str) -> Option<HashMap<String, String>> {
    let cmd = cmd.trim();
    if !cmd.starts_with("playerctl") {
        return None;
    }
    let rest = cmd["playerctl".len()..].trim();
    if rest.is_empty() {
        return None;
    }

    let mut tokens = shell_like_tokens(rest);
    if tokens.is_empty() {
        return None;
    }

    let mut map = HashMap::new();
    let mut all_players = false;
    let mut player = String::new();

    while let Some(tok) = tokens.first() {
        if tok == "--all-players" || tok == "-a" {
            all_players = true;
            tokens.remove(0);
            continue;
        }
        if let Some(name) = tok.strip_prefix("--player=") {
            player = name.to_string();
            tokens.remove(0);
            continue;
        }
        if tok == "--player" || tok == "-p" {
            tokens.remove(0);
            if let Some(name) = tokens.first().cloned() {
                player = name;
                tokens.remove(0);
            }
            continue;
        }
        break;
    }

    if tokens.is_empty() {
        return None;
    }
    let cli = tokens.join(" ");
    let action = playerctl_action_for_cli(&cli)?;
    map.insert("action".into(), action.to_string());
    if all_players {
        map.insert("all_players".into(), "true".into());
    }
    if !player.is_empty() {
        map.insert("player".into(), player);
    }
    Some(map)
}

/// Split on whitespace while keeping simple quoted segments intact.
fn shell_like_tokens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes: Option<char> = None;
    for ch in s.chars() {
        if let Some(q) = in_quotes {
            if ch == q {
                in_quotes = None;
            } else {
                cur.push(ch);
            }
            continue;
        }
        match ch {
            '"' | '\'' => in_quotes = Some(ch),
            c if c.is_whitespace() => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

static CATALOG: [DispatcherDef; 43] = [
    // General
    DispatcherDef {
        id: "exec_cmd",
        label: "Run command",
        category: Category::General,
        path: "hl.dsp.exec_cmd",
        fields: fields![FieldDef {
            key: "cmd",
            label: "Command",
            kind: FieldKind::Command,
            required: true,
        }],
        positional: true,
    },
    DispatcherDef {
        id: "exec_raw",
        label: "Run raw command",
        category: Category::General,
        path: "hl.dsp.exec_raw",
        fields: fields![FieldDef {
            key: "cmd",
            label: "Command",
            kind: FieldKind::Command,
            required: true,
        }],
        positional: true,
    },
    DispatcherDef {
        id: "playerctl",
        label: "Playerctl",
        category: Category::General,
        path: "hl.dsp.exec_cmd",
        fields: fields![
            FieldDef {
                key: "action",
                label: "Action",
                kind: FieldKind::Choice {
                    options: PLAYERCTL_ACTION_OPTIONS,
                },
                required: true,
            },
            FieldDef {
                key: "player",
                label: "Player",
                kind: FieldKind::Text {
                    placeholder: "spotify / vlc / %any (optional)",
                },
                required: false,
            },
            FieldDef {
                key: "all_players",
                label: "All players",
                kind: FieldKind::Bool { default: false },
                required: false,
            },
        ],
        positional: false,
    },
    DispatcherDef {
        id: "focus.direction",
        label: "Focus (direction)",
        category: Category::General,
        path: "hl.dsp.focus",
        fields: fields![DIR],
        positional: false,
    },
    DispatcherDef {
        id: "focus.workspace",
        label: "Focus (workspace)",
        category: Category::General,
        path: "hl.dsp.focus",
        fields: fields![
            FieldDef {
                key: "workspace",
                label: "Workspace",
                kind: FieldKind::Workspace,
                required: true,
            },
            FieldDef {
                key: "on_current_monitor",
                label: "Current monitor only",
                kind: FieldKind::Bool { default: false },
                required: false,
            },
        ],
        positional: false,
    },
    DispatcherDef {
        id: "focus.monitor",
        label: "Focus (monitor)",
        category: Category::General,
        path: "hl.dsp.focus",
        fields: fields![FieldDef {
            key: "monitor",
            label: "Monitor",
            kind: FieldKind::Text {
                placeholder: "current / +1 / DP-1",
            },
            required: true,
        }],
        positional: false,
    },
    DispatcherDef {
        id: "focus.window",
        label: "Focus (window)",
        category: Category::General,
        path: "hl.dsp.focus",
        fields: fields![FieldDef {
            key: "window",
            label: "Window",
            kind: FieldKind::Text {
                placeholder: "class:firefox",
            },
            required: true,
        }],
        positional: false,
    },
    DispatcherDef {
        id: "focus.last",
        label: "Focus last window",
        category: Category::General,
        path: "hl.dsp.focus",
        fields: fields![FieldDef {
            key: "last",
            label: "Last",
            kind: FieldKind::Bool { default: true },
            required: true,
        }],
        positional: false,
    },
    DispatcherDef {
        id: "exit",
        label: "Exit Hyprland",
        category: Category::General,
        path: "hl.dsp.exit",
        fields: &[],
        positional: false,
    },
    DispatcherDef {
        id: "submap",
        label: "Enter submap",
        category: Category::General,
        path: "hl.dsp.submap",
        fields: fields![FieldDef {
            key: "name",
            label: "Submap",
            kind: FieldKind::Text {
                placeholder: "resize / reset",
            },
            required: true,
        }],
        positional: true,
    },
    DispatcherDef {
        id: "layout",
        label: "Layout message",
        category: Category::General,
        path: "hl.dsp.layout",
        fields: fields![FieldDef {
            key: "message",
            label: "Message",
            kind: FieldKind::Choice {
                options: &["togglesplit", "swapsplit", "orientleft", "orientright", "orientup", "orientdown"],
            },
            required: true,
        }],
        positional: true,
    },
    DispatcherDef {
        id: "dpms",
        label: "DPMS",
        category: Category::General,
        path: "hl.dsp.dpms",
        fields: fields![ACTION],
        positional: false,
    },
    DispatcherDef {
        id: "pass",
        label: "Pass shortcut",
        category: Category::General,
        path: "hl.dsp.pass",
        fields: fields![WINDOW],
        positional: false,
    },
    DispatcherDef {
        id: "global",
        label: "Global shortcut",
        category: Category::General,
        path: "hl.dsp.global",
        fields: fields![FieldDef {
            key: "name",
            label: "Shortcut",
            kind: FieldKind::Text {
                placeholder: "app:action",
            },
            required: true,
        }],
        positional: true,
    },
    DispatcherDef {
        id: "no_op",
        label: "No-op",
        category: Category::General,
        path: "hl.dsp.no_op",
        fields: &[],
        positional: false,
    },
    // Window
    DispatcherDef {
        id: "window.close",
        label: "Close window",
        category: Category::Window,
        path: "hl.dsp.window.close",
        fields: fields![WINDOW],
        positional: false,
    },
    DispatcherDef {
        id: "window.kill",
        label: "Kill window",
        category: Category::Window,
        path: "hl.dsp.window.kill",
        fields: fields![WINDOW],
        positional: false,
    },
    DispatcherDef {
        id: "window.float",
        label: "Float window",
        category: Category::Window,
        path: "hl.dsp.window.float",
        fields: fields![ACTION, WINDOW],
        positional: false,
    },
    DispatcherDef {
        id: "window.fullscreen",
        label: "Fullscreen",
        category: Category::Window,
        path: "hl.dsp.window.fullscreen",
        fields: fields![
            FieldDef {
                key: "mode",
                label: "Mode",
                kind: FieldKind::Choice {
                    options: &["fullscreen", "maximized", "0", "1", "2"],
                },
                required: false,
            },
            ACTION,
            FieldDef {
                key: "layout_aware",
                label: "Layout aware",
                kind: FieldKind::Bool { default: true },
                required: false,
            },
            WINDOW,
        ],
        positional: false,
    },
    DispatcherDef {
        id: "window.pseudo",
        label: "Pseudo-tile",
        category: Category::Window,
        path: "hl.dsp.window.pseudo",
        fields: fields![ACTION, WINDOW],
        positional: false,
    },
    DispatcherDef {
        id: "window.move.direction",
        label: "Move window (direction)",
        category: Category::Window,
        path: "hl.dsp.window.move",
        fields: fields![
            DIR,
            FieldDef {
                key: "group_aware",
                label: "Group aware",
                kind: FieldKind::Bool { default: false },
                required: false,
            },
            WINDOW,
        ],
        positional: false,
    },
    DispatcherDef {
        id: "window.move.workspace",
        label: "Move window (workspace)",
        category: Category::Window,
        path: "hl.dsp.window.move",
        fields: fields![
            FieldDef {
                key: "workspace",
                label: "Workspace",
                kind: FieldKind::Workspace,
                required: true,
            },
            FOLLOW,
            WINDOW,
        ],
        positional: false,
    },
    DispatcherDef {
        id: "window.move.monitor",
        label: "Move window (monitor)",
        category: Category::Window,
        path: "hl.dsp.window.move",
        fields: fields![
            FieldDef {
                key: "monitor",
                label: "Monitor",
                kind: FieldKind::Text {
                    placeholder: "+1 / DP-1",
                },
                required: true,
            },
            FOLLOW,
            WINDOW,
        ],
        positional: false,
    },
    DispatcherDef {
        id: "window.swap.direction",
        label: "Swap window",
        category: Category::Window,
        path: "hl.dsp.window.swap",
        fields: fields![DIR],
        positional: false,
    },
    DispatcherDef {
        id: "window.center",
        label: "Center window",
        category: Category::Window,
        path: "hl.dsp.window.center",
        fields: fields![WINDOW],
        positional: false,
    },
    DispatcherDef {
        id: "window.cycle_next",
        label: "Cycle windows",
        category: Category::Window,
        path: "hl.dsp.window.cycle_next",
        fields: fields![
            FieldDef {
                key: "next",
                label: "Next",
                kind: FieldKind::Bool { default: true },
                required: false,
            },
            FieldDef {
                key: "tiled",
                label: "Tiled only",
                kind: FieldKind::Bool { default: false },
                required: false,
            },
            FieldDef {
                key: "floating",
                label: "Floating only",
                kind: FieldKind::Bool { default: false },
                required: false,
            },
        ],
        positional: false,
    },
    DispatcherDef {
        id: "window.pin",
        label: "Pin window",
        category: Category::Window,
        path: "hl.dsp.window.pin",
        fields: fields![ACTION, WINDOW],
        positional: false,
    },
    DispatcherDef {
        id: "window.drag",
        label: "Drag window (mouse)",
        category: Category::Window,
        path: "hl.dsp.window.drag",
        fields: &[],
        positional: false,
    },
    DispatcherDef {
        id: "window.resize.mouse",
        label: "Resize window (mouse)",
        category: Category::Window,
        path: "hl.dsp.window.resize",
        fields: &[],
        positional: false,
    },
    DispatcherDef {
        id: "window.resize.amount",
        label: "Resize window (amount)",
        category: Category::Window,
        path: "hl.dsp.window.resize",
        fields: fields![
            FieldDef {
                key: "x",
                label: "Width Δ",
                kind: FieldKind::Number {
                    min: -500.0,
                    max: 500.0,
                    step: 10.0,
                    default: 20.0,
                },
                required: true,
            },
            FieldDef {
                key: "y",
                label: "Height Δ",
                kind: FieldKind::Number {
                    min: -500.0,
                    max: 500.0,
                    step: 10.0,
                    default: 0.0,
                },
                required: true,
            },
            FieldDef {
                key: "relative",
                label: "Relative",
                kind: FieldKind::Bool { default: true },
                required: false,
            },
            WINDOW,
        ],
        positional: false,
    },
    DispatcherDef {
        id: "window.tag",
        label: "Tag window",
        category: Category::Window,
        path: "hl.dsp.window.tag",
        fields: fields![
            FieldDef {
                key: "tag",
                label: "Tag",
                kind: FieldKind::Text { placeholder: "mytag" },
                required: true,
            },
            WINDOW,
        ],
        positional: false,
    },
    // Workspace
    DispatcherDef {
        id: "workspace.toggle_special",
        label: "Toggle special workspace",
        category: Category::Workspace,
        path: "hl.dsp.workspace.toggle_special",
        fields: fields![FieldDef {
            key: "name",
            label: "Name",
            kind: FieldKind::Text {
                placeholder: "magic",
            },
            required: true,
        }],
        positional: true,
    },
    DispatcherDef {
        id: "workspace.rename",
        label: "Rename workspace",
        category: Category::Workspace,
        path: "hl.dsp.workspace.rename",
        fields: fields![
            FieldDef {
                key: "workspace",
                label: "Workspace",
                kind: FieldKind::Workspace,
                required: true,
            },
            FieldDef {
                key: "name",
                label: "New name",
                kind: FieldKind::Text {
                    placeholder: "web",
                },
                required: false,
            },
        ],
        positional: false,
    },
    DispatcherDef {
        id: "workspace.move",
        label: "Move workspace to monitor",
        category: Category::Workspace,
        path: "hl.dsp.workspace.move",
        fields: fields![
            FieldDef {
                key: "workspace",
                label: "Workspace",
                kind: FieldKind::Workspace,
                required: false,
            },
            FieldDef {
                key: "monitor",
                label: "Monitor",
                kind: FieldKind::Text {
                    placeholder: "+1 / DP-1",
                },
                required: true,
            },
        ],
        positional: false,
    },
    // Group
    DispatcherDef {
        id: "group.toggle",
        label: "Toggle group",
        category: Category::Group,
        path: "hl.dsp.group.toggle",
        fields: fields![WINDOW],
        positional: false,
    },
    DispatcherDef {
        id: "group.next",
        label: "Next in group",
        category: Category::Group,
        path: "hl.dsp.group.next",
        fields: fields![WINDOW],
        positional: false,
    },
    DispatcherDef {
        id: "group.prev",
        label: "Previous in group",
        category: Category::Group,
        path: "hl.dsp.group.prev",
        fields: fields![WINDOW],
        positional: false,
    },
    DispatcherDef {
        id: "group.lock",
        label: "Lock group",
        category: Category::Group,
        path: "hl.dsp.group.lock",
        fields: fields![ACTION, WINDOW],
        positional: false,
    },
    // Cursor
    DispatcherDef {
        id: "cursor.move",
        label: "Move cursor",
        category: Category::Cursor,
        path: "hl.dsp.cursor.move",
        fields: fields![
            FieldDef {
                key: "x",
                label: "X",
                kind: FieldKind::Number {
                    min: 0.0,
                    max: 10000.0,
                    step: 1.0,
                    default: 0.0,
                },
                required: true,
            },
            FieldDef {
                key: "y",
                label: "Y",
                kind: FieldKind::Number {
                    min: 0.0,
                    max: 10000.0,
                    step: 1.0,
                    default: 0.0,
                },
                required: true,
            },
        ],
        positional: false,
    },
    DispatcherDef {
        id: "cursor.move_to_corner",
        label: "Cursor to corner",
        category: Category::Cursor,
        path: "hl.dsp.cursor.move_to_corner",
        fields: fields![
            FieldDef {
                key: "corner",
                label: "Corner (0-3)",
                kind: FieldKind::Number {
                    min: 0.0,
                    max: 3.0,
                    step: 1.0,
                    default: 0.0,
                },
                required: true,
            },
            WINDOW,
        ],
        positional: false,
    },
    // extras kept for completeness near end
    DispatcherDef {
        id: "send_shortcut",
        label: "Send shortcut",
        category: Category::General,
        path: "hl.dsp.send_shortcut",
        fields: fields![
            FieldDef {
                key: "mods",
                label: "Mods",
                kind: FieldKind::Text {
                    placeholder: "SUPER",
                },
                required: true,
            },
            FieldDef {
                key: "key",
                label: "Key",
                kind: FieldKind::Text { placeholder: "F4" },
                required: true,
            },
            WINDOW,
        ],
        positional: false,
    },
    DispatcherDef {
        id: "force_idle",
        label: "Force idle",
        category: Category::General,
        path: "hl.dsp.force_idle",
        fields: fields![FieldDef {
            key: "seconds",
            label: "Seconds",
            kind: FieldKind::Number {
                min: 0.0,
                max: 3600.0,
                step: 1.0,
                default: 1.0,
            },
            required: true,
        }],
        positional: true,
    },
    DispatcherDef {
        id: "event",
        label: "Emit event",
        category: Category::General,
        path: "hl.dsp.event",
        fields: fields![FieldDef {
            key: "name",
            label: "Event",
            kind: FieldKind::Text {
                placeholder: "custom",
            },
            required: true,
        }],
        positional: true,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_focus_workspace() {
        let action = "hl.dsp.focus({ workspace = 3 })";
        let parsed = parse(action).unwrap();
        assert_eq!(parsed.dispatcher_id, "focus.workspace");
        assert_eq!(parsed.values.get("workspace").unwrap(), "3");
        let def = find(&parsed.dispatcher_id).unwrap();
        let out = generate(def, &parsed.values);
        assert!(out.contains("workspace = 3"));
    }

    #[test]
    fn roundtrip_exec_cmd_var() {
        let action = "hl.dsp.exec_cmd(terminal)";
        let parsed = parse(action).unwrap();
        assert_eq!(parsed.dispatcher_id, "exec_cmd");
        let def = find("exec_cmd").unwrap();
        let mut values = HashMap::new();
        values.insert("cmd".into(), "{{terminal}}".into());
        let out = generate(def, &values);
        assert_eq!(out, "hl.dsp.exec_cmd(terminal)");
    }

    #[test]
    fn roundtrip_window_move() {
        let action = "hl.dsp.window.move({ workspace = 2, follow = false })";
        let parsed = parse(action).unwrap();
        assert_eq!(parsed.dispatcher_id, "window.move.workspace");
        let def = find(&parsed.dispatcher_id).unwrap();
        let out = generate(def, &parsed.values);
        assert!(out.contains("workspace = 2"));
        assert!(out.contains("follow = false"));
    }

    #[test]
    fn describes_focus_direction() {
        assert_eq!(
            describe_action("hl.dsp.focus({ direction = \"right\" })").as_deref(),
            Some("Move focus right")
        );
        assert_eq!(
            describe_action("hl.dsp.focus({ direction = \"l\" })").as_deref(),
            Some("Move focus left")
        );
    }

    #[test]
    fn describes_common_actions() {
        assert_eq!(
            describe_action("hl.dsp.window.close()").as_deref(),
            Some("Close window")
        );
        assert_eq!(
            describe_action("hl.dsp.exec_cmd(\"kitty\")").as_deref(),
            Some("Run kitty")
        );
        assert_eq!(
            describe_action("hl.dsp.focus({ workspace = 3 })").as_deref(),
            Some("Focus workspace 3")
        );
        assert_eq!(
            describe_action("hl.dsp.window.move({ direction = \"up\" })").as_deref(),
            Some("Move window up")
        );
        assert_eq!(
            describe_action("hl.dsp.submap(\"resize\")").as_deref(),
            Some("Enter submap “resize”")
        );
        assert!(describe_action("<lua function>").is_none());
    }

    #[test]
    fn describes_exec_at_depth() {
        let action = "hl.dsp.exec_cmd(\"ags request toggle\")";
        assert_eq!(describe_action(action).as_deref(), Some("Run ags…"));
        assert_eq!(
            describe_action_at_depth(action, 2).as_deref(),
            Some("Run ags request…")
        );
        assert_eq!(
            describe_action_at_depth(action, 3).as_deref(),
            Some("Run ags request toggle")
        );
    }

    #[test]
    fn roundtrip_playerctl() {
        let def = find("playerctl").unwrap();
        let mut values = HashMap::new();
        values.insert("action".into(), "Play/Pause".into());
        let out = generate(def, &values);
        assert_eq!(out, "hl.dsp.exec_cmd(\"playerctl play-pause\")");

        let parsed = parse(&out).unwrap();
        assert_eq!(parsed.dispatcher_id, "playerctl");
        assert_eq!(parsed.values.get("action").unwrap(), "Play/Pause");

        values.insert("player".into(), "spotify".into());
        let out = generate(def, &values);
        assert_eq!(
            out,
            "hl.dsp.exec_cmd(\"playerctl --player=spotify play-pause\")"
        );
        let parsed = parse(&out).unwrap();
        assert_eq!(parsed.values.get("player").unwrap(), "spotify");

        values.remove("player");
        values.insert("all_players".into(), "true".into());
        values.insert("action".into(), "Next".into());
        let out = generate(def, &values);
        assert_eq!(
            out,
            "hl.dsp.exec_cmd(\"playerctl --all-players next\")"
        );
        assert_eq!(
            describe_action(&out).as_deref(),
            Some("Playerctl Next (all)")
        );
    }
}
