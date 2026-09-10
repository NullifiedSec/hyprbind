import QtQuick
import QtQuick.Controls
import dev.hyprbinds.ui

Item {
    id: root

    property bool darkMode: true
    property string configPath: ""
    property string originalText: "{}"
    property bool dirty: configEditor.text !== originalText

    signal status(string message)
    signal configResolved(string path)
    signal healthChanged(bool healthy)

    HyprbindTheme { id: theme; darkMode: root.darkMode }
    CatalogBridge { id: catalog }

    function parse(raw) {
        try { return JSON.parse(raw) }
        catch (error) { return { ok: false, error: "Invalid Rust bridge response: " + error } }
    }

    function load() {
        const payload = parse(catalog.configSnapshot())
        configPath = payload.configPath || ""
        configResolved(configPath)
        healthChanged(payload.ok === true)
        if (!payload.ok) {
            configEditor.text = "{}"
            originalText = configEditor.text
            status(payload.error || "Could not load config.")
            return
        }
        configEditor.text = JSON.stringify(payload.config || {}, null, 2)
        originalText = configEditor.text
        status(payload.warning || "Merged config loaded. Changes are written through the managed override writer.")
    }

    function save() {
        let parsed
        try {
            parsed = JSON.parse(configEditor.text)
            if (!parsed || Array.isArray(parsed) || typeof parsed !== "object")
                throw new Error("top level must be an object")
        } catch (error) {
            status("Config JSON is invalid: " + error)
            return
        }
        const payload = parse(catalog.saveConfig(JSON.stringify({ config: parsed })))
        if (!payload.ok) {
            status(payload.error || "Config save failed.")
            return
        }
        originalText = configEditor.text
        status(payload.message || "Config saved.")
    }

    Component.onCompleted: load()

    Column {
        anchors.fill: parent
        spacing: 10

        Item {
            width: parent.width
            height: root.width < 620 ? 82 : 46

            Text {
                id: hint
                anchors.left: parent.left
                anchors.right: root.width < 620 ? parent.right : actions.left
                anchors.rightMargin: root.width < 620 ? 0 : 14
                anchors.top: parent.top
                anchors.verticalCenter: root.width < 620 ? undefined : parent.verticalCenter
                text: "Advanced merged-config editor · typed JSON is preserved by the existing managed writer."
                color: theme.textSecondary
                font.family: "Inter"
                font.pixelSize: 10
                elide: Text.ElideRight
            }

            Row {
                id: actions
                anchors.right: parent.right
                anchors.top: root.width < 620 ? hint.bottom : parent.top
                anchors.topMargin: root.width < 620 ? 7 : 3
                spacing: 7
                IconButton { iconName: "reload"; tooltip: "Discard edits and reload"; darkMode: root.darkMode; onClicked: root.load() }
                IconButton { iconName: "save"; tooltip: root.dirty ? "Save config override" : "No unsaved changes"; darkMode: root.darkMode; accent: root.dirty; enabled: root.dirty; onClicked: root.save() }
            }
        }

        GlassTextArea {
            id: configEditor
            width: parent.width
            height: parent.height - y
            darkMode: root.darkMode
            placeholderText: "{}"
        }
    }
}
