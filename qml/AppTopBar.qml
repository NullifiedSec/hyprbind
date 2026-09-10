import QtQuick
import QtQuick.Controls
import QtQuick.Window

Rectangle {
    id: root

    required property var appWindow
    property bool darkMode: true
    property bool realtimeEnabled: false
    property bool backupAvailable: false
    property string backupAge: ""

    signal toggleThemeRequested()
    signal toggleRealtimeRequested()
    signal restoreRequested()
    signal reloadRequested()

    HyprbindTheme { id: theme; darkMode: root.darkMode }

    readonly property bool showTagline: width >= 1180
    readonly property bool showActionLabels: width >= 1010
    readonly property bool compact: width < 900

    height: 55
    color: theme.toolbarFill

    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: 1
        color: theme.divider
    }

    MouseArea {
        anchors.fill: parent
        onPressed: root.appWindow.startSystemMove()
        onDoubleClicked: {
            if (root.appWindow.visibility === Window.Maximized)
                root.appWindow.showNormal()
            else
                root.appWindow.showMaximized()
        }
    }

    Row {
        id: brandRow
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
        anchors.right: windowControls.left
        anchors.rightMargin: root.compact ? 8 : 14
        anchors.verticalCenter: parent.verticalCenter
        spacing: root.compact ? 5 : 9

        Button {
            id: themeButton
            implicitWidth: root.showActionLabels ? 105 : 36
            implicitHeight: 36
            padding: 0
            flat: true
            hoverEnabled: true

            contentItem: Row {
                anchors.centerIn: parent
                spacing: 7
                UiIcon {
                    width: 18
                    height: 18
                    name: root.darkMode ? "moon" : "sun"
                    iconColor: theme.textPrimary
                    anchors.verticalCenter: parent.verticalCenter
                }
                Text {
                    visible: root.showActionLabels
                    text: root.darkMode ? "Dark mode" : "Light mode"
                    color: theme.textPrimary
                    font.family: "Inter"
                    font.pixelSize: 12
                    anchors.verticalCenter: parent.verticalCenter
                }
            }

            background: Rectangle {
                radius: 10
                color: themeButton.hovered ? theme.hoverFill : "transparent"
                border.width: themeButton.hovered ? 1 : 0
                border.color: theme.controlRim
            }
            onClicked: root.toggleThemeRequested()
            ToolTip.visible: hovered
            ToolTip.text: root.darkMode ? "Switch to light theme" : "Switch to dark theme"
        }

        Button {
            id: realtimeButton
            implicitWidth: root.showActionLabels ? 105 : 36
            implicitHeight: 36
            padding: 0
            flat: true
            hoverEnabled: true

            contentItem: Row {
                anchors.centerIn: parent
                spacing: 7
                UiIcon {
                    width: 17
                    height: 17
                    name: "realtime"
                    iconColor: root.realtimeEnabled ? theme.accent : theme.textSecondary
                    anchors.verticalCenter: parent.verticalCenter
                }
                Text {
                    visible: root.showActionLabels
                    text: "Realtime"
                    color: root.realtimeEnabled ? theme.textPrimary : theme.textSecondary
                    font.family: "Inter"
                    font.pixelSize: 12
                    anchors.verticalCenter: parent.verticalCenter
                }
                Rectangle {
                    visible: root.showActionLabels
                    width: 5
                    height: 5
                    radius: 2.5
                    color: root.realtimeEnabled ? theme.accent : theme.alpha(theme.textSecondary, 0.28)
                    anchors.verticalCenter: parent.verticalCenter
                }
            }

            background: Rectangle {
                radius: 10
                color: realtimeButton.hovered ? theme.hoverFill : "transparent"
                border.width: root.realtimeEnabled ? 1 : 0
                border.color: theme.alpha(theme.accent, 0.20)
            }
            onClicked: root.toggleRealtimeRequested()
            ToolTip.visible: hovered
            ToolTip.text: root.realtimeEnabled ? "Disable live preview" : "Enable live preview on supported pages"
        }

        Rectangle {
            width: 1
            height: 23
            color: theme.divider
            anchors.verticalCenter: parent.verticalCenter
        }

        IconButton {
            iconName: "history"
            tooltip: root.backupAvailable
                ? (root.backupAge.length ? "Restore latest snapshot · " + root.backupAge : "Restore latest snapshot")
                : "No snapshot available"
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
                readonly property bool isRestoring: isMaximize && root.appWindow.visibility === Window.Maximized
                readonly property string effectiveIcon: isRestoring ? "restore-window" : modelData.icon
                readonly property string effectiveTip: isRestoring ? "Restore" : modelData.tip

                implicitWidth: root.compact ? 36 : 39
                implicitHeight: 36
                padding: 0
                flat: true
                hoverEnabled: true
                scale: pressed ? 0.94 : 1.0

                Behavior on scale {
                    NumberAnimation { duration: 90; easing.type: Easing.OutCubic }
                }

                contentItem: Item {
                    UiIcon {
                        anchors.centerIn: parent
                        width: 15
                        height: 15
                        name: windowButton.effectiveIcon
                        strokeWidth: 1.35
                        iconColor: windowButton.isClose && windowButton.hovered
                            ? "#ffe4e1"
                            : windowButton.hovered
                                ? theme.alpha(theme.textPrimary, 0.96)
                                : theme.alpha(theme.textSecondary, 0.82)
                    }
                }

                background: Rectangle {
                    radius: 9
                    color: windowButton.isClose && windowButton.hovered
                        ? Qt.rgba(0.72, 0.18, 0.20, windowButton.pressed ? 0.58 : 0.44)
                        : windowButton.pressed
                            ? theme.controlPressed
                            : windowButton.hovered
                                ? theme.hoverFill
                                : "transparent"
                    border.width: windowButton.hovered && !windowButton.isClose ? 1 : 0
                    border.color: theme.controlRim

                    Behavior on color {
                        ColorAnimation { duration: 100 }
                    }
                }

                onClicked: {
                    if (modelData.icon === "minimize") {
                        root.appWindow.showMinimized()
                    } else if (modelData.icon === "maximize") {
                        if (root.appWindow.visibility === Window.Maximized)
                            root.appWindow.showNormal()
                        else
                            root.appWindow.showMaximized()
                    } else {
                        root.appWindow.close()
                    }
                }

                ToolTip.visible: hovered
                ToolTip.delay: 450
                ToolTip.text: effectiveTip
            }
        }
    }
}
