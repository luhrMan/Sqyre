//! Desktop click feedback: the executor reports where each press lands so the
//! shell can flash a marker on screen (Settings → Macros → click effect).

use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Receives click positions from the executor (UI marshals onto its thread).
pub trait ClickFxSink: Send + Sync {
    /// A mouse press landed at virtual-desktop pixel `(x, y)`.
    fn click_at(&self, x: i32, y: i32);
}

/// Queued positions beyond this are dropped oldest-first — presses inside a run
/// are serialized, so only the newest flash matters anyway.
const MAX_PENDING: usize = 64;

/// Thread-safe click-effect queue for the egui shell.
#[derive(Clone)]
pub struct SharedClickFx {
    enabled: Arc<AtomicBool>,
    pending: Arc<Mutex<VecDeque<(i32, i32)>>>,
}

impl Default for SharedClickFx {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedClickFx {
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(true)),
            pending: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
        if !enabled {
            self.clear();
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Take every queued position (newest last).
    pub fn drain(&self) -> Vec<(i32, i32)> {
        std::mem::take(&mut *self.pending.lock()).into()
    }

    pub fn clear(&self) {
        self.pending.lock().clear();
    }
}

impl ClickFxSink for SharedClickFx {
    fn click_at(&self, x: i32, y: i32) {
        if !self.enabled.load(Ordering::SeqCst) {
            return;
        }
        let mut g = self.pending.lock();
        if g.len() >= MAX_PENDING {
            g.pop_front();
        }
        g.push_back((x, y));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queued_positions_drain_in_order() {
        let fx = SharedClickFx::new();
        fx.click_at(10, 20);
        fx.click_at(30, 40);
        assert_eq!(fx.drain(), vec![(10, 20), (30, 40)]);
        assert!(fx.drain().is_empty());
    }

    #[test]
    fn disabled_drops_events_and_clears() {
        let fx = SharedClickFx::new();
        fx.click_at(1, 2);
        fx.set_enabled(false);
        assert!(fx.drain().is_empty());
        fx.click_at(3, 4);
        assert!(fx.drain().is_empty());
    }

    #[test]
    fn queue_is_capped_oldest_first() {
        let fx = SharedClickFx::new();
        for i in 0..(MAX_PENDING as i32 + 5) {
            fx.click_at(i, i);
        }
        let drained = fx.drain();
        assert_eq!(drained.len(), MAX_PENDING);
        assert_eq!(drained.first().copied(), Some((5, 5)));
    }
}
