//! Search-area selection outline while recording (Go recording selection rect).
//!
//! Driven by [`sqyre_hotkeys::ScreenClickBridge`]. Draws via direct X11 edge
//! windows ([`sqyre_capture::SelectionOutline`]) — not a fullscreen desktop
//! snapshot. A short poller owns the X11 connection so the rect keeps tracking
//! the cursor even when the main window is hidden for recording.
//!
//! Status text stays in the Data Editor; there is no coords-only HUD.

use sqyre_capture::SelectionOutline;
use sqyre_hotkeys::ScreenClickBridge;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

const POLL_MS: u64 = 16;

/// Owns the Linux outline poller and syncs it to the armed search-area draft.
#[derive(Default)]
pub struct RecordingOverlay {
    stop: Option<Arc<AtomicBool>>,
    join: Option<JoinHandle<()>>,
}

impl RecordingOverlay {
    pub fn new() -> Self {
        Self::default()
    }

    /// Call every frame. Starts the outline poller once search-area recording arms.
    pub fn sync(&mut self, screen_click: &ScreenClickBridge) {
        // Warm the X11 edges when search-area recording starts (before first click).
        if screen_click.peek_search_area_draft().is_some() {
            self.ensure_worker(screen_click.clone());
        }
    }

    fn ensure_worker(&mut self, bridge: ScreenClickBridge) {
        if self.join.is_some() {
            return;
        }
        let stop = Arc::new(AtomicBool::new(false));
        self.stop = Some(Arc::clone(&stop));
        self.join = Some(thread::spawn(move || {
            let mut outline = match SelectionOutline::open() {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("sqyre: selection outline unavailable: {e}");
                    return;
                }
            };
            while !stop.load(Ordering::Relaxed) {
                match bridge.peek_search_area_selection() {
                    Some((lx, ty, rx, by)) => outline.set_rect(lx, ty, rx, by),
                    None => outline.clear(),
                }
                thread::sleep(Duration::from_millis(POLL_MS));
            }
            outline.clear();
        }));
    }
}

impl Drop for RecordingOverlay {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            stop.store(true, Ordering::Relaxed);
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}
