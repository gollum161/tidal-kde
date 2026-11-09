import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.tidalkde

Kirigami.Page {
    title: "Tidal KDE"

    readonly property bool waiting: Backend.loginCode !== ""

    Connections {
        target: Backend
        function onLoginUrlChanged() {
            if (Backend.loginUrl !== "")
                Qt.openUrlExternally(Backend.loginUrl)
        }
    }

    Kirigami.PlaceholderMessage {
        anchors.centerIn: parent
        width: parent.width - Kirigami.Units.gridUnit * 4
        icon.name: "audio-headphones"
        text: waiting ? "Approve this device on TIDAL" : "Connect your TIDAL account"
        explanation: waiting
            ? "Your browser should have opened TIDAL's login page. Check that it shows this code, then approve."
            : "You'll approve Tidal KDE on TIDAL's website. Playback needs a TIDAL subscription."
        helpfulAction: Kirigami.Action {
            text: waiting ? "Open the login page again" : "Log in with TIDAL"
            icon.name: waiting ? "internet-services" : "go-next"
            onTriggered: waiting ? Qt.openUrlExternally(Backend.loginUrl) : Backend.startLogin()
        }

        Kirigami.Heading {
            Layout.alignment: Qt.AlignHCenter
            visible: waiting
            text: Backend.loginCode
            level: 1
            font.letterSpacing: 4
        }
    }
}
