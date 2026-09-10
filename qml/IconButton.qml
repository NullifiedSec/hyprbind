import QtQuick
import QtQuick.Controls

Button {
    id: control
    property string glyph: ""
    property string iconName: ""
    property bool accent: false
    property bool danger: false
    property bool darkMode: true
    hoverEnabled: true
    HyprbindTheme { id: theme; darkMode: control.darkMode }

    implicitWidth: 40
    implicitHeight: 40
    padding: 0
    scale: down ? 0.94 : 1
    opacity: enabled ? 1 : 0.30

    Behavior on scale { NumberAnimation { duration: 105; easing.type: Easing.OutCubic } }

    contentItem: Item {
        UiIcon {
            visible: control.iconName.length > 0
            anchors.centerIn: parent
            width: 19
            height: 19
            name: control.iconName
            iconColor: control.danger ? theme.danger
                : control.accent ? theme.textPrimary : theme.alpha(theme.textPrimary, 0.90)
        }
        Text {
            visible: control.iconName.length === 0
            anchors.fill: parent
            text: control.glyph
            font.family: "Inter"
            font.pixelSize: 18
            color: control.danger ? theme.danger : theme.textPrimary
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
    }

    background: Rectangle {
        radius: 13
        antialiasing: true
        color: control.danger
            ? (control.hovered ? theme.dangerFill : "transparent")
            : control.accent
                ? theme.alpha(theme.accent, control.hovered ? 0.15 : 0.10)
                : control.hovered ? theme.hoverFill : "transparent"
        border.width: control.accent || (control.danger && control.hovered) ? 1 : 0
        border.color: control.danger ? theme.dangerRim : theme.alpha(theme.accent, 0.20)
        Behavior on color { ColorAnimation { duration: 145; easing.type: Easing.OutCubic } }
        Behavior on border.color { ColorAnimation { duration: 145 } }
    }
}
