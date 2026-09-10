import QtQuick

GlassPanel {
    id: root
    property string title: "Section"
    property bool darkMode: true
    default property alias rows: rowColumn.data

    HyprbindTheme { id: theme; darkMode: root.darkMode }
    elevated: false
    cornerRadius: 16
    implicitHeight: contentColumn.implicitHeight + 30

    Column {
        id: contentColumn
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: 15
        spacing: 4

        Text {
            text: root.title
            color: theme.alpha(theme.textSecondary, 0.72)
            font.family: "Inter"
            font.pixelSize: 10
            font.weight: Font.DemiBold
            font.letterSpacing: 1.0
        }

        Column {
            id: rowColumn
            width: parent.width
            spacing: 0
        }
    }
}
