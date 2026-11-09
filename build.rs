use cxx_qt_build::{CxxQtBuilder, QmlModule};

/// GCC link-time optimisation (-flto, which makepkg adds by default) turns C++ into
/// GCC-internal bytecode that Rust's linker (lld) can't read, so linking fails with
/// "undefined symbol" errors. That also affects dependencies, so stop early with a
/// clear message instead of a confusing linker error.
fn reject_gcc_lto_flags() {
    for key in ["CFLAGS", "CXXFLAGS"] {
        println!("cargo:rerun-if-env-changed={key}");
        if std::env::var(key).is_ok_and(|v| v.split_whitespace().any(|f| f.starts_with("-flto"))) {
            panic!(
                "\n\n{key} contains -flto (GCC link-time optimisation), which can't be linked \
                 with Rust's linker.\nBuild with the PKGBUILD in packaging/ (it sets options=('!lto')), \
                 or unset {key} for a manual `cargo build`.\n\n"
            );
        }
    }
}

/// KDE Frameworks used by cpp/app.cpp (colour schemes, window blur).
/// Headers live in KF6/K<Name>, the library is libKF6<Name> (KColorScheme, KF6ColorScheme).
const KF6_FRAMEWORKS: [&str; 2] = ["ColorScheme", "WindowSystem"];

/// KF6 installs its headers in <prefix>/include/KF6/<Framework> (Arch, Debian, Fedora).
fn kf6_include_dir() -> String {
    std::env::var("KF6_INCLUDE_DIR").unwrap_or_else(|_| "/usr/include/KF6".into())
}

fn main() {
    reject_gcc_lto_flags();

    CxxQtBuilder::new()
        .qt_module("DBus")
        .qt_module("Network")
        .qt_module("Quick")
        .qt_module("QuickControls2")
        .qt_module("Widgets")
        .qml_module(QmlModule {
            uri: "org.tidalkde",
            rust_files: &["src/bridge.rs"],
            qml_files: &[
                "qml/Main.qml",
                "qml/ItemListPage.qml",
                "qml/QueuePage.qml",
                "qml/LyricsPage.qml",
                "qml/LoginPage.qml",
                "qml/PlayerBar.qml",
                "qml/CoverImage.qml",
                "qml/SettingsPage.qml",
                "qml/TrayIcon.qml",
            ],
            // Fallback app icon for the tray and window when it isn't installed in
            // the icon theme (dev builds): qrc:/qt/qml/org/tidalkde/icons/…
            qrc_files: &["icons/io.github.tidal_kde.TidalKde.svg"],
            ..Default::default()
        })
        .cc_builder(|cc| {
            let version = format!("\"{}\"", env!("CARGO_PKG_VERSION"));
            cc.define("TIDAL_KDE_VERSION", version.as_str());
            for framework in KF6_FRAMEWORKS {
                cc.include(format!("{}/K{framework}", kf6_include_dir()));
            }
            cc.file("cpp/app.cpp");
        })
        .build();
    println!("cargo:rerun-if-changed=cpp/app.cpp");
    println!("cargo:rerun-if-env-changed=KF6_INCLUDE_DIR");
    for framework in KF6_FRAMEWORKS {
        println!("cargo:rustc-link-lib=KF6{framework}");
    }
}
