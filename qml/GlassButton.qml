import QtQuick
import QtQuick.Controls

Button {
    id: control
    property bool accent: false
    property bool danger: false
    property bool darkMode: true
    hoverEnabled: true
    HyprbindTheme { id: theme; darkMode: control.darkMode }

    implicitHeight: 38
    leftPadding: 16
    rightPadding: 16
    font.family: "Inter"
    font.pixelSize: 12
    font.weight: accent ? Font.DemiBold : Font.Normal
    scale: down ? 0.96 : 1
    Behavior on scale { NumberAnimation { duration: 105; easing.type: Easing.OutCubic } }

    contentItem: Text {
        text: control.text
        font: control.font
        color: control.danger ? theme.danger : theme.textPrimary
        opacity: control.enabled ? 1 : 0.38
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
    }

    background: Rectangle {
        radius: 12
        antialiasing: true
        color: control.danger
            ? (control.hovered ? theme.dangerFill : theme.alpha(theme.danger, 0.035))
            : control.accent
                ? theme.alpha(theme.accent, control.hovered ? 0.18 : 0.13)
                : control.hovered ? theme.controlHover : theme.controlFill
        border.width: 1
        border.color: control.danger ? theme.dangerRim
            : control.accent ? theme.alpha(theme.accent, 0.24) : theme.controlRim
        Behavior on color { ColorAnimation { duration: 145; easing.type: Easing.OutCubic } }
    }
}
