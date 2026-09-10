import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    required property var bridge
    property bool darkMode: true
    property string configPath: ""
    property bool healthy: true
    property int environmentCount: 0
    property int variableCount: 0
    property bool backupAvailable: false
    property string backupAge: ""
    property bool blurEnabled: false
    property int rounding: 0
    property real inactiveOpacity: 1.0

    signal status(string message)
    signal configResolved(string path)
    signal healthChanged(bool healthy)

    HyprbindTheme { id: theme; darkMode: root.darkMode }

    function parse(raw) {
        try { return JSON.parse(raw) }
        catch (error) { return { ok: false, error: "Invalid Rust bridge response: " + error } }
    }

    function load() {
        const env = parse(bridge.environmentSnapshot())
        const vars = parse(bridge.variablesSnapshot())
        const look = parse(bridge.lookFeelSnapshot())
        const ui = parse(bridge.uiSnapshot())

        configPath = env.configPath || vars.configPath || ""
        configResolved(configPath)
        environmentCount = env.ok ? (env.env || []).length : 0
        variableCount = vars.ok ? (vars.variables || []).length : 0
        backupAvailable = ui.hasBackup === true
        backupAge = ui.backupAge || ""
        blurEnabled = look.blurEnabled === true
        rounding = look.rounding || 0
        inactiveOpacity = look.inactiveOpacity === undefined ? 1.0 : look.inactiveOpacity
        healthy = env.ok === true && vars.ok === true && look.ok === true && ui.ok === true
        healthChanged(healthy)

        if (!healthy) {
            status(env.error || vars.error || look.error || ui.error || "Overview loaded with errors.")
            return
        }
        status("Overview refreshed.")
    }

    Component.onCompleted: load()

    Flickable {
        anchors.fill: parent
        clip: true
        contentWidth: width
        contentHeight: contentColumn.implicitHeight
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

        ColumnLayout {
            id: contentColumn
            width: parent.width
            spacing: 14

            GridLayout {
                Layout.fillWidth: true
                columns: root.width >= 760 ? 2 : 1
                columnSpacing: 14
                rowSpacing: 14

                SummaryCard {
                    Layout.fillWidth: true
                    darkMode: root.darkMode
                    iconName: "document"
                    title: "ENVIRONMENT"
                    value: root.environmentCount.toString()
                    subtitle: root.environmentCount === 1 ? "1 environment variable" : root.environmentCount + " environment variables"
                    accent: true
                }

                SummaryCard {
                    Layout.fillWidth: true
                    darkMode: root.darkMode
                    iconName: "braces"
                    title: "VARIABLES"
                    value: root.variableCount.toString()
                    subtitle: root.variableCount === 1 ? "1 reusable config variable" : root.variableCount + " reusable config variables"
                }

                SummaryCard {
                    Layout.fillWidth: true
                    darkMode: root.darkMode
                    iconName: "diamond"
                    title: "WINDOW MATERIAL"
                    value: root.blurEnabled ? "Blur on" : "Blur off"
                    subtitle: "Rounding " + root.rounding + " · inactive opacity " + Number(root.inactiveOpacity).toFixed(2)
                }

                SummaryCard {
                    Layout.fillWidth: true
                    darkMode: root.darkMode
                    iconName: "history"
                    title: "LATEST SNAPSHOT"
                    value: root.backupAvailable ? "Available" : "None yet"
                    subtitle: root.backupAvailable
                        ? (root.backupAge.length ? root.backupAge : "A restore point is available")
                        : "A snapshot appears after Hyprbind writes config"
                }
            }

            GlassPanel {
                Layout.fillWidth: true
                Layout.preferredHeight: 116
                darkMode: root.darkMode
                cornerRadius: 15
                elevated: false

                Row {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.leftMargin: 18
                    anchors.rightMargin: 18
                    spacing: 15

                    Rectangle {
                        width: 42
                        height: 42
                        radius: 13
                        color: root.healthy ? theme.alpha(theme.accent, 0.07) : theme.dangerFill
                        border.width: 1
                        border.color: root.healthy ? theme.alpha(theme.accent, 0.14) : theme.dangerRim

                        UiIcon {
                            anchors.centerIn: parent
                            width: 20
                            height: 20
                            name: root.healthy ? "check" : "info"
                            iconColor: root.healthy ? theme.accent : theme.danger
                        }
                    }

                    Column {
                        width: parent.width - 57
                        spacing: 4
                        Text {
                            width: parent.width
                            text: root.healthy ? "Configuration loaded cleanly" : "Configuration needs attention"
                            color: theme.textPrimary
                            font.family: "Inter"
                            font.pixelSize: 15
                            font.weight: Font.DemiBold
                            elide: Text.ElideRight
                        }
                        Text {
                            width: parent.width
                            text: root.configPath.length ? root.configPath : "Hyprbind could not resolve the config path."
                            color: theme.alpha(theme.textSecondary, 0.80)
                            font.family: "Inter"
                            font.pixelSize: 11
                            elide: Text.ElideMiddle
                        }
                    }
                }
            }
        }
    }
}
