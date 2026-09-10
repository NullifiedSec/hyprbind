use cxx_qt_lib::QString;
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[cxx_qt::bridge]
mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        type CatalogBridge = super::CatalogBridgeRust;

        #[qinvokable]
        #[rust_name = "snapshot"]
        fn snapshot(self: &CatalogBridge, kind: &QString) -> QString;

        #[qinvokable]
        #[rust_name = "save_item"]
        fn saveItem(self: &CatalogBridge, kind: &QString, id: i32, payload: &QString) -> QString;

        #[qinvokable]
        #[rust_name = "delete_item"]
        fn deleteItem(self: &CatalogBridge, kind: &QString, id: i32) -> QString;

        #[qinvokable]
        #[rust_name = "config_snapshot"]
        fn configSnapshot(self: &CatalogBridge) -> QString;

        #[qinvokable]
        #[rust_name = "save_config"]
        fn saveConfig(self: &CatalogBridge, payload: &QString) -> QString;
    }
}

#[derive(Default)]
pub struct CatalogBridgeRust;

impl qobject::CatalogBridge {
    fn snapshot(&self, kind: &QString) -> QString {
        let kind = String::from(kind);
        let mut collection = match crate::config::load_binds(None) {
            Ok(collection) => collection,
            Err(error) => return action_error(error.to_string()),
        };
        collection.finalize();

        let items = match kind.as_str() {
            "binds" => {
                let conflicts = crate::conflicts::conflicting_ids(&collection.binds);
                collection.binds.iter().map(|item| json!({
                    "entryId": item.id,
                    "displayName": item.name,
                    "name": item.name,
                    "keys": item.keys,
                    "action": item.action,
                    "flags": item.flags,
                    "flagsLabel": item.flags_label(),
                    "submap": item.submap,
                    "sourceFile": item.source_file,
                    "sourceLine": item.source_line,
                    "sourceLabel": item.source_label(),
                    "sharedSource": collection.source_share_count(item) > 1,
                    "conflict": conflicts.contains(&item.id),
                })).collect::<Vec<_>>()
            }
            "window_rules" => rule_rows(&collection.window_rules),
            "layer_rules" => rule_rows(&collection.layer_rules),
            "workspace_rules" => spec_rows(&collection.workspace_rules),
            "monitors" => spec_rows(&collection.monitors),
            "devices" => spec_rows(&collection.devices),
            "animations" => spec_rows(&collection.animations),
            "curves" => spec_rows(&collection.curves),
            "gestures" => spec_rows(&collection.gestures),
            _ => return action_error(format!("Unknown catalog kind `{kind}`")),
        };

        to_qstring(json!({
            "ok": true,
            "configPath": collection.config_path.display().to_string(),
            "items": items,
            "submaps": collection.submap_names(),
            "warning": collection.error.unwrap_or_default(),
        }))
    }

    fn save_item(&self, kind: &QString, id: i32, payload: &QString) -> QString {
        let kind = String::from(kind);
        let input = match parse_payload(payload) {
            Ok(value) => value,
            Err(error) => return action_error(error),
        };
        let mut collection = match crate::config::load_binds(None) {
            Ok(collection) => collection,
            Err(error) => return action_error(error.to_string()),
        };
        collection.finalize();
        let adding = id < 0;

        let result = match kind.as_str() {
            "binds" => {
                let name = text(&input, "name");
                let keys = text(&input, "keys");
                let action = text(&input, "action");
                let submap = text(&input, "submap");
                if adding {
                    crate::writer::add_bind(&collection.config_path, &name, &keys, &action, &submap)
                } else {
                    let Some(item) = collection.bind_by_id(id as usize) else {
                        return action_error("The selected bind is no longer present.");
                    };
                    let shared = collection.source_share_count(item) > 1;
                    crate::writer::save_bind(item, &name, &keys, &action, shared)
                }
            }
            "window_rules" | "layer_rules" => {
                let name = text(&input, "name");
                let match_props = match object_map(&input, "match") {
                    Ok(map) => map,
                    Err(error) => return action_error(error),
                };
                let effects = match object_map(&input, "effects") {
                    Ok(map) => map,
                    Err(error) => return action_error(error),
                };
                if kind == "window_rules" {
                    if adding {
                        crate::writer::add_window_rule(&collection.config_path, &name, &match_props, &effects)
                    } else {
                        let Some(item) = collection.rule_by_id(id as usize) else {
                            return action_error("The selected window rule is no longer present.");
                        };
                        crate::writer::save_window_rule(item, &name, &match_props, &effects)
                    }
                } else if adding {
                    crate::writer::add_layer_rule(&collection.config_path, &name, &match_props, &effects)
                } else {
                    let Some(item) = collection.layer_rule_by_id(id as usize) else {
                        return action_error("The selected layer rule is no longer present.");
                    };
                    crate::writer::save_layer_rule(item, &name, &match_props, &effects)
                }
            }
            "workspace_rules" | "monitors" | "devices" | "animations" | "curves" | "gestures" => {
                let fields = match object_map(&input, "fields") {
                    Ok(map) => map,
                    Err(error) => return action_error(error),
                };
                save_spec_kind(&collection, &kind, id, adding, &input, &fields)
            }
            _ => return action_error(format!("Unknown catalog kind `{kind}`")),
        };

        match result {
            Ok(result) => action_ok(format!("Saved to {}", result.path)),
            Err(error) => action_error(error.to_string()),
        }
    }

