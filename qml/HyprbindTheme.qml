import QtQuick

QtObject {
    id: theme
    property bool darkMode: true

    function alpha(c, a) { return Qt.rgba(c.r, c.g, c.b, a) }
    function mix(a, b, amount) {
        const t = Math.max(0, Math.min(1, amount))
        return Qt.rgba(
            a.r * (1 - t) + b.r * t,
            a.g * (1 - t) + b.g * t,
            a.b * (1 - t) + b.b * t,
            a.a * (1 - t) + b.a * t
        )
    }

    function pageIcon(page) {
        if (page === "Overview") return "home"
        if (page === "Binds") return "keyboard"
        if (page === "Variables") return "braces"
        if (page === "Environment") return "document"
        if (page === "Submaps") return "grid"
        if (page === "Startup") return "power"
        if (page === "Window rules") return "window"
        if (page === "Workspace rules") return "grid"
        if (page === "Layer rules") return "layers"
        if (page === "Look & Feel") return "diamond"
        if (page === "Config") return "settings"
        if (page === "Monitors") return "monitor"
        if (page === "Devices") return "device"
        if (page === "Animations") return "sparkle"
        if (page === "Curves") return "curve"
        if (page === "Gestures") return "gesture"
        if (page === "Health") return "heart"
        if (page === "Logs") return "logs"
        return "document"
    }

    readonly property color background: darkMode ? "#101416" : "#eef3f5"
    readonly property color surface: darkMode ? "#1a2023" : "#f8fafb"
    readonly property color surfaceHigh: darkMode ? "#252d31" : "#ffffff"
    readonly property color foreground: darkMode ? "#eef2f3" : "#182328"
    readonly property color muted: darkMode ? "#bec6ca" : "#5b6a72"
    readonly property color accent: darkMode ? "#37d5e9" : "#168fa3"

    readonly property color textPrimary: foreground
    readonly property color textSecondary: alpha(muted, darkMode ? 0.78 : 0.90)

    readonly property color familyShell: darkMode
        ? mix(surfaceHigh, background, 0.54)
        : mix(surfaceHigh, background, 0.12)
    readonly property color shellFill: alpha(familyShell, darkMode ? 0.62 : 0.985)
    readonly property color shellRim: alpha(foreground, darkMode ? 0.095 : 0.12)
    readonly property color shellTopSpecular: darkMode
        ? alpha(foreground, 0.034)
        : alpha(surfaceHigh, 0.70)
    readonly property color shellAccentWash: darkMode ? alpha(accent, 0.018) : "transparent"
    readonly property color shellBottomShade: darkMode ? alpha(accent, 0.028) : "transparent"
    readonly property color shellInnerLine: darkMode
        ? alpha(foreground, 0.052)
        : alpha(surfaceHigh, 0.76)

    readonly property color toolbarFill: darkMode
        ? alpha(mix(surfaceHigh, background, 0.44), 0.30)
        : alpha(mix(surfaceHigh, background, 0.08), 0.96)
    readonly property color sidebarFill: darkMode
        ? alpha(mix(surfaceHigh, background, 0.55), 0.42)
        : alpha(mix(surfaceHigh, background, 0.14), 0.97)
    readonly property color contentFill: darkMode
        ? alpha(mix(surface, background, 0.62), 0.18)
        : alpha(mix(surface, background, 0.10), 0.95)
    readonly property color raisedFill: darkMode
        ? alpha(mix(surfaceHigh, background, 0.58), 0.40)
        : alpha(mix(surfaceHigh, background, 0.06), 0.985)
    readonly property color quietRim: alpha(foreground, darkMode ? 0.065 : 0.095)
    readonly property color divider: alpha(foreground, darkMode ? 0.036 : 0.065)

    readonly property color controlFill: darkMode
        ? alpha(mix(surfaceHigh, background, 0.60), 0.42)
        : alpha(mix(surfaceHigh, background, 0.06), 0.94)
    readonly property color controlHover: darkMode
        ? alpha(mix(surfaceHigh, accent, 0.08), 0.50)
        : alpha(mix(surfaceHigh, accent, 0.045), 0.98)
    readonly property color controlPressed: darkMode
        ? alpha(mix(surfaceHigh, accent, 0.12), 0.56)
        : alpha(mix(surfaceHigh, accent, 0.065), 1.0)
    readonly property color controlRim: alpha(foreground, darkMode ? 0.055 : 0.085)
    readonly property color controlRimActive: alpha(accent, darkMode ? 0.14 : 0.24)
    readonly property color controlInnerRim: alpha(foreground, darkMode ? 0.025 : 0.038)

    readonly property color searchFill: darkMode
        ? alpha(mix(surfaceHigh, background, 0.62), 0.42)
        : alpha(mix(surfaceHigh, background, 0.04), 0.96)
    readonly property color searchFocusedFill: searchFill
    readonly property color searchRim: alpha(foreground, darkMode ? 0.055 : 0.09)
    readonly property color searchFocusRim: alpha(foreground, darkMode ? 0.12 : 0.15)
    readonly property color searchSpecular: darkMode
        ? alpha(foreground, 0.026)
        : alpha(surfaceHigh, 0.66)
    readonly property color searchSpecularFocus: searchSpecular

    readonly property color hoverFill: darkMode
        ? alpha(foreground, 0.028)
        : alpha(foreground, 0.048)
    readonly property color selectedFill: darkMode
        ? alpha(mix(surfaceHigh, accent, 0.10), 0.54)
        : alpha(mix(surfaceHigh, accent, 0.060), 0.96)
    readonly property color selectedRim: alpha(accent, darkMode ? 0.10 : 0.16)
    readonly property color selectedSpecular: darkMode
        ? alpha(foreground, 0.028)
        : alpha(surfaceHigh, 0.56)

    readonly property color danger: darkMode ? "#dca3a0" : "#9c3f3b"
    readonly property color dangerFill: alpha(danger, darkMode ? 0.055 : 0.065)
    readonly property color dangerRim: alpha(danger, darkMode ? 0.14 : 0.18)
}
