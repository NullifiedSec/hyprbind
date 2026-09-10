import QtQuick
import QtQuick.Controls

Button {
    id: control
    property string iconName: "document"
    property bool selected: false
    property bool darkMode: true
    hoverEnabled: true
    HyprbindTheme { id: theme; darkMode: control.darkMode }

    implicitHeight: 38
    leftPadding: 11
    rightPadding: 11
    scale: down ? 0.985 : 1
    Behavior on scale { NumberAnimation { duration: 90; easing.type: Easing.OutCubic } }

    contentItem: Row {
        spacing: 11
        anchors.verticalCenter: parent.verticalCenter
        UiIcon {
            width: 18
            height: 18
            name: control.iconName
            iconColor: control.selected ? theme.textPrimary : theme.alpha(theme.textSecondary, 0.88)
            anchors.verticalCenter: parent.verticalCenter
        }
        Text {
            text: control.text
            color: control.selected ? theme.textPrimary : theme.alpha(theme.textPrimary, 0.86)
            font.family: "Inter"
            font.pixelSize: 13
            font.weight: control.selected ? Font.Medium : Font.Normal
            anchors.verticalCenter: parent.verticalCenter
        }
    }

    background: Rectangle {
        radius: 12
        antialiasing: true
        color: control.selected ? theme.selectedFill
            : control.hovered ? theme.hoverFill : "transparent"
        border.width: control.selected ? 1 : 0
        border.color: theme.selectedRim
        Behavior on color { ColorAnimation { duration: 145; easing.type: Easing.OutCubic } }

        Rectangle {
            visible: control.selected
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.leftMargin: 15
            anchors.rightMargin: 15
            anchors.top: parent.top
            height: 1
            color: theme.selectedSpecular
        }
    }
}
