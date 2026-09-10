import QtQuick
import QtQuick.Controls

Item {
    id: root

    required property var bridge
    property bool darkMode: true
    property var allEnvironment: []
    property string selectedName: ""
    property string configPath: ""
    property bool healthy: true
    readonly property bool compactToolbar: width < 620

    signal status(string message)
    signal configResolved(string path)
    signal healthChanged(bool healthy)

    HyprbindTheme { id: theme; darkMode: root.darkMode }
    ListModel { id: environmentModel }

    function parse(raw) {
        try { return JSON.parse(raw) }
        catch (error) { return { ok: false, error: "Invalid Rust bridge response: " + error } }
    }

    function load() {
        const payload = parse(bridge.environmentSnapshot())
        configPath = payload.configPath || ""
        configResolved(configPath)
        allEnvironment = payload.env || []
        healthy = payload.ok === true
        healthChanged(healthy)
        selectedName = ""
        if (!payload.ok)
            status(payload.error || "Could not load the Hyprland config.")
        else if (payload.warning)
            status(payload.warning)
        else
            status("Environment values loaded from config.")
        rebuildModel()
    }

    function rebuildModel() {
        const query = envSearch.text.trim().toLowerCase()
        environmentModel.clear()
        for (let i = 0; i < allEnvironment.length; ++i) {
            const item = allEnvironment[i]
            const haystack = (item.name + " " + item.value).toLowerCase()
            if (!query || haystack.indexOf(query) >= 0)
                environmentModel.append(item)
        }
    }

    function selectedEnvironment() {
        for (let i = 0; i < allEnvironment.length; ++i) {
            if (allEnvironment[i].name === selectedName)
                return allEnvironment[i]
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
            envEditor.close()
        load()
        status(payload.message || "Saved.")
        return true
    }

    function openAddEditor() {
        editorTitle.text = "Add environment variable"
        editorName.text = ""
        editorValue.text = ""
        editorError.text = ""
        envEditor.editing = false
        envEditor.originalName = ""
        envEditor.open()
        editorName.forceActiveFocus()
    }

    function openEditEditor() {
        const item = selectedEnvironment()
        if (!item)
            return
        editorTitle.text = "Edit environment variable"
        editorName.text = item.name
        editorValue.text = item.value
        editorError.text = ""
        envEditor.editing = true
        envEditor.originalName = item.name
        envEditor.open()
        editorName.forceActiveFocus()
    }

    function deleteSelected() {
        if (!selectedName)
            return
        const deleting = selectedName
        const payload = parse(bridge.deleteEnvironment(deleting))
        if (!payload.ok) {
            status(payload.error || "Delete failed.")
            return
        }
        load()
        status(payload.message || ("Deleted " + deleting + "."))
    }

    Component.onCompleted: load()

    Item {
        id: toolbar
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: root.compactToolbar ? 92 : 52

        GlassField {
            id: envSearch
            anchors.left: parent.left
            anchors.right: root.compactToolbar ? parent.right : toolbarButtons.left
            anchors.rightMargin: root.compactToolbar ? 0 : 12
            anchors.top: parent.top
            anchors.topMargin: 4
            implicitHeight: 43
            placeholderText: "Filter environment variables..."
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
            anchors.top: root.compactToolbar ? envSearch.bottom : parent.top
            anchors.topMargin: root.compactToolbar ? 6 : 4
            spacing: 7

            IconButton {
                iconName: "plus"
                tooltip: "Add environment variable"
                darkMode: root.darkMode
                accent: true
                onClicked: root.openAddEditor()
            }
            IconButton {
                iconName: "edit"
                tooltip: "Edit selected variable"
                darkMode: root.darkMode
                enabled: root.selectedName.length > 0
                onClicked: root.openEditEditor()
            }
            IconButton {
                iconName: "trash"
                tooltip: "Delete selected variable"
                darkMode: root.darkMode
                danger: true
                enabled: root.selectedName.length > 0
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
            id: environmentList
            anchors.fill: parent
            clip: true
            model: environmentModel
            spacing: 4
            visible: environmentModel.count > 0

            delegate: Rectangle {
                id: envRow
                required property string name
                required property string value
                required property string sourceFile
                required property int sourceLine

                width: environmentList.width
                height: 68
                radius: 11
                color: root.selectedName === name
                    ? theme.selectedFill
                    : rowMouse.containsMouse ? theme.hoverFill : "transparent"
                border.width: root.selectedName === name ? 1 : 0
                border.color: theme.selectedRim

                Rectangle {
                    visible: root.selectedName === name
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
                    anchors.right: parent.right
                    anchors.rightMargin: 15
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 3

                    Text {
                        width: parent.width
                        text: name
                        color: theme.textPrimary
                        font.family: "Inter"
                        font.pixelSize: 13
                        font.weight: Font.Medium
                        elide: Text.ElideRight
                    }
                    Text {
                        width: parent.width
                        text: value
                        color: theme.alpha(theme.textSecondary, 0.88)
                        font.family: "Inter"
                        font.pixelSize: 11
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

                MouseArea {
                    id: rowMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: root.selectedName = name
                    onDoubleClicked: {
                        root.selectedName = name
                        root.openEditEditor()
                    }
                }
            }
        }

        Column {
            anchors.centerIn: parent
            width: Math.min(parent.width - 40, 500)
            spacing: 13
            visible: environmentModel.count === 0

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
                    name: "document"
                    iconColor: theme.alpha(theme.textPrimary, 0.82)
                    strokeWidth: 1.55
                }
            }

            Text {
                text: root.allEnvironment.length === 0
                    ? "No environment variables yet"
                    : "No matching environment variables"
                color: theme.textPrimary
                font.family: "Inter"
                font.pixelSize: 19
                font.weight: Font.DemiBold
                anchors.horizontalCenter: parent.horizontalCenter
            }

            Text {
                width: parent.width
                text: root.allEnvironment.length === 0
                    ? "Environment variables define key-value pairs available to Hyprland and launched apps."
                    : "Try a different filter."
                color: theme.textSecondary
                font.family: "Inter"
                font.pixelSize: 12
                wrapMode: Text.WordWrap
                horizontalAlignment: Text.AlignHCenter
            }

            GlassButton {
                visible: root.allEnvironment.length === 0
                text: "Add your first variable"
                darkMode: root.darkMode
                accent: true
                anchors.horizontalCenter: parent.horizontalCenter
                onClicked: root.openAddEditor()
            }
        }
    }

    Popup {
        id: envEditor
        property bool editing: false
        property string originalName: ""
        width: Math.min(470, root.width - 32)
        height: 326
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
                text: "Add environment variable"
                color: theme.textPrimary
                font.family: "Inter"
                font.pixelSize: 20
                font.weight: Font.DemiBold
            }

            Text {
                width: parent.width
                text: "Changes use Hyprbind’s existing writer and backup path."
                color: theme.textSecondary
                font.family: "Inter"
                font.pixelSize: 11
                elide: Text.ElideRight
            }

            GlassField {
                id: editorName
                width: parent.width
                darkMode: root.darkMode
                placeholderText: "Name — e.g. XCURSOR_SIZE"
            }

            GlassField {
                id: editorValue
                width: parent.width
                darkMode: root.darkMode
                placeholderText: "Value — e.g. 24"
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
                    onClicked: envEditor.close()
                }

                GlassButton {
                    id: editorSave
                    text: envEditor.editing ? "Save" : "Add"
                    darkMode: root.darkMode
                    accent: true
                    enabled: editorName.text.trim().length > 0
                    onClicked: {
                        editorError.text = ""
                        if (envEditor.editing) {
                            root.runAction(
                                root.bridge.editEnvironment(envEditor.originalName, editorName.text, editorValue.text),
                                true
                            )
                        } else {
                            root.runAction(root.bridge.addEnvironment(editorName.text, editorValue.text), true)
                        }
                    }
                }
            }
        }
    }
}
