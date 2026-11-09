# Tidal KDE

An unofficial [TIDAL](https://tidal.com) music client for KDE Plasma 6, written in Rust with a
Kirigami (QML) user interface.

> [!WARNING]
> **This project is vibe-coded.** It was written largely by an AI assistant (Claude), with a human
> steering, testing and reviewing the result. It works on the author's machine, but expect rough
> edges, and read the code before you trust it with anything important. Bug reports and pull
> requests are welcome.

> [!NOTE]
> This is not an official TIDAL product and is not affiliated with, endorsed by or supported by
> TIDAL. You need your own paid TIDAL subscription. Use it at your own risk and in line with
> TIDAL's terms of service.

## Features

- Sign in with your TIDAL account (OAuth device login, the session is remembered)
- Search, and browse your library: favorite tracks, albums, artists and playlists
- Album, artist and playlist pages; add tracks, albums, artists and playlists to your favorites
- Play queue: play next, add to queue, shuffle upcoming tracks, repeat (off / all / one)
- Time-synced lyrics that follow playback (when TIDAL has them)
- MPRIS media controls: media keys, the Plasma media applet and the lock screen all work
- Streaming quality setting (up to 16-bit / 44.1 kHz FLAC)
- System tray icon: closing the window keeps the music playing (can be turned off)
- Themes: pick any installed KDE color scheme (Breeze, Breeze Dark, …) or follow the system
- Adjustable background opacity, optionally with the background blurred (KWin)
- All settings (volume, quality, theme, opacity, tray) are remembered between sessions
- Native look on Plasma through the Breeze Qt Quick style
- Only one instance: starting the app again brings the running window back

## Requirements

Tidal KDE is made for **KDE Plasma 6** and developed on Arch Linux. There are ready-made packages
for Arch Linux, for Debian and Ubuntu, and an AppImage for other distributions (see
[Installation](#installation)). Every package is built and started on a fresh system before a
release. Other desktops should work too but aren't tested; the tray icon and window blur need a
desktop that supports them.

You need a **TIDAL subscription**. The packages pull in everything else; if you build from
source, these are the pieces:

| Component | Arch Linux | Debian / Ubuntu |
| --- | --- | --- |
| Qt 6 | `qt6-base`, `qt6-declarative` | `qt6-base-dev`, `qt6-declarative-dev`, `qmake6` |
| Kirigami | `kirigami` | `qml6-module-org-kde-kirigami` |
| Breeze style for Qt Quick (optional) | `qqc2-desktop-style` | `qml6-module-org-kde-desktop` |
| Tray icon | part of `qt6-declarative` | `qml6-module-qt-labs-platform` |
| KDE color schemes, window blur | `kcolorscheme`, `kwindowsystem` | `libkf6colorscheme-dev`, `libkf6windowsystem-dev` |
| Audio engine | `mpv` | `mpv` |
| Build tools | `rust` (1.80+), `base-devel` | Rust 1.80+ ([rustup](https://rustup.rs)), `build-essential` |

On Debian/Ubuntu you also need the Qt Quick modules `qml6-module-qtquick-controls`,
`qml6-module-qtquick-layouts`, `qml6-module-qtquick-templates`, `qml6-module-qtquick-window` and
`qml6-module-qtqml-workerscript` at runtime.

## Installation

Download a package from the [latest release](https://github.com/gollum161/tidal-kde/releases/latest):

| System | File | Install |
| --- | --- | --- |
| Arch Linux, Manjaro, EndeavourOS, CachyOS | `.pkg.tar.zst` | `sudo pacman -U tidal-kde-*.pkg.tar.zst` |
| Debian 13, Ubuntu 25.04 or newer | `.deb` | `sudo apt install ./tidal-kde-*.deb` |
| Other Linux systems | `.AppImage` | `chmod +x tidal-kde-*.AppImage`, then run it |

The AppImage bundles Qt, Kirigami, the Breeze style, Breeze icons and the Breeze color schemes,
but not mpv: install `mpv` with your package manager. It needs glibc 2.41 or newer (e.g. Fedora 42+,
openSUSE Tumbleweed).

There is no Windows build yet.

### Build the Arch package yourself

```bash
git clone https://github.com/gollum161/tidal-kde.git
cd tidal-kde/packaging
makepkg -si
```

The `PKGBUILD` lives in `packaging/` on purpose: makepkg uses `./src` as scratch space, which
would clash with the Rust `src/` folder in the project root.

## Development build

```bash
QMAKE=/usr/bin/qmake6 cargo build --release
./target/release/tidal-kde
```

The first build compiles all dependencies and takes a few minutes. QML files are compiled into the
binary; if a QML change isn't picked up, run `touch build.rs` and build again. Start the app from a
terminal to see QML errors with file and line numbers.

> [!IMPORTANT]
> Don't build with GCC LTO (`-flto` in `CFLAGS`/`CXXFLAGS`). Rust links with lld, which can't read
> GCC LTO objects. The PKGBUILD disables LTO, and `build.rs` stops early with a clear message if
> it finds `-flto`.

## Usage

1. Start **Tidal KDE** from the application launcher.
2. Click **Log in with TIDAL**. Your browser opens TIDAL's login page; check that it shows the same
   code as the app, then approve the device.
3. Search or browse your library and click a track to play it. Use the context menu for
   *Play next*, *Add to queue*, *Add to favorites* and *Go to album/artist*.

## Files

| Path | Contents |
| --- | --- |
| `~/.config/tidal-kde/session.json` | Login tokens (readable by you only). Delete it to sign out. |
| `~/.config/tidal-kde/settings.json` | Streaming quality, volume, color scheme, opacity, blur, tray |
| `~/.config/tidal-kde/client.json` | Optional: your own API client credentials (see below) |
| `~/.cache/tidal-kde/` | Cached cover art |

### Client credentials

Like other open-source TIDAL clients, Tidal KDE uses TIDAL's public device-client credentials (the
same ones as [python-tidalapi](https://github.com/tamland/python-tidal)). If TIDAL rotates them and
login stops working, you can supply your own without rebuilding:

```json
{ "client_id": "...", "client_secret": "..." }
```

Save this as `~/.config/tidal-kde/client.json`.

## Known limitations

- Quality tops out at 16-bit FLAC. Hi-Res (24-bit) is streamed by TIDAL as MPEG-DASH, which isn't
  supported yet.
- "Saved playlists" (playlists by other users that you follow) use an older API endpoint. If it
  fails, they are silently skipped.
- Without a system tray (e.g. GNOME without an AppIndicator extension), closing the window quits.
- Blur behind the window needs a compositor that supports it (KWin on KDE Plasma).
- The queue can't be reordered by drag and drop yet.
- Linux only for now.

## Releases

[`.github/workflows/packages.yml`](.github/workflows/packages.yml) builds the `.deb`, the AppImage
(both on Debian 13) and the Arch package on every push, and starts each one in a fresh container
as a smoke test. When everything passes on `main` and the version in `Cargo.toml` has no tag yet,
the workflow tags that commit `v<version>` and publishes the packages as a GitHub release. So a
release is: raise `version` in `Cargo.toml` and `pkgver` in `packaging/PKGBUILD`, commit, push. The build scripts live in [`packaging/ci/`](packaging/ci/).

## How it works

- `src/tidal.rs`: TIDAL API client (blocking HTTP via `ureq`); endpoints mirror python-tidalapi
- `src/mpv.rs`: starts mpv and talks to it over its JSON IPC socket
- `src/bridge.rs`: the [cxx-qt](https://github.com/KDAB/cxx-qt) bridge that exposes a `Backend`
  singleton to QML (queue, playback state, lyrics, login, MPRIS via
  [souvlaki](https://github.com/Sinono3/souvlaki))
- `cpp/app.cpp`: creates the `QApplication` and loads the QML UI
- `qml/`: the Kirigami interface

## Credits

- [python-tidalapi](https://github.com/tamland/python-tidal), the reference for the TIDAL API
- [cxx-qt](https://github.com/KDAB/cxx-qt) by KDAB, for Rust ↔ Qt bindings
- [Kirigami](https://develop.kde.org/frameworks/kirigami/) by KDE
- [mpv](https://mpv.io) for audio playback
- [souvlaki](https://github.com/Sinono3/souvlaki) for MPRIS

## License

Copyright © 2026 gollum161

Tidal KDE is free software: you can redistribute it and/or modify it under the terms of the
[GNU General Public License](LICENSE) as published by the Free Software Foundation, either
version 3 of the License, or (at your option) any later version (GPL-3.0-or-later).

It is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the
implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
