//! Process + cache memory sampling for leak / RSS growth checks.
//!
//! Enable with `SQYRE_MEM=1` (or `true` / `yes`). Samples go to stderr and
//! append-only `mem.log` under the Sqyre data dir (independent of `SQYRE_DIAG`).
//!
//! Optional interval override: `SQYRE_MEM_INTERVAL_MS` (default 5000).
//!
//! For allocation stacks, rebuild with `--features dhat-heap` and quit cleanly
//! to write `dhat-heap.json` in the working directory.

use sqyre_capture::{event_log, log_dir, note};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use web_time::Instant;

/// Append-only memory sample trail (`~/.sqyre/mem.log` when `SQYRE_MEM` is on).
pub const MEM_LOG_FILE: &str = "mem.log";

const DEFAULT_INTERVAL_MS: u64 = 5000;
/// RSS growth (KiB) between successive post-run baselines that triggers a warn line.
const GROW_WARN_KIB: u64 = 16 * 1024;

static ENABLED: AtomicBool = AtomicBool::new(false);
static LAST_TICK_MS: AtomicU64 = AtomicU64::new(0);
static INTERVAL_MS: AtomicU64 = AtomicU64::new(DEFAULT_INTERVAL_MS);
static STATE: Mutex<MemState> = Mutex::new(MemState::new());

struct MemState {
    /// First sample RSS (KiB), if known.
    baseline_rss_kib: Option<u64>,
    /// Last sample RSS (KiB).
    last_rss_kib: Option<u64>,
    /// RSS after the most recent post-run clear+trim (leak smell if this climbs).
    post_run_rss_kib: Option<u64>,
    started: Option<Instant>,
}

impl MemState {
    const fn new() -> Self {
        Self {
            baseline_rss_kib: None,
            last_rss_kib: None,
            post_run_rss_kib: None,
            started: None,
        }
    }
}

