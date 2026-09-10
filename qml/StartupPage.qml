import QtQuick
import QtQuick.Controls

Item {
    id: root

    property bool darkMode: true
    property var allEntries: []
    property int selectedId: -1
    property string configPath: ""
    property bool healthy: true
    readonly property bool compactToolbar: width < 620

    signal status(string message)
    signal configResolved(string path)
    signal healthChanged(bool healthy)

    HyprbindTheme { id: theme; darkMode: root.darkMode }
    StartupBridge { id: startupBackend }
    ListModel { id: startupModel }

    function parse(raw) {
        try { return JSON.parse(raw) }
        catch (error) { return { ok: false, error: "Invalid Rust bridge response: " + error } }
    }

    function load() {
        const payload = parse(startupBackend.snapshot())
        configPath = payload.configPath || ""
        configResolved(configPath)
        allEntries = payload.entries || []
        healthy = payload.ok === true
        healthChanged(healthy)
        selectedId = -1
        if (!payload.ok)
            status(payload.error || "Could not load startup entries.")
        else if (payload.warning)
            status(payload.warning)
        else
            status("Startup entries loaded from config.")
        rebuildModel()
    }

    function rebuildModel() {
        const query = startupSearch.text.trim().toLowerCase()
        startupModel.clear()
        for (let i = 0; i < allEntries.length; ++i) {
            const item = allEntries[i]
            const haystack = (item.command + " " + item.when + " " + item.workspace).toLowerCase()
            if (!query || haystack.indexOf(query) >= 0)
                startupModel.append(item)
        }
    }

    function selectedEntry() {
        for (let i = 0; i < allEntries.length; ++i) {
            if (allEntries[i].entryId === selectedId)
                return allEntries[i]
        }
        return null
    }

    function runAction(raw, closeEditor) {
        const payload = parse(raw)
        if (!payload.ok) {
            editorError.text = payload.error || "The operation failed."
            status(editorError.text)
            return false
        }
        if (closeEditor)
            startupEditor.close()
        load()
        status(payload.message || "Saved.")
        return true
    }

    function openAddEditor() {
        editorTitle.text = "Add startup entry"
        editorCommand.text = ""
        editorWorkspace.text = ""
        editorError.text = ""
        startupEditor.editing = false
        startupEditor.entryId = -1
        startupEditor.selectedWhen = "start"
        startupEditor.open()
        editorCommand.forceActiveFocus()
    }

    function openEditEditor() {
        const item = selectedEntry()
        if (!item)
            return
        editorTitle.text = "Edit startup entry"
        editorCommand.text = item.command
        editorWorkspace.text = item.workspace || ""
        editorError.text = ""
        startupEditor.editing = true
        startupEditor.entryId = item.entryId
        startupEditor.selectedWhen = item.when || "start"
        startupEditor.open()
        editorCommand.forceActiveFocus()
    }

    function deleteSelected() {
        if (selectedId < 0)
            return
        const payload = parse(startupBackend.deleteEntry(selectedId))
        if (!payload.ok) {
            status(payload.error || "Delete failed.")
            return
        }
        load()
        status(payload.message || "Startup entry deleted.")
    }

    Component.onCompleted: load()

    Item {
        id: toolbar
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: root.compactToolbar ? 92 : 52

        GlassField {
            id: startupSearch
            anchors.left: parent.left
            anchors.right: root.compactToolbar ? parent.right : toolbarButtons.left
            anchors.rightMargin: root.compactToolbar ? 0 : 12
            anchors.top: parent.top
            anchors.topMargin: 4
            implicitHeight: 43
            placeholderText: "Filter startup entries..."
            leftPadding: 42
            darkMode: root.darkMode
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
            id: toolbarButtons
            anchors.right: parent.right
            anchors.top: root.compactToolbar ? startupSearch.bottom : parent.top
            anchors.topMargin: root.compactToolbar ? 6 : 4
            spacing: 7

            IconButton {
                iconName: "plus"
                tooltip: "Add startup entry"
                darkMode: root.darkMode
                accent: true
                onClicked: root.openAddEditor()
            }
            IconButton {
                iconName: "edit"
                tooltip: "Edit selected entry"
                darkMode: root.darkMode
                enabled: root.selectedId >= 0
                onClicked: root.openEditEditor()
            }
            IconButton {
                iconName: "trash"
                tooltip: "Delete selected entry"
                darkMode: root.darkMode
                danger: true
                enabled: root.selectedId >= 0
                onClicked: root.deleteSelected()
            }
        }
    }

    Item {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: toolbar.bottom
        anchors.bottom: parent.bottom
        anchors.topMargin: 6

        ListView {
            id: startupList
            anchors.fill: parent
            clip: true
            model: startupModel
            spacing: 4
            visible: startupModel.count > 0

            delegate: Rectangle {
                id: startupRow
                required property int entryId
                required property string command
                required property string when
                required property string workspace
                required property string sourceFile
                required property int sourceLine

                width: startupList.width
                height: 74
                radius: 11
                color: root.selectedId === entryId
                    ? theme.selectedFill
                    : rowMouse.containsMouse ? theme.hoverFill : "transparent"
                border.width: root.selectedId === entryId ? 1 : 0
                border.color: theme.selectedRim

                Rectangle {
                    visible: root.selectedId === entryId
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.leftMargin: 16
                    anchors.rightMargin: 16
                    height: 1
                    color: theme.selectedSpecular
                }

                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12
                    height: 1
                    color: theme.divider
                }

                Column {
                    anchors.left: parent.left
                    anchors.leftMargin: 15
                    anchors.right: metaRow.left
                    anchors.rightMargin: 15
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 4

                    Text {
                        width: parent.width
                        text: command
                        color: theme.textPrimary
                        font.family: "Inter"
                        font.pixelSize: 13
                        font.weight: Font.Medium
                        elide: Text.ElideRight
                    }

                    Text {
                        width: parent.width
                        text: sourceFile ? sourceFile.split("/").pop() + ":" + sourceLine : "managed config"
                        color: theme.alpha(theme.textSecondary, 0.56)
                        font.family: "Inter"
                        font.pixelSize: 9
                        elide: Text.ElideRight
                    }
                }

                Row {
                    id: metaRow
                    anchors.right: parent.right
                    anchors.rightMargin: 15
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 7

                    Rectangle {
                        width: whenText.implicitWidth + 16
                        height: 28
                        radius: 9
                        color: theme.controlFill
                        border.width: 1
                        border.color: theme.controlRim
                        Text {
                            id: whenText
                            anchors.centerIn: parent
                            text: when === "reload" ? "on reload" : when === "shutdown" ? "on shutdown" : "on start"
                            color: theme.alpha(theme.textPrimary, 0.86)
                            font.family: "Inter"
                            font.pixelSize: 10
                        }
                    }

                    Rectangle {
                        visible: workspace.length > 0
                        width: workspaceText.implicitWidth + 16
                        height: 28
                        radius: 9
                        color: theme.alpha(theme.accent, 0.06)
                        border.width: 1
                        border.color: theme.alpha(theme.accent, 0.12)
                        Text {
                            id: workspaceText
                            anchors.centerIn: parent
                            text: workspace
                            color: theme.alpha(theme.textPrimary, 0.84)
                            font.family: "Inter"
                            font.pixelSize: 10
                        }
                    }
                }

                MouseArea {
                    id: rowMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: root.selectedId = entryId
                    onDoubleClicked: {
                        root.selectedId = entryId
                        root.openEditEditor()
                    }
                }
            }
        }

        Column {
            anchors.centerIn: parent
            width: Math.min(parent.width - 40, 500)
            spacing: 13
            visible: startupModel.count === 0

            GlassPanel {
                width: 72
                height: 72
                cornerRadius: 36
                anchors.horizontalCenter: parent.horizontalCenter
                darkMode: root.darkMode
                elevated: true
                border.color: theme.controlRim

                UiIcon {
                    anchors.centerIn: parent
                    width: 27
                    height: 27
                    name: "power"
                    iconColor: theme.alpha(theme.textPrimary, 0.82)
                    strokeWidth: 1.55
                }
            }

            Text {
                text: root.allEntries.length === 0 ? "No startup entries yet" : "No matching startup entries"
                color: theme.textPrimary
                font.family: "Inter"
                font.pixelSize: 19
                font.weight: Font.DemiBold
                anchors.horizontalCenter: parent.horizontalCenter
            }

            Text {
                width: parent.width
                text: root.allEntries.length === 0
                    ? "Launch apps or commands when Hyprland starts, reloads, or shuts down."
                    : "Try a different filter."
                color: theme.textSecondary
                font.family: "Inter"
                font.pixelSize: 12
                wrapMode: Text.WordWrap
                horizontalAlignment: Text.AlignHCenter
            }

            GlassButton {
                visible: root.allEntries.length === 0
                text: "Add your first entry"
                darkMode: root.darkMode
                accent: true
                anchors.horizontalCenter: parent.horizontalCenter
                onClicked: root.openAddEditor()
            }
        }
    }

    Popup {
        id: startupEditor
        property bool editing: false
        property int entryId: -1
        property string selectedWhen: "start"

        width: Math.min(520, root.width - 32)
        height: 410
        x: Math.round((root.width - width) / 2)
        y: Math.round((root.height - height) / 2)
        modal: true
        focus: true
        closePolicy: Popup.CloseOnEscape
        padding: 0

        Overlay.modal: Rectangle { color: theme.alpha(theme.background, root.darkMode ? 0.46 : 0.24) }

        background: Rectangle {
            radius: 18
            color: theme.alpha(theme.familyShell, root.darkMode ? 0.94 : 0.96)
            border.width: 1
            border.color: theme.shellRim
            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.leftMargin: 24
                anchors.rightMargin: 24
                anchors.top: parent.top
                height: 1
                color: theme.shellInnerLine
            }
        }

        Column {
            anchors.fill: parent
            anchors.margins: 22
            spacing: 14

            Text {
                id: editorTitle
                text: "Add startup entry"
                color: theme.textPrimary
                font.family: "Inter"
                font.pixelSize: 20
                font.weight: Font.DemiBold
            }

            Text {
                width: parent.width
                text: "Choose when the command runs. Workspace is optional."
                color: theme.textSecondary
                font.family: "Inter"
                font.pixelSize: 11
                wrapMode: Text.WordWrap
            }

            GlassField {
                id: editorCommand
                width: parent.width
                darkMode: root.darkMode
                placeholderText: "Command — e.g. waybar"
            }

            Row {
                spacing: 8
                GlassButton {
                    text: "Start"
                    darkMode: root.darkMode
                    accent: startupEditor.selectedWhen === "start"
                    onClicked: startupEditor.selectedWhen = "start"
                }
                GlassButton {
                    text: "Reload"
                    darkMode: root.darkMode
                    accent: startupEditor.selectedWhen === "reload"
                    onClicked: startupEditor.selectedWhen = "reload"
                }
                GlassButton {
                    text: "Shutdown"
                    darkMode: root.darkMode
                    accent: startupEditor.selectedWhen === "shutdown"
                    onClicked: startupEditor.selectedWhen = "shutdown"
                }
            }

            GlassField {
                id: editorWorkspace
                width: parent.width
                darkMode: root.darkMode
                placeholderText: "Workspace (optional)"
                onAccepted: editorSave.clicked()
            }

            Text {
                id: editorError
                width: parent.width
                height: text ? implicitHeight : 0
                visible: text.length > 0
                color: "#e1a19f"
                font.family: "Inter"
                font.pixelSize: 11
                wrapMode: Text.WordWrap
            }

            Item { width: 1; height: 2 }

            Row {
                anchors.right: parent.right
                spacing: 9

                GlassButton {
                    text: "Cancel"
                    darkMode: root.darkMode
                    onClicked: startupEditor.close()
                }

                GlassButton {
                    id: editorSave
                    text: startupEditor.editing ? "Save" : "Add"
                    darkMode: root.darkMode
                    accent: true
                    enabled: editorCommand.text.trim().length > 0
                    onClicked: {
                        editorError.text = ""
                        if (startupEditor.editing) {
                            root.runAction(
                                startupBackend.editEntry(
                                    startupEditor.entryId,
                                    editorCommand.text,
                                    startupEditor.selectedWhen,
                                    editorWorkspace.text
                                ),
                                true
                            )
                        } else {
                            root.runAction(
                                startupBackend.addEntry(
                                    editorCommand.text,
                                    startupEditor.selectedWhen,
                                    editorWorkspace.text
                                ),
                                true
                            )
                        }
                    }
                }
            }
        }
    }
}
