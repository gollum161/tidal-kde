import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.tidalkde

Kirigami.ScrollablePage {
    id: page
    title: "Queue"

    readonly property var queue: JSON.parse(Backend.queueJson)

    actions: [
        Kirigami.Action {
            text: "Shuffle upcoming"
            icon.name: "media-playlist-shuffle"
            enabled: page.queue.length > Backend.queueIndex + 2
            onTriggered: Backend.shuffleUpcoming()
        },
        Kirigami.Action {
            text: "Clear"
            icon.name: "edit-clear-all"
            enabled: page.queue.length > 0
            onTriggered: Backend.queueClear()
        }
    ]

    ListView {
        id: list
        model: page.queue
        currentIndex: Backend.queueIndex
        highlightFollowsCurrentItem: false
        Component.onCompleted: if (currentIndex >= 0) positionViewAtIndex(currentIndex, ListView.Center)

        delegate: QQC2.ItemDelegate {
            id: row
            required property var modelData
            required property int index
            readonly property bool current: index === Backend.queueIndex

            width: ListView.view.width
            highlighted: current
            opacity: index < Backend.queueIndex ? 0.6 : 1
            onClicked: Backend.queuePlay(index)

            contentItem: RowLayout {
                spacing: Kirigami.Units.largeSpacing

                Item {
                    Layout.preferredWidth: Kirigami.Units.gridUnit * 1.5
                    Layout.fillHeight: true
                    Kirigami.Icon {
                        anchors.centerIn: parent
                        visible: row.current
                        source: Backend.playing ? "media-playback-start" : "media-playback-pause"
                        implicitWidth: Kirigami.Units.iconSizes.small
                        implicitHeight: Kirigami.Units.iconSizes.small
                    }
                    QQC2.Label {
                        anchors.centerIn: parent
                        visible: !row.current
                        text: row.index + 1
                        opacity: 0.6
                        font.features: { "tnum": 1 }
                    }
                }
                CoverImage {
                    size: Kirigami.Units.iconSizes.medium
                    source: row.modelData.cover
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0
                    QQC2.Label {
                        Layout.fillWidth: true
                        text: row.modelData.title
                        elide: Text.ElideRight
                        font.bold: row.current
                    }
                    QQC2.Label {
                        Layout.fillWidth: true
                        text: row.modelData.subtitle
                        elide: Text.ElideRight
                        opacity: 0.7
                        font: Kirigami.Theme.smallFont
                    }
                }
                QQC2.Label {
                    text: Math.floor(row.modelData.duration / 60) + ":"
                          + String(Math.floor(row.modelData.duration % 60)).padStart(2, "0")
                    opacity: 0.6
                    font.features: { "tnum": 1 }
                }
                QQC2.ToolButton {
                    icon.name: "go-up"
                    display: QQC2.AbstractButton.IconOnly
                    text: "Move up"
                    enabled: row.index > 0
                    onClicked: Backend.queueMove(row.index, row.index - 1)
                }
                QQC2.ToolButton {
                    icon.name: "go-down"
                    display: QQC2.AbstractButton.IconOnly
                    text: "Move down"
                    enabled: row.index < page.queue.length - 1
                    onClicked: Backend.queueMove(row.index, row.index + 1)
                }
                QQC2.ToolButton {
                    icon.name: "list-remove"
                    display: QQC2.AbstractButton.IconOnly
                    text: "Remove from queue"
                    QQC2.ToolTip.text: text
                    QQC2.ToolTip.visible: hovered
                    onClicked: Backend.queueRemove(row.index)
                }
            }
        }

        Kirigami.PlaceholderMessage {
            anchors.centerIn: parent
            width: parent.width - Kirigami.Units.gridUnit * 4
            visible: page.queue.length === 0
            icon.name: "format-list-ordered"
            text: "Queue is empty"
            explanation: "Play something, or use the + button on any track to add it here."
        }
    }
}
