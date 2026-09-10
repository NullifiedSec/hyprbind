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

    readonly property color background: darkMode ? "#101416" : "#e9eef0"
    readonly property color surface: darkMode ? "#1a2023" : "#f4f7f8"
    readonly property color surfaceHigh: darkMode ? "#252d31" : "#ffffff"
    readonly property color foreground: darkMode ? "#eef2f3" : "#172126"
    readonly property color muted: darkMode ? "#bec6ca" : "#526068"
    readonly property color accent: "#37d5e9"

    readonly property color textPrimary: foreground
    readonly property color textSecondary: alpha(muted, 0.78)
    readonly property color familyShell: mix(surfaceHigh, background, darkMode ? 0.54 : 0.30)
    readonly property color shellFill: alpha(familyShell, darkMode ? 0.62 : 0.74)
    readonly property color shellRim: alpha(foreground, darkMode ? 0.095 : 0.13)
    readonly property color shellTopSpecular: alpha(foreground, darkMode ? 0.034 : 0.11)
    readonly property color shellAccentWash: alpha(accent, 0.018)
    readonly property color shellBottomShade: alpha(accent, darkMode ? 0.028 : 0.018)
    readonly property color shellInnerLine: alpha(foreground, darkMode ? 0.052 : 0.09)
    readonly property color toolbarFill: alpha(mix(surfaceHigh, background, 0.44), darkMode ? 0.30 : 0.40)
    readonly property color sidebarFill: alpha(mix(surfaceHigh, background, 0.55), darkMode ? 0.42 : 0.50)
    readonly property color contentFill: alpha(mix(surface, background, 0.62), darkMode ? 0.18 : 0.28)
    readonly property color raisedFill: alpha(mix(surfaceHigh, background, 0.58), darkMode ? 0.40 : 0.52)
    readonly property color quietRim: alpha(foreground, darkMode ? 0.065 : 0.11)
    readonly property color divider: alpha(foreground, darkMode ? 0.036 : 0.075)

    readonly property color controlFill: alpha(mix(surfaceHigh, background, 0.60), darkMode ? 0.42 : 0.50)
    readonly property color controlHover: alpha(mix(surfaceHigh, accent, 0.08), darkMode ? 0.50 : 0.56)
    readonly property color controlPressed: alpha(mix(surfaceHigh, accent, 0.12), darkMode ? 0.56 : 0.62)
    readonly property color controlRim: alpha(foreground, darkMode ? 0.055 : 0.10)
    readonly property color controlRimActive: alpha(accent, darkMode ? 0.14 : 0.22)
    readonly property color controlInnerRim: alpha(foreground, darkMode ? 0.025 : 0.055)

    readonly property color searchFill: alpha(mix(surfaceHigh, background, 0.62), darkMode ? 0.42 : 0.52)
    readonly property color searchFocusedFill: alpha(mix(surfaceHigh, accent, 0.055), darkMode ? 0.50 : 0.58)
    readonly property color searchRim: alpha(foreground, darkMode ? 0.055 : 0.10)
    readonly property color searchFocusRim: alpha(accent, darkMode ? 0.14 : 0.24)
    readonly property color searchSpecular: alpha(foreground, darkMode ? 0.026 : 0.075)
    readonly property color searchSpecularFocus: alpha(mix(foreground, accent, 0.10), darkMode ? 0.040 : 0.10)

    readonly property color hoverFill: alpha(foreground, darkMode ? 0.028 : 0.060)
    readonly property color selectedFill: alpha(mix(surfaceHigh, accent, 0.10), darkMode ? 0.54 : 0.60)
    readonly property color selectedRim: alpha(accent, darkMode ? 0.10 : 0.18)
    readonly property color selectedSpecular: alpha(foreground, darkMode ? 0.028 : 0.070)
    readonly property color danger: darkMode ? "#dca3a0" : "#9c3f3b"
    readonly property color dangerFill: alpha(danger, darkMode ? 0.055 : 0.075)
    readonly property color dangerRim: alpha(danger, darkMode ? 0.14 : 0.20)
}
