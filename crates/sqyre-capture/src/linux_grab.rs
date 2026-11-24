//! Linux selection grab: layer-shell when available, else X11.

use crate::linux::session::LinuxSessionInfo;
use crate::linux::wayland::layer_shell_surface_works;
use crate::linux::wayland::wayland_grab::LayerSelectionGrab;
use crate::selection_grab::GrabPoll;
use crate::x11_grab::X11SelectionGrab;
use crate::CaptureError;

/// Fullscreen pointer-owning grab for screen-click recording.
pub enum SelectionGrab {
    X11(Box<X11SelectionGrab>),
    Layer(Box<LayerSelectionGrab>),
}

impl SelectionGrab {
    pub fn open() -> Result<Self, CaptureError> {
        let session = LinuxSessionInfo::detect();
        if session.has_wayland && layer_shell_surface_works().is_ok() {
            match LayerSelectionGrab::open() {
                Ok(layer) => return Ok(Self::Layer(Box::new(layer))),
                Err(e) => {
                    sqyre_ports::cap_log(
                        "GRAB",
                        "layer-fail",
                        &format!("falling back to X11: {e}"),
                    );
                }
            }
        }
        X11SelectionGrab::open().map(|g| Self::X11(Box::new(g)))
    }

    pub fn arm(&mut self) -> Result<(), CaptureError> {
        match self {
            Self::X11(g) => g.arm(),
            Self::Layer(g) => g.arm(),
        }
    }

    pub fn disarm(&mut self) {
        match self {
            Self::X11(g) => g.disarm(),
            Self::Layer(g) => g.disarm(),
        }
    }

    pub fn is_armed(&self) -> bool {
        match self {
            Self::X11(g) => g.is_armed(),
            Self::Layer(g) => g.is_armed(),
        }
    }

    pub fn poll(&mut self) -> GrabPoll {
        match self {
            Self::X11(g) => g.poll(),
            Self::Layer(g) => g.poll(),
        }
    }
}
