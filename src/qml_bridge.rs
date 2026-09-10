use cxx_qt_lib::QString;
use serde_json::{Value, json};

#[cxx_qt::bridge]
mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        type HyprbindBridge = super::HyprbindBridgeRust;

        #[qinvokable]
        #[rust_name = "ui_snapshot"]
        fn uiSnapshot(self: &HyprbindBridge) -> QString;

        #[qinvokable]
        #[rust_name = "set_dark_mode"]
        fn setDarkMode(self: &HyprbindBridge, enabled: bool) -> QString;

        #[qinvokable]
        #[rust_name = "restore_config"]
        fn restoreConfig(self: &HyprbindBridge) -> QString;

        #[qinvokable]
        #[rust_name = "lookfeel_snapshot"]
        fn lookFeelSnapshot(self: &HyprbindBridge) -> QString;

        #[qinvokable]
        #[rust_name = "save_lookfeel"]
        fn saveLookFeel(self: &HyprbindBridge, payload: &QString) -> QString;

        #[qinvokable]
        #[rust_name = "preview_lookfeel"]
        fn previewLookFeel(self: &HyprbindBridge, payload: &QString) -> QString;

        #[qinvokable]
        #[rust_name = "environment_snapshot"]
        fn environmentSnapshot(self: &HyprbindBridge) -> QString;

        #[qinvokable]
        #[rust_name = "add_environment"]
        fn addEnvironment(self: &HyprbindBridge, name: &QString, value: &QString) -> QString;

        #[qinvokable]
        #[rust_name = "edit_environment"]
        fn editEnvironment(
            self: &HyprbindBridge,
            current_name: &QString,
            name: &QString,
            value: &QString,
        ) -> QString;

        #[qinvokable]
        #[rust_name = "delete_environment"]
        fn deleteEnvironment(self: &HyprbindBridge, name: &QString) -> QString;

        #[qinvokable]
        #[rust_name = "variables_snapshot"]
        fn variablesSnapshot(self: &HyprbindBridge) -> QString;

        #[qinvokable]
        #[rust_name = "add_variable"]
        fn addVariable(self: &HyprbindBridge, name: &QString, value: &QString) -> QString;

        #[qinvokable]
        #[rust_name = "edit_variable"]
        fn editVariable(
            self: &HyprbindBridge,
            current_name: &QString,
            name: &QString,
            value: &QString,
        ) -> QString;

        #[qinvokable]
        #[rust_name = "delete_variable"]
        fn deleteVariable(self: &HyprbindBridge, name: &QString) -> QString;
    }
}

#[derive(Default)]
pub struct HyprbindBridgeRust;

impl qobject::HyprbindBridge {
    fn ui_snapshot(&self) -> QString {
        let prefs = crate::ui_prefs::load();
        let config_path = crate::config::default_config_path();
        let has_backup = config_path
            .as_ref()
            .is_some_and(|path| crate::backup::has_backup(path));
        let backup_age = config_path
            .as_ref()
            .and_then(|path| crate::backup::backup_age_label(path))
            .unwrap_or_default();
        to_qstring(json!({
            "ok": true,
            "darkMode": prefs.dark_mode,
            "hasBackup": has_backup,
            "backupAge": backup_age,
        }))
    }

    fn set_dark_mode(&self, enabled: bool) -> QString {
        crate::ui_prefs::update(|prefs| prefs.dark_mode = enabled);
        action_ok(if enabled {
            "Dark glass theme enabled.".into()
        } else {
            "Light glass theme enabled.".into()
        })
    }

    fn restore_config(&self) -> QString {
        let Some(path) = crate::config::default_config_path() else {
            return action_error("Could not determine the Hyprland config path.");
        };
        match crate::backup::restore_last_good(&path) {
            Ok(source) => action_ok(format!(
                "Restored {} from {}",
                path.display(),
                source.display()
            )),
            Err(error) => action_error(error.to_string()),
        }
    }

