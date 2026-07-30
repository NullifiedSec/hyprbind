use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigVariable {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub source_file: String,
    #[serde(default)]
    pub source_line: u32,
}

impl ConfigVariable {
    pub fn token(&self) -> String {
        format!("{{{{{}}}}}", self.name)
    }

    pub fn suggestion_label(&self) -> String {
        format!("{} = \"{}\"", self.name, self.value)
    }
}

/// Turn `{{mainMod}} + Q` into a Lua expression: `mainMod .. " + Q"`.
pub fn template_to_lua_expr(template: &str) -> Result<String, String> {
    let template = template.trim();
    if template.is_empty() {
        return Err("template is empty".into());
    }

    let parts = split_template(template)?;
    if parts.is_empty() {
        return Err("template is empty".into());
    }

    // Entire template is a single variable → bare identifier.
    if parts.len() == 1 {
        return match &parts[0] {
            TemplatePart::Var(name) => Ok(name.clone()),
            TemplatePart::Lit(lit) => Ok(crate::keys::lua_string_literal(lit)),
        };
    }

    let mut out = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            out.push_str(" .. ");
        }
        match part {
            TemplatePart::Var(name) => out.push_str(name),
            TemplatePart::Lit(lit) => out.push_str(&crate::keys::lua_string_literal(lit)),
        }
    }
    Ok(out)
}

/// Replace `{{var}}` inside a Lua/action string.
/// - `{{name}}` alone → `name`
/// - embedded → substituted as identifier (for use in hl.dsp.exec_cmd({{terminal}}))
pub fn substitute_action_template(action: &str) -> Result<String, String> {
    let action = action.trim();
    if !action.contains("{{") {
        return Ok(action.to_string());
    }

    // Whole-action variable.
    if let Some(name) = exact_var_token(action) {
        return Ok(format!("hl.dsp.exec_cmd({name})"));
    }

    let mut out = String::new();
    let mut rest = action;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            return Err("unclosed {{ in action".into());
        };
        let name = after[..end].trim();
        if !is_ident(name) {
            return Err(format!("invalid variable name in {{{{{{name}}}}}}"));
        }
        out.push_str(name);
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Expand `{{var}}` using known values (for preview / recording normalization).
pub fn expand_template(template: &str, vars: &[ConfigVariable]) -> Result<String, String> {
    let map: HashMap<&str, &str> = vars.iter().map(|v| (v.name.as_str(), v.value.as_str())).collect();
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            return Err("unclosed {{".into());
        };
        let name = after[..end].trim();
        let value = map
            .get(name)
            .ok_or_else(|| format!("unknown variable '{name}'"))?;
        out.push_str(value);
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Convert a resolved chord like `SUPER + Q` into `{{mainMod}} + Q` when possible.
pub fn resolve_to_template(resolved: &str, vars: &[ConfigVariable]) -> String {
    if resolved.contains("{{") {
        return resolved.to_string();
    }

    let mut by_value: HashMap<&str, Vec<&ConfigVariable>> = HashMap::new();
    for var in vars {
        by_value.entry(var.value.as_str()).or_default().push(var);
    }

    resolved
        .split('+')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|token| {
            if let Some(candidates) = by_value.get(token) {
                let chosen = pick_variable(candidates);
                format!("{{{{{}}}}}", chosen.name)
            } else {
                token.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

/// Also reverse string literals inside actions: `"ghostty"` → `{{terminal}}` when matched.
pub fn action_to_template(action: &str, vars: &[ConfigVariable]) -> String {
    if action.contains("{{") || vars.is_empty() {
        return action.to_string();
    }

    let mut result = action.to_string();
    let mut pairs: Vec<_> = vars.iter().collect();
    pairs.sort_by(|a, b| b.value.len().cmp(&a.value.len()));

    for var in pairs {
        if var.value.is_empty() {
            continue;
        }
        let quoted_double = format!("\"{}\"", var.value.replace('\\', "\\\\").replace('"', "\\\""));
        let quoted_single = format!("'{}'", var.value.replace('\\', "\\\\").replace('\'', "\\'"));
        let token = var.token();
        if result.contains(&quoted_double) {
            result = result.replace(&quoted_double, &token);
        } else if result.contains(&quoted_single) {
            result = result.replace(&quoted_single, &token);
        }
    }
    result
}

/// Reverse a plain string field (window-rule class/title/etc.) into `{{var}}` when values match.
pub fn string_to_template(value: &str, vars: &[ConfigVariable]) -> String {
    if value.contains("{{") || vars.is_empty() {
        return value.to_string();
    }

    // Prefer exact match first.
    let mut exact: Vec<&ConfigVariable> = vars.iter().filter(|v| v.value == value).collect();
    if !exact.is_empty() {
        exact.sort_by(|a, b| a.name.len().cmp(&b.name.len()));
        return pick_variable(&exact).token();
    }

    let mut result = value.to_string();
    let mut pairs: Vec<_> = vars.iter().collect();
    pairs.sort_by(|a, b| b.value.len().cmp(&a.value.len()));
    for var in pairs {
        if var.value.is_empty() {
            continue;
        }
        if result.contains(&var.value) {
            result = result.replace(&var.value, &var.token());
        }
    }
    result
}

pub fn filter_variables<'a>(vars: &'a [ConfigVariable], prefix: &str) -> Vec<&'a ConfigVariable> {
    let prefix = prefix.to_lowercase();
    vars.iter()
        .filter(|v| {
            prefix.is_empty()
                || v.name.to_lowercase().starts_with(&prefix)
                || v.name.to_lowercase().contains(&prefix)
        })
        .collect()
}

/// If text ends with `{{something` (unclosed), return the prefix typed after `{{`.
pub fn unfinished_var_prefix(text: &str) -> Option<String> {
    let Some(start) = text.rfind("{{") else {
        return None;
    };
    let after = &text[start + 2..];
    if after.contains("}}") {
        return None;
    }
    if after.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Some(after.to_string());
    }
    None
}

