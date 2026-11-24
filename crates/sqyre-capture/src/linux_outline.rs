//! Linux selection outline: layer-shell when available, else X11.

use crate::linux::session::LinuxSessionInfo;
use crate::linux::wayland::layer_shell_surface_works;
use crate::linux::wayland::wayland_outline::LayerSelectionOutline;
use crate::x11_outline::X11SelectionOutline;
use crate::CaptureError;
use sqyre_ports::DesktopRect;

/// Hollow selection rectangle over the desktop (X11 OR windows or layer-shell edges).
pub enum SelectionOutline {
    X11(Box<X11SelectionOutline>),
    Layer(Box<LayerSelectionOutline>),
}

impl SelectionOutline {
    pub fn open() -> Result<Self, CaptureError> {
        let session = LinuxSessionInfo::detect();
        if session.has_wayland && layer_shell_surface_works().is_ok() {
            match LayerSelectionOutline::open() {
                Ok(layer) => return Ok(Self::Layer(Box::new(layer))),
                Err(e) => {
                    sqyre_ports::cap_log(
                        "OUTLINE",
                        "layer-fail",
                        &format!("falling back to X11: {e}"),
                    );
                }
            }
        }
        X11SelectionOutline::open().map(|o| Self::X11(Box::new(o)))
    }

    pub fn set_rect(&mut self, left: i32, top: i32, right: i32, bottom: i32) {
        match self {
            Self::X11(o) => o.set_rect(left, top, right, bottom),
            Self::Layer(o) => o.set_rect(left, top, right, bottom),
        }
    }

    pub fn input_passthrough(&self) -> bool {
        match self {
            Self::X11(o) => o.input_passthrough(),
            Self::Layer(o) => o.input_passthrough(),
        }
    }

    pub fn has_separate_pointer_conn(&self) -> bool {
        match self {
            Self::X11(o) => o.has_separate_pointer_conn(),
            Self::Layer(o) => o.has_separate_pointer_conn(),
        }
    }

    pub fn query_pointer(&self) -> Option<(i32, i32, bool)> {
        match self {
            Self::X11(o) => o.query_pointer(),
            Self::Layer(o) => o.query_pointer(),
        }
    }

    pub fn root_size(&self) -> Option<(i32, i32)> {
        match self {
            Self::X11(o) => o.root_size(),
            Self::Layer(o) => o.root_size(),
        }
    }

    pub fn virtual_rects(&self) -> Vec<DesktopRect> {
        match self {
            Self::X11(o) => o.virtual_rects(),
            Self::Layer(o) => o.virtual_rects(),
        }
    }

    pub fn clear(&mut self) {
        match self {
            Self::X11(o) => o.clear(),
            Self::Layer(o) => o.clear(),
        }
    }

    pub fn is_active(&self) -> bool {
        match self {
            Self::X11(o) => o.is_active(),
            Self::Layer(o) => o.is_active(),
        }
    }
}
