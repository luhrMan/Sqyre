//! Best-effort crash breadcrumbs for native / X11 abort diagnosis.
//!
//! X11 fatal errors abort the process without a Rust panic, so the last flushed
//! site in [`LAST_SITE_FILE`] is often the only clue. Append-only notes go to
//! [`DIAG_LOG_FILE`] for overlay/focus lifecycle context.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Overwritten single-line file: last code site before a hard abort.
pub const LAST_SITE_FILE: &str = "last_site.txt";
/// Append-only diagnostic log (overlay + X11 + panics pointer).
pub const DIAG_LOG_FILE: &str = "diag.log";
/// Panic / unwind dump written by the app panic hook.
pub const CRASH_LOG_FILE: &str = "crash.log";

static LOG_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Point diagnostics at the Sqyre data dir (e.g. `~/.sqyre`). Call once at startup.
pub fn set_log_dir(path: Option<PathBuf>) {
    if let Ok(mut g) = LOG_DIR.lock() {
        *g = path;
    }
}

/// Resolved log directory (override, else `~/.sqyre`, else temp).
pub fn log_dir() -> PathBuf {
    if let Ok(g) = LOG_DIR.lock() {
        if let Some(p) = g.clone() {
            return p;
        }
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join(".sqyre")
}

fn stamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn ensure_parent(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
}

/// Overwrite [`LAST_SITE_FILE`] and flush — call immediately before risky native work.
pub fn mark_site(site: &str) {
    let path = log_dir().join(LAST_SITE_FILE);
    ensure_parent(&path);
    let line = format!("{}\t{site}\n", stamp());
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
    {
        let _ = f.write_all(line.as_bytes());
        let _ = f.flush();
    }
}

/// Append a line to [`DIAG_LOG_FILE`] (also mirrors to stderr).
pub fn note(msg: &str) {
    let line = format!("{} {msg}", stamp());
    eprintln!("sqyre: {line}");
    let path = log_dir().join(DIAG_LOG_FILE);
    ensure_parent(&path);
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{line}");
        let _ = f.flush();
    }
}

/// Read the last marked site, if any.
pub fn read_last_site() -> Option<String> {
    let path = log_dir().join(LAST_SITE_FILE);
    let text = fs::read_to_string(path).ok()?;
    let line = text.lines().next()?.trim();
    if line.is_empty() {
        None
    } else {
        Some(line.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_and_read_site() {
        let dir = std::env::temp_dir().join(format!("sqyre-diag-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        set_log_dir(Some(dir.clone()));
        mark_site("x11:get_active_window:before_open");
        let site = read_last_site().unwrap();
        assert!(site.contains("get_active_window"));
        note("overlay: test note");
        let log = fs::read_to_string(dir.join(DIAG_LOG_FILE)).unwrap();
        assert!(log.contains("overlay: test note"));
        set_log_dir(None);
        let _ = fs::remove_dir_all(&dir);
    }
}
