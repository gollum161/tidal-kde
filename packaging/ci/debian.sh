#!/bin/bash
# Builds the .deb and the AppImage on Debian 13 (runs in debian:trixie).
# The .deb is installed afterwards for a smoke test; the AppImage is tested
# separately on a system without Qt (see the workflow).
. "$(dirname "$0")/common.sh"

LINUXDEPLOY_VERSION=1-alpha-20251107-1
LINUXDEPLOY_QT_VERSION=1-alpha-20250213-1
APPIMAGETOOL_VERSION=1.9.1

# Runtime QML modules. Kirigami and the Qt Quick modules are required;
# the Breeze style (org.kde.desktop) is used when present.
QML_RUNTIME="qml6-module-org-kde-kirigami qml6-module-qtquick-controls qml6-module-qtquick-layouts \
qml6-module-qtquick-templates qml6-module-qtquick-window qml6-module-qtqml-workerscript \
qml6-module-qt-labs-platform"

export DEBIAN_FRONTEND=noninteractive
apt-get update
# shellcheck disable=SC2086
apt-get install -y --no-install-recommends \
	build-essential ca-certificates curl file pkg-config \
	qmake6 qt6-base-dev qt6-declarative-dev qt6-declarative-dev-tools \
	libkf6colorscheme-dev libkf6windowsystem-dev \
	qt6-wayland qt6-svg-plugins qml6-module-org-kde-desktop breeze-icon-theme kde-style-breeze-data \
	$QML_RUNTIME

curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
. "$HOME/.cargo/env"

export QMAKE=/usr/bin/qmake6
cargo build --release --locked
bin=target/release/tidal-kde
app_id=io.github.tidal_kde.TidalKde

install_files() { # <root>
	install -Dm755 "$bin" "$1/usr/bin/tidal-kde"
	install -Dm644 "$app_id.desktop" "$1/usr/share/applications/$app_id.desktop"
	install -Dm644 "icons/$app_id.svg" "$1/usr/share/icons/hicolor/scalable/apps/$app_id.svg"
}

# --- .deb ----------------------------------------------------------------------
root="$(mktemp -d)/deb"
install_files "$root"
install -Dm644 LICENSE "$root/usr/share/doc/tidal-kde/copyright"

# Library dependencies, as Debian's own packages compute them
shlib_dir="$(mktemp -d)"
mkdir "$shlib_dir/debian"
printf 'Source: tidal-kde\n\nPackage: tidal-kde\nArchitecture: any\n' > "$shlib_dir/debian/control"
shlibs="$(cd "$shlib_dir" && dpkg-shlibdeps -O "$root/usr/bin/tidal-kde" | sed 's/^shlibs:Depends=//')"

mkdir -p "$root/DEBIAN"
cat > "$root/DEBIAN/control" <<CONTROL
Package: tidal-kde
Version: $VERSION-$PKGREL
Architecture: amd64
Maintainer: gollum161 <gollum161@users.noreply.github.com>
Installed-Size: $(du -sk "$root/usr" | cut -f1)
Depends: $shlibs, mpv, $(echo $QML_RUNTIME | sed 's/ /, /g')
Recommends: qml6-module-org-kde-desktop, breeze-icon-theme, kde-style-breeze-data
Section: sound
Priority: optional
Homepage: https://github.com/gollum161/tidal-kde
Description: Unofficial TIDAL music client for KDE Plasma
 Kirigami (Qt 6) client for TIDAL with search and library browsing,
 a play queue, time-synced lyrics and MPRIS media controls.
 Not affiliated with TIDAL. Requires a TIDAL subscription.
CONTROL
dpkg-deb --root-owner-group -Zxz --build "$root" "dist/$PKG.deb"

# --- AppImage ------------------------------------------------------------------
tools="$(mktemp -d)"
curl -fsSL -o "$tools/linuxdeploy" \
	"https://github.com/linuxdeploy/linuxdeploy/releases/download/$LINUXDEPLOY_VERSION/linuxdeploy-x86_64.AppImage"
curl -fsSL -o "$tools/linuxdeploy-plugin-qt" \
	"https://github.com/linuxdeploy/linuxdeploy-plugin-qt/releases/download/$LINUXDEPLOY_QT_VERSION/linuxdeploy-plugin-qt-x86_64.AppImage"
curl -fsSL -o "$tools/appimagetool" \
	"https://github.com/AppImage/appimagetool/releases/download/$APPIMAGETOOL_VERSION/appimagetool-x86_64.AppImage"
chmod +x "$tools"/*
export PATH="$tools:$PATH" APPIMAGE_EXTRACT_AND_RUN=1

appdir="$(mktemp -d)/AppDir"
install_files "$appdir"
# Breeze icons for desktops without them (the app falls back to this theme)
mkdir -p "$appdir/usr/share/icons"
cp -r /usr/share/icons/breeze "$appdir/usr/share/icons/"
# Breeze colour schemes for the theme setting (found through XDG_DATA_DIRS, see app.cpp)
cp -r /usr/share/color-schemes "$appdir/usr/share/"
# KWindowSystem's Wayland/X11 plugins (window blur); their libraries are deployed below
kwsdir="$appdir/usr/plugins/kf6/kwindowsystem"
mkdir -p "$kwsdir"
cp /usr/lib/x86_64-linux-gnu/qt6/plugins/kf6/kwindowsystem/*.so "$kwsdir/"

# The plugin bundles every QML module imported by these files (incl. the Breeze style,
# which the app loads at runtime, see packaging/appimage/runtime-imports.qml).
export QML_SOURCES_PATHS="$PWD/qml:$PWD/packaging/appimage"
export EXTRA_PLATFORM_PLUGINS="libqwayland-generic.so;libqwayland-egl.so;libqoffscreen.so"
export EXTRA_QT_MODULES="svg"
linuxdeploy --appdir "$appdir" \
	--executable "$appdir/usr/bin/tidal-kde" \
	--desktop-file "$appdir/usr/share/applications/$app_id.desktop" \
	--icon-file "$appdir/usr/share/icons/hicolor/scalable/apps/$app_id.svg" \
	--deploy-deps-only "$kwsdir" \
	--plugin qt
ARCH=x86_64 appimagetool --no-appstream "$appdir" "dist/$PKG.AppImage"

# --- smoke test of the .deb ----------------------------------------------------
apt-get install -y "./dist/$PKG.deb"
packaging/ci/smoke-test.sh tidal-kde

ls -l dist
