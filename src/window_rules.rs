use crate::spec;
use crate::variables::{self, ConfigVariable};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowRule {
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "match")]
    pub match_props: BTreeMap<String, Value>,
    #[serde(default)]
    pub effects: BTreeMap<String, Value>,
    #[serde(default)]
    #[allow(dead_code)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub source_file: String,
    #[serde(default)]
    pub source_line: u32,
    #[serde(default)]
    pub id: usize,
    /// Display name: rule name if set, otherwise untitled-N.
    #[serde(default)]
    pub display_name: String,
}

pub fn assign_rule_names(rules: &mut [WindowRule]) {
    let mut untitled = 0usize;
    for (idx, rule) in rules.iter_mut().enumerate() {
        rule.id = idx;
        let name = rule.name.trim();
        if name.is_empty() {
            untitled += 1;
            rule.display_name = format!("untitled-{untitled}");
        } else {
            rule.display_name = name.to_string();
        }
    }
}

impl WindowRule {
    pub fn matches(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        let q = query.to_lowercase();
        self.display_name.to_lowercase().contains(&q)
            || self.name.to_lowercase().contains(&q)
            || self.match_label().to_lowercase().contains(&q)
            || self.effects_label().to_lowercase().contains(&q)
            || self.source_file.to_lowercase().contains(&q)
    }

