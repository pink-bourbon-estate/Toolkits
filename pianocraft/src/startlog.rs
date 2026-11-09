//! A tiny local log of startup/shutdown steps (%APPDATA%\\PianoCraft\\startup.log), so that if
//! PianoCraft ever fails to open, the last line shows where it stopped. Only step names and
//! times are written; it never leaves your computer.

use std::io::Write;

pub fn log(step: &str) {
    let path = crate::curriculum::data_dir().join("startup.log");
    // keep the file small
    if std::fs::metadata(&path).map(|m| m.len() > 64 * 1024).unwrap_or(false) {
        let _ = std::fs::remove_file(&path);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let _ = writeln!(f, "{} pid {} {} {}", crate::curriculum::day_string((secs / 86400) as i64), std::process::id(), secs % 86400, step);
    }
}
