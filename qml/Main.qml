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
    // Bars (page toolbar, sidebar, player bar) add this much of their own background on
    // top of the translucent window, so they stay a little more solid than the content.
    readonly property real barOverlay: 0.25
    // Started by the autostart entry with --hidden: stay in the tray (if there is one).
    readonly property bool startHidden: Qt.application.arguments.indexOf("--hidden") >= 0
        && Backend.closeToTray && tray.available
    // How many playlists the sidebar lists before "All playlists…".
    readonly property int sidebarPlaylistLimit: 10
    readonly property var sidebarPlaylists: JSON.parse(Backend.sidebarPlaylistsJson)

    visible: !startHidden

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

    // A playlist picked in the sidebar replaces the current page, like a section.
    function openSidebarPlaylist(id, title) {
        section = "playlist:" + id
        pageStack.clear()
        pageStack.push(listPage, { mode: "playlist", arg: String(id), pageTitle: title })
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
            Backend.refreshSidebarPlaylists()
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

    // Logging out needs a confirmation: a misclick in the sidebar would otherwise
    // end the session and stop playback right away.
    Kirigami.PromptDialog {
        id: logoutDialog
        title: "Log out of TIDAL?"
        subtitle: "Playback stops and the queue is cleared. To log back in, you'll approve this device on TIDAL's website again."
        standardButtons: Kirigami.Dialog.NoButton
        customFooterActions: [
            Kirigami.Action {
                text: "Log out"
                icon.name: "system-log-out"
                onTriggered: {
                    logoutDialog.close()
                    Backend.logout()
                }
            },
            Kirigami.Action {
                text: "Cancel"
                icon.name: "dialog-cancel"
                onTriggered: logoutDialog.close()
            }
        ]
    }

    // Kirigami paints opaque backgrounds for pages, page toolbars, the sidebar
    // and the footer. When translucent, pages show only the window background and
    // the bars add a light layer (barOverlay) on top of it.
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
        value: root.barOverlay
        when: root.translucent && target !== null
    }
    Binding {
        target: drawer.background
        property: "opacity"
        value: root.barOverlay
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
        // Navigation is built from plain delegates rather than drawer actions: with
        // actions, Kirigami puts a stretching spacer between them and the content
        // below, which pushed the playlists to the bottom of the sidebar.
        component SidebarItem: QQC2.ItemDelegate {
            property string sectionName
            Layout.fillWidth: true
            highlighted: root.section === sectionName
            onClicked: {
                root.showSection(sectionName)
                if (drawer.modal) drawer.close()
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0

            SidebarItem { text: "Search"; icon.name: "search"; sectionName: "search" }
            SidebarItem { text: "Favorite tracks"; icon.name: "love"; sectionName: "tracks" }
            SidebarItem { text: "Albums"; icon.name: "view-media-album-cover"; sectionName: "albums" }
            SidebarItem { text: "Artists"; icon.name: "view-media-artist"; sectionName: "artists" }
            SidebarItem { text: "Playlists"; icon.name: "view-media-playlist"; sectionName: "playlists" }
            Kirigami.Separator {
                Layout.fillWidth: true
                Layout.topMargin: Kirigami.Units.smallSpacing
                Layout.bottomMargin: Kirigami.Units.smallSpacing
            }
            SidebarItem { text: "Queue"; icon.name: "format-list-ordered"; sectionName: "queue" }
            SidebarItem { text: "Lyrics"; icon.name: "view-media-lyrics"; sectionName: "lyrics" }
        }

        // The user's playlists, one click away (below the actions, scrolls with them).
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0
            visible: root.sidebarPlaylists.length > 0

            Kirigami.ListSectionHeader {
                text: "Your playlists"
                Layout.fillWidth: true
            }
            Repeater {
                model: root.sidebarPlaylists.slice(0, root.sidebarPlaylistLimit)
                delegate: QQC2.ItemDelegate {
                    required property var modelData
                    Layout.fillWidth: true
                    highlighted: root.section === "playlist:" + modelData.id
                    QQC2.ToolTip.text: modelData.title
                    QQC2.ToolTip.visible: hovered && label.truncated
                    QQC2.ToolTip.delay: Kirigami.Units.toolTipDelay
                    contentItem: RowLayout {
                        spacing: Kirigami.Units.smallSpacing
                        CoverImage {
                            size: Kirigami.Units.iconSizes.smallMedium
                            radius: Kirigami.Units.cornerRadius / 2
                            source: modelData.cover
                            fallbackIcon: "view-media-playlist"
                        }
                        QQC2.Label {
                            id: label
                            text: modelData.title
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }
                    }
                    onClicked: {
                        root.openSidebarPlaylist(modelData.id, modelData.title)
                        if (drawer.modal) drawer.close()
                    }
                }
            }
            QQC2.ItemDelegate {
                visible: root.sidebarPlaylists.length > root.sidebarPlaylistLimit
                Layout.fillWidth: true
                text: "All playlists…"
                icon.name: "view-more-symbolic"
                onClicked: {
                    root.showSection("playlists")
                    if (drawer.modal) drawer.close()
                }
            }
        }

        // Takes up the remaining height so the lists above stay at the top.
        Item { Layout.fillHeight: true }

        // Settings and logging out stay at the bottom, out of the way.
        footer: ColumnLayout {
            spacing: 0
            Kirigami.Separator { Layout.fillWidth: true }
            QQC2.ItemDelegate {
                Layout.fillWidth: true
                text: "Settings"
                icon.name: "configure"
                highlighted: root.section === "settings"
                onClicked: {
                    root.showSection("settings")
                    if (drawer.modal) drawer.close()
                }
            }
            QQC2.ItemDelegate {
                Layout.fillWidth: true
                text: "Log out"
                icon.name: "system-log-out"
                onClicked: {
                    if (drawer.modal) drawer.close()
                    logoutDialog.open()
                }
            }
        }
    }

    footer: PlayerBar {
        id: playerBar
        // Kirigami moves the footer left by the docked sidebar's width (x < 0) but
        // sizes it like the content area, so it would stop short of the right
        // edge: stretch it to the window's right edge and pad its content back
        // into view. Older Kirigami lets the docked sidebar cover the footer;
        // then the content also has to clear the sidebar.
        readonly property bool drawerCovers: !drawer.modal
            && drawer.y + drawer.height > root.height - height + 1
        width: root.width - x
        leftPadding: -x + (drawerCovers ? drawer.width * drawer.position : 0) + Kirigami.Units.smallSpacing
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
