import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.tidalkde

Kirigami.Page {
    id: page
    title: "Lyrics"

    readonly property var lines: JSON.parse(Backend.lyricsJson)
    readonly property bool hasTrack: Backend.title !== ""

    ColumnLayout {
        anchors.fill: parent
        spacing: Kirigami.Units.largeSpacing * 2
        visible: page.hasTrack

        RowLayout {
            Layout.fillWidth: true
            Layout.maximumWidth: Kirigami.Units.gridUnit * 40
            Layout.alignment: Qt.AlignHCenter
            spacing: Kirigami.Units.largeSpacing * 2

            CoverImage {
                size: Kirigami.Units.gridUnit * 6
                radius: Kirigami.Units.cornerRadius * 2
                source: Backend.cover
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: Kirigami.Units.smallSpacing
                Kirigami.Heading {
                    Layout.fillWidth: true
                    text: Backend.title
                    level: 1
                    wrapMode: Text.Wrap
                }
                QQC2.Label {
                    Layout.fillWidth: true
                    text: Backend.artist
                    opacity: 0.7
                    wrapMode: Text.Wrap
                }
            }
        }

        // Time-synced lyrics: the current line is highlighted and kept in view.
        ListView {
            id: synced
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.maximumWidth: Kirigami.Units.gridUnit * 40
            Layout.alignment: Qt.AlignHCenter
            visible: Backend.lyricsState === "ready" && Backend.lyricsSynced
            clip: true
            model: page.lines
            currentIndex: Backend.lyricsIndex
            highlightRangeMode: ListView.ApplyRange
            preferredHighlightBegin: height * 0.3
            preferredHighlightEnd: height * 0.3 + Kirigami.Units.gridUnit * 3
            highlightMoveDuration: Kirigami.Units.veryLongDuration
            highlightFollowsCurrentItem: true
            highlight: Item {}
            QQC2.ScrollBar.vertical: QQC2.ScrollBar {}

            delegate: QQC2.Label {
                id: line
                required property var modelData
                required property int index
                readonly property bool current: index === Backend.lyricsIndex

                width: ListView.view.width
                topPadding: Kirigami.Units.smallSpacing
                bottomPadding: Kirigami.Units.smallSpacing
                text: modelData.text !== "" ? modelData.text : "♪"
                wrapMode: Text.Wrap
                font.pointSize: Kirigami.Theme.defaultFont.pointSize * 1.7
                font.weight: Font.Bold
                color: current ? Kirigami.Theme.highlightColor : Kirigami.Theme.textColor
                opacity: current ? 1 : (index < Backend.lyricsIndex ? 0.5 : 0.3)
                Behavior on opacity { NumberAnimation { duration: Kirigami.Units.longDuration } }
                Behavior on color { ColorAnimation { duration: Kirigami.Units.longDuration } }

                HoverHandler { id: hover; cursorShape: Qt.PointingHandCursor }
                TapHandler { onTapped: Backend.seek(line.modelData.t) }
                QQC2.ToolTip.text: "Jump here"
                QQC2.ToolTip.visible: hover.hovered && !current
                QQC2.ToolTip.delay: Kirigami.Units.toolTipDelay
            }
        }

        // Unsynced lyrics: plain text.
        QQC2.ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.maximumWidth: Kirigami.Units.gridUnit * 40
            Layout.alignment: Qt.AlignHCenter
            visible: Backend.lyricsState === "ready" && !Backend.lyricsSynced
            QQC2.Label {
                width: parent.width
                text: page.lines.length > 0 ? page.lines[0].text : ""
                wrapMode: Text.Wrap
                font.pointSize: Kirigami.Theme.defaultFont.pointSize * 1.2
                lineHeight: 1.3
            }
        }

        QQC2.Label {
            Layout.maximumWidth: Kirigami.Units.gridUnit * 40
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignHCenter
            visible: Backend.lyricsState === "ready" && Backend.lyricsProvider !== ""
            text: "Lyrics provided by " + Backend.lyricsProvider
            font: Kirigami.Theme.smallFont
            opacity: 0.6
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: Backend.lyricsState !== "ready"

            QQC2.BusyIndicator {
                anchors.centerIn: parent
                visible: Backend.lyricsState === "loading"
                running: visible
            }
            Kirigami.PlaceholderMessage {
                anchors.centerIn: parent
                width: parent.width - Kirigami.Units.gridUnit * 4
                visible: Backend.lyricsState === "none"
                icon.name: "view-media-lyrics"
                text: "No lyrics for this track"
                explanation: "TIDAL doesn't have lyrics for this song."
            }
        }
    }

    Kirigami.PlaceholderMessage {
        anchors.centerIn: parent
        width: parent.width - Kirigami.Units.gridUnit * 4
        visible: !page.hasTrack
        icon.name: "view-media-lyrics"
        text: "Nothing playing"
        explanation: "Lyrics for the current track appear here."
    }
}
