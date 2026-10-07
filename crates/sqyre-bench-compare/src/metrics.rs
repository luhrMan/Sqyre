//! Process resource sampling shared across harness sections.

use serde::{Deserialize, Serialize};
use std::fs;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub wall: Instant,
    pub cpu_user: Duration,
    pub cpu_sys: Duration,
    pub io_read: u64,
    pub io_write: u64,
    pub rss_kb: u64,
    pub hwm_kb: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SectionResult {
    pub name: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
    pub iterations: u64,
    pub wall_ns_total: u64,
    pub wall_ns_per_iter: u64,
    pub cpu_user_ns: u64,
    pub cpu_sys_ns: u64,
    /// Peak RSS (VmHWM) observed after the section, kibibytes.
    pub peak_rss_kb: u64,
    /// RSS (VmRSS) after the section, kibibytes.
    pub rss_kb: u64,
    pub io_read_bytes: u64,
    pub io_write_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl SectionResult {
    pub fn skipped(name: &str, reason: impl Into<String>) -> Self {
        Self {
            name: name.to_string(),
            status: "skipped".into(),
            skip_reason: Some(reason.into()),
            iterations: 0,
            wall_ns_total: 0,
            wall_ns_per_iter: 0,
            cpu_user_ns: 0,
            cpu_sys_ns: 0,
            peak_rss_kb: 0,
            rss_kb: 0,
            io_read_bytes: 0,
            io_write_bytes: 0,
            notes: None,
        }
    }

    pub fn error(name: &str, err: impl Into<String>) -> Self {
        Self {
            name: name.to_string(),
            status: "error".into(),
            skip_reason: Some(err.into()),
            iterations: 0,
            wall_ns_total: 0,
            wall_ns_per_iter: 0,
            cpu_user_ns: 0,
            cpu_sys_ns: 0,
            peak_rss_kb: 0,
            rss_kb: 0,
            io_read_bytes: 0,
            io_write_bytes: 0,
            notes: None,
        }
    }
}

pub fn sample_now() -> Sample {
    let (cpu_user, cpu_sys) = cpu_times();
    let (io_read, io_write) = io_bytes();
    let (rss_kb, hwm_kb) = rss_and_hwm_kb();
    Sample {
        wall: Instant::now(),
        cpu_user,
        cpu_sys,
        io_read,
        io_write,
        rss_kb,
        hwm_kb,
    }
}

pub fn finish(name: &str, iterations: u64, start: Sample, notes: Option<String>) -> SectionResult {
    let end = sample_now();
    let wall = end.wall.duration_since(start.wall);
    let wall_ns = wall.as_nanos() as u64;
    let per = wall_ns.checked_div(iterations).unwrap_or(0);
    let cpu_user = end.cpu_user.saturating_sub(start.cpu_user);
    let cpu_sys = end.cpu_sys.saturating_sub(start.cpu_sys);
    SectionResult {
        name: name.to_string(),
        status: "ok".into(),
        skip_reason: None,
        iterations,
        wall_ns_total: wall_ns,
        wall_ns_per_iter: per,
        cpu_user_ns: duration_ns(cpu_user),
        cpu_sys_ns: duration_ns(cpu_sys),
        peak_rss_kb: end.hwm_kb,
        rss_kb: end.rss_kb,
        io_read_bytes: end.io_read.saturating_sub(start.io_read),
        io_write_bytes: end.io_write.saturating_sub(start.io_write),
        notes,
    }
}

fn duration_ns(d: Duration) -> u64 {
    d.as_nanos() as u64
}

#[cfg(not(unix))]
fn cpu_times() -> (Duration, Duration) {
    (Duration::ZERO, Duration::ZERO)
}

#[cfg(unix)]
fn cpu_times() -> (Duration, Duration) {
    // SAFETY: `getrusage` gets a valid out-pointer to a zeroed `rusage`; `assume_init` runs
    // only on success, and an all-zero `rusage` is a valid value regardless.
    unsafe {
        let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
        if libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) != 0 {
            return (Duration::ZERO, Duration::ZERO);
        }
        let usage = usage.assume_init();
        (
            timeval_to_duration(usage.ru_utime),
            timeval_to_duration(usage.ru_stime),
        )
    }
}

#[cfg(unix)]
fn timeval_to_duration(tv: libc::timeval) -> Duration {
    let secs = tv.tv_sec.max(0) as u64;
    let micros = tv.tv_usec.max(0) as u32;
    Duration::new(secs, micros.saturating_mul(1_000))
}

fn io_bytes() -> (u64, u64) {
    let Ok(text) = fs::read_to_string("/proc/self/io") else {
        return (0, 0);
    };
    let mut read = 0u64;
    let mut write = 0u64;
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("read_bytes: ") {
            read = v.parse().unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("write_bytes: ") {
            write = v.parse().unwrap_or(0);
        }
    }
    (read, write)
}

fn rss_and_hwm_kb() -> (u64, u64) {
    let Ok(text) = fs::read_to_string("/proc/self/status") else {
        return (0, 0);
    };
    let mut rss = 0u64;
    let mut hwm = 0u64;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            rss = parse_kb(rest);
        } else if let Some(rest) = line.strip_prefix("VmHWM:") {
            hwm = parse_kb(rest);
        }
    }
    (rss, hwm)
}

fn parse_kb(rest: &str) -> u64 {
    rest.split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}
