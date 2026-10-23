//! Overlay button taps and relocate drags, delivered from the accessibility windows.
//!
//! The Kotlin shell calls [`overlay_clicked`] / [`overlay_moved`] on the main thread.
//! Handlers are installed by the overlay host and the app; both run after the lock
//! is released.

use parking_lot::Mutex;
use std::sync::Arc;

pub type OverlayClickHandler = Arc<dyn Fn(&str) + Send + Sync>;
pub type OverlayMoveHandler = Arc<dyn Fn(&str, i32, i32) + Send + Sync>;
pub type OverlayRunHandler = Arc<dyn Fn(&str) + Send + Sync>;

static CLICK: Mutex<Option<OverlayClickHandler>> = Mutex::new(None);
static MOVED: Mutex<Option<OverlayMoveHandler>> = Mutex::new(None);
static RUN: Mutex<Option<OverlayRunHandler>> = Mutex::new(None);

pub fn set_overlay_click_handler(handler: OverlayClickHandler) {
    *CLICK.lock() = Some(handler);
}

pub fn set_overlay_move_handler(handler: OverlayMoveHandler) {
    *MOVED.lock() = Some(handler);
}

pub fn clear_overlay_input_handlers() {
    *CLICK.lock() = None;
    *MOVED.lock() = None;
}

pub fn clear_overlay_run_handler() {
    *RUN.lock() = None;
}

/// App-level "start this macro" used when a button is tapped.
pub fn set_overlay_run_handler(handler: OverlayRunHandler) {
    *RUN.lock() = Some(handler);
}

pub fn overlay_clicked(id: &str) {
    let handler = CLICK.lock().clone();
    if let Some(handler) = handler {
        handler(id);
    }
}

pub fn overlay_moved(id: &str, x: i32, y: i32) {
    let handler = MOVED.lock().clone();
    if let Some(handler) = handler {
        handler(id, x, y);
    }
}

pub fn request_overlay_run(macro_name: &str) {
    let handler = RUN.lock().clone();
    if let Some(handler) = handler {
        handler(macro_name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn click_and_move_reach_the_installed_handlers() {
        let clicks = Arc::new(AtomicUsize::new(0));
        let moves = Arc::new(AtomicUsize::new(0));
        let runs = Arc::new(AtomicUsize::new(0));
        let click_hits = Arc::clone(&clicks);
        set_overlay_click_handler(Arc::new(move |id| {
            assert_eq!(id, "btn");
            click_hits.fetch_add(1, Ordering::SeqCst);
        }));
        let move_hits = Arc::clone(&moves);
        set_overlay_move_handler(Arc::new(move |id, x, y| {
            assert_eq!((id, x, y), ("btn", 3, 4));
            move_hits.fetch_add(1, Ordering::SeqCst);
        }));
        let run_hits = Arc::clone(&runs);
        set_overlay_run_handler(Arc::new(move |name| {
            assert_eq!(name, "Go");
            run_hits.fetch_add(1, Ordering::SeqCst);
        }));
        overlay_clicked("btn");
        overlay_moved("btn", 3, 4);
        request_overlay_run("Go");
        clear_overlay_input_handlers();
        overlay_clicked("btn");
        overlay_moved("btn", 0, 0);
        assert_eq!(clicks.load(Ordering::SeqCst), 1);
        assert_eq!(moves.load(Ordering::SeqCst), 1);
        assert_eq!(runs.load(Ordering::SeqCst), 1);
        // The run handler stays installed; only the window-input pair is cleared.
        request_overlay_run("Go");
        assert_eq!(runs.load(Ordering::SeqCst), 2);
        clear_overlay_run_handler();
    }
}
