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
    selectionColor: theme.alpha(theme.accent, 0.26)
    selectedTextColor: theme.textPrimary
    font.family: "Inter"
    font.pixelSize: 13
    selectByMouse: true

    background: Rectangle {
        radius: 14
        antialiasing: true
        color: control.activeFocus ? theme.searchFocusedFill : theme.searchFill
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
            color: control.activeFocus ? theme.searchSpecularFocus : theme.searchSpecular
        }

        Behavior on color { ColorAnimation { duration: 185; easing.type: Easing.OutCubic } }
        Behavior on border.color { ColorAnimation { duration: 185; easing.type: Easing.OutCubic } }
    }
}
