//! Linux session advisory for X11 capture (typed capture errors live in `sqyre-ports`).

pub use sqyre_ports::CaptureError;

/// Human-readable warning when the Linux session cannot support capture yet.
///
/// Returns `None` on non-Linux targets and when X11/XWayland is available.
#[cfg(target_os = "linux")]
pub fn linux_session_capture_warning() -> Option<String> {
    crate::linux::LinuxSessionInfo::detect().capture_warning()
}

#[cfg(not(target_os = "linux"))]
pub fn linux_session_capture_warning() -> Option<String> {
    None
}