    fn lookfeel_snapshot(&self) -> QString {
        match crate::config::load_binds(None) {
            Ok(mut collection) => {
                collection.finalize();
                let merged = &collection.config_merged;
                to_qstring(json!({
                    "ok": true,
                    "gapsIn": crate::settings_config::get_i64(merged, &["general", "gaps_in"], 5),
                    "gapsOut": crate::settings_config::get_i64(merged, &["general", "gaps_out"], 20),
                    "borderSize": crate::settings_config::get_i64(merged, &["general", "border_size"], 1),
                    "rounding": crate::settings_config::get_i64(merged, &["decoration", "rounding"], 10),
                    "activeOpacity": crate::settings_config::get_f64(merged, &["decoration", "active_opacity"], 1.0),
                    "inactiveOpacity": crate::settings_config::get_f64(merged, &["decoration", "inactive_opacity"], 1.0),
                    "blurSize": crate::settings_config::get_i64(merged, &["decoration", "blur", "size"], 3),
                    "blurEnabled": crate::settings_config::get_bool(merged, &["decoration", "blur", "enabled"], true),
                    "shadowEnabled": crate::settings_config::get_bool(merged, &["decoration", "shadow", "enabled"], true),
                    "animationsEnabled": crate::settings_config::get_bool(merged, &["animations", "enabled"], true),
                    "activeBorder": crate::settings_config::get_path(merged, &["general", "col", "active_border"])
                        .map(crate::settings_config::color_to_display)
                        .unwrap_or_default(),
                    "inactiveBorder": crate::settings_config::get_path(merged, &["general", "col", "inactive_border"])
                        .map(crate::settings_config::color_to_display)
                        .unwrap_or_default(),
                }))
            }
            Err(error) => action_error(error.to_string()),
        }
    }

    fn save_lookfeel(&self, payload: &QString) -> QString {
        let input = match parse_json_payload(payload) {
            Ok(value) => value,
            Err(error) => return action_error(error),
        };
        let mut collection = match crate::config::load_binds(None) {
            Ok(collection) => collection,
            Err(error) => return action_error(error.to_string()),
        };
        collection.finalize();
        let mut merged = collection.config_merged.clone();
        if !merged.is_object() {
            merged = json!({});
        }
        apply_lookfeel_payload(&mut merged, &input);
        match crate::writer::save_config_override(&collection.config_path, &merged) {
            Ok(result) => action_ok(format!("Saved Look & Feel to {}", result.path)),
            Err(error) => action_error(error.to_string()),
        }
    }

    fn preview_lookfeel(&self, payload: &QString) -> QString {
        let input = match parse_json_payload(payload) {
            Ok(value) => value,
            Err(error) => return action_error(error),
        };
        let expr = build_lookfeel_eval(&input);
        let output = match std::process::Command::new("hyprctl")
            .args(["eval", &expr])
            .output()
        {
            Ok(output) => output,
            Err(error) => return action_error(error.to_string()),
        };
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{stdout}{stderr}").to_ascii_lowercase();
        if !output.status.success() || combined.contains("error") || combined.contains("can't work")
        {
            let message = if !stderr.trim().is_empty() {
                stderr.trim()
            } else {
                stdout.trim()
            };
            return action_error(if message.is_empty() {
                "Live preview failed"
            } else {
                message
            });
        }
        action_ok("Live preview applied.".into())
    }

    fn environment_snapshot(&self) -> QString {
        match crate::config::load_binds(None) {
            Ok(mut collection) => {
                collection.finalize();
                let env = collection
                    .env
                    .iter()
                    .map(|item| {
                        json!({
                            "name": item.name,
                            "value": item.value,
                            "sourceFile": item.source_file,
                            "sourceLine": item.source_line,
                        })
                    })
                    .collect::<Vec<_>>();
                to_qstring(json!({
                    "ok": true,
                    "configPath": collection.config_path.display().to_string(),
                    "env": env,
                    "warning": collection.error.clone().unwrap_or_default(),
                }))
            }
            Err(error) => to_qstring(json!({
                "ok": false,
                "configPath": crate::config::default_config_path()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default(),
                "env": [],
                "error": error.to_string(),
            })),
        }
    }

