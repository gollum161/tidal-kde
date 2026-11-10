//! "Start on login": an XDG autostart entry in ~/.config/autostart, which KDE
//! Plasma, GNOME and most other desktops read when the user logs in.

use std::path::{Path, PathBuf};

const FILE_NAME: &str = "io.github.tidal_kde.TidalKde.desktop";

/// Command-line flag that starts the app hidden in the system tray.
pub const HIDDEN_FLAG: &str = "--hidden";

fn entry_path() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("autostart").join(FILE_NAME)
}

pub fn is_enabled() -> bool {
    entry_path().is_file()
}

/// The program to start: the AppImage file itself when running from one (its
/// internal path changes on every start), the name `tidal-kde` when installed
/// in a standard location, else the full path of this binary (dev builds).
fn program() -> String {
    if let Some(appimage) = std::env::var_os("APPIMAGE").filter(|p| !p.is_empty()) {
        return PathBuf::from(appimage).display().to_string();
    }
    match std::env::current_exe() {
        Ok(exe) if exe == Path::new("/usr/bin/tidal-kde") || exe == Path::new("/usr/local/bin/tidal-kde") => {
            "tidal-kde".into()
        }
        Ok(exe) => exe.display().to_string(),
        Err(_) => "tidal-kde".into(),
    }
}

/// Quotes an argument for the Exec key of a .desktop file (Desktop Entry spec):
/// plain words stay as they are, anything else goes in double quotes with
/// `"`, `` ` ``, `$` and `\` escaped.
fn quote_exec_arg(arg: &str) -> String {
    let plain = !arg.is_empty()
        && arg.chars().all(|c| c.is_ascii_alphanumeric() || "/._-+=:,@".contains(c));
    if plain {
        return arg.to_string();
    }
    let mut quoted = String::from("\"");
    for c in arg.chars() {
        if matches!(c, '"' | '`' | '$' | '\\') {
            quoted.push('\\');
        }
        quoted.push(c);
    }
    quoted.push('"');
    // A literal % must be written as %% in Exec.
    quoted.replace('%', "%%")
}

fn entry_text(program: &str, hidden: bool) -> String {
    let mut exec = quote_exec_arg(program);
    if hidden {
        exec.push(' ');
        exec.push_str(HIDDEN_FLAG);
    }
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Tidal KDE\n\
         Comment=Stream music from your TIDAL account\n\
         Exec={exec}\n\
         Icon=io.github.tidal_kde.TidalKde\n\
         Terminal=false\n\
         X-GNOME-Autostart-enabled=true\n"
    )
}

/// Turns the autostart entry on (rewriting it, so a moved AppImage or a changed
/// "start hidden" choice is picked up) or off.
pub fn set_enabled(enable: bool, hidden: bool) -> std::io::Result<()> {
    let path = entry_path();
    if enable {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, entry_text(&program(), hidden))
    } else {
        match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_exec_arguments() {
        assert_eq!(quote_exec_arg("tidal-kde"), "tidal-kde");
        assert_eq!(quote_exec_arg("/opt/apps/tidal-kde-0.3.0-1-x86_64.AppImage"),
                   "/opt/apps/tidal-kde-0.3.0-1-x86_64.AppImage");
        assert_eq!(quote_exec_arg("/home/me/My Apps/tidal.AppImage"), "\"/home/me/My Apps/tidal.AppImage\"");
        assert_eq!(quote_exec_arg("/tmp/a$b\"c"), "\"/tmp/a\\$b\\\"c\"");
        assert_eq!(quote_exec_arg("/tmp/100% sure"), "\"/tmp/100%% sure\"");
    }

    #[test]
    fn writes_hidden_flag_only_when_asked() {
        assert!(entry_text("tidal-kde", true).contains("\nExec=tidal-kde --hidden\n"));
        assert!(entry_text("tidal-kde", false).contains("\nExec=tidal-kde\n"));
        assert!(entry_text("tidal-kde", false).starts_with("[Desktop Entry]\nType=Application\n"));
    }
}
