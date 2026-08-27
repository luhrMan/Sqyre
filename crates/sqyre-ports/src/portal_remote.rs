//! Wayland portal Remote Desktop (EIS) pointer/keyboard, implemented in capture.

use crate::AutomationError;

/// Injected into `sqyre-input` so it never calls capture internals directly.
pub trait PortalRemoteInput: Send + Sync {
    fn ready(&self) -> bool;
    /// Start or reuse the combined portal session until EIS is connected.
    fn ensure(&self) -> Result<(), AutomationError>;
    fn last_pos(&self) -> Option<(i32, i32)>;
    fn move_pointer(&self, x: i32, y: i32) -> Result<(), AutomationError>;
    fn click(&self, button: &str, down: bool) -> Result<(), AutomationError>;
    fn key(&self, evdev: u32, down: bool) -> Result<(), AutomationError>;
    fn scroll(&self, up: bool) -> Result<(), AutomationError>;
    fn note(&self, _msg: &str) {}
}
