import QtQuick
import QtQuick.Controls

Item {
    id: root

    required property var bridge
    property bool darkMode: true
    property bool hyprlandSelected: true
    property bool waybarSelected: true
    property bool appSelected: true
    property bool viaSelected: true
    property bool systemSelected: true

    signal status(string message)
    signal imported()

    HyprbindTheme { id: theme; darkMode: root.darkMode }

    function parse(raw) {
        try { return JSON.parse(raw) }
        catch (error) { return { ok: false, error: "Invalid bridge response: " + error } }
    }

    function doExport() {
        const result = parse(bridge.exportBundle(exportPath.text))
        status(result.ok ? result.message : (result.error || "Export failed."))
    }

    function doImport() {
        const result = parse(bridge.importBundle(
            importPath.text,
            hyprlandSelected,
            waybarSelected,
            appSelected,
            viaSelected,
            systemSelected
        ))
        status(result.ok ? result.message : (result.error || "Import failed."))
        if (result.ok) imported()
    }

    Flickable {
        anchors.fill: parent
        contentWidth: width
        contentHeight: content.implicitHeight
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

        Column {
            id: content
            width: parent.width
            spacing: 12

            GlassPanel {
                width: parent.width
                height: 166
                darkMode: root.darkMode
                elevated: true

                Column {
                    anchors.fill: parent
                    anchors.margins: 18
                    spacing: 10

                    Text { text: "EXPORT BUNDLE"; color: theme.textPrimary; font.family: "Inter"; font.pixelSize: 12; font.weight: Font.DemiBold; font.letterSpacing: 0.7 }
                    Text { width: parent.width; text: "Write Hyprland settings, Waybar, app preferences, VIA assets, and supported system config to one JSON bundle."; color: theme.textSecondary; font.family: "Inter"; font.pixelSize: 11; wrapMode: Text.Wrap }
                    Row {
                        width: parent.width
                        spacing: 8
                        GlassField { id: exportPath; width: Math.max(160, parent.width - exportButton.width - 8); text: "~/hyprbinds-export.json"; darkMode: root.darkMode; placeholderText: "~/hyprbinds-export.json" }
                        GlassButton { id: exportButton; text: "Export"; accent: true; darkMode: root.darkMode; onClicked: root.doExport() }
                    }
                }
            }

            GlassPanel {
                width: parent.width
                height: 410
                darkMode: root.darkMode
                elevated: false

                Column {
                    anchors.fill: parent
                    anchors.margins: 18
                    spacing: 8

                    Text { text: "IMPORT BUNDLE"; color: theme.textPrimary; font.family: "Inter"; font.pixelSize: 12; font.weight: Font.DemiBold; font.letterSpacing: 0.7 }
                    Text { width: parent.width; text: "Restore selected sections through the existing Rust import path. Managed Hyprland sections are backed up before replacement; unmanaged Lua remains outside Hyprbind markers."; color: theme.textSecondary; font.family: "Inter"; font.pixelSize: 11; wrapMode: Text.Wrap }

                    GlassField { id: importPath; width: parent.width; text: "~/hyprbinds-export.json"; darkMode: root.darkMode; placeholderText: "Path to hyprbinds export JSON" }

                    SettingToggle { width: parent.width; darkMode: root.darkMode; title: "Hyprland"; subtitle: "Binds, rules, monitors, env, startup, and managed settings"; checked: root.hyprlandSelected; onToggled: checked => root.hyprlandSelected = checked }
                    SettingToggle { width: parent.width; darkMode: root.darkMode; title: "Waybar"; subtitle: "Restore exported Waybar configuration"; checked: root.waybarSelected; onToggled: checked => root.waybarSelected = checked }
                    SettingToggle { width: parent.width; darkMode: root.darkMode; title: "App preferences"; subtitle: "Theme, developer mode, and Hyprbind preferences"; checked: root.appSelected; onToggled: checked => root.appSelected = checked }
                    SettingToggle { width: parent.width; darkMode: root.darkMode; title: "VIA assets"; subtitle: "VIA definitions and RGB presets"; checked: root.viaSelected; onToggled: checked => root.viaSelected = checked }
                    SettingToggle { width: parent.width; darkMode: root.darkMode; title: "Portal config"; subtitle: "Supported desktop portal configuration"; checked: root.systemSelected; onToggled: checked => root.systemSelected = checked }

                    Item {
                        width: parent.width
                        height: 42
                        GlassButton { anchors.right: parent.right; text: "Import selected"; accent: true; darkMode: root.darkMode; enabled: root.hyprlandSelected || root.waybarSelected || root.appSelected || root.viaSelected || root.systemSelected; onClicked: root.doImport() }
                    }
                }
            }
        }
    }
}
