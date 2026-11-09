//! mpv as a playback engine, driven over its JSON IPC socket.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

pub struct Mpv {
    child: Child,
    writer: Mutex<UnixStream>,
    socket: PathBuf,
}

impl Mpv {
    /// Start mpv. `on_message` is called on a background thread for every mpv event.
    pub fn spawn(on_message: impl Fn(Value) + Send + 'static) -> Result<Mpv, String> {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"));
        let socket = runtime.join(format!("tidal-kde-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&socket);

        let child = Command::new("mpv")
            .args(["--idle=yes", "--no-video", "--no-terminal", "--no-config",
                   "--load-scripts=no", "--audio-display=no", "--audio-client-name=Tidal KDE"])
            .arg(format!("--input-ipc-server={}", socket.display()))
            .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Couldn't start mpv ({e}). Install mpv with your package manager, e.g. sudo pacman -S mpv or sudo apt install mpv"))?;

        let mut stream = None;
        for _ in 0..100 {
            if let Ok(s) = UnixStream::connect(&socket) {
                stream = Some(s);
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let stream = stream.ok_or("Couldn't connect to mpv")?;
        let reader = stream.try_clone().map_err(|e| e.to_string())?;

        std::thread::spawn(move || {
            for line in BufReader::new(reader).lines() {
                let Ok(line) = line else { break };
                if let Ok(v) = serde_json::from_str::<Value>(&line) {
                    if v.get("event").is_some() {
                        on_message(v);
                    }
                }
            }
            on_message(json!({"event": "mpv-dead"}));
        });

        Ok(Mpv { child, writer: Mutex::new(stream), socket })
    }

    pub fn command(&self, args: Value) {
        let mut line = json!({ "command": args }).to_string();
        line.push('\n');
        if let Ok(mut w) = self.writer.lock() {
            let _ = w.write_all(line.as_bytes());
        }
    }
}

impl Drop for Mpv {
    fn drop(&mut self) {
        self.command(json!(["quit"]));
        for _ in 0..20 {
            if let Ok(Some(_)) = self.child.try_wait() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.socket);
    }
}
