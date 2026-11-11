//! Wake the egui loop when an overlay click is queued.
//!
//! Overlay fires only drain on a ROOT frame. On GNOME Wayland an unfocused
//! idle window often ignores a single `request_repaint` (same as macro hotkeys),
//! so we nudge until the shared pending queue empties.

use egui::Context;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

static NUDGING: AtomicBool = AtomicBool::new(false);

const NUDGE_ITERS: u32 = 100;
const NUDGE_MS: u64 = 20;

/// `request_repaint` now and keep nudging until `pending` is empty.
pub(crate) fn wake_until_drained(ctx: &Context, pending: &Arc<Mutex<Vec<String>>>) {
    ctx.request_repaint();
    start_nudge(ctx.clone(), Arc::clone(pending));
}

fn start_nudge(ctx: Context, pending: Arc<Mutex<Vec<String>>>) {
    if NUDGING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    let _ = thread::Builder::new()
        .name("sqyre-overlay-wake".into())
        .spawn(move || {
            for _ in 0..NUDGE_ITERS {
                if pending.lock().is_empty() {
                    break;
                }
                ctx.request_repaint();
                thread::sleep(Duration::from_millis(NUDGE_MS));
            }
            NUDGING.store(false, Ordering::SeqCst);
            if !pending.lock().is_empty() {
                ctx.request_repaint();
                start_nudge(ctx, pending);
            }
        });
}
