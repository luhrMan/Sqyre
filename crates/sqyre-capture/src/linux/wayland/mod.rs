//! Wayland-native backends (portal ScreenCast, foreign-toplevel, layer-shell).

mod app_resolve;
mod atspi_windows;
mod foreign_toplevel;
mod windows;

#[cfg(feature = "portal-capture")]
mod eis;
#[cfg(feature = "portal-capture")]
mod portal_capture;

#[cfg(feature = "portal-capture")]
pub use portal_capture::{
    portal_input_click, portal_input_key, portal_input_move, portal_input_ready,
    portal_input_scroll, portal_remote_desktop_granted, portal_screencast_granted,
    request_portal_screencast_picker, PortalCapturer,
};

pub use windows::OsWindowFocuser;
pub(crate) use windows::{get_active_window, list_open_windows};

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

/// Probe: Wayland foreign-toplevel and/or AT-SPI window listing is usable.
pub fn toplevel_focus_available() -> Result<(), CaptureError> {
    windows::toplevel_focus_available()
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
