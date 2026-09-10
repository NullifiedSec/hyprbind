import QtQuick
import QtQuick.Controls

Button {
    id: control

    property string iconName: "document"
    property bool selected: false
    property bool darkMode: true
    property bool compact: false

    hoverEnabled: true
    HyprbindTheme { id: theme; darkMode: control.darkMode }

    implicitHeight: 38
    leftPadding: compact ? 0 : 11
    rightPadding: compact ? 0 : 11
    scale: down ? 0.97 : 1
    Behavior on scale { NumberAnimation { duration: 90; easing.type: Easing.OutCubic } }

    contentItem: Item {
        UiIcon {
            width: 18
            height: 18
            name: control.iconName
            iconColor: control.selected ? theme.textPrimary : theme.alpha(theme.textSecondary, 0.88)
            anchors.verticalCenter: parent.verticalCenter
            anchors.horizontalCenter: control.compact ? parent.horizontalCenter : undefined
            anchors.left: control.compact ? undefined : parent.left
        }

        Text {
            visible: !control.compact
            anchors.left: parent.left
            anchors.leftMargin: 29
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            text: control.text
            color: control.selected ? theme.textPrimary : theme.alpha(theme.textPrimary, 0.86)
            font.family: "Inter"
            font.pixelSize: 13
            font.weight: control.selected ? Font.Medium : Font.Normal
            elide: Text.ElideRight
        }
    }

    background: Rectangle {
        radius: 12
        antialiasing: true
        color: control.selected ? theme.selectedFill : control.hovered ? theme.hoverFill : "transparent"
        border.width: control.selected ? 1 : 0
        border.color: theme.selectedRim
        Behavior on color { ColorAnimation { duration: 145; easing.type: Easing.OutCubic } }

        Rectangle {
            visible: control.selected
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.leftMargin: control.compact ? 10 : 15
            anchors.rightMargin: control.compact ? 10 : 15
            anchors.top: parent.top
            height: 1
            color: theme.selectedSpecular
        }
    }

    ToolTip.visible: control.compact && control.hovered
    ToolTip.delay: 450
    ToolTip.text: control.text
}
