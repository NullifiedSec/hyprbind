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

    readonly property bool compactHero: width < 720
    readonly property int metricColumns: width >= 1080 ? 3 : width >= 640 ? 2 : 1

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
        contentHeight: contentColumn.implicitHeight + 4
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

        ColumnLayout {
            id: contentColumn
            width: parent.width
            spacing: 14

            GlassPanel {
                Layout.fillWidth: true
                Layout.preferredHeight: root.compactHero ? 224 : 156
                darkMode: root.darkMode
                cornerRadius: 17
                elevated: false
                tint: root.healthy ? theme.accent : theme.danger

                GridLayout {
                    anchors.fill: parent
                    anchors.margins: 18
                    columns: root.compactHero ? 1 : 2
                    columnSpacing: 28
                    rowSpacing: 14

                    RowLayout {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        spacing: 15

                        Rectangle {
                            Layout.preferredWidth: 48
                            Layout.preferredHeight: 48
                            radius: 14
                            color: root.healthy ? theme.alpha(theme.accent, 0.075) : theme.dangerFill
                            border.width: 1
                            border.color: root.healthy ? theme.alpha(theme.accent, 0.16) : theme.dangerRim

                            UiIcon {
                                anchors.centerIn: parent
                                width: 22
                                height: 22
                                name: root.healthy ? "check" : "info"
                                iconColor: root.healthy ? theme.accent : theme.danger
                                strokeWidth: 1.75
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 4

                            Text {
                                Layout.fillWidth: true
                                text: root.healthy ? "Configuration ready" : "Configuration needs attention"
                                color: theme.textPrimary
                                font.family: "Inter"
                                font.pixelSize: root.compactHero ? 19 : 21
                                font.weight: Font.DemiBold
                                elide: Text.ElideRight
                            }

                            Text {
                                Layout.fillWidth: true
                                text: root.healthy
                                    ? "Hyprbind is reading the managed configuration cleanly."
                                    : "One or more configuration sources could not be read."
                                color: theme.alpha(theme.textSecondary, 0.82)
                                font.family: "Inter"
                                font.pixelSize: 11
                                wrapMode: Text.WordWrap
                            }
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.alignment: Qt.AlignVCenter
                        spacing: 9

                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 47
                            radius: 12
                            color: theme.alpha(theme.controlFill, 0.72)
                            border.width: 1
                            border.color: theme.controlRim

                            Column {
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.verticalCenter: parent.verticalCenter
                                anchors.leftMargin: 13
                                anchors.rightMargin: 13
                                spacing: 2

                                Text {
                                    width: parent.width
                                    text: "CONFIG SOURCE"
                                    color: theme.alpha(theme.textSecondary, 0.64)
                                    font.family: "Inter"
                                    font.pixelSize: 9
                                    font.weight: Font.DemiBold
                                    font.letterSpacing: 0.8
                                }
                                Text {
                                    width: parent.width
                                    text: root.configPath.length ? root.configPath : "Config path unresolved"
                                    color: theme.alpha(theme.textPrimary, 0.88)
                                    font.family: "monospace"
                                    font.pixelSize: 10
                                    elide: Text.ElideMiddle
                                }
                            }
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 47
                            radius: 12
                            color: theme.alpha(theme.controlFill, 0.72)
                            border.width: 1
                            border.color: root.backupAvailable ? theme.alpha(theme.accent, 0.12) : theme.controlRim

                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: 13
                                anchors.rightMargin: 13
                                spacing: 10

                                Text {
                                    text: "RECOVERY"
                                    color: theme.alpha(theme.textSecondary, 0.64)
                                    font.family: "Inter"
                                    font.pixelSize: 9
                                    font.weight: Font.DemiBold
                                    font.letterSpacing: 0.8
                                }
                                Item { Layout.fillWidth: true }
                                Text {
                                    text: root.backupAvailable
                                        ? (root.backupAge.length ? root.backupAge : "Snapshot available")
                                        : "No snapshot yet"
                                    color: root.backupAvailable ? theme.alpha(theme.accent, 0.90) : theme.textSecondary
                                    font.family: "Inter"
                                    font.pixelSize: 10
                                    font.weight: Font.Medium
                                    elide: Text.ElideRight
                                }
                            }
                        }
                    }
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 10
                Text {
                    text: "AT A GLANCE"
                    color: theme.alpha(theme.textSecondary, 0.66)
                    font.family: "Inter"
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    font.letterSpacing: 1.0
                }
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 1
                    color: theme.divider
                }
            }

            GridLayout {
                Layout.fillWidth: true
                columns: root.metricColumns
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
                    iconName: "history"
                    title: "RESTORE POINT"
                    value: root.backupAvailable ? "Ready" : "None"
                    subtitle: root.backupAvailable
                        ? (root.backupAge.length ? root.backupAge : "A managed snapshot is available")
                        : "Created after Hyprbind writes config"
                    accent: root.backupAvailable
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 10
                Text {
                    text: "WINDOW FEEL"
                    color: theme.alpha(theme.textSecondary, 0.66)
                    font.family: "Inter"
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    font.letterSpacing: 1.0
                }
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 1
                    color: theme.divider
                }
            }

            GlassPanel {
                Layout.fillWidth: true
                Layout.preferredHeight: root.width < 640 ? 246 : 112
                darkMode: root.darkMode
                cornerRadius: 15
                elevated: false

                GridLayout {
                    anchors.fill: parent
                    anchors.margins: 16
                    columns: root.width < 640 ? 1 : 3
                    columnSpacing: 0
                    rowSpacing: 8

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 3
                        Text { text: "BLUR"; color: theme.alpha(theme.textSecondary, 0.64); font.family: "Inter"; font.pixelSize: 9; font.weight: Font.DemiBold; font.letterSpacing: 0.8 }
                        Text { text: root.blurEnabled ? "Enabled" : "Disabled"; color: root.blurEnabled ? theme.accent : theme.textPrimary; font.family: "Inter"; font.pixelSize: 18; font.weight: Font.DemiBold }
                        Text { text: "Backdrop blur state"; color: theme.textSecondary; font.family: "Inter"; font.pixelSize: 10 }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 3
                        Text { text: "ROUNDING"; color: theme.alpha(theme.textSecondary, 0.64); font.family: "Inter"; font.pixelSize: 9; font.weight: Font.DemiBold; font.letterSpacing: 0.8 }
                        Text { text: root.rounding + " px"; color: theme.textPrimary; font.family: "Inter"; font.pixelSize: 18; font.weight: Font.DemiBold }
                        Text { text: "Window corner radius"; color: theme.textSecondary; font.family: "Inter"; font.pixelSize: 10 }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 3
                        Text { text: "INACTIVE OPACITY"; color: theme.alpha(theme.textSecondary, 0.64); font.family: "Inter"; font.pixelSize: 9; font.weight: Font.DemiBold; font.letterSpacing: 0.8 }
                        Text { text: Number(root.inactiveOpacity).toFixed(2); color: theme.textPrimary; font.family: "Inter"; font.pixelSize: 18; font.weight: Font.DemiBold }
                        Text { text: "Unfocused window opacity"; color: theme.textSecondary; font.family: "Inter"; font.pixelSize: 10 }
                    }
                }
            }
        }
    }
}
