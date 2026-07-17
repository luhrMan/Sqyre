//! Panic hook + crash dump for the desktop shell.
//!
//! Writes [`sqyre_capture::CRASH_LOG_FILE`] under the Sqyre data dir and includes
//! the last [`sqyre_capture::mark_site`] breadcrumb. Continuous `diag.log` /
//! `last_site.txt` disk writes require `SQYRE_DIAG=1`.

use sqyre_capture::{
    disk_logging_enabled, note, read_last_site, set_log_dir, CRASH_LOG_FILE, DIAG_LOG_FILE,
    LAST_SITE_FILE,
};
use std::backtrace::Backtrace;
use std::fs::OpenOptions;
use std::io::Write;
use std::panic;
use std::path::PathBuf;

/// Point diag files at `dir` and install a panic hook that dumps to `crash.log`.
pub fn install(dir: PathBuf) {
    set_log_dir(Some(dir.clone()));
    if disk_logging_enabled() {
        note(&format!(
            "diag: disk logging on → {} ({DIAG_LOG_FILE}, {LAST_SITE_FILE}, {CRASH_LOG_FILE})",
            dir.display()
        ));
    } else {
        note(&format!(
            "diag: stderr only (set SQYRE_DIAG=1 for {DIAG_LOG_FILE}/{LAST_SITE_FILE}; panics → {})",
            dir.join(CRASH_LOG_FILE).display()
        ));
    }

    let crash_path = dir.join(CRASH_LOG_FILE);
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let payload = panic_payload(info);
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "(unknown)".into());
        let last_site = read_last_site().unwrap_or_else(|| "(none)".into());
        let backtrace = Backtrace::force_capture();
        let body = format!(
            "sqyre panic\n\
             location: {location}\n\
             payload: {payload}\n\
             last_site: {last_site}\n\
             \n\
             {backtrace}\n"
        );

        eprintln!("{body}");
        note(&format!(
            "panic at {location}: {payload} (last_site={last_site})"
        ));

        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&crash_path)
        {
            let _ = writeln!(f, "----");
            let _ = write!(f, "{body}");
            let _ = f.flush();
        }

        default_hook(info);
    }));
}

fn panic_payload(info: &panic::PanicHookInfo<'_>) -> String {
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        s.clone()
    } else {
        "(non-string panic payload)".into()
    }
}
