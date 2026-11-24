//! Panic containment for OS FFI callbacks (Win32 wndprocs / enum procs, …).
//!
//! Unwinding across `extern "system"` is undefined and typically aborts. Call
//! [`catch_ffi`] around callback bodies so a Rust panic is logged via diag and
//! the callback returns a safe fallback instead.
//!
//! # Panic profile
//!
//! This uses [`std::panic::catch_unwind`]. Builds with `panic = "abort"` make
//! `catch_unwind` a no-op: the process still aborts and these guards cannot
//! recover. Sqyre keeps `panic = "unwind"` in release; do not switch the
//! profile without replacing this strategy.

use crate::diag::{mark_site, note};

/// Run `f` inside `catch_unwind`. On panic: [`mark_site`], [`note`], return `fallback`.
///
/// Prefer calling `DefWindowProcW` / `CallNextHookEx` / continuing an enum
/// *outside* the closure when those are the correct post-panic returns — pass
/// that value (or compute it in the caller after a sentinel) as `fallback`.
pub fn catch_ffi<R>(site: &str, fallback: R, f: impl FnOnce() -> R) -> R {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(v) => v,
        Err(_) => {
            mark_site(site);
            note(&format!("ffi panic at {site}"));
            fallback
        }
    }
}

#[cfg(test)]
mod tests {
    use super::catch_ffi;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn catch_ffi_returns_ok_value() {
        assert_eq!(catch_ffi("test.ok", -1, || 42), 42);
    }

    #[test]
    fn catch_ffi_returns_fallback_on_panic() {
        // Serialize with diag tests: catch path calls mark_site on LAST_SITE.
        let _guard = crate::diag::DIAG_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let hit = AtomicBool::new(false);
        let out = catch_ffi("test.panic", 7, || {
            hit.store(true, Ordering::Relaxed);
            panic!("ffi test panic");
        });
        assert!(hit.load(Ordering::Relaxed));
        assert_eq!(out, 7);
    }
}
