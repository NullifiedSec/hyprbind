import QtQuick

Canvas {
    id: icon
    property string name: "plus"
    property color iconColor: "#d7e0e6"
    property real strokeWidth: 1.65

    implicitWidth: 20
    implicitHeight: 20
    antialiasing: true

    onNameChanged: requestPaint()
    onIconColorChanged: requestPaint()
    onStrokeWidthChanged: requestPaint()
    onWidthChanged: requestPaint()
    onHeightChanged: requestPaint()

    onPaint: {
        const c = getContext("2d")
        c.reset()
        c.scale(width / 20, height / 20)
        c.strokeStyle = iconColor
        c.fillStyle = iconColor
        c.lineWidth = strokeWidth
        c.lineCap = "round"
        c.lineJoin = "round"
        c.beginPath()

        if (name === "plus") {
            c.moveTo(10, 4); c.lineTo(10, 16); c.moveTo(4, 10); c.lineTo(16, 10)
        } else if (name === "edit") {
            c.moveTo(5, 15); c.lineTo(6.3, 11.4); c.lineTo(13.4, 4.3); c.lineTo(15.8, 6.7); c.lineTo(8.7, 13.9); c.closePath();
            c.moveTo(12.2, 5.5); c.lineTo(14.7, 8)
        } else if (name === "trash") {
            c.moveTo(5, 6.2); c.lineTo(15, 6.2); c.moveTo(7, 6.2); c.lineTo(7.7, 16); c.lineTo(12.3, 16); c.lineTo(13, 6.2);
            c.moveTo(7.7, 4.2); c.lineTo(12.3, 4.2); c.moveTo(9, 4.2); c.lineTo(9.3, 3); c.lineTo(10.7, 3); c.lineTo(11, 4.2)
        } else if (name === "copy") {
            c.rect(6.5, 6.5, 9, 9); c.moveTo(5, 13); c.lineTo(4, 13); c.lineTo(4, 4); c.lineTo(13, 4); c.lineTo(13, 5)
        } else if (name === "check") {
            c.moveTo(4.2, 10.4); c.lineTo(8.3, 14.1); c.lineTo(15.9, 5.9)
        } else if (name === "search") {
            c.arc(8.6, 8.6, 4.7, 0, Math.PI * 2); c.moveTo(12.1, 12.1); c.lineTo(16.3, 16.3)
        } else if (name === "reload") {
            c.arc(10, 10, 6, -0.55, Math.PI * 1.42); c.moveTo(14.6, 3.8); c.lineTo(15.6, 7.1); c.lineTo(12.3, 6.4)
        } else if (name === "save") {
            c.moveTo(4, 3.5); c.lineTo(13.2, 3.5); c.lineTo(16, 6.3); c.lineTo(16, 16.5); c.lineTo(4, 16.5); c.closePath();
            c.rect(6.2, 4.8, 5.8, 4.1); c.rect(6.5, 11.2, 7, 4.2)
        } else if (name === "history") {
            c.arc(10.2, 10, 6, -2.45, Math.PI * 0.9); c.moveTo(5.6, 4.8); c.lineTo(4.2, 8); c.lineTo(7.6, 7.3); c.moveTo(10.2, 6.5); c.lineTo(10.2, 10.2); c.lineTo(12.8, 11.8)
        } else if (name === "transfer") {
            c.moveTo(3.5, 7); c.lineTo(15.5, 7); c.moveTo(12.5, 4); c.lineTo(15.5, 7); c.lineTo(12.5, 10);
            c.moveTo(16.5, 13); c.lineTo(4.5, 13); c.moveTo(7.5, 10); c.lineTo(4.5, 13); c.lineTo(7.5, 16)
        } else if (name === "terminal") {
            c.moveTo(4, 5); c.lineTo(8, 9); c.lineTo(4, 13); c.moveTo(10, 14); c.lineTo(16, 14)
        } else if (name === "monitor") {
            c.rect(3.2, 4.2, 13.6, 9.2); c.moveTo(8, 16); c.lineTo(12, 16); c.moveTo(10, 13.5); c.lineTo(10, 16)
        } else if (name === "info") {
            c.arc(10, 10, 7, 0, Math.PI * 2); c.moveTo(10, 9); c.lineTo(10, 13.5); c.stroke(); c.beginPath(); c.arc(10, 6.4, 0.7, 0, Math.PI * 2); c.fill(); return
        } else if (name === "close") {
            c.moveTo(6.2, 6.2); c.lineTo(13.8, 13.8); c.moveTo(13.8, 6.2); c.lineTo(6.2, 13.8)
        } else if (name === "minimize") {
            c.moveTo(5.6, 12.5); c.lineTo(14.4, 12.5)
        } else if (name === "maximize") {
            c.rect(5.5, 5.5, 9, 9)
        } else if (name === "restore-window") {
            c.moveTo(7.3, 5.3); c.lineTo(14.7, 5.3); c.lineTo(14.7, 12.7)
            c.rect(5.3, 7.3, 7.4, 7.4)
        } else if (name === "moon") {
            c.arc(10.5, 9.5, 6, 0.35, Math.PI * 1.78); c.quadraticCurveTo(8.5, 14.4, 7.2, 11.4); c.quadraticCurveTo(5.8, 7.8, 9.5, 3.8)
        } else if (name === "sun") {
            c.arc(10, 10, 3.5, 0, Math.PI * 2); for (let i = 0; i < 8; ++i) { const a = i * Math.PI / 4; c.moveTo(10 + Math.cos(a) * 5.5, 10 + Math.sin(a) * 5.5); c.lineTo(10 + Math.cos(a) * 7.4, 10 + Math.sin(a) * 7.4) }
        } else if (name === "realtime") {
            c.rect(4, 4.5, 12, 9.5); c.moveTo(8, 16); c.lineTo(12, 16); c.moveTo(10, 14); c.lineTo(10, 16)
        } else if (name === "home") {
            c.moveTo(3.5, 9); c.lineTo(10, 3.8); c.lineTo(16.5, 9); c.moveTo(5.3, 8); c.lineTo(5.3, 16); c.lineTo(14.7, 16); c.lineTo(14.7, 8)
        } else if (name === "keyboard") {
            c.rect(3, 5, 14, 10); c.moveTo(5, 8); c.lineTo(5.1, 8); c.moveTo(8, 8); c.lineTo(8.1, 8); c.moveTo(11, 8); c.lineTo(11.1, 8); c.moveTo(14, 8); c.lineTo(14.1, 8); c.moveTo(6, 11); c.lineTo(14, 11)
        } else if (name === "braces") {
            c.moveTo(7.5, 3.5); c.quadraticCurveTo(5.5, 3.5, 5.5, 6); c.lineTo(5.5, 8); c.quadraticCurveTo(5.5, 10, 3.8, 10); c.quadraticCurveTo(5.5, 10, 5.5, 12); c.lineTo(5.5, 14); c.quadraticCurveTo(5.5, 16.5, 7.5, 16.5); c.moveTo(12.5, 3.5); c.quadraticCurveTo(14.5, 3.5, 14.5, 6); c.lineTo(14.5, 8); c.quadraticCurveTo(14.5, 10, 16.2, 10); c.quadraticCurveTo(14.5, 10, 14.5, 12); c.lineTo(14.5, 14); c.quadraticCurveTo(14.5, 16.5, 12.5, 16.5)
        } else if (name === "document") {
            c.moveTo(5, 3.5); c.lineTo(12, 3.5); c.lineTo(15, 6.5); c.lineTo(15, 16.5); c.lineTo(5, 16.5); c.closePath(); c.moveTo(12, 3.5); c.lineTo(12, 6.5); c.lineTo(15, 6.5); c.moveTo(7.5, 10); c.lineTo(12.5, 10); c.moveTo(7.5, 13); c.lineTo(11.5, 13)
        } else if (name === "grid") {
            c.rect(3.5, 3.5, 5, 5); c.rect(11.5, 3.5, 5, 5); c.rect(3.5, 11.5, 5, 5); c.rect(11.5, 11.5, 5, 5)
        } else if (name === "power") {
            c.moveTo(10, 2.8); c.lineTo(10, 9); c.moveTo(6.3, 5); c.arc(10, 10, 6, -2.25, 0.9)
        } else if (name === "window") {
            c.rect(3.5, 4, 13, 12); c.moveTo(3.5, 7); c.lineTo(16.5, 7)
        } else if (name === "layers") {
            c.moveTo(10, 3.5); c.lineTo(17, 7); c.lineTo(10, 10.5); c.lineTo(3, 7); c.closePath(); c.moveTo(4.2, 10); c.lineTo(10, 13); c.lineTo(15.8, 10); c.moveTo(4.2, 13); c.lineTo(10, 16); c.lineTo(15.8, 13)
        } else if (name === "diamond") {
            c.moveTo(10, 3); c.lineTo(17, 10); c.lineTo(10, 17); c.lineTo(3, 10); c.closePath(); c.moveTo(7.5, 10); c.lineTo(10, 7.5); c.lineTo(12.5, 10); c.lineTo(10, 12.5); c.closePath()
        } else if (name === "settings") {
            c.arc(10, 10, 3, 0, Math.PI * 2); for (let i = 0; i < 8; ++i) { const a = i * Math.PI / 4; c.moveTo(10 + Math.cos(a) * 5, 10 + Math.sin(a) * 5); c.lineTo(10 + Math.cos(a) * 7, 10 + Math.sin(a) * 7) }
        } else if (name === "device") {
            c.arc(10, 10, 6.5, 0, Math.PI * 2); c.arc(10, 10, 2.2, 0, Math.PI * 2)
        } else if (name === "sparkle") {
            c.moveTo(10, 2.8); c.quadraticCurveTo(11.2, 8.8, 17.2, 10); c.quadraticCurveTo(11.2, 11.2, 10, 17.2); c.quadraticCurveTo(8.8, 11.2, 2.8, 10); c.quadraticCurveTo(8.8, 8.8, 10, 2.8)
        } else if (name === "curve") {
            c.moveTo(3, 13); c.bezierCurveTo(6, 4, 9, 16, 17, 6)
        } else if (name === "gesture") {
            c.moveTo(4, 13); c.bezierCurveTo(6, 10, 8, 10, 10, 13); c.bezierCurveTo(12, 16, 14, 16, 16, 12); c.moveTo(5, 7); c.lineTo(8, 4); c.moveTo(5, 7); c.lineTo(8, 8)
        } else if (name === "heart") {
            c.moveTo(10, 16); c.lineTo(4.4, 10.5); c.bezierCurveTo(0.8, 6.7, 5.5, 2.3, 10, 6); c.bezierCurveTo(14.5, 2.3, 19.2, 6.7, 15.6, 10.5); c.closePath()
        } else if (name === "logs") {
            c.moveTo(4, 5); c.lineTo(16, 5); c.moveTo(4, 10); c.lineTo(16, 10); c.moveTo(4, 15); c.lineTo(13, 15)
        } else {
            c.arc(10, 10, 2, 0, Math.PI * 2)
        }
        c.stroke()
    }
}
