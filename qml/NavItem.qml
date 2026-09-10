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
        id: content
        clip: true

        Item {
            id: iconSlot
            width: 20
            height: 20
            x: control.compact ? Math.round((content.width - width) / 2) : 0
            y: Math.round((content.height - height) / 2)

            UiIcon {
                anchors.centerIn: parent
                width: 18
                height: 18
                name: control.iconName
                iconColor: control.selected
                    ? theme.textPrimary
                    : theme.alpha(theme.textSecondary, 0.88)
            }
        }

        Text {
            visible: !control.compact
            anchors.left: iconSlot.right
            anchors.leftMargin: 10
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
        color: control.selected ? theme.selectedFill
            : control.hovered ? theme.hoverFill : "transparent"
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
