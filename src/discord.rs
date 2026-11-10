//! Discord Rich Presence ("Listening to …") over Discord's local IPC socket.
//!
//! A worker thread owns the connection: Discord's IPC calls block, and Discord
//! may not be running at all. The Qt side sends the current state about once a
//! second; the worker only talks to Discord when something visible changed
//! (track, pause, a seek, the options), which keeps it well under Discord's
//! rate limit, and reconnects on its own when Discord starts later.

use discord_rich_presence::activity::{Activity, ActivityType, Assets, Button, StatusDisplayType, Timestamps};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The "Tidal KDE" application in Discord's developer portal (public, not a secret).
const APPLICATION_ID: &str = "1569614507444150363";
/// How often to retry while Discord isn't reachable.
const RETRY: Duration = Duration::from_secs(15);
/// A position jump larger than this (a seek) updates the progress bar.
const SEEK_TOLERANCE: i64 = 2;

/// What the user chose to show.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    pub enabled: bool,
    /// Text in the member list: "name" (Tidal KDE), "details" (track title) or "state" (artist).
    pub status: String,
    pub show_cover: bool,
    pub show_album: bool,
    pub show_button: bool,
    pub show_paused: bool,
}

/// The player's state, sent by the Qt side.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NowPlaying {
    pub track_id: i64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub cover: String,
    pub duration: f64,
    pub position: f64,
    pub active: bool,
    pub paused: bool,
}

enum Msg {
    Update(Options, NowPlaying),
    Quit,
}

/// Handle to the worker thread. Dropping it clears the presence and disconnects.
pub struct Presence {
    tx: Sender<Msg>,
}

impl Presence {
    /// `on_connected` is called (on the worker thread) whenever the connection
    /// to Discord comes up or goes down.
    pub fn start(on_connected: impl Fn(bool) + Send + 'static) -> Presence {
        let (tx, rx) = channel::<Msg>();
        std::thread::spawn(move || {
            let mut worker = Worker::new(on_connected);
            let mut latest: Option<(Options, NowPlaying)> = None;
            loop {
                match rx.recv_timeout(RETRY) {
                    Ok(Msg::Update(options, now)) => latest = Some((options, now)),
                    Ok(Msg::Quit) | Err(RecvTimeoutError::Disconnected) => break,
                    Err(RecvTimeoutError::Timeout) => {} // retry a failed connection
                }
                if let Some((options, now)) = &latest {
                    worker.apply(options, now);
                }
            }
            worker.disconnect();
        });
        Presence { tx }
    }

    pub fn update(&self, options: Options, now: NowPlaying) {
        let _ = self.tx.send(Msg::Update(options, now));
    }
}

impl Drop for Presence {
    fn drop(&mut self) {
        let _ = self.tx.send(Msg::Quit);
    }
}

/// What is shown in Discord, minus the exact start time (compared with a tolerance).
#[derive(Clone, Debug, PartialEq)]
struct Shown {
    details: String,
    state: String,
    status: String,
    cover: String,
    large_text: String,
    button: Option<String>,
    end_offset: Option<i64>,
}

struct Worker<F: Fn(bool)> {
    client: Option<DiscordIpcClient>,
    last_attempt: Option<Instant>,
    /// What Discord currently shows: None = nothing (cleared), plus the start time.
    shown: Option<(Shown, Option<i64>)>,
    on_connected: F,
}

impl<F: Fn(bool)> Worker<F> {
    fn new(on_connected: F) -> Self {
        Worker { client: None, last_attempt: None, shown: None, on_connected }
    }

    fn apply(&mut self, options: &Options, now: &NowPlaying) {
        let wanted = wanted(options, now, unix_now());
        if !options.enabled {
            self.disconnect();
            return;
        }
        // Skip when nothing visible changed (the start time may drift a little).
        let unchanged = match (&self.shown, &wanted) {
            (None, None) => self.client.is_some(),
            (Some((a, sa)), Some((b, sb))) => a == b && match (sa, sb) {
                (Some(x), Some(y)) => (x - y).abs() <= SEEK_TOLERANCE,
                (None, None) => true,
                _ => false,
            },
            _ => false,
        };
        if unchanged {
            return;
        }
        if self.client.is_none() && !self.connect() {
            return;
        }
        let client = self.client.as_mut().expect("connected above");
        let result = match &wanted {
            Some((shown, start)) => client.set_activity(activity(shown, *start)),
            None => client.clear_activity(),
        };
        match result {
            Ok(_) => self.shown = wanted,
            Err(_) => self.drop_connection(), // Discord quit: retry later
        }
    }

