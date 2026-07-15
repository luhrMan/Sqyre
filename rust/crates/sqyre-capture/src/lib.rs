//! Screen capture in absolute virtual-desktop coordinates.

mod stub;
#[cfg(target_os = "linux")]
mod x11_capture;
#[cfg(target_os = "linux")]
mod x11_focus;

pub use stub::{NullCapturer, SolidCapturer};

#[cfg(target_os = "linux")]
pub use x11_capture::X11Capturer;

#[cfg(target_os = "linux")]
pub use x11_focus::X11WindowFocuser;

#[cfg(not(target_os = "linux"))]
pub type X11Capturer = NullCapturer;

/// No-op focuser for non-Linux (or tests without a display).
#[cfg(not(target_os = "linux"))]
#[derive(Debug, Default, Clone, Copy)]
pub struct X11WindowFocuser;

#[cfg(not(target_os = "linux"))]
impl sqyre_executor::WindowFocuser for X11WindowFocuser {
    fn focus(&self, _process_path: &str, _window_title: &str) -> Result<(), String> {
        Err("focus window: not supported on this platform".into())
    }
}
