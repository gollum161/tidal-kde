import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.tidalkde

Kirigami.ApplicationWindow {
    id: root

    property string section: ""
    // Background opacity from the settings; text and covers stay opaque.
    readonly property real surfaceAlpha: Backend.windowOpacity
    readonly property bool translucent: surfaceAlpha < 1
    readonly property bool trayAvailable: tray.available

    title: Backend.title !== "" ? Backend.title + " — " + Backend.artist : "Tidal KDE"
    width: Kirigami.Units.gridUnit * 66
    height: Kirigami.Units.gridUnit * 42
    minimumWidth: Kirigami.Units.gridUnit * 34
    minimumHeight: Kirigami.Units.gridUnit * 26
    color: Qt.alpha(Kirigami.Theme.backgroundColor, surfaceAlpha)

    pageStack.columnView.columnResizeMode: Kirigami.ColumnView.SingleColumn
    pageStack.globalToolBar.showNavigationButtons: Kirigami.ApplicationHeaderStyle.ShowBackButton

    function showSection(name) {
        section = name
        pageStack.clear()
        if (name === "queue") {
            pageStack.push(queuePage)
        } else if (name === "lyrics") {
            pageStack.push(lyricsPage)
        } else if (name === "settings") {
            pageStack.push(settingsPage)
        } else {
            pageStack.push(listPage, { mode: name })
        }
    }

    function openItem(kind, id, title) {
        if (kind === "album" || kind === "artist" || kind === "playlist") {
            pageStack.push(listPage, { mode: kind, arg: String(id), pageTitle: title })
        }
    }

    function refreshRoot() {
        pageStack.clear()
        section = ""
        if (Backend.connecting) {
            pageStack.push(connectingPage)
        } else if (!Backend.loggedIn) {
            pageStack.push(loginPage)
        } else {
            showSection("search")
        }
    }

    Component.onCompleted: {
        refreshRoot()
        Backend.applyWindowEffects()
    }
    // Blur support is only known after the compositor answered (see app.cpp).
    Timer {
        interval: 1000
        running: true
        onTriggered: Backend.applyWindowEffects()
    }
    // A hidden window loses its native surface; restore the blur when it comes back.
    onVisibleChanged: if (visible) Backend.applyWindowEffects()

    // With a tray icon, closing only hides the window and playback continues.
    onClosing: close => {
        if (tray.available && Backend.closeToTray) {
            close.accepted = false
            root.hide()
        }
    }

    TrayIcon {
        id: tray
        window: root
    }

    // Kirigami paints opaque backgrounds for pages, page toolbars, the sidebar
    // and the footer. When translucent, pages show the window background and
    // the bars fade with it (they stay a little more solid than the content).
    Binding {
        target: root.pageStack.currentItem ? root.pageStack.currentItem.background : null
        property: "opacity"
        value: 0
        when: root.translucent && target !== null
    }
    Binding {
        target: root.pageStack.currentItem && root.pageStack.currentItem.globalToolBarItem
            ? root.pageStack.currentItem.globalToolBarItem.background : null
        property: "opacity"
        value: root.surfaceAlpha
        when: root.translucent && target !== null
    }
    Binding {
        target: drawer.background
        property: "opacity"
        value: root.surfaceAlpha
        when: root.translucent && target !== null
    }

    Connections {
        target: Backend
        function onLoggedInChanged() { root.refreshRoot() }
        function onConnectingChanged() { root.refreshRoot() }
        function onShowMessage(message) { root.showPassiveNotification(message) }
        function onRaiseRequested() { root.show(); root.raise(); root.requestActivate() }
        function onQuitRequested() { Qt.quit() }
    }

    globalDrawer: Kirigami.GlobalDrawer {
        id: drawer
        enabled: Backend.loggedIn
        modal: !root.wideScreen
        width: Kirigami.Units.gridUnit * 13
        isMenu: false
        actions: [
            Kirigami.Action {
                text: "Search"; icon.name: "search"
                checkable: true; checked: root.section === "search"
                onTriggered: root.showSection("search")
            },
            Kirigami.Action {
                text: "Favorite tracks"; icon.name: "love"
                checkable: true; checked: root.section === "tracks"
                onTriggered: root.showSection("tracks")
            },
            Kirigami.Action {
                text: "Albums"; icon.name: "view-media-album-cover"
                checkable: true; checked: root.section === "albums"
                onTriggered: root.showSection("albums")
            },
            Kirigami.Action {
                text: "Artists"; icon.name: "view-media-artist"
                checkable: true; checked: root.section === "artists"
                onTriggered: root.showSection("artists")
            },
            Kirigami.Action {
                text: "Playlists"; icon.name: "view-media-playlist"
                checkable: true; checked: root.section === "playlists"
                onTriggered: root.showSection("playlists")
            },
            Kirigami.Action { separator: true },
            Kirigami.Action {
                text: "Queue"; icon.name: "format-list-ordered"
                checkable: true; checked: root.section === "queue"
                onTriggered: root.showSection("queue")
            },
            Kirigami.Action {
                text: "Lyrics"; icon.name: "view-media-lyrics"
                checkable: true; checked: root.section === "lyrics"
                onTriggered: root.showSection("lyrics")
            },
            Kirigami.Action { separator: true },
            Kirigami.Action {
                text: "Streaming quality"; icon.name: "audio-x-generic"
                Kirigami.Action {
                    text: "Lossless (FLAC)"; checkable: true
                    checked: Backend.quality === "LOSSLESS"
                    onTriggered: Backend.setStreamQuality("LOSSLESS")
                }
                Kirigami.Action {
                    text: "High (320 kbps)"; checkable: true
                    checked: Backend.quality === "HIGH"
                    onTriggered: Backend.setStreamQuality("HIGH")
                }
                Kirigami.Action {
                    text: "Low (96 kbps)"; checkable: true
                    checked: Backend.quality === "LOW"
                    onTriggered: Backend.setStreamQuality("LOW")
                }
            },
            Kirigami.Action {
                text: "Settings"; icon.name: "configure"
                checkable: true; checked: root.section === "settings"
                onTriggered: root.showSection("settings")
            },
            Kirigami.Action {
                text: "Log out"; icon.name: "system-log-out"
                onTriggered: Backend.logout()
            }
        ]
    }

    footer: PlayerBar {
        // The docked sidebar covers the full window height, so keep clear of it.
        leftPadding: (drawer.modal ? 0 : drawer.width * drawer.position) + Kirigami.Units.smallSpacing
        visible: Backend.loggedIn
        onLyricsRequested: root.showSection("lyrics")
        onQueueRequested: root.showSection("queue")
    }

    Shortcut { sequence: "Ctrl+Space"; onActivated: Backend.togglePause() }
    Shortcut { sequence: "Ctrl+."; onActivated: Backend.next() }
    Shortcut { sequence: "Ctrl+,"; onActivated: Backend.previous() }
    Shortcut { sequence: "Ctrl+F"; enabled: Backend.loggedIn; onActivated: root.showSection("search") }
    Shortcut { sequence: "Ctrl+L"; enabled: Backend.loggedIn; onActivated: root.showSection("lyrics") }
    Shortcut { sequence: "Ctrl+U"; enabled: Backend.loggedIn; onActivated: root.showSection("queue") }
    Shortcut { sequence: StandardKey.Quit; onActivated: Qt.quit() }

    Component { id: listPage; ItemListPage {} }
    Component { id: queuePage; QueuePage {} }
    Component { id: lyricsPage; LyricsPage {} }
    Component { id: loginPage; LoginPage {} }
    Component { id: settingsPage; SettingsPage {} }
    Component {
        id: connectingPage
        Kirigami.Page {
            title: "Tidal KDE"
            QQC2.BusyIndicator { anchors.centerIn: parent; running: true }
        }
    }
}
