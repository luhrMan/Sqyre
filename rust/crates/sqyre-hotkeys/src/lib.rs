//! Global hotkey service with injectable no-op stub (Go `nohook` pattern).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Callbacks invoked from the hook thread (keep short).
#[derive(Clone)]
pub struct HotkeyCallbacks {
    pub on_escape_stop: Arc<dyn Fn() + Send + Sync>,
    pub on_failsafe: Arc<dyn Fn() + Send + Sync>,
}

impl Default for HotkeyCallbacks {
    fn default() -> Self {
        Self {
            on_escape_stop: Arc::new(|| {}),
            on_failsafe: Arc::new(|| {}),
        }
    }
}

/// Shared stop flag used by app + executor.
#[derive(Clone, Default)]
pub struct StopFlag(Arc<AtomicBool>);

impl StopFlag {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
    pub fn request_stop(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    pub fn clear(&self) {
        self.0.store(false, Ordering::SeqCst);
    }
    pub fn is_stopped(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
    pub fn raw(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.0)
    }
}

pub trait HotkeyService: Send {
    fn start(&mut self, callbacks: HotkeyCallbacks) -> Result<(), String>;
    fn stop(&mut self);
}

/// Always-available stub (CI / tests).
#[derive(Debug, Default)]
pub struct NullHotkeys {
    running: bool,
}

impl HotkeyService for NullHotkeys {
    fn start(&mut self, _callbacks: HotkeyCallbacks) -> Result<(), String> {
        self.running = true;
        Ok(())
    }
    fn stop(&mut self) {
        self.running = false;
    }
}

#[cfg(feature = "hooks")]
mod hooks;

#[cfg(feature = "hooks")]
pub use hooks::RdevHotkeys;

/// Pick hooks when feature enabled, otherwise null.
pub fn default_hotkeys() -> Box<dyn HotkeyService> {
    #[cfg(feature = "hooks")]
    {
        Box::new(RdevHotkeys::default())
    }
    #[cfg(not(feature = "hooks"))]
    {
        Box::new(NullHotkeys::default())
    }
}

/// Press-latch helpers (Go `hotkeytrigger`) for macro chord logic.
pub mod latch {
    use parking_lot::Mutex;

    pub fn try_acquire(mu: &Mutex<bool>) -> bool {
        let mut g = mu.lock();
        if *g {
            return false;
        }
        *g = true;
        true
    }

    pub fn clear(mu: &Mutex<bool>) {
        *mu.lock() = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_flag_roundtrip() {
        let f = StopFlag::new();
        assert!(!f.is_stopped());
        f.request_stop();
        assert!(f.is_stopped());
        f.clear();
        assert!(!f.is_stopped());
    }

    #[test]
    fn null_hotkeys_start_stop() {
        let mut h = NullHotkeys::default();
        h.start(HotkeyCallbacks::default()).unwrap();
        h.stop();
    }

    #[test]
    fn latch_blocks_repeat() {
        let m = parking_lot::Mutex::new(false);
        assert!(latch::try_acquire(&m));
        assert!(!latch::try_acquire(&m));
        latch::clear(&m);
        assert!(latch::try_acquire(&m));
    }
}
