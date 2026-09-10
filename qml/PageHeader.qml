import QtQuick
import QtQuick.Controls

Item {
    id: root

    property bool darkMode: true
    property string currentPage: "Environment"
    property string configPath: ""
    signal status(string message)

    HyprbindTheme { id: theme; darkMode: root.darkMode }

    readonly property bool fullPath: width >= 920
    readonly property bool showSubtitle: width >= 560
    readonly property bool configContext: currentPage !== "Overview" && currentPage !== "Health" && currentPage !== "Logs" && currentPage !== "Import / Export" && currentPage !== "Developer tools"
    readonly property int edgeMargin: width < 700 ? 16 : 24
    property bool copied: false

    height: width < 700 ? 82 : 94

    function pageSubtitle() {
        if (currentPage === "Environment") return "Manage environment variables for Hyprland and apps launched in your session."
        if (currentPage === "Look & Feel") return "Shape Hyprland’s spacing, transparency, effects, and window borders."
        if (currentPage === "Overview") return "A quick view of your configuration and Hyprbind state."
        if (currentPage === "Binds") return "Inspect, search, add, and safely edit keybinds across global and submap contexts."
        if (currentPage === "Variables") return "Reusable values referenced across binds, rules, and commands."
        if (currentPage === "Submaps") return "Organize keybinds into named modes and see which binds each mode owns."
        if (currentPage === "Startup") return "Choose which commands run when Hyprland starts, reloads, or shuts down."
        if (currentPage === "Window rules") return "Match windows and apply behavior without leaving the managed configuration workflow."
        if (currentPage === "Workspace rules") return "Control per-workspace selectors, gaps, layouts, monitors, and related properties."
        if (currentPage === "Layer rules") return "Match layer-shell namespaces such as bars and launchers and apply layer effects."
        if (currentPage === "Config") return "Inspect and edit the merged configuration through Hyprbind’s managed override writer."
        if (currentPage === "Monitors") return "Manage collected monitor calls and their typed output configuration fields."
        if (currentPage === "Devices") return "Configure device-specific input behavior using the existing typed call writer."
        if (currentPage === "Animations") return "Edit animation leaves, speed, curves, and related animation fields."
        if (currentPage === "Curves") return "Create and edit named animation curves while preserving typed curve data."
        if (currentPage === "Gestures") return "Manage gesture calls, actions, fingers, direction, and related fields."
        if (currentPage === "Health") return "Inspect session, Hyprland, portals, audio, and desktop integration."
        if (currentPage === "Logs") return "Search the recent user-session journal without leaving Hyprbind."
        if (currentPage === "Import / Export") return "Back up and restore Hyprbind settings through the existing Rust bundle pipeline."
        if (currentPage === "Developer tools") return "Access the existing developer-only companion studios during the QML migration."
        return "Hyprland configuration."
    }

    GlassPanel {
        id: pageIcon
        width: root.width < 700 ? 44 : 52
        height: width
        anchors.left: parent.left
        anchors.leftMargin: root.edgeMargin
        anchors.verticalCenter: parent.verticalCenter
        cornerRadius: root.width < 700 ? 11 : 12
        darkMode: root.darkMode
        elevated: false
        tint: theme.accent
        border.color: theme.alpha(theme.accent, 0.14)

        Rectangle {
            anchors.fill: parent
            anchors.margins: 8
            radius: 9
            color: theme.alpha(theme.mix(theme.surfaceHigh, theme.accent, 0.08), root.darkMode ? 0.12 : 0.18)
        }

        UiIcon {
            anchors.centerIn: parent
            width: root.width < 700 ? 21 : 24
            height: width
            name: theme.pageIcon(root.currentPage)
            iconColor: theme.accent
            strokeWidth: 1.75
        }
    }

    GlassPanel {
        id: pathPanel
        width: root.fullPath ? 320 : 44
        height: 43
        anchors.right: parent.right
        anchors.rightMargin: root.edgeMargin
        anchors.verticalCenter: parent.verticalCenter
        darkMode: root.darkMode
        elevated: false
        visible: root.configContext && root.configPath.length > 0

        TextInput {
            id: pathText
            x: root.fullPath ? 14 : 1
            width: root.fullPath ? Math.max(1, copyPath.x - 22) : 1
            anchors.verticalCenter: parent.verticalCenter
            text: root.configPath
            readOnly: true
            selectByMouse: root.fullPath
            opacity: root.fullPath ? 1 : 0
            color: theme.alpha(theme.textPrimary, 0.80)
            font.family: "Inter"
            font.pixelSize: 11
            clip: true
        }

        Button {
            id: copyPath
            width: root.fullPath ? 42 : parent.width
            height: parent.height
            anchors.right: parent.right
            padding: 0
            flat: true
            hoverEnabled: true
            scale: 1

            contentItem: Item {
                Item {
                    width: 20
                    height: 20
                    anchors.centerIn: parent
                    opacity: root.copied ? 0 : 1
                    scale: root.copied ? 0.78 : 1

                    Rectangle {
                        x: 3
                        y: 4
                        width: 9
                        height: 1
                        radius: 0.5
                        color: copyPath.hovered ? theme.alpha(theme.textPrimary, 0.94) : theme.alpha(theme.textSecondary, 0.88)
                    }
                    Rectangle {
                        x: 3
                        y: 4
                        width: 1
                        height: 9
                        radius: 0.5
                        color: copyPath.hovered ? theme.alpha(theme.textPrimary, 0.94) : theme.alpha(theme.textSecondary, 0.88)
                    }
                    Rectangle {
                        x: 3
                        y: 12
                        width: 4
                        height: 1
                        radius: 0.5
                        color: copyPath.hovered ? theme.alpha(theme.textPrimary, 0.94) : theme.alpha(theme.textSecondary, 0.88)
                    }
                    Rectangle {
                        x: 7
                        y: 7
                        width: 10
                        height: 10
                        radius: 2
                        color: "transparent"
                        border.width: 1
                        border.color: copyPath.hovered ? theme.alpha(theme.textPrimary, 0.94) : theme.alpha(theme.textSecondary, 0.88)
                        antialiasing: true
                    }

                    Behavior on opacity { NumberAnimation { duration: 100 } }
                    Behavior on scale { NumberAnimation { duration: 110; easing.type: Easing.OutCubic } }
                }

                UiIcon {
                    anchors.centerIn: parent
                    width: 18
                    height: 18
                    name: "check"
                    iconColor: theme.accent
                    opacity: root.copied ? 1 : 0
                    scale: root.copied ? 1 : 0.72
                    Behavior on opacity { NumberAnimation { duration: 120 } }
                    Behavior on scale { NumberAnimation { duration: 140; easing.type: Easing.OutBack } }
                }
            }

            background: Rectangle {
                radius: 10
                color: root.copied
                    ? theme.alpha(theme.accent, 0.10)
                    : copyPath.pressed
                        ? theme.controlPressed
                        : copyPath.hovered ? theme.controlHover : "transparent"
                border.width: copyPath.hovered || root.copied ? 1 : 0
                border.color: root.copied ? theme.alpha(theme.accent, 0.22) : theme.controlRim
                Behavior on color { ColorAnimation { duration: 120 } }
                Behavior on border.color { ColorAnimation { duration: 120 } }
            }

            onClicked: {
                pathText.forceActiveFocus()
                pathText.selectAll()
                pathText.copy()
                pathText.deselect()
                root.copied = true
                copyPulse.restart()
                copiedReset.restart()
                root.status("Copied config path.")
            }

            ToolTip.visible: hovered
            ToolTip.delay: 420
            ToolTip.text: root.copied ? "Copied" : "Copy config path"
        }
    }

    SequentialAnimation {
        id: copyPulse
        PropertyAnimation { target: copyPath; property: "scale"; to: 0.94; duration: 65; easing.type: Easing.OutCubic }
        PropertyAnimation { target: copyPath; property: "scale"; to: 1.035; duration: 95; easing.type: Easing.OutBack }
        PropertyAnimation { target: copyPath; property: "scale"; to: 1.0; duration: 90; easing.type: Easing.OutCubic }
    }

    Timer { id: copiedReset; interval: 1050; repeat: false; onTriggered: root.copied = false }

    Column {
        id: titleColumn
        anchors.left: pageIcon.right
        anchors.leftMargin: root.width < 700 ? 13 : 18
        anchors.right: pathPanel.visible ? pathPanel.left : parent.right
        anchors.rightMargin: pathPanel.visible ? 16 : root.edgeMargin
        anchors.verticalCenter: parent.verticalCenter
        spacing: 4

        Text {
            width: parent.width
            text: root.currentPage
            color: theme.textPrimary
            font.family: "Inter"
            font.pixelSize: root.width < 700 ? 22 : 25
            font.weight: Font.DemiBold
            elide: Text.ElideRight
        }

        Text {
            visible: root.showSubtitle && parent.width > 220
            width: parent.width
            text: root.pageSubtitle()
            color: theme.textSecondary
            font.family: "Inter"
            font.pixelSize: 12
            elide: Text.ElideRight
        }
    }
}
