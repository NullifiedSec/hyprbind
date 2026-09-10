use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(
        QmlModule::new("dev.hyprbinds.ui")
            .qml_files([
                "qml/Main.qml",
                "qml/GlassPanel.qml",
                "qml/GlassField.qml",
                "qml/GlassButton.qml",
                "qml/IconButton.qml",
                "qml/NavItem.qml",
                "qml/UiIcon.qml",
                "qml/HyprbindTheme.qml",
                "qml/LookFeelPage.qml",
                "qml/SettingsCard.qml",
                "qml/SettingField.qml",
                "qml/SettingToggle.qml",
                "qml/SettingSlider.qml",
            ])
            .depend("QtQuick")
            .depend("QtQuick.Controls")
            .depend("QtQuick.Layouts"),
    )
    .files(["src/qml_bridge.rs"])
    .qt_module("Quick")
    .qt_module("QuickControls2")
    .build();
}
