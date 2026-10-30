//! Keeps overlay buttons hidden for a whole macro when detection should not see them.
//!
//! Hiding only for each grab lets the buttons flash back between attempts. The
//! app holds one span from macro start until the run thread exits. The capture
//! backend registers the hide and show hooks.

use parking_lot::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

type Hook = Arc<dyn Fn() + Send + Sync>;

static DEPTH: AtomicU32 = AtomicU32::new(0);
static ENTER: Mutex<Option<Hook>> = Mutex::new(None);
static LEAVE: Mutex<Option<Hook>> = Mutex::new(None);

/// `enter` runs as a macro starts; `leave` when the outermost one ends.
pub fn set_overlay_macro_hide_hooks(enter: Option<Hook>, leave: Option<Hook>) {
    *ENTER.lock() = enter;
    *LEAVE.lock() = leave;
}

/// True while at least one macro is inside [`OverlayMacroHide`].
pub fn overlay_macro_hide_held() -> bool {
    DEPTH.load(Ordering::SeqCst) > 0
}

/// Hide overlay buttons for one macro when "appear in detection" is off.
///
/// No-op until the capture backend installs hooks. Nested spans stay hidden
/// until the outermost one drops.
pub struct OverlayMacroHide {
    live: bool,
}

impl OverlayMacroHide {
    pub fn enter() -> Self {
        DEPTH.fetch_add(1, Ordering::SeqCst);
        let span = Self { live: true };
        let hook = ENTER.lock().clone();
        if let Some(hook) = hook {
            hook();
        }
        span
    }
}

impl Drop for OverlayMacroHide {
    fn drop(&mut self) {
        if !self.live {
            return;
        }
        self.live = false;
        let prev = DEPTH.fetch_sub(1, Ordering::SeqCst);
        if prev == 1 {
            let hook = LEAVE.lock().clone();
            if let Some(hook) = hook {
                hook();
            }
        }
    }
}
