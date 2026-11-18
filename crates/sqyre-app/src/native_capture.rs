//! Native shell wiring for capture: shared capturer + monitor layout.
//!
//! Leaf UI modules take [`ScreenCapturer`] / [`MonitorLayout`] and must not
//! import `sqyre_capture` free functions directly. This module is the allowed
//! app→capture edge for opening the shared session and reading layout.

use sqyre_capture::{OsMonitorLayout, SharedRunCapturer};
use sqyre_ports::{CaptureError, MonitorLayout, NotReady};

/// Open (or reuse) the process-wide capturer as a [`ScreenCapturer`] wrapper.
pub fn open_shared_run() -> Result<SharedRunCapturer, CaptureError> {
    Ok(SharedRunCapturer(sqyre_capture::shared_capturer()?))
}

/// Non-blocking peek: `Ok(None)` while the portal open is still in flight.
pub fn try_open_shared_run() -> Result<Option<SharedRunCapturer>, CaptureError> {
    match sqyre_capture::shared_capturer_nonblocking() {
        Ok(c) => Ok(Some(SharedRunCapturer(c))),
        Err(e)
            if matches!(
                e,
                CaptureError::NotReady(NotReady::AwaitingPortalPermission)
                    | CaptureError::NotReady(NotReady::AwaitingFirstFrame)
            ) =>
        {
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

/// Live monitor layout for catalog / overlay geometry.
pub fn monitor_layout() -> OsMonitorLayout {
    OsMonitorLayout
}

/// Preferred monitor rects via [`MonitorLayout`].
pub fn preferred_rects() -> Vec<sqyre_ports::DesktopRect> {
    monitor_layout().preferred_rects()
}

/// Display count via [`MonitorLayout`].
pub fn monitor_count() -> usize {
    monitor_layout().count()
}
