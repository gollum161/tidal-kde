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
  `LyricsPage.qml`, `LoginPage.qml`, `PlayerBar.qml`, `CoverImage.qml`, `SettingsPage.qml`
  (color scheme, background opacity, blur, close-to-tray), `TrayIcon.qml` (Qt.labs.platform
  SystemTrayIcon; uses `icon.source` from qrc because labs ignores `source` when `name` is set).
- `src/appearance.rs` – lists installed KDE color schemes (`*.colors` in XDG data dirs) and wraps
  the C functions in app.cpp: `KColorSchemeManager` (scheme by file id, autosave off — the choice
  lives in settings.json) and `KWindowEffects` blur. Links KF6ColorScheme + KF6WindowSystem
  (headers from `/usr/include/KF6/K<Name>`, override with `KF6_INCLUDE_DIR`).
- `packaging/PKGBUILD` – Arch package build (also installs the app icon).
- License: GPL-3.0-or-later (`LICENSE` = GNU's gpl-3.0.txt). Arch and Debian reference their
  common GPL copy instead of installing it; the .deb has a DEP-5 `copyright` file.
- `icons/io.github.tidal_kde.TidalKde.svg` – app icon (desktop file `Icon=` uses this id).
- `.github/workflows/packages.yml` + `packaging/ci/` – CI: `debian.sh` builds `.deb` and AppImage
  on Debian 13 (linuxdeploy + Qt plugin, Breeze icons bundled), `arch.sh` builds with the PKGBUILD,
  `smoke-test.sh` starts the app offscreen for 15 s and fails on missing QML modules/libraries,
  `test-appimage.sh` runs the AppImage on bare Arch without Qt. Releases are automatic:
  after all jobs pass on main, `release-check.sh` looks for the tag `v<version>`; if it doesn't
  exist, the release job creates it on that commit and publishes the release. So releasing =
  bump `version` (Cargo.toml) + `pkgver` (PKGBUILD) and push; never bump without being asked.
  A hand-pushed tag `v<version>` still works. Asset names: `tidal-kde-<version>-<pkgrel>-x86_64.<ext>`.
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
  observed, and saved whenever mpv reports a change (slider or MPRIS). Also color scheme,
  window opacity (0.2–1), blur and close-to-tray; the scheme is applied in `Backend::initialize`,
  before the window is shown.
- Translucency: windows have an alpha buffer (`setDefaultAlphaBuffer` in app.cpp); the window
  color carries the alpha, and `Binding`s in Main.qml/PlayerBar.qml fade Kirigami's own opaque
  backgrounds: current page → 0, page toolbar/drawer/footer → `barOverlay` (0.25) on top of the
  window alpha. (Giving the bars the same alpha as the window compounded to 75–90 % and hid the
  blur.) The settings page shows *transparency* (0–95 %, presets Off/Light/Medium/Strong);
  settings.json stores `window_opacity` = 1 − transparency.
- mpv is started with `PR_SET_PDEATHSIG` (mpv.rs) so it dies with the app on a crash/kill;
  it must be spawned from the Qt main thread for that to hold.
- KWindowEffects only knows blur is available after a Wayland roundtrip: the first query always
  says no. app.cpp asks once at startup, Main.qml re-applies after 1 s.
- Single instance: app.cpp checks D-Bus for `org.mpris.MediaPlayer2.tidal_kde` and sends MPRIS
  `Raise` instead of starting a second copy (the first may be hidden in the tray).
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
Run from a terminal to see QML errors (they include file:line).
`TIDAL_KDE_SCREENSHOT=/path.png` (+ optional `TIDAL_KDE_SCREENSHOT_DELAY` ms) saves the window and
quits — with `QT_QPA_PLATFORM=offscreen` this checks the UI without a display. Without a terminal Qt logs to the
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
- **Footer geometry with a docked GlobalDrawer.** Kirigami binds the footer's `x` to
  `-contentItem.x` (= minus the drawer width) but QQC2 sizes it like the content area, so it
  stopped short of the right edge. Main.qml sets `width: root.width - x` and
  `leftPadding: -x (+ drawer width only if the drawer still covers the footer, as in older
  Kirigami)`. Kirigami 6.31 ends the docked drawer above the footer.

## Status

- The Rust backend was tested at runtime (queue operations, repeat, volume, quality,
  MPRIS registration, mpv cleanup). The Kirigami UI is only tested on the user's machine.
- Live TIDAL calls (login, browsing, playback, lyrics) need checking against the real service.
- "Saved playlists" use an older v1 endpoint and are skipped silently if it fails.
- Quality tops out at 16-bit FLAC (Hi-Res uses MPEG-DASH, not supported).
- Not done yet: drag-and-drop queue reordering, an About page, a Windows build (.exe: needs mpv over named pipes,
  Kirigami built in CI, bundled mpv.exe, no Breeze style).