/// True when `SQYRE_MEM` is `1` / `true` / `yes`.
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Read env and announce. Call once from [`crate::diag::install`].
pub fn install() {
    let on = matches!(
        std::env::var("SQYRE_MEM").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    );
    ENABLED.store(on, Ordering::SeqCst);
    if !on {
        return;
    }
    let interval = std::env::var("SQYRE_MEM_INTERVAL_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|&ms| ms >= 500)
        .unwrap_or(DEFAULT_INTERVAL_MS);
    INTERVAL_MS.store(interval, Ordering::SeqCst);
    if let Ok(mut g) = STATE.lock() {
        g.started = Some(Instant::now());
        g.baseline_rss_kib = None;
        g.last_rss_kib = None;
        g.post_run_rss_kib = None;
    }
    LAST_TICK_MS.store(0, Ordering::SeqCst);
    note(&format!(
        "mem: sampling on → {} (interval_ms={interval}; set SQYRE_MEM_INTERVAL_MS to change)",
        log_dir().join(MEM_LOG_FILE).display()
    ));
    sample("start");
}

/// Periodic sample from the egui logic tick (no-op unless [`enabled`]).
pub fn tick(ctx: &eframe::egui::Context) {
    if !enabled() {
        return;
    }
    let interval = INTERVAL_MS.load(Ordering::Relaxed);
    ctx.request_repaint_after(Duration::from_millis(interval));
    let Ok(g) = STATE.lock() else {
        return;
    };
    let Some(started) = g.started else {
        return;
    };
    let elapsed = started.elapsed().as_millis() as u64;
    drop(g);
    let last = LAST_TICK_MS.load(Ordering::Relaxed);
    if elapsed.saturating_sub(last) < interval {
        return;
    }
    LAST_TICK_MS.store(elapsed, Ordering::Relaxed);
    sample("tick");
}

/// Sample immediately (e.g. before/after macro run cleanup).
pub fn sample(reason: &str) {
    sample_inner(reason, None);
}

/// Like [`sample`], including action-log image retention (post-run baselines).
pub fn sample_with_action_log(reason: &str, log: &sqyre_ports::SharedActionLog) {
    sample_inner(reason, Some(log.image_bytes()));
}

fn sample_inner(reason: &str, action_log_bytes: Option<usize>) {
    if !enabled() {
        return;
    }
    let proc = read_process_mem();
    let cache = sqyre_vision::search_cache_stats();
    let cache_kib = (cache.bytes / 1024) as u64;
    let portal_kib = (sqyre_capture::capture_frame_cache_bytes() / 1024) as u64;
    let action_log_kib = action_log_bytes.map(|b| (b / 1024) as u64);

    let (delta_rss, since_start, grow) = {
        let Ok(mut g) = STATE.lock() else {
            return;
        };
        let delta_rss = match (proc.rss_kib, g.last_rss_kib) {
            (Some(now), Some(prev)) => Some(now as i64 - prev as i64),
            _ => None,
        };
        if g.baseline_rss_kib.is_none() {
            g.baseline_rss_kib = proc.rss_kib;
        }
        let since_start = match (proc.rss_kib, g.baseline_rss_kib) {
            (Some(now), Some(base)) => Some(now as i64 - base as i64),
            _ => None,
        };
        g.last_rss_kib = proc.rss_kib;

        let mut grow = false;
        if reason == "post_run" {
            if let (Some(now), Some(prev)) = (proc.rss_kib, g.post_run_rss_kib) {
                if now > prev.saturating_add(GROW_WARN_KIB) {
                    grow = true;
                }
            }
            g.post_run_rss_kib = proc.rss_kib;
        }
        (delta_rss, since_start, grow)
    };

    let mut fields: Vec<(&str, String)> = Vec::with_capacity(14);
    fields.push(("reason", reason.to_string()));
    if let Some(v) = proc.rss_kib {
        fields.push(("rss_kib", v.to_string()));
    }
    if let Some(v) = proc.hwm_kib {
        fields.push(("hwm_kib", v.to_string()));
    }
    if let Some(v) = proc.size_kib {
        fields.push(("size_kib", v.to_string()));
    }
    if let Some(v) = proc.threads {
        fields.push(("threads", v.to_string()));
    }
    if let Some(d) = delta_rss {
        fields.push(("delta_rss_kib", d.to_string()));
    }
    if let Some(d) = since_start {
        fields.push(("since_start_kib", d.to_string()));
    }
    fields.push(("search_cache_kib", cache_kib.to_string()));
    fields.push(("search_tmpl", cache.templates.to_string()));
    fields.push(("search_mask", cache.masks.to_string()));
    fields.push(("search_prep", cache.prepared.to_string()));
    fields.push(("portal_cache_kib", portal_kib.to_string()));
    if let Some(v) = action_log_kib {
        fields.push(("action_log_kib", v.to_string()));
    }

    let owned: Vec<(&str, &str)> = fields.iter().map(|(k, v)| (*k, v.as_str())).collect();
    event_log("SQYRE_MEM=sample", &owned);
    append_mem_line(&format_line("sample", &owned));

    if grow {
        let detail = format!(
            "post_run RSS climbed >{GROW_WARN_KIB} KiB vs prior post_run baseline (possible leak or unreclaimed heap)"
        );
        note(&format!("SQYRE_MEM=grow {detail}"));
        append_mem_line(&format!("grow {detail}"));
    }
}

fn format_line(kind: &str, fields: &[(&str, &str)]) -> String {
    let kv = fields
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(" ");
    format!("{kind} {kv}")
}

fn append_mem_line(line: &str) {
    let path = log_dir().join(MEM_LOG_FILE);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{line}");
        let _ = f.flush();
    }
}

#[derive(Default, Clone, Copy)]
struct ProcessMem {
    rss_kib: Option<u64>,
    hwm_kib: Option<u64>,
    size_kib: Option<u64>,
    threads: Option<u64>,
}

fn read_process_mem() -> ProcessMem {
    #[cfg(target_os = "linux")]
    {
        read_proc_self_status()
    }
    #[cfg(not(target_os = "linux"))]
    {
        ProcessMem::default()
    }
}

#[cfg(target_os = "linux")]
fn read_proc_self_status() -> ProcessMem {
    let Ok(text) = std::fs::read_to_string("/proc/self/status") else {
        return ProcessMem::default();
    };
    let mut out = ProcessMem::default();
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let Some(key) = parts.next() else {
            continue;
        };
        let Some(val) = parts.next() else {
            continue;
        };
        let Ok(n) = val.parse::<u64>() else {
            continue;
        };
        match key {
            "VmRSS:" => out.rss_kib = Some(n),
            "VmHWM:" => out.hwm_kib = Some(n),
            "VmSize:" => out.size_kib = Some(n),
            "Threads:" => out.threads = Some(n),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn proc_status_has_rss() {
        let m = read_proc_self_status();
        assert!(m.rss_kib.is_some_and(|n| n > 0));
        assert!(m.threads.is_some_and(|n| n >= 1));
    }

    #[test]
    fn format_line_stable() {
        let s = format_line("sample", &[("reason", "tick"), ("rss_kib", "100")]);
        assert_eq!(s, "sample reason=tick rss_kib=100");
    }

    #[test]
    fn sample_noop_when_disabled() {
        ENABLED.store(false, Ordering::SeqCst);
        sample("test");
    }
}
