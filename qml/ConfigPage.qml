import QtQuick
import QtQuick.Controls
import dev.hyprbinds.ui

Item {
    id: root

    property bool darkMode: true
    property string configPath: ""
    property string originalText: "{}"
    property bool dirty: configEditor.text !== originalText
    readonly property bool compactToolbar: width < 620

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
            id: toolbar
            width: parent.width
            height: root.compactToolbar ? 82 : 46

            Row {
                id: actions
                spacing: 7
                x: Math.max(0, toolbar.width - width)
                y: root.compactToolbar
                    ? Math.max(0, toolbar.height - height)
                    : Math.round((toolbar.height - height) / 2)

                IconButton {
                    iconName: "reload"
                    tooltip: "Discard edits and reload"
                    darkMode: root.darkMode
                    onClicked: root.load()
                }
                IconButton {
                    iconName: "save"
                    tooltip: root.dirty ? "Save config override" : "No unsaved changes"
                    darkMode: root.darkMode
                    accent: root.dirty
                    enabled: root.dirty
                    onClicked: root.save()
                }
            }

            Text {
                id: hint
                x: 0
                y: root.compactToolbar
                    ? 0
                    : Math.round((toolbar.height - height) / 2)
                width: root.compactToolbar
                    ? toolbar.width
                    : Math.max(0, actions.x - 14)
                text: "Advanced merged-config editor · typed JSON is preserved by the existing managed writer."
                color: theme.textSecondary
                font.family: "Inter"
                font.pixelSize: 10
                elide: Text.ElideRight
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
