//! Linux session detection and Wayland backend stubs.

pub mod capturer;
pub mod session;
pub mod wayland;

pub use capturer::{
    reset_shared_capturer, shared_capturer, shared_capturer_if_ready, shared_capturer_is_opening,
    shared_capturer_open_superseded, OsCapturer, SharedRunCapturer,
};
pub use session::{LinuxCaptureBackend, LinuxSessionInfo, LinuxSessionKind};
