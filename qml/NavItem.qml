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
                    ? theme.alpha(theme.accent, 0.92)
                    : control.hovered
                        ? theme.alpha(theme.textPrimary, 0.92)
                        : theme.alpha(theme.textSecondary, 0.86)
                Behavior on iconColor { ColorAnimation { duration: 110 } }
            }
        }

        Text {
            visible: !control.compact
            anchors.left: iconSlot.right
            anchors.leftMargin: 10
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            text: control.text
            color: control.selected
                ? theme.textPrimary
                : control.hovered
                    ? theme.alpha(theme.textPrimary, 0.94)
                    : theme.alpha(theme.textPrimary, 0.84)
            font.family: "Inter"
            font.pixelSize: 13
            font.weight: control.selected ? Font.Medium : Font.Normal
            elide: Text.ElideRight
            Behavior on color { ColorAnimation { duration: 110 } }
        }
    }

    background: Rectangle {
        radius: 12
        antialiasing: true
        color: control.down
            ? (control.selected
                ? theme.alpha(theme.mix(theme.surfaceHigh, theme.accent, 0.12), 0.58)
                : theme.alpha(theme.foreground, root.darkMode ? 0.040 : 0.070))
            : control.selected
                ? theme.selectedFill
                : control.hovered ? theme.hoverFill : "transparent"
        border.width: 0
        Behavior on color { ColorAnimation { duration: 115; easing.type: Easing.OutCubic } }
    }

    ToolTip.visible: control.compact && control.hovered
    ToolTip.delay: 450
    ToolTip.text: control.text
}
