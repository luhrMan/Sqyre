//! Desktop-owned macro shortcuts (Wayland GlobalShortcuts portal).
//!
//! When active, the compositor delivers macro chords and lets the user rebind
//! them in its own settings; Sqyre's key matcher skips those macros.

/// Live state of the desktop shortcut backend.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SystemShortcutsStatus {
    /// No GlobalShortcuts portal on this platform / session.
    #[default]
    Unavailable,
    /// The desktop's shortcut dialog is open.
    Waiting,
    /// The desktop delivers `bound` macro hotkeys.
    Active { bound: usize },
    /// The user dismissed the dialog; Sqyre matches chords from input devices.
    Declined,
    /// The portal session failed; Sqyre matches chords from input devices.
    Failed(String),
}

#[cfg(all(feature = "portal-shortcuts", target_os = "linux"))]
pub use crate::linux_portal_shortcuts::{
    open_system_shortcuts, system_shortcuts_configurable, system_shortcuts_status,
};

#[cfg(not(all(feature = "portal-shortcuts", target_os = "linux")))]
pub fn system_shortcuts_status() -> SystemShortcutsStatus {
    SystemShortcutsStatus::Unavailable
}

/// Whether the desktop can reopen its shortcut editor for Sqyre.
#[cfg(not(all(feature = "portal-shortcuts", target_os = "linux")))]
pub fn system_shortcuts_configurable() -> bool {
    false
}

/// Show the desktop's shortcut editor (or re-ask after a dismissed dialog).
#[cfg(not(all(feature = "portal-shortcuts", target_os = "linux")))]
pub fn open_system_shortcuts() {}
