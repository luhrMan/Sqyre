//! Wake the egui loop when a global hotkey queues a macro.
//!
//! Esc-stop writes an [`sqyre_hotkeys::StopFlag`] on the hook thread and needs no UI
//! frame. Macro start only runs on the UI thread via
//! [`crate::SqyreApp::drain_pending_hotkey_macros`]. On GNOME Wayland an unfocused
//! idle window often ignores a single `request_repaint`, so we nudge until the
//! queue drains (same idea as the recording-overlay wake poller).

use eframe::egui;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

static NUDGING: AtomicBool = AtomicBool::new(false);

const NUDGE_ITERS: u32 = 100;
const NUDGE_MS: u64 = 20;

/// Queue `name` for the UI thread and ensure egui wakes to drain it.
pub fn queue_macro_hotkey(
    pending: &Arc<Mutex<Vec<String>>>,
    repaint: &Arc<Mutex<Option<egui::Context>>>,
    name: String,
) {
    #[cfg(feature = "native-runtime")]
    {
        sqyre_capture::event_log(
            "SQYRE_HOTKEY",
            &[("fire", "queue"), ("name", name.as_str())],
        );
    }
    pending.lock().push(name);
    let Some(ctx) = repaint.lock().clone() else {
        return;
    };
    ctx.request_repaint();
    start_nudge(ctx, Arc::clone(pending));
}

fn start_nudge(ctx: egui::Context, pending: Arc<Mutex<Vec<String>>>) {
    if NUDGING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    let _ = thread::Builder::new()
        .name("sqyre-hotkey-wake".into())
        .spawn(move || {
            for _ in 0..NUDGE_ITERS {
                if pending.lock().is_empty() {
                    break;
                }
                ctx.request_repaint();
                thread::sleep(Duration::from_millis(NUDGE_MS));
            }
            NUDGING.store(false, Ordering::SeqCst);
            // A chord may have queued while we were clearing the flag.
            if !pending.lock().is_empty() {
                ctx.request_repaint();
                start_nudge(ctx, pending);
            }
        });
}
