import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.tidalkde

QQC2.ToolBar {
    id: bar
    position: QQC2.ToolBar.Footer

    // Fade the bar's background along with the window (see Main.qml).
    Binding {
        target: bar.background
        property: "opacity"
        value: Backend.windowOpacity
        when: Backend.windowOpacity < 1 && target !== null
    }

    signal lyricsRequested()
    signal queueRequested()

    function time(seconds) {
        const s = Math.max(0, Math.floor(seconds))
        return Math.floor(s / 60) + ":" + String(s % 60).padStart(2, "0")
    }

    contentItem: ColumnLayout {
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing
            QQC2.Label {
                text: bar.time(Backend.position)
                font: Kirigami.Theme.smallFont
                opacity: 0.7
                Layout.preferredWidth: Kirigami.Units.gridUnit * 2.5
                horizontalAlignment: Text.AlignRight
            }
            QQC2.Slider {
                id: progress
                Layout.fillWidth: true
                from: 0
                to: Math.max(Backend.duration, 1)
                enabled: Backend.active
                onMoved: Backend.seek(value)
                Binding on value {
                    value: Backend.position
                    when: !progress.pressed
                    restoreMode: Binding.RestoreNone
                }
            }
            QQC2.Label {
                text: bar.time(Backend.duration)
                font: Kirigami.Theme.smallFont
                opacity: 0.7
                Layout.preferredWidth: Kirigami.Units.gridUnit * 2.5
            }
        }

        Item {
            Layout.fillWidth: true
            implicitHeight: Math.max(info.implicitHeight, transport.implicitHeight, extras.implicitHeight)

            // Now playing (click for lyrics)
            RowLayout {
                id: info
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                width: Math.min(implicitWidth, parent.width / 2 - transport.width / 2 - Kirigami.Units.largeSpacing)
                spacing: Kirigami.Units.largeSpacing

                CoverImage {
                    size: Kirigami.Units.gridUnit * 3
                    source: Backend.cover
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0
                    QQC2.Label {
                        Layout.fillWidth: true
                        text: Backend.title !== "" ? Backend.title : "Nothing playing"
                        font.bold: true
                        elide: Text.ElideRight
                    }
                    QQC2.Label {
                        Layout.fillWidth: true
                        text: Backend.artist
                        elide: Text.ElideRight
                        opacity: 0.7
                        font: Kirigami.Theme.smallFont
                    }
                }
                HoverHandler { cursorShape: Backend.title !== "" ? Qt.PointingHandCursor : Qt.ArrowCursor }
                TapHandler { onTapped: if (Backend.title !== "") bar.lyricsRequested() }
            }

            // Transport, centred
            RowLayout {
                id: transport
                anchors.centerIn: parent
                spacing: Kirigami.Units.smallSpacing

                QQC2.ToolButton {
                    icon.name: "media-playlist-shuffle"
                    display: QQC2.AbstractButton.IconOnly
                    text: "Shuffle upcoming tracks"
                    QQC2.ToolTip.text: text
                    QQC2.ToolTip.visible: hovered
                    onClicked: Backend.shuffleUpcoming()
                }
                QQC2.ToolButton {
                    icon.name: "media-skip-backward"
                    display: QQC2.AbstractButton.IconOnly
                    text: "Previous"
                    onClicked: Backend.previous()
                }
                QQC2.RoundButton {
                    icon.name: Backend.playing ? "media-playback-pause" : "media-playback-start"
                    icon.width: Kirigami.Units.iconSizes.medium
                    icon.height: Kirigami.Units.iconSizes.medium
                    highlighted: true
                    text: Backend.playing ? "Pause" : "Play"
                    display: QQC2.AbstractButton.IconOnly
                    QQC2.ToolTip.text: text
                    QQC2.ToolTip.visible: hovered
                    onClicked: Backend.togglePause()
                }
                QQC2.ToolButton {
                    icon.name: "media-skip-forward"
                    display: QQC2.AbstractButton.IconOnly
                    text: "Next"
                    onClicked: Backend.next()
                }
                QQC2.ToolButton {
                    icon.name: Backend.repeatMode === 2 ? "media-repeat-single" : "media-repeat-all"
                    display: QQC2.AbstractButton.IconOnly
                    checkable: true
                    checked: Backend.repeatMode !== 0
                    text: ["Repeat: off", "Repeat: queue", "Repeat: this track"][Backend.repeatMode]
                    QQC2.ToolTip.text: text
                    QQC2.ToolTip.visible: hovered
                    onClicked: Backend.cycleRepeat()
                }
            }

            // Lyrics, queue, volume
            RowLayout {
                id: extras
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                spacing: Kirigami.Units.smallSpacing

                QQC2.ToolButton {
                    icon.name: "view-media-lyrics"
                    display: QQC2.AbstractButton.IconOnly
                    text: "Lyrics"
                    QQC2.ToolTip.text: text
                    QQC2.ToolTip.visible: hovered
                    onClicked: bar.lyricsRequested()
                }
                QQC2.ToolButton {
                    icon.name: "format-list-ordered"
                    display: QQC2.AbstractButton.IconOnly
                    text: "Queue"
                    QQC2.ToolTip.text: text
                    QQC2.ToolTip.visible: hovered
                    onClicked: bar.queueRequested()
                }
                Kirigami.Icon {
                    implicitWidth: Kirigami.Units.iconSizes.small
                    implicitHeight: Kirigami.Units.iconSizes.small
                    source: Backend.volume <= 0 ? "audio-volume-muted"
                          : Backend.volume < 34 ? "audio-volume-low"
                          : Backend.volume < 67 ? "audio-volume-medium" : "audio-volume-high"
                }
                QQC2.Slider {
                    id: volume
                    Layout.preferredWidth: Kirigami.Units.gridUnit * 6
                    from: 0
                    to: 100
                    onMoved: Backend.changeVolume(value)
                    Binding on value {
                        value: Backend.volume
                        when: !volume.pressed
                        restoreMode: Binding.RestoreNone
                    }
                }
            }
        }
    }
}
