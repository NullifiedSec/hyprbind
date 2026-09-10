import QtQuick
import QtQuick.Controls

Item {
    id: root

    property bool darkMode: true
    property var allSubmaps: []
    property int selectedId: -1
    property string configPath: ""
    property bool healthy: true
    readonly property bool compactToolbar: width < 620

    signal status(string message)
    signal configResolved(string path)
    signal healthChanged(bool healthy)

    HyprbindTheme { id: theme; darkMode: root.darkMode }
    SubmapsBridge { id: submapsBackend }
    ListModel { id: submapsModel }

    function parse(raw) {
        try { return JSON.parse(raw) }
        catch (error) { return { ok: false, error: "Invalid Rust bridge response: " + error } }
    }

    function load() {
        const payload = parse(submapsBackend.snapshot())
        configPath = payload.configPath || ""
        configResolved(configPath)
        allSubmaps = payload.submaps || []
        healthy = payload.ok === true
        healthChanged(healthy)
        selectedId = -1
        if (!payload.ok)
            status(payload.error || "Could not load submaps.")
        else if (payload.warning)
            status(payload.warning)
        else
            status("Submaps loaded from config.")
        rebuildModel()
    }

    function rebuildModel() {
        const query = submapSearch.text.trim().toLowerCase()
        submapsModel.clear()
        for (let i = 0; i < allSubmaps.length; ++i) {
            const item = allSubmaps[i]
            const haystack = (item.name + " " + (item.reset || "")).toLowerCase()
            if (!query || haystack.indexOf(query) >= 0)
                submapsModel.append(item)
        }
    }

    function selectedSubmap() {
        for (let i = 0; i < allSubmaps.length; ++i) {
            if (allSubmaps[i].id === selectedId)
                return allSubmaps[i]
        }
        return null
    }

    function openAddEditor() {
        editorName.text = ""
        editorError.text = ""
        addEditor.open()
        editorName.forceActiveFocus()
    }

    function addSubmap() {
        const payload = parse(submapsBackend.addSubmap(editorName.text))
        if (!payload.ok) {
            editorError.text = payload.error || "Could not add submap."
            status(editorError.text)
            return
        }
        addEditor.close()
        load()
        status(payload.message || "Submap added.")
    }

    function deleteSelected() {
        const selected = selectedSubmap()
        if (!selected)
            return
        if (selected.bindCount > 0) {
            status("Move or delete this submap’s binds before deleting the submap.")
            return
        }
        const payload = parse(submapsBackend.deleteSubmap(selectedId))
        if (!payload.ok) {
            status(payload.error || "Delete failed.")
            return
        }
        load()
        status(payload.message || "Submap deleted.")
    }

    Component.onCompleted: load()

    Item {
        id: toolbar
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: root.compactToolbar ? 92 : 52

        GlassField {
            id: submapSearch
            anchors.left: parent.left
            anchors.right: root.compactToolbar ? parent.right : toolbarButtons.left
            anchors.rightMargin: root.compactToolbar ? 0 : 12
            anchors.top: parent.top
            anchors.topMargin: 4
            implicitHeight: 43
            placeholderText: "Filter submaps..."
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
            anchors.top: root.compactToolbar ? submapSearch.bottom : parent.top
            anchors.topMargin: root.compactToolbar ? 6 : 4
            spacing: 7

            IconButton {
                iconName: "plus"
                tooltip: "Add submap"
                darkMode: root.darkMode
                accent: true
                onClicked: root.openAddEditor()
            }

            IconButton {
                iconName: "trash"
                tooltip: {
                    const selected = root.selectedSubmap()
                    if (!selected) return "Select a submap to delete"
                    if (selected.bindCount > 0) return "Submap still owns binds"
                    return "Delete selected submap"
                }
                darkMode: root.darkMode
                danger: true
                enabled: {
                    const selected = root.selectedSubmap()
                    return selected !== null && selected.bindCount === 0
                }
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
            id: submapsList
            anchors.fill: parent
            clip: true
            model: submapsModel
            spacing: 4
            visible: submapsModel.count > 0

            delegate: Rectangle {
                id: submapRow
                required property int id
                required property string name
                required property string reset
                required property int bindCount
                required property string sourceFile
                required property int sourceLine

                width: submapsList.width
                height: 72
                radius: 11
                color: root.selectedId === id
                    ? theme.selectedFill
                    : rowMouse.containsMouse ? theme.hoverFill : "transparent"
                border.width: root.selectedId === id ? 1 : 0
                border.color: theme.selectedRim

                Rectangle {
                    visible: root.selectedId === id
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
                    anchors.right: badges.left
                    anchors.rightMargin: 14
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 4

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
                        text: sourceFile ? sourceFile.split("/").pop() + ":" + sourceLine : "managed submap"
                        color: theme.alpha(theme.textSecondary, 0.56)
                        font.family: "Inter"
                        font.pixelSize: 9
                        elide: Text.ElideRight
                    }
                }

                Row {
                    id: badges
                    anchors.right: parent.right
                    anchors.rightMargin: 15
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 7

                    Rectangle {
                        width: bindText.implicitWidth + 16
                        height: 28
                        radius: 9
                        color: bindCount > 0 ? theme.alpha(theme.accent, 0.06) : theme.controlFill
                        border.width: 1
                        border.color: bindCount > 0 ? theme.alpha(theme.accent, 0.12) : theme.controlRim
                        Text {
                            id: bindText
                            anchors.centerIn: parent
                            text: bindCount === 1 ? "1 bind" : bindCount + " binds"
                            color: theme.alpha(theme.textPrimary, 0.86)
                            font.family: "Inter"
                            font.pixelSize: 10
                        }
                    }

                    Rectangle {
                        visible: reset.length > 0
                        width: resetText.implicitWidth + 16
                        height: 28
                        radius: 9
                        color: theme.controlFill
                        border.width: 1
                        border.color: theme.controlRim
                        Text {
                            id: resetText
                            anchors.centerIn: parent
                            text: "reset " + reset
                            color: theme.alpha(theme.textSecondary, 0.82)
                            font.family: "Inter"
                            font.pixelSize: 10
                        }
                    }
                }

                MouseArea {
                    id: rowMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: root.selectedId = id
                }
            }
        }

        Column {
            anchors.centerIn: parent
            width: Math.min(parent.width - 40, 500)
            spacing: 13
            visible: submapsModel.count === 0

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
                    name: "grid"
                    iconColor: theme.alpha(theme.textPrimary, 0.82)
                    strokeWidth: 1.55
                }
            }

            Text {
                text: root.allSubmaps.length === 0 ? "No submaps yet" : "No matching submaps"
                color: theme.textPrimary
                font.family: "Inter"
                font.pixelSize: 19
                font.weight: Font.DemiBold
                anchors.horizontalCenter: parent.horizontalCenter
            }

            Text {
                width: parent.width
                text: root.allSubmaps.length === 0
                    ? "Submaps group keybinds into named modes that can be entered and reset independently."
                    : "Try a different filter."
                color: theme.textSecondary
                font.family: "Inter"
                font.pixelSize: 12
                wrapMode: Text.WordWrap
                horizontalAlignment: Text.AlignHCenter
            }

            GlassButton {
                visible: root.allSubmaps.length === 0
                text: "Add your first submap"
                darkMode: root.darkMode
                accent: true
                anchors.horizontalCenter: parent.horizontalCenter
                onClicked: root.openAddEditor()
            }
        }
    }

    Popup {
        id: addEditor
        width: Math.min(440, root.width - 32)
        height: 250
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
                text: "Add submap"
                color: theme.textPrimary
                font.family: "Inter"
                font.pixelSize: 20
                font.weight: Font.DemiBold
            }

            Text {
                width: parent.width
                text: "The new submap starts empty. Binds can be added to it from the Binds page later."
                color: theme.textSecondary
                font.family: "Inter"
                font.pixelSize: 11
                wrapMode: Text.WordWrap
            }

            GlassField {
                id: editorName
                width: parent.width
                darkMode: root.darkMode
                placeholderText: "Name — e.g. resize"
                onAccepted: addButton.clicked()
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

            Row {
                anchors.right: parent.right
                spacing: 9
                GlassButton {
                    text: "Cancel"
                    darkMode: root.darkMode
                    onClicked: addEditor.close()
                }
                GlassButton {
                    id: addButton
                    text: "Add"
                    darkMode: root.darkMode
                    accent: true
                    enabled: editorName.text.trim().length > 0
                    onClicked: root.addSubmap()
                }
            }
        }
    }
}