    fn connect(&mut self) -> bool {
        if self.last_attempt.is_some_and(|t| t.elapsed() < RETRY) {
            return false;
        }
        self.last_attempt = Some(Instant::now());
        let mut client = DiscordIpcClient::new(APPLICATION_ID);
        if client.connect().is_err() {
            return false;
        }
        self.client = Some(client);
        self.shown = None;
        (self.on_connected)(true);
        true
    }

    fn drop_connection(&mut self) {
        if let Some(mut client) = self.client.take() {
            let _ = client.close();
            (self.on_connected)(false);
        }
        self.shown = None;
    }

    fn disconnect(&mut self) {
        if let Some(client) = self.client.as_mut() {
            let _ = client.clear_activity();
        }
        self.drop_connection();
        self.last_attempt = None;
    }
}

fn unix_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Discord wants 2–128 characters in its text fields.
fn field(text: &str) -> String {
    let mut text: String = text.trim().to_string();
    if text.chars().count() > 128 {
        text = text.chars().take(127).collect::<String>() + "…";
    }
    while text.chars().count() < 2 {
        text.push('\u{200b}'); // zero-width space
    }
    text
}

/// What should be shown for this state (None = nothing), with the start time.
fn wanted(options: &Options, now: &NowPlaying, unix_now: i64) -> Option<(Shown, Option<i64>)> {
    if !options.enabled || !now.active || now.title.is_empty() || (now.paused && !options.show_paused) {
        return None;
    }
    let state = if now.paused { format!("Paused · {}", now.artist) } else { now.artist.clone() };
    let start = (!now.paused).then(|| unix_now - now.position.max(0.0) as i64);
    let shown = Shown {
        details: field(&now.title),
        state: field(&state),
        status: options.status.clone(),
        cover: if options.show_cover { now.cover.clone() } else { String::new() },
        large_text: if options.show_album && !now.album.is_empty() { field(&now.album) } else { String::new() },
        button: (options.show_button && now.track_id > 0)
            .then(|| format!("https://tidal.com/browse/track/{}", now.track_id)),
        end_offset: (!now.paused && now.duration > 0.0).then(|| now.duration as i64),
    };
    Some((shown, start))
}

