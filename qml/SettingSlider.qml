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

            background: Rectangle {
                x: slider.leftPadding
                y: slider.topPadding + slider.availableHeight / 2 - height / 2
                width: slider.availableWidth
                height: 4
                radius: 2
                color: theme.alpha(theme.textSecondary, 0.12)
                Rectangle {
                    width: slider.visualPosition * parent.width
                    height: parent.height
                    radius: 2
                    color: theme.alpha(theme.accent, 0.54)
                }
            }

            handle: Rectangle {
                x: slider.leftPadding + slider.visualPosition * (slider.availableWidth - width)
                y: slider.topPadding + slider.availableHeight / 2 - height / 2
                width: 16
                height: 16
                radius: 8
                color: theme.mix(theme.surfaceHigh, theme.foreground, 0.08)
                border.width: 1
                border.color: theme.alpha(theme.accent, slider.pressed ? 0.48 : 0.28)
                scale: slider.pressed ? 1.10 : 1
                Behavior on scale { NumberAnimation { duration: 105; easing.type: Easing.OutCubic } }
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
