//! Capture session status for UI / probe without depending on `sqyre-capture`.
//!
//! The capture crate installs real backends via [`install_capture_status`]. Until
//! then every query returns a safe idle default (closed, not opening, not granted).

use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;

type BoolFn = Arc<dyn Fn() -> bool + Send + Sync>;
type OptPosFn = Arc<dyn Fn() -> Option<(i32, i32)> + Send + Sync>;
type OptStrFn = Arc<dyn Fn() -> Option<String> + Send + Sync>;
type ActionFn = Arc<dyn Fn() + Send + Sync>;
type SuspendFn = Arc<dyn Fn(bool) + Send + Sync>;
type OverlayDetectFn = Arc<dyn Fn(bool) + Send + Sync>;
type IdleCacheFn = Arc<dyn Fn(Duration) -> Option<Duration> + Send + Sync>;
type MonitorRectsFn = Arc<dyn Fn() -> Vec<crate::DesktopRect> + Send + Sync>;

/// Hooks the capture adapter registers once at process start.
#[derive(Default, Clone)]
pub struct CaptureStatusHooks {
    pub open_may_block: Option<BoolFn>,
    pub is_opening: Option<BoolFn>,
    pub capturer_ready: Option<BoolFn>,
    pub capturer_dead: Option<BoolFn>,
    pub screencast_granted: Option<BoolFn>,
    pub input_ready: Option<BoolFn>,
    pub pipewire_waking: Option<BoolFn>,
    pub session_warning: Option<OptStrFn>,
    pub portal_cursor: Option<OptPosFn>,
    pub desktop_cursor: Option<OptPosFn>,
    pub preferred_monitors: Option<MonitorRectsFn>,
    pub request_screencast_picker: Option<ActionFn>,
    pub revoke_grants: Option<ActionFn>,
    pub nudge_after_ui_hide: Option<ActionFn>,
    pub release_frame_cache: Option<ActionFn>,
    pub release_idle_frame_cache: Option<IdleCacheFn>,
    pub set_pipewire_suspend: Option<SuspendFn>,
    pub set_overlay_in_detection: Option<OverlayDetectFn>,
}

static HOOKS: Mutex<CaptureStatusHooks> = Mutex::new(CaptureStatusHooks {
    open_may_block: None,
    is_opening: None,
    capturer_ready: None,
    capturer_dead: None,
    screencast_granted: None,
    input_ready: None,
    pipewire_waking: None,
    session_warning: None,
    portal_cursor: None,
    desktop_cursor: None,
    preferred_monitors: None,
    request_screencast_picker: None,
    revoke_grants: None,
    nudge_after_ui_hide: None,
    release_frame_cache: None,
    release_idle_frame_cache: None,
    set_pipewire_suspend: None,
    set_overlay_in_detection: None,
});

/// Replace capture-status backends (capture crate calls this at startup).
pub fn install_capture_status(hooks: CaptureStatusHooks) {
    *HOOKS.lock() = hooks;
}

fn hooks() -> CaptureStatusHooks {
    HOOKS.lock().clone()
}

/// True when opening the shared capturer may block on a portal permission dialog.
pub fn shared_capturer_open_may_block() -> bool {
    hooks().open_may_block.map(|f| f()).unwrap_or(false)
}

/// True while the shared capturer open is in flight.
pub fn shared_capturer_is_opening() -> bool {
    hooks().is_opening.map(|f| f()).unwrap_or(false)
}

/// True when a live shared capturer is ready (not merely opening / failed).
pub fn shared_capturer_ready() -> bool {
    hooks().capturer_ready.map(|f| f()).unwrap_or(false)
}

/// True when the shared capturer is open but its stream/session is dead.
pub fn shared_capturer_dead() -> bool {
    hooks().capturer_dead.map(|f| f()).unwrap_or(false)
}

/// ScreenCast grant present, restore token stored, or capturer already open.
pub fn portal_screencast_granted() -> bool {
    hooks().screencast_granted.map(|f| f()).unwrap_or(false)
}

