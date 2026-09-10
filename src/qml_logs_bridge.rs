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
        type LogsBridge = super::LogsBridgeRust;

        #[qinvokable]
        #[rust_name = "snapshot"]
        fn snapshot(self: &LogsBridge) -> QString;
    }
}

#[derive(Default)]
pub struct LogsBridgeRust;

impl qobject::LogsBridge {
    fn snapshot(&self) -> QString {
        match crate::logs::fetch(300) {
            Ok(entries) => {
                let rows = entries
                    .iter()
                    .map(|item| {
                        json!({
                            "id": item.id,
                            "timestamp": item.timestamp_display,
                            "severity": crate::logs::severity_label(item.severity),
                            "message": item.message,
                            "identifier": item.identifier,
                            "unit": item.unit,
                            "tags": item.tags,
                        })
                    })
                    .collect::<Vec<_>>();
                let tags = crate::logs::known_tags(&entries);
                to_qstring(json!({
                    "ok": true,
                    "entries": rows,
                    "tags": tags,
                    "count": entries.len(),
                }))
            }
            Err(error) => to_qstring(json!({
                "ok": false,
                "entries": [],
                "tags": [],
                "count": 0,
                "error": error,
            })),
        }
    }
}

fn to_qstring(value: serde_json::Value) -> QString {
    QString::from(value.to_string())
}
