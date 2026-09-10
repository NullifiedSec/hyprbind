import QtQuick
import QtQuick.Controls
import dev.hyprbinds.ui

Item {
    id: root

    required property string kind
    property bool darkMode: true
    property var allItems: []
    property int selectedId: -1
    property string configPath: ""
    readonly property bool compactToolbar: width < 700
    readonly property bool layerMode: kind === "layer_rules"

    signal status(string message)
    signal configResolved(string path)
    signal healthChanged(bool healthy)

    HyprbindTheme { id: theme; darkMode: root.darkMode }
    CatalogBridge { id: catalog }
    ListModel { id: model }

    function parse(raw) {
        try { return JSON.parse(raw) }
        catch (error) { return { ok: false, error: "Invalid Rust bridge response: " + error } }
    }

    function selectedItem() {
        for (let i = 0; i < allItems.length; ++i)
            if (allItems[i].entryId === selectedId) return allItems[i]
        return null
    }

    function load() {
        const payload = parse(catalog.snapshot(kind))
        allItems = payload.items || []
        configPath = payload.configPath || ""
        configResolved(configPath)
        healthChanged(payload.ok === true)
        if (!payload.ok) status(payload.error || "Could not load rules.")
        else if (payload.warning) status(payload.warning)
        else status((layerMode ? "Layer" : "Window") + " rules loaded from config.")
        if (selectedId >= 0 && !selectedItem()) selectedId = -1
        rebuildModel()
    }

    function rebuildModel() {
        const q = searchField.text.trim().toLowerCase()
        model.clear()
        for (let i = 0; i < allItems.length; ++i) {
            const item = allItems[i]
            const haystack = (item.displayName + " " + item.matchLabel + " " + item.effectsLabel + " " + item.sourceLabel).toLowerCase()
            if (!q || haystack.indexOf(q) >= 0) model.append(item)
        }
    }

    function openEditor(item) {
        editor.entryId = item ? item.entryId : -1
        editorName.text = item ? (item.name || item.displayName) : ""
        matchText.text = JSON.stringify(item ? item.match : {}, null, 2)
        effectsText.text = JSON.stringify(item ? item.effects : {}, null, 2)
        editorError.text = ""
        editor.open()
        editorName.forceActiveFocus()
    }

    function parseObject(text, label) {
        try {
            const value = JSON.parse(text)
            if (!value || Array.isArray(value) || typeof value !== "object")
                throw new Error(label + " must be a JSON object")
            return value
        } catch (error) {
            editorError.text = label + ": " + error
            return null
        }
    }

    function saveEditor() {
        const matchObject = parseObject(matchText.text, "Match")
        if (matchObject === null) return
        const effectsObject = parseObject(effectsText.text, "Effects")
        if (effectsObject === null) return
        const payload = parse(catalog.saveItem(kind, editor.entryId, JSON.stringify({
            name: editorName.text,
            match: matchObject,
            effects: effectsObject
        })))
        if (!payload.ok) {
            editorError.text = payload.error || "Save failed."
            status(editorError.text)
            return
        }
        editor.close()
        load()
        status(payload.message || "Rule saved.")
    }

    function deleteSelected() {
        if (selectedId < 0) return
        const payload = parse(catalog.deleteItem(kind, selectedId))
        if (!payload.ok) { status(payload.error || "Delete failed."); return }
        selectedId = -1
        load()
        status(payload.message || "Rule deleted.")
    }

    Component.onCompleted: load()

    Item {
        id: toolbar
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: root.compactToolbar ? 96 : 52

        GlassField {
            id: searchField
            anchors.left: parent.left
            anchors.right: root.compactToolbar ? parent.right : actions.left
            anchors.rightMargin: root.compactToolbar ? 0 : 12
            anchors.top: parent.top
            anchors.topMargin: 4
            implicitHeight: 43
            darkMode: root.darkMode
            placeholderText: root.layerMode ? "Filter layer rules..." : "Filter window rules..."
            leftPadding: 42
            onTextChanged: root.rebuildModel()
            UiIcon { width: 17; height: 17; name: "search"; anchors.left: parent.left; anchors.leftMargin: 14; anchors.verticalCenter: parent.verticalCenter; iconColor: theme.textSecondary }
        }

        Row {
            id: actions
            anchors.right: parent.right
            anchors.top: root.compactToolbar ? searchField.bottom : parent.top
            anchors.topMargin: root.compactToolbar ? 7 : 4
            spacing: 7
            IconButton { iconName: "plus"; tooltip: "Add rule"; darkMode: root.darkMode; accent: true; onClicked: root.openEditor(null) }
            IconButton { iconName: "edit"; tooltip: "Edit selected rule"; darkMode: root.darkMode; enabled: root.selectedId >= 0; onClicked: root.openEditor(root.selectedItem()) }
            IconButton { iconName: "trash"; tooltip: "Delete selected rule"; darkMode: root.darkMode; danger: true; enabled: root.selectedId >= 0; onClicked: root.deleteSelected() }
        }
    }

    ListView {
        id: list
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: toolbar.bottom
        anchors.bottom: parent.bottom
        anchors.topMargin: 5
        model: model
        clip: true
        spacing: 4
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

        delegate: Rectangle {
            id: row
            required property int entryId
            required property string displayName
            required property string matchLabel
            required property string effectsLabel
            required property string sourceLabel
            width: list.width
            height: root.width < 620 ? 102 : 84
            radius: 11
            color: root.selectedId === entryId ? theme.selectedFill : hover.hovered ? theme.hoverFill : "transparent"
            border.width: root.selectedId === entryId ? 1 : 0
            border.color: theme.selectedRim
            HoverHandler { id: hover }

            Column {
                anchors.left: parent.left
                anchors.leftMargin: 15
                anchors.right: parent.right
                anchors.rightMargin: 15
                anchors.verticalCenter: parent.verticalCenter
                spacing: 4
                Text { width: parent.width; text: displayName; color: theme.textPrimary; font.family: "Inter"; font.pixelSize: 13; font.weight: Font.Medium; elide: Text.ElideRight }
                Text { width: parent.width; text: "match  " + matchLabel; color: theme.alpha(theme.textSecondary, 0.90); font.family: "monospace"; font.pixelSize: 10; elide: Text.ElideRight }
                Text { width: parent.width; text: "effects  " + effectsLabel; color: theme.alpha(theme.textSecondary, 0.76); font.family: "monospace"; font.pixelSize: 10; elide: Text.ElideRight }
                Text { width: parent.width; visible: sourceLabel.length > 0; text: sourceLabel; color: theme.alpha(theme.textSecondary, 0.50); font.family: "Inter"; font.pixelSize: 9; elide: Text.ElideRight }
            }
            TapHandler { onTapped: root.selectedId = entryId; onDoubleTapped: { root.selectedId = entryId; root.openEditor(root.selectedItem()) } }
        }

        Column {
            anchors.centerIn: parent
            spacing: 12
            visible: model.count === 0
            UiIcon { width: 30; height: 30; anchors.horizontalCenter: parent.horizontalCenter; name: root.layerMode ? "layers" : "window"; iconColor: theme.alpha(theme.textSecondary, 0.65) }
            Text { anchors.horizontalCenter: parent.horizontalCenter; text: root.allItems.length ? "No rules match this filter" : "No rules yet"; color: theme.textPrimary; font.family: "Inter"; font.pixelSize: 17; font.weight: Font.Medium }
            GlassButton { visible: root.allItems.length === 0; anchors.horizontalCenter: parent.horizontalCenter; text: "Add rule"; darkMode: root.darkMode; accent: true; onClicked: root.openEditor(null) }
        }
    }

    Popup {
        id: editor
        property int entryId: -1
        width: Math.min(720, root.width - 28)
        height: Math.min(650, root.height - 24)
        x: Math.round((root.width - width) / 2)
        y: Math.round((root.height - height) / 2)
        modal: true
        focus: true
        closePolicy: Popup.CloseOnEscape
        padding: 0
        Overlay.modal: Rectangle { color: theme.alpha(theme.background, root.darkMode ? 0.48 : 0.24) }
        background: Rectangle { radius: 18; color: theme.alpha(theme.familyShell, 0.96); border.width: 1; border.color: theme.shellRim }

        Column {
            anchors.fill: parent
            anchors.margins: root.width < 620 ? 16 : 22
            spacing: 10
            Text { text: editor.entryId >= 0 ? "Edit rule" : "Add rule"; color: theme.textPrimary; font.family: "Inter"; font.pixelSize: 20; font.weight: Font.DemiBold }
            Text { width: parent.width; text: root.layerMode ? "Define namespace matching and layer effects. Values remain typed JSON so booleans and numbers stay intact." : "Define window matching and effects. Values remain typed JSON so booleans and numbers stay intact."; color: theme.textSecondary; font.family: "Inter"; font.pixelSize: 10; wrapMode: Text.WordWrap }
            GlassField { id: editorName; width: parent.width; darkMode: root.darkMode; placeholderText: "Rule name (optional)" }
            Text { text: "MATCH"; color: theme.alpha(theme.textSecondary, 0.70); font.family: "Inter"; font.pixelSize: 9; font.weight: Font.DemiBold; font.letterSpacing: 1 }
            GlassTextArea { id: matchText; width: parent.width; height: Math.max(100, (editor.height - 260) / 2); darkMode: root.darkMode; placeholderText: root.layerMode ? "{\n  \"namespace\": \"waybar\"\n}" : "{\n  \"class\": \"^kitty$\"\n}" }
            Text { text: "EFFECTS"; color: theme.alpha(theme.textSecondary, 0.70); font.family: "Inter"; font.pixelSize: 9; font.weight: Font.DemiBold; font.letterSpacing: 1 }
            GlassTextArea { id: effectsText; width: parent.width; height: Math.max(100, (editor.height - 260) / 2); darkMode: root.darkMode; placeholderText: "{\n  \"float\": true\n}" }
            Text { id: editorError; width: parent.width; visible: text.length > 0; color: theme.danger; font.family: "Inter"; font.pixelSize: 10; wrapMode: Text.WordWrap }
            Row {
                anchors.right: parent.right
                spacing: 8
                GlassButton { text: "Cancel"; darkMode: root.darkMode; onClicked: editor.close() }
                GlassButton { text: "Save"; darkMode: root.darkMode; accent: true; onClicked: root.saveEditor() }
            }
        }
    }
}
