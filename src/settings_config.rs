//! Merged `hl.config` view helpers (MVP sections).

use serde_json::{json, Map, Value};

/// Keys shown/edited in the Config settings tab.
pub const MVP_SECTIONS: &[&str] = &[
    "general",
    "decoration",
    "input",
    "animations",
    "dwindle",
    "master",
    "misc",
];

pub fn section(merged: &Value, name: &str) -> Value {
    merged
        .as_object()
        .and_then(|o| o.get(name))
        .cloned()
        .unwrap_or_else(|| json!({}))
}

pub fn get_path<'a>(root: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut cur = root;
    for key in path {
        cur = cur.as_object()?.get(*key)?;
    }
    Some(cur)
}

pub fn get_bool(root: &Value, path: &[&str], default: bool) -> bool {
    get_path(root, path)
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

pub fn get_f64(root: &Value, path: &[&str], default: f64) -> f64 {
    get_path(root, path)
        .and_then(Value::as_f64)
        .unwrap_or(default)
}

pub fn get_i64(root: &Value, path: &[&str], default: i64) -> i64 {
    get_path(root, path)
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
        .unwrap_or(default)
}

pub fn get_string(root: &Value, path: &[&str], default: &str) -> String {
    get_path(root, path)
        .and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            _ => None,
        })
        .unwrap_or_else(|| default.to_string())
}

/// Set a dotted path inside an object Value (creates intermediate objects).
pub fn set_path(root: &mut Value, path: &[&str], value: Value) {
    if path.is_empty() {
        *root = value;
        return;
    }
    if !root.is_object() {
        *root = Value::Object(Map::new());
    }
    let mut cur = root.as_object_mut().unwrap();
    for (i, key) in path.iter().enumerate() {
        if i + 1 == path.len() {
            cur.insert((*key).to_string(), value);
            return;
        }
        let entry = cur
            .entry((*key).to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if !entry.is_object() {
            *entry = Value::Object(Map::new());
        }
        cur = entry.as_object_mut().unwrap();
    }
}

/// Keep only MVP sections for the override write.
pub fn filter_mvp(merged: &Value) -> Value {
    let Some(obj) = merged.as_object() else {
        return json!({});
    };
    let mut out = Map::new();
    for key in MVP_SECTIONS {
        if let Some(v) = obj.get(*key) {
            out.insert((*key).to_string(), v.clone());
        }
    }
    Value::Object(out)
}

pub fn color_to_display(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Object(o) => {
            if let Some(colors) = o.get("colors").and_then(|c| c.as_array()) {
                let joined: Vec<String> = colors
                    .iter()
                    .filter_map(|c| c.as_str().map(|s| s.to_string()))
                    .collect();
                let angle = o
                    .get("angle")
                    .and_then(|a| a.as_f64().or_else(|| a.as_i64().map(|i| i as f64)))
                    .map(|a| {
                        if a.fract() == 0.0 {
                            format!(" @{}", a as i64)
                        } else {
                            format!(" @{a}")
                        }
                    })
                    .unwrap_or_default();
                format!("{}{angle}", joined.join(" "))
            } else {
                v.to_string()
            }
        }
        Value::Number(n) => {
            // Prefer hex for color-like integers (ARGB).
            if let Some(i) = n.as_u64() {
                if i > 0x00ff_ffff {
                    return format!("0x{i:08x}");
                }
            }
            n.to_string()
        }
        other => other.to_string(),
    }
}

/// Parse a color / gradient field from the Config UI.
///
/// - `rgba(...)` → string
/// - `rgba(a) rgba(b) @45` → `{ colors = {...}, angle = 45 }`
/// - `0xee1a1a1a` → number
pub fn parse_color_input(raw: &str) -> Value {
    let raw = raw.trim();
    if raw.is_empty() {
        return Value::Null;
    }

    if let Some(hex) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        if let Ok(n) = u64::from_str_radix(hex, 16) {
            return json!(n);
        }
    }

    // Gradient: one or more color tokens, optional `@angle`
    let (colors_part, angle) = if let Some((left, right)) = raw.rsplit_once('@') {
        let angle = right.trim().parse::<f64>().ok();
        (left.trim(), angle)
    } else {
        (raw, None)
    };

    let colors: Vec<String> = colors_part
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|s| s.to_string())
        .collect();

    if colors.len() >= 2 || angle.is_some() {
        let mut obj = serde_json::Map::new();
        obj.insert("colors".into(), json!(colors));
        if let Some(a) = angle {
            if a.fract() == 0.0 {
                obj.insert("angle".into(), json!(a as i64));
            } else {
                obj.insert("angle".into(), json!(a));
            }
        }
        return Value::Object(obj);
    }

    if colors.len() == 1 {
        return json!(colors[0].clone());
    }

    json!(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_roundtrip() {
        let original = json!({
            "colors": ["rgba(33ccffee)", "rgba(00ff99ee)"],
            "angle": 45
        });
        let display = color_to_display(&original);
        assert_eq!(display, "rgba(33ccffee) rgba(00ff99ee) @45");
        let parsed = parse_color_input(&display);
        assert_eq!(parsed, original);
    }

    #[test]
    fn solid_color_roundtrip() {
        let display = color_to_display(&json!("rgba(595959aa)"));
        assert_eq!(display, "rgba(595959aa)");
        assert_eq!(parse_color_input(&display), json!("rgba(595959aa)"));
    }
}
