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
        spacing: 1

        Repeater {
            model: [
                { icon: "minimize", tip: "Minimize" },
                { icon: "maximize", tip: "Maximize" },
                { icon: "close", tip: "Close" }
            ]
            delegate: Button {
                required property var modelData
                implicitWidth: root.compact ? 36 : 40
                implicitHeight: 36
                padding: 0
                flat: true
                hoverEnabled: true

                contentItem: UiIcon {
                    width: 15
                    height: 15
                    anchors.centerIn: parent
                    name: modelData.icon
                    iconColor: modelData.icon === "close" && parent.hovered
                        ? "#ffd9d6" : theme.textSecondary
                }

                background: Rectangle {
                    radius: 8
                    color: modelData.icon === "close" && parent.hovered
                        ? Qt.rgba(0.73, 0.20, 0.22, 0.44)
                        : parent.hovered ? theme.hoverFill : "transparent"
                }

                onClicked: {
                    if (modelData.icon === "minimize")
                        root.appWindow.showMinimized()
                    else if (modelData.icon === "maximize") {
                        if (root.appWindow.visibility === Window.Maximized)
                            root.appWindow.showNormal()
                        else
                            root.appWindow.showMaximized()
                    } else {
                        root.appWindow.close()
                    }
                }
                ToolTip.visible: hovered
                ToolTip.text: modelData.tip
            }
        }
    }
}
