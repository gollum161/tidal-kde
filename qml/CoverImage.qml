import QtQuick
import org.kde.kirigami as Kirigami

// Album art with rounded corners and an icon while it loads (or when there is none).
Item {
    id: cover

    property string source: ""
    property int size: Kirigami.Units.iconSizes.medium
    property string fallbackIcon: "media-album-cover"
    property real radius: Kirigami.Units.cornerRadius

    implicitWidth: size
    implicitHeight: size

    Rectangle {
        anchors.fill: parent
        radius: cover.radius
        color: Kirigami.Theme.alternateBackgroundColor
        visible: image.status !== Image.Ready

        Kirigami.Icon {
            anchors.centerIn: parent
            width: Math.round(parent.width * 0.55)
            height: width
            source: cover.fallbackIcon
            opacity: 0.6
        }
    }

    Kirigami.ShadowedImage {
        id: image
        anchors.fill: parent
        source: cover.source
        radius: cover.radius
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        sourceSize.width: cover.size * 2
        sourceSize.height: cover.size * 2
        visible: cover.source !== ""
    }
}
