//! The QML-facing backend: playback, queue, lyrics, login and MPRIS.
//! Exposed to QML as the singleton `Backend` in the `org.tidalkde` module.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(bool, logged_in)]
        #[qproperty(bool, connecting)]
        #[qproperty(QString, login_url)]
        #[qproperty(QString, login_code)]
        #[qproperty(QString, title)]
        #[qproperty(QString, artist)]
        #[qproperty(QString, album)]
        #[qproperty(QString, cover)]
        #[qproperty(f64, position)]
        #[qproperty(f64, duration)]
        #[qproperty(bool, playing)]
        #[qproperty(bool, active)]
        #[qproperty(f64, volume)]
        #[qproperty(i32, repeat_mode)]
        #[qproperty(QString, queue_json)]
        #[qproperty(i32, queue_index)]
        #[qproperty(QString, lyrics_json)]
        #[qproperty(QString, lyrics_state)]
        #[qproperty(QString, lyrics_provider)]
        #[qproperty(bool, lyrics_synced)]
        #[qproperty(i32, lyrics_index)]
        #[qproperty(QString, quality)]
        // Appearance and behaviour settings (saved in settings.json)
        #[qproperty(QString, color_scheme)]
        #[qproperty(QString, color_schemes_json)]
        #[qproperty(f64, window_opacity)]
        #[qproperty(bool, blur)]
        #[qproperty(bool, blur_available)]
        #[qproperty(bool, close_to_tray)]
        type Backend = super::BackendRust;

        #[qinvokable]
        fn start_login(self: Pin<&mut Backend>);
        #[qinvokable]
        fn logout(self: Pin<&mut Backend>);
        #[qinvokable]
        fn load(self: Pin<&mut Backend>, request: i32, what: &QString, arg: &QString);
        #[qinvokable]
        fn play_tracks(self: Pin<&mut Backend>, tracks: &QString, start: i32);
        #[qinvokable]
        fn enqueue(self: Pin<&mut Backend>, tracks: &QString);
        #[qinvokable]
        fn play_next(self: Pin<&mut Backend>, tracks: &QString);
        #[qinvokable]
        fn queue_play(self: Pin<&mut Backend>, index: i32);
        #[qinvokable]
        fn queue_move(self: Pin<&mut Backend>, from: i32, to: i32);
        #[qinvokable]
        fn queue_remove(self: Pin<&mut Backend>, index: i32);
        #[qinvokable]
        fn queue_clear(self: Pin<&mut Backend>);
        #[qinvokable]
        fn shuffle_upcoming(self: Pin<&mut Backend>);
        #[qinvokable]
        fn toggle_pause(self: Pin<&mut Backend>);
        #[qinvokable]
        fn next(self: Pin<&mut Backend>);
        #[qinvokable]
        fn previous(self: Pin<&mut Backend>);
        #[qinvokable]
        fn seek(self: Pin<&mut Backend>, seconds: f64);
        #[qinvokable]
        fn change_volume(self: Pin<&mut Backend>, value: f64);
        #[qinvokable]
        fn cycle_repeat(self: Pin<&mut Backend>);
        #[qinvokable]
        fn like(self: Pin<&mut Backend>, kind: &QString, id: &QString);
        #[qinvokable]
        fn set_stream_quality(self: Pin<&mut Backend>, quality: &QString);
        /// Empty id = follow the system colour scheme.
        #[qinvokable]
        fn change_color_scheme(self: Pin<&mut Backend>, id: &QString);
        /// Background opacity of the window, 0.2–1.0.
        #[qinvokable]
        fn change_window_opacity(self: Pin<&mut Backend>, value: f64);
        #[qinvokable]
        fn change_blur(self: Pin<&mut Backend>, enable: bool);
        #[qinvokable]
        fn change_close_to_tray(self: Pin<&mut Backend>, enable: bool);
        /// Called by the window once it exists (blur needs a native window).
        #[qinvokable]
        fn apply_window_effects(self: Pin<&mut Backend>);

        #[qsignal]
        fn page_loaded(self: Pin<&mut Backend>, request: i32, payload: QString);
        #[qsignal]
        fn page_failed(self: Pin<&mut Backend>, request: i32, message: QString);
        #[qsignal]
        fn show_message(self: Pin<&mut Backend>, message: QString);
        #[qsignal]
        fn raise_requested(self: Pin<&mut Backend>);
        #[qsignal]
        fn quit_requested(self: Pin<&mut Backend>);
    }

    impl cxx_qt::Threading for Backend {}
    impl cxx_qt::Initialize for Backend {}
}

