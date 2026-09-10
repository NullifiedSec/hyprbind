import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    property bool darkMode: true
    property string title: "Setting"
    property string subtitle: ""
    property real from: 0
    property real to: 100
    property real stepSize: 1
    property real value: 0
    property int decimals: 0
    property string suffix: ""
    readonly property bool compactLayout: width < 500

    signal edited(real value)

    HyprbindTheme { id: theme; darkMode: root.darkMode }

    implicitHeight: compactLayout ? 96 : 70

    GridLayout {
        anchors.fill: parent
        columns: root.compactLayout ? 1 : 2
        columnSpacing: 24
        rowSpacing: 8

        Column {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignVCenter
            spacing: 3

            Text {
                width: parent.width
                text: root.title
                color: theme.textPrimary
                font.family: "Inter"
                font.pixelSize: 13
                font.weight: Font.Medium
                elide: Text.ElideRight
            }

            Text {
                width: parent.width
                text: root.subtitle
                color: theme.alpha(theme.textSecondary, 0.78)
                font.family: "Inter"
                font.pixelSize: 11
                elide: Text.ElideRight
            }
        }

        RowLayout {
            Layout.fillWidth: root.compactLayout
            Layout.preferredWidth: root.compactLayout ? -1 : 286
            Layout.minimumWidth: root.compactLayout ? 0 : 220
            Layout.alignment: Qt.AlignVCenter | Qt.AlignRight
            spacing: 12

            Slider {
                id: slider

                Layout.fillWidth: true
                Layout.minimumWidth: root.compactLayout ? 130 : 154
                Layout.preferredWidth: 220
                Layout.preferredHeight: 34

                from: root.from
                to: root.to
                stepSize: root.stepSize
                value: root.value
                hoverEnabled: true
                onMoved: root.edited(value)

                background: Item {
                    x: slider.leftPadding
                    y: slider.topPadding + slider.availableHeight / 2 - height / 2
                    width: slider.availableWidth
                    height: 12

                    Rectangle {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        height: 4
                        radius: 2
                        color: theme.alpha(theme.textSecondary, slider.hovered ? 0.14 : 0.095)

                        Rectangle {
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.top: parent.top
                            anchors.leftMargin: 2
                            anchors.rightMargin: 2
                            height: 1
                            radius: 1
                            color: theme.alpha(theme.foreground, slider.hovered ? 0.035 : 0.022)
                        }

                        Behavior on color { ColorAnimation { duration: 140 } }
                    }

                    Rectangle {
                        anchors.left: parent.left
                        anchors.verticalCenter: parent.verticalCenter
                        width: Math.max(3, slider.visualPosition * parent.width)
                        height: 4
                        radius: 2
                        color: theme.alpha(theme.accent, slider.pressed ? 0.72 : slider.hovered ? 0.62 : 0.54)

                        Rectangle {
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.top: parent.top
                            anchors.leftMargin: 2
                            anchors.rightMargin: 2
                            height: 1
                            radius: 1
                            color: theme.alpha(theme.foreground, slider.pressed ? 0.09 : 0.055)
                        }

                        Behavior on color { ColorAnimation { duration: 130 } }
                    }
                }

                handle: Item {
                    x: slider.leftPadding + slider.visualPosition * (slider.availableWidth - width)
                    y: slider.topPadding + slider.availableHeight / 2 - height / 2
                    width: 28
                    height: 34

                    Rectangle {
                        anchors.centerIn: parent
                        width: slider.hovered || slider.pressed ? 22 : 18
                        height: slider.hovered || slider.pressed ? 30 : 26
                        radius: 11
                        color: theme.alpha(theme.accent, slider.pressed ? 0.055 : slider.hovered ? 0.032 : 0)

                        Behavior on width { NumberAnimation { duration: 120; easing.type: Easing.OutCubic } }
                        Behavior on height { NumberAnimation { duration: 120; easing.type: Easing.OutCubic } }
                        Behavior on color { ColorAnimation { duration: 120 } }
                    }

                    Rectangle {
                        anchors.centerIn: parent
                        width: slider.pressed ? 12 : 11
                        height: slider.pressed ? 23 : 21
                        radius: width / 2
                        antialiasing: true
                        color: slider.pressed
                            ? theme.mix(theme.surfaceHigh, theme.foreground, 0.13)
                            : theme.mix(theme.surfaceHigh, theme.foreground, 0.075)
                        border.width: 1
                        border.color: theme.alpha(
                            slider.hovered || slider.pressed ? theme.accent : theme.foreground,
                            slider.pressed ? 0.30 : slider.hovered ? 0.18 : 0.085
                        )

                        Rectangle {
                            anchors.horizontalCenter: parent.horizontalCenter
                            anchors.top: parent.top
                            anchors.topMargin: 2
                            width: Math.max(3, parent.width - 5)
                            height: 1
                            radius: 1
                            color: theme.alpha(theme.foreground, slider.pressed ? 0.18 : 0.11)
                        }

                        Rectangle {
                            anchors.centerIn: parent
                            width: 2
                            height: 9
                            radius: 1
                            color: theme.alpha(theme.accent, slider.pressed ? 0.74 : slider.hovered ? 0.58 : 0.42)
                        }

                        Behavior on width { NumberAnimation { duration: 100; easing.type: Easing.OutCubic } }
                        Behavior on height { NumberAnimation { duration: 100; easing.type: Easing.OutCubic } }
                        Behavior on color { ColorAnimation { duration: 120 } }
                        Behavior on border.color { ColorAnimation { duration: 120 } }
                    }
                }
            }

            Rectangle {
                Layout.preferredWidth: root.decimals > 0 ? 58 : 54
                Layout.preferredHeight: 31
                radius: 10
                color: theme.controlFill
                border.width: 1
                border.color: theme.controlRim

                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.leftMargin: 8
                    anchors.rightMargin: 8
                    height: 1
                    radius: 1
                    color: theme.controlInnerRim
                }

                Text {
                    anchors.centerIn: parent
                    text: Number(root.value).toFixed(root.decimals) + root.suffix
                    color: theme.alpha(theme.textPrimary, 0.90)
                    font.family: "Inter"
                    font.pixelSize: 11
                    font.weight: Font.Medium
                }
            }
        }
    }
}
