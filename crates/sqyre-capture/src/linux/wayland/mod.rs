//! Wayland-native backends (portal ScreenCast, foreign-toplevel, layer-shell).

mod app_resolve;
mod atspi_windows;
mod foreign_toplevel;
mod layer_shell;
mod wayland_clients;
mod windows;

#[cfg(feature = "portal-capture")]
mod compositor_kick;
#[cfg(feature = "portal-capture")]
mod eis;
#[cfg(feature = "portal-capture")]
mod portal_dma;
#[cfg(feature = "portal-capture")]
mod portal_pipewire;
#[cfg(feature = "portal-capture")]
mod portal_remote;
#[cfg(feature = "portal-capture")]
mod portal_session;

#[cfg(feature = "portal-capture")]
pub use portal_pipewire::PortalCapturer;
#[cfg(feature = "portal-capture")]
pub use portal_remote::PortalEisInput;
#[cfg(feature = "portal-capture")]
pub use portal_session::{
    portal_cursor_position, portal_input_click, portal_input_key, portal_input_last_pos,
    portal_input_move, portal_input_ready, portal_input_scroll, portal_remote_desktop_granted,
    portal_screencast_granted, request_portal_screencast_picker, revoke_portal_grants,
};

pub use layer_shell::{layer_shell_available, prefers_layer_shell_overlay_session};
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

/// Probe: wlr-layer-shell is available for native Wayland overlays.
pub fn layer_outline_available() -> Result<(), CaptureError> {
    layer_shell::layer_shell_available()
}

/// Probe: compositor pointer grab can use layer-shell (same global as outline).
pub fn layer_grab_available() -> Result<(), CaptureError> {
    layer_shell::layer_shell_available()
}
