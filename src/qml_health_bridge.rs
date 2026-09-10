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
        type HealthBridge = super::HealthBridgeRust;

        #[qinvokable]
        #[rust_name = "snapshot"]
        fn snapshot(self: &HealthBridge) -> QString;

        #[qinvokable]
        #[rust_name = "system_info"]
        fn systemInfo(self: &HealthBridge) -> QString;
    }
}

#[derive(Default)]
pub struct HealthBridgeRust;

impl qobject::HealthBridge {
    fn system_info(&self) -> QString {
        QString::from(crate::sysinfo::collect_report())
    }

    fn snapshot(&self) -> QString {
        let checks = crate::health::run_all();
        let rows = checks
            .iter()
            .map(|item| {
                json!({
                    "category": item.category,
                    "title": item.title,
                    "detail": item.detail,
                    "severity": item.severity.label(),
                    "fixHint": item.fix_hint.clone().unwrap_or_default(),
                    "fixCommand": item.fix_command.clone().unwrap_or_default(),
                })
            })
            .collect::<Vec<_>>();
        let (ok_count, warnings, failures) = crate::health::summarize(&checks);
        to_qstring(json!({
            "ok": true,
            "checks": rows,
            "okCount": ok_count,
            "failures": failures,
            "warnings": warnings,
            "total": checks.len(),
        }))
    }
}

fn to_qstring(value: serde_json::Value) -> QString {
    QString::from(value.to_string())
}
