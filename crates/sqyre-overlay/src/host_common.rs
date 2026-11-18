//! OS-neutral overlay host DTOs shared by X11 and Windows button hosts.

use crate::ButtonFace;

/// Desired on-screen button (physical pixels, host desktop coordinates).
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayButtonSpec {
    pub id: String,
    pub macro_name: String,
    pub face: ButtonFace,
    /// Hover tip text (label / display name). Empty = no tip.
    pub tip: String,
    /// egui `pixels_per_point` — tip font/padding track Settings → UI scale.
    pub ui_scale: f32,
    pub busy: bool,
    /// When true, the host maps this button only while visibility says found.
    pub visibility_gated: bool,
    /// When false, hit-tests pass through and presses are ignored (hub fan travel).
    pub input_enabled: bool,
}

/// Desktop position committed after a relocate-mode drag (root / virtual-desktop coords).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayButtonMove {
    pub id: String,
    pub x: i32,
    pub y: i32,
}