    fn add_environment(&self, name: &QString, value: &QString) -> QString {
        let name = String::from(name);
        let value = String::from(value);
        let Some(path) = crate::config::default_config_path() else {
            return action_error("Could not determine the Hyprland config path.");
        };
        match crate::writer::add_env(&path, &name, &value) {
            Ok(result) => action_ok(format!("Added {name} in {}", result.path)),
            Err(error) => action_error(error.to_string()),
        }
    }

    fn edit_environment(&self, current_name: &QString, name: &QString, value: &QString) -> QString {
        let current_name = String::from(current_name);
        let name = String::from(name);
        let value = String::from(value);
        let mut collection = match crate::config::load_binds(None) {
            Ok(collection) => collection,
            Err(error) => return action_error(error.to_string()),
        };
        collection.finalize();
        let Some(var) = collection.env.iter().find(|item| item.name == current_name) else {
            return action_error(format!(
                "Environment variable `{current_name}` is no longer present."
            ));
        };
        match crate::writer::save_env(var, &name, &value) {
            Ok(result) => action_ok(format!("Saved {name} in {}", result.path)),
            Err(error) => action_error(error.to_string()),
        }
    }

    fn delete_environment(&self, name: &QString) -> QString {
        let name = String::from(name);
        let mut collection = match crate::config::load_binds(None) {
            Ok(collection) => collection,
            Err(error) => return action_error(error.to_string()),
        };
        collection.finalize();
        let Some(var) = collection.env.iter().find(|item| item.name == name) else {
            return action_error(format!(
                "Environment variable `{name}` is no longer present."
            ));
        };
        match crate::writer::delete_env(var) {
            Ok(result) => action_ok(format!("Deleted {name} from {}", result.path)),
            Err(error) => action_error(error.to_string()),
        }
    }

    fn variables_snapshot(&self) -> QString {
        match crate::config::load_binds(None) {
            Ok(mut collection) => {
                collection.finalize();
                let variables = collection
                    .variables
                    .iter()
                    .map(|item| {
                        json!({
                            "name": item.name,
                            "value": item.value,
                            "sourceFile": item.source_file,
                            "sourceLine": item.source_line,
                        })
                    })
                    .collect::<Vec<_>>();
                to_qstring(json!({
                    "ok": true,
                    "configPath": collection.config_path.display().to_string(),
                    "variables": variables,
                    "warning": collection.error.clone().unwrap_or_default(),
                }))
            }
            Err(error) => to_qstring(json!({
                "ok": false,
                "configPath": crate::config::default_config_path()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default(),
                "variables": [],
                "error": error.to_string(),
            })),
        }
    }

    fn add_variable(&self, name: &QString, value: &QString) -> QString {
        let name = String::from(name);
        let value = String::from(value);
        let Some(path) = crate::config::default_config_path() else {
            return action_error("Could not determine the Hyprland config path.");
        };
        match crate::writer::add_variable(&path, &name, &value) {
            Ok(result) => action_ok(format!("Added {name} in {}", result.path)),
            Err(error) => action_error(error.to_string()),
        }
    }

    fn edit_variable(&self, current_name: &QString, name: &QString, value: &QString) -> QString {
        let current_name = String::from(current_name);
        let name = String::from(name);
        let value = String::from(value);
        let mut collection = match crate::config::load_binds(None) {
            Ok(collection) => collection,
            Err(error) => return action_error(error.to_string()),
        };
        collection.finalize();
        let related_files = collection.related_files();
        let existing_names = collection
            .variables
            .iter()
            .map(|item| item.name.clone())
            .collect::<Vec<_>>();
        let Some(var) = collection
            .variables
            .iter()
            .find(|item| item.name == current_name)
        else {
            return action_error(format!("Variable `{current_name}` is no longer present."));
        };
        match crate::writer::save_variable(var, &name, &value, &related_files, &existing_names) {
            Ok(result) => action_ok(format!("Saved {name} in {}", result.path)),
            Err(error) => action_error(error.to_string()),
        }
    }

