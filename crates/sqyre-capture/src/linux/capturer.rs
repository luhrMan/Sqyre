//! Linux capture backend dispatch (X11 / XWayland vs portal + PipeWire).

use crate::error::CaptureError;
use crate::linux::session::{LinuxCaptureBackend, LinuxSessionInfo};
use crate::x11_capture::X11Capturer;
use image::RgbaImage;
use sqyre_ports::{DesktopRect, RgbCapture};

#[cfg(feature = "portal-capture")]
use crate::linux::wayland::PortalCapturer;

/// OS-neutral Linux capturer — X11 when available, else portal ScreenCast + PipeWire.
pub enum OsCapturer {
    X11(X11Capturer),
    #[cfg(feature = "portal-capture")]
    Portal(PortalCapturer),
}

impl OsCapturer {
    /// Open the preferred capture backend for the current session.
    pub fn open() -> Result<Self, CaptureError> {
        let info = LinuxSessionInfo::detect();
        match info.capture_backend() {
            LinuxCaptureBackend::WaylandPortal => open_wayland_portal(),
            LinuxCaptureBackend::X11Native | LinuxCaptureBackend::XWayland => {
                X11Capturer::open().map(Self::X11)
            }
            LinuxCaptureBackend::Unavailable => Err(CaptureError::OpenDisplay),
        }
    }

    /// Absolute pointer position (X11 only today).
    pub fn pointer_position(&self) -> Result<(i32, i32), CaptureError> {
        match self {
            Self::X11(c) => c.pointer_position(),
            #[cfg(feature = "portal-capture")]
            Self::Portal(_) => Err(CaptureError::Message(
                "pointer position unavailable with portal capture".into(),
            )),
        }
    }

    pub fn capture_rect_ref(&self, rect: DesktopRect) -> Result<RgbaImage, CaptureError> {
        match self {
            Self::X11(c) => c.capture_rect_ref(rect),
            #[cfg(feature = "portal-capture")]
            Self::Portal(c) => c.capture_rect_ref(rect),
        }
    }

    /// Capture after waiting for a new portal frame when applicable (manual refresh).
    pub fn capture_rect_fresh_ref(&self, rect: DesktopRect) -> Result<RgbaImage, CaptureError> {
        match self {
            Self::X11(c) => c.capture_rect_ref(rect),
            #[cfg(feature = "portal-capture")]
            Self::Portal(c) => c.capture_rect_fresh_ref(rect),
        }
    }

    pub fn capture_rect_rgb_ref(&self, rect: DesktopRect) -> Result<RgbCapture, CaptureError> {
        match self {
            Self::X11(c) => c.capture_rect_rgb_ref(rect),
            #[cfg(feature = "portal-capture")]
            Self::Portal(c) => c.capture_rect_rgb_ref(rect),
        }
    }

    /// Wait for a newer portal frame (wait/repeat retries and manual refresh).
    pub fn capture_rect_rgb_fresh_ref(
        &self,
        rect: DesktopRect,
    ) -> Result<RgbCapture, CaptureError> {
        match self {
            Self::X11(c) => c.capture_rect_rgb_ref(rect),
            #[cfg(feature = "portal-capture")]
            Self::Portal(c) => c.capture_rect_rgb_fresh_ref(rect),
        }
    }

    pub fn virtual_bounds_ref(&self) -> Result<DesktopRect, CaptureError> {
        match self {
            Self::X11(c) => c.virtual_bounds_ref(),
            #[cfg(feature = "portal-capture")]
            Self::Portal(c) => c.virtual_bounds_ref(),
        }
    }

    pub fn monitor_rects_ref(&self) -> Result<Vec<DesktopRect>, CaptureError> {
        match self {
            Self::X11(c) => c.monitor_rects_ref(),
            #[cfg(feature = "portal-capture")]
            Self::Portal(c) => c.monitor_rects_ref(),
        }
    }

    pub fn monitor_sizes_ref(&self) -> Result<Vec<(i32, i32)>, CaptureError> {
        match self {
            Self::X11(c) => c.monitor_sizes_ref(),
            #[cfg(feature = "portal-capture")]
            Self::Portal(c) => c.monitor_sizes_ref(),
        }
    }
}

fn open_wayland_portal() -> Result<OsCapturer, CaptureError> {
    #[cfg(feature = "portal-capture")]
    {
        PortalCapturer::open().map(OsCapturer::Portal)
    }
    #[cfg(not(feature = "portal-capture"))]
    {
        Err(CaptureError::Message(
            "pure Wayland capture requires rebuilding Sqyre with the portal-capture feature \
             (needs libpipewire ≥ 1.0 development packages)"
                .into(),
        ))
    }
}

crate::define_shared_run_capturer!();
