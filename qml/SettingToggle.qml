import QtQuick

Item {
    id: root

    property bool darkMode: true
    property string title: "Setting"
    property string subtitle: ""
    property bool checked: false

    signal toggled(bool checked)

    HyprbindTheme { id: theme; darkMode: root.darkMode }

    implicitHeight: 64

    Column {
        anchors.left: parent.left
        anchors.right: switchTrack.left
        anchors.rightMargin: 18
        anchors.verticalCenter: parent.verticalCenter
        spacing: 3

        Text {
            width: parent.width
            text: root.title
            color: theme.textPrimary
            font.family: "Inter"
            font.pixelSize: 13
            font.weight: Font.Medium
            elide: Text.ElideRight
        }

        Text {
            width: parent.width
            text: root.subtitle
            color: theme.alpha(theme.textSecondary, 0.78)
            font.family: "Inter"
            font.pixelSize: 11
            elide: Text.ElideRight
        }
    }

    Rectangle {
        id: switchTrack
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        width: 44
        height: 25
        radius: 13
        color: root.checked ? theme.alpha(theme.accent, 0.22) : theme.controlFill
        border.width: 1
        border.color: root.checked ? theme.alpha(theme.accent, 0.30) : theme.controlRim
        scale: tap.pressed ? 0.97 : 1

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.leftMargin: 7
            anchors.rightMargin: 7
            height: 1
            radius: 1
            color: root.checked
                ? theme.alpha(theme.foreground, 0.08)
                : theme.controlInnerRim
        }

        Rectangle {
            width: 19
            height: 19
            radius: 10
            y: 3
            x: root.checked ? parent.width - width - 3 : 3
            color: root.checked
                ? theme.mix(theme.foreground, theme.accent, 0.07)
                : theme.alpha(theme.foreground, 0.74)
            border.width: 1
            border.color: theme.alpha(theme.foreground, root.checked ? 0.10 : 0.055)

            Behavior on x { NumberAnimation { duration: 165; easing.type: Easing.OutCubic } }
            Behavior on color { ColorAnimation { duration: 150 } }
        }

        HoverHandler { id: hover }
        TapHandler { id: tap; onTapped: root.toggled(!root.checked) }

        Behavior on color { ColorAnimation { duration: 160; easing.type: Easing.OutCubic } }
        Behavior on border.color { ColorAnimation { duration: 150 } }
        Behavior on scale { NumberAnimation { duration: 105; easing.type: Easing.OutCubic } }
    }
}
