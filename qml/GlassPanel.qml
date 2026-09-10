import QtQuick

Rectangle {
    id: panel
    property bool darkMode: true
    property bool elevated: false
    property color tint: "#37d5e9"
    property real cornerRadius: 13

    HyprbindTheme { id: theme; darkMode: panel.darkMode }

    color: elevated ? theme.raisedFill : theme.contentFill
    radius: cornerRadius
    border.width: 1
    border.color: elevated ? theme.shellRim : theme.quietRim
    antialiasing: true
    clip: true

    Rectangle {
        anchors.fill: parent
        radius: panel.radius
        antialiasing: true
        color: "transparent"
        gradient: Gradient {
            GradientStop { position: 0.00; color: theme.shellTopSpecular }
            GradientStop { position: 0.22; color: theme.shellAccentWash }
            GradientStop { position: 0.74; color: "transparent" }
            GradientStop { position: 1.00; color: theme.shellBottomShade }
        }
    }

    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Math.max(18, panel.cornerRadius * 1.4)
        anchors.rightMargin: Math.max(18, panel.cornerRadius * 1.4)
        anchors.top: parent.top
        anchors.topMargin: 1
        height: 1
        radius: 1
        antialiasing: true
        color: theme.shellInnerLine
    }
}
