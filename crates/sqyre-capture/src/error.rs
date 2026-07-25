//! Linux session advisory for X11 capture (typed capture errors live in `sqyre-executor`).

pub use sqyre_executor::CaptureError;

/// Human-readable warning when the Linux session cannot support X11 capture.
///
/// Returns `None` on non-Linux targets and when `DISPLAY` is available.
#[cfg(target_os = "linux")]
pub fn linux_session_capture_warning() -> Option<String> {
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    let wayland = session.eq_ignore_ascii_case("wayland")
        || std::env::var_os("WAYLAND_DISPLAY").is_some();
    let has_x11 = std::env::var_os("DISPLAY").is_some();
    if wayland && !has_x11 {
        Some(
            "Pure Wayland session detected (no DISPLAY). Sqyre needs X11 or XWayland for screen capture, window focus, and overlays."
                .into(),
        )
    } else {
        None
    }
}

#[cfg(not(target_os = "linux"))]
pub fn linux_session_capture_warning() -> Option<String> {
    None
}
