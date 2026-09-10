import QtQuick
import QtQuick.Layouts

Item {
    id: root

    property bool darkMode: true
    property string title: "Setting"
    property string subtitle: ""
    property alias text: field.text
    property string placeholderText: ""
    readonly property bool compactLayout: width < 500

    signal committed(string text)

    HyprbindTheme { id: theme; darkMode: root.darkMode }

    implicitHeight: compactLayout ? 104 : 68

    GridLayout {
        anchors.fill: parent
        columns: root.compactLayout ? 1 : 2
        columnSpacing: 24
        rowSpacing: 8

        Column {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignVCenter
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

        GlassField {
            id: field
            Layout.fillWidth: root.compactLayout
            Layout.preferredWidth: root.compactLayout ? root.width : 250
            Layout.minimumWidth: root.compactLayout ? 0 : 180
            Layout.alignment: Qt.AlignVCenter | Qt.AlignRight
            darkMode: root.darkMode
            placeholderText: root.placeholderText
            onEditingFinished: root.committed(text)
        }
    }
}
