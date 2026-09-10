import QtQuick

GlassPanel {
    id: root

    property string iconName: "info"
    property string title: "Summary"
    property string value: "—"
    property string subtitle: ""
    property bool accent: false

    HyprbindTheme { id: theme; darkMode: root.darkMode }

    implicitHeight: 126
    cornerRadius: 15
    elevated: false

    Rectangle {
        width: 40
        height: 40
        radius: 12
        anchors.left: parent.left
        anchors.leftMargin: 16
        anchors.top: parent.top
        anchors.topMargin: 16
        color: root.accent ? theme.alpha(theme.accent, 0.08) : theme.controlFill
        border.width: 1
        border.color: root.accent ? theme.alpha(theme.accent, 0.16) : theme.controlRim

        UiIcon {
            anchors.centerIn: parent
            width: 19
            height: 19
            name: root.iconName
            iconColor: root.accent ? theme.accent : theme.alpha(theme.textPrimary, 0.78)
        }
    }

    Column {
        anchors.left: parent.left
        anchors.leftMargin: 68
        anchors.right: parent.right
        anchors.rightMargin: 16
        anchors.top: parent.top
        anchors.topMargin: 15
        spacing: 2

        Text {
            width: parent.width
            text: root.title
            color: theme.alpha(theme.textSecondary, 0.76)
            font.family: "Inter"
            font.pixelSize: 10
            font.weight: Font.DemiBold
            font.letterSpacing: 0.7
            elide: Text.ElideRight
        }

        Text {
            width: parent.width
            text: root.value
            color: theme.textPrimary
            font.family: "Inter"
            font.pixelSize: 24
            font.weight: Font.DemiBold
            elide: Text.ElideRight
        }
    }

    Text {
        anchors.left: parent.left
        anchors.leftMargin: 16
        anchors.right: parent.right
        anchors.rightMargin: 16
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 15
        text: root.subtitle
        color: theme.alpha(theme.textSecondary, 0.78)
        font.family: "Inter"
        font.pixelSize: 11
        elide: Text.ElideRight
    }
}
