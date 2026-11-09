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

fn main() {
    reject_gcc_lto_flags();

    CxxQtBuilder::new()
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
            ],
            ..Default::default()
        })
        .cc_builder(|cc| {
            let version = format!("\"{}\"", env!("CARGO_PKG_VERSION"));
            cc.define("TIDAL_KDE_VERSION", version.as_str());
            cc.file("cpp/app.cpp");
        })
        .build();
    println!("cargo:rerun-if-changed=cpp/app.cpp");
}
