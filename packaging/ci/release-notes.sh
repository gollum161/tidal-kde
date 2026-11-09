#!/bin/bash
# Writes the GitHub release text to release-notes.md
. "$(dirname "$0")/common.sh"

cat > release-notes.md <<TXT
## Tidal KDE $VERSION

Unofficial TIDAL music client for KDE Plasma 6: search and library browsing, a play queue,
time-synced lyrics and MPRIS media controls. You need a TIDAL subscription.

> **Vibe-coded:** this app was written largely by an AI assistant (Claude) with a human testing
> and reviewing it. Expect rough edges. Not affiliated with TIDAL.

| System | File |
|---|---|
| **Arch Linux, Manjaro, EndeavourOS, CachyOS** | \`sudo pacman -U $PKG.pkg.tar.zst\` |
| **Debian 13, Ubuntu 25.04 or newer** | \`sudo apt install ./$PKG.deb\` |
| **Other Linux systems** | Download \`$PKG.AppImage\`, make it executable and start it |

The AppImage needs **mpv** installed on the system and glibc 2.41 or newer
(e.g. Fedora 42+, openSUSE Tumbleweed). There is no Windows version yet.
TXT

# Release title for the workflow
[ -n "${GITHUB_OUTPUT:-}" ] && echo "name=Tidal KDE $VERSION" >> "$GITHUB_OUTPUT"
true
