import QtQuick
import QtQuick.Controls
import QtQuick.Window
import dev.hyprbinds.ui

Rectangle {
    id: root

    required property var appWindow
    property bool darkMode: true
    property bool realtimeEnabled: false
    property bool backupAvailable: false
    property string backupAge: ""
    property bool compositorMaximized: false

    signal toggleThemeRequested()
    signal toggleRealtimeRequested()
    signal restoreRequested()
    signal reloadRequested()

    HyprbindTheme { id: theme; darkMode: root.darkMode }
    WindowControlBridge { id: windowBridge }

    readonly property bool showTagline: width >= 1180
    readonly property bool showActionLabels: width >= 1010
    readonly property bool compact: width < 900

    height: 55
    color: theme.toolbarFill

    function parse(raw) {
        try { return JSON.parse(raw) }
        catch (error) { return { ok: false, message: String(error) } }
    }

    function toggleMaximize() {
        const result = parse(windowBridge.toggleMaximize())
        if (result.ok) {
            compositorMaximized = !compositorMaximized
            return
        }

        if (appWindow.visibility === Window.Maximized || compositorMaximized) {
            appWindow.showNormal()
            compositorMaximized = false
        } else {
            appWindow.showMaximized()
            compositorMaximized = true
        }
    }

    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: 1
        color: theme.divider
    }

    MouseArea {
        z: 0
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton
        onPressed: root.appWindow.startSystemMove()
        onDoubleClicked: root.toggleMaximize()
    }

    Row {
        id: brandRow
        z: 2
        anchors.left: parent.left
        anchors.leftMargin: root.compact ? 14 : 20
        anchors.right: actionRow.left
        anchors.rightMargin: 18
        anchors.verticalCenter: parent.verticalCenter
        spacing: 16
        clip: true

        Text {
            text: "Hyprbind"
            color: theme.textPrimary
            font.family: "Inter"
            font.pixelSize: root.compact ? 17 : 18
            font.weight: Font.DemiBold
        }

        Rectangle {
            visible: root.showTagline
            width: 1
            height: 23
            color: theme.divider
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            visible: root.showTagline
            text: "Configure. Bind. Customize."
            color: theme.alpha(theme.textSecondary, 0.72)
            font.family: "Inter"
            font.pixelSize: 12
            anchors.verticalCenter: parent.verticalCenter
        }
    }

    Row {
        id: actionRow
        z: 3
        anchors.right: windowControls.left
        anchors.rightMargin: root.compact ? 8 : 14
        anchors.verticalCenter: parent.verticalCenter
        spacing: root.compact ? 5 : 7

        Button {
            id: themeButton
            implicitWidth: root.showActionLabels ? 108 : 36
            implicitHeight: 36
            padding: 0
            flat: true
            hoverEnabled: true
            scale: pressed ? 0.965 : 1
            Behavior on scale { NumberAnimation { duration: 85; easing.type: Easing.OutCubic } }

            contentItem: Row {
                anchors.centerIn: parent
                spacing: 7

                UiIcon {
                    width: 17
                    height: 17
                    name: root.darkMode ? "moon" : "sun"
                    iconColor: themeButton.hovered ? theme.alpha(theme.accent, 0.90) : theme.alpha(theme.textPrimary, 0.88)
                    anchors.verticalCenter: parent.verticalCenter
                    Behavior on iconColor { ColorAnimation { duration: 120 } }
                }

                Text {
                    visible: root.showActionLabels
                    text: root.darkMode ? "Dark mode" : "Light mode"
                    color: themeButton.hovered ? theme.textPrimary : theme.alpha(theme.textPrimary, 0.88)
                    font.family: "Inter"
                    font.pixelSize: 12
                    font.weight: Font.Medium
                    anchors.verticalCenter: parent.verticalCenter
                    Behavior on color { ColorAnimation { duration: 120 } }
                }
            }

            background: Rectangle {
                radius: 10
                color: themeButton.pressed
                    ? theme.controlPressed
                    : themeButton.hovered
                        ? theme.controlHover
                        : theme.alpha(theme.controlFill, 0.17)
                border.width: 1
                border.color: themeButton.hovered
                    ? theme.alpha(theme.foreground, root.darkMode ? 0.085 : 0.12)
                    : theme.alpha(theme.foreground, root.darkMode ? 0.022 : 0.05)
                Behavior on color { ColorAnimation { duration: 120; easing.type: Easing.OutCubic } }
                Behavior on border.color { ColorAnimation { duration: 120 } }

                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.leftMargin: 9
                    anchors.rightMargin: 9
                    height: 1
                    radius: 1
                    color: theme.alpha(theme.foreground, themeButton.hovered ? 0.045 : 0.018)
                    Behavior on color { ColorAnimation { duration: 120 } }
                }
            }

            onClicked: root.toggleThemeRequested()
            ToolTip.visible: hovered && !root.showActionLabels
            ToolTip.delay: 450
            ToolTip.text: root.darkMode ? "Switch to light theme" : "Switch to dark theme"
        }

        Button {
            id: realtimeButton
            implicitWidth: root.showActionLabels ? 108 : 36
            implicitHeight: 36
            padding: 0
            flat: true
            hoverEnabled: true
            scale: pressed ? 0.965 : 1
            Behavior on scale { NumberAnimation { duration: 85; easing.type: Easing.OutCubic } }

            contentItem: Row {
                anchors.centerIn: parent
                spacing: 7

                UiIcon {
                    width: 17
                    height: 17
                    name: "realtime"
                    iconColor: root.realtimeEnabled
                        ? theme.accent
                        : realtimeButton.hovered ? theme.alpha(theme.textPrimary, 0.88) : theme.alpha(theme.textSecondary, 0.82)
                    anchors.verticalCenter: parent.verticalCenter
                    Behavior on iconColor { ColorAnimation { duration: 120 } }
                }

                Text {
                    visible: root.showActionLabels
                    text: "Realtime"
                    color: root.realtimeEnabled || realtimeButton.hovered ? theme.textPrimary : theme.alpha(theme.textSecondary, 0.86)
                    font.family: "Inter"
                    font.pixelSize: 12
                    font.weight: root.realtimeEnabled ? Font.Medium : Font.Normal
                    anchors.verticalCenter: parent.verticalCenter
                    Behavior on color { ColorAnimation { duration: 120 } }
                }

                Rectangle {
                    visible: root.showActionLabels
                    width: 5
                    height: 5
                    radius: 2.5
                    color: root.realtimeEnabled ? theme.accent : theme.alpha(theme.textSecondary, realtimeButton.hovered ? 0.42 : 0.25)
                    anchors.verticalCenter: parent.verticalCenter
                    Behavior on color { ColorAnimation { duration: 120 } }
                }
            }

            background: Rectangle {
                radius: 10
                color: realtimeButton.pressed
                    ? theme.controlPressed
                    : root.realtimeEnabled
                        ? theme.alpha(theme.mix(theme.surfaceHigh, theme.accent, 0.12), realtimeButton.hovered ? 0.34 : 0.24)
                        : realtimeButton.hovered ? theme.controlHover : theme.alpha(theme.controlFill, 0.13)
                border.width: 1
                border.color: root.realtimeEnabled
                    ? theme.alpha(theme.accent, realtimeButton.hovered ? 0.22 : 0.14)
                    : realtimeButton.hovered
                        ? theme.alpha(theme.foreground, root.darkMode ? 0.075 : 0.11)
                        : theme.alpha(theme.foreground, root.darkMode ? 0.018 : 0.045)
                Behavior on color { ColorAnimation { duration: 120; easing.type: Easing.OutCubic } }
                Behavior on border.color { ColorAnimation { duration: 120 } }

                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.leftMargin: 9
                    anchors.rightMargin: 9
                    height: 1
                    radius: 1
                    color: root.realtimeEnabled
                        ? theme.alpha(theme.mix(theme.foreground, theme.accent, 0.18), realtimeButton.hovered ? 0.060 : 0.035)
                        : theme.alpha(theme.foreground, realtimeButton.hovered ? 0.040 : 0.014)
                    Behavior on color { ColorAnimation { duration: 120 } }
                }
            }

            onClicked: root.toggleRealtimeRequested()
            ToolTip.visible: hovered && !root.showActionLabels
            ToolTip.delay: 450
            ToolTip.text: root.realtimeEnabled ? "Disable live preview" : "Enable live preview on supported pages"
        }

        Rectangle { width: 1; height: 23; color: theme.divider; anchors.verticalCenter: parent.verticalCenter }

        IconButton {
            iconName: "history"
            tooltip: root.backupAvailable ? (root.backupAge.length ? "Restore latest snapshot · " + root.backupAge : "Restore latest snapshot") : "No snapshot available"
            darkMode: root.darkMode
            implicitWidth: 36
            implicitHeight: 36
            enabled: root.backupAvailable
            onClicked: root.restoreRequested()
        }

        IconButton {
            iconName: "reload"
            tooltip: "Reload current page"
            darkMode: root.darkMode
            implicitWidth: 36
            implicitHeight: 36
            onClicked: root.reloadRequested()
        }
    }

    Row {
        id: windowControls
        z: 4
        anchors.right: parent.right
        anchors.rightMargin: 8
        anchors.verticalCenter: parent.verticalCenter
        spacing: 2

        Repeater {
            model: [
                { icon: "minimize", tip: "Minimize" },
                { icon: "maximize", tip: "Maximize" },
                { icon: "close", tip: "Close" }
            ]

            delegate: Button {
                id: windowButton
                required property var modelData
                readonly property bool isClose: modelData.icon === "close"
                readonly property bool isMaximize: modelData.icon === "maximize"
                readonly property bool isRestoring: isMaximize && (root.compositorMaximized || root.appWindow.visibility === Window.Maximized)
                readonly property string effectiveIcon: isRestoring ? "restore-window" : modelData.icon
                readonly property string effectiveTip: isRestoring ? "Restore" : modelData.tip

                implicitWidth: root.compact ? 36 : 39
                implicitHeight: 36
                padding: 0
                flat: true
                hoverEnabled: true
                scale: pressed ? 0.94 : 1.0
                Behavior on scale { NumberAnimation { duration: 90; easing.type: Easing.OutCubic } }

                contentItem: Item {
                    UiIcon {
                        anchors.centerIn: parent
                        width: 15
                        height: 15
                        name: windowButton.effectiveIcon
                        strokeWidth: 1.30
                        iconColor: windowButton.isClose && windowButton.hovered ? "#ffe4e1" : windowButton.hovered ? theme.alpha(theme.textPrimary, 0.96) : theme.alpha(theme.textSecondary, 0.82)
                    }
                }

                background: Rectangle {
                    radius: 9
                    color: windowButton.isClose && windowButton.hovered
                        ? Qt.rgba(0.72, 0.18, 0.20, windowButton.pressed ? 0.58 : 0.44)
                        : windowButton.pressed ? theme.controlPressed : windowButton.hovered ? theme.hoverFill : "transparent"
                    border.width: windowButton.hovered && !windowButton.isClose ? 1 : 0
                    border.color: theme.controlRim
                    Behavior on color { ColorAnimation { duration: 100 } }
                }

                onClicked: {
                    if (modelData.icon === "minimize") root.appWindow.showMinimized()
                    else if (modelData.icon === "maximize") root.toggleMaximize()
                    else root.appWindow.close()
                }

                ToolTip.visible: hovered
                ToolTip.delay: 450
                ToolTip.text: effectiveTip
            }
        }
    }
}
