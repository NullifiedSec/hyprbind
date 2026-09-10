import QtQuick

Item {
    id: root
    property bool darkMode: true
    property string title: "Setting"
    property string subtitle: ""
    property alias text: field.text
    property string placeholderText: ""
    signal committed(string text)

    HyprbindTheme { id: theme; darkMode: root.darkMode }
    implicitHeight: 68

    Column {
        anchors.left: parent.left
        anchors.right: field.left
        anchors.rightMargin: 24
        anchors.verticalCenter: parent.verticalCenter
        spacing: 3
        Text {
            text: root.title
            color: theme.textPrimary
            font.family: "Inter"
            font.pixelSize: 13
            font.weight: Font.Medium
        }
        Text {
            text: root.subtitle
            color: theme.alpha(theme.textSecondary, 0.78)
            font.family: "Inter"
            font.pixelSize: 11
        }
    }

    GlassField {
        id: field
        width: 250
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        darkMode: root.darkMode
        placeholderText: root.placeholderText
        onEditingFinished: root.committed(text)
    }
}
