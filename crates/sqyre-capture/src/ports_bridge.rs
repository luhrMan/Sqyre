//! Register capture session status with [`sqyre_ports`] so UI/probe avoid
//! depending on this crate for status queries alone.

use std::sync::{Arc, Once};

/// Install ports capture-status hooks once. Safe to call repeatedly.
pub fn wire_ports_capture_status() {
    static ONCE: Once = Once::new();
    ONCE.call_once(install);
}

fn install() {
    use sqyre_ports::CaptureStatusHooks;

    sqyre_ports::install_capture_status(CaptureStatusHooks {
        open_may_block: Some(Arc::new(crate::shared_capturer_open_may_block)),
        is_opening: Some(Arc::new(crate::shared_capturer_is_opening)),
        capturer_ready: Some(Arc::new(|| {
            matches!(crate::shared_capturer_if_ready(), Some(Ok(_)))
        })),
        capturer_dead: Some(Arc::new(|| {
            crate::shared_capturer_if_ready()
                .and_then(|r| r.ok())
                .is_some_and(|c| c.is_dead())
        })),
        screencast_granted: Some(Arc::new(crate::portal_screencast_granted)),
        input_ready: Some(Arc::new(crate::portal_input_ready)),
        pipewire_waking: Some(Arc::new(crate::pipewire_stream_waking)),
        session_warning: Some(Arc::new(crate::linux_session_capture_warning)),
        portal_cursor: Some(Arc::new(crate::portal_cursor_position)),
        desktop_cursor: Some(Arc::new(crate::desktop_cursor_position)),
        preferred_monitors: Some(Arc::new(crate::preferred_monitor_rects)),
        request_screencast_picker: Some(Arc::new(crate::request_portal_screencast_picker)),
        revoke_grants: Some(Arc::new(crate::revoke_portal_grants)),
        nudge_after_ui_hide: Some(Arc::new(crate::nudge_portal_capture_after_ui_hide)),
        release_frame_cache: Some(Arc::new(crate::release_capture_frame_cache)),
        release_idle_frame_cache: Some(Arc::new(crate::release_idle_capture_frame_cache)),
        set_pipewire_suspend: Some(Arc::new(crate::set_pipewire_suspend_on_idle)),
        set_overlay_in_detection: Some(Arc::new(crate::set_overlay_buttons_in_detection)),
    });
}
