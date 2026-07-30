//! Shared collected Hyprland call specs (monitors, devices, gestures, etc.).

use crate::variables;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecItem {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub fields: BTreeMap<String, Value>,
    #[serde(default)]
    pub source_file: String,
    #[serde(default)]
    pub source_line: u32,
    #[serde(default)]
    pub id: usize,
    #[serde(default)]
    pub display_name: String,
}

pub fn assign_spec_names(items: &mut [SpecItem], fallback_key: &str) {
    let mut untitled = 0usize;
    for (idx, item) in items.iter_mut().enumerate() {
        item.id = idx;
        let named = item.name.trim();
        if !named.is_empty() {
            item.display_name = named.to_string();
            continue;
        }
        if let Some(v) = item.fields.get(fallback_key).and_then(|v| value_as_string(v)) {
            if !v.is_empty() {
                item.display_name = v;
                continue;
            }
        }
        untitled += 1;
        item.display_name = format!("untitled-{untitled}");
    }
}

impl SpecItem {
    pub fn matches(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        let q = query.to_lowercase();
        self.display_name.to_lowercase().contains(&q)
            || self.name.to_lowercase().contains(&q)
            || self.fields_label().to_lowercase().contains(&q)
            || self.source_file.to_lowercase().contains(&q)
    }

    pub fn fields_label(&self) -> String {
        if self.fields.is_empty() {
            return "(empty)".into();
        }
        self.fields
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

    pub fn get_string(&self, key: &str) -> Option<String> {
        self.fields.get(key).and_then(value_as_string)
    }

    pub fn get_string_template(&self, key: &str, vars: &[variables::ConfigVariable]) -> Option<String> {
        self.get_string(key)
            .map(|s| variables::string_to_template(&s, vars))
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.fields.get(key).and_then(Value::as_bool)
    }

    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.fields.get(key).and_then(Value::as_f64)
    }

    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.fields.get(key).and_then(Value::as_i64)
    }
}

pub fn value_as_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Null => None,
        other => Some(other.to_string()),
    }
}

pub fn value_short(v: &Value) -> String {
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
        Value::Array(a) => {
            if a.len() == 2
                && a.iter().all(|v| {
                    v.as_array()
                        .map(|p| p.len() == 2 && p.iter().all(|n| n.is_number()))
                        .unwrap_or(false)
                })
            {
                format!(
                    "({}, {})→({}, {})",
                    a[0][0].as_f64().unwrap_or(0.0),
                    a[0][1].as_f64().unwrap_or(0.0),
                    a[1][0].as_f64().unwrap_or(0.0),
                    a[1][1].as_f64().unwrap_or(0.0),
                )
            } else {
                format!("[{} items]", a.len())
            }
        }
        Value::Object(o) => {
            if o.contains_key("__lua_expr") {
                o.get("__lua_expr")
                    .and_then(|x| x.as_str())
                    .unwrap_or("expr")
                    .to_string()
            } else {
                format!("{{{} keys}}", o.len())
            }
        }
    }
}

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

pub fn format_call(fn_name: &str, fields: &BTreeMap<String, Value>, name: Option<&str>) -> String {
    let mut parts = Vec::new();
    if let Some(n) = name {
        let n = n.trim();
        if !n.is_empty() && !fields.contains_key("name") {
            parts.push(format!("  name = {},", lua_string(n)));
        }
    }
    for (k, v) in fields {
        parts.push(format!("  {k} = {},", lua_value_at(v, 1)));
    }
    format!("{fn_name}({{\n{}\n}})", parts.join("\n"))
}

/// `hl.curve("name", { ... })`
pub fn format_curve_call(name: &str, fields: &BTreeMap<String, Value>) -> String {
    let mut parts = Vec::new();
    for (k, v) in fields {
        parts.push(format!("  {k} = {},", lua_value_at(v, 1)));
    }
    format!(
        "hl.curve({}, {{\n{}\n}})",
        lua_string(name.trim()),
        parts.join("\n")
    )
}

pub fn lua_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn lua_pad(indent: usize) -> String {
    "  ".repeat(indent)
}

