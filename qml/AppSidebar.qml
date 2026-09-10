import QtQuick
import QtQuick.Controls

Rectangle {
    id: root

    property bool darkMode: true
    property string currentPage: "Environment"
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
        anchors.top: navSearch.bottom
        anchors.topMargin: 10
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 9
        clip: true
        contentHeight: navColumn.implicitHeight
        boundsBehavior: Flickable.StopAtBounds

        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

        Column {
            id: navColumn
            width: navFlick.width
            spacing: 3

            Repeater {
                model: [
                    { title: "START HERE", items: [
                        { name: "Overview", iconName: "home" }
                    ]},
                    { title: "KEYBOARD & CONFIG", items: [
                        { name: "Binds", iconName: "keyboard" },
                        { name: "Variables", iconName: "braces" },
                        { name: "Environment", iconName: "document" },
                        { name: "Submaps", iconName: "grid" },
                        { name: "Startup", iconName: "power" }
                    ]},
                    { title: "RULES", items: [
                        { name: "Window rules", iconName: "window" },
                        { name: "Workspace rules", iconName: "grid" },
                        { name: "Layer rules", iconName: "layers" }
                    ]},
                    { title: "APPEARANCE & INPUT", items: [
                        { name: "Look & Feel", iconName: "diamond" },
                        { name: "Config", iconName: "settings" },
                        { name: "Monitors", iconName: "monitor" },
                        { name: "Devices", iconName: "device" },
                        { name: "Animations", iconName: "sparkle" },
                        { name: "Curves", iconName: "curve" },
                        { name: "Gestures", iconName: "gesture" }
                    ]},
                    { title: "SYSTEM", items: [
                        { name: "Health", iconName: "heart" },
                        { name: "Logs", iconName: "logs" }
                    ]}
                ]

                delegate: Column {
                    id: groupColumn
                    width: navColumn.width
                    property var groupData: modelData
                    spacing: 3

                    Item { width: 1; height: index === 0 ? 3 : 9 }

                    Text {
                        text: groupColumn.groupData.title
                        color: theme.alpha(theme.textSecondary, 0.66)
                        font.family: "Inter"
                        font.pixelSize: 10
                        font.weight: Font.DemiBold
                        font.letterSpacing: 1.15
                        leftPadding: 20
                        height: 22
                        verticalAlignment: Text.AlignVCenter
                    }

                    Repeater {
                        model: groupColumn.groupData.items
                        delegate: NavItem {
                            property var itemData: modelData
                            width: groupColumn.width - 24
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: itemData.name
                            iconName: itemData.iconName
                            darkMode: root.darkMode
                            selected: root.currentPage === itemData.name
                            visible: !navSearch.text || itemData.name.toLowerCase().indexOf(navSearch.text.toLowerCase()) >= 0
                            height: visible ? implicitHeight : 0
                            onClicked: root.pageSelected(itemData.name)
                        }
                    }
                }
            }
        }
    }
}
