import QtQuick
import QtQuick.Controls

Item {
    id: root

    required property var bridge
    property bool darkMode: true
    property var allVariables: []
    property string selectedName: ""
    property string configPath: ""
    property bool healthy: true
    readonly property bool compactToolbar: width < 620

    signal status(string message)
    signal configResolved(string path)
    signal healthChanged(bool healthy)

    HyprbindTheme { id: theme; darkMode: root.darkMode }
    ListModel { id: variablesModel }

    function parse(raw) {
        try { return JSON.parse(raw) }
        catch (error) { return { ok: false, error: "Invalid Rust bridge response: " + error } }
    }

    function load() {
        const payload = parse(bridge.variablesSnapshot())
        configPath = payload.configPath || ""
        configResolved(configPath)
        allVariables = payload.variables || []
        healthy = payload.ok === true
        healthChanged(healthy)
        selectedName = ""
        if (!payload.ok)
            status(payload.error || "Could not load variables.")
        else if (payload.warning)
            status(payload.warning)
        else
            status("Variables loaded from config.")
        rebuildModel()
    }

    function rebuildModel() {
        const query = variableSearch.text.trim().toLowerCase()
        variablesModel.clear()
        for (let i = 0; i < allVariables.length; ++i) {
            const item = allVariables[i]
            const haystack = (item.name + " " + item.value).toLowerCase()
            if (!query || haystack.indexOf(query) >= 0)
                variablesModel.append(item)
        }
    }

    function selectedVariable() {
        for (let i = 0; i < allVariables.length; ++i) {
            if (allVariables[i].name === selectedName)
                return allVariables[i]
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
            variableEditor.close()
        load()
        status(payload.message || "Saved.")
        return true
    }

    function openAddEditor() {
        editorTitle.text = "Add variable"
        editorName.text = ""
        editorValue.text = ""
        editorError.text = ""
        variableEditor.editing = false
        variableEditor.originalName = ""
        variableEditor.open()
        editorName.forceActiveFocus()
    }

    function openEditEditor() {
        const item = selectedVariable()
        if (!item)
            return
        editorTitle.text = "Edit variable"
        editorName.text = item.name
        editorValue.text = item.value
        editorError.text = ""
        variableEditor.editing = true
        variableEditor.originalName = item.name
        variableEditor.open()
        editorName.forceActiveFocus()
    }

    function deleteSelected() {
        if (!selectedName)
            return
        const deleting = selectedName
        const payload = parse(bridge.deleteVariable(deleting))
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
            id: variableSearch
            anchors.left: parent.left
            anchors.right: root.compactToolbar ? parent.right : toolbarButtons.left
            anchors.rightMargin: root.compactToolbar ? 0 : 12
            anchors.top: parent.top
            anchors.topMargin: 4
            implicitHeight: 43
            placeholderText: "Filter variables..."
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
            anchors.top: root.compactToolbar ? variableSearch.bottom : parent.top
            anchors.topMargin: root.compactToolbar ? 6 : 4
            spacing: 7

            IconButton {
                iconName: "plus"
                tooltip: "Add variable"
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
            id: variablesList
            anchors.fill: parent
            clip: true
            model: variablesModel
            spacing: 4
            visible: variablesModel.count > 0

            delegate: Rectangle {
                id: variableRow
                required property string name
                required property string value
                required property string sourceFile
                required property int sourceLine

                width: variablesList.width
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

                Row {
                    anchors.left: parent.left
                    anchors.leftMargin: 15
                    anchors.right: parent.right
                    anchors.rightMargin: 15
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 14

                    Column {
                        width: Math.max(120, parent.width * 0.38)
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
                            text: sourceFile ? sourceFile.split("/").pop() + ":" + sourceLine : "managed config"
                            color: theme.alpha(theme.textSecondary, 0.56)
                            font.family: "Inter"
                            font.pixelSize: 9
                            elide: Text.ElideRight
                        }
                    }

                    Text {
                        width: Math.max(80, parent.width - x)
                        anchors.verticalCenter: parent.verticalCenter
                        text: value
                        color: theme.alpha(theme.textSecondary, 0.88)
                        font.family: "Inter"
                        font.pixelSize: 11
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
            visible: variablesModel.count === 0

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
                    name: "braces"
                    iconColor: theme.alpha(theme.textPrimary, 0.82)
                    strokeWidth: 1.55
                }
            }

            Text {
                text: root.allVariables.length === 0 ? "No variables yet" : "No matching variables"
                color: theme.textPrimary
                font.family: "Inter"
                font.pixelSize: 19
                font.weight: Font.DemiBold
                anchors.horizontalCenter: parent.horizontalCenter
            }

            Text {
                width: parent.width
                text: root.allVariables.length === 0
                    ? "Variables let you reuse values across binds, commands, and rules without duplicating them."
                    : "Try a different filter."
                color: theme.textSecondary
                font.family: "Inter"
                font.pixelSize: 12
                wrapMode: Text.WordWrap
                horizontalAlignment: Text.AlignHCenter
            }

            GlassButton {
                visible: root.allVariables.length === 0
                text: "Add your first variable"
                darkMode: root.darkMode
                accent: true
                anchors.horizontalCenter: parent.horizontalCenter
                onClicked: root.openAddEditor()
            }
        }
    }

    Popup {
        id: variableEditor
        property bool editing: false
        property string originalName: ""
        width: Math.min(500, root.width - 32)
        height: 344
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
                text: "Add variable"
                color: theme.textPrimary
                font.family: "Inter"
                font.pixelSize: 20
                font.weight: Font.DemiBold
            }

            Text {
                width: parent.width
                text: variableEditor.editing
                    ? "Renaming a variable updates its identifier references across related config files."
                    : "Variables are written as local Lua declarations through Hyprbind’s existing writer."
                color: theme.textSecondary
                font.family: "Inter"
                font.pixelSize: 11
                wrapMode: Text.WordWrap
            }

            GlassField {
                id: editorName
                width: parent.width
                darkMode: root.darkMode
                placeholderText: "Name — e.g. mainMod"
            }

            GlassField {
                id: editorValue
                width: parent.width
                darkMode: root.darkMode
                placeholderText: "Value — e.g. SUPER"
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
                    onClicked: variableEditor.close()
                }

                GlassButton {
                    id: editorSave
                    text: variableEditor.editing ? "Save" : "Add"
                    darkMode: root.darkMode
                    accent: true
                    enabled: editorName.text.trim().length > 0
                    onClicked: {
                        editorError.text = ""
                        if (variableEditor.editing) {
                            root.runAction(
                                root.bridge.editVariable(variableEditor.originalName, editorName.text, editorValue.text),
                                true
                            )
                        } else {
                            root.runAction(root.bridge.addVariable(editorName.text, editorValue.text), true)
                        }
                    }
                }
            }
        }
    }
}