use crate::appearance;
use crate::mpv::Mpv;
use crate::tidal::{config_dir, Lyrics, Tidal, Track};
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition,
               PlatformConfig, SeekDirection};
use std::sync::Arc;
use std::time::Duration;

const QUALITIES: [&str; 3] = ["LOW", "HIGH", "LOSSLESS"];
const REPEAT_OFF: i32 = 0;
const REPEAT_ALL: i32 = 1;
const REPEAT_ONE: i32 = 2;

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    quality: String,
    /// 0–100, restored into mpv at startup.
    volume: f64,
    /// KDE colour scheme id (file name without `.colors`); empty = system.
    color_scheme: String,
    /// Background opacity of the window, MIN_WINDOW_OPACITY–1.0.
    window_opacity: f64,
    /// Blur behind a translucent window (KWin).
    blur: bool,
    /// Closing the window hides it to the tray and keeps playing.
    close_to_tray: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            quality: "LOSSLESS".into(),
            volume: 100.0,
            color_scheme: String::new(),
            window_opacity: 1.0,
            blur: true,
            close_to_tray: true,
        }
    }
}

const MIN_WINDOW_OPACITY: f64 = 0.2;

fn load_settings() -> Settings {
    std::fs::read_to_string(config_dir().join("settings.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_settings(settings: &Settings) {
    let _ = std::fs::create_dir_all(config_dir());
    let _ = std::fs::write(config_dir().join("settings.json"),
                           serde_json::to_string(settings).unwrap_or_default());
}

fn qs(text: &str) -> QString {
    QString::from(text)
}

pub struct Core {
    tidal: Arc<Tidal>,
    mpv: Option<Mpv>,
    controls: Option<MediaControls>,
    queue: Vec<Track>,
    pos: i32,
    gen: u64,
    loading: bool,
    paused: bool,
    lyric_times: Vec<f64>,
    lyrics_gen: u64,
    settings: Settings,
    art: Option<String>,
    mpris_second: i64,
}

impl Core {
    fn current(&self) -> Option<&Track> {
        usize::try_from(self.pos).ok().and_then(|i| self.queue.get(i))
    }
}

pub struct BackendRust {
    logged_in: bool,
    connecting: bool,
    login_url: QString,
    login_code: QString,
    title: QString,
    artist: QString,
    album: QString,
    cover: QString,
    position: f64,
    duration: f64,
    playing: bool,
    active: bool,
    volume: f64,
    repeat_mode: i32,
    queue_json: QString,
    queue_index: i32,
    lyrics_json: QString,
    lyrics_state: QString,
    lyrics_provider: QString,
    lyrics_synced: bool,
    lyrics_index: i32,
    quality: QString,
    color_scheme: QString,
    color_schemes_json: QString,
    window_opacity: f64,
    blur: bool,
    blur_available: bool,
    close_to_tray: bool,
    core: Core,
}

impl Default for BackendRust {
    fn default() -> Self {
        let mut settings = load_settings();
        settings.volume = settings.volume.clamp(0.0, 100.0);
        settings.window_opacity = settings.window_opacity.clamp(MIN_WINDOW_OPACITY, 1.0);
        let schemes = appearance::color_schemes();
        if !schemes.iter().any(|s| s.id == settings.color_scheme) {
            settings.color_scheme.clear(); // uninstalled scheme → system
        }
        BackendRust {
            logged_in: false,
            connecting: true,
            login_url: QString::default(),
            login_code: QString::default(),
            title: QString::default(),
            artist: QString::default(),
            album: QString::default(),
            cover: QString::default(),
            position: 0.0,
            duration: 0.0,
            playing: false,
            active: false,
            volume: settings.volume,
            repeat_mode: REPEAT_OFF,
            queue_json: qs("[]"),
            queue_index: -1,
            lyrics_json: qs("[]"),
            lyrics_state: qs("idle"),
            lyrics_provider: QString::default(),
            lyrics_synced: false,
            lyrics_index: -1,
            quality: qs(&settings.quality),
            color_scheme: qs(&settings.color_scheme),
            color_schemes_json: qs(&serde_json::to_string(&schemes).unwrap_or_else(|_| "[]".into())),
            window_opacity: settings.window_opacity,
            blur: settings.blur,
            blur_available: false,
            close_to_tray: settings.close_to_tray,
            core: Core {
                tidal: Arc::new(Tidal::new()),
                mpv: None,
                controls: None,
                queue: Vec::new(),
                pos: -1,
                gen: 0,
                loading: false,
                paused: false,
                lyric_times: Vec::new(),
                lyrics_gen: 0,
                settings,
                art: None,
                mpris_second: -1,
            },
        }
    }
}

impl cxx_qt::Initialize for qobject::Backend {
    fn initialize(mut self: Pin<&mut Self>) {
        let thread = self.qt_thread();

        // The singleton is created while the QML loads, before the window is
        // shown, so the saved colour scheme is in place from the first frame.
        if !self.core().settings.color_scheme.is_empty() {
            appearance::apply_color_scheme(&self.core().settings.color_scheme);
        }

        let t = thread.clone();
        match Mpv::spawn(move |msg| {
            let _ = t.queue(move |b| b.on_mpv(msg));
        }) {
            Ok(mpv) => {
                // Restore the saved volume before observing it, so the first
                // "volume" event already carries the restored value.
                mpv.command(json!(["set_property", "volume", self.core().settings.volume]));
                for (i, prop) in ["time-pos", "duration", "pause", "volume", "idle-active"].iter().enumerate() {
                    mpv.command(json!(["observe_property", i + 1, prop]));
                }
                self.as_mut().core_mut().mpv = Some(mpv);
            }
            Err(e) => eprintln!("[tidal-kde] {e}"),
        }

        let config = PlatformConfig { dbus_name: "tidal_kde", display_name: "Tidal KDE", hwnd: None };
        match MediaControls::new(config) {
            Ok(mut controls) => {
                let t = thread.clone();
                if controls.attach(move |event| { let _ = t.queue(move |b| b.on_media(event)); }).is_ok() {
                    self.as_mut().core_mut().controls = Some(controls);
                }
            }
            Err(e) => eprintln!("[tidal-kde] media controls unavailable: {e:?}"),
        }

        let tidal = self.core().tidal.clone();
        std::thread::spawn(move || {
            let ok = tidal.restore().unwrap_or(false);
            let _ = thread.queue(move |mut b| {
                b.as_mut().set_logged_in(ok);
                b.as_mut().set_connecting(false);
            });
        });
    }
}

impl qobject::Backend {
    // ─── small helpers ───────────────────────────────────────────────────────

    fn core(&self) -> &Core {
        &self.rust().core
    }

    fn core_mut(self: Pin<&mut Self>) -> &mut Core {
        &mut self.rust_mut().get_mut().core
    }

    fn mpv(&self, args: Value) {
        if let Some(mpv) = &self.core().mpv {
            mpv.command(args);
        }
    }

    fn message(self: Pin<&mut Self>, text: impl AsRef<str>) {
        self.show_message(qs(text.as_ref()));
    }

    fn busy(&self) -> bool {
        self.core().loading || *self.active()
    }

    fn parse_tracks(json: &QString) -> Vec<Track> {
        serde_json::from_str(&json.to_string()).unwrap_or_default()
    }

    // ─── login ───────────────────────────────────────────────────────────────

    pub fn start_login(mut self: Pin<&mut Self>) {
        self.as_mut().set_login_code(QString::default());
        let tidal = self.core().tidal.clone();
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let login = match tidal.device_login_start() {
                Ok(login) => login,
                Err(e) => {
                    let _ = thread.queue(move |b| b.message(format!("Couldn't start login: {e}")));
                    return;
                }
            };
            let (code, url) = (login.user_code.clone(), login.url.clone());
            let _ = thread.queue(move |mut b| {
                b.as_mut().set_login_code(qs(&code));
                b.as_mut().set_login_url(qs(&url));
            });
            let result = tidal.device_login_wait(&login);
            let _ = thread.queue(move |mut b| {
                b.as_mut().set_login_code(QString::default());
                b.as_mut().set_login_url(QString::default());
                match result {
                    Ok(()) => b.as_mut().set_logged_in(true),
                    Err(e) => b.as_mut().message(e),
                }
            });
        });
    }

    pub fn logout(mut self: Pin<&mut Self>) {
        self.as_mut().queue_clear();
        self.core().tidal.logout();
        self.as_mut().set_logged_in(false);
    }

    // ─── browsing ────────────────────────────────────────────────────────────

    pub fn load(self: Pin<&mut Self>, request: i32, what: &QString, arg: &QString) {
        let (what, arg) = (what.to_string(), arg.to_string());
        let tidal = self.core().tidal.clone();
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let page = match what.as_str() {
                "search" => tidal.search(&arg),
                "tracks" | "albums" | "artists" | "playlists" => tidal.favorites(&what),
                "album" => tidal.album(&arg),
                "artist" => tidal.artist(&arg),
                "playlist" => tidal.playlist(&arg),
                other => Err(format!("Unknown page type “{other}”")),
            };
            let result = page.map(|p| serde_json::to_string(&p).unwrap_or_default());
            let _ = thread.queue(move |b| match result {
                Ok(json) => b.page_loaded(request, qs(&json)),
                Err(e) => b.page_failed(request, qs(&e)),
            });
        });
    }

    pub fn like(self: Pin<&mut Self>, kind: &QString, id: &QString) {
        let (kind, id) = (kind.to_string(), id.to_string());
        let tidal = self.core().tidal.clone();
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let result = tidal.like(&kind, &id);
            let _ = thread.queue(move |b| match result {
                Ok(()) => b.message("Added to favorites"),
                Err(e) => b.message(format!("Couldn't add to favorites: {e}")),
            });
        });
    }

    pub fn set_stream_quality(mut self: Pin<&mut Self>, quality: &QString) {
        let q = quality.to_string();
        if !QUALITIES.contains(&q.as_str()) {
            return;
        }
        self.as_mut().core_mut().settings.quality = q.clone();
        save_settings(&self.core().settings);
        self.as_mut().set_quality(qs(&q));
        self.message("Streaming quality changed. It applies from the next track.");
    }

    // ─── appearance & behaviour settings ────────────────────────────────────

    pub fn change_color_scheme(mut self: Pin<&mut Self>, id: &QString) {
        let id = id.to_string();
        appearance::apply_color_scheme(&id);
        self.as_mut().core_mut().settings.color_scheme = id.clone();
        save_settings(&self.core().settings);
        self.set_color_scheme(qs(&id));
    }

    pub fn change_window_opacity(mut self: Pin<&mut Self>, value: f64) {
        let value = value.clamp(MIN_WINDOW_OPACITY, 1.0);
        if (self.core().settings.window_opacity - value).abs() < 0.001 {
            return;
        }
        self.as_mut().core_mut().settings.window_opacity = value;
        save_settings(&self.core().settings);
        self.as_mut().set_window_opacity(value);
        self.apply_window_effects();
    }

    pub fn change_blur(mut self: Pin<&mut Self>, enable: bool) {
        self.as_mut().core_mut().settings.blur = enable;
        save_settings(&self.core().settings);
        self.as_mut().set_blur(enable);
        self.apply_window_effects();
    }

    pub fn change_close_to_tray(mut self: Pin<&mut Self>, enable: bool) {
        self.as_mut().core_mut().settings.close_to_tray = enable;
        save_settings(&self.core().settings);
        self.set_close_to_tray(enable);
    }

    /// Blur only makes sense behind a translucent window.
    pub fn apply_window_effects(mut self: Pin<&mut Self>) {
        let available = appearance::blur_available();
        self.as_mut().set_blur_available(available);
        let s = &self.core().settings;
        appearance::apply_blur(available && s.blur && s.window_opacity < 1.0);
    }

    // ─── queue ───────────────────────────────────────────────────────────────

    pub fn play_tracks(mut self: Pin<&mut Self>, tracks: &QString, start: i32) {
        let tracks = Self::parse_tracks(tracks);
        if tracks.is_empty() {
            return;
        }
        let start = start.clamp(0, tracks.len() as i32 - 1);
        self.as_mut().core_mut().queue = tracks;
        self.play_index(start);
    }

    pub fn enqueue(mut self: Pin<&mut Self>, tracks: &QString) {
        let tracks = Self::parse_tracks(tracks);
        if tracks.is_empty() {
            return;
        }
        let note = if tracks.len() == 1 {
            format!("Added “{}” to the queue", tracks[0].title)
        } else {
            format!("Added {} tracks to the queue", tracks.len())
        };
        let busy = self.busy();
        let start = self.core().queue.len() as i32;
        self.as_mut().core_mut().queue.extend(tracks);
        if busy {
            self.as_mut().sync_queue();
        } else {
            self.as_mut().play_index(start);
        }
        self.message(note);
    }

    pub fn play_next(mut self: Pin<&mut Self>, tracks: &QString) {
        let tracks = Self::parse_tracks(tracks);
        if tracks.is_empty() {
            return;
        }
        let note = format!("“{}” plays next", tracks[0].title);
        let busy = self.busy();
        let at = (self.core().pos + 1).max(0) as usize;
        let core = self.as_mut().core_mut();
        let at = at.min(core.queue.len());
        core.queue.splice(at..at, tracks);
        if busy {
            self.as_mut().sync_queue();
        } else {
            self.as_mut().play_index(at as i32);
        }
        self.message(note);
    }

    pub fn queue_play(self: Pin<&mut Self>, index: i32) {
        self.play_index(index);
    }

    pub fn queue_move(mut self: Pin<&mut Self>, from: i32, to: i32) {
        let len = self.core().queue.len() as i32;
        if from == to || !(0..len).contains(&from) || !(0..len).contains(&to) {
            return;
        }
        let core = self.as_mut().core_mut();
        let track = core.queue.remove(from as usize);
        core.queue.insert(to as usize, track);
        if core.pos == from {
            core.pos = to;
        } else if from < core.pos && core.pos <= to {
            core.pos -= 1;
        } else if to <= core.pos && core.pos < from {
            core.pos += 1;
        }
        self.sync_queue();
    }

    pub fn queue_remove(mut self: Pin<&mut Self>, index: i32) {
        let len = self.core().queue.len() as i32;
        if !(0..len).contains(&index) {
            return;
        }
        let busy = self.busy();
        let pos = self.core().pos;
        self.as_mut().core_mut().queue.remove(index as usize);
        if index < pos {
            self.as_mut().core_mut().pos -= 1;
        } else if index == pos {
            if busy && index < len - 1 {
                self.play_index(index);
                return;
            }
            if busy {
                self.as_mut().stop_playback();
            }
            self.as_mut().core_mut().pos = -1;
            self.as_mut().track_changed();
            return;
        }
        self.sync_queue();
    }

    pub fn queue_clear(mut self: Pin<&mut Self>) {
        self.as_mut().stop_playback();
        let core = self.as_mut().core_mut();
        core.queue.clear();
        core.pos = -1;
        self.track_changed();
    }

    pub fn shuffle_upcoming(mut self: Pin<&mut Self>) {
        let start = (self.core().pos + 1).max(0) as usize;
        let core = self.as_mut().core_mut();
        if start + 1 >= core.queue.len() {
            return;
        }
        fastrand::shuffle(&mut core.queue[start..]);
        self.as_mut().sync_queue();
        self.message("Shuffled upcoming tracks");
    }

    fn sync_queue(mut self: Pin<&mut Self>) {
        let list: Vec<Value> = self.core().queue.iter().map(|t| json!({
            "title": t.title, "subtitle": t.artist, "duration": t.duration, "cover": t.cover,
        })).collect();
        let pos = self.core().pos;
        self.as_mut().set_queue_json(qs(&Value::Array(list).to_string()));
        self.as_mut().set_queue_index(pos);
    }

    // ─── transport ───────────────────────────────────────────────────────────

    fn play_index(mut self: Pin<&mut Self>, index: i32) {
        let (track, gen, quality, tidal) = {
            let core = self.as_mut().core_mut();
            let Some(track) = usize::try_from(index).ok().and_then(|i| core.queue.get(i)).cloned() else {
                return;
            };
            core.pos = index;
            core.gen += 1;
            core.loading = true;
            (track, core.gen, core.settings.quality.clone(), core.tidal.clone())
        };
        self.as_mut().track_changed();
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let result = tidal.stream_url(track.id, &quality);
            let _ = thread.queue(move |b| b.stream_ready(gen, result));
        });
    }

    fn stream_ready(mut self: Pin<&mut Self>, gen: u64, result: Result<String, String>) {
        if self.core().gen != gen {
            return;
        }
        match result {
            Ok(url) => {
                self.mpv(json!(["loadfile", url, "replace"]));
                self.mpv(json!(["set_property", "pause", false]));
            }
            Err(e) => {
                let title = self.core().current().map(|t| t.title.clone()).unwrap_or_default();
                self.as_mut().core_mut().loading = false;
                // A connection problem would fail every track, so only skip ahead
                // when this particular track is the problem.
                let offline = e.starts_with("Couldn't reach TIDAL");
                self.as_mut().message(if offline { e } else { format!("Couldn't play “{title}”: {e}") });
                let (pos, len) = (self.core().pos, self.core().queue.len() as i32);
                if !offline && pos + 1 < len {
                    self.play_index(pos + 1);
                } else {
                    self.sync_playback();
                }
            }
        }
    }

    fn stop_playback(mut self: Pin<&mut Self>) {
        let core = self.as_mut().core_mut();
        core.gen += 1;
        core.loading = false;
        self.mpv(json!(["stop"]));
    }

    fn play(self: Pin<&mut Self>) {
        if self.busy() {
            self.mpv(json!(["set_property", "pause", false]));
        } else if !self.core().queue.is_empty() {
            let pos = self.core().pos.max(0);
            self.play_index(pos);
        }
    }

    fn pause(&self) {
        self.mpv(json!(["set_property", "pause", true]));
    }

    pub fn toggle_pause(self: Pin<&mut Self>) {
        if self.busy() && !self.core().paused {
            self.pause();
        } else {
            self.play();
        }
    }

    pub fn next(self: Pin<&mut Self>) {
        let (pos, len) = (self.core().pos, self.core().queue.len() as i32);
        if pos + 1 < len {
            self.play_index(pos + 1);
        } else if *self.repeat_mode() == REPEAT_ALL && len > 0 {
            self.play_index(0);
        }
    }

    pub fn previous(self: Pin<&mut Self>) {
        let pos = self.core().pos;
        if *self.position() > 3.0 || pos <= 0 {
            if self.busy() {
                self.seek(0.0);
            }
        } else {
            self.play_index(pos - 1);
        }
    }

    pub fn seek(mut self: Pin<&mut Self>, seconds: f64) {
        let seconds = seconds.max(0.0);
        self.mpv(json!(["seek", seconds, "absolute"]));
        self.as_mut().set_position(seconds);
        self.as_mut().update_lyric_index(seconds);
        self.sync_mpris();
    }

    fn seek_by(self: Pin<&mut Self>, delta: f64) {
        let target = *self.position() + delta;
        let duration = *self.duration();
        if duration > 0.0 && target >= duration {
            self.next();
        } else {
            self.seek(target);
        }
    }

    pub fn change_volume(self: Pin<&mut Self>, value: f64) {
        self.mpv(json!(["set_property", "volume", value.clamp(0.0, 100.0)]));
    }

    pub fn cycle_repeat(self: Pin<&mut Self>) {
        let next = (*self.repeat_mode() + 1) % 3;
        self.set_repeat_mode(next);
    }

    // ─── state sync ──────────────────────────────────────────────────────────

    fn track_changed(mut self: Pin<&mut Self>) {
        let track = self.core().current().cloned();
        let t = track.clone().unwrap_or_default();
        self.as_mut().set_title(qs(&t.title));
        self.as_mut().set_artist(qs(&t.artist));
        self.as_mut().set_album(qs(&t.album));
        self.as_mut().set_cover(qs(&t.cover_large));
        self.as_mut().set_duration(t.duration);
        self.as_mut().set_position(0.0);
        self.as_mut().core_mut().art = None;
        self.as_mut().sync_queue();
        self.as_mut().sync_playback();
        self.as_mut().update_mpris_metadata();
        self.as_mut().fetch_lyrics(track.clone());

        if let Some(t) = track {
            let tidal = self.core().tidal.clone();
            let thread = self.qt_thread();
            std::thread::spawn(move || {
                if let Some(art) = tidal.cached_cover(t.album_id, &t.cover_large) {
                    let _ = thread.queue(move |mut b| {
                        if b.core().current().map(|c| c.id) == Some(t.id) {
                            b.as_mut().core_mut().art = Some(art);
                            b.update_mpris_metadata();
                        }
                    });
                }
            });
        }
    }

    fn sync_playback(mut self: Pin<&mut Self>) {
        let playing = self.busy() && !self.core().paused;
        if *self.playing() != playing {
            self.as_mut().set_playing(playing);
        }
        self.sync_mpris();
    }

    fn sync_mpris(mut self: Pin<&mut Self>) {
        let busy = self.busy();
        let paused = self.core().paused;
        let progress = Some(MediaPosition(Duration::from_secs_f64(self.position().max(0.0))));
        if let Some(controls) = self.as_mut().core_mut().controls.as_mut() {
            let state = if !busy {
                MediaPlayback::Stopped
            } else if paused {
                MediaPlayback::Paused { progress }
            } else {
                MediaPlayback::Playing { progress }
            };
            let _ = controls.set_playback(state);
        }
    }

    fn update_mpris_metadata(mut self: Pin<&mut Self>) {
        let duration = self.duration().max(0.0);
        let core = self.as_mut().core_mut();
        let track = core.current().cloned();
        let art = core.art.clone();
        let Some(controls) = core.controls.as_mut() else { return };
        let _ = match &track {
            Some(t) => controls.set_metadata(MediaMetadata {
                title: Some(&t.title),
                album: Some(&t.album),
                artist: Some(&t.artist),
                cover_url: art.as_deref(),
                duration: Some(Duration::from_secs_f64(duration)),
            }),
            None => controls.set_metadata(MediaMetadata {
                title: None, album: None, artist: None, cover_url: None, duration: None,
            }),
        };
    }

    // ─── lyrics ──────────────────────────────────────────────────────────────

    fn fetch_lyrics(mut self: Pin<&mut Self>, track: Option<Track>) {
        let gen = {
            let core = self.as_mut().core_mut();
            core.lyrics_gen += 1;
            core.lyric_times.clear();
            core.lyrics_gen
        };
        self.as_mut().set_lyrics_json(qs("[]"));
        self.as_mut().set_lyrics_index(-1);
        self.as_mut().set_lyrics_provider(QString::default());
        self.as_mut().set_lyrics_synced(false);
        let Some(track) = track else {
            self.set_lyrics_state(qs("idle"));
            return;
        };
        self.as_mut().set_lyrics_state(qs("loading"));
        let tidal = self.core().tidal.clone();
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let result = tidal.lyrics(track.id);
            let _ = thread.queue(move |b| b.lyrics_ready(gen, result));
        });
    }

    fn lyrics_ready(mut self: Pin<&mut Self>, gen: u64, result: Result<Option<Lyrics>, String>) {
        if self.core().lyrics_gen != gen {
            return;
        }
        match result {
            Ok(Some(lyrics)) if !lyrics.lines.is_empty() => {
                let lines: Vec<Value> = lyrics.lines.iter()
                    .map(|(t, text)| json!({ "t": t, "text": text })).collect();
                self.as_mut().core_mut().lyric_times = lyrics.lines.iter().map(|l| l.0).collect();
                self.as_mut().set_lyrics_json(qs(&Value::Array(lines).to_string()));
                self.as_mut().set_lyrics_synced(true);
                self.as_mut().set_lyrics_provider(qs(&lyrics.provider));
                self.as_mut().set_lyrics_state(qs("ready"));
                let pos = *self.position();
                self.update_lyric_index(pos);
            }
            Ok(Some(lyrics)) if !lyrics.text.trim().is_empty() => {
                self.as_mut().set_lyrics_json(qs(&json!([{ "t": -1, "text": lyrics.text }]).to_string()));
                self.as_mut().set_lyrics_provider(qs(&lyrics.provider));
                self.as_mut().set_lyrics_state(qs("ready"));
            }
            _ => self.set_lyrics_state(qs("none")),
        }
    }

    fn update_lyric_index(self: Pin<&mut Self>, position: f64) {
        let times = &self.core().lyric_times;
        if times.is_empty() {
            return;
        }
        let index = times.partition_point(|&t| t <= position + 0.2) as i32 - 1;
        if index != *self.lyrics_index() {
            self.set_lyrics_index(index);
        }
    }

    // ─── events from mpv and the desktop ─────────────────────────────────────

    fn on_mpv(mut self: Pin<&mut Self>, msg: Value) {
        match msg["event"].as_str().unwrap_or_default() {
            "property-change" => {
                let data = &msg["data"];
                match msg["name"].as_str().unwrap_or_default() {
                    "time-pos" => {
                        if let Some(p) = data.as_f64() {
                            self.as_mut().set_position(p);
                            self.as_mut().update_lyric_index(p);
                            let second = p as i64;
                            if second != self.core().mpris_second {
                                self.as_mut().core_mut().mpris_second = second;
                                self.sync_mpris();
                            }
                        }
                    }
                    "duration" => {
                        if let Some(d) = data.as_f64().filter(|d| *d > 0.0) {
                            self.as_mut().set_duration(d);
                            self.update_mpris_metadata();
                        }
                    }
                    "pause" => {
                        self.as_mut().core_mut().paused = data.as_bool().unwrap_or(false);
                        self.sync_playback();
                    }
                    "volume" => {
                        if let Some(v) = data.as_f64() {
                            self.as_mut().set_volume(v);
                            if (self.core().settings.volume - v).abs() > 0.01 {
                                self.as_mut().core_mut().settings.volume = v;
                                save_settings(&self.core().settings);
                            }
                            if let Some(c) = self.as_mut().core_mut().controls.as_mut() {
                                let _ = c.set_volume(v / 100.0);
                            }
                        }
                    }
                    "idle-active" => {
                        if data.as_bool() == Some(false) {
                            self.as_mut().core_mut().loading = false;
                            self.as_mut().set_active(true);
                        } else if !self.core().loading {
                            self.as_mut().set_active(false);
                            self.as_mut().set_position(0.0);
                        }
                        self.sync_playback();
                    }
                    _ => {}
                }
            }
            "end-file" => {
                let reason = msg["reason"].as_str().unwrap_or_default().to_string();
                self.on_end(&reason);
            }
            "mpv-dead" => {
                self.as_mut().core_mut().mpv = None;
                self.as_mut().set_active(false);
                self.as_mut().sync_playback();
                self.message("The playback engine (mpv) stopped. Restart the app to play music.");
            }
            _ => {}
        }
    }

    fn on_end(mut self: Pin<&mut Self>, reason: &str) {
        // "stop" means we replaced the file ourselves; "quit" means shutdown.
        let (pos, len, repeat) = (self.core().pos, self.core().queue.len() as i32, *self.repeat_mode());
        match reason {
            "eof" => {
                if repeat == REPEAT_ONE {
                    self.play_index(pos);
                } else if pos + 1 < len {
                    self.play_index(pos + 1);
                } else if repeat == REPEAT_ALL && len > 0 {
                    self.play_index(0);
                }
            }
            "error" => {
                self.as_mut().message("This track couldn't be played.");
                if pos + 1 < len {
                    self.play_index(pos + 1);
                }
            }
            _ => {}
        }
    }

    fn on_media(self: Pin<&mut Self>, event: MediaControlEvent) {
        match event {
            MediaControlEvent::Play => self.play(),
            MediaControlEvent::Pause => self.pause(),
            MediaControlEvent::Toggle => self.toggle_pause(),
            MediaControlEvent::Next => self.next(),
            MediaControlEvent::Previous => self.previous(),
            MediaControlEvent::Stop => self.stop_playback(),
            MediaControlEvent::Seek(dir) => {
                self.seek_by(if matches!(dir, SeekDirection::Forward) { 10.0 } else { -10.0 })
            }
            MediaControlEvent::SeekBy(dir, d) => {
                let s = d.as_secs_f64();
                self.seek_by(if matches!(dir, SeekDirection::Forward) { s } else { -s })
            }
            MediaControlEvent::SetPosition(MediaPosition(d)) => self.seek(d.as_secs_f64()),
            MediaControlEvent::SetVolume(v) => self.change_volume(v * 100.0),
            MediaControlEvent::Raise => self.raise_requested(),
            MediaControlEvent::Quit => self.quit_requested(),
            MediaControlEvent::OpenUri(_) => {}
        }
    }
}
