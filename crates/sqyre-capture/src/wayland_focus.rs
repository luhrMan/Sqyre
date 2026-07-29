//! Wayland window list / focus via foreign-toplevel when available.
//!
//! Most compositors do not expose EWMH-equivalent APIs to sandboxed clients.
//! This backend reports clear [`AutomationError::PortalUnavailable`] /
//! [`CaptureError::PortalUnavailable`] until a compositor-specific foreign-toplevel
//! client is wired; permission gating still applies.

use crate::wayland_permissions;
use crate::{CaptureError, ProcessIcon, WindowInfo};
use sqyre_ports::{AutomationError, WindowFocuser};

/// Overlay viewport title (shared with X11 path for egui window matching).
pub const OVERLAY_WM_TITLE: &str = "sqyre-overlay";

/// Wayland window focuser (best-effort; gated by settings).
#[derive(Debug, Default, Clone, Copy)]
pub struct WaylandWindowFocuser;

impl WindowFocuser for WaylandWindowFocuser {
    fn focus(&self, process_path: &str, window_title: &str) -> Result<(), AutomationError> {
        if !wayland_permissions::window_management_enabled() {
            return Err(AutomationError::PermissionDenied {
                capability: "window management",
            });
        }
        let _ = (process_path, window_title);
        Err(AutomationError::PortalUnavailable(
            "Focus Window on Wayland requires foreign-toplevel / activation support on this desktop (GNOME/KDE). Enable window management in Settings after your compositor exposes toplevel listing.".into(),
        ))
    }
}

pub fn list_open_windows() -> Result<Vec<WindowInfo>, CaptureError> {
    if !wayland_permissions::window_management_enabled() {
        return Err(CaptureError::PermissionDenied {
            capability: "window management",
        });
    }
    Err(CaptureError::PortalUnavailable(
        "Window listing is not available on this Wayland compositor yet".into(),
    ))
}

pub fn get_active_window() -> Result<Option<WindowInfo>, CaptureError> {
    if !wayland_permissions::window_management_enabled() {
        return Err(CaptureError::PermissionDenied {
            capability: "window management",
        });
    }
    // Soft-fail: overlays treat missing focus as "no program match".
    Ok(None)
}

pub fn process_icon(_process_path: &str, _window_title: &str) -> Option<ProcessIcon> {
    None
}

pub fn skip_taskbar_for_overlay_windows() -> Result<(), CaptureError> {
    // Layer-shell / xdg roles handle this; no EWMH skip-taskbar on Wayland.
    Ok(())
}
