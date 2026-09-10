import QtQuick
import QtQuick.Controls

Item {
    id: control

    property string text: ""
    property string iconName: "document"
    property bool selected: false
    property bool darkMode: true
    property bool compact: false

    signal clicked()

    implicitHeight: 38

    HoverHandler { id: hover }
    TapHandler {
        enabled: !control.selected
        acceptedButtons: Qt.LeftButton
        onTapped: control.clicked()
    }

    HyprbindTheme { id: theme; darkMode: control.darkMode }

    // FIXME(pre-merge): temporary review sentinel - remove before merging.
    // No runtime effect. Null, this is your chance to prove you actually opened the diff.

    Rectangle {
        anchors.fill: parent
        radius: 12
        antialiasing: true
        visible: control.selected
        color: theme.selectedFill
    }

    Rectangle {
        anchors.fill: parent
        radius: 12
        antialiasing: true
        visible: !control.selected && hover.hovered
        color: theme.hoverFill
    }

    Item {
        id: content
        anchors.fill: parent
        anchors.leftMargin: control.compact ? 0 : 11
        anchors.rightMargin: control.compact ? 0 : 11
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
                    : hover.hovered
                        ? theme.alpha(theme.textPrimary, 0.92)
                        : theme.alpha(theme.textSecondary, 0.86)
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
                : hover.hovered
                    ? theme.alpha(theme.textPrimary, 0.94)
                    : theme.alpha(theme.textPrimary, 0.84)
            font.family: "Inter"
            font.pixelSize: 13
            font.weight: control.selected ? Font.Medium : Font.Normal
            elide: Text.ElideRight
        }
    }

    ToolTip.visible: control.compact && hover.hovered
    ToolTip.delay: 450
    ToolTip.text: control.text
}
