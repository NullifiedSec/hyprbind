import QtQuick
import QtQuick.Controls
import QtQuick.Window
import dev.hyprbinds.ui

ApplicationWindow {
    id: appWindow

    width: 1440
    height: 900
    minimumWidth: 620
    minimumHeight: 500
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

    readonly property int sidebarWidth: width < 900 ? 72 : width < 1180 ? 240 : 286
    readonly property int contentMargin: width < 900 ? 10 : width < 1180 ? 16 : 24

    HyprbindBridge { id: backend }
    HyprbindTheme { id: theme; darkMode: appWindow.darkMode }

    function parsePayload(raw) {
        try { return JSON.parse(raw) }
        catch (error) { return { ok: false, error: "Invalid Rust bridge response: " + error } }
    }

    function loadUiState() {
        const payload = parsePayload(backend.uiSnapshot())
        if (!payload.ok) return
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
        statusMessage = realtimeEnabled ? "Live preview enabled on supported pages." : "Live preview disabled."
    }

    function reloadCurrentPage() {
        loadUiState()
        if (pageLoader.item && typeof pageLoader.item.load === "function") {
            pageLoader.item.load()
            return
        }
        statusMessage = "This page does not expose reload."
    }

    function restoreLastBackup() {
        const payload = parsePayload(backend.restoreConfig())
        if (!payload.ok) {
            statusMessage = payload.error || "Restore failed."
            return
        }
        loadUiState()
        if (pageLoader.item && typeof pageLoader.item.load === "function") pageLoader.item.load()
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
                anchors.bottomMargin: 10
                cornerRadius: 14
                darkMode: appWindow.darkMode
                elevated: false

                Loader {
                    id: pageLoader
                    anchors.fill: parent
                    anchors.margins: appWindow.currentPage === "Look & Feel" ? 0 : (appWindow.width < 900 ? 10 : 14)
                    sourceComponent: appWindow.currentPage === "Overview" ? overviewComponent
                        : appWindow.currentPage === "Binds" ? bindsComponent
                        : appWindow.currentPage === "Variables" ? variablesComponent
                        : appWindow.currentPage === "Environment" ? environmentComponent
                        : appWindow.currentPage === "Submaps" ? submapsComponent
                        : appWindow.currentPage === "Startup" ? startupComponent
                        : appWindow.currentPage === "Window rules" ? windowRulesComponent
                        : appWindow.currentPage === "Workspace rules" ? workspaceRulesComponent
                        : appWindow.currentPage === "Layer rules" ? layerRulesComponent
                        : appWindow.currentPage === "Look & Feel" ? lookFeelComponent
                        : appWindow.currentPage === "Config" ? configComponent
                        : appWindow.currentPage === "Monitors" ? monitorsComponent
                        : appWindow.currentPage === "Devices" ? devicesComponent
                        : appWindow.currentPage === "Animations" ? animationsComponent
                        : appWindow.currentPage === "Curves" ? curvesComponent
                        : appWindow.currentPage === "Gestures" ? gesturesComponent
                        : appWindow.currentPage === "Health" ? healthComponent
                        : logsComponent
                }
            }
        }
    }

    Component { id: overviewComponent; OverviewPage { bridge: backend; darkMode: appWindow.darkMode; onStatus: message => appWindow.statusMessage = message; onConfigResolved: path => appWindow.configPath = path; onHealthChanged: healthy => appWindow.backendHealthy = healthy } }
    Component { id: bindsComponent; BindsPage { darkMode: appWindow.darkMode; onStatus: message => appWindow.statusMessage = message; onConfigResolved: path => appWindow.configPath = path; onHealthChanged: healthy => appWindow.backendHealthy = healthy } }
    Component { id: variablesComponent; VariablesPage { bridge: backend; darkMode: appWindow.darkMode; onStatus: message => appWindow.statusMessage = message; onConfigResolved: path => appWindow.configPath = path; onHealthChanged: healthy => appWindow.backendHealthy = healthy } }
    Component { id: environmentComponent; EnvironmentPage { bridge: backend; darkMode: appWindow.darkMode; onStatus: message => appWindow.statusMessage = message; onConfigResolved: path => appWindow.configPath = path; onHealthChanged: healthy => appWindow.backendHealthy = healthy } }
    Component { id: submapsComponent; SubmapsPage { darkMode: appWindow.darkMode; onStatus: message => appWindow.statusMessage = message; onConfigResolved: path => appWindow.configPath = path; onHealthChanged: healthy => appWindow.backendHealthy = healthy } }
    Component { id: startupComponent; StartupPage { darkMode: appWindow.darkMode; onStatus: message => appWindow.statusMessage = message; onConfigResolved: path => appWindow.configPath = path; onHealthChanged: healthy => appWindow.backendHealthy = healthy } }

    Component { id: windowRulesComponent; RuleCatalogPage { kind: "window_rules"; darkMode: appWindow.darkMode; onStatus: message => appWindow.statusMessage = message; onConfigResolved: path => appWindow.configPath = path; onHealthChanged: healthy => appWindow.backendHealthy = healthy } }
    Component { id: workspaceRulesComponent; SpecCatalogPage { kind: "workspace_rules"; darkMode: appWindow.darkMode; onStatus: message => appWindow.statusMessage = message; onConfigResolved: path => appWindow.configPath = path; onHealthChanged: healthy => appWindow.backendHealthy = healthy } }
    Component { id: layerRulesComponent; RuleCatalogPage { kind: "layer_rules"; darkMode: appWindow.darkMode; onStatus: message => appWindow.statusMessage = message; onConfigResolved: path => appWindow.configPath = path; onHealthChanged: healthy => appWindow.backendHealthy = healthy } }

    Component {
        id: lookFeelComponent
        LookFeelPage {
            bridge: backend
            darkMode: appWindow.darkMode
            realtimeEnabled: appWindow.realtimeEnabled
            onStatus: message => appWindow.statusMessage = message
            onSaved: { appWindow.loadUiState(); appWindow.statusMessage = "Look & Feel saved." }
        }
    }

    Component { id: configComponent; ConfigPage { darkMode: appWindow.darkMode; onStatus: message => appWindow.statusMessage = message; onConfigResolved: path => appWindow.configPath = path; onHealthChanged: healthy => appWindow.backendHealthy = healthy } }
    Component { id: monitorsComponent; SpecCatalogPage { kind: "monitors"; darkMode: appWindow.darkMode; onStatus: message => appWindow.statusMessage = message; onConfigResolved: path => appWindow.configPath = path; onHealthChanged: healthy => appWindow.backendHealthy = healthy } }
    Component { id: devicesComponent; SpecCatalogPage { kind: "devices"; darkMode: appWindow.darkMode; onStatus: message => appWindow.statusMessage = message; onConfigResolved: path => appWindow.configPath = path; onHealthChanged: healthy => appWindow.backendHealthy = healthy } }
    Component { id: animationsComponent; SpecCatalogPage { kind: "animations"; darkMode: appWindow.darkMode; onStatus: message => appWindow.statusMessage = message; onConfigResolved: path => appWindow.configPath = path; onHealthChanged: healthy => appWindow.backendHealthy = healthy } }
    Component { id: curvesComponent; SpecCatalogPage { kind: "curves"; darkMode: appWindow.darkMode; onStatus: message => appWindow.statusMessage = message; onConfigResolved: path => appWindow.configPath = path; onHealthChanged: healthy => appWindow.backendHealthy = healthy } }
    Component { id: gesturesComponent; SpecCatalogPage { kind: "gestures"; darkMode: appWindow.darkMode; onStatus: message => appWindow.statusMessage = message; onConfigResolved: path => appWindow.configPath = path; onHealthChanged: healthy => appWindow.backendHealthy = healthy } }

    Component { id: healthComponent; HealthPage { darkMode: appWindow.darkMode; onStatus: message => appWindow.statusMessage = message; onHealthChanged: healthy => appWindow.backendHealthy = healthy } }
    Component { id: logsComponent; LogsPage { darkMode: appWindow.darkMode; onStatus: message => appWindow.statusMessage = message; onHealthChanged: healthy => appWindow.backendHealthy = healthy } }
}
