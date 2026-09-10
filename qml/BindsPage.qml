import QtQuick
import QtQuick.Controls
import dev.hyprbinds.ui

Item {
    id: root

    property bool darkMode: true
    property var allItems: []
    property var submaps: []
    property int selectedId: -1
    property string configPath: ""
    readonly property bool compactToolbar: width < 700

    signal status(string message)
    signal configResolved(string path)
    signal healthChanged(bool healthy)

    HyprbindTheme { id: theme; darkMode: root.darkMode }
    CatalogBridge { id: catalog }
    ListModel { id: bindModel }

    function parse(raw) {
        try { return JSON.parse(raw) }
        catch (error) { return { ok: false, error: "Invalid Rust bridge response: " + error } }
    }

    function load() {
        const payload = parse(catalog.snapshot("binds"))
        allItems = payload.items || []
        submaps = payload.submaps || []
        configPath = payload.configPath || ""
        configResolved(configPath)
        healthChanged(payload.ok === true)
        if (!payload.ok)
            status(payload.error || "Could not load binds.")
        else if (payload.warning)
            status(payload.warning)
        else
            status("Binds loaded from config.")
        if (selectedId >= 0 && !selectedItem()) selectedId = -1
        rebuildModel()
    }

    function rebuildModel() {
        const query = searchField.text.trim().toLowerCase()
        bindModel.clear()
        for (let i = 0; i < allItems.length; ++i) {
            const item = allItems[i]
            const haystack = (item.displayName + " " + item.keys + " " + item.action + " " + item.flagsLabel + " " + item.submap + " " + item.sourceLabel).toLowerCase()
            if (!query || haystack.indexOf(query) >= 0)
                bindModel.append(item)
        }
    }

    function selectedItem() {
        for (let i = 0; i < allItems.length; ++i)
            if (allItems[i].entryId === selectedId) return allItems[i]
        return null
    }

    function openAdd() {
        editor.editing = false
        editor.entryId = -1
        editorName.text = ""
        editorKeys.text = ""
        editorAction.text = ""
        editorSubmap.text = ""
        editorMeta.text = "New binds are written to Hyprbind’s managed section."
        editorError.text = ""
        editor.open()
        editorName.forceActiveFocus()
    }

    function openEdit() {
        const item = selectedItem()
        if (!item) return
        editor.editing = true
        editor.entryId = item.entryId
        editorName.text = item.name || item.displayName
        editorKeys.text = item.keys || ""
        editorAction.text = item.action || ""
        editorSubmap.text = item.submap || ""
        editorMeta.text = (item.sharedSource ? "Shared source · edits materialize safely.  " : "") + (item.sourceLabel || "managed config") + (item.flagsLabel ? "  ·  flags: " + item.flagsLabel : "")
        editorError.text = ""
        editor.open()
        editorName.forceActiveFocus()
    }

    function saveEditor() {
        const payload = parse(catalog.saveItem("binds", editor.entryId, JSON.stringify({
            name: editorName.text,
            keys: editorKeys.text,
            action: editorAction.text,
            submap: editor.editing ? editor.originalSubmap : editorSubmap.text
        })))
        if (!payload.ok) {
            editorError.text = payload.error || "Save failed."
            status(editorError.text)
            return
        }
        editor.close()
        load()
        status(payload.message || "Bind saved.")
    }

    function deleteSelected() {
        if (selectedId < 0) return
        const payload = parse(catalog.deleteItem("binds", selectedId))
        if (!payload.ok) {
            status(payload.error || "Delete failed.")
            return
        }
        selectedId = -1
        load()
        status(payload.message || "Bind deleted.")
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
            placeholderText: "Filter binds by key, action, name, submap..."
            leftPadding: 42
            onTextChanged: root.rebuildModel()

            UiIcon {
                width: 17
                height: 17
                name: "search"
                anchors.left: parent.left
                anchors.leftMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                iconColor: theme.textSecondary
            }
        }

        Row {
            id: actions
            anchors.right: parent.right
            anchors.top: root.compactToolbar ? searchField.bottom : parent.top
            anchors.topMargin: root.compactToolbar ? 7 : 4
            spacing: 7

            IconButton { iconName: "plus"; tooltip: "Add bind"; darkMode: root.darkMode; accent: true; onClicked: root.openAdd() }
            IconButton { iconName: "edit"; tooltip: "Edit selected bind"; darkMode: root.darkMode; enabled: root.selectedId >= 0; onClicked: root.openEdit() }
            IconButton { iconName: "trash"; tooltip: "Delete selected bind"; darkMode: root.darkMode; danger: true; enabled: root.selectedId >= 0; onClicked: root.deleteSelected() }
        }
    }

    ListView {
        id: list
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: toolbar.bottom
        anchors.bottom: parent.bottom
        anchors.topMargin: 5
        model: bindModel
        clip: true
        spacing: 4
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

        delegate: Rectangle {
            id: row
            required property int entryId
            required property string displayName
            required property string keys
            required property string action
            required property string flagsLabel
            required property string submap
            required property string sourceLabel
            required property bool sharedSource
            required property bool conflict

            width: list.width
            height: root.width < 640 ? 96 : 78
            radius: 11
            color: root.selectedId === entryId ? theme.selectedFill : hover.hovered ? theme.hoverFill : "transparent"
            border.width: root.selectedId === entryId || conflict ? 1 : 0
            border.color: conflict ? theme.alpha(theme.danger, 0.24) : theme.selectedRim

            HoverHandler { id: hover }

            Column {
                anchors.left: parent.left
                anchors.leftMargin: 14
                anchors.right: badges.left
                anchors.rightMargin: 12
                anchors.verticalCenter: parent.verticalCenter
                spacing: 4

                Row {
                    width: parent.width
                    spacing: 8
                    Text {
                        width: Math.max(50, parent.width - keysBadge.width - 8)
                        text: displayName
                        color: theme.textPrimary
                        font.family: "Inter"
                        font.pixelSize: 13
                        font.weight: Font.Medium
                        elide: Text.ElideRight
                    }
                    Rectangle {
                        id: keysBadge
                        width: Math.min(190, keysText.implicitWidth + 18)
                        height: 25
                        radius: 8
                        color: theme.alpha(theme.accent, 0.06)
                        border.width: 1
                        border.color: theme.alpha(theme.accent, 0.13)
                        Text {
                            id: keysText
                            anchors.centerIn: parent
                            text: keys
                            color: theme.alpha(theme.textPrimary, 0.90)
                            font.family: "monospace"
                            font.pixelSize: 10
                            elide: Text.ElideRight
                        }
                    }
                }

                Text {
                    width: parent.width
                    text: action
                    color: theme.alpha(theme.textSecondary, 0.88)
                    font.family: "monospace"
                    font.pixelSize: 10
                    elide: Text.ElideRight
                }

                Text {
                    width: parent.width
                    text: sourceLabel + (sharedSource ? "  ·  shared source" : "") + (conflict ? "  ·  conflict" : "")
                    color: conflict ? theme.danger : theme.alpha(theme.textSecondary, 0.54)
                    font.family: "Inter"
                    font.pixelSize: 9
                    elide: Text.ElideRight
                }
            }

            Row {
                id: badges
                anchors.right: parent.right
                anchors.rightMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                spacing: 6
                visible: root.width >= 640

                Rectangle {
                    visible: submap.length > 0
                    width: submapText.implicitWidth + 16
                    height: 26
                    radius: 8
                    color: theme.controlFill
                    border.width: 1
                    border.color: theme.controlRim
                    Text { id: submapText; anchors.centerIn: parent; text: submap; color: theme.textSecondary; font.family: "Inter"; font.pixelSize: 9 }
                }
                Rectangle {
                    visible: flagsLabel.length > 0
                    width: flagsText.implicitWidth + 16
                    height: 26
                    radius: 8
                    color: theme.controlFill
                    border.width: 1
                    border.color: theme.controlRim
                    Text { id: flagsText; anchors.centerIn: parent; text: flagsLabel; color: theme.textSecondary; font.family: "Inter"; font.pixelSize: 9 }
                }
            }

            TapHandler {
                onTapped: root.selectedId = entryId
                onDoubleTapped: { root.selectedId = entryId; root.openEdit() }
            }
        }

        Column {
            anchors.centerIn: parent
            width: Math.min(parent.width - 40, 500)
            spacing: 12
            visible: bindModel.count === 0
            UiIcon { width: 30; height: 30; anchors.horizontalCenter: parent.horizontalCenter; name: "keyboard"; iconColor: theme.alpha(theme.textSecondary, 0.65) }
            Text { anchors.horizontalCenter: parent.horizontalCenter; text: root.allItems.length ? "No binds match this filter" : "No binds found"; color: theme.textPrimary; font.family: "Inter"; font.pixelSize: 17; font.weight: Font.Medium }
            GlassButton { visible: root.allItems.length === 0; anchors.horizontalCenter: parent.horizontalCenter; text: "Add your first bind"; darkMode: root.darkMode; accent: true; onClicked: root.openAdd() }
        }
    }

    Popup {
        id: editor
        property bool editing: false
        property int entryId: -1
        property string originalSubmap: editorSubmap.text

        width: Math.min(650, root.width - 28)
        height: Math.min(560, root.height - 24)
        x: Math.round((root.width - width) / 2)
        y: Math.round((root.height - height) / 2)
        modal: true
        focus: true
        closePolicy: Popup.CloseOnEscape
        padding: 0
        onOpened: originalSubmap = editorSubmap.text

        Overlay.modal: Rectangle { color: theme.alpha(theme.background, root.darkMode ? 0.48 : 0.24) }
        background: Rectangle { radius: 18; color: theme.alpha(theme.familyShell, 0.96); border.width: 1; border.color: theme.shellRim }

        Column {
            anchors.fill: parent
            anchors.margins: root.width < 600 ? 16 : 22
            spacing: 11

            Text { text: editor.editing ? "Edit bind" : "Add bind"; color: theme.textPrimary; font.family: "Inter"; font.pixelSize: 20; font.weight: Font.DemiBold }
            Text { id: editorMeta; width: parent.width; color: theme.textSecondary; font.family: "Inter"; font.pixelSize: 10; wrapMode: Text.WordWrap }
            GlassField { id: editorName; width: parent.width; darkMode: root.darkMode; placeholderText: "Name" }
            GlassField { id: editorKeys; width: parent.width; darkMode: root.darkMode; placeholderText: "Keys — e.g. SUPER + RETURN" }
            GlassField { id: editorSubmap; width: parent.width; darkMode: root.darkMode; enabled: !editor.editing; placeholderText: editor.editing ? "Submap cannot be moved from this editor" : "Submap (optional)" }
            GlassTextArea { id: editorAction; width: parent.width; height: Math.max(110, editor.height - 325); darkMode: root.darkMode; placeholderText: "Action — hl.dsp.* call or command" }
            Text { id: editorError; width: parent.width; visible: text.length > 0; color: theme.danger; font.family: "Inter"; font.pixelSize: 10; wrapMode: Text.WordWrap }
            Row {
                anchors.right: parent.right
                spacing: 8
                GlassButton { text: "Cancel"; darkMode: root.darkMode; onClicked: editor.close() }
                GlassButton { text: editor.editing ? "Save" : "Add"; darkMode: root.darkMode; accent: true; enabled: editorName.text.trim().length > 0 && editorKeys.text.trim().length > 0 && editorAction.text.trim().length > 0; onClicked: root.saveEditor() }
            }
        }
    }
}
