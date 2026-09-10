import QtQuick
import QtQuick.Controls

Item {
    id: root

    required property var bridge
    property bool darkMode: true
    property bool developerMode: false

    signal status(string message)
    signal developerModeChanged(bool enabled)

    HyprbindTheme { id: theme; darkMode: root.darkMode }

    function parse(raw) {
        try { return JSON.parse(raw) }
        catch (error) { return { ok: false, error: "Invalid bridge response: " + error } }
    }

    function setDeveloperMode(enabled) {
        const result = parse(bridge.setDeveloperMode(enabled))
        if (!result.ok) {
            status(result.error || "Could not update developer mode.")
            return
        }
        developerModeChanged(enabled)
        status(result.message)
    }

    Column {
        anchors.fill: parent
        spacing: 12

        GlassPanel {
            width: parent.width
            height: 118
            darkMode: root.darkMode
            elevated: true

            Column {
                anchors.fill: parent
                anchors.margins: 18
                spacing: 8
                Text { text: "EXPERIMENTAL COMPANION STUDIOS"; color: theme.textPrimary; font.family: "Inter"; font.pixelSize: 12; font.weight: Font.DemiBold; font.letterSpacing: 0.7 }
                Text { width: parent.width; text: "Wallpaper, Waybar, Starship, VIA, Screenshare, and Audio are still hosted by the GTK compatibility UI while their QML migration is intentionally deferred."; color: theme.textSecondary; font.family: "Inter"; font.pixelSize: 11; wrapMode: Text.Wrap }
            }
        }

        GlassPanel {
            width: parent.width
            height: 176
            darkMode: root.darkMode
            elevated: false

            Column {
                anchors.fill: parent
                anchors.margins: 18
                spacing: 8

                SettingToggle {
                    width: parent.width
                    darkMode: root.darkMode
                    title: "Developer mode"
                    subtitle: "Unlock the existing experimental companion studios"
                    checked: root.developerMode
                    onToggled: checked => root.setDeveloperMode(checked)
                }

                Rectangle { width: parent.width; height: 1; color: theme.divider }

                Item {
                    width: parent.width
                    height: 48
                    Text { anchors.left: parent.left; anchors.verticalCenter: parent.verticalCenter; text: root.developerMode ? "Developer tools are unlocked." : "Enable developer mode to open the experimental studios."; color: theme.textSecondary; font.family: "Inter"; font.pixelSize: 11 }
                    GlassButton {
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        text: "Open GTK tools"
                        accent: true
                        darkMode: root.darkMode
                        enabled: root.developerMode
                        onClicked: {
                            const result = root.parse(root.bridge.launchGtkUi())
                            root.status(result.ok ? result.message : (result.error || "Could not launch GTK compatibility UI."))
                        }
                    }
                }
            }
        }
    }
}
