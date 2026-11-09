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
            currentIndex: Math.max(0, indexOfValue(Backend.colorScheme))
            onActivated: Backend.changeColorScheme(currentValue)
        }

        RowLayout {
            Kirigami.FormData.label: "Background opacity:"
            spacing: Kirigami.Units.smallSpacing
            QQC2.Slider {
                id: opacitySlider
                from: 0.2
                to: 1
                stepSize: 0.05
                value: Backend.windowOpacity
                Layout.preferredWidth: Kirigami.Units.gridUnit * 12
                onMoved: Backend.changeWindowOpacity(value)
            }
            QQC2.Label {
                text: Math.round(opacitySlider.value * 100) + " %"
                Layout.minimumWidth: Kirigami.Units.gridUnit * 2.5
            }
        }

        QQC2.CheckBox {
            text: "Blur what's behind the window"
            checked: Backend.blur
            enabled: Backend.blurAvailable && Backend.windowOpacity < 1
            onToggled: Backend.changeBlur(checked)
        }
        QQC2.Label {
            visible: !Backend.blurAvailable
            text: "Blur needs a compositor that supports it, such as KWin on KDE Plasma."
            font: Kirigami.Theme.smallFont
            opacity: 0.7
            wrapMode: Text.Wrap
            Layout.maximumWidth: Kirigami.Units.gridUnit * 20
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
    }
}
