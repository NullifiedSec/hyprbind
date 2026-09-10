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

                readonly property real handleBoxWidth: 28
                readonly property real trackInset: handleBoxWidth / 2

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
                    x: slider.leftPadding + slider.trackInset
                    y: slider.topPadding + slider.availableHeight / 2 - height / 2
                    width: Math.max(0, slider.availableWidth - slider.handleBoxWidth)
                    height: 10

                    Rectangle {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        height: 3
                        radius: 1.5
                        color: theme.alpha(theme.textSecondary, slider.hovered ? 0.14 : 0.09)

                        Rectangle {
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.top: parent.top
                            anchors.leftMargin: 1
                            anchors.rightMargin: 1
                            height: 1
                            radius: 1
                            color: theme.alpha(theme.foreground, slider.hovered ? 0.036 : 0.022)
                        }
                    }

                    Rectangle {
                        anchors.left: parent.left
                        anchors.verticalCenter: parent.verticalCenter
                        width: slider.visualPosition * parent.width
                        height: 3
                        radius: 1.5
                        color: theme.alpha(theme.accent, slider.pressed ? 0.72 : slider.hovered ? 0.62 : 0.54)

                        Rectangle {
                            visible: parent.width > 2
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.top: parent.top
                            anchors.leftMargin: 1
                            anchors.rightMargin: 1
                            height: 1
                            radius: 1
                            color: theme.alpha(theme.foreground, slider.pressed ? 0.08 : 0.045)
                        }

                        Behavior on color { ColorAnimation { duration: 120 } }
                    }
                }

                handle: Item {
                    id: handleBox
                    x: Math.round(slider.leftPadding + slider.visualPosition * (slider.availableWidth - width))
                    y: Math.round(slider.topPadding + slider.availableHeight / 2 - height / 2)
                    width: slider.handleBoxWidth
                    height: 34

                    Rectangle {
                        anchors.centerIn: parent
                        width: 20
                        height: 28
                        radius: 10
                        color: theme.alpha(theme.accent, slider.pressed ? 0.055 : slider.hovered ? 0.028 : 0)
                        opacity: slider.hovered || slider.pressed ? 1 : 0
                        scale: slider.pressed ? 1.04 : 1

                        Behavior on opacity { NumberAnimation { duration: 110 } }
                        Behavior on scale { NumberAnimation { duration: 90; easing.type: Easing.OutCubic } }
                    }

                    Rectangle {
                        id: visibleHandle
                        anchors.centerIn: parent
                        width: 10
                        height: 22
                        radius: 5
                        antialiasing: true
                        scale: slider.pressed ? 1.045 : slider.hovered ? 1.02 : 1
                        color: slider.pressed
                            ? theme.mix(theme.surfaceHigh, theme.foreground, 0.14)
                            : theme.mix(theme.surfaceHigh, theme.foreground, 0.08)
                        border.width: 1
                        border.color: theme.alpha(
                            slider.hovered || slider.pressed ? theme.accent : theme.foreground,
                            slider.pressed ? 0.30 : slider.hovered ? 0.18 : 0.085
                        )

                        Rectangle {
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.top: parent.top
                            anchors.leftMargin: 2
                            anchors.rightMargin: 2
                            anchors.topMargin: 2
                            height: 1
                            radius: 1
                            color: theme.alpha(theme.foreground, slider.pressed ? 0.19 : 0.11)
                        }

                        Rectangle {
                            anchors.centerIn: parent
                            width: 2
                            height: 8
                            radius: 1
                            color: theme.alpha(theme.accent, slider.pressed ? 0.76 : slider.hovered ? 0.60 : 0.44)
                        }

                        Behavior on scale { NumberAnimation { duration: 90; easing.type: Easing.OutCubic } }
                        Behavior on color { ColorAnimation { duration: 110 } }
                        Behavior on border.color { ColorAnimation { duration: 110 } }
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
