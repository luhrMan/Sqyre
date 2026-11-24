//! Wayland-native backends (portal ScreenCast, foreign-toplevel, layer-shell).

mod app_resolve;
mod atspi_windows;
mod foreign_toplevel;
mod layer_shell;
pub mod layer_surface;
mod wayland_clients;
pub(crate) mod wayland_grab;
pub(crate) mod wayland_outline;
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
pub(crate) use portal_pipewire::{
    pipewire_stream_waking, set_pipewire_suspend_on_idle, stream0_frame_count,
};
#[cfg(feature = "portal-capture")]
pub use portal_remote::PortalEisInput;
#[cfg(feature = "portal-capture")]
pub use portal_session::{
    portal_cursor_position, portal_input_click, portal_input_key, portal_input_last_pos,
    portal_input_move, portal_input_ready, portal_input_scroll, portal_input_wait,
    portal_remote_desktop_granted, portal_screencast_granted, register_portal_app_now,
    request_portal_screencast_picker, revoke_portal_grants,
};

pub use layer_shell::{layer_shell_available, prefers_layer_shell_overlay_session};
pub use layer_surface::layer_shell_surface_works;
pub use windows::OsWindowFocuser;
pub(crate) use windows::{get_active_window, list_open_windows};

pub(crate) use app_resolve::{
    desktop_icon_for_app_id, desktop_icon_for_pid, process_from_pid as process_identity,
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
        Err(CaptureError::Portal(
            "not enabled in this build (missing portal-capture feature)".into(),
        ))
    }
}

/// Probe: Wayland foreign-toplevel and/or AT-SPI window listing is usable.
pub fn toplevel_focus_available() -> Result<(), CaptureError> {
    windows::toplevel_focus_available()
}

/// Probe: wlr-layer-shell can open a real overlay surface (outline/grab/buttons).
pub fn layer_outline_available() -> Result<(), CaptureError> {
    layer_surface::layer_shell_surface_works()
}

/// Probe: compositor pointer grab can use layer-shell (same surface open path).
pub fn layer_grab_available() -> Result<(), CaptureError> {
    layer_surface::layer_shell_surface_works()
}

/// Probe: overlay buttons can use a layer-shell surface.
pub fn layer_overlay_available() -> Result<(), CaptureError> {
    layer_surface::layer_shell_surface_works()
}
