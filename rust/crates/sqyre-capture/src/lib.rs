//! Screen capture in absolute virtual-desktop coordinates.

mod stub;
#[cfg(target_os = "linux")]
mod x11_capture;

pub use stub::{NullCapturer, SolidCapturer};

#[cfg(target_os = "linux")]
pub use x11_capture::X11Capturer;

#[cfg(not(target_os = "linux"))]
pub type X11Capturer = NullCapturer;
