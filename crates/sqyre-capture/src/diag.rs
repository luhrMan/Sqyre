//! Best-effort crash breadcrumbs for native / X11 / Win32 abort diagnosis.
//!
//! [`mark_site`] always updates in-memory state and overwrites [`LAST_SITE_FILE`]
//! (one small line — needed when a hard abort never reaches the Rust panic hook /
//! `crash.log`). [`note`] prints to stderr; set `SQYRE_DIAG=1` to also append
//! [`DIAG_LOG_FILE`].

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use web_time::{SystemTime, UNIX_EPOCH};

/// Set at the start of process quit so Drop paths can skip blocking joins
/// (portal session Close, tray D-Bus unregister, kick/overlay thread joins).
static PROCESS_EXITING: AtomicBool = AtomicBool::new(false);

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

/// Mark that the process is quitting. Blocking teardown should detach/abandon.
pub fn set_process_exiting() {
    PROCESS_EXITING.store(true, Ordering::SeqCst);
}

/// True after [`set_process_exiting`] (title-bar close, tray Quit, or `SqyreApp` Drop).
pub fn process_exiting() -> bool {
    PROCESS_EXITING.load(Ordering::SeqCst)
}

fn stamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn ensure_parent(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
}

/// Append-only site trail (quit overwrites [`LAST_SITE_FILE`]; this keeps history).
pub const SITE_HIST_FILE: &str = "site_hist.txt";

/// Record the current code site (memory + [`LAST_SITE_FILE`] always).
pub fn mark_site(site: &str) {
    let line = format!("{}\t{site}", stamp());
    if let Ok(mut g) = LAST_SITE.lock() {
        *g = Some(line.clone());
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
    // Always append a short trail so tray/quit cannot erase the freeze breadcrumb.
    let hist = log_dir().join(SITE_HIST_FILE);
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&hist) {
        let _ = f.write_all(file_line.as_bytes());
    }
}

/// Print a diagnostic line to stderr; append to [`DIAG_LOG_FILE`] only when disk logging is on.
pub fn note(msg: &str) {
    let line = format!("{} {msg}", stamp());
    eprintln!("sqyre: {line}");
    append_diag_line(&line);
}

/// Stable key=value log line for agents (e.g. `SQYRE_CAP=ok backend=x11 size=1920x1080`).
pub fn event_log(prefix: &str, fields: &[(&str, &str)]) {
    let kv = fields
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(" ");
    note(&format!("{prefix} {kv}"));
}

/// Category status log (`SQYRE_{category}={status} …`).
pub fn cap_log(category: &str, status: &str, detail: &str) {
    let prefix = format!("SQYRE_{category}={status}");
    if detail.is_empty() {
        note(&prefix);
    } else {
        note(&format!("{prefix} {detail}"));
    }
}

fn append_diag_line(line: &str) {
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
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn mark_and_read_site_memory_only() {
        let _guard = TEST_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!("sqyre-diag-mem-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        set_log_dir(Some(dir.clone()));
        set_disk_logging(Some(false));
        mark_site("x11:get_active_window:before_open");
        let site = read_last_site().unwrap();
        assert!(site.contains("get_active_window"));
        set_disk_logging(None);
        set_log_dir(None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn mark_site_always_writes_last_site_file() {
        let _guard = TEST_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!("sqyre-diag-site-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        set_log_dir(Some(dir.clone()));
        set_disk_logging(Some(false));
        mark_site("preview:finish_texture:done");
        let on_disk = fs::read_to_string(dir.join(LAST_SITE_FILE)).unwrap();
        assert!(on_disk.contains("preview:finish_texture:done"));
        assert!(
            !dir.join(DIAG_LOG_FILE).exists(),
            "diag.log stays off without SQYRE_DIAG"
        );
        set_disk_logging(None);
        set_log_dir(None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn mark_and_read_site_disk() {
        let _guard = TEST_LOCK.lock().unwrap();
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
