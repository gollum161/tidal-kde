//! Minimal TIDAL API client: OAuth device login, token refresh, browsing,
//! stream URLs, lyrics and favorites. Endpoints follow python-tidalapi.

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const API: &str = "https://api.tidal.com/v1/";
const AUTH: &str = "https://auth.tidal.com/v1/oauth2/";
const CLIENT_VERSION: &str = "2025.7.16";
const USER_AGENT: &str = "Mozilla/5.0 (Linux; Android 12; wv) AppleWebKit/537.36 \
    (KHTML, like Gecko) Version/4.0 Chrome/91.0.4472.114 Safari/537.36";
const SCOPE: &str = "r_usr w_usr w_sub";

pub type Result<T> = std::result::Result<T, String>;

pub fn config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("tidal-kde")
}

pub fn cache_dir() -> PathBuf {
    dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp")).join("tidal-kde")
}

fn write_private(path: &PathBuf, data: &str) {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let _ = std::fs::create_dir_all(config_dir());
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .write(true).create(true).truncate(true).mode(0o600).open(path)
    {
        let _ = f.write_all(data.as_bytes());
    }
}

/// The public device-client credentials used by open-source TIDAL clients
/// (same obfuscated values as python-tidalapi). If TIDAL rotates them, put
/// {"client_id": "...", "client_secret": "..."} in ~/.config/tidal-kde/client.json.
fn client_credentials() -> (String, String) {
    #[derive(Deserialize)]
    struct Override { client_id: String, client_secret: String }
    if let Ok(text) = std::fs::read_to_string(config_dir().join("client.json")) {
        if let Ok(o) = serde_json::from_str::<Override>(&text) {
            return (o.client_id, o.client_secret);
        }
    }
    fn unwrap(a: &str, b: &str) -> String {
        let mut joined = STANDARD.decode(a).unwrap_or_default();
        joined.extend(STANDARD.decode(b).unwrap_or_default());
        String::from_utf8(STANDARD.decode(joined).unwrap_or_default()).unwrap_or_default()
    }
    (
        unwrap("WmxneVNuaGtiVzUw", "V2xkTE1HbDRWQT09"),
        unwrap("TVU1dU9VRm1SRUZxZUhKblNrWktZa3RPVjB4bFFY",
               "bExSMVpIYlVsT2RWaFFVRXhJVmxoQmRuaEJaejA9"),
    )
}

// ─── Data passed to QML ──────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Track {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub artist_id: i64,
    pub album: String,
    pub album_id: i64,
    pub duration: f64,
    pub number: i32,
    pub cover: String,
    pub cover_large: String,
}

