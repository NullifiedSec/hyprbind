import QtQuick
import QtQuick.Controls
import QtQuick.Window
import dev.hyprbinds.ui

ApplicationWindow {
    id: appWindow

    width: 1440
    height: 900
    minimumWidth: 760
    minimumHeight: 560
    visible: true
    title: "Hyprbind"
    color: "transparent"
    flags: Qt.Window | Qt.FramelessWindowHint

    property string currentPage: "Environment"
    property string configPath: ""
    property string statusMessage: "Ready."
    property bool backendHealthy: true
    property bool darkMode: true
    property bool realtimeEnabled: false
    property bool backupAvailable: false
    property string backupAge: ""

    readonly property int sidebarWidth: width < 900 ? 210 : width < 1180 ? 240 : 286
    readonly property int contentMargin: width < 900 ? 12 : width < 1180 ? 18 : 24

    HyprbindBridge { id: backend }
    HyprbindTheme { id: theme; darkMode: appWindow.darkMode }

    function parsePayload(raw) {
        try { return JSON.parse(raw) }
        catch (error) { return { ok: false, error: "Invalid Rust bridge response: " + error } }
    }

    function loadUiState() {
        const payload = parsePayload(backend.uiSnapshot())
        if (!payload.ok)
            return
        darkMode = payload.darkMode !== false
        backupAvailable = payload.hasBackup === true
        backupAge = payload.backupAge || ""
    }

    function toggleDarkMode() {
        const next = !darkMode
        const payload = parsePayload(backend.setDarkMode(next))
        if (!payload.ok) {
            statusMessage = payload.error || "Could not save the theme preference."
            return
        }
        darkMode = next
        statusMessage = payload.message || (next ? "Dark theme enabled." : "Light theme enabled.")
    }

    function toggleRealtime() {
        realtimeEnabled = !realtimeEnabled
        statusMessage = realtimeEnabled
            ? "Live preview enabled on supported pages."
            : "Live preview disabled."
    }

    function reloadCurrentPage() {
        loadUiState()
        if (pageLoader.item && typeof pageLoader.item.load === "function") {
            pageLoader.item.load()
            return
        }
        statusMessage = "This page does not expose reload yet."
    }

    function restoreLastBackup() {
        const payload = parsePayload(backend.restoreConfig())
        if (!payload.ok) {
            statusMessage = payload.error || "Restore failed."
            return
        }
        loadUiState()
        if (pageLoader.item && typeof pageLoader.item.load === "function")
            pageLoader.item.load()
        statusMessage = payload.message || "Restored the latest config snapshot."
    }

    onCurrentPageChanged: {
        backendHealthy = true
        statusMessage = "Ready."
    }

    Component.onCompleted: loadUiState()

    Rectangle {
        id: shell
        anchors.fill: parent
        anchors.margins: 1
        radius: 18
        clip: true
        color: theme.shellFill
        border.width: 1
        border.color: theme.shellRim
        antialiasing: true

        Rectangle {
            anchors.fill: parent
            radius: shell.radius
            antialiasing: true
            color: "transparent"
            gradient: Gradient {
                GradientStop { position: 0.00; color: theme.shellTopSpecular }
                GradientStop { position: 0.20; color: theme.shellAccentWash }
                GradientStop { position: 0.74; color: "transparent" }
                GradientStop { position: 1.00; color: theme.shellBottomShade }
            }
        }

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.leftMargin: 28
            anchors.rightMargin: 28
            anchors.top: parent.top
            anchors.topMargin: 1
            height: 1
            color: theme.shellInnerLine
        }

        AppTopBar {
            id: topBar
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            appWindow: appWindow
            darkMode: appWindow.darkMode
            realtimeEnabled: appWindow.realtimeEnabled
            backupAvailable: appWindow.backupAvailable
            backupAge: appWindow.backupAge
            onToggleThemeRequested: appWindow.toggleDarkMode()
            onToggleRealtimeRequested: appWindow.toggleRealtime()
            onRestoreRequested: appWindow.restoreLastBackup()
            onReloadRequested: appWindow.reloadCurrentPage()
        }

        AppSidebar {
            id: sidebar
            width: appWindow.sidebarWidth
            anchors.left: parent.left
            anchors.top: topBar.bottom
            anchors.bottom: parent.bottom
            darkMode: appWindow.darkMode
            currentPage: appWindow.currentPage
            onPageSelected: page => appWindow.currentPage = page
        }

        Item {
            id: mainArea
            anchors.left: sidebar.right
            anchors.right: parent.right
            anchors.top: topBar.bottom
            anchors.bottom: parent.bottom

            PageHeader {
                id: pageHeader
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                currentPage: appWindow.currentPage
                configPath: appWindow.configPath
                darkMode: appWindow.darkMode
                onStatus: message => appWindow.statusMessage = message
            }

            StatusBar {
                id: statusBar
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                anchors.leftMargin: appWindow.contentMargin
                anchors.rightMargin: appWindow.contentMargin
                anchors.bottomMargin: appWindow.contentMargin
                darkMode: appWindow.darkMode
                healthy: appWindow.backendHealthy
                message: appWindow.statusMessage
            }

            GlassPanel {
                id: contentPanel
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: pageHeader.bottom
                anchors.bottom: statusBar.top
                anchors.leftMargin: appWindow.contentMargin
                anchors.rightMargin: appWindow.contentMargin
                anchors.topMargin: 0
                anchors.bottomMargin: 10
                cornerRadius: 14
                darkMode: appWindow.darkMode
                elevated: false

                Loader {
                    id: pageLoader
                    anchors.fill: parent
                    anchors.margins: appWindow.currentPage === "Look & Feel" ? 0 : 14
                    sourceComponent: appWindow.currentPage === "Overview"
                        ? overviewComponent
                        : appWindow.currentPage === "Environment"
                            ? environmentComponent
                            : appWindow.currentPage === "Variables"
                                ? variablesComponent
                                : appWindow.currentPage === "Submaps"
                                    ? submapsComponent
                                    : appWindow.currentPage === "Startup"
                                        ? startupComponent
                                        : appWindow.currentPage === "Look & Feel"
                                            ? lookFeelComponent
                                            : appWindow.currentPage === "Health"
                                                ? healthComponent
                                                : appWindow.currentPage === "Logs"
                                                    ? logsComponent
                                                    : placeholderComponent
                }
            }
        }
    }

    Component {
        id: overviewComponent
        OverviewPage {
            bridge: backend
            darkMode: appWindow.darkMode
            onStatus: message => appWindow.statusMessage = message
            onConfigResolved: path => appWindow.configPath = path
            onHealthChanged: healthy => appWindow.backendHealthy = healthy
        }
    }

    Component {
        id: environmentComponent
        EnvironmentPage {
            bridge: backend
            darkMode: appWindow.darkMode
            onStatus: message => appWindow.statusMessage = message
            onConfigResolved: path => appWindow.configPath = path
            onHealthChanged: healthy => appWindow.backendHealthy = healthy
        }
    }

    Component {
        id: variablesComponent
        VariablesPage {
            bridge: backend
            darkMode: appWindow.darkMode
            onStatus: message => appWindow.statusMessage = message
            onConfigResolved: path => appWindow.configPath = path
            onHealthChanged: healthy => appWindow.backendHealthy = healthy
        }
    }

    Component {
        id: submapsComponent
        SubmapsPage {
            darkMode: appWindow.darkMode
            onStatus: message => appWindow.statusMessage = message
            onConfigResolved: path => appWindow.configPath = path
            onHealthChanged: healthy => appWindow.backendHealthy = healthy
        }
    }

    Component {
        id: startupComponent
        StartupPage {
            darkMode: appWindow.darkMode
            onStatus: message => appWindow.statusMessage = message
            onConfigResolved: path => appWindow.configPath = path
            onHealthChanged: healthy => appWindow.backendHealthy = healthy
        }
    }

    Component {
        id: lookFeelComponent
        LookFeelPage {
            bridge: backend
            darkMode: appWindow.darkMode
            realtimeEnabled: appWindow.realtimeEnabled
            onStatus: message => appWindow.statusMessage = message
            onSaved: {
                appWindow.loadUiState()
                appWindow.statusMessage = "Look & Feel saved."
            }
        }
    }

    Component {
        id: healthComponent
        HealthPage {
            darkMode: appWindow.darkMode
            onStatus: message => appWindow.statusMessage = message
            onHealthChanged: healthy => appWindow.backendHealthy = healthy
        }
    }

    Component {
        id: logsComponent
        LogsPage {
            darkMode: appWindow.darkMode
            onStatus: message => appWindow.statusMessage = message
            onHealthChanged: healthy => appWindow.backendHealthy = healthy
        }
    }

    Component {
        id: placeholderComponent
        Item {
            Column {
                anchors.centerIn: parent
                width: Math.min(parent.width - 40, 460)
                spacing: 12

                UiIcon {
                    width: 28
                    height: 28
                    anchors.horizontalCenter: parent.horizontalCenter
                    name: pageHeader.pageIconName()
                    iconColor: theme.alpha(theme.textSecondary, 0.72)
                }

                Text {
                    text: appWindow.currentPage
                    color: theme.textPrimary
                    font.family: "Inter"
                    font.pixelSize: 19
                    font.weight: Font.DemiBold
                    anchors.horizontalCenter: parent.horizontalCenter
                }

                Text {
                    width: parent.width
                    text: "This page is next in the QML migration. The existing GTK frontend still owns its behavior for now."
                    color: theme.textSecondary
                    font.family: "Inter"
                    font.pixelSize: 12
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                }
            }
        }
    }
}
