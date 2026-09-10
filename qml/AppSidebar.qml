import QtQuick
import QtQuick.Controls

Rectangle {
    id: root

    property bool darkMode: true
    property string currentPage: "Environment"
    readonly property bool compact: width < 110
    signal pageSelected(string page)

    HyprbindTheme { id: theme; darkMode: root.darkMode }
    color: theme.sidebarFill

    Rectangle {
        width: 1
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.right: parent.right
        color: theme.divider
    }

    GlassField {
        id: navSearch
        visible: !root.compact
        anchors.top: parent.top
        anchors.topMargin: 14
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: 14
        anchors.rightMargin: 14
        implicitHeight: 39
        placeholderText: "Search settings..."
        leftPadding: 38
        darkMode: root.darkMode

        UiIcon {
            width: 17
            height: 17
            name: "search"
            anchors.left: parent.left
            anchors.leftMargin: 13
            anchors.verticalCenter: parent.verticalCenter
            iconColor: theme.textSecondary
        }

        Text {
            visible: root.width >= 240
            text: "Ctrl K"
            anchors.right: parent.right
            anchors.rightMargin: 10
            anchors.verticalCenter: parent.verticalCenter
            color: theme.alpha(theme.textSecondary, 0.62)
            font.family: "Inter"
            font.pixelSize: 10
        }
    }

    Flickable {
        id: navFlick
        anchors.top: root.compact ? parent.top : navSearch.bottom
        anchors.topMargin: root.compact ? 9 : 10
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 9
        clip: true
        contentHeight: navColumn.implicitHeight
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar { policy: root.compact ? ScrollBar.AlwaysOff : ScrollBar.AsNeeded }

        Column {
            id: navColumn
            width: navFlick.width
            spacing: root.compact ? 2 : 3

            Repeater {
                model: [
                    { title: "START HERE", items: [
                        { name: "Overview" }
                    ]},
                    { title: "KEYBOARD & CONFIG", items: [
                        { name: "Binds" },
                        { name: "Variables" },
                        { name: "Environment" },
                        { name: "Submaps" },
                        { name: "Startup" }
                    ]},
                    { title: "RULES", items: [
                        { name: "Window rules" },
                        { name: "Workspace rules" },
                        { name: "Layer rules" }
                    ]},
                    { title: "APPEARANCE & INPUT", items: [
                        { name: "Look & Feel" },
                        { name: "Config" },
                        { name: "Monitors" },
                        { name: "Devices" },
                        { name: "Animations" },
                        { name: "Curves" },
                        { name: "Gestures" }
                    ]},
                    { title: "SYSTEM", items: [
                        { name: "Health" },
                        { name: "Logs" }
                    ]}
                ]

                delegate: Column {
                    id: groupColumn
                    width: navColumn.width
                    property var groupData: modelData
                    spacing: root.compact ? 2 : 3

                    Item { width: 1; height: index === 0 ? 2 : (root.compact ? 7 : 9) }

                    Rectangle {
                        visible: root.compact && index > 0
                        width: 28
                        height: 1
                        anchors.horizontalCenter: parent.horizontalCenter
                        color: theme.divider
                    }

                    Text {
                        visible: !root.compact
                        text: groupColumn.groupData.title
                        color: theme.alpha(theme.textSecondary, 0.66)
                        font.family: "Inter"
                        font.pixelSize: 10
                        font.weight: Font.DemiBold
                        font.letterSpacing: 1.15
                        leftPadding: 20
                        height: visible ? 22 : 0
                        verticalAlignment: Text.AlignVCenter
                    }

                    Repeater {
                        model: groupColumn.groupData.items
                        delegate: NavItem {
                            property var itemData: modelData
                            width: root.compact ? 48 : groupColumn.width - 24
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: itemData.name
                            iconName: theme.pageIcon(itemData.name)
                            darkMode: root.darkMode
                            compact: root.compact
                            selected: root.currentPage === itemData.name
                            visible: root.compact || !navSearch.text || itemData.name.toLowerCase().indexOf(navSearch.text.toLowerCase()) >= 0
                            height: visible ? implicitHeight : 0
                            onClicked: root.pageSelected(itemData.name)
                        }
                    }
                }
            }
        }
    }
}
