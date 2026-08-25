//! [`sqyre_ports::PortalRemoteInput`] over the existing portal EIS helpers.

use super::{
    portal_input_click, portal_input_key, portal_input_last_pos, portal_input_move,
    portal_input_ready, portal_input_scroll, portal_remote_desktop_granted,
};
use sqyre_ports::{AutomationError, PortalRemoteInput};

/// Process-wide EIS backend used by `OsAutomation` on Wayland.
pub struct PortalEisInput;

impl PortalRemoteInput for PortalEisInput {
    fn ready(&self) -> bool {
        portal_input_ready()
    }

    fn ensure(&self) -> Result<(), AutomationError> {
        if self.ready() {
            return Ok(());
        }
        let _ = crate::shared_capturer();
        if self.ready() {
            Ok(())
        } else if portal_remote_desktop_granted() {
            Err(AutomationError::Backend(
                "Remote Desktop granted but EIS is not ready".into(),
            ))
        } else {
            Err(AutomationError::Backend(
                "desktop control not granted (enable Allow Remote Interaction, then Share)".into(),
            ))
        }
    }

    fn last_pos(&self) -> Option<(i32, i32)> {
        portal_input_last_pos()
    }

    fn move_pointer(&self, x: i32, y: i32) -> Result<(), AutomationError> {
        portal_input_move(x, y)
    }

    fn click(&self, button: &str, down: bool) -> Result<(), AutomationError> {
        portal_input_click(button, down)
    }

    fn key(&self, evdev: u32, down: bool) -> Result<(), AutomationError> {
        portal_input_key(evdev, down)
    }

    fn scroll(&self, up: bool) -> Result<(), AutomationError> {
        portal_input_scroll(up)
    }

    fn note(&self, msg: &str) {
        crate::note(msg);
    }
}
