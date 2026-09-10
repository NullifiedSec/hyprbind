import QtQuick
import QtQuick.Controls

ScrollView {
    id: control

    property bool darkMode: true
    property alias text: editor.text
    property alias placeholderText: editor.placeholderText
    property alias readOnly: editor.readOnly

    clip: true
    padding: 1

    HyprbindTheme { id: theme; darkMode: control.darkMode }

    background: Rectangle {
        radius: 11
        color: editor.activeFocus ? theme.searchFocusedFill : theme.searchFill
        border.width: 1
        border.color: editor.activeFocus ? theme.searchFocusRim : theme.searchRim

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.leftMargin: 12
            anchors.rightMargin: 12
            anchors.top: parent.top
            height: 1
            color: editor.activeFocus ? theme.searchSpecularFocus : theme.searchSpecular
        }
    }

    TextArea {
        id: editor
        color: theme.textPrimary
        placeholderTextColor: theme.alpha(theme.textSecondary, 0.58)
        selectionColor: theme.alpha(theme.accent, 0.28)
        selectedTextColor: theme.textPrimary
        font.family: "monospace"
        font.pixelSize: 11
        wrapMode: TextEdit.NoWrap
        padding: 12
        background: null
    }
}
