//! Overlay host FPS sampling when `SQYRE_DIAG` is on.
//!
//! ~1 s window (like [`mem_diag`]): host-loop wake rate plus portal ScreenCast
//! stream Hz when capture has installed the frame counter. Not the game's
//! internal FPS meter — use it to correlate overlay state with compositor /
//! PipeWire delivery while gathering evidence.

use parking_lot::Mutex;
use sqyre_ports::{disk_logging_enabled, event_log, note, portal_stream_frame_count};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Default sample window (ms).
pub const SAMPLE_MS: u64 = 1000;

/// Overlay surface + poll state for one host-loop sample.
#[derive(Debug, Clone, Copy)]
pub struct OverlayFpsSnapshot {
    pub mapped_faces: u32,
    pub mapped_hits: u32,
    pub click_through: bool,
    pub poll_ms: u64,
}

static HOST_TICKS: AtomicU64 = AtomicU64::new(0);
static STATE: Mutex<SampleState> = Mutex::new(SampleState::new());

struct SampleState {
    window_start: Option<Instant>,
    ticks_at_start: u64,
    portal_at_start: u64,
}

impl SampleState {
    const fn new() -> Self {
        Self {
            window_start: None,
            ticks_at_start: 0,
            portal_at_start: 0,
        }
    }
}

/// Hz from a count delta over an elapsed window (tests / callers).
pub fn rate_hz(delta_count: u64, elapsed_secs: f64) -> f64 {
    if elapsed_secs <= 0.0 {
        return 0.0;
    }
    delta_count as f64 / elapsed_secs
}

/// Count one host-loop iteration. When `SQYRE_DIAG` is on, emits
/// `SQYRE_OVERLAY_FPS` about once per [`SAMPLE_MS`].
pub fn host_tick(snap: OverlayFpsSnapshot) {
    if !disk_logging_enabled() {
        return;
    }
    let ticks = HOST_TICKS.fetch_add(1, Ordering::Relaxed) + 1;
    let portal_now = portal_stream_frame_count();
    let mut g = STATE.lock();
    let Some(start) = g.window_start else {
        g.window_start = Some(Instant::now());
        g.ticks_at_start = ticks.saturating_sub(1);
        g.portal_at_start = portal_now;
        return;
    };
    let elapsed = start.elapsed();
    if elapsed.as_millis() < u128::from(SAMPLE_MS) {
        return;
    }
    let elapsed_secs = elapsed.as_secs_f64().max(1e-6);
    let host_hz = rate_hz(ticks.saturating_sub(g.ticks_at_start), elapsed_secs);
    let portal_fps = rate_hz(portal_now.saturating_sub(g.portal_at_start), elapsed_secs);
    g.window_start = Some(Instant::now());
    g.ticks_at_start = ticks;
    g.portal_at_start = portal_now;
    drop(g);

    let host_s = format!("{host_hz:.1}");
    let portal_s = format!("{portal_fps:.1}");
    let faces = snap.mapped_faces.to_string();
    let hits = snap.mapped_hits.to_string();
    let poll = snap.poll_ms.to_string();
    let ct = if snap.click_through { "on" } else { "off" };
    event_log(
        "SQYRE_OVERLAY_FPS",
        &[
            ("overlay_host_hz", &host_s),
            ("portal_fps", &portal_s),
            ("mapped_faces", &faces),
            ("mapped_hits", &hits),
            ("click_through", ct),
            ("poll_ms", &poll),
        ],
    );
}

/// Reset counters (tests).
#[cfg(test)]
pub fn reset_for_test() {
    HOST_TICKS.store(0, Ordering::SeqCst);
    *STATE.lock() = SampleState::new();
}

/// Announce once when the host starts under diag (optional breadcrumb).
pub fn note_enabled_once() {
    static LOGGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if !disk_logging_enabled() {
        return;
    }
    if LOGGED.swap(true, Ordering::Relaxed) {
        return;
    }
    note(&format!(
        "overlay-fps: sampling on (interval_ms={SAMPLE_MS}; portal_fps=stream0 when ScreenCast live)"
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serialize tests that toggle the process-wide diag disk override.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn rate_hz_divides_count_by_elapsed() {
        assert!((rate_hz(60, 1.0) - 60.0).abs() < 1e-9);
        assert!((rate_hz(120, 2.0) - 60.0).abs() < 1e-9);
        assert_eq!(rate_hz(10, 0.0), 0.0);
    }

    #[test]
    fn host_tick_is_noop_without_diag() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_for_test();
        sqyre_ports::set_disk_logging(Some(false));
        for _ in 0..5 {
            host_tick(OverlayFpsSnapshot {
                mapped_faces: 1,
                mapped_hits: 1,
                click_through: false,
                poll_ms: 16,
            });
        }
        assert_eq!(HOST_TICKS.load(Ordering::Relaxed), 0);
        sqyre_ports::set_disk_logging(None);
    }

    #[test]
    fn host_tick_counts_when_diag_on() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_for_test();
        let dir = std::env::temp_dir().join(format!("sqyre-overlay-fps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        sqyre_ports::set_log_dir(Some(dir.clone()));
        sqyre_ports::set_disk_logging(Some(true));
        host_tick(OverlayFpsSnapshot {
            mapped_faces: 1,
            mapped_hits: 0,
            click_through: true,
            poll_ms: 50,
        });
        assert_eq!(HOST_TICKS.load(Ordering::Relaxed), 1);
        sqyre_ports::set_disk_logging(None);
        sqyre_ports::set_log_dir(None);
        let _ = std::fs::remove_dir_all(&dir);
        reset_for_test();
    }
}