fn activity(shown: &Shown, start: Option<i64>) -> Activity<'_> {
    let status = match shown.status.as_str() {
        "name" => StatusDisplayType::Name,
        "state" => StatusDisplayType::State,
        _ => StatusDisplayType::Details,
    };
    let mut activity = Activity::new()
        .activity_type(ActivityType::Listening)
        .status_display_type(status)
        .details(shown.details.as_str())
        .state(shown.state.as_str());
    if !shown.cover.is_empty() || !shown.large_text.is_empty() {
        let mut assets = Assets::new();
        if !shown.cover.is_empty() {
            assets = assets.large_image(shown.cover.as_str());
        }
        if !shown.large_text.is_empty() {
            assets = assets.large_text(shown.large_text.as_str());
        }
        activity = activity.assets(assets);
    }
    if let Some(start) = start {
        let mut timestamps = Timestamps::new().start(start);
        if let Some(length) = shown.end_offset {
            timestamps = timestamps.end(start + length);
        }
        activity = activity.timestamps(timestamps);
    }
    if let Some(url) = &shown.button {
        activity = activity.buttons(vec![Button::new("Listen on TIDAL", url.as_str())]);
    }
    activity
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> Options {
        Options { enabled: true, status: "details".into(), show_cover: true, show_album: true,
                  show_button: true, show_paused: false }
    }

    fn playing() -> NowPlaying {
        NowPlaying { track_id: 42, title: "Song".into(), artist: "Band".into(), album: "Record".into(),
                     cover: "https://resources.tidal.com/images/a/b/640x640.jpg".into(),
                     duration: 200.0, position: 50.0, active: true, paused: false }
    }

    #[test]
    fn shows_track_with_progress() {
        let (shown, start) = wanted(&options(), &playing(), 1000).unwrap();
        assert_eq!(shown.details, "Song");
        assert_eq!(shown.state, "Band");
        assert_eq!(start, Some(950));
        assert_eq!(shown.end_offset, Some(200));
        assert_eq!(shown.button.as_deref(), Some("https://tidal.com/browse/track/42"));
    }

    #[test]
    fn respects_options_and_pause() {
        let paused = NowPlaying { paused: true, ..playing() };
        assert!(wanted(&options(), &paused, 1000).is_none());
        let (shown, start) = wanted(&Options { show_paused: true, ..options() }, &paused, 1000).unwrap();
        assert_eq!(shown.state, "Paused · Band");
        assert_eq!(start, None);
        let bare = Options { show_cover: false, show_album: false, show_button: false, ..options() };
        let (shown, _) = wanted(&bare, &playing(), 1000).unwrap();
        assert!(shown.cover.is_empty() && shown.large_text.is_empty() && shown.button.is_none());
        assert!(wanted(&Options { enabled: false, ..options() }, &playing(), 1000).is_none());
        assert!(wanted(&options(), &NowPlaying { active: false, ..playing() }, 1000).is_none());
    }

    /// Talks to a fake Discord socket and checks what the worker sends.
    #[test]
    fn worker_sends_only_changes() {
        use std::io::{Read, Write};
        use std::os::unix::net::UnixListener;

        let dir = std::env::temp_dir().join(format!("tidal-kde-discord-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let listener = UnixListener::bind(dir.join("discord-ipc-0")).unwrap();
        // Only this test reads XDG_RUNTIME_DIR, so changing it doesn't affect others.
        std::env::set_var("XDG_RUNTIME_DIR", &dir);

        let fake = std::thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            sock.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
            let mut frames = Vec::new();
            loop {
                let mut header = [0u8; 8];
                if sock.read_exact(&mut header).is_err() {
                    break; // client closed or went quiet
                }
                let op = u32::from_le_bytes(header[..4].try_into().unwrap());
                let len = u32::from_le_bytes(header[4..].try_into().unwrap()) as usize;
                let mut body = vec![0u8; len];
                sock.read_exact(&mut body).unwrap();
                let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
                if op == 0 {
                    // Handshake: answer like Discord's READY event.
                    let ready = br#"{"cmd":"DISPATCH","evt":"READY","data":{"v":1}}"#;
                    let mut reply = 1u32.to_le_bytes().to_vec();
                    reply.extend((ready.len() as u32).to_le_bytes());
                    reply.extend(ready);
                    sock.write_all(&reply).unwrap();
                }
                frames.push((op, json));
            }
            frames
        });

        let connected = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = connected.clone();
        let mut worker = Worker::new(move |c| log.lock().unwrap().push(c));
        worker.apply(&options(), &playing());
        worker.apply(&options(), &NowPlaying { position: 51.0, ..playing() }); // 1 s later: no update
        worker.apply(&options(), &NowPlaying { paused: true, ..playing() });   // paused: cleared
        worker.disconnect();
        let frames = fake.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(frames[0].0, 0, "handshake first");
        assert_eq!(frames[0].1["client_id"], APPLICATION_ID);
        let set: Vec<&serde_json::Value> = frames.iter().filter(|(op, _)| *op == 1).map(|(_, j)| j).collect();
        assert_eq!(set.len(), 3, "activity, clear on pause, clear on disconnect: {set:?}");
        let activity = &set[0]["args"]["activity"];
        assert_eq!(activity["type"], 2, "Listening");
        assert_eq!(activity["details"], "Song");
        assert_eq!(activity["state"], "Band");
        assert_eq!(activity["status_display_type"], 2);
        assert_eq!(activity["assets"]["large_image"], "https://resources.tidal.com/images/a/b/640x640.jpg");
        assert_eq!(activity["assets"]["large_text"], "Record");
        assert_eq!(activity["buttons"][0]["url"], "https://tidal.com/browse/track/42");
        let start = activity["timestamps"]["start"].as_i64().unwrap();
        assert_eq!(activity["timestamps"]["end"].as_i64().unwrap() - start, 200);
        assert!(set[1]["args"]["activity"].is_null(), "pause clears the presence");
        assert_eq!(*connected.lock().unwrap(), vec![true, false]);
    }

    #[test]
    fn fits_discord_field_limits() {
        assert_eq!(field("X").chars().count(), 2);
        assert_eq!(field(&"a".repeat(300)).chars().count(), 128);
        assert_eq!(field("  Song  "), "Song");
    }
}
