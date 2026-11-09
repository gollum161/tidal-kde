import QtQuick
import Qt.labs.platform as Platform
import org.tidalkde

// System tray icon (StatusNotifierItem on Plasma). Clicking it shows or hides
// the window; playback keeps running while the window is hidden.
Platform.SystemTrayIcon {
    id: tray

    required property var window

    function toggleWindow() {
        if (window.visible) {
            window.hide()
        } else {
            window.show()
            window.raise()
            window.requestActivate()
        }
    }

    visible: Backend.closeToTray
    // The bundled file, not the theme name: Qt.labs.platform ignores `source` when
    // `name` is set, and the theme icon is missing in dev builds and on some hosts.
    icon.source: "qrc:/qt/qml/org/tidalkde/icons/io.github.tidal_kde.TidalKde.svg"
    tooltip: Backend.title !== "" ? Backend.title + " — " + Backend.artist : "Tidal KDE"

    onActivated: reason => {
        if (reason === Platform.SystemTrayIcon.Trigger) {
            tray.toggleWindow()
        }
    }

    menu: Platform.Menu {
        Platform.MenuItem {
            text: tray.window.visible ? "Hide window" : "Show window"
            onTriggered: tray.toggleWindow()
        }
        Platform.MenuSeparator {}
        Platform.MenuItem {
            text: Backend.playing ? "Pause" : "Play"
            icon.name: Backend.playing ? "media-playback-pause" : "media-playback-start"
            enabled: Backend.active
            onTriggered: Backend.togglePause()
        }
        Platform.MenuItem {
            text: "Next track"
            icon.name: "media-skip-forward"
            enabled: Backend.active
            onTriggered: Backend.next()
        }
        Platform.MenuItem {
            text: "Previous track"
            icon.name: "media-skip-backward"
            enabled: Backend.active
            onTriggered: Backend.previous()
        }
        Platform.MenuSeparator {}
        Platform.MenuItem {
            text: "Quit"
            icon.name: "application-exit"
            onTriggered: Qt.quit()
        }
    }
}
