//! Global Esc for Sqyre popups that may not hold keyboard focus.
//!
//! The native hotkey chooser is an override-redirect X window: under XWayland it
//! never gets key events while a native Wayland app is focused. While a popup
//! claims Esc, the hook thread latches the press here instead of stopping macros.

use std::sync::atomic::{AtomicBool, Ordering};

static CLAIMED: AtomicBool = AtomicBool::new(false);
static PRESSED: AtomicBool = AtomicBool::new(false);

/// Route the next Esc to the open popup (`true`) or back to macro stop (`false`).
pub fn claim_popup_escape(claimed: bool) {
    if CLAIMED.swap(claimed, Ordering::SeqCst) != claimed {
        PRESSED.store(false, Ordering::SeqCst);
    }
}

/// Take a latched Esc press for the claiming popup.
pub fn take_popup_escape() -> bool {
    PRESSED.swap(false, Ordering::SeqCst)
}

/// Hook thread: returns `true` when an open popup took this Esc.
#[cfg_attr(
    not(feature = "hooks"),
    allow(dead_code, reason = "called only by the hooks thread and tests")
)]
pub(crate) fn on_escape() -> bool {
    if CLAIMED.load(Ordering::SeqCst) {
        PRESSED.store(true, Ordering::SeqCst);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claim_routes_escape_and_release_clears_latch() {
        claim_popup_escape(false);
        assert!(!on_escape());
        assert!(!take_popup_escape());

        claim_popup_escape(true);
        assert!(on_escape());
        assert!(take_popup_escape());
        assert!(!take_popup_escape());

        assert!(on_escape());
        claim_popup_escape(false);
        assert!(!take_popup_escape());
    }
}
