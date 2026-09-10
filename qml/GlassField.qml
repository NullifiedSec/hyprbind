import QtQuick
import QtQuick.Controls

TextField {
    id: control
    property bool darkMode: true
    HyprbindTheme { id: theme; darkMode: control.darkMode }

    implicitHeight: 43
    leftPadding: 14
    rightPadding: 14
    color: theme.textPrimary
    placeholderTextColor: theme.alpha(theme.textSecondary, 0.72)
    selectionColor: theme.alpha(theme.foreground, control.darkMode ? 0.14 : 0.16)
    selectedTextColor: theme.textPrimary
    font.family: "Inter"
    font.pixelSize: 13
    selectByMouse: true

    background: Rectangle {
        radius: 14
        antialiasing: true
        color: theme.searchFill
        border.width: 1
        border.color: control.activeFocus ? theme.searchFocusRim : theme.searchRim

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.leftMargin: 16
            anchors.rightMargin: 16
            anchors.top: parent.top
            height: 1
            radius: 1
            color: theme.searchSpecular
        }

        Behavior on border.color { ColorAnimation { duration: 135; easing.type: Easing.OutCubic } }
    }
}
