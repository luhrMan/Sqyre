//! Per-thread choice to capture with an accessibility screenshot.

use std::cell::Cell;
use std::time::{Duration, Instant};

thread_local! {
    static PREFER_SCREENSHOT: Cell<bool> = const { Cell::new(false) };
}

/// Set for the macro thread while that run should call `takeScreenshot`.
pub struct ScreenshotGuard;

impl Drop for ScreenshotGuard {
    fn drop(&mut self) {
        PREFER_SCREENSHOT.with(|flag| flag.set(false));
    }
}

/// Later captures on this thread use screenshots until the guard drops.
pub fn enter_screenshot_capture() -> ScreenshotGuard {
    PREFER_SCREENSHOT.with(|flag| flag.set(true));
    ScreenshotGuard
}

/// True on a thread that [enter_screenshot_capture] armed.
pub fn prefer_screenshot() -> bool {
    PREFER_SCREENSHOT.with(|flag| flag.get())
}

/// How long to wait so a new screenshot stays at least `min` after `last`.
pub fn shot_delay(last: Option<Instant>, now: Instant, min: Duration) -> Duration {
    match last {
        Some(taken) => min.saturating_sub(now.saturating_duration_since(taken)),
        None => Duration::ZERO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn guard_is_thread_local() {
        assert!(!prefer_screenshot());
        let guard = enter_screenshot_capture();
        assert!(prefer_screenshot());
        let seen = std::thread::spawn(prefer_screenshot).join().expect("join");
        assert!(!seen);
        drop(guard);
        assert!(!prefer_screenshot());
    }

    #[test]
    fn shot_delay_waits_out_the_remainder() {
        let start = Instant::now();
        let min = Duration::from_millis(333);
        assert_eq!(shot_delay(None, start, min), Duration::ZERO);
        assert_eq!(
            shot_delay(Some(start), start + Duration::from_millis(100), min),
            Duration::from_millis(233)
        );
        assert_eq!(
            shot_delay(Some(start), start + Duration::from_millis(333), min),
            Duration::ZERO
        );
        assert_eq!(
            shot_delay(Some(start), start + Duration::from_millis(1000), min),
            Duration::ZERO
        );
    }
}
