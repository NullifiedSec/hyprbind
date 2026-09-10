import QtQuick
import QtQuick.Controls

Item {
    id: root

    property bool darkMode: true
    property string currentPage: "Environment"
    property string configPath: ""
    signal status(string message)

    HyprbindTheme { id: theme; darkMode: root.darkMode }

    readonly property bool fullPath: width >= 880
    readonly property bool showSubtitle: width >= 560
    readonly property bool configContext: currentPage !== "Overview" && currentPage !== "Health" && currentPage !== "Logs"
    readonly property int edgeMargin: width < 700 ? 16 : 24
    property bool copied: false

    height: width < 700 ? 82 : 94

    function pageIconName() {
        if (currentPage === "Environment") return "terminal"
        if (currentPage === "Look & Feel") return "diamond"
        if (currentPage === "Overview") return "home"
        if (currentPage === "Variables") return "braces"
        if (currentPage === "Binds") return "keyboard"
        if (currentPage === "Submaps") return "grid"
        if (currentPage === "Startup") return "power"
        if (currentPage === "Config") return "settings"
        if (currentPage === "Monitors") return "monitor"
        if (currentPage === "Devices") return "device"
        if (currentPage === "Animations") return "sparkle"
        if (currentPage === "Curves") return "curve"
        if (currentPage === "Gestures") return "gesture"
        if (currentPage === "Health") return "heart"
        if (currentPage === "Logs") return "logs"
        if (currentPage.indexOf("rule") >= 0) return "window"
        return "document"
    }

    function pageSubtitle() {
        if (currentPage === "Environment")
            return "Manage environment variables for Hyprland and apps launched in your session."
        if (currentPage === "Look & Feel")
            return "Shape Hyprland’s spacing, transparency, effects, and window borders."
        if (currentPage === "Overview")
            return "A quick view of your configuration and Hyprbind state."
        if (currentPage === "Variables")
            return "Reusable values referenced across binds, rules, and commands."
        if (currentPage === "Submaps")
            return "Organize keybinds into named modes and see which binds each mode owns."
        if (currentPage === "Startup")
            return "Choose which commands run when Hyprland starts, reloads, or shuts down."
        if (currentPage === "Health")
            return "Inspect session, Hyprland, portals, audio, and desktop integration."
        if (currentPage === "Logs")
            return "Search the recent user-session journal without leaving Hyprbind."
        return "This page is being migrated to the new interface."
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
            name: root.pageIconName()
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
            width: root.fullPath ? 40 : parent.width
            height: parent.height
            anchors.right: parent.right
            padding: 0
            flat: true
            hoverEnabled: true
            scale: 1

            contentItem: UiIcon {
                width: 18
                height: 18
                anchors.centerIn: parent
                name: root.copied ? "check" : "copy"
                iconColor: root.copied ? theme.accent : theme.textSecondary
            }

            background: Rectangle {
                radius: 10
                color: root.copied
                    ? theme.alpha(theme.accent, 0.12)
                    : copyPath.hovered ? theme.hoverFill : "transparent"
                border.width: root.copied ? 1 : 0
                border.color: theme.alpha(theme.accent, 0.24)
                Behavior on color { ColorAnimation { duration: 140 } }
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
            ToolTip.text: root.copied ? "Copied" : "Copy config path"
        }
    }

    SequentialAnimation {
        id: copyPulse
        PropertyAnimation { target: copyPath; property: "scale"; to: 0.88; duration: 70; easing.type: Easing.OutCubic }
        PropertyAnimation { target: copyPath; property: "scale"; to: 1.08; duration: 110; easing.type: Easing.OutBack }
        PropertyAnimation { target: copyPath; property: "scale"; to: 1.0; duration: 110; easing.type: Easing.OutCubic }
    }

    Timer {
        id: copiedReset
        interval: 1100
        repeat: false
        onTriggered: root.copied = false
    }

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