    fn delete_item(&self, kind: &QString, id: i32) -> QString {
        if id < 0 {
            return action_error("Select an item first.");
        }
        let kind = String::from(kind);
        let mut collection = match crate::config::load_binds(None) {
            Ok(collection) => collection,
            Err(error) => return action_error(error.to_string()),
        };
        collection.finalize();
        let id = id as usize;

        let result = match kind.as_str() {
            "binds" => {
                let Some(item) = collection.bind_by_id(id) else { return action_error("Bind not found."); };
                crate::writer::delete_bind(item, collection.source_share_count(item) > 1)
            }
            "window_rules" => {
                let Some(item) = collection.rule_by_id(id) else { return action_error("Window rule not found."); };
                crate::writer::delete_window_rule(item)
            }
            "layer_rules" => {
                let Some(item) = collection.layer_rule_by_id(id) else { return action_error("Layer rule not found."); };
                crate::writer::delete_layer_rule(item)
            }
            "workspace_rules" => delete_spec(collection.workspace_rule_by_id(id), crate::writer::delete_workspace_rule),
            "monitors" => delete_spec(collection.monitor_by_id(id), crate::writer::delete_monitor),
            "devices" => delete_spec(collection.device_by_id(id), crate::writer::delete_device),
            "animations" => delete_spec(collection.animation_by_id(id), crate::writer::delete_animation),
            "curves" => delete_spec(collection.curve_by_id(id), crate::writer::delete_curve),
            "gestures" => delete_spec(collection.gesture_by_id(id), crate::writer::delete_gesture),
            _ => return action_error(format!("Unknown catalog kind `{kind}`")),
        };

        match result {
            Ok(result) => action_ok(format!("Deleted from {}", result.path)),
            Err(error) => action_error(error.to_string()),
        }
    }

    fn config_snapshot(&self) -> QString {
        let mut collection = match crate::config::load_binds(None) {
            Ok(collection) => collection,
            Err(error) => return action_error(error.to_string()),
        };
        collection.finalize();
        to_qstring(json!({
            "ok": true,
            "configPath": collection.config_path.display().to_string(),
            "config": collection.config_merged,
            "warning": collection.error.unwrap_or_default(),
        }))
    }

    fn save_config(&self, payload: &QString) -> QString {
        let input = match parse_payload(payload) {
            Ok(value) => value,
            Err(error) => return action_error(error),
        };
        let Some(config) = input.get("config") else {
            return action_error("Missing config object.");
        };
        if !config.is_object() {
            return action_error("Config must be a JSON object.");
        }
        let Some(path) = crate::config::default_config_path() else {
            return action_error("Could not determine the Hyprland config path.");
        };
        match crate::writer::save_config_override(&path, config) {
            Ok(result) => action_ok(format!("Saved config override to {}", result.path)),
            Err(error) => action_error(error.to_string()),
        }
    }
}

fn rule_rows(items: &[crate::window_rules::WindowRule]) -> Vec<Value> {
    items.iter().map(|item| json!({
        "entryId": item.id,
        "displayName": item.display_name,
        "name": item.name,
        "match": item.match_props,
        "effects": item.effects,
        "matchLabel": item.match_label(),
        "effectsLabel": item.effects_label(),
        "sourceFile": item.source_file,
        "sourceLine": item.source_line,
        "sourceLabel": item.source_label(),
    })).collect()
}

