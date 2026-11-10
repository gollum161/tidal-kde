import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.tidalkde

// Appearance and behaviour settings. Every change is applied and saved at once.
Kirigami.ScrollablePage {
    id: page
    title: "Settings"

    readonly property var schemes: [{ id: "", name: "System default" }]
        .concat(JSON.parse(Backend.colorSchemesJson))

    Kirigami.FormLayout {
        Kirigami.Separator {
            Kirigami.FormData.isSection: true
            Kirigami.FormData.label: "Appearance"
        }

        QQC2.ComboBox {
            id: schemeBox
            Kirigami.FormData.label: "Color scheme:"
            model: page.schemes
            textRole: "name"
            valueRole: "id"
            // From the list itself: indexOfValue() runs before the model is filled.
            currentIndex: Math.max(0, page.schemes.findIndex(s => s.id === Backend.colorScheme))
            onActivated: Backend.changeColorScheme(currentValue)
        }

        // Shown as transparency (0 % = solid), stored as the window's opacity.
        RowLayout {
            Kirigami.FormData.label: "Transparency:"
            spacing: Kirigami.Units.smallSpacing
            QQC2.Slider {
                id: transparencySlider
                from: 0
                to: 0.95 // = MIN_WINDOW_OPACITY 0.05 in bridge.rs
                stepSize: 0.05
                snapMode: QQC2.Slider.SnapAlways
                value: 1 - Backend.windowOpacity
                Layout.preferredWidth: Kirigami.Units.gridUnit * 12
                onMoved: Backend.changeWindowOpacity(1 - value)
            }
            QQC2.Label {
                text: Math.round(transparencySlider.value * 100) + " %"
                Layout.minimumWidth: Kirigami.Units.gridUnit * 2.5
            }
        }

        // One-click presets for the slider above.
        RowLayout {
            spacing: Kirigami.Units.smallSpacing
            Repeater {
                model: [
                    { text: "Off", value: 0 },
                    { text: "Light", value: 0.25 },
                    { text: "Medium", value: 0.5 },
                    { text: "Strong", value: 0.7 }
                ]
                delegate: QQC2.Button {
                    required property var modelData
                    text: modelData.text
                    checkable: true
                    checked: Math.abs((1 - Backend.windowOpacity) - modelData.value) < 0.01
                    onClicked: Backend.changeWindowOpacity(1 - modelData.value)
                }
            }
        }

        QQC2.CheckBox {
            text: "Blur what's behind the window"
            checked: Backend.blur
            enabled: Backend.blurAvailable
            onToggled: Backend.changeBlur(checked)
        }
        QQC2.Label {
            text: !Backend.blurAvailable
                ? "Blur needs a compositor that supports it, such as KWin on KDE Plasma."
                : Backend.windowOpacity >= 1
                    ? "Blur shows once the transparency is above 0 %. It's clearest from about 40 %."
                    : "How strong the blur is comes from KDE's System Settings (Desktop Effects → Blur)."
            font: Kirigami.Theme.smallFont
            opacity: 0.7
            wrapMode: Text.Wrap
            Layout.maximumWidth: Kirigami.Units.gridUnit * 20
        }

        Kirigami.Separator {
            Kirigami.FormData.isSection: true
            Kirigami.FormData.label: "Playback"
        }

        QQC2.ComboBox {
            Kirigami.FormData.label: "Streaming quality:"
            readonly property var qualities: [
                { id: "LOSSLESS", name: "Lossless (FLAC, 16-bit)" },
                { id: "HIGH", name: "High (320 kbps)" },
                { id: "LOW", name: "Low (96 kbps)" }
            ]
            model: qualities
            textRole: "name"
            valueRole: "id"
            currentIndex: Math.max(0, qualities.findIndex(q => q.id === Backend.quality))
            onActivated: Backend.setStreamQuality(currentValue)
        }

        Kirigami.Separator {
            Kirigami.FormData.isSection: true
            Kirigami.FormData.label: "Behavior"
        }

        QQC2.CheckBox {
            Kirigami.FormData.label: "System tray:"
            text: "Keep playing in the tray when the window is closed"
            checked: Backend.closeToTray
            onToggled: Backend.changeCloseToTray(checked)
        }
        QQC2.Label {
            visible: !applicationWindow().trayAvailable
            text: "No system tray was found, so closing the window quits the app."
            font: Kirigami.Theme.smallFont
            opacity: 0.7
            wrapMode: Text.Wrap
            Layout.maximumWidth: Kirigami.Units.gridUnit * 20
        }

        Kirigami.Separator {
            Kirigami.FormData.isSection: true
            Kirigami.FormData.label: "Startup"
        }

        QQC2.CheckBox {
            Kirigami.FormData.label: "On login:"
            text: "Start Tidal KDE when I log in"
            checked: Backend.autostart
            onToggled: Backend.changeAutostart(checked)
        }
        QQC2.CheckBox {
            text: "Start hidden in the system tray"
            checked: Backend.autostartHidden
            enabled: Backend.autostart && Backend.closeToTray
            onToggled: Backend.changeAutostartHidden(checked)
        }
        QQC2.Label {
            visible: Backend.autostart && !Backend.closeToTray
            text: "Starting hidden needs \"Keep playing in the tray\" above."
            font: Kirigami.Theme.smallFont
            opacity: 0.7
            wrapMode: Text.Wrap
            Layout.maximumWidth: Kirigami.Units.gridUnit * 20
        }
    }
}
