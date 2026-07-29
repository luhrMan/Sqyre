//! Linux `OsCapturer` / focus / outline router (X11 vs Wayland).

use crate::linux_session::{is_wayland_backend, LinuxDisplayBackend};
use crate::wayland_capture::WaylandCapturer;
use crate::wayland_focus;
use crate::wayland_outline::WaylandSelectionOutline;
use crate::x11_capture::X11Capturer;
use crate::x11_focus;
use crate::x11_outline;
use crate::{CaptureError, ProcessIcon, WindowInfo};
use image::RgbaImage;
use sqyre_ports::{AutomationError, DesktopRect, RgbCapture, WindowFocuser};

crate::define_shared_run_capturer!();

/// Public Linux capturer (X11 or Wayland portal backend).
pub enum OsCapturer {
    X11(X11Capturer),
    Wayland(WaylandCapturer),
}

impl OsCapturer {
    pub fn open() -> Result<Self, CaptureError> {
        match crate::linux_session::linux_display_backend() {
            LinuxDisplayBackend::X11 => Ok(Self::X11(X11Capturer::open()?)),
            LinuxDisplayBackend::Wayland => Ok(Self::Wayland(WaylandCapturer::open()?)),
        }
    }

    pub fn capture_rect_ref(&self, rect: DesktopRect) -> Result<RgbaImage, CaptureError> {
        match self {
            Self::X11(c) => c.capture_rect_ref(rect),
            Self::Wayland(c) => c.capture_rect_ref(rect),
        }
    }

    pub fn capture_rect_rgb_ref(&self, rect: DesktopRect) -> Result<RgbCapture, CaptureError> {
        match self {
            Self::X11(c) => c.capture_rect_rgb_ref(rect),
            Self::Wayland(c) => c.capture_rect_rgb_ref(rect),
        }
    }

    pub fn virtual_bounds_ref(&self) -> Result<DesktopRect, CaptureError> {
        match self {
            Self::X11(c) => c.virtual_bounds_ref(),
            Self::Wayland(c) => c.virtual_bounds_ref(),
        }
    }

    pub fn monitor_rects_ref(&self) -> Result<Vec<DesktopRect>, CaptureError> {
        match self {
            Self::X11(c) => c.monitor_rects_ref(),
            Self::Wayland(c) => c.monitor_rects_ref(),
        }
    }

    pub fn monitor_sizes_ref(&self) -> Result<Vec<(i32, i32)>, CaptureError> {
        match self {
            Self::X11(c) => c.monitor_sizes_ref(),
            Self::Wayland(c) => c.monitor_sizes_ref(),
        }
    }
}

/// Linux window focuser (X11 EWMH or Wayland portal/foreign-toplevel).
#[derive(Debug, Default, Clone, Copy)]
pub struct OsWindowFocuser;

impl WindowFocuser for OsWindowFocuser {
    fn focus(&self, process_path: &str, window_title: &str) -> Result<(), AutomationError> {
        if is_wayland_backend() {
            wayland_focus::WaylandWindowFocuser.focus(process_path, window_title)
        } else {
            x11_focus::OsWindowFocuser.focus(process_path, window_title)
        }
    }
}

/// Selection outline (X11 override-redirect or Wayland geometry tracker).
pub enum SelectionOutline {
    X11(x11_outline::SelectionOutline),
    Wayland(WaylandSelectionOutline),
}

impl SelectionOutline {
    pub fn open() -> Result<Self, CaptureError> {
        if is_wayland_backend() {
            Ok(Self::Wayland(WaylandSelectionOutline::open()?))
        } else {
            Ok(Self::X11(x11_outline::SelectionOutline::open()?))
        }
    }

    pub fn set_rect(&mut self, left: i32, top: i32, right: i32, bottom: i32) {
        match self {
            Self::X11(o) => o.set_rect(left, top, right, bottom),
            Self::Wayland(o) => o.set_rect(left, top, right, bottom),
        }
    }

    pub fn clear(&mut self) {
        match self {
            Self::X11(o) => o.clear(),
            Self::Wayland(o) => o.clear(),
        }
    }
}

pub fn list_open_windows() -> Result<Vec<WindowInfo>, CaptureError> {
    if is_wayland_backend() {
        wayland_focus::list_open_windows()
    } else {
        x11_focus::list_open_windows()
    }
}

pub fn get_active_window() -> Result<Option<WindowInfo>, CaptureError> {
    if is_wayland_backend() {
        wayland_focus::get_active_window()
    } else {
        x11_focus::get_active_window()
    }
}

pub fn process_icon(process_path: &str, window_title: &str) -> Option<ProcessIcon> {
    if is_wayland_backend() {
        wayland_focus::process_icon(process_path, window_title)
    } else {
        x11_focus::process_icon(process_path, window_title)
    }
}

pub fn skip_taskbar_for_overlay_windows() -> Result<(), CaptureError> {
    if is_wayland_backend() {
        wayland_focus::skip_taskbar_for_overlay_windows()
    } else {
        x11_focus::skip_taskbar_for_overlay_windows()
    }
}

pub use wayland_focus::OVERLAY_WM_TITLE;

pub(crate) fn primary_monitor_scale() -> Option<f32> {
    if is_wayland_backend() {
        Some(1.0)
    } else {
        crate::x11_capture::primary_monitor_scale()
    }
}
