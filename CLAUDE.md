# Tidal KDE

Unofficial TIDAL music client for KDE Plasma, written in Rust with a Kirigami (QML) UI.
Features: search and library browsing, a play queue, time-synced lyrics, MPRIS media controls.
Target platform: Arch Linux, KDE Plasma 6, Qt 6, KF6 Kirigami.

## Layout

- `src/tidal.rs` – TIDAL API client (ureq, blocking). OAuth device login, token refresh,
  search/favorites/album/artist/playlist pages, stream URLs (`urlpostpaywall`), lyrics, favorites.
  Endpoints mirror python-tidalapi. Client credentials can be overridden in
  `~/.config/tidal-kde/client.json`.
- `src/mpv.rs` – mpv as the audio engine, driven over its JSON IPC socket
  (`--no-config --load-scripts=no` so user mpv scripts don't register a second MPRIS player).
- `src/bridge.rs` – cxx-qt 0.7 bridge. QML singleton `Backend` (module `org.tidalkde`):
  queue, playback state, lyrics sync, login, MPRIS via `souvlaki` (zbus).
  `#[auto_cxx_name]` makes all names camelCase in QML (`play_tracks` → `playTracks`).
- `src/main.rs` – calls `tidal_kde_run()` from `cpp/app.cpp`.
- `cpp/app.cpp` – creates a `QApplication` (needed for the Breeze `org.kde.desktop` Qt Quick style,
  which cxx-qt-lib doesn't wrap) and loads `qrc:/qt/qml/org/tidalkde/qml/Main.qml`.
  `TIDAL_KDE_QML=/path/file.qml` loads a different root file (for testing).
- `qml/` – Kirigami UI: `Main.qml` (window, global drawer sidebar, page stack, footer player bar),
  `ItemListPage.qml` (search, library and album/artist/playlist pages), `QueuePage.qml`,
  `LyricsPage.qml`, `LoginPage.qml`, `PlayerBar.qml`, `CoverImage.qml`.
- `packaging/PKGBUILD` – Arch package build (also installs `LICENSE` and the app icon).
- `icons/io.github.tidal_kde.TidalKde.svg` – app icon (desktop file `Icon=` uses this id).
- `.github/workflows/packages.yml` + `packaging/ci/` – CI: `debian.sh` builds `.deb` and AppImage
  on Debian 13 (linuxdeploy + Qt plugin, Breeze icons bundled), `arch.sh` builds with the PKGBUILD,
  `smoke-test.sh` starts the app offscreen for 15 s and fails on missing QML modules/libraries,
  `test-appimage.sh` runs the AppImage on bare Arch without Qt. A tag `v<version>` (must match
  Cargo.toml) publishes a GitHub release. Asset names: `tidal-kde-<version>-<pkgrel>-x86_64.<ext>`.
  `packaging/appimage/runtime-imports.qml` lists QML modules loaded at runtime (Breeze style)
  so linuxdeploy bundles them.
- Version: only in `Cargo.toml` (+ `pkgver` in the PKGBUILD); `build.rs` passes it to `app.cpp`
  as `TIDAL_KDE_VERSION`.
- `README.md` – user-facing docs (English, includes the vibe-coded notice). Keep its feature list,
  file paths and limitations in sync when behaviour changes.

## Data flow

- Rust → QML lists are passed as JSON strings (`queueJson`, `lyricsJson`, `pageLoaded(request, payload)`);
  QML `JSON.parse`s them. Page rows share one uniform shape (`Row` in tidal.rs) so a `ListModel` can hold them.
- QML → Rust: tracks go back as JSON strings (`Backend.playTracks(JSON.stringify(tracks), index)`).
- Persistent settings live in `~/.config/tidal-kde/settings.json` (`Settings` in bridge.rs:
  streaming quality and volume). The volume is restored into mpv before its `volume` property is
  observed, and saved whenever mpv reports a change (slider or MPRIS).
- Background work runs on `std::thread`s and returns results via `qt_thread().queue(...)`.
  Generation counters (`gen`, `lyrics_gen`) drop stale results.

## Build

```bash
# Package and install (always from packaging/):
cd packaging && makepkg -sif

# Quick dev build:
QMAKE=/usr/bin/qmake6 cargo build --release && ./target/release/tidal-kde
```

QML changes are compiled into the binary: `touch build.rs` if a rebuild doesn't pick them up.
Run from a terminal to see QML errors (they include file:line). Without a terminal Qt logs to the
journal; `QT_FORCE_STDERR_LOGGING=1` forces stderr. The startup warning "Created graphical object
was not placed in the graphics scene" comes from Kirigami's `pageStack.push(Component)` and is harmless.

## Known gotchas (already fixed — don't reintroduce)

- **No GCC LTO.** makepkg adds `-flto` by default; Rust links with lld, which can't read GCC LTO
  objects → `undefined symbol` errors. PKGBUILD has `options=('!lto')`; build.rs fails early if
  `CFLAGS`/`CXXFLAGS` contain `-flto`.
- **PKGBUILD lives in `packaging/`.** makepkg uses `./src` as scratch space; next to the project root
  it would collide with the Rust `src/` folder (and `makepkg --cleanbuild` would delete it).
- **Don't declare QML properties that Qt already defines as FINAL** (e.g. `header` on a page).
  `ItemListPage` uses `detail` for the album/artist header data for this reason.
- **The docked GlobalDrawer covers the window footer.** The `PlayerBar` footer gets
  `leftPadding` equal to the drawer width when the drawer is docked (`!drawer.modal`).

## Status

- The Rust backend was tested at runtime (queue operations, repeat, volume, quality,
  MPRIS registration, mpv cleanup). The Kirigami UI is only tested on the user's machine.
- Live TIDAL calls (login, browsing, playback, lyrics) need checking against the real service.
- "Saved playlists" use an older v1 endpoint and are skipped silently if it fails.
- Quality tops out at 16-bit FLAC (Hi-Res uses MPEG-DASH, not supported).
- Not done yet: drag-and-drop queue reordering, a tray icon or background playback after
  closing the window, an About page, a Windows build (.exe: needs mpv over named pipes,
  Kirigami built in CI, bundled mpv.exe, no Breeze style).