    pub fn match_label(&self) -> String {
        if self.match_props.is_empty() {
            return "(no match)".into();
        }
        self.match_props
            .iter()
            .map(|(k, v)| format!("{k}={}", value_short(v)))
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn effects_label(&self) -> String {
        if self.effects.is_empty() {
            return "(no effects)".into();
        }
        self.effects
            .iter()
            .map(|(k, v)| format!("{k}={}", value_short(v)))
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn source_label(&self) -> String {
        if self.source_file.is_empty() {
            String::new()
        } else {
            let name = std::path::Path::new(&self.source_file)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(self.source_file.as_str());
            format!("{name}:{}", self.source_line)
        }
    }

    pub fn match_string(&self, key: &str) -> Option<String> {
        self.match_props.get(key).and_then(value_as_string)
    }

    pub fn match_string_template(&self, key: &str, vars: &[ConfigVariable]) -> Option<String> {
        self.match_string(key)
            .map(|s| variables::string_to_template(&s, vars))
    }

    pub fn match_bool(&self, key: &str) -> Option<bool> {
        self.match_props.get(key).and_then(Value::as_bool)
    }

    pub fn effect_bool(&self, key: &str) -> Option<bool> {
        self.effects.get(key).and_then(Value::as_bool)
    }

    pub fn effect_string(&self, key: &str) -> Option<String> {
        self.effects.get(key).and_then(value_as_string)
    }

    pub fn effect_string_template(&self, key: &str, vars: &[ConfigVariable]) -> Option<String> {
        self.effect_string(key)
            .map(|s| variables::string_to_template(&s, vars))
    }

    pub fn match_label_templated(&self, vars: &[ConfigVariable]) -> String {
        if self.match_props.is_empty() {
            return "(no match)".into();
        }
        self.match_props
            .iter()
            .map(|(k, v)| {
                let shown = value_as_string(v)
                    .map(|s| variables::string_to_template(&s, vars))
                    .unwrap_or_else(|| value_short(v));
                format!("{k}={shown}")
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn effects_label_templated(&self, vars: &[ConfigVariable]) -> String {
        if self.effects.is_empty() {
            return "(no effects)".into();
        }
        self.effects
            .iter()
            .map(|(k, v)| {
                let shown = value_as_string(v)
                    .map(|s| variables::string_to_template(&s, vars))
                    .unwrap_or_else(|| value_short(v));
                format!("{k}={shown}")
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn value_as_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Null => None,
        other => Some(other.to_string()),
    }
}

fn value_short(v: &Value) -> String {
    match v {
        Value::String(s) => {
            if s.len() > 40 {
                format!("{}…", &s[..37])
            } else {
                s.clone()
            }
        }
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Null => "nil".into(),
        Value::Array(a) => format!("[{} items]", a.len()),
        Value::Object(o) => format!("{{{} keys}}", o.len()),
    }
}

/// Common boolean effects shown as checkboxes in the editor.
pub const BOOL_EFFECTS: &[(&str, &str)] = &[
    ("float", "Float"),
    ("tile", "Tile"),
    ("fullscreen", "Fullscreen"),
    ("maximize", "Maximize"),
    ("center", "Center"),
    ("pin", "Pin"),
    ("pseudo", "Pseudo-tile"),
    ("no_focus", "No focus"),
    ("no_anim", "No animation"),
    ("no_blur", "No blur"),
    ("no_shadow", "No shadow"),
    ("opaque", "Opaque"),
    ("dim_around", "Dim around"),
    ("stay_focused", "Stay focused"),
    ("no_initial_focus", "No initial focus"),
];

/// Boolean match props (optional filters).
pub const BOOL_MATCH_PROPS: &[(&str, &str)] = &[
    ("xwayland", "XWayland"),
    ("float", "Floating"),
    ("fullscreen", "Fullscreen"),
    ("pin", "Pinned"),
    ("focus", "Focused"),
    ("group", "Grouped"),
    ("modal", "Modal"),
];

/// Convert a UI field (plain text or `{{var}}` template) into a JSON value for Lua emission.
pub fn field_to_value(raw: &str) -> Result<Option<Value>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    if raw.contains("{{") {
        let expr = variables::template_to_lua_expr(raw)?;
        Ok(Some(json!({ "__lua_expr": expr })))
    } else {
        Ok(Some(json!(raw)))
    }
}

/// Format a window rule as a Lua `hl.window_rule({...})` call.
pub fn format_lua_rule(
    name: &str,
    match_props: &BTreeMap<String, Value>,
    effects: &BTreeMap<String, Value>,
) -> String {
    let mut parts = Vec::new();
    if !name.trim().is_empty() {
        parts.push(format!("  name = {},", spec::lua_string(name.trim())));
    }

    let match_map: Map<String, Value> = match_props
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    parts.push(format!(
        "  match = {},",
        spec::lua_value_at(&Value::Object(match_map), 1)
    ));

    for (k, v) in effects {
        parts.push(format!("  {k} = {},", spec::lua_value_at(v, 1)));
    }

    format!("hl.window_rule({{\n{}\n}})", parts.join("\n"))
}

/// Prefer exact class match when picking from a live window.
pub fn exact_class_regex(class: &str) -> String {
    let class = class.trim();
    if class.is_empty() {
        return String::new();
    }
    if class.starts_with('^') || class.contains(".*") || class.contains('(') {
        return class.to_string();
    }
    format!("^{}$", escape_re2_literal(class))
}

fn escape_re2_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '.' | '+' | '*' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '\\' | '^' | '$' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn formats_named_rule() {
        let mut match_props = BTreeMap::new();
        match_props.insert("class".into(), json!("^firefox$"));
        let mut effects = BTreeMap::new();
        effects.insert("float".into(), json!(true));
        effects.insert("workspace".into(), json!("2"));
        let lua = format_lua_rule("browser", &match_props, &effects);
        assert!(lua.contains("name = \"browser\""));
        assert!(lua.contains("  match = {\n    class = \"^firefox$\",\n  },"));
        assert!(lua.contains("float = true"));
        assert!(lua.contains("workspace = \"2\""));
    }

    #[test]
    fn exact_class_wraps() {
        assert_eq!(exact_class_regex("zen"), "^zen$");
        assert_eq!(exact_class_regex("^zen$"), "^zen$");
        assert_eq!(exact_class_regex("a.b"), r"^a\.b$");
    }

    #[test]
    fn formats_variable_expr() {
        let mut match_props = BTreeMap::new();
        match_props.insert(
            "class".into(),
            json!({"__lua_expr": "\"^\" .. terminal .. \"$\""}),
        );
        let effects = BTreeMap::new();
        let lua = format_lua_rule("", &match_props, &effects);
        assert!(lua.contains("class = \"^\" .. terminal .. \"$\""));
        assert!(!lua.contains("__lua_expr"));
    }
}
