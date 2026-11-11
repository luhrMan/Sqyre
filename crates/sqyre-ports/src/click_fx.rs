//! Desktop click feedback: the executor reports where each press lands so the
//! shell can flash a marker on screen (Settings → Macros → click effect).

use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Receives click positions from the executor (UI marshals onto its thread).
pub trait ClickFxSink: Send + Sync {
    /// A mouse press landed at virtual-desktop pixel `(x, y)`.
    fn click_at(&self, x: i32, y: i32);
}

type OnClick = Arc<dyn Fn(i32, i32) + Send + Sync>;

/// Thread-safe click-effect sink for the shell.
///
/// Presses are forwarded immediately to an optional [`Self::set_on_click`]
/// listener (dedicated outline worker) so the marker does not wait for egui.
#[derive(Clone)]
pub struct SharedClickFx {
    enabled: Arc<AtomicBool>,
    on_click: Arc<Mutex<Option<OnClick>>>,
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
            on_click: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Direct listener invoked from the executor thread on each press.
    pub fn set_on_click(&self, on_click: Option<impl Fn(i32, i32) + Send + Sync + 'static>) {
        *self.on_click.lock() = on_click.map(|f| Arc::new(f) as OnClick);
    }

    /// Drop the listener (worker stopped / setting off).
    pub fn clear_on_click(&self) {
        *self.on_click.lock() = None;
    }
}

impl ClickFxSink for SharedClickFx {
    fn click_at(&self, x: i32, y: i32) {
        if !self.enabled.load(Ordering::SeqCst) {
            return;
        }
        if let Some(on_click) = self.on_click.lock().clone() {
            on_click(x, y);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn on_click_fires_when_enabled() {
        let fx = SharedClickFx::new();
        let hits = Arc::new(AtomicUsize::new(0));
        let hits2 = Arc::clone(&hits);
        fx.set_on_click(Some(move |x, y| {
            assert_eq!((x, y), (10, 20));
            hits2.fetch_add(1, Ordering::SeqCst);
        }));
        fx.click_at(10, 20);
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn disabled_skips_listener() {
        let fx = SharedClickFx::new();
        let hits = Arc::new(AtomicUsize::new(0));
        let hits2 = Arc::clone(&hits);
        fx.set_on_click(Some(move |_, _| {
            hits2.fetch_add(1, Ordering::SeqCst);
        }));
        fx.set_enabled(false);
        fx.click_at(1, 2);
        assert_eq!(hits.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn clear_on_click_stops_delivery() {
        let fx = SharedClickFx::new();
        let hits = Arc::new(AtomicUsize::new(0));
        let hits2 = Arc::clone(&hits);
        fx.set_on_click(Some(move |_, _| {
            hits2.fetch_add(1, Ordering::SeqCst);
        }));
        fx.clear_on_click();
        fx.click_at(1, 2);
        assert_eq!(hits.load(Ordering::SeqCst), 0);
    }
}
