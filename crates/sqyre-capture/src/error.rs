//! Human-readable warning when the Linux session needs attention.

pub use sqyre_ports::CaptureError;

/// Advisory when pure Wayland is active (permissions / portal backends apply).
///
/// Returns `None` on non-Linux targets and when `DISPLAY` is available (X11 path).
#[cfg(target_os = "linux")]
pub fn linux_session_capture_warning() -> Option<String> {
    if crate::linux_session::is_wayland_backend() {
        Some(
            "Pure Wayland session detected. Sqyre uses XDG Desktop Portals for capture, input, and shortcuts — grant permissions on first start or in User Settings."
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