/// Wayland EIS input backend is connected.
pub fn portal_input_ready() -> bool {
    hooks().input_ready.map(|f| f()).unwrap_or(false)
}

/// True while a slept portal PipeWire stream is waking for a capture.
pub fn pipewire_stream_waking() -> bool {
    hooks().pipewire_waking.map(|f| f()).unwrap_or(false)
}

/// Human-readable Linux session capture warning, if any.
pub fn linux_session_capture_warning() -> Option<String> {
    hooks().session_warning.and_then(|f| f())
}

/// Compositor pointer from ScreenCast cursor metadata (Wayland).
pub fn portal_cursor_position() -> Option<(i32, i32)> {
    hooks().portal_cursor.and_then(|f| f())
}

/// Absolute desktop cursor in physical pixels.
pub fn desktop_cursor_position() -> Option<(i32, i32)> {
    hooks().desktop_cursor.and_then(|f| f())
}

/// Preferred monitor rectangles (primary first when known).
pub fn preferred_monitor_rects() -> Vec<crate::DesktopRect> {
    hooks().preferred_monitors.map(|f| f()).unwrap_or_default()
}

/// Show the portal ScreenCast picker again. No-op until capture installs hooks.
pub fn request_portal_screencast_picker() {
    if let Some(f) = hooks().request_screencast_picker {
        f();
    }
}

/// Drop the live portal session and forget persisted grants.
pub fn revoke_portal_grants() {
    if let Some(f) = hooks().revoke_grants {
        f();
    }
}

/// After unmapping the main window, wait for a fresh portal frame.
pub fn nudge_portal_capture_after_ui_hide() {
    if let Some(f) = hooks().nudge_after_ui_hide {
        f();
    }
}

/// Release the portal CPU frame mirror after a macro.
pub fn release_capture_frame_cache() {
    if let Some(f) = hooks().release_frame_cache {
        f();
    }
}

/// Release the portal CPU frame mirror once idle for `idle`.
pub fn release_idle_capture_frame_cache(idle: Duration) -> Option<Duration> {
    hooks().release_idle_frame_cache.and_then(|f| f(idle))
}

/// Whether portal PipeWire nodes suspend after the desktop sits idle.
pub fn set_pipewire_suspend_on_idle(suspend: bool) {
    if let Some(f) = hooks().set_pipewire_suspend {
        f(suspend);
    }
}

/// Publish the "overlay buttons appear in detection" setting.
pub fn set_overlay_buttons_in_detection(include: bool) {
    if let Some(f) = hooks().set_overlay_in_detection {
        f(include);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn defaults_are_idle_before_install() {
        install_capture_status(CaptureStatusHooks::default());
        assert!(!shared_capturer_open_may_block());
        assert!(!shared_capturer_is_opening());
        assert!(!shared_capturer_ready());
        assert!(!portal_screencast_granted());
        assert!(!portal_input_ready());
        assert!(!pipewire_stream_waking());
        assert!(linux_session_capture_warning().is_none());
        assert!(portal_cursor_position().is_none());
        assert!(desktop_cursor_position().is_none());
        assert!(preferred_monitor_rects().is_empty());
        assert!(release_idle_capture_frame_cache(Duration::from_secs(1)).is_none());
    }

    #[test]
    fn installed_hooks_are_invoked() {
        let flag = Arc::new(AtomicBool::new(false));
        let flag_r = Arc::clone(&flag);
        install_capture_status(CaptureStatusHooks {
            open_may_block: Some(Arc::new(|| true)),
            capturer_ready: Some(Arc::new(|| true)),
            request_screencast_picker: Some(Arc::new(move || {
                flag_r.store(true, Ordering::SeqCst);
            })),
            ..CaptureStatusHooks::default()
        });
        assert!(shared_capturer_open_may_block());
        assert!(shared_capturer_ready());
        request_portal_screencast_picker();
        assert!(flag.load(Ordering::SeqCst));
        install_capture_status(CaptureStatusHooks::default());
    }
}
