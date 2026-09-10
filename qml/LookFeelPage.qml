import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    required property var bridge
    property bool darkMode: true
    property bool realtimeEnabled: false
    readonly property bool compactToolbar: width < 760
    readonly property bool singleColumnCards: width < 860

    signal status(string message)
    signal saved()

    HyprbindTheme { id: theme; darkMode: root.darkMode }

    property real gapsIn: 5
    property real gapsOut: 20
    property real borderSize: 1
    property real rounding: 10
    property real activeOpacity: 1
    property real inactiveOpacity: 1
    property real blurSize: 3
    property bool blurEnabled: true
    property bool shadowEnabled: true
    property bool animationsEnabled: true
    property string activeBorder: ""
    property string inactiveBorder: ""
    property bool loaded: false

    function parse(raw) {
        try { return JSON.parse(raw) }
        catch (e) { return { ok: false, error: "Invalid Rust bridge response: " + e } }
    }

    function payload() {
        return JSON.stringify({
            gapsIn: Math.round(gapsIn),
            gapsOut: Math.round(gapsOut),
            borderSize: Math.round(borderSize),
            rounding: Math.round(rounding),
            activeOpacity: Math.round(activeOpacity * 100) / 100,
            inactiveOpacity: Math.round(inactiveOpacity * 100) / 100,
            blurSize: Math.round(blurSize),
            blurEnabled: blurEnabled,
            shadowEnabled: shadowEnabled,
            animationsEnabled: animationsEnabled,
            activeBorder: activeBorder,
            inactiveBorder: inactiveBorder
        })
    }

    function load() {
        const data = parse(bridge.lookFeelSnapshot())
        if (!data.ok) {
            status(data.error || "Could not load Look & Feel settings.")
            return
        }
        gapsIn = data.gapsIn
        gapsOut = data.gapsOut
        borderSize = data.borderSize
        rounding = data.rounding
        activeOpacity = data.activeOpacity
        inactiveOpacity = data.inactiveOpacity
        blurSize = data.blurSize
        blurEnabled = data.blurEnabled
        shadowEnabled = data.shadowEnabled
        animationsEnabled = data.animationsEnabled
        activeBorder = data.activeBorder || ""
        inactiveBorder = data.inactiveBorder || ""
        loaded = true
        status("Look & Feel values loaded from config.")
    }

    function preview() {
        if (!loaded || !realtimeEnabled)
            return
        previewTimer.restart()
    }

    function save() {
        const data = parse(bridge.saveLookFeel(payload()))
        if (!data.ok) {
            status(data.error || "Could not save Look & Feel settings.")
            return
        }
        status(data.message || "Look & Feel saved.")
        saved()
    }

    function applyPreset(name) {
        if (name === "compact") {
            gapsIn = 2; gapsOut = 4; borderSize = 1; rounding = 4
            activeOpacity = 1; inactiveOpacity = 1; blurSize = 2
            blurEnabled = false; shadowEnabled = false; animationsEnabled = true
        } else if (name === "comfortable") {
            gapsIn = 5; gapsOut = 12; borderSize = 2; rounding = 10
            activeOpacity = 1; inactiveOpacity = 0.95; blurSize = 5
            blurEnabled = true; shadowEnabled = true; animationsEnabled = true
        } else if (name === "spacious") {
            gapsIn = 12; gapsOut = 24; borderSize = 2; rounding = 16
            activeOpacity = 0.98; inactiveOpacity = 0.88; blurSize = 8
            blurEnabled = true; shadowEnabled = true; animationsEnabled = true
        }
        preview()
    }

    Component.onCompleted: load()

    Timer {
        id: previewTimer
        interval: 180
        repeat: false
        onTriggered: {
            const data = root.parse(root.bridge.previewLookFeel(root.payload()))
            if (!data.ok)
                root.status(data.error || "Live preview unavailable.")
        }
    }

    Component {
        id: spacingCardComponent
        SettingsCard {
            title: "SPACING & WINDOWS"
            darkMode: root.darkMode
            SettingSlider { width: parent.width; darkMode: root.darkMode; title: "Inner gaps"; subtitle: "Space between tiled windows"; from: 0; to: 40; value: root.gapsIn; onEdited: value => { root.gapsIn = value; root.preview() } }
            Rectangle { width: parent.width; height: 1; color: theme.divider }
            SettingSlider { width: parent.width; darkMode: root.darkMode; title: "Outer gaps"; subtitle: "Space between windows and screen edges"; from: 0; to: 60; value: root.gapsOut; onEdited: value => { root.gapsOut = value; root.preview() } }
            Rectangle { width: parent.width; height: 1; color: theme.divider }
            SettingSlider { width: parent.width; darkMode: root.darkMode; title: "Border size"; subtitle: "Thickness of window borders"; from: 0; to: 10; value: root.borderSize; onEdited: value => { root.borderSize = value; root.preview() } }
            Rectangle { width: parent.width; height: 1; color: theme.divider }
            SettingSlider { width: parent.width; darkMode: root.darkMode; title: "Corner rounding"; subtitle: "How round window corners appear"; from: 0; to: 40; value: root.rounding; onEdited: value => { root.rounding = value; root.preview() } }
        }
    }

    Component {
        id: transparencyCardComponent
        SettingsCard {
            title: "TRANSPARENCY"
            darkMode: root.darkMode
            SettingSlider { width: parent.width; darkMode: root.darkMode; title: "Active opacity"; subtitle: "Opacity of the focused window"; from: 0.3; to: 1; stepSize: 0.05; decimals: 2; value: root.activeOpacity; onEdited: value => { root.activeOpacity = value; root.preview() } }
            Rectangle { width: parent.width; height: 1; color: theme.divider }
            SettingSlider { width: parent.width; darkMode: root.darkMode; title: "Inactive opacity"; subtitle: "Opacity of unfocused windows"; from: 0.3; to: 1; stepSize: 0.05; decimals: 2; value: root.inactiveOpacity; onEdited: value => { root.inactiveOpacity = value; root.preview() } }
        }
    }

    Component {
        id: effectsCardComponent
        SettingsCard {
            title: "EFFECTS"
            darkMode: root.darkMode
            SettingToggle { width: parent.width; darkMode: root.darkMode; title: "Blur"; subtitle: "Blur behind translucent windows"; checked: root.blurEnabled; onToggled: checked => { root.blurEnabled = checked; root.preview() } }
            Rectangle { width: parent.width; height: 1; color: theme.divider }
            SettingSlider { width: parent.width; darkMode: root.darkMode; title: "Blur strength"; subtitle: "Blur kernel size"; from: 0; to: 20; value: root.blurSize; onEdited: value => { root.blurSize = value; root.preview() } }
            Rectangle { width: parent.width; height: 1; color: theme.divider }
            SettingToggle { width: parent.width; darkMode: root.darkMode; title: "Shadows"; subtitle: "Drop shadows under windows"; checked: root.shadowEnabled; onToggled: checked => { root.shadowEnabled = checked; root.preview() } }
            Rectangle { width: parent.width; height: 1; color: theme.divider }
            SettingToggle { width: parent.width; darkMode: root.darkMode; title: "Animations"; subtitle: "Window and workspace motion"; checked: root.animationsEnabled; onToggled: checked => { root.animationsEnabled = checked; root.preview() } }
        }
    }

    Component {
        id: borderCardComponent
        SettingsCard {
            title: "BORDER COLORS"
            darkMode: root.darkMode
            SettingField { width: parent.width; darkMode: root.darkMode; title: "Active border"; subtitle: "Focused window border color"; text: root.activeBorder; placeholderText: "rgba(37d5e9ee)"; onCommitted: text => { root.activeBorder = text; root.preview() } }
            Rectangle { width: parent.width; height: 1; color: theme.divider }
            SettingField { width: parent.width; darkMode: root.darkMode; title: "Inactive border"; subtitle: "Unfocused window border color"; text: root.inactiveBorder; placeholderText: "rgba(6b747baa)"; onCommitted: text => { root.inactiveBorder = text; root.preview() } }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 14

        Item {
            id: toolbar
            Layout.fillWidth: true
            Layout.preferredHeight: root.compactToolbar ? 78 : 40

            Row {
                id: presetsRow
                x: 0
                y: root.compactToolbar ? 0 : Math.round((toolbar.height - height) / 2)
                spacing: 8

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: "PRESETS"
                    color: theme.alpha(theme.textSecondary, 0.68)
                    font.family: "Inter"
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.9
                }
                GlassButton { text: "Compact"; darkMode: root.darkMode; onClicked: root.applyPreset("compact") }
                GlassButton { text: "Comfortable"; darkMode: root.darkMode; onClicked: root.applyPreset("comfortable") }
                GlassButton { text: "Spacious"; darkMode: root.darkMode; onClicked: root.applyPreset("spacious") }
            }

            Row {
                id: actionRow
                x: Math.max(0, toolbar.width - width)
                y: root.compactToolbar
                    ? Math.max(0, toolbar.height - height)
                    : Math.round((toolbar.height - height) / 2)
                spacing: 8

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.realtimeEnabled ? "Live preview on" : "Live preview off"
                    color: root.realtimeEnabled ? theme.alpha(theme.accent, 0.90) : theme.alpha(theme.textSecondary, 0.60)
                    font.family: "Inter"
                    font.pixelSize: 11
                }
                IconButton {
                    iconName: "reload"
                    tooltip: "Reload values"
                    darkMode: root.darkMode
                    onClicked: root.load()
                }
                IconButton {
                    iconName: "save"
                    tooltip: "Save changes"
                    accent: true
                    darkMode: root.darkMode
                    onClicked: root.save()
                }
            }
        }

        Flickable {
            id: scroller
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            contentWidth: width
            contentHeight: cardStage.height + 4
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

            Item {
                id: cardStage
                width: scroller.width
                readonly property bool twoColumns: !root.singleColumnCards
                height: twoColumns ? wideColumns.implicitHeight : narrowColumn.implicitHeight

                RowLayout {
                    id: wideColumns
                    width: parent.width
                    visible: cardStage.twoColumns
                    spacing: 14

                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.preferredWidth: cardStage.width / 2
                        Layout.alignment: Qt.AlignTop
                        spacing: 14

                        Loader {
                            Layout.fillWidth: true
                            Layout.preferredHeight: item ? item.implicitHeight : 0
                            active: cardStage.twoColumns
                            sourceComponent: spacingCardComponent
                            onLoaded: item.width = width
                            onWidthChanged: if (item) item.width = width
                        }
                        Loader {
                            Layout.fillWidth: true
                            Layout.preferredHeight: item ? item.implicitHeight : 0
                            active: cardStage.twoColumns
                            sourceComponent: effectsCardComponent
                            onLoaded: item.width = width
                            onWidthChanged: if (item) item.width = width
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.preferredWidth: cardStage.width / 2
                        Layout.alignment: Qt.AlignTop
                        spacing: 14

                        Loader {
                            Layout.fillWidth: true
                            Layout.preferredHeight: item ? item.implicitHeight : 0
                            active: cardStage.twoColumns
                            sourceComponent: transparencyCardComponent
                            onLoaded: item.width = width
                            onWidthChanged: if (item) item.width = width
                        }
                        Loader {
                            Layout.fillWidth: true
                            Layout.preferredHeight: item ? item.implicitHeight : 0
                            active: cardStage.twoColumns
                            sourceComponent: borderCardComponent
                            onLoaded: item.width = width
                            onWidthChanged: if (item) item.width = width
                        }
                    }
                }

                ColumnLayout {
                    id: narrowColumn
                    width: parent.width
                    visible: !cardStage.twoColumns
                    spacing: 14

                    Loader {
                        Layout.fillWidth: true
                        Layout.preferredHeight: item ? item.implicitHeight : 0
                        active: !cardStage.twoColumns
                        sourceComponent: spacingCardComponent
                        onLoaded: item.width = width
                        onWidthChanged: if (item) item.width = width
                    }
                    Loader {
                        Layout.fillWidth: true
                        Layout.preferredHeight: item ? item.implicitHeight : 0
                        active: !cardStage.twoColumns
                        sourceComponent: transparencyCardComponent
                        onLoaded: item.width = width
                        onWidthChanged: if (item) item.width = width
                    }
                    Loader {
                        Layout.fillWidth: true
                        Layout.preferredHeight: item ? item.implicitHeight : 0
                        active: !cardStage.twoColumns
                        sourceComponent: effectsCardComponent
                        onLoaded: item.width = width
                        onWidthChanged: if (item) item.width = width
                    }
                    Loader {
                        Layout.fillWidth: true
                        Layout.preferredHeight: item ? item.implicitHeight : 0
                        active: !cardStage.twoColumns
                        sourceComponent: borderCardComponent
                        onLoaded: item.width = width
                        onWidthChanged: if (item) item.width = width
                    }
                }
            }
        }
    }
}
