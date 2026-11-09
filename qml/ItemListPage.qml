import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.tidalkde

// One page type for search, library sections and album/artist/playlist pages.
Kirigami.ScrollablePage {
    id: page

    property string mode: "search"      // search | tracks | albums | artists | playlists | album | artist | playlist
    property string arg: ""
    property string pageTitle: ""
    property int request: 0
    property var tracks: []
    property var detail: null
    property bool loading: false
    property bool sectioned: false
    property string errorText: ""
    property string query: ""
    property int menuIndex: -1

    readonly property bool isSearch: mode === "search"
    readonly property bool isDetail: mode === "album" || mode === "artist" || mode === "playlist"

    title: isSearch ? "Search" : pageTitle

    function reload() {
        rows.clear()
        tracks = []
        detail = null
        errorText = ""
        request = Math.floor(Math.random() * 2000000000)
        if (isSearch && query === "") {
            loading = false
            return
        }
        loading = true
        Backend.load(request, mode, isSearch ? query : arg)
    }

    function shuffled(list) {
        const copy = list.slice()
        for (let i = copy.length - 1; i > 0; i--) {
            const j = Math.floor(Math.random() * (i + 1))
            const t = copy[i]; copy[i] = copy[j]; copy[j] = t
        }
        return copy
    }

    function playAll(shuffle) {
        if (tracks.length > 0)
            Backend.playTracks(JSON.stringify(shuffle ? shuffled(tracks) : tracks), 0)
    }

    Component.onCompleted: reload()

    Connections {
        target: Backend
        function onPageLoaded(request, payload) {
            if (request !== page.request)
                return
            const data = JSON.parse(payload)
            page.loading = false
            page.sectioned = data.sectioned
            page.detail = data.header
            page.tracks = data.tracks
            if (data.title && !page.isSearch)
                page.pageTitle = data.title
            rows.clear()
            for (const row of data.rows)
                rows.append(row)
        }
        function onPageFailed(request, message) {
            if (request !== page.request)
                return
            page.loading = false
            page.errorText = message
        }
    }

    titleDelegate: RowLayout {
        Layout.fillWidth: true
        Kirigami.Heading {
            visible: !page.isSearch
            Layout.fillWidth: true
            text: page.title
            level: 2
            elide: Text.ElideRight
        }
        Kirigami.SearchField {
            id: searchField
            visible: page.isSearch
            Layout.fillWidth: true
            Layout.maximumWidth: Kirigami.Units.gridUnit * 26
            placeholderText: "Tracks, albums, artists, playlists"
            delaySearch: true
            text: page.query
            onAccepted: {
                page.query = text.trim()
                page.reload()
            }
            Component.onCompleted: if (page.isSearch) forceActiveFocus()
        }
    }

    actions: [
        Kirigami.Action {
            text: "Play all"
            icon.name: "media-playback-start"
            visible: page.mode === "tracks" && page.tracks.length > 0
            onTriggered: page.playAll(false)
        },
        Kirigami.Action {
            text: "Shuffle"
            icon.name: "media-playlist-shuffle"
            visible: page.mode === "tracks" && page.tracks.length > 0
            onTriggered: page.playAll(true)
        }
    ]

    QQC2.Menu {
        id: trackMenu
        QQC2.MenuItem {
            text: "Play next"
            icon.name: "media-playlist-append"
            onTriggered: Backend.playNext(JSON.stringify([page.tracks[rows.get(page.menuIndex).trackIndex]]))
        }
        QQC2.MenuItem {
            text: "Add to queue"
            icon.name: "list-add"
            onTriggered: Backend.enqueue(JSON.stringify([page.tracks[rows.get(page.menuIndex).trackIndex]]))
        }
        QQC2.MenuItem {
            text: "Add to favorites"
            icon.name: "love"
            onTriggered: Backend.like("track", rows.get(page.menuIndex).rid)
        }
        QQC2.MenuSeparator {}
        QQC2.MenuItem {
            text: "Go to album"
            icon.name: "view-media-album-cover"
            onTriggered: applicationWindow().openItem("album", rows.get(page.menuIndex).albumId, "")
        }
        QQC2.MenuItem {
            text: "Go to artist"
            icon.name: "view-media-artist"
            onTriggered: applicationWindow().openItem("artist", rows.get(page.menuIndex).artistId, "")
        }
    }

    ListView {
        id: list
        model: ListModel { id: rows }
        reuseItems: true

        section.property: page.sectioned ? "section" : ""
        section.delegate: Kirigami.ListSectionHeader {
            required property string section
            width: ListView.view.width
            text: section
        }

        header: page.detail ? detailHeader : null

        Component {
            id: detailHeader
            RowLayout {
                width: list.width
                spacing: Kirigami.Units.largeSpacing * 2

                CoverImage {
                    Layout.margins: Kirigami.Units.largeSpacing
                    size: Kirigami.Units.gridUnit * 9
                    radius: Kirigami.Units.cornerRadius * 2
                    source: page.detail ? page.detail.cover : ""
                    fallbackIcon: page.mode === "artist" ? "view-media-artist" : "media-album-cover"
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.rightMargin: Kirigami.Units.largeSpacing
                    spacing: Kirigami.Units.smallSpacing

                    Kirigami.Heading {
                        Layout.fillWidth: true
                        text: page.detail ? page.detail.title : ""
                        level: 1
                        wrapMode: Text.Wrap
                    }
                    QQC2.Label {
                        Layout.fillWidth: true
                        text: page.detail ? page.detail.subtitle : ""
                        wrapMode: Text.Wrap
                        opacity: 0.7
                    }
                    RowLayout {
                        Layout.topMargin: Kirigami.Units.largeSpacing
                        spacing: Kirigami.Units.smallSpacing
                        QQC2.Button {
                            text: page.mode === "artist" ? "Play popular" : "Play"
                            icon.name: "media-playback-start"
                            enabled: page.tracks.length > 0
                            onClicked: page.playAll(false)
                        }
                        QQC2.Button {
                            text: "Shuffle"
                            icon.name: "media-playlist-shuffle"
                            enabled: page.tracks.length > 0
                            onClicked: page.playAll(true)
                        }
                        QQC2.Button {
                            text: "Add to queue"
                            icon.name: "list-add"
                            enabled: page.tracks.length > 0
                            onClicked: Backend.enqueue(JSON.stringify(page.tracks))
                        }
                        QQC2.Button {
                            icon.name: "love"
                            visible: page.mode !== "playlist"
                            QQC2.ToolTip.text: "Add to favorites"
                            QQC2.ToolTip.visible: hovered
                            onClicked: Backend.like(page.mode, page.arg)
                        }
                    }
                }
            }
        }

        delegate: QQC2.ItemDelegate {
            id: row
            required property int index
            required property string kind
            required property string rid
            required property string title
            required property string subtitle
            required property string cover
            required property real duration
            required property int number
            required property int trackIndex

            readonly property bool isTrack: kind === "track"

            width: ListView.view.width
            onClicked: isTrack
                ? Backend.playTracks(JSON.stringify(page.tracks), trackIndex)
                : applicationWindow().openItem(kind, rid, title)

            contentItem: RowLayout {
                spacing: Kirigami.Units.largeSpacing

                QQC2.Label {
                    visible: row.isTrack && row.cover === "" && row.number > 0
                    text: row.number
                    opacity: 0.6
                    horizontalAlignment: Text.AlignRight
                    Layout.preferredWidth: Kirigami.Units.gridUnit * 1.5
                    font.features: { "tnum": 1 }
                }
                CoverImage {
                    visible: row.cover !== "" || !row.isTrack
                    size: Kirigami.Units.iconSizes.medium
                    source: row.cover
                    radius: row.kind === "artist" ? size / 2 : Kirigami.Units.cornerRadius
                    fallbackIcon: row.kind === "artist" ? "view-media-artist"
                                : row.kind === "playlist" ? "view-media-playlist" : "media-album-cover"
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0
                    QQC2.Label {
                        Layout.fillWidth: true
                        text: row.title
                        elide: Text.ElideRight
                    }
                    QQC2.Label {
                        Layout.fillWidth: true
                        text: row.subtitle
                        elide: Text.ElideRight
                        opacity: 0.7
                        font: Kirigami.Theme.smallFont
                    }
                }
                QQC2.Label {
                    visible: row.isTrack
                    text: Math.floor(row.duration / 60) + ":" + String(Math.floor(row.duration % 60)).padStart(2, "0")
                    opacity: 0.6
                    font.features: { "tnum": 1 }
                }
                QQC2.ToolButton {
                    visible: row.isTrack
                    icon.name: "list-add"
                    display: QQC2.AbstractButton.IconOnly
                    text: "Add to queue"
                    QQC2.ToolTip.text: text
                    QQC2.ToolTip.visible: hovered
                    onClicked: Backend.enqueue(JSON.stringify([page.tracks[row.trackIndex]]))
                }
                QQC2.ToolButton {
                    visible: row.isTrack
                    icon.name: "overflow-menu"
                    display: QQC2.AbstractButton.IconOnly
                    text: "More"
                    onClicked: {
                        page.menuIndex = row.index
                        trackMenu.popup()
                    }
                }
                Kirigami.Icon {
                    visible: !row.isTrack
                    source: "go-next"
                    implicitWidth: Kirigami.Units.iconSizes.small
                    implicitHeight: Kirigami.Units.iconSizes.small
                }
            }
        }

        QQC2.BusyIndicator {
            anchors.centerIn: parent
            visible: page.loading
            running: visible
        }

        Kirigami.PlaceholderMessage {
            anchors.centerIn: parent
            width: parent.width - Kirigami.Units.gridUnit * 4
            visible: !page.loading && rows.count === 0
            icon.name: page.errorText !== "" ? "dialog-error"
                     : page.isSearch ? "search" : "view-media-playlist"
            text: page.errorText !== "" ? "Couldn't load this page"
                : page.isSearch && page.query === "" ? "Search TIDAL"
                : page.isSearch ? "No results"
                : "Nothing here yet"
            explanation: page.errorText !== "" ? page.errorText
                : page.isSearch && page.query === "" ? "Find tracks, albums, artists and playlists."
                : page.isSearch ? "Nothing on TIDAL matches “" + page.query + "”. Check the spelling or try fewer words."
                : "Add favorites on TIDAL or from the ⋯ menu on any track, and they'll show up here."
            helpfulAction: Kirigami.Action {
                enabled: page.errorText !== ""
                text: "Try again"
                icon.name: "view-refresh"
                onTriggered: page.reload()
            }
        }
    }
}
