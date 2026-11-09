#!/bin/bash
# Runs the AppImage on a bare Arch system without Qt or Kirigami, to check that
# everything it needs is bundled. Runs in archlinux:base.
. "$(dirname "$0")/common.sh"

# What an AppImage may expect from the host: glibc, OpenGL, fonts — and mpv.
pacman -Syu --noconfirm --needed mpv libglvnd fontconfig ttf-dejavu
chmod +x "dist/$PKG.AppImage"
APPIMAGE_EXTRACT_AND_RUN=1 packaging/ci/smoke-test.sh "dist/$PKG.AppImage"
