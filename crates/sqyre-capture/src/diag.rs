//! Best-effort crash breadcrumbs for native / X11 abort diagnosis.
//!
//! By default, sites stay in memory and [`note`] only prints to stderr — no
//! hot-path disk I/O. Set `SQYRE_DIAG=1` to also write [`LAST_SITE_FILE`] and
//! append [`DIAG_LOG_FILE`] (useful when diagnosing X11 fatal aborts that never
//! reach a Rust panic hook).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use web_time::{SystemTime, UNIX_EPOCH};

/// Overwritten single-line file: last code site before a hard abort.
pub const LAST_SITE_FILE: &str = "last_site.txt";
/// Append-only diagnostic log (overlay + X11 + panics pointer).
pub const DIAG_LOG_FILE: &str = "diag.log";
/// Panic / unwind dump written by the app panic hook.
pub const CRASH_LOG_FILE: &str = "crash.log";

static LOG_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);
static LAST_SITE: Mutex<Option<String>> = Mutex::new(None);
/// `None` = follow `SQYRE_DIAG`; `Some` overrides (tests).
static DISK_OVERRIDE: Mutex<Option<bool>> = Mutex::new(None);

/// Point diagnostics at the Sqyre data dir (e.g. `~/.sqyre`). Call once at startup.
pub fn set_log_dir(path: Option<PathBuf>) {
    if let Ok(mut g) = LOG_DIR.lock() {
        *g = path;
    }
}

/// Override disk logging (`None` restores `SQYRE_DIAG` / default-off).
pub fn set_disk_logging(enabled: Option<bool>) {
    if let Ok(mut g) = DISK_OVERRIDE.lock() {
        *g = enabled;
    }
}

/// Whether diag files are written (`SQYRE_DIAG=1`/`true`/`yes`, unless overridden).
pub fn disk_logging_enabled() -> bool {
    if let Ok(g) = DISK_OVERRIDE.lock() {
        if let Some(v) = *g {
            return v;
        }
    }
    matches!(
        std::env::var("SQYRE_DIAG").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

/// Resolved log directory (override, else `~/.sqyre`, else temp).
pub fn log_dir() -> PathBuf {
    if let Ok(g) = LOG_DIR.lock() {
        if let Some(p) = g.clone() {
            return p;
        }
    }
    // `std::env::temp_dir()` panics on wasm32-unknown-unknown.
    #[cfg(target_arch = "wasm32")]
    {
        return PathBuf::from("/sqyre");
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(".sqyre")
    }
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

/// Record the current code site (memory always; disk only when [`disk_logging_enabled`]).
pub fn mark_site(site: &str) {
    let line = format!("{}\t{site}", stamp());
    if let Ok(mut g) = LAST_SITE.lock() {
        *g = Some(line.clone());
    }
    if !disk_logging_enabled() {
        return;
    }
    let path = log_dir().join(LAST_SITE_FILE);
    ensure_parent(&path);
    let file_line = format!("{line}\n");
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
    {
        let _ = f.write_all(file_line.as_bytes());
        let _ = f.flush();
    }
}

/// Print a diagnostic line to stderr; append to [`DIAG_LOG_FILE`] only when disk logging is on.
pub fn note(msg: &str) {
    let line = format!("{} {msg}", stamp());
    eprintln!("sqyre: {line}");
    if !disk_logging_enabled() {
        return;
    }
    let path = log_dir().join(DIAG_LOG_FILE);
    ensure_parent(&path);
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{line}");
        let _ = f.flush();
    }
}

/// Read the last marked site (memory first, then [`LAST_SITE_FILE`] if present).
pub fn read_last_site() -> Option<String> {
    if let Ok(g) = LAST_SITE.lock() {
        if let Some(ref s) = *g {
            return Some(s.clone());
        }
    }
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
    fn mark_and_read_site_memory_only() {
        set_disk_logging(Some(false));
        set_log_dir(None);
        mark_site("x11:get_active_window:before_open");
        let site = read_last_site().unwrap();
        assert!(site.contains("get_active_window"));
        set_disk_logging(None);
    }

    #[test]
    fn mark_and_read_site_disk() {
        let dir = std::env::temp_dir().join(format!("sqyre-diag-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        set_log_dir(Some(dir.clone()));
        set_disk_logging(Some(true));
        mark_site("x11:get_active_window:before_open");
        let site = read_last_site().unwrap();
        assert!(site.contains("get_active_window"));
        note("overlay: test note");
        let log = fs::read_to_string(dir.join(DIAG_LOG_FILE)).unwrap();
        assert!(log.contains("overlay: test note"));
        set_disk_logging(None);
        set_log_dir(None);
        let _ = fs::remove_dir_all(&dir);
    }
}
