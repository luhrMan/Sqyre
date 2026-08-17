//! Linux session detection and Wayland backend stubs.

pub mod capturer;
pub mod session;
pub mod wayland;

pub use capturer::{shared_capturer, OsCapturer, SharedRunCapturer};
pub use session::{LinuxCaptureBackend, LinuxSessionInfo, LinuxSessionKind};