/// One list row. Every row has the same fields so QML's ListModel can hold any mix.
#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub kind: String,
    pub rid: String,
    pub title: String,
    pub subtitle: String,
    pub cover: String,
    pub duration: f64,
    pub number: i32,
    pub section: String,
    pub track_index: i32,
    pub album_id: String,
    pub artist_id: String,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct Header {
    pub kind: String,
    pub rid: String,
    pub title: String,
    pub subtitle: String,
    pub cover: String,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct Page {
    pub title: String,
    pub header: Option<Header>,
    pub rows: Vec<Row>,
    pub tracks: Vec<Track>,
    pub sectioned: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Lyrics {
    pub lines: Vec<(f64, String)>,
    pub text: String,
    pub provider: String,
}

pub struct DeviceLogin {
    pub device_code: String,
    pub user_code: String,
    pub url: String,
    pub expires_in: f64,
    pub interval: f64,
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct Tokens { token_type: String, access_token: String, refresh_token: String }

#[derive(Default, Clone)]
struct State { tokens: Option<Tokens>, user_id: i64, country: String, session_id: String }

// ─── Parsing helpers ─────────────────────────────────────────────────────────

pub fn image(uuid: &str, size: u32) -> String {
    if uuid.is_empty() {
        return String::new();
    }
    format!("https://resources.tidal.com/images/{}/{size}x{size}.jpg", uuid.replace('-', "/"))
}

fn s(v: &Value) -> String { v.as_str().unwrap_or_default().to_string() }

fn names(v: &Value) -> String {
    let list: Vec<String> = v["artists"].as_array().map(|a| {
        a.iter().filter_map(|x| x["name"].as_str().map(str::to_string)).collect()
    }).unwrap_or_default();
    if !list.is_empty() { list.join(", ") } else { s(&v["artist"]["name"]) }
}

fn parse_track(v: &Value) -> Option<Track> {
    let id = v["id"].as_i64()?;
    let mut title = s(&v["title"]);
    if let Some(version) = v["version"].as_str().filter(|x| !x.is_empty()) {
        title = format!("{title} ({version})");
    }
    let cover = s(&v["album"]["cover"]);
    Some(Track {
        id,
        title,
        artist: names(v),
        artist_id: v["artist"]["id"].as_i64()
            .or_else(|| v["artists"][0]["id"].as_i64()).unwrap_or(0),
        album: s(&v["album"]["title"]),
        album_id: v["album"]["id"].as_i64().unwrap_or(0),
        duration: v["duration"].as_f64().unwrap_or(0.0),
        number: v["trackNumber"].as_i64().unwrap_or(0) as i32,
        cover: image(&cover, 160),
        cover_large: image(&cover, 640),
    })
}

fn album_row(v: &Value, section: &str) -> Option<Row> {
    let year = s(&v["releaseDate"]).chars().take(4).collect::<String>();
    let artist = names(v);
    Some(Row {
        kind: "album".into(),
        rid: v["id"].as_i64()?.to_string(),
        title: s(&v["title"]),
        subtitle: if year.is_empty() { artist } else { format!("{artist}, {year}") },
        cover: image(&s(&v["cover"]), 160),
        section: section.into(),
        track_index: -1,
        ..Default::default()
    })
}

fn artist_row(v: &Value, section: &str) -> Option<Row> {
    Some(Row {
        kind: "artist".into(),
        rid: v["id"].as_i64()?.to_string(),
        title: s(&v["name"]),
        subtitle: "Artist".into(),
        cover: image(&s(&v["picture"]), 160),
        section: section.into(),
        track_index: -1,
        ..Default::default()
    })
}

fn playlist_row(v: &Value, section: &str) -> Option<Row> {
    let n = v["numberOfTracks"].as_i64().unwrap_or(0);
    Some(Row {
        kind: "playlist".into(),
        rid: v["uuid"].as_str()?.to_string(),
        title: s(&v["title"]),
        subtitle: format!("{n} tracks"),
        cover: image(&s(&v["squareImage"]), 160),
        section: section.into(),
        track_index: -1,
        ..Default::default()
    })
}

fn track_rows(tracks: &[Track], offset: usize, section: &str, covers: bool, numbered: bool) -> Vec<Row> {
    tracks.iter().enumerate().map(|(i, t)| Row {
        kind: "track".into(),
        rid: t.id.to_string(),
        title: t.title.clone(),
        subtitle: t.artist.clone(),
        cover: if covers { t.cover.clone() } else { String::new() },
        duration: t.duration,
        number: if numbered { t.number } else { 0 },
        section: section.into(),
        track_index: (offset + i) as i32,
        album_id: t.album_id.to_string(),
        artist_id: t.artist_id.to_string(),
    }).collect()
}

fn items(v: &Value) -> Vec<Value> {
    v["items"].as_array().cloned().unwrap_or_default()
}

/// Favorites wrap each entry as {"created": ..., "item": {...}}.
fn unwrap_items(list: Vec<Value>) -> Vec<Value> {
    list.into_iter().map(|x| if x.get("item").is_some() { x["item"].clone() } else { x }).collect()
}

pub fn parse_lrc(text: &str) -> Vec<(f64, String)> {
    let mut out = Vec::new();
    for raw in text.lines() {
        let mut rest = raw.trim();
        let mut stamps = Vec::new();
        while rest.starts_with('[') {
            let Some(end) = rest.find(']') else { break };
            let stamp = &rest[1..end];
            let Some((m, sec)) = stamp.split_once(':') else { break };
            match (m.parse::<f64>(), sec.parse::<f64>()) {
                (Ok(m), Ok(sec)) => stamps.push(m * 60.0 + sec),
                _ => break,
            }
            rest = rest[end + 1..].trim_start();
        }
        for t in stamps {
            out.push((t, rest.trim().to_string()));
        }
    }
    out.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    out
}

// ─── Client ─────────────────────────────────────────────────────────────────

pub struct Tidal {
    agent: ureq::Agent,
    client_id: String,
    client_secret: String,
    state: Mutex<State>,
}

fn http_err(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, resp) => {
            let body = resp.into_string().unwrap_or_default();
            let detail: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
            let msg = detail["userMessage"].as_str()
                .or(detail["error_description"].as_str())
                .unwrap_or("request failed");
            format!("TIDAL returned {code}: {msg}")
        }
        ureq::Error::Transport(t) => format!("Couldn't reach TIDAL ({}). Check your connection.", t.kind()),
    }
}

impl Default for Tidal {
    fn default() -> Self { Self::new() }
}

impl Tidal {
    pub fn new() -> Self {
        let (client_id, client_secret) = client_credentials();
        Tidal {
            agent: ureq::AgentBuilder::new()
                .timeout(Duration::from_secs(20))
                .user_agent(USER_AGENT)
                .build(),
            client_id,
            client_secret,
            state: Mutex::new(State::default()),
        }
    }

    fn session_file() -> PathBuf { config_dir().join("session.json") }

    fn set_tokens(&self, json: &Value, keep_refresh: bool) -> Result<()> {
        let mut st = self.state.lock().unwrap();
        let old_refresh = st.tokens.as_ref().map(|t| t.refresh_token.clone()).unwrap_or_default();
        let refresh = json["refresh_token"].as_str().map(str::to_string)
            .filter(|r| !r.is_empty())
            .unwrap_or(if keep_refresh { old_refresh } else { String::new() });
        let tokens = Tokens {
            token_type: json["token_type"].as_str().unwrap_or("Bearer").to_string(),
            access_token: json["access_token"].as_str().ok_or("No access token in response")?.to_string(),
            refresh_token: refresh,
        };
        write_private(&Self::session_file(), &serde_json::to_string(&tokens).unwrap_or_default());
        st.tokens = Some(tokens);
        Ok(())
    }

    /// Restore a saved login. Returns Ok(false) when there is none.
    pub fn restore(&self) -> Result<bool> {
        let Ok(text) = std::fs::read_to_string(Self::session_file()) else { return Ok(false) };
        let Ok(tokens) = serde_json::from_str::<Tokens>(&text) else { return Ok(false) };
        self.state.lock().unwrap().tokens = Some(tokens);
        if self.refresh().is_err() {
            self.state.lock().unwrap().tokens = None;
            return Ok(false);
        }
        self.load_session()?;
        Ok(true)
    }

    fn refresh(&self) -> Result<()> {
        let refresh = self.state.lock().unwrap().tokens.as_ref()
            .map(|t| t.refresh_token.clone()).unwrap_or_default();
        if refresh.is_empty() {
            return Err("Not logged in".into());
        }
        let resp = self.agent.post(&format!("{AUTH}token"))
            .send_form(&[("grant_type", "refresh_token"), ("refresh_token", &refresh),
                         ("client_id", &self.client_id), ("client_secret", &self.client_secret)])
            .map_err(http_err)?;
        let json: Value = resp.into_json().map_err(|e| e.to_string())?;
        self.set_tokens(&json, true)
    }

    fn load_session(&self) -> Result<()> {
        let v = self.get("sessions", &[])?;
        let mut st = self.state.lock().unwrap();
        st.user_id = v["userId"].as_i64().ok_or("No user id in session")?;
        st.country = s(&v["countryCode"]);
        st.session_id = s(&v["sessionId"]);
        Ok(())
    }

    pub fn device_login_start(&self) -> Result<DeviceLogin> {
        let resp = self.agent.post(&format!("{AUTH}device_authorization"))
            .send_form(&[("client_id", &self.client_id), ("scope", SCOPE)])
            .map_err(http_err)?;
        let v: Value = resp.into_json().map_err(|e| e.to_string())?;
        let mut url = s(&v["verificationUriComplete"]);
        if !url.starts_with("http") {
            url = format!("https://{url}");
        }
        Ok(DeviceLogin {
            device_code: s(&v["deviceCode"]),
            user_code: s(&v["userCode"]),
            url,
            expires_in: v["expiresIn"].as_f64().unwrap_or(300.0),
            interval: v["interval"].as_f64().unwrap_or(2.0).max(1.0),
        })
    }

    pub fn device_login_wait(&self, login: &DeviceLogin) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs_f64(login.expires_in);
        while Instant::now() < deadline {
            std::thread::sleep(Duration::from_secs_f64(login.interval));
            let result = self.agent.post(&format!("{AUTH}token")).send_form(&[
                ("client_id", &self.client_id), ("client_secret", &self.client_secret),
                ("device_code", &login.device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"), ("scope", SCOPE),
            ]);
            match result {
                Ok(resp) => {
                    let json: Value = resp.into_json().map_err(|e| e.to_string())?;
                    self.set_tokens(&json, false)?;
                    return self.load_session();
                }
                Err(ureq::Error::Status(_, resp)) => {
                    let v: Value = resp.into_json().unwrap_or(Value::Null);
                    if v["error"] == "expired_token" {
                        break;
                    }
                }
                Err(e) => return Err(http_err(e)),
            }
        }
        Err("The login link expired. Try again.".into())
    }

    pub fn logout(&self) {
        let _ = std::fs::remove_file(Self::session_file());
        *self.state.lock().unwrap() = State::default();
    }

    fn user_id(&self) -> i64 { self.state.lock().unwrap().user_id }

    fn request(&self, method: &str, path: &str, params: &[(&str, String)],
               form: Option<&[(&str, &str)]>) -> std::result::Result<ureq::Response, ureq::Error> {
        let st = self.state.lock().unwrap().clone();
        let mut req = self.agent.request(method, &format!("{API}{path}"))
            .set("x-tidal-client-version", CLIENT_VERSION);
        if let Some(t) = &st.tokens {
            req = req.set("authorization", &format!("{} {}", t.token_type, t.access_token));
        }
        if !st.country.is_empty() {
            req = req.query("countryCode", &st.country);
        }
        if !st.session_id.is_empty() {
            req = req.query("sessionId", &st.session_id);
        }
        for (k, v) in params {
            req = req.query(k, v);
        }
        match form {
            Some(f) => req.send_form(f),
            None => req.call(),
        }
    }

    fn send(&self, method: &str, path: &str, params: &[(&str, String)],
            form: Option<&[(&str, &str)]>) -> Result<ureq::Response> {
        match self.request(method, path, params, form) {
            Err(ureq::Error::Status(401, _)) => {
                self.refresh()?;
                self.request(method, path, params, form).map_err(http_err)
            }
            other => other.map_err(http_err),
        }
    }

    pub fn get(&self, path: &str, params: &[(&str, String)]) -> Result<Value> {
        self.send("GET", path, params, None)?.into_json().map_err(|e| e.to_string())
    }

    fn get_all(&self, path: &str, params: &[(&str, String)], cap: usize) -> Result<Vec<Value>> {
        let mut out = Vec::new();
        let limit = 100usize;
        loop {
            let mut p: Vec<(&str, String)> = params.to_vec();
            p.push(("limit", limit.to_string()));
            p.push(("offset", out.len().to_string()));
            let v = self.get(path, &p)?;
            let batch = items(&v);
            let n = batch.len();
            out.extend(batch);
            let total = v["totalNumberOfItems"].as_u64().unwrap_or(0) as usize;
            if n < limit || out.len() >= cap || (total > 0 && out.len() >= total) {
                break;
            }
        }
        Ok(out)
    }

    // ─── Pages ───────────────────────────────────────────────────────────────

    pub fn search(&self, query: &str) -> Result<Page> {
        let v = self.get("search", &[("query", query.to_string()), ("limit", "12".into()),
                                     ("offset", "0".into()),
                                     ("types", "TRACKS,ALBUMS,ARTISTS,PLAYLISTS".into())])?;
        let tracks: Vec<Track> = items(&v["tracks"]).iter().filter_map(parse_track).collect();
        let mut rows = track_rows(&tracks, 0, "Tracks", true, false);
        rows.extend(items(&v["albums"]).iter().take(8).filter_map(|x| album_row(x, "Albums")));
        rows.extend(items(&v["artists"]).iter().take(6).filter_map(|x| artist_row(x, "Artists")));
        rows.extend(items(&v["playlists"]).iter().take(6).filter_map(|x| playlist_row(x, "Playlists")));
        Ok(Page { title: "Search".into(), header: None, rows, tracks, sectioned: true })
    }

    pub fn favorites(&self, kind: &str) -> Result<Page> {
        let uid = self.user_id();
        let order = [("order", "DATE".to_string()), ("orderDirection", "DESC".to_string())];
        match kind {
            "tracks" => {
                let list = unwrap_items(self.get_all(&format!("users/{uid}/favorites/tracks"), &order, 10_000)?);
                let tracks: Vec<Track> = list.iter().filter_map(parse_track).collect();
                let covers = tracks.len() <= 400;
                Ok(Page { title: "Favorite tracks".into(), rows: track_rows(&tracks, 0, "", covers, false),
                          tracks, ..Default::default() })
            }
            "albums" => {
                let list = unwrap_items(self.get_all(&format!("users/{uid}/favorites/albums"), &order, 5000)?);
                Ok(Page { title: "Albums".into(),
                          rows: list.iter().filter_map(|x| album_row(x, "")).collect(), ..Default::default() })
            }
            "artists" => {
                let list = unwrap_items(self.get_all(&format!("users/{uid}/favorites/artists"), &order, 5000)?);
                Ok(Page { title: "Artists".into(),
                          rows: list.iter().filter_map(|x| artist_row(x, "")).collect(), ..Default::default() })
            }
            _ => {
                let own = self.get_all(&format!("users/{uid}/playlists"), &[], 2000)?;
                let mut rows: Vec<Row> = own.iter().filter_map(|x| playlist_row(x, "Your playlists")).collect();
                let own_ids: Vec<String> = rows.iter().map(|r| r.rid.clone()).collect();
                // Saved playlists: older v1 endpoint; skipped quietly if TIDAL rejects it.
                if let Ok(saved) = self.get_all(&format!("users/{uid}/favorites/playlists"), &order, 2000) {
                    rows.extend(unwrap_items(saved).iter()
                        .filter_map(|x| playlist_row(x, "Saved playlists"))
                        .filter(|r| !own_ids.contains(&r.rid)));
                }
                Ok(Page { title: "Playlists".into(), rows, sectioned: true, ..Default::default() })
            }
        }
    }

    pub fn album(&self, id: &str) -> Result<Page> {
        let a = self.get(&format!("albums/{id}"), &[])?;
        let tracks: Vec<Track> = self.get_all(&format!("albums/{id}/tracks"), &[], 1000)?
            .iter().filter_map(parse_track).collect();
        let year = s(&a["releaseDate"]).chars().take(4).collect::<String>();
        let minutes = tracks.iter().map(|t| t.duration).sum::<f64>() as i64 / 60;
        let mut subtitle = names(&a);
        if !year.is_empty() { subtitle.push_str(&format!(", {year}")); }
        subtitle.push_str(&format!(" — {} tracks, {minutes} min", tracks.len()));
        Ok(Page {
            title: s(&a["title"]),
            header: Some(Header { kind: "album".into(), rid: id.into(), title: s(&a["title"]),
                                  subtitle, cover: image(&s(&a["cover"]), 320) }),
            rows: track_rows(&tracks, 0, "", false, true),
            tracks,
            sectioned: false,
        })
    }

    pub fn artist(&self, id: &str) -> Result<Page> {
        let a = self.get(&format!("artists/{id}"), &[])?;
        let top: Vec<Track> = items(&self.get(&format!("artists/{id}/toptracks"), &[("limit", "10".into())])?)
            .iter().filter_map(parse_track).collect();
        let albums = self.get_all(&format!("artists/{id}/albums"), &[], 300).unwrap_or_default();
        let singles = self.get_all(&format!("artists/{id}/albums"),
                                   &[("filter", "EPSANDSINGLES".into())], 300).unwrap_or_default();
        let mut rows = track_rows(&top, 0, "Popular", true, false);
        rows.extend(albums.iter().filter_map(|x| album_row(x, "Albums")));
        rows.extend(singles.iter().filter_map(|x| album_row(x, "Singles and EPs")));
        Ok(Page {
            title: s(&a["name"]),
            header: Some(Header { kind: "artist".into(), rid: id.into(), title: s(&a["name"]),
                                  subtitle: "Artist".into(), cover: image(&s(&a["picture"]), 320) }),
            rows,
            tracks: top,
            sectioned: true,
        })
    }

    pub fn playlist(&self, uuid: &str) -> Result<Page> {
        let p = self.get(&format!("playlists/{uuid}"), &[])?;
        let tracks: Vec<Track> = self.get_all(&format!("playlists/{uuid}/tracks"), &[], 10_000)?
            .iter().filter_map(parse_track).collect();
        let minutes = tracks.iter().map(|t| t.duration).sum::<f64>() as i64 / 60;
        let creator = s(&p["creator"]["name"]);
        let mut subtitle = format!("{} tracks, {minutes} min", tracks.len());
        if !creator.is_empty() { subtitle = format!("By {creator} — {subtitle}"); }
        let covers = tracks.len() <= 400;
        Ok(Page {
            title: s(&p["title"]),
            header: Some(Header { kind: "playlist".into(), rid: uuid.into(), title: s(&p["title"]),
                                  subtitle, cover: image(&s(&p["squareImage"]), 320) }),
            rows: track_rows(&tracks, 0, "", covers, false),
            tracks,
            sectioned: false,
        })
    }

    // ─── Playback data ───────────────────────────────────────────────────────

    pub fn stream_url(&self, track_id: i64, quality: &str) -> Result<String> {
        let mut order = vec![quality.to_string()];
        for q in ["LOSSLESS", "HIGH", "LOW"] {
            if !order.iter().any(|x| x == q) { order.push(q.into()); }
        }
        let mut last = String::from("no stream available");
        for q in order {
            match self.get(&format!("tracks/{track_id}/urlpostpaywall"),
                           &[("urlusagemode", "STREAM".into()), ("audioquality", q.clone()),
                             ("assetpresentation", "FULL".into())]) {
                Ok(v) => {
                    if let Some(url) = v["urls"][0].as_str() {
                        return Ok(url.to_string());
                    }
                }
                Err(e) => last = e,
            }
        }
        Err(last)
    }

    pub fn lyrics(&self, track_id: i64) -> Result<Option<Lyrics>> {
        match self.send("GET", &format!("tracks/{track_id}/lyrics"), &[], None) {
            Ok(resp) => {
                let v: Value = resp.into_json().map_err(|e| e.to_string())?;
                Ok(Some(Lyrics {
                    lines: parse_lrc(v["subtitles"].as_str().unwrap_or_default()),
                    text: s(&v["lyrics"]),
                    provider: s(&v["lyricsProvider"]),
                }))
            }
            Err(e) if e.contains(" 404") => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn like(&self, kind: &str, id: &str) -> Result<()> {
        let uid = self.user_id();
        let (path, field) = match kind {
            "track" => ("tracks", "trackId"),
            "album" => ("albums", "albumId"),
            "artist" => ("artists", "artistId"),
            _ => return Err("Only tracks, albums and artists can be added to favorites here".into()),
        };
        self.send("POST", &format!("users/{uid}/favorites/{path}"), &[], Some(&[(field, id)]))?;
        Ok(())
    }

    /// Download album art to the cache (for MPRIS, which wants a file:// URL).
    pub fn cached_cover(&self, album_id: i64, url: &str) -> Option<String> {
        if url.is_empty() { return None; }
        let dir = cache_dir().join("covers");
        let path = dir.join(format!("{album_id}.jpg"));
        if !path.exists() {
            let _ = std::fs::create_dir_all(&dir);
            let resp = self.agent.get(url).call().ok()?;
            let mut bytes = Vec::new();
            std::io::Read::read_to_end(&mut resp.into_reader(), &mut bytes).ok()?;
            let tmp = path.with_extension("part");
            std::fs::write(&tmp, &bytes).ok()?;
            std::fs::rename(&tmp, &path).ok()?;
        }
        Some(format!("file://{}", path.display()))
    }
}
