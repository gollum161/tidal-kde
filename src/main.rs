//! tidal-kde: an unofficial KDE (Kirigami) TIDAL client.

mod appearance;
mod autostart;
mod bridge;
mod mpv;
mod tidal;

use std::ffi::{c_char, c_int, CString};

extern "C" {
    // Defined in cpp/app.cpp: creates the QApplication and loads the QML UI.
    fn tidal_kde_run(argc: c_int, argv: *mut *mut c_char) -> c_int;
}

fn main() {
    let args: Vec<CString> = std::env::args()
        .map(|a| CString::new(a).unwrap_or_default())
        .collect();
    let mut argv: Vec<*mut c_char> = args.iter().map(|a| a.as_ptr() as *mut c_char).collect();
    argv.push(std::ptr::null_mut());
    let code = unsafe { tidal_kde_run(args.len() as c_int, argv.as_mut_ptr()) };
    drop(args);
    std::process::exit(code);
}
