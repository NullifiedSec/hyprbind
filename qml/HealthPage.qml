import QtQuick
import QtQuick.Controls
import dev.hyprbinds.ui

Item {
    id: root

    property bool darkMode: true
    property var allChecks: []
    property string severityFilter: "ALL"
    property int total: 0
    property int okCount: 0
    property int warnings: 0
    property int failures: 0

    readonly property bool compactToolbar: width < 760
    readonly property int summaryColumns: width >= 980 ? 3 : width >= 620 ? 2 : 1
    readonly property real summaryGap: 10
    readonly property real summaryCardHeight: 126
    readonly property real summaryCardWidth: summaryColumns === 3
        ? (width - summaryGap * 2) / 3
        : summaryColumns === 2
            ? (width - summaryGap) / 2
            : width

    signal status(string message)
    signal healthChanged(bool healthy)

    HyprbindTheme { id: theme; darkMode: root.darkMode }
    HealthBridge { id: healthBridge }
    ListModel { id: checkModel }

    TextInput {
        id: clipboardBuffer
        width: 1
        height: 1
        opacity: 0
        readOnly: true
    }

    function parse(raw) {
        try { return JSON.parse(raw) }
        catch (error) { return { ok: false, error: "Invalid diagnostics response: " + error } }
    }

    function load() {
        const payload = parse(healthBridge.snapshot())
        if (!payload.ok) {
            allChecks = []
            total = 0
            okCount = 0
            warnings = 0
            failures = 1
            healthChanged(false)
            status(payload.error || "Could not run health diagnostics.")
            rebuildModel()
            return
        }

        allChecks = payload.checks || []
        total = payload.total || allChecks.length
        okCount = payload.okCount || 0
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

    function formatReport() {
        let out = "Hyprbinds health report\n" + okCount + " ok · " + warnings + " warnings · " + failures + " failures\n\n"
        let lastCategory = ""
        for (let i = 0; i < allChecks.length; ++i) {
            const item = allChecks[i]
            if (item.category !== lastCategory) {
                lastCategory = item.category
                out += "## " + item.category + "\n"
            }
            out += "[" + item.severity + "] " + item.title + "\n  " + item.detail + "\n"
            if (item.fixHint) out += "  hint: " + item.fixHint + "\n"
            if (item.fixCommand) out += "  fix: " + item.fixCommand + "\n"
            out += "\n"
        }
        return out
    }

    function copyText(text, message) {
        clipboardBuffer.text = text
        clipboardBuffer.forceActiveFocus()
        clipboardBuffer.selectAll()
        clipboardBuffer.copy()
        clipboardBuffer.deselect()
        status(message)
    }

    Component.onCompleted: load()

    Column {
        id: topArea
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        spacing: 12

        Grid {
            id: summaryGrid
            width: parent.width
            columns: root.summaryColumns
            spacing: root.summaryGap
            height: Math.ceil(3 / root.summaryColumns) * root.summaryCardHeight
                + (Math.ceil(3 / root.summaryColumns) - 1) * root.summaryGap

            SummaryCard {
                width: root.summaryCardWidth
                height: root.summaryCardHeight
                darkMode: root.darkMode
                iconName: "heart"
                title: "CHECKS"
                value: root.total.toString()
                subtitle: "Desktop and session diagnostics"
                accent: root.failures === 0
            }

            SummaryCard {
                width: root.summaryCardWidth
                height: root.summaryCardHeight
                darkMode: root.darkMode
                iconName: "info"
                title: "WARNINGS"
                value: root.warnings.toString()
                subtitle: root.warnings === 1 ? "One item needs attention" : "Items worth reviewing"
            }

            SummaryCard {
                width: root.summaryCardWidth
                height: root.summaryCardHeight
                darkMode: root.darkMode
                iconName: "heart"
                title: "FAILURES"
                value: root.failures.toString()
                subtitle: root.failures === 0 ? "No blocking checks" : "Checks that are currently failing"
                accent: root.failures === 0
            }
        }

        Item {
            id: reportActions
            width: parent.width
            height: 38

            Row {
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                spacing: 7

                GlassButton {
                    text: "Copy report"
                    darkMode: root.darkMode
                    enabled: root.allChecks.length > 0
                    onClicked: root.copyText(root.formatReport(), "Copied full health report to clipboard.")
                }

                GlassButton {
                    text: "Copy system info"
                    darkMode: root.darkMode
                    onClicked: {
                        const report = String(healthBridge.systemInfo())
                        root.copyText(report, "Copied system info to clipboard.")
                    }
                }
            }
        }

        Item {
            id: toolbar
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
    }

    ListView {
        id: healthList
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: topArea.bottom
        anchors.bottom: parent.bottom
        anchors.topMargin: 8
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

            width: ListView.view.width
            height: rowContent.implicitHeight + 24
            radius: 12
            color: hoverHandler.hovered ? theme.hoverFill : "transparent"
            border.width: 1
            border.color: theme.divider

            HoverHandler { id: hoverHandler }

            Column {
                id: rowContent
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.leftMargin: 14
                anchors.rightMargin: 14
                anchors.topMargin: 12
                spacing: 7

                Item {
                    width: parent.width
                    height: 40

                    Column {
                        anchors.left: parent.left
                        anchors.right: severityBadge.left
                        anchors.rightMargin: 12
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 3

                        Text {
                            width: parent.width
                            text: category.toUpperCase()
                            color: theme.alpha(theme.textSecondary, 0.64)
                            font.family: "Inter"
                            font.pixelSize: 9
                            font.weight: Font.DemiBold
                            font.letterSpacing: 0.7
                            elide: Text.ElideRight
                        }

                        Text {
                            width: parent.width
                            text: title
                            color: theme.textPrimary
                            font.family: "Inter"
                            font.pixelSize: 13
                            font.weight: Font.Medium
                            elide: Text.ElideRight
                        }
                    }

                    Rectangle {
                        id: severityBadge
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        width: severityText.implicitWidth + 18
                        height: 25
                        radius: 8
                        color: theme.alpha(root.severityColor(severity), 0.08)
                        border.width: 1
                        border.color: theme.alpha(root.severityColor(severity), 0.20)

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

                Rectangle {
                    width: parent.width
                    height: 1
                    color: theme.divider
                    opacity: 0.72
                }

                Text {
                    width: parent.width
                    text: detail
                    color: theme.alpha(theme.textSecondary, 0.88)
                    font.family: "Inter"
                    font.pixelSize: 11
                    lineHeight: 1.2
                    wrapMode: Text.WordWrap
                }

                Text {
                    visible: fixHint.length > 0
                    width: parent.width
                    text: fixHint
                    color: theme.alpha(theme.textSecondary, 0.68)
                    font.family: "Inter"
                    font.pixelSize: 10
                    lineHeight: 1.2
                    wrapMode: Text.WordWrap
                }

                GlassPanel {
                    visible: fixCommand.length > 0
                    width: parent.width
                    height: visible ? 42 : 0
                    cornerRadius: 9
                    darkMode: root.darkMode
                    elevated: false

                    GlassButton {
                        id: copyFixButton
                        anchors.right: parent.right
                        anchors.rightMargin: 5
                        anchors.verticalCenter: parent.verticalCenter
                        text: "Copy fix"
                        darkMode: root.darkMode
                        implicitHeight: 32
                        onClicked: root.copyText(fixCommand, "Copied fix command to clipboard.")
                    }

                    Text {
                        anchors.left: parent.left
                        anchors.right: copyFixButton.left
                        anchors.leftMargin: 12
                        anchors.rightMargin: 8
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
            width: Math.min(parent.width - 36, 420)
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
                width: parent.width
                text: "No diagnostics match this filter"
                color: theme.textPrimary
                font.family: "Inter"
                font.pixelSize: 16
                font.weight: Font.Medium
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
            }
        }
    }
}
