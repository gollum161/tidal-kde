// Application entry point. Kept in C++ because KDE's Breeze style for
// Qt Quick (org.kde.desktop) needs a QApplication, which cxx-qt-lib doesn't wrap.
// Also home to the KDE calls used by src/appearance.rs (colour schemes, blur).
#include <QAbstractItemModel>
#include <QApplication>
#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusMessage>
#include <QDir>
#include <QFileInfo>
#include <QIcon>
#include <QLibraryInfo>
#include <QQmlApplicationEngine>
#include <QQuickStyle>
#include <QQuickWindow>
#include <QTimer>
#include <QUrl>

#include <KColorSchemeManager>
#include <KWindowEffects>

static QQmlApplicationEngine *s_engine = nullptr;

static bool kdeStyleInstalled()
{
    const QString imports = QLibraryInfo::path(QLibraryInfo::QmlImportsPath);
    return QDir(imports + QStringLiteral("/org/kde/desktop")).exists();
}

static QWindow *mainWindow()
{
    if (!s_engine || s_engine->rootObjects().isEmpty()) {
        return nullptr;
    }
    return qobject_cast<QWindow *>(s_engine->rootObjects().constFirst());
}

// The AppImage ships Breeze icons and colour schemes in its own usr/share.
// Returns that folder, or an empty string for a normal install (where
// /usr/share is already searched). Needs the QApplication.
static QString bundledShareDir()
{
    QStringList candidates;
    const QString appDir = qEnvironmentVariable("APPDIR"); // set by the AppImage runtime
    if (!appDir.isEmpty()) {
        candidates << appDir + QStringLiteral("/usr/share");
    }
    candidates << QDir::cleanPath(QApplication::applicationDirPath() + QStringLiteral("/../share"));
    for (const QString &dir : candidates) {
        if (QDir(dir + QStringLiteral("/color-schemes")).exists() && dir != QStringLiteral("/usr/share")) {
            return dir;
        }
    }
    return {};
}

// KDE finds colour schemes through XDG_DATA_DIRS, icons through the theme search path.
static void useBundledData()
{
    const QString share = bundledShareDir();
    if (share.isEmpty()) {
        return;
    }
    const QString dirs = qEnvironmentVariable("XDG_DATA_DIRS", QStringLiteral("/usr/local/share:/usr/share"));
    if (!dirs.split(QLatin1Char(':')).contains(share)) {
        qputenv("XDG_DATA_DIRS", (dirs + QLatin1Char(':') + share).toLocal8Bit());
    }
    QIcon::setThemeSearchPaths(QIcon::themeSearchPaths() << share + QStringLiteral("/icons"));
}

// Only one instance: the window may be hidden in the tray, so a second start
// brings the running one to the front (MPRIS Raise) instead of playing twice.
static bool raiseRunningInstance()
{
    const QString service = QStringLiteral("org.mpris.MediaPlayer2.tidal_kde");
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected() || !bus.interface()->isServiceRegistered(service)) {
        return false;
    }
    bus.call(QDBusMessage::createMethodCall(service, QStringLiteral("/org/mpris/MediaPlayer2"),
                                            QStringLiteral("org.mpris.MediaPlayer2"),
                                            QStringLiteral("Raise")),
             QDBus::Block, 2000);
    return true;
}

extern "C" void tidal_kde_apply_color_scheme(const char *id)
{
    KColorSchemeManager *manager = KColorSchemeManager::instance();
    manager->setAutosaveChanges(false); // the choice is saved in settings.json
    const QString wanted = QString::fromUtf8(id);
    QAbstractItemModel *model = manager->model();
    if (!wanted.isEmpty()) {
        for (int row = 0; row < model->rowCount(); ++row) {
            const QModelIndex index = model->index(row, 0);
            const QString path = index.data(Qt::UserRole).toString(); // KColorSchemeModel::PathRole
            if (!path.isEmpty() && QFileInfo(path).completeBaseName() == wanted) {
                manager->activateScheme(index);
                return;
            }
        }
    }
    manager->activateScheme(QModelIndex()); // system default
}

extern "C" bool tidal_kde_blur_available()
{
    return KWindowEffects::isEffectAvailable(KWindowEffects::BlurBehind);
}

extern "C" void tidal_kde_apply_blur(bool enable)
{
    if (QWindow *window = mainWindow()) {
        KWindowEffects::enableBlurBehind(window, enable);
    }
}

extern "C" int tidal_kde_run(int argc, char **argv)
{
    static int s_argc = argc;  // QApplication keeps a reference to argc
    QApplication app(s_argc, argv);
    QApplication::setApplicationName(QStringLiteral("tidal-kde"));
    QApplication::setApplicationDisplayName(QStringLiteral("Tidal KDE"));
    QApplication::setApplicationVersion(QStringLiteral(TIDAL_KDE_VERSION));
    QApplication::setDesktopFileName(QStringLiteral("io.github.tidal_kde.TidalKde"));

    if (raiseRunningInstance()) {
        return 0;
    }

    // Bundled icons and colour schemes (AppImage). Breeze is only a fallback for
    // icons the user's theme doesn't have.
    useBundledData();
    QIcon::setFallbackThemeName(QStringLiteral("breeze"));
    QApplication::setWindowIcon(QIcon::fromTheme(QStringLiteral("io.github.tidal_kde.TidalKde"),
                                                QIcon(QStringLiteral(":/qt/qml/org/tidalkde/icons/io.github.tidal_kde.TidalKde.svg"))));

    // Use Breeze for Qt Quick Controls unless the user picked a style.
    if (qEnvironmentVariableIsEmpty("QT_QUICK_CONTROLS_STYLE") && kdeStyleInstalled()) {
        QQuickStyle::setStyle(QStringLiteral("org.kde.desktop"));
    }

    // KWindowSystem learns about KWin's blur only after a Wayland roundtrip, so the
    // first query always says "unavailable". Ask once now; Main.qml asks again later.
    KWindowEffects::isEffectAvailable(KWindowEffects::BlurBehind);

    // Windows get an alpha channel so the background opacity setting can work.
    QQuickWindow::setDefaultAlphaBuffer(true);

    QQmlApplicationEngine engine;
    s_engine = &engine;
    // Developer override: TIDAL_KDE_QML=/path/to/file.qml loads a different UI.
    const QString custom = qEnvironmentVariable("TIDAL_KDE_QML");
    engine.load(custom.isEmpty() ? QUrl(QStringLiteral("qrc:/qt/qml/org/tidalkde/qml/Main.qml"))
                                 : QUrl::fromLocalFile(custom));
    if (engine.rootObjects().isEmpty()) {
        return 1;
    }

    // Developer aid: TIDAL_KDE_SCREENSHOT=/path/shot.png saves the window after
    // a few seconds and quits (works with QT_QPA_PLATFORM=offscreen).
    const QString shot = qEnvironmentVariable("TIDAL_KDE_SCREENSHOT");
    if (!shot.isEmpty()) {
        QTimer::singleShot(qEnvironmentVariableIntValue("TIDAL_KDE_SCREENSHOT_DELAY") ?: 4000, &app, [shot] {
            if (auto *window = qobject_cast<QQuickWindow *>(mainWindow())) {
                window->grabWindow().save(shot);
            }
            QApplication::quit();
        });
    }

    const int code = app.exec();
    s_engine = nullptr;
    return code;
}
