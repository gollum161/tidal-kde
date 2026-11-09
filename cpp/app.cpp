// Application entry point. Kept in C++ because KDE's Breeze style for
// Qt Quick (org.kde.desktop) needs a QApplication, which cxx-qt-lib doesn't wrap.
#include <QApplication>
#include <QDir>
#include <QIcon>
#include <QLibraryInfo>
#include <QQmlApplicationEngine>
#include <QQuickStyle>
#include <QUrl>

static bool kdeStyleInstalled()
{
    const QString imports = QLibraryInfo::path(QLibraryInfo::QmlImportsPath);
    return QDir(imports + QStringLiteral("/org/kde/desktop")).exists();
}

extern "C" int tidal_kde_run(int argc, char **argv)
{
    static int s_argc = argc;  // QApplication keeps a reference to argc
    QApplication app(s_argc, argv);
    QApplication::setApplicationName(QStringLiteral("tidal-kde"));
    QApplication::setApplicationDisplayName(QStringLiteral("Tidal KDE"));
    QApplication::setApplicationVersion(QStringLiteral(TIDAL_KDE_VERSION));
    QApplication::setDesktopFileName(QStringLiteral("io.github.tidal_kde.TidalKde"));
    // Icons bundled next to the binary (the AppImage ships Breeze in usr/share/icons).
    // Breeze is only a fallback for icons the user's theme doesn't have.
    QIcon::setThemeSearchPaths(QIcon::themeSearchPaths()
                               << QApplication::applicationDirPath() + QStringLiteral("/../share/icons"));
    QIcon::setFallbackThemeName(QStringLiteral("breeze"));
    QApplication::setWindowIcon(QIcon::fromTheme(QStringLiteral("io.github.tidal_kde.TidalKde"),
                                                QIcon::fromTheme(QStringLiteral("audio-headphones"))));

    // Use Breeze for Qt Quick Controls unless the user picked a style.
    if (qEnvironmentVariableIsEmpty("QT_QUICK_CONTROLS_STYLE") && kdeStyleInstalled()) {
        QQuickStyle::setStyle(QStringLiteral("org.kde.desktop"));
    }

    QQmlApplicationEngine engine;
    // Developer override: TIDAL_KDE_QML=/path/to/file.qml loads a different UI.
    const QString custom = qEnvironmentVariable("TIDAL_KDE_QML");
    engine.load(custom.isEmpty() ? QUrl(QStringLiteral("qrc:/qt/qml/org/tidalkde/qml/Main.qml"))
                                 : QUrl::fromLocalFile(custom));
    if (engine.rootObjects().isEmpty()) {
        return 1;
    }
    return app.exec();
}