fn is_lua_table_key(k: &str) -> bool {
    let mut chars = k.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_scalar_lua_value(v: &Value) -> bool {
    match v {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => true,
        Value::Object(map) => map.contains_key("__lua_expr"),
        Value::Array(_) => false,
    }
}

/// Compact / indent-0 rendering of a JSON value as Lua.
pub fn lua_value(v: &Value) -> String {
    lua_value_at(v, 0)
}

/// Pretty-print a JSON value as Lua at the given indent depth (2 spaces per level).
///
/// Objects always expand to multi-line tables. Arrays of scalars stay inline;
/// arrays that contain tables expand.
pub fn lua_value_at(v: &Value, indent: usize) -> String {
    if let Some(expr) = v
        .as_object()
        .and_then(|o| o.get("__lua_expr"))
        .and_then(|x| x.as_str())
    {
        return expr.to_string();
    }
    match v {
        Value::Null => "nil".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => {
            // Emit hex for color-like ARGB integers so Lua stays readable.
            if let Some(i) = n.as_u64() {
                if i > 0x00ff_ffff {
                    return format!("0x{i:08x}");
                }
            }
            n.to_string()
        }
        Value::String(s) => lua_string(s),
        Value::Array(arr) => {
            if arr.is_empty() {
                return "{}".into();
            }
            if arr.iter().all(is_scalar_lua_value) {
                let parts: Vec<String> = arr.iter().map(|x| lua_value_at(x, 0)).collect();
                return format!("{{ {} }}", parts.join(", "));
            }
            let inner = lua_pad(indent + 1);
            let outer = lua_pad(indent);
            let mut parts = Vec::new();
            for item in arr {
                parts.push(format!("{inner}{},", lua_value_at(item, indent + 1)));
            }
            format!("{{\n{}\n{outer}}}", parts.join("\n"))
        }
        Value::Object(map) => {
            if map.is_empty() {
                return "{}".into();
            }
            let inner = lua_pad(indent + 1);
            let outer = lua_pad(indent);
            let mut parts = Vec::new();
            for (k, val) in map {
                let key = if is_lua_table_key(k) {
                    k.clone()
                } else {
                    format!("[{}]", lua_string(k))
                };
                parts.push(format!(
                    "{inner}{key} = {},",
                    lua_value_at(val, indent + 1)
                ));
            }
            format!("{{\n{}\n{outer}}}", parts.join("\n"))
        }
    }
}

pub fn format_config_call(merged: &Value) -> String {
    match merged {
        Value::Object(map) if !map.is_empty() => {
            let mut parts = Vec::new();
            for (k, v) in map {
                parts.push(format!("  {k} = {},", lua_value_at(v, 1)));
            }
            format!("hl.config({{\n{}\n}})", parts.join("\n"))
        }
        _ => "hl.config({})".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn format_config_call_nests_tables() {
        let merged = json!({
            "general": { "gaps_in": 8, "layout": "dwindle" },
            "decoration": { "rounding": 12 }
        });
        let lua = format_config_call(&merged);
        assert!(lua.contains("hl.config({"));
        assert!(lua.contains("  general = {\n    gaps_in = 8,\n    layout = \"dwindle\",\n  },"));
        assert!(lua.contains("  decoration = {\n    rounding = 12,\n  },"));
        assert!(!lua.contains("general = { gaps_in"));
    }

    #[test]
    fn format_call_nests_field_tables() {
        let mut fields = BTreeMap::new();
        fields.insert("output".into(), json!("DP-1"));
        fields.insert(
            "transform".into(),
            json!({ "rotate": 90, "reflect_x": false }),
        );
        let lua = format_call("hl.monitor", &fields, None);
        assert!(lua.contains("output = \"DP-1\","));
        assert!(lua.contains("  transform = {\n    reflect_x = false,\n    rotate = 90,\n  },"));
    }

    #[test]
    fn scalar_arrays_stay_inline() {
        let v = json!([1, 2, 3]);
        assert_eq!(lua_value_at(&v, 0), "{ 1, 2, 3 }");
    }

    #[test]
    fn hex_colors_for_large_ints() {
        let v = json!(0xff0000ffu64);
        assert_eq!(lua_value(&v), "0xff0000ff");
    }
}
