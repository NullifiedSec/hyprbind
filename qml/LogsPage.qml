import QtQuick
import QtQuick.Controls
import dev.hyprbinds.ui

Item {
    id: root

    property bool darkMode: true
    property var allEntries: []
    property string severityFilter: "ALL"
    property string loadError: ""
    readonly property bool compactToolbar: width < 820

    signal status(string message)
    signal healthChanged(bool healthy)

    HyprbindTheme { id: theme; darkMode: root.darkMode }
    LogsBridge { id: logsBridge }
    ListModel { id: logModel }

    function parse(raw) {
        try { return JSON.parse(raw) }
        catch (error) { return { ok: false, error: "Invalid logs response: " + error } }
    }

    function load() {
        const payload = parse(logsBridge.snapshot())
        loadError = payload.ok ? "" : (payload.error || "Could not read the user journal.")
        allEntries = payload.entries || []
        healthChanged(payload.ok === true)
        status(payload.ok
            ? "Loaded " + allEntries.length + " recent journal entries."
            : loadError)
        rebuildModel()
    }

    function severityMatches(severity) {
        if (severityFilter === "ALL") return true
        if (severityFilter === "ERRORS")
            return severity === "EMERG" || severity === "ALERT" || severity === "CRIT" || severity === "ERROR"
        if (severityFilter === "WARN") return severity === "WARN"
        if (severityFilter === "INFO") return severity === "NOTE" || severity === "INFO"
        if (severityFilter === "DEBUG") return severity === "DEBUG"
        return true
    }

    function rebuildModel() {
        const query = searchField.text.trim().toLowerCase()
        logModel.clear()
        for (let i = 0; i < allEntries.length; ++i) {
            const item = allEntries[i]
            const haystack = (item.message + " " + item.identifier + " " + item.unit + " " + item.severity).toLowerCase()
            if (severityMatches(item.severity) && (!query || haystack.indexOf(query) >= 0)) {
                logModel.append({
                    entryId: item.id,
                    timestamp: item.timestamp,
                    severity: item.severity,
                    message: item.message,
                    identifier: item.identifier,
                    unit: item.unit
                })
            }
        }
    }

    function severityColor(severity) {
        if (severity === "EMERG" || severity === "ALERT" || severity === "CRIT" || severity === "ERROR")
            return root.darkMode ? "#e39b96" : "#a84843"
        if (severity === "WARN")
            return root.darkMode ? "#e6bd79" : "#98651d"
        if (severity === "DEBUG")
            return theme.alpha(theme.textSecondary, 0.62)
        return theme.accent
    }

    Component.onCompleted: load()

    Item {
        id: toolbar
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: root.compactToolbar ? 92 : 44

        GlassField {
            id: searchField
            anchors.left: parent.left
            anchors.right: root.compactToolbar ? parent.right : filterFlow.left
            anchors.rightMargin: root.compactToolbar ? 0 : 10
            anchors.top: parent.top
            implicitHeight: 42
            darkMode: root.darkMode
            placeholderText: "Search recent logs..."
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

        Flow {
            id: filterFlow
            width: root.compactToolbar ? parent.width : implicitWidth
            anchors.right: parent.right
            anchors.top: root.compactToolbar ? searchField.bottom : parent.top
            anchors.topMargin: root.compactToolbar ? 6 : 0
            spacing: 6

            Repeater {
                model: [
                    { label: "All", value: "ALL" },
                    { label: "Errors", value: "ERRORS" },
                    { label: "Warnings", value: "WARN" },
                    { label: "Info", value: "INFO" },
                    { label: "Debug", value: "DEBUG" }
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
        id: logList
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: toolbar.bottom
        anchors.bottom: parent.bottom
        anchors.topMargin: 7
        clip: true
        model: logModel
        spacing: 3
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

        delegate: Rectangle {
            id: row
            required property int entryId
            required property string timestamp
            required property string severity
            required property string message
            required property string identifier
            required property string unit

            width: logList.width
            height: root.width < 620 ? 86 : 72
            radius: 10
            color: hoverHandler.hovered ? theme.hoverFill : "transparent"

            HoverHandler { id: hoverHandler }

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                anchors.leftMargin: 10
                anchors.rightMargin: 10
                height: 1
                color: theme.divider
            }

            Rectangle {
                id: severityBadge
                width: severityText.implicitWidth + 16
                height: 23
                radius: 8
                anchors.left: parent.left
                anchors.leftMargin: 13
                anchors.top: parent.top
                anchors.topMargin: 11
                color: theme.alpha(root.severityColor(severity), 0.08)
                border.width: 1
                border.color: theme.alpha(root.severityColor(severity), 0.18)

                Text {
                    id: severityText
                    anchors.centerIn: parent
                    text: severity
                    color: root.severityColor(severity)
                    font.family: "Inter"
                    font.pixelSize: 9
                    font.weight: Font.DemiBold
                }
            }

            Text {
                id: timestampText
                anchors.right: parent.right
                anchors.rightMargin: 13
                anchors.top: parent.top
                anchors.topMargin: 13
                text: timestamp
                color: theme.alpha(theme.textSecondary, 0.52)
                font.family: "monospace"
                font.pixelSize: 9
            }

            Text {
                id: sourceText
                anchors.left: severityBadge.right
                anchors.leftMargin: 9
                anchors.right: timestampText.left
                anchors.rightMargin: 10
                anchors.verticalCenter: severityBadge.verticalCenter
                text: identifier.length ? identifier : (unit.length ? unit : "journal")
                color: theme.alpha(theme.textPrimary, 0.78)
                font.family: "Inter"
                font.pixelSize: 11
                font.weight: Font.Medium
                elide: Text.ElideRight
            }

            Text {
                anchors.left: parent.left
                anchors.leftMargin: 13
                anchors.right: parent.right
                anchors.rightMargin: 13
                anchors.top: severityBadge.bottom
                anchors.topMargin: 7
                text: message
                color: theme.alpha(theme.textSecondary, 0.88)
                font.family: "Inter"
                font.pixelSize: 11
                elide: Text.ElideRight
                maximumLineCount: root.width < 620 ? 2 : 1
                wrapMode: root.width < 620 ? Text.WordWrap : Text.NoWrap
            }
        }

        Column {
            anchors.centerIn: parent
            width: Math.min(parent.width - 36, 460)
            spacing: 10
            visible: logModel.count === 0

            UiIcon {
                width: 28
                height: 28
                anchors.horizontalCenter: parent.horizontalCenter
                name: "logs"
                iconColor: theme.alpha(theme.textSecondary, 0.66)
            }
            Text {
                text: root.loadError.length ? "Logs are unavailable" : "No logs match this filter"
                color: theme.textPrimary
                font.family: "Inter"
                font.pixelSize: 16
                font.weight: Font.Medium
                anchors.horizontalCenter: parent.horizontalCenter
            }
            Text {
                visible: root.loadError.length > 0
                width: parent.width
                text: root.loadError
                color: theme.textSecondary
                font.family: "Inter"
                font.pixelSize: 11
                wrapMode: Text.WordWrap
                horizontalAlignment: Text.AlignHCenter
            }
        }
    }
}