fn spec_rows(items: &[crate::spec::SpecItem]) -> Vec<Value> {
    items.iter().map(|item| json!({
        "entryId": item.id,
        "displayName": item.display_name,
        "name": item.name,
        "fields": item.fields,
        "fieldsLabel": item.fields_label(),
        "sourceFile": item.source_file,
        "sourceLine": item.source_line,
        "sourceLabel": item.source_label(),
    })).collect()
}

fn save_spec_kind(
    collection: &crate::bind::BindCollection,
    kind: &str,
    id: i32,
    adding: bool,
    input: &Value,
    fields: &BTreeMap<String, Value>,
) -> Result<crate::writer::WriteResult, crate::writer::WriteError> {
    match kind {
        "workspace_rules" => if adding {
            crate::writer::add_workspace_rule(&collection.config_path, fields)
        } else {
            let item = collection.workspace_rule_by_id(id as usize).ok_or_else(|| crate::writer::WriteError::Invalid("workspace rule not found".into()))?;
            crate::writer::save_workspace_rule(item, fields)
        },
        "monitors" => if adding {
            crate::writer::add_monitor(&collection.config_path, fields)
        } else {
            let item = collection.monitor_by_id(id as usize).ok_or_else(|| crate::writer::WriteError::Invalid("monitor not found".into()))?;
            crate::writer::save_monitor(item, fields)
        },
        "devices" => if adding {
            crate::writer::add_device(&collection.config_path, fields)
        } else {
            let item = collection.device_by_id(id as usize).ok_or_else(|| crate::writer::WriteError::Invalid("device not found".into()))?;
            crate::writer::save_device(item, fields)
        },
        "animations" => if adding {
            crate::writer::add_animation(&collection.config_path, fields)
        } else {
            let item = collection.animation_by_id(id as usize).ok_or_else(|| crate::writer::WriteError::Invalid("animation not found".into()))?;
            crate::writer::save_animation(item, fields)
        },
        "curves" => {
            let name = text(input, "name");
            if adding {
                crate::writer::add_curve(&collection.config_path, &name, fields)
            } else {
                let item = collection.curve_by_id(id as usize).ok_or_else(|| crate::writer::WriteError::Invalid("curve not found".into()))?;
                crate::writer::save_curve(item, &name, fields)
            }
        }
        "gestures" => if adding {
            crate::writer::add_gesture(&collection.config_path, fields)
        } else {
            let item = collection.gesture_by_id(id as usize).ok_or_else(|| crate::writer::WriteError::Invalid("gesture not found".into()))?;
            crate::writer::save_gesture(item, fields)
        },
        _ => Err(crate::writer::WriteError::Invalid(format!("unsupported catalog kind `{kind}`"))),
    }
}

fn delete_spec(
    item: Option<&crate::spec::SpecItem>,
    delete: fn(&crate::spec::SpecItem) -> Result<crate::writer::WriteResult, crate::writer::WriteError>,
) -> Result<crate::writer::WriteResult, crate::writer::WriteError> {
    let item = item.ok_or_else(|| crate::writer::WriteError::Invalid("item not found".into()))?;
    delete(item)
}

fn object_map(input: &Value, key: &str) -> Result<BTreeMap<String, Value>, String> {
    let Some(object) = input.get(key).and_then(Value::as_object) else {
        return Err(format!("`{key}` must be a JSON object."));
    };
    Ok(object.iter().map(|(key, value)| (key.clone(), value.clone())).collect())
}

fn text(input: &Value, key: &str) -> String {
    input.get(key).and_then(Value::as_str).unwrap_or_default().trim().to_string()
}

fn parse_payload(payload: &QString) -> Result<Value, String> {
    serde_json::from_str::<Value>(&String::from(payload)).map_err(|error| error.to_string())
}

fn to_qstring(value: Value) -> QString {
    QString::from(value.to_string())
}

fn action_ok(message: String) -> QString {
    to_qstring(json!({ "ok": true, "message": message }))
}

fn action_error(message: impl Into<String>) -> QString {
    to_qstring(json!({ "ok": false, "error": message.into() }))
}
