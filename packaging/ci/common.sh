#!/bin/bash
# Shared settings for the CI scripts. Run them from the project root.
set -euo pipefail

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
PKGREL="$(sed -n 's/^pkgrel=//p' packaging/PKGBUILD)"
ARCH=x86_64
# File names follow <name>-<version>-<release>-<arch>.<ext>
PKG="tidal-kde-$VERSION-$PKGREL-$ARCH"

# A release tag must match the version in Cargo.toml.
if [ "${GITHUB_REF_TYPE:-}" = tag ] && [ "${GITHUB_REF_NAME:-}" != "v$VERSION" ]; then
	echo "Tag ${GITHUB_REF_NAME} doesn't match Cargo.toml version $VERSION" >&2
	exit 1
fi

mkdir -p dist
