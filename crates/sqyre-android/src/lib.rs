//! Bridge between Sqyre and the Kotlin Android shell (`android/`).
//!
//! The only crate that talks JNI. Frame storage, gesture planning, and status decoding
//! are plain Rust so they build and test on the host; [`bridge`] exists only on Android.

mod apps;
mod error;
mod frame;
mod insets;
mod picks;
mod pointer;
pub mod status;

#[cfg(target_os = "android")]
pub mod bridge;

pub use apps::{parse_app_line, parse_app_list, AppIcon, LaunchableApp};
pub use error::AndroidError;
pub use frame::{Frame, FrameLayout, FrameStore, Projection, DEMAND_WINDOW};
pub use insets::{InsetStore, Insets};
pub use picks::DocumentPicks;
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

static PICKS: DocumentPicks = DocumentPicks::new();

/// Process-wide document pick slot answered by the shell.
pub fn picks() -> &'static DocumentPicks {
    &PICKS
}

static INSETS: InsetStore = InsetStore::new();

/// Process-wide window insets reported by the shell.
pub fn insets() -> &'static InsetStore {
    &INSETS
}

pub type ShellHandler = Arc<dyn Fn() + Send + Sync>;

/// One notification action's callback, replaced when the app reloads.
struct HandlerSlot(Mutex<Option<ShellHandler>>);

impl HandlerSlot {
    const fn new() -> Self {
        Self(Mutex::new(None))
    }

    fn set(&self, handler: ShellHandler) {
        *self.0.lock() = Some(handler);
    }

    /// No-op before a handler is set. The lock is released before the call.
    fn fire(&self) {
        let handler = self.0.lock().clone();
        if let Some(handler) = handler {
            handler();
        }
    }
}

static STOP_HANDLER: HandlerSlot = HandlerSlot::new();
static CONTINUE_HANDLER: HandlerSlot = HandlerSlot::new();

/// Called when the user taps Stop on the screen-recording notification.
pub fn set_stop_handler(handler: ShellHandler) {
    STOP_HANDLER.set(handler);
}

/// Forward a Stop tap to the registered handler.
pub fn request_stop() {
    STOP_HANDLER.fire();
}

/// Called when the user taps Continue on the screen-recording notification.
pub fn set_continue_handler(handler: ShellHandler) {
    CONTINUE_HANDLER.set(handler);
}

/// Forward a Continue tap to the registered handler.
pub fn request_continue() {
    CONTINUE_HANDLER.fire();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn counting_handler() -> (ShellHandler, Arc<AtomicUsize>) {
        let hits = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&hits);
        let handler: ShellHandler = Arc::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        });
        (handler, hits)
    }

    #[test]
    fn slot_reaches_the_latest_handler() {
        let slot = HandlerSlot::new();
        slot.fire();
        let (first, first_hits) = counting_handler();
        slot.set(first);
        slot.fire();
        let (second, second_hits) = counting_handler();
        slot.set(second);
        slot.fire();
        slot.fire();
        assert_eq!(first_hits.load(Ordering::SeqCst), 1);
        assert_eq!(second_hits.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn stop_and_continue_use_separate_handlers() {
        let (stop, stop_hits) = counting_handler();
        let (cont, cont_hits) = counting_handler();
        set_stop_handler(stop);
        set_continue_handler(cont);
        request_continue();
        request_continue();
        request_stop();
        assert_eq!(stop_hits.load(Ordering::SeqCst), 1);
        assert_eq!(cont_hits.load(Ordering::SeqCst), 2);
    }
}
