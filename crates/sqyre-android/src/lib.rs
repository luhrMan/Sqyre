//! Bridge between Sqyre and the Kotlin Android shell (`android/`).
//!
//! The only crate that talks JNI. Frame storage, gesture planning, and status decoding
//! are plain Rust so they build and test on the host; [`bridge`] exists only on Android.

mod error;
mod frame;
mod pointer;
pub mod status;

#[cfg(target_os = "android")]
pub mod bridge;

pub use error::AndroidError;
pub use frame::{Frame, FrameLayout, FrameStore, Projection, DEMAND_WINDOW};
pub use pointer::{
    Gesture, PointerError, PointerPlanner, LONG_PRESS_MS, MAX_GESTURE_MS, SCROLL_MS, TAP_MS,
};

use parking_lot::Mutex;
use std::sync::Arc;

static FRAMES: FrameStore = FrameStore::new();

/// Process-wide projection frame store fed by the shell.
pub fn frames() -> &'static FrameStore {
    &FRAMES
}

type StopHandler = Arc<dyn Fn() + Send + Sync>;

static STOP_HANDLER: Mutex<Option<StopHandler>> = Mutex::new(None);

/// Called when the user taps Stop on the screen-recording notification.
pub fn set_stop_handler(handler: StopHandler) {
    *STOP_HANDLER.lock() = Some(handler);
}

/// Forward a Stop tap to the registered handler (no-op before one is set).
pub fn request_stop() {
    let handler = STOP_HANDLER.lock().clone();
    if let Some(handler) = handler {
        handler();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn stop_reaches_the_latest_handler() {
        request_stop();
        let hits = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&hits);
        set_stop_handler(Arc::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        }));
        request_stop();
        request_stop();
        assert_eq!(hits.load(Ordering::SeqCst), 2);
    }
}
