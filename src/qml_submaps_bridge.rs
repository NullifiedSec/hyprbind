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
        type SubmapsBridge = super::SubmapsBridgeRust;

        #[qinvokable]
        #[rust_name = "snapshot"]
        fn snapshot(self: &SubmapsBridge) -> QString;

        #[qinvokable]
        #[rust_name = "add_submap"]
        fn addSubmap(self: &SubmapsBridge, name: &QString) -> QString;

        #[qinvokable]
        #[rust_name = "delete_submap"]
        fn deleteSubmap(self: &SubmapsBridge, id: i32) -> QString;
    }
}

#[derive(Default)]
pub struct SubmapsBridgeRust;

impl qobject::SubmapsBridge {
    fn snapshot(&self) -> QString {
        match crate::config::load_binds(None) {
            Ok(mut collection) => {
                collection.finalize();
                let submaps = collection
                    .submaps
                    .iter()
                    .map(|item| {
                        json!({
                            "entryId": item.id,
                            "name": item.name,
                            "reset": item.reset,
                            "bindCount": item.bind_count,
                            "sourceFile": item.source_file,
                            "sourceLine": item.source_line,
                        })
                    })
                    .collect::<Vec<_>>();
                to_qstring(json!({
                    "ok": true,
                    "configPath": collection.config_path.display().to_string(),
                    "submaps": submaps,
                    "warning": collection.error.clone().unwrap_or_default(),
                }))
            }
            Err(error) => to_qstring(json!({
                "ok": false,
                "configPath": crate::config::default_config_path()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default(),
                "submaps": [],
                "error": error.to_string(),
            })),
        }
    }

    fn add_submap(&self, name: &QString) -> QString {
        let name = String::from(name);
        let Some(path) = crate::config::default_config_path() else {
            return action_error("Could not determine the Hyprland config path.");
        };
        match crate::writer::add_submap(&path, &name) {
            Ok(result) => action_ok(format!("Added submap {name} in {}", result.path)),
            Err(error) => action_error(error.to_string()),
        }
    }

    fn delete_submap(&self, id: i32) -> QString {
        if id < 0 {
            return action_error("Invalid submap id.");
        }
        let mut collection = match crate::config::load_binds(None) {
            Ok(collection) => collection,
            Err(error) => return action_error(error.to_string()),
        };
        collection.finalize();
        let Some(submap) = collection.submap_by_id(id as usize) else {
            return action_error(format!("Submap `{id}` is no longer present."));
        };
        match crate::writer::delete_submap(submap) {
            Ok(result) => action_ok(format!("Deleted submap {} from {}", submap.name, result.path)),
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