fn pick_variable<'a>(candidates: &[&'a ConfigVariable]) -> &'a ConfigVariable {
    const PREFERRED: &[&str] = &["mainMod", "mainmod", "mod", "modifier", "terminal", "menu", "fileManager"];
    for preferred in PREFERRED {
        if let Some(found) = candidates.iter().find(|v| v.name.eq_ignore_ascii_case(preferred)) {
            return found;
        }
    }
    candidates[0]
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TemplatePart {
    Lit(String),
    Var(String),
}

fn split_template(template: &str) -> Result<Vec<TemplatePart>, String> {
    let mut parts = Vec::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        if start > 0 {
            parts.push(TemplatePart::Lit(rest[..start].to_string()));
        }
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            return Err("unclosed {{".into());
        };
        let name = after[..end].trim();
        if !is_ident(name) {
            return Err(format!("invalid variable name '{name}'"));
        }
        parts.push(TemplatePart::Var(name.to_string()));
        rest = &after[end + 2..];
    }
    if !rest.is_empty() {
        parts.push(TemplatePart::Lit(rest.to_string()));
    }
    Ok(parts)
}

fn exact_var_token(s: &str) -> Option<&str> {
    let s = s.trim();
    if let Some(inner) = s.strip_prefix("{{").and_then(|s| s.strip_suffix("}}")) {
        let name = inner.trim();
        if is_ident(name) && !name.contains('{') {
            return Some(name);
        }
    }
    None
}

fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn var(name: &str, value: &str) -> ConfigVariable {
        ConfigVariable {
            name: name.into(),
            value: value.into(),
            source_file: String::new(),
            source_line: 0,
        }
    }

    #[test]
    fn keys_template_to_lua() {
        assert_eq!(
            template_to_lua_expr("{{mainMod}} + Q").unwrap(),
            "mainMod .. \" + Q\""
        );
        assert_eq!(
            template_to_lua_expr("SUPER + Q").unwrap(),
            "\"SUPER + Q\""
        );
        assert_eq!(template_to_lua_expr("{{mainMod}}").unwrap(), "mainMod");
    }

    #[test]
    fn reverse_keys_use_variables() {
        let vars = vec![var("mainMod", "SUPER"), var("terminal", "ghostty")];
        assert_eq!(resolve_to_template("SUPER + Q", &vars), "{{mainMod}} + Q");
        assert_eq!(
            action_to_template("hl.dsp.exec_cmd(\"ghostty\")", &vars),
            "hl.dsp.exec_cmd({{terminal}})"
        );
        assert_eq!(string_to_template("ghostty", &vars), "{{terminal}}");
        assert_eq!(string_to_template("^ghostty$", &vars), "^{{terminal}}$");
    }
}
