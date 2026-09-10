import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import dev.hyprbinds.ui

ApplicationWindow {
    id: appWindow
    width: 1440
    height: 900
    minimumWidth: 1060
    minimumHeight: 700
    visible: true
    title: "Hyprbind"
    color: "transparent"
    flags: Qt.Window | Qt.FramelessWindowHint

    readonly property color accent: theme.accent
    readonly property color textPrimary: theme.textPrimary
    readonly property color textMuted: theme.textSecondary
    readonly property color hairline: theme.divider

    property string currentPage: "Environment"
    property string configPath: ""
    property var allEnvironment: []
    property string selectedName: ""
    property string statusMessage: "Environment variables are loaded from your config and applied when Hyprland starts."
    property bool backendHealthy: true
    property bool darkMode: true
    property bool realtimeEnabled: false
    property bool backupAvailable: false
    property string backupAge: ""

    HyprbindBridge { id: backend }
    HyprbindTheme { id: theme; darkMode: appWindow.darkMode }
    ListModel { id: environmentModel }

    function parsePayload(raw) {
        try {
            return JSON.parse(raw)
        } catch (error) {
            return { ok: false, error: "Invalid response from Rust bridge: " + error }
        }
    }

    function loadUiState() {
        const payload = parsePayload(backend.uiSnapshot())
        if (!payload.ok)
            return
        darkMode = payload.darkMode !== false
        backupAvailable = payload.hasBackup === true
        backupAge = payload.backupAge || ""
    }

    function toggleDarkMode() {
        const next = !darkMode
        const payload = parsePayload(backend.setDarkMode(next))
        if (!payload.ok) {
            statusMessage = payload.error || "Could not save the theme preference."
            return
        }
        darkMode = next
        statusMessage = payload.message || (next ? "Dark glass theme enabled." : "Light glass theme enabled.")
    }

    function restoreLastBackup() {
        const payload = parsePayload(backend.restoreConfig())
        if (!payload.ok) {
            statusMessage = payload.error || "Restore failed."
            return
        }
        loadEnvironment()
        loadUiState()
        statusMessage = payload.message || "Restored the latest config snapshot."
    }

    function loadEnvironment() {
        const payload = parsePayload(backend.environmentSnapshot())
        configPath = payload.configPath || ""
        allEnvironment = payload.env || []
        backendHealthy = payload.ok === true
        selectedName = ""
        if (!payload.ok)
            statusMessage = payload.error || "Could not load the Hyprland config."
        else if (payload.warning)
            statusMessage = payload.warning
        else
            statusMessage = "Environment variables are loaded from your config and applied when Hyprland starts."
        rebuildEnvironmentModel()
    }

    function rebuildEnvironmentModel() {
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
        const payload = parsePayload(raw)
        if (!payload.ok) {
            editorError.text = payload.error || "The operation failed."
            statusMessage = editorError.text
            return false
        }
        statusMessage = payload.message || "Saved."
        if (closeEditor)
            envEditor.close()
        loadEnvironment()
        statusMessage = payload.message || "Saved."
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
        const payload = parsePayload(backend.deleteEnvironment(deleting))
        if (!payload.ok) {
            statusMessage = payload.error || "Delete failed."
            return
        }
        loadEnvironment()
        statusMessage = payload.message || ("Deleted " + deleting + ".")
    }

    Component.onCompleted: {
        loadUiState()
        loadEnvironment()
    }

    Rectangle {
        id: shell
        anchors.fill: parent
        anchors.margins: 1
        radius: 18
        clip: true
        color: theme.shellFill
        border.width: 1
        border.color: theme.shellRim
        antialiasing: true

        Rectangle {
            anchors.fill: parent
            radius: shell.radius
            antialiasing: true
            color: "transparent"
            gradient: Gradient {
                GradientStop { position: 0.00; color: theme.shellTopSpecular }
                GradientStop { position: 0.20; color: theme.shellAccentWash }
                GradientStop { position: 0.74; color: "transparent" }
                GradientStop { position: 1.00; color: theme.shellBottomShade }
            }
        }

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.leftMargin: 30
            anchors.rightMargin: 30
            anchors.top: parent.top
            anchors.topMargin: 1
            height: 1
            radius: 1
            antialiasing: true
            color: theme.shellInnerLine
        }

        Rectangle {
            id: topBar
            height: 55
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            color: theme.toolbarFill

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: 1
                color: hairline
            }

            MouseArea {
                anchors.fill: parent
                anchors.rightMargin: 360
                onPressed: appWindow.startSystemMove()
                onDoubleClicked: {
                    if (appWindow.visibility === Window.Maximized)
                        appWindow.showNormal()
                    else
                        appWindow.showMaximized()
                }
            }

            Row {
                anchors.left: parent.left
                anchors.leftMargin: 20
                anchors.verticalCenter: parent.verticalCenter
                spacing: 17

                Text {
                    text: "Hyprbind"
                    color: textPrimary
                    font.family: "Inter"
                    font.pixelSize: 18
                    font.weight: Font.DemiBold
                }
                Rectangle {
                    width: 1
                    height: 24
                    color: hairline
                    anchors.verticalCenter: parent.verticalCenter
                }
                Text {
                    text: "Configure. Bind. Customize."
                    color: darkMode ? Qt.rgba(0.76, 0.82, 0.86, 0.76) : Qt.rgba(0.20, 0.32, 0.38, 0.68)
                    font.family: "Inter"
                    font.pixelSize: 12
                    anchors.verticalCenter: parent.verticalCenter
                }
            }

            Row {
                anchors.right: windowControls.left
                anchors.rightMargin: 18
                anchors.verticalCenter: parent.verticalCenter
                spacing: 13

                Button {
                    id: themeButton
                    implicitHeight: 36
                    leftPadding: 10
                    rightPadding: 11
                    hoverEnabled: true
                    flat: true
                    contentItem: Row {
                        spacing: 8
                        UiIcon {
                            width: 18; height: 18
                            name: darkMode ? "moon" : "sun"
                            iconColor: textPrimary
                            anchors.verticalCenter: parent.verticalCenter
                        }
                        Text {
                            text: darkMode ? "Dark mode" : "Light mode"
                            color: textPrimary
                            font.family: "Inter"
                            font.pixelSize: 12
                            anchors.verticalCenter: parent.verticalCenter
                        }
                    }
                    background: Rectangle {
                        radius: 9
                        color: themeButton.hovered
                            ? (darkMode ? Qt.rgba(1,1,1,0.055) : Qt.rgba(1,1,1,0.40))
                            : "transparent"
                        border.width: themeButton.hovered ? 1 : 0
                        border.color: hairline
                    }
                    onClicked: toggleDarkMode()
                    ToolTip.visible: hovered
                    ToolTip.text: "Switch Hyprbind's persisted glass theme"
                }

                Rectangle { width: 1; height: 25; color: hairline; anchors.verticalCenter: parent.verticalCenter }

                Button {
                    id: realtimeButton
                    implicitHeight: 36
                    leftPadding: 10
                    rightPadding: 11
                    hoverEnabled: true
                    flat: true
                    contentItem: Row {
                        spacing: 8
                        UiIcon {
                            width: 17; height: 17
                            name: "realtime"
                            iconColor: realtimeEnabled ? accent : textMuted
                            anchors.verticalCenter: parent.verticalCenter
                        }
                        Text {
                            text: "Realtime"
                            color: realtimeEnabled ? textPrimary : textMuted
                            font.family: "Inter"
                            font.pixelSize: 12
                            anchors.verticalCenter: parent.verticalCenter
                        }
                        Rectangle {
                            width: 6; height: 6; radius: 3
                            color: realtimeEnabled ? accent : Qt.rgba(0.55,0.62,0.66,0.34)
                            anchors.verticalCenter: parent.verticalCenter
                        }
                    }
                    background: Rectangle {
                        radius: 9
                        color: realtimeButton.hovered
                            ? (darkMode ? Qt.rgba(1,1,1,0.055) : Qt.rgba(1,1,1,0.40))
                            : "transparent"
                        border.width: realtimeEnabled ? 1 : 0
                        border.color: Qt.rgba(0.25,0.84,0.94,0.22)
                    }
                    onClicked: {
                        realtimeEnabled = !realtimeEnabled
                        statusMessage = realtimeEnabled
                            ? "Realtime preview is armed for migrated live-edit pages. Environment still writes on Save."
                            : "Realtime preview disabled."
                    }
                    ToolTip.visible: hovered
                    ToolTip.text: "Autosave/live preview is used only where a migrated page supports it"
                }

                Rectangle { width: 1; height: 25; color: hairline; anchors.verticalCenter: parent.verticalCenter }

                IconButton {
                    iconName: "history"
                    darkMode: appWindow.darkMode
                    implicitWidth: 36
                    implicitHeight: 36
                    enabled: backupAvailable
                    onClicked: restoreLastBackup()
                    ToolTip.visible: hovered
                    ToolTip.text: backupAvailable
                        ? (backupAge.length ? "Restore latest snapshot · " + backupAge : "Restore latest snapshot")
                        : "No Hyprbind snapshot is available yet"
                }
                IconButton {
                    iconName: "reload"
                    darkMode: appWindow.darkMode
                    implicitWidth: 36
                    implicitHeight: 36
                    onClicked: {
                        loadEnvironment()
                        loadUiState()
                        statusMessage = "Reloaded config from disk."
                    }
                    ToolTip.visible: hovered
                    ToolTip.text: "Reload config"
                }
            }

            Row {
                id: windowControls
                anchors.right: parent.right
                anchors.rightMargin: 10
                anchors.verticalCenter: parent.verticalCenter
                spacing: 2

                Button {
                    id: minimizeButton
                    implicitWidth: 40
                    implicitHeight: 36
                    flat: true
                    hoverEnabled: true
                    contentItem: UiIcon {
                        width: 16; height: 16
                        anchors.centerIn: parent
                        name: "minimize"
                        iconColor: textMuted
                    }
                    background: Rectangle {
                        radius: 8
                        color: minimizeButton.hovered
                            ? (darkMode ? Qt.rgba(1,1,1,0.055) : Qt.rgba(1,1,1,0.38))
                            : "transparent"
                    }
                    onClicked: appWindow.showMinimized()
                }
                Button {
                    id: maximizeButton
                    implicitWidth: 40
                    implicitHeight: 36
                    flat: true
                    hoverEnabled: true
                    contentItem: UiIcon {
                        width: 15; height: 15
                        anchors.centerIn: parent
                        name: "maximize"
                        iconColor: textMuted
                    }
                    background: Rectangle {
                        radius: 8
                        color: maximizeButton.hovered
                            ? (darkMode ? Qt.rgba(1,1,1,0.055) : Qt.rgba(1,1,1,0.38))
                            : "transparent"
                    }
                    onClicked: appWindow.visibility === Window.Maximized ? appWindow.showNormal() : appWindow.showMaximized()
                }
                Button {
                    id: closeButton
                    implicitWidth: 40
                    implicitHeight: 36
                    flat: true
                    hoverEnabled: true
                    contentItem: UiIcon {
                        width: 16; height: 16
                        anchors.centerIn: parent
                        name: "close"
                        iconColor: closeButton.hovered ? "#ffd9d6" : textMuted
                    }
                    background: Rectangle {
                        radius: 8
                        color: closeButton.hovered ? Qt.rgba(0.73,0.20,0.22,0.44) : "transparent"
                    }
                    onClicked: appWindow.close()
                }
            }
        }

        Rectangle {
            id: sidebar
            width: 286
            anchors.left: parent.left
            anchors.top: topBar.bottom
            anchors.bottom: parent.bottom
            color: theme.sidebarFill

            Rectangle {
                width: 1
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                anchors.right: parent.right
                color: hairline
            }

            GlassField {
                id: navSearch
                anchors.top: parent.top
                anchors.topMargin: 15
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.leftMargin: 16
                anchors.rightMargin: 16
                implicitHeight: 39
                placeholderText: "Search settings..."
                leftPadding: 38
                darkMode: appWindow.darkMode

                UiIcon {
                    width: 17
                    height: 17
                    name: "search"
                    anchors.left: parent.left
                    anchors.leftMargin: 13
                    anchors.verticalCenter: parent.verticalCenter
                    iconColor: textMuted
                }
                Text {
                    text: "Ctrl K"
                    anchors.right: parent.right
                    anchors.rightMargin: 10
                    anchors.verticalCenter: parent.verticalCenter
                    color: darkMode ? Qt.rgba(0.79,0.84,0.87,0.66) : Qt.rgba(0.18,0.31,0.37,0.56)
                    font.family: "Inter"
                    font.pixelSize: 10
                }
            }

            Flickable {
                id: navFlick
                anchors.top: navSearch.bottom
                anchors.topMargin: 10
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                anchors.bottomMargin: 10
                clip: true
                contentHeight: navColumn.implicitHeight
                boundsBehavior: Flickable.StopAtBounds

                ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

                Column {
                    id: navColumn
                    width: navFlick.width
                    spacing: 3

                    Repeater {
                        model: [
                            { title: "START HERE", items: [
                                { name: "Overview", iconName: "home" }
                            ]},
                            { title: "KEYBOARD & CONFIG", items: [
                                { name: "Binds", iconName: "keyboard" },
                                { name: "Variables", iconName: "braces" },
                                { name: "Environment", iconName: "document" },
                                { name: "Submaps", iconName: "grid" },
                                { name: "Startup", iconName: "power" }
                            ]},
                            { title: "RULES", items: [
                                { name: "Window rules", iconName: "window" },
                                { name: "Workspace rules", iconName: "grid" },
                                { name: "Layer rules", iconName: "layers" }
                            ]},
                            { title: "APPEARANCE & INPUT", items: [
                                { name: "Look & Feel", iconName: "diamond" },
                                { name: "Config", iconName: "settings" },
                                { name: "Monitors", iconName: "monitor" },
                                { name: "Devices", iconName: "device" },
                                { name: "Animations", iconName: "sparkle" },
                                { name: "Curves", iconName: "curve" },
                                { name: "Gestures", iconName: "gesture" }
                            ]},
                            { title: "SYSTEM", items: [
                                { name: "Health", iconName: "heart" },
                                { name: "Logs", iconName: "logs" }
                            ]}
                        ]

                        delegate: Column {
                            id: groupColumn
                            width: navColumn.width
                            property var groupData: modelData
                            spacing: 3

                            Item { width: 1; height: index === 0 ? 3 : 9 }
                            Text {
                                text: groupColumn.groupData.title
                                color: darkMode ? Qt.rgba(0.73, 0.79, 0.82, 0.70) : Qt.rgba(0.20, 0.33, 0.39, 0.62)
                                font.family: "Inter"
                                font.pixelSize: 10
                                font.weight: Font.DemiBold
                                font.letterSpacing: 1.25
                                leftPadding: 22
                                height: 22
                                verticalAlignment: Text.AlignVCenter
                            }
                            Repeater {
                                model: groupColumn.groupData.items
                                delegate: NavItem {
                                    property var itemData: modelData
                                    width: groupColumn.width - 28
                                    anchors.horizontalCenter: parent.horizontalCenter
                                    text: itemData.name
                                    iconName: itemData.iconName
                                    darkMode: appWindow.darkMode
                                    selected: appWindow.currentPage === itemData.name
                                    visible: !navSearch.text || itemData.name.toLowerCase().indexOf(navSearch.text.toLowerCase()) >= 0
                                    height: visible ? implicitHeight : 0
                                    onClicked: appWindow.currentPage = itemData.name
                                }
                            }
                        }
                    }
                }
            }
        }

        Item {
            id: mainArea
            anchors.left: sidebar.right
            anchors.right: parent.right
            anchors.top: topBar.bottom
            anchors.bottom: parent.bottom

            Item {
                id: pageHeader
                height: 94
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.leftMargin: 25
                anchors.rightMargin: 25

                GlassPanel {
                    id: pageIcon
                    width: 52
                    height: 52
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    cornerRadius: 12
                    darkMode: appWindow.darkMode
                    elevated: false
                    tint: accent
                    border.color: theme.alpha(accent, 0.14)

                    Rectangle {
                        anchors.fill: parent
                        anchors.margins: 8
                        radius: 10
                        color: theme.alpha(theme.mix(theme.surfaceHigh, accent, 0.08), darkMode ? 0.12 : 0.18)
                    }
                    UiIcon {
                        anchors.centerIn: parent
                        width: 24
                        height: 24
                        name: appWindow.currentPage === "Environment" ? "terminal"
                            : appWindow.currentPage === "Look & Feel" ? "diamond" : "document"
                        iconColor: (appWindow.currentPage === "Environment" || appWindow.currentPage === "Look & Feel") ? accent : textMuted
                        strokeWidth: 1.8
                    }
                }

                Column {
                    anchors.left: pageIcon.right
                    anchors.leftMargin: 18
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 4

                    Text {
                        text: appWindow.currentPage
                        color: textPrimary
                        font.family: "Inter"
                        font.pixelSize: 25
                        font.weight: Font.DemiBold
                    }
                    Text {
                        text: appWindow.currentPage === "Environment"
                            ? "Manage environment variables for Hyprland and apps launched in your session."
                            : appWindow.currentPage === "Look & Feel"
                                ? "Shape Hyprland’s spacing, transparency, effects, and window borders."
                                : "This page remains available in the GTK frontend while the QML migration is in progress."
                        color: textMuted
                        font.family: "Inter"
                        font.pixelSize: 12
                    }
                }

                GlassPanel {
                    id: pathPanel
                    width: 320
                    height: 43
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    darkMode: appWindow.darkMode
                    elevated: false

                    TextInput {
                        id: pathText
                        anchors.left: parent.left
                        anchors.leftMargin: 14
                        anchors.right: copyPath.left
                        anchors.rightMargin: 10
                        anchors.verticalCenter: parent.verticalCenter
                        text: configPath || "No config loaded"
                        readOnly: true
                        selectByMouse: true
                        color: configPath
                            ? (darkMode ? Qt.rgba(0.82, 0.88, 0.91, 0.84) : Qt.rgba(0.16, 0.29, 0.36, 0.82))
                            : textMuted
                        font.family: "Inter"
                        font.pixelSize: 11
                        clip: true
                    }
                    Button {
                        id: copyPath
                        width: 40
                        height: parent.height
                        anchors.right: parent.right
                        flat: true
                        enabled: configPath.length > 0
                        contentItem: UiIcon {
                            width: 17
                            height: 17
                            anchors.centerIn: parent
                            name: "copy"
                            iconColor: copyPath.enabled ? textMuted : Qt.rgba(0.55,0.62,0.66,0.28)
                        }
                        background: Rectangle {
                            color: copyPath.hovered
                                ? (darkMode ? Qt.rgba(1,1,1,0.055) : Qt.rgba(1,1,1,0.34))
                                : "transparent"
                            radius: 9
                        }
                        onClicked: {
                            pathText.selectAll()
                            pathText.copy()
                            pathText.deselect()
                            statusMessage = "Copied config path."
                        }
                        ToolTip.visible: hovered
                        ToolTip.text: "Copy config path"
                    }
                }
            }

            Item {
                id: environmentToolbar
                height: appWindow.currentPage === "Environment" ? 47 : 0
                visible: appWindow.currentPage === "Environment"
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: pageHeader.bottom
                anchors.leftMargin: 25
                anchors.rightMargin: 25

                GlassField {
                    id: envSearch
                    anchors.left: parent.left
                    anchors.right: toolbarButtons.left
                    anchors.rightMargin: 14
                    anchors.verticalCenter: parent.verticalCenter
                    implicitHeight: 43
                    placeholderText: "Filter environment variables..."
                    leftPadding: 42
                    darkMode: appWindow.darkMode
                    onTextChanged: rebuildEnvironmentModel()

                    UiIcon {
                        width: 17
                        height: 17
                        name: "search"
                        anchors.left: parent.left
                        anchors.leftMargin: 14
                        anchors.verticalCenter: parent.verticalCenter
                        iconColor: textMuted
                    }
                }

                Row {
                    id: toolbarButtons
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 9

                    IconButton {
                        iconName: "plus"
                        darkMode: appWindow.darkMode
                        accent: true
                        onClicked: openAddEditor()
                        ToolTip.visible: hovered
                        ToolTip.text: "Add environment variable"
                    }
                    IconButton {
                        iconName: "edit"
                        darkMode: appWindow.darkMode
                        enabled: selectedName.length > 0
                        onClicked: openEditEditor()
                        ToolTip.visible: hovered
                        ToolTip.text: "Edit selected variable"
                    }
                    IconButton {
                        iconName: "trash"
                        darkMode: appWindow.darkMode
                        danger: true
                        enabled: selectedName.length > 0
                        onClicked: deleteSelected()
                        ToolTip.visible: hovered
                        ToolTip.text: "Delete selected variable"
                    }
                }
            }

            Rectangle {
                id: contentHalo
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: appWindow.currentPage === "Environment" ? environmentToolbar.bottom : pageHeader.bottom
                anchors.bottom: infoPanel.top
                anchors.leftMargin: 22
                anchors.rightMargin: 22
                anchors.topMargin: appWindow.currentPage === "Environment" ? 9 : -1
                anchors.bottomMargin: 11
                radius: 17
                color: "transparent"
                border.width: 1
                border.color: Qt.rgba(0.25, 0.84, 0.94, darkMode ? 0.055 : 0.075)
            }

            GlassPanel {
                id: contentPanel
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: appWindow.currentPage === "Environment" ? environmentToolbar.bottom : pageHeader.bottom
                anchors.bottom: infoPanel.top
                anchors.leftMargin: 25
                anchors.rightMargin: 25
                anchors.topMargin: appWindow.currentPage === "Environment" ? 12 : 2
                anchors.bottomMargin: 14
                cornerRadius: 14
                darkMode: appWindow.darkMode
                elevated: false

                Item {
                    anchors.fill: parent
                    visible: appWindow.currentPage === "Environment"

                    ListView {
                        id: environmentList
                        anchors.fill: parent
                        anchors.margins: 12
                        clip: true
                        model: environmentModel
                        spacing: 7
                        visible: environmentModel.count > 0

                        delegate: Rectangle {
                            id: envRow
                            required property string name
                            required property string value
                            required property string sourceFile
                            required property int sourceLine
                            width: environmentList.width
                            height: 69
                            radius: 10
                            color: selectedName === name
                                ? theme.selectedFill
                                : rowMouse.containsMouse ? theme.hoverFill : "transparent"
                            border.width: selectedName === name ? 1 : 0
                            border.color: theme.selectedRim

                            Rectangle {
                                visible: selectedName === name
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
                                anchors.leftMargin: 16
                                anchors.right: parent.right
                                anchors.rightMargin: 16
                                anchors.verticalCenter: parent.verticalCenter
                                spacing: 3
                                Text {
                                    text: name
                                    color: textPrimary
                                    font.family: "Inter"
                                    font.pixelSize: 13
                                    font.weight: Font.Medium
                                }
                                Text {
                                    text: value
                                    color: theme.alpha(theme.textSecondary, 0.88)
                                    font.family: "Inter"
                                    font.pixelSize: 11
                                    elide: Text.ElideRight
                                    width: parent.width
                                }
                                Text {
                                    text: sourceFile ? sourceFile.split("/").pop() + ":" + sourceLine : "managed config"
                                    color: theme.alpha(theme.textSecondary, 0.58)
                                    font.family: "Inter"
                                    font.pixelSize: 9
                                }
                            }

                            MouseArea {
                                id: rowMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                onClicked: selectedName = name
                                onDoubleClicked: {
                                    selectedName = name
                                    openEditEditor()
                                }
                            }
                        }
                    }

                    Column {
                        anchors.centerIn: parent
                        width: Math.min(parent.width - 80, 520)
                        spacing: 14
                        visible: environmentModel.count === 0

                        GlassPanel {
                            width: 76
                            height: 76
                            cornerRadius: 38
                            anchors.horizontalCenter: parent.horizontalCenter
                            darkMode: appWindow.darkMode
                            elevated: true
                            border.color: theme.controlRim

                            Rectangle {
                                anchors.centerIn: parent
                                width: 48
                                height: 48
                                radius: 24
                                color: theme.alpha(theme.accent, darkMode ? 0.035 : 0.065)
                            }
                            UiIcon {
                                anchors.centerIn: parent
                                width: 28
                                height: 28
                                name: "document"
                                iconColor: darkMode ? "#dbe8ee" : "#34505c"
                                strokeWidth: 1.55
                            }
                        }
                        Item { width: 1; height: 2 }
                        Text {
                            text: allEnvironment.length === 0
                                ? "No environment variables yet"
                                : "No matching environment variables"
                            color: textPrimary
                            font.family: "Inter"
                            font.pixelSize: 19
                            font.weight: Font.DemiBold
                            anchors.horizontalCenter: parent.horizontalCenter
                        }
                        Text {
                            width: parent.width
                            text: allEnvironment.length === 0
                                ? "Environment variables let you define key-value pairs for Hyprland and apps launched in your session."
                                : "Try a different filter."
                            color: textMuted
                            font.family: "Inter"
                            font.pixelSize: 12
                            wrapMode: Text.WordWrap
                            horizontalAlignment: Text.AlignHCenter
                        }
                        Item { width: 1; height: 2 }
                        GlassButton {
                            visible: allEnvironment.length === 0
                            text: "Add your first variable"
                            darkMode: appWindow.darkMode
                            accent: true
                            anchors.horizontalCenter: parent.horizontalCenter
                            onClicked: openAddEditor()
                        }
                    }
                }

                Loader {
                    id: lookFeelLoader
                    anchors.fill: parent
                    active: appWindow.currentPage === "Look & Feel"
                    sourceComponent: Component {
                        LookFeelPage {
                            bridge: backend
                            darkMode: appWindow.darkMode
                            realtimeEnabled: appWindow.realtimeEnabled
                            onStatus: message => appWindow.statusMessage = message
                            onSaved: {
                                appWindow.loadUiState()
                                appWindow.loadEnvironment()
                            }
                        }
                    }
                }

                Column {
                    anchors.centerIn: parent
                    spacing: 12
                    visible: appWindow.currentPage !== "Environment" && appWindow.currentPage !== "Look & Feel"
                    Text {
                        text: "QML migration in progress"
                        color: textPrimary
                        font.family: "Inter"
                        font.pixelSize: 19
                        font.weight: Font.DemiBold
                        anchors.horizontalCenter: parent.horizontalCenter
                    }
                    Text {
                        width: 430
                        text: "Use the existing GTK frontend for " + appWindow.currentPage + " while this page is ported."
                        color: textMuted
                        font.family: "Inter"
                        font.pixelSize: 12
                        wrapMode: Text.WordWrap
                        horizontalAlignment: Text.AlignHCenter
                    }
                }
            }

            GlassPanel {
                id: infoPanel
                height: 58
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                anchors.leftMargin: 25
                anchors.rightMargin: 25
                anchors.bottomMargin: 18
                cornerRadius: 11
                darkMode: appWindow.darkMode
                elevated: false

                UiIcon {
                    anchors.left: parent.left
                    anchors.leftMargin: 17
                    anchors.verticalCenter: parent.verticalCenter
                    width: 18
                    height: 18
                    name: "info"
                    iconColor: backendHealthy
                        ? (darkMode ? "#dce6eb" : "#38535f")
                        : (darkMode ? "#e3aaa7" : "#ad4c49")
                }
                Text {
                    anchors.left: parent.left
                    anchors.leftMargin: 46
                    anchors.right: learnMore.left
                    anchors.rightMargin: 16
                    anchors.verticalCenter: parent.verticalCenter
                    text: statusMessage
                    color: theme.alpha(theme.textSecondary, 0.90)
                    font.family: "Inter"
                    font.pixelSize: 11
                    elide: Text.ElideRight
                }
                Text {
                    id: learnMore
                    anchors.right: parent.right
                    anchors.rightMargin: 18
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Learn more  ↗"
                    color: accent
                    opacity: 0.82
                    font.family: "Inter"
                    font.pixelSize: 11
                }
            }
        }
    }

    Popup {
        id: envEditor
        property bool editing: false
        property string originalName: ""
        width: 470
        height: 326
        x: Math.round((appWindow.width - width) / 2)
        y: Math.round((appWindow.height - height) / 2)
        modal: true
        focus: true
        closePolicy: Popup.CloseOnEscape
        padding: 0

        Overlay.modal: Rectangle { color: theme.alpha(theme.background, darkMode ? 0.46 : 0.24) }

        background: Rectangle {
            radius: 18
            color: theme.alpha(theme.familyShell, darkMode ? 0.92 : 0.95)
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
                color: textPrimary
                font.family: "Inter"
                font.pixelSize: 20
                font.weight: Font.DemiBold
            }
            Text {
                text: "Changes are written through Hyprbind’s existing Rust writer and backup path."
                color: textMuted
                font.family: "Inter"
                font.pixelSize: 11
            }
            GlassField {
                id: editorName
                width: parent.width
                darkMode: appWindow.darkMode
                placeholderText: "Name — e.g. XCURSOR_SIZE"
            }
            GlassField {
                id: editorValue
                width: parent.width
                darkMode: appWindow.darkMode
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
                    darkMode: appWindow.darkMode
                    onClicked: envEditor.close()
                }
                GlassButton {
                    id: editorSave
                    text: envEditor.editing ? "Save" : "Add"
                    darkMode: appWindow.darkMode
                    accent: true
                    enabled: editorName.text.trim().length > 0
                    onClicked: {
                        editorError.text = ""
                        if (envEditor.editing) {
                            runAction(
                                backend.editEnvironment(envEditor.originalName, editorName.text, editorValue.text),
                                true
                            )
                        } else {
                            runAction(backend.addEnvironment(editorName.text, editorValue.text), true)
                        }
                    }
                }
            }
        }
    }
}
