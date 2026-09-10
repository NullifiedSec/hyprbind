import QtQuick
import QtQuick.Controls

Item {
    id: root

    property bool darkMode: true
    property var allChecks: []
    property string severityFilter: "ALL"
    property int total: 0
    property int warnings: 0
    property int failures: 0
    readonly property bool compactToolbar: width < 760

    signal status(string message)
    signal healthChanged(bool healthy)

    HyprbindTheme { id: theme; darkMode: root.darkMode }
    HealthBridge { id: healthBridge }
    ListModel { id: checkModel }

    function parse(raw) {
        try { return JSON.parse(raw) }
        catch (error) { return { ok: false, error: "Invalid diagnostics response: " + error } }
    }

    function load() {
        const payload = parse(healthBridge.snapshot())
        if (!payload.ok) {
            allChecks = []
            total = 0
            warnings = 0
            failures = 1
            healthChanged(false)
            status(payload.error || "Could not run health diagnostics.")
            rebuildModel()
            return
        }
        allChecks = payload.checks || []
        total = payload.total || allChecks.length
        warnings = payload.warnings || 0
        failures = payload.failures || 0
        healthChanged(failures === 0)
        status(failures > 0
            ? failures + " health check" + (failures === 1 ? " is" : "s are") + " failing."
            : warnings > 0
                ? warnings + " diagnostic warning" + (warnings === 1 ? " remains." : "s remain.")
                : "All health checks passed.")
        rebuildModel()
    }

    function severityMatches(severity) {
        if (severityFilter === "ALL") return true
        if (severityFilter === "ISSUES") return severity === "FAIL" || severity === "WARN"
        if (severityFilter === "WARN") return severity === "WARN"
        if (severityFilter === "OK") return severity === "OK"
        return true
    }

    function rebuildModel() {
        const query = searchField.text.trim().toLowerCase()
        checkModel.clear()
        for (let i = 0; i < allChecks.length; ++i) {
            const item = allChecks[i]
            const haystack = (item.category + " " + item.title + " " + item.detail + " " + item.fixHint + " " + item.fixCommand).toLowerCase()
            if (severityMatches(item.severity) && (!query || haystack.indexOf(query) >= 0))
                checkModel.append(item)
        }
    }

    function severityColor(severity) {
        if (severity === "FAIL") return root.darkMode ? "#e39b96" : "#a84843"
        if (severity === "WARN") return root.darkMode ? "#e6bd79" : "#98651d"
        if (severity === "OK") return root.darkMode ? "#9bd2ad" : "#31744a"
        return theme.textSecondary
    }

    Component.onCompleted: load()

    Column {
        anchors.fill: parent
        spacing: 12

        Grid {
            id: summaryGrid
            width: parent.width
            columns: root.width >= 760 ? 3 : 1
            spacing: 10

            SummaryCard {
                width: summaryGrid.columns === 3 ? (summaryGrid.width - 20) / 3 : summaryGrid.width
                darkMode: root.darkMode
                iconName: "heart"
                title: "CHECKS"
                value: root.total.toString()
                subtitle: "Desktop and session diagnostics"
                accent: root.failures === 0
            }
            SummaryCard {
                width: summaryGrid.columns === 3 ? (summaryGrid.width - 20) / 3 : summaryGrid.width
                darkMode: root.darkMode
                iconName: "info"
                title: "WARNINGS"
                value: root.warnings.toString()
                subtitle: root.warnings === 1 ? "One item needs attention" : "Items worth reviewing"
            }
            SummaryCard {
                width: summaryGrid.columns === 3 ? (summaryGrid.width - 20) / 3 : summaryGrid.width
                darkMode: root.darkMode
                iconName: "heart"
                title: "FAILURES"
                value: root.failures.toString()
                subtitle: root.failures === 0 ? "No blocking checks" : "Checks that are currently failing"
                accent: root.failures === 0
            }
        }

        Item {
            width: parent.width
            height: root.compactToolbar ? 90 : 44

            GlassField {
                id: searchField
                anchors.left: parent.left
                anchors.right: root.compactToolbar ? parent.right : filterRow.left
                anchors.rightMargin: root.compactToolbar ? 0 : 10
                anchors.top: parent.top
                implicitHeight: 42
                darkMode: root.darkMode
                placeholderText: "Filter diagnostics..."
                leftPadding: 40
                onTextChanged: root.rebuildModel()

                UiIcon {
                    width: 16
                    height: 16
                    name: "search"
                    anchors.left: parent.left
                    anchors.leftMargin: 13
                    anchors.verticalCenter: parent.verticalCenter
                    iconColor: theme.textSecondary
                }
            }

            Row {
                id: filterRow
                anchors.right: parent.right
                anchors.top: root.compactToolbar ? searchField.bottom : parent.top
                anchors.topMargin: root.compactToolbar ? 6 : 0
                spacing: 6

                Repeater {
                    model: [
                        { label: "All", value: "ALL" },
                        { label: "Issues", value: "ISSUES" },
                        { label: "Warnings", value: "WARN" },
                        { label: "OK", value: "OK" }
                    ]
                    delegate: GlassButton {
                        required property var modelData
                        text: modelData.label
                        darkMode: root.darkMode
                        accent: root.severityFilter === modelData.value
                        onClicked: {
                            root.severityFilter = modelData.value
                            root.rebuildModel()
                        }
                    }
                }
            }
        }

        ListView {
            id: healthList
            width: parent.width
            height: parent.height - y
            clip: true
            model: checkModel
            spacing: 6
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

            delegate: Rectangle {
                id: row
                required property string category
                required property string title
                required property string detail
                required property string severity
                required property string fixHint
                required property string fixCommand

                width: healthList.width
                height: contentColumn.implicitHeight + 24
                radius: 12
                color: hoverHandler.hovered ? theme.hoverFill : "transparent"
                border.width: 1
                border.color: theme.divider

                HoverHandler { id: hoverHandler }

                Column {
                    id: contentColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.leftMargin: 14
                    anchors.rightMargin: 14
                    anchors.topMargin: 12
                    spacing: 5

                    Row {
                        width: parent.width
                        spacing: 9

                        Text {
                            text: category.toUpperCase()
                            color: theme.alpha(theme.textSecondary, 0.66)
                            font.family: "Inter"
                            font.pixelSize: 9
                            font.weight: Font.DemiBold
                            font.letterSpacing: 0.7
                            anchors.verticalCenter: parent.verticalCenter
                        }

                        Text {
                            width: Math.max(0, parent.width - severityBadge.width - 90)
                            text: title
                            color: theme.textPrimary
                            font.family: "Inter"
                            font.pixelSize: 13
                            font.weight: Font.Medium
                            elide: Text.ElideRight
                            anchors.verticalCenter: parent.verticalCenter
                        }

                        Rectangle {
                            id: severityBadge
                            width: severityText.implicitWidth + 16
                            height: 24
                            radius: 8
                            color: theme.alpha(root.severityColor(severity), 0.08)
                            border.width: 1
                            border.color: theme.alpha(root.severityColor(severity), 0.20)
                            anchors.verticalCenter: parent.verticalCenter

                            Text {
                                id: severityText
                                anchors.centerIn: parent
                                text: severity
                                color: root.severityColor(severity)
                                font.family: "Inter"
                                font.pixelSize: 9
                                font.weight: Font.DemiBold
                                font.letterSpacing: 0.5
                            }
                        }
                    }

                    Text {
                        width: parent.width
                        text: detail
                        color: theme.alpha(theme.textSecondary, 0.88)
                        font.family: "Inter"
                        font.pixelSize: 11
                        wrapMode: Text.Wrap
                    }

                    Text {
                        visible: fixHint.length > 0
                        width: parent.width
                        text: fixHint
                        color: theme.alpha(theme.textSecondary, 0.68)
                        font.family: "Inter"
                        font.pixelSize: 10
                        wrapMode: Text.Wrap
                    }

                    GlassPanel {
                        visible: fixCommand.length > 0
                        width: parent.width
                        height: 36
                        cornerRadius: 9
                        darkMode: root.darkMode
                        elevated: false

                        Text {
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.leftMargin: 11
                            anchors.rightMargin: 11
                            anchors.verticalCenter: parent.verticalCenter
                            text: fixCommand
                            color: theme.alpha(theme.textPrimary, 0.76)
                            font.family: "monospace"
                            font.pixelSize: 10
                            elide: Text.ElideRight
                        }
                    }
                }
            }

            Column {
                anchors.centerIn: parent
                visible: checkModel.count === 0
                spacing: 10

                UiIcon {
                    width: 28
                    height: 28
                    anchors.horizontalCenter: parent.horizontalCenter
                    name: "heart"
                    iconColor: theme.alpha(theme.textSecondary, 0.66)
                }
                Text {
                    text: "No diagnostics match this filter"
                    color: theme.textPrimary
                    font.family: "Inter"
                    font.pixelSize: 16
                    font.weight: Font.Medium
                    anchors.horizontalCenter: parent.horizontalCenter
                }
            }
        }
    }
}
