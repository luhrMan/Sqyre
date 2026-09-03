//! Shared types for the fullscreen mouse-owning selection grab.

/// Input drained from [`crate::SelectionGrab::poll`] since the previous call.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct GrabPoll {
    /// Absolute virtual-desktop pointer position.
    pub x: i32,
    pub y: i32,
    /// True when the pointer moved (or was sampled) since the last poll.
    pub moved: bool,
    /// Number of left-button presses observed.
    pub left_clicks: u32,
    /// Number of left-button releases observed (search-area drag-to-select).
    pub left_releases: u32,
    /// True when Escape was pressed.
    pub escape: bool,
}
