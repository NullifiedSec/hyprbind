use cxx_qt_lib::QString;
use serde_json::json;

#[cxx_qt::bridge]
mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        type StartupBridge = super::StartupBridgeRust;

        #[qinvokable]
        #[rust_name = "snapshot"]
        fn snapshot(self: &StartupBridge) -> QString;

        #[qinvokable]
        #[rust_name = "add_entry"]
        fn addEntry(
            self: &StartupBridge,
            command: &QString,
            when: &QString,
            workspace: &QString,
        ) -> QString;

        #[qinvokable]
        #[rust_name = "edit_entry"]
        fn editEntry(
            self: &StartupBridge,
            id: i32,
            command: &QString,
            when: &QString,
            workspace: &QString,
        ) -> QString;

        #[qinvokable]
        #[rust_name = "delete_entry"]
        fn deleteEntry(self: &StartupBridge, id: i32) -> QString;
    }
}

#[derive(Default)]
pub struct StartupBridgeRust;

impl qobject::StartupBridge {
    fn snapshot(&self) -> QString {
        match crate::config::load_binds(None) {
            Ok(mut collection) => {
                collection.finalize();
                let entries = collection
                    .startup
                    .iter()
                    .map(|item| {
                        json!({
                            "id": item.id,
                            "command": item.command,
                            "when": item.when,
                            "workspace": item.workspace,
                            "sourceFile": item.source_file,
                            "sourceLine": item.source_line,
                        })
                    })
                    .collect::<Vec<_>>();
                to_qstring(json!({
                    "ok": true,
                    "configPath": collection.config_path.display().to_string(),
                    "entries": entries,
                    "warning": collection.error.clone().unwrap_or_default(),
                }))
            }
            Err(error) => to_qstring(json!({
                "ok": false,
                "configPath": crate::config::default_config_path()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default(),
                "entries": [],
                "error": error.to_string(),
            })),
        }
    }

    fn add_entry(&self, command: &QString, when: &QString, workspace: &QString) -> QString {
        let command = String::from(command);
        let when = String::from(when);
        let workspace = String::from(workspace);
        let Some(path) = crate::config::default_config_path() else {
            return action_error("Could not determine the Hyprland config path.");
        };
        match crate::writer::add_startup(&path, &command, &when, &workspace) {
            Ok(result) => action_ok(format!("Added startup entry in {}", result.path)),
            Err(error) => action_error(error.to_string()),
        }
    }

    fn edit_entry(
        &self,
        id: i32,
        command: &QString,
        when: &QString,
        workspace: &QString,
    ) -> QString {
        if id < 0 {
            return action_error("Invalid startup entry id.");
        }
        let command = String::from(command);
        let when = String::from(when);
        let workspace = String::from(workspace);
        let mut collection = match crate::config::load_binds(None) {
            Ok(collection) => collection,
            Err(error) => return action_error(error.to_string()),
        };
        collection.finalize();
        let Some(entry) = collection.startup_by_id(id as usize) else {
            return action_error(format!("Startup entry `{id}` is no longer present."));
        };
        match crate::writer::save_startup(entry, &command, &when, &workspace) {
            Ok(result) => action_ok(format!("Saved startup entry in {}", result.path)),
            Err(error) => action_error(error.to_string()),
        }
    }

    fn delete_entry(&self, id: i32) -> QString {
        if id < 0 {
            return action_error("Invalid startup entry id.");
        }
        let mut collection = match crate::config::load_binds(None) {
            Ok(collection) => collection,
            Err(error) => return action_error(error.to_string()),
        };
        collection.finalize();
        let Some(entry) = collection.startup_by_id(id as usize) else {
            return action_error(format!("Startup entry `{id}` is no longer present."));
        };
        match crate::writer::delete_startup(entry) {
            Ok(result) => action_ok(format!("Deleted startup entry from {}", result.path)),
            Err(error) => action_error(error.to_string()),
        }
    }
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
