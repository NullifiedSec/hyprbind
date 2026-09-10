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
        type WindowControlBridge = super::WindowControlBridgeRust;

        #[qinvokable]
        #[rust_name = "toggle_maximize"]
        fn toggleMaximize(self: &WindowControlBridge) -> QString;
    }
}

#[derive(Default)]
pub struct WindowControlBridgeRust;

impl qobject::WindowControlBridge {
    fn toggle_maximize(&self) -> QString {
        const EXPR: &str =
            "hl.dispatch(hl.dsp.window.fullscreen({ mode = \"maximized\", action = \"toggle\" }))";

        let output = match std::process::Command::new("hyprctl")
            .args(["eval", EXPR])
            .output()
        {
            Ok(output) => output,
            Err(error) => return response(false, error.to_string()),
        };

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let combined = format!("{stdout}\n{stderr}").to_ascii_lowercase();
        if output.status.success()
            && !combined.contains("error")
            && !combined.contains("failed")
        {
            response(true, if stdout.is_empty() { "ok".into() } else { stdout })
        } else {
            response(
                false,
                if !stderr.is_empty() {
                    stderr
                } else if !stdout.is_empty() {
                    stdout
                } else {
                    "Hyprland maximize request failed".into()
                },
            )
        }
    }
}

fn response(ok: bool, message: String) -> QString {
    QString::from(json!({ "ok": ok, "message": message }).to_string())
}
