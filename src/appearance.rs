//! Window appearance: KDE colour schemes, window blur and the system tray.
//! The Qt/KDE calls live in cpp/app.cpp; this module finds the installed
//! colour schemes and wraps the C functions.

use serde::Serialize;
use std::collections::BTreeMap;
use std::ffi::CString;
use std::os::raw::c_char;
use std::path::PathBuf;

extern "C" {
    // Defined in cpp/app.cpp.
    fn tidal_kde_apply_color_scheme(id: *const c_char);
    fn tidal_kde_apply_blur(enable: bool);
    fn tidal_kde_blur_available() -> bool;
}

/// One installed colour scheme. `id` is the file name without `.colors`.
#[derive(Serialize)]
pub struct ColorScheme {
    pub id: String,
    pub name: String,
}

/// Folders that may contain `color-schemes/`, highest priority first
/// (the same lookup KDE uses: XDG_DATA_HOME, then XDG_DATA_DIRS).
fn data_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = dirs::data_dir() {
        dirs.push(home);
    }
    let system = std::env::var("XDG_DATA_DIRS").unwrap_or_default();
    let system = if system.is_empty() { "/usr/local/share:/usr/share".to_string() } else { system };
    dirs.extend(system.split(':').filter(|d| !d.is_empty()).map(PathBuf::from));
    dirs
}

/// The display name from a `.colors` file (`Name=` in `[General]`).
fn scheme_name(text: &str) -> Option<String> {
    let mut in_general = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_general = line == "[General]";
        } else if in_general {
            if let Some(name) = line.strip_prefix("Name=") {
                return Some(name.trim().to_string());
            }
        }
    }
    None
}

/// All installed colour schemes, sorted by name. A scheme in the user's own
/// folder hides a system scheme with the same id, as in KDE.
pub fn color_schemes() -> Vec<ColorScheme> {
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    for dir in data_dirs() {
        let Ok(entries) = std::fs::read_dir(dir.join("color-schemes")) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(true, |e| e != "colors") {
                continue;
            }
            let Some(id) = path.file_stem().and_then(|s| s.to_str()).map(str::to_string) else { continue };
            if found.contains_key(&id) {
                continue;
            }
            let name = std::fs::read_to_string(&path).ok()
                .and_then(|t| scheme_name(&t))
                .unwrap_or_else(|| id.clone());
            found.insert(id, name);
        }
    }
    let mut list: Vec<ColorScheme> = found.into_iter().map(|(id, name)| ColorScheme { id, name }).collect();
    list.sort_by_key(|s| s.name.to_lowercase());
    list
}

/// Activates a colour scheme by id; an empty id follows the system scheme.
pub fn apply_color_scheme(id: &str) {
    let id = CString::new(id).unwrap_or_default();
    unsafe { tidal_kde_apply_color_scheme(id.as_ptr()) }
}

/// Turns the compositor's blur behind the window on or off (KWin only).
pub fn apply_blur(enable: bool) {
    unsafe { tidal_kde_apply_blur(enable) }
}

/// Whether the compositor can blur behind windows.
pub fn blur_available() -> bool {
    unsafe { tidal_kde_blur_available() }
}

#[cfg(test)]
mod tests {
    use super::scheme_name;

    #[test]
    fn reads_name_from_general_group() {
        let text = "[Colors:View]\nName=Wrong\n\n[General]\nColorScheme=BreezeDark\nName=Breeze Dark\nName[de]=Breeze-Dunkel\n";
        assert_eq!(scheme_name(text).as_deref(), Some("Breeze Dark"));
        assert_eq!(scheme_name("[General]\n"), None);
    }
}
