import QtQuick

GlassPanel {
    id: root

    property bool healthy: true
    property string message: "Ready."

    HyprbindTheme { id: theme; darkMode: root.darkMode }

    height: 50
    cornerRadius: 11
    elevated: false

    UiIcon {
        anchors.left: parent.left
        anchors.leftMargin: 16
        anchors.verticalCenter: parent.verticalCenter
        width: 18
        height: 18
        name: "info"
        iconColor: root.healthy
            ? theme.alpha(theme.textPrimary, 0.78)
            : (root.darkMode ? "#e3aaa7" : "#ad4c49")
    }

    Text {
        anchors.left: parent.left
        anchors.leftMargin: 45
        anchors.right: parent.right
        anchors.rightMargin: 16
        anchors.verticalCenter: parent.verticalCenter
        text: root.message
        color: theme.alpha(theme.textSecondary, 0.90)
        font.family: "Inter"
        font.pixelSize: 11
        elide: Text.ElideRight
    }
}
