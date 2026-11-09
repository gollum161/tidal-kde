#!/bin/bash
# Builds the .pkg.tar.zst with packaging/PKGBUILD (runs in archlinux:base-devel)
# and installs it for a smoke test.
. "$(dirname "$0")/common.sh"

pacman -Syu --noconfirm --needed rust qt6-base qt6-declarative kirigami qqc2-desktop-style \
	kcolorscheme kwindowsystem mpv

# makepkg refuses to run as root
useradd -m builder
chown -R builder: .
su builder -c "cd packaging && makepkg --noconfirm"
cp "packaging/$PKG.pkg.tar.zst" dist/

pacman -U --noconfirm "dist/$PKG.pkg.tar.zst"
packaging/ci/smoke-test.sh tidal-kde
