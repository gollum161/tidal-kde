// Not part of the app. The app loads the Breeze Qt Quick style (org.kde.desktop)
// at runtime instead of importing it, so it's listed here for the AppImage build:
// linuxdeploy-plugin-qt bundles every module imported by the QML files it scans.
import QtQuick
import org.kde.desktop

Item {}