    fn delete_variable(&self, name: &QString) -> QString {
        let name = String::from(name);
        let mut collection = match crate::config::load_binds(None) {
            Ok(collection) => collection,
            Err(error) => return action_error(error.to_string()),
        };
        collection.finalize();
        let Some(var) = collection.variables.iter().find(|item| item.name == name) else {
            return action_error(format!("Variable `{name}` is no longer present."));
        };
        match crate::writer::delete_variable(var) {
            Ok(result) => action_ok(format!("Deleted {name} from {}", result.path)),
            Err(error) => action_error(error.to_string()),
        }
    }
}

fn parse_json_payload(payload: &QString) -> Result<Value, String> {
    serde_json::from_str::<Value>(&String::from(payload)).map_err(|error| error.to_string())
}

fn payload_i64(input: &Value, key: &str, fallback: i64) -> i64 {
    input.get(key).and_then(Value::as_i64).unwrap_or(fallback)
}

fn payload_f64(input: &Value, key: &str, fallback: f64) -> f64 {
    input.get(key).and_then(Value::as_f64).unwrap_or(fallback)
}

fn payload_bool(input: &Value, key: &str, fallback: bool) -> bool {
    input.get(key).and_then(Value::as_bool).unwrap_or(fallback)
}

fn payload_string(input: &Value, key: &str) -> String {
    input
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn apply_lookfeel_payload(merged: &mut Value, input: &Value) {
    crate::settings_config::set_path(
        merged,
        &["general", "gaps_in"],
        json!(payload_i64(input, "gapsIn", 5)),
    );
    crate::settings_config::set_path(
        merged,
        &["general", "gaps_out"],
        json!(payload_i64(input, "gapsOut", 20)),
    );
    crate::settings_config::set_path(
        merged,
        &["general", "border_size"],
        json!(payload_i64(input, "borderSize", 1)),
    );
    crate::settings_config::set_path(
        merged,
        &["decoration", "rounding"],
        json!(payload_i64(input, "rounding", 10)),
    );
    crate::settings_config::set_path(
        merged,
        &["decoration", "active_opacity"],
        json!(payload_f64(input, "activeOpacity", 1.0)),
    );
    crate::settings_config::set_path(
        merged,
        &["decoration", "inactive_opacity"],
        json!(payload_f64(input, "inactiveOpacity", 1.0)),
    );
    crate::settings_config::set_path(
        merged,
        &["decoration", "blur", "enabled"],
        json!(payload_bool(input, "blurEnabled", true)),
    );
    crate::settings_config::set_path(
        merged,
        &["decoration", "blur", "size"],
        json!(payload_i64(input, "blurSize", 3)),
    );
    crate::settings_config::set_path(
        merged,
        &["decoration", "shadow", "enabled"],
        json!(payload_bool(input, "shadowEnabled", true)),
    );
    crate::settings_config::set_path(
        merged,
        &["animations", "enabled"],
        json!(payload_bool(input, "animationsEnabled", true)),
    );
    let active = payload_string(input, "activeBorder");
    if !active.is_empty() {
        crate::settings_config::set_path(
            merged,
            &["general", "col", "active_border"],
            crate::settings_config::parse_color_input(&active),
        );
    }
    let inactive = payload_string(input, "inactiveBorder");
    if !inactive.is_empty() {
        crate::settings_config::set_path(
            merged,
            &["general", "col", "inactive_border"],
            crate::settings_config::parse_color_input(&inactive),
        );
    }
}

fn lua_escape(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

fn build_lookfeel_eval(input: &Value) -> String {
    let gaps_in = payload_i64(input, "gapsIn", 5);
    let gaps_out = payload_i64(input, "gapsOut", 20);
    let border = payload_i64(input, "borderSize", 1);
    let rounding = payload_i64(input, "rounding", 10);
    let active_op = payload_f64(input, "activeOpacity", 1.0);
    let inactive_op = payload_f64(input, "inactiveOpacity", 1.0);
    let blur_size = payload_i64(input, "blurSize", 3);
    let blur = payload_bool(input, "blurEnabled", true);
    let shadow = payload_bool(input, "shadowEnabled", true);
    let anims = payload_bool(input, "animationsEnabled", true);
    let mut colors = Vec::new();
    let active = payload_string(input, "activeBorder");
    let inactive = payload_string(input, "inactiveBorder");
    if !active.is_empty() {
        colors.push(format!("active_border = \"{}\"", lua_escape(&active)));
    }
    if !inactive.is_empty() {
        colors.push(format!("inactive_border = \"{}\"", lua_escape(&inactive)));
    }
    let general = if colors.is_empty() {
        format!(
            "general = {{ gaps_in = {gaps_in}, gaps_out = {gaps_out}, border_size = {border} }}"
        )
    } else {
        format!(
            "general = {{ gaps_in = {gaps_in}, gaps_out = {gaps_out}, border_size = {border}, col = {{ {} }} }}",
            colors.join(", ")
        )
    };
    format!(
        "hl.config({{ {general}, decoration = {{ rounding = {rounding}, active_opacity = {active_op:.2}, inactive_opacity = {inactive_op:.2}, blur = {{ enabled = {blur}, size = {blur_size} }}, shadow = {{ enabled = {shadow} }} }}, animations = {{ enabled = {anims} }} }})"
    )
}

fn to_qstring(value: serde_json::Value) -> QString {
    QString::from(value.to_string())
}

fn action_ok(message: String) -> QString {
    to_qstring(json!({ "ok": true, "message": message }))
}

fn action_error(message: impl Into<String>) -> QString {
    to_qstring(json!({ "ok": false, "error": message.into() }))
}

pub fn run() {
    use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};

    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/dev/hyprbinds/ui/qml/Main.qml"));
    }
    if let Some(app) = app.as_mut() {
        app.exec();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookfeel_payload_maps_to_config_and_live_eval() {
        let input = json!({
            "gapsIn": 7,
            "gapsOut": 19,
            "borderSize": 2,
            "rounding": 14,
            "activeOpacity": 0.97,
            "inactiveOpacity": 0.84,
            "blurSize": 6,
            "blurEnabled": true,
            "shadowEnabled": false,
            "animationsEnabled": true,
            "activeBorder": "rgba(37d5e9ee)",
            "inactiveBorder": "rgba(6b747baa)"
        });
        let mut merged = json!({});
        apply_lookfeel_payload(&mut merged, &input);
        assert_eq!(
            crate::settings_config::get_i64(&merged, &["general", "gaps_in"], -1),
            7
        );
        assert_eq!(
            crate::settings_config::get_i64(&merged, &["decoration", "rounding"], -1),
            14
        );
        assert_eq!(
            crate::settings_config::get_bool(&merged, &["decoration", "blur", "enabled"], false),
            true
        );
        assert_eq!(
            crate::settings_config::get_bool(&merged, &["decoration", "shadow", "enabled"], true),
            false
        );
        let eval = build_lookfeel_eval(&input);
        assert!(eval.contains("gaps_in = 7"));
        assert!(eval.contains("rounding = 14"));
        assert!(eval.contains("size = 6"));
        assert!(eval.contains("active_border = \"rgba(37d5e9ee)\""));
    }

    #[test]
    fn lookfeel_config_write_uses_existing_managed_writer() {
        let unique = format!(
            "hyprbind-lookfeel-{}-{}.lua",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::write(&path, "-- untouched user config\n").unwrap();
        let input = json!({ "gapsIn": 9, "gapsOut": 21, "borderSize": 2, "rounding": 12 });
        let mut merged = json!({});
        apply_lookfeel_payload(&mut merged, &input);
        crate::writer::save_config_override(&path, &merged).unwrap();
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.contains("-- untouched user config"));
        assert!(written.contains("hyprbinds"));
        assert!(written.contains("gaps_in"));
        assert!(crate::backup::sidecar_backup(&path).is_file());
        let _ = std::fs::remove_file(crate::backup::sidecar_backup(&path));
        let _ = std::fs::remove_file(path);
    }
}
