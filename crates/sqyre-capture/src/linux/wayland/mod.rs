//! Wayland-native backends (portal ScreenCast, foreign-toplevel, layer-shell).

#[cfg(feature = "portal-capture")]
mod portal_capture;

#[cfg(feature = "portal-capture")]
pub use portal_capture::{
    portal_screencast_granted, request_portal_screencast_picker, PortalCapturer,
};

use sqyre_ports::CaptureError;

/// Whether a Wayland portal capture backend is wired into Sqyre.
pub fn portal_capture_implemented() -> bool {
    cfg!(feature = "portal-capture")
}

/// Probe-only: whether this build includes portal capture.
pub fn portal_capture_available() -> Result<(), CaptureError> {
    if portal_capture_implemented() {
        Ok(())
    } else {
        Err(CaptureError::Message(
            "portal capture not enabled in this build (missing portal-capture feature)".into(),
        ))
    }
}

/// Probe-only: foreign-toplevel window management (not implemented).
pub fn toplevel_focus_available() -> Result<(), CaptureError> {
    Err(CaptureError::Message(
        "Wayland foreign-toplevel focus not implemented yet".into(),
    ))
}

/// Probe-only: wlr-layer-shell overlays (not implemented).
pub fn layer_outline_available() -> Result<(), CaptureError> {
    Err(CaptureError::Message(
        "Wayland layer-shell outline not implemented yet".into(),
    ))
}

/// Probe-only: compositor pointer grab via layer-shell (not implemented).
pub fn layer_grab_available() -> Result<(), CaptureError> {
    Err(CaptureError::Message(
        "Wayland layer-shell grab not implemented yet".into(),
    ))
}
