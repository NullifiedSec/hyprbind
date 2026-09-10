import QtQuick
import QtQuick.Controls

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
    signal edited(real value)

    HyprbindTheme { id: theme; darkMode: root.darkMode }
    implicitHeight: 70

    Column {
        anchors.left: parent.left
        anchors.right: sliderWrap.left
        anchors.rightMargin: 24
        anchors.verticalCenter: parent.verticalCenter
        spacing: 3

        Text {
            text: root.title
            color: theme.textPrimary
            font.family: "Inter"
            font.pixelSize: 13
            font.weight: Font.Medium
        }
        Text {
            text: root.subtitle
            color: theme.alpha(theme.textSecondary, 0.78)
            font.family: "Inter"
            font.pixelSize: 11
        }
    }

    Row {
        id: sliderWrap
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        spacing: 12

        Slider {
            id: slider
            width: 220
            height: 32
            from: root.from
            to: root.to
            stepSize: root.stepSize
            value: root.value
            onMoved: root.edited(value)

            background: Item {
                x: slider.leftPadding
                y: slider.topPadding + slider.availableHeight / 2 - height / 2
                width: slider.availableWidth
                height: 10

                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    height: 3
                    radius: 1.5
                    color: theme.alpha(theme.textSecondary, slider.hovered ? 0.15 : 0.10)
                    Behavior on color { ColorAnimation { duration: 130 } }
                }

                Rectangle {
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    width: slider.visualPosition * parent.width
                    height: 3
                    radius: 1.5
                    color: theme.alpha(theme.accent, slider.pressed ? 0.72 : 0.56)
                    Behavior on color { ColorAnimation { duration: 120 } }
                }
            }

            handle: Item {
                x: slider.leftPadding + slider.visualPosition * (slider.availableWidth - width)
                y: slider.topPadding + slider.availableHeight / 2 - height / 2
                width: 10
                height: 22

                Rectangle {
                    anchors.centerIn: parent
                    width: slider.pressed ? 8 : 7
                    height: slider.pressed ? 20 : 18
                    radius: width / 2
                    color: slider.pressed
                        ? theme.mix(theme.surfaceHigh, theme.foreground, 0.16)
                        : theme.mix(theme.surfaceHigh, theme.foreground, 0.08)
                    border.width: 1
                    border.color: theme.alpha(theme.accent, slider.hovered || slider.pressed ? 0.32 : 0.16)
                    antialiasing: true

                    Rectangle {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.leftMargin: 2
                        anchors.rightMargin: 2
                        anchors.topMargin: 2
                        height: 1
                        radius: 1
                        color: theme.alpha(theme.foreground, slider.pressed ? 0.15 : 0.08)
                    }

                    Behavior on width { NumberAnimation { duration: 100; easing.type: Easing.OutCubic } }
                    Behavior on height { NumberAnimation { duration: 100; easing.type: Easing.OutCubic } }
                    Behavior on color { ColorAnimation { duration: 120 } }
                    Behavior on border.color { ColorAnimation { duration: 120 } }
                }
            }
        }

        Rectangle {
            width: 54
            height: 31
            radius: 10
            color: theme.controlFill
            border.width: 1
            border.color: theme.controlRim
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
