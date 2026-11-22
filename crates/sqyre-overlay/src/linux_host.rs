//! Linux overlay button host facade (X11 today; layer-shell when available).

use crate::host_common::{OverlayButtonMove, OverlayButtonSpec};
use crate::x11_buttons::X11ButtonHost;
use egui::Context as EguiContext;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use thiserror::Error;

/// Result of a native hotkey conflict menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyChooserResult {
    Picked(String),
    Dismissed,
}

/// Spec for a near-cursor macro picker (physical root / desktop coordinates).
#[derive(Debug, Clone)]
pub struct HotkeyChooserSpec {
    pub title: String,
    pub names: Vec<String>,
    pub x: i32,
    pub y: i32,
}

/// Failure starting a Linux overlay button host.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LinuxHostError {
    #[error("{0}")]
    Message(String),
}

/// OS-neutral Linux button host: X11 override-redirect or Wayland layer-shell.
pub enum LinuxButtonHost {
    X11(X11ButtonHost),
}

impl LinuxButtonHost {
    /// Prefer layer-shell when the compositor advertises it; otherwise X11.
    #[expect(
        clippy::too_many_arguments,
        reason = "host owns macro, move, chooser, and context queues plus stop flags"
    )]
    pub fn start(
        pending: Arc<Mutex<Vec<String>>>,
        pending_moves: Arc<Mutex<Vec<OverlayButtonMove>>>,
        running_macro: Arc<Mutex<Option<String>>>,
        pending_chooser: Arc<Mutex<Option<HotkeyChooserResult>>>,
        pending_context: Arc<Mutex<Vec<crate::OverlayButtonContextAction>>>,
        visibility: Arc<Mutex<HashMap<String, bool>>>,
        run_stop: Arc<Mutex<sqyre_hotkeys::StopFlag>>,
        press_stops: Arc<AtomicBool>,
    ) -> Result<Self, LinuxHostError> {
        // Layer-shell backend lands in a later unit; X11 remains the only path here.
        X11ButtonHost::start(
            pending,
            pending_moves,
            running_macro,
            pending_chooser,
            pending_context,
            visibility,
            run_stop,
            press_stops,
        )
        .map(Self::X11)
        .map_err(|e| LinuxHostError::Message(e.to_string()))
    }

    pub fn set_buttons(&self, buttons: Vec<OverlayButtonSpec>) {
        match self {
            Self::X11(h) => h.set_buttons(buttons),
        }
    }

    pub fn set_relocate_mode(&self, enabled: bool) {
        match self {
            Self::X11(h) => h.set_relocate_mode(enabled),
        }
    }

    pub fn set_relocate_keep_above(&self, enabled: bool) {
        match self {
            Self::X11(h) => h.set_relocate_keep_above(enabled),
        }
    }

    pub fn set_visibility_map(&self, map: Arc<Mutex<HashMap<String, bool>>>) {
        match self {
            Self::X11(h) => h.set_visibility_map(map),
        }
    }

    pub fn set_wake(&self, ctx: EguiContext) {
        match self {
            Self::X11(h) => h.set_wake(ctx),
        }
    }

    pub fn set_hub_fan_dismiss(&self, enabled: bool) {
        match self {
            Self::X11(h) => h.set_hub_fan_dismiss(enabled),
        }
    }

    pub fn set_hub_fan(&self, expanded: bool, snap: bool) {
        match self {
            Self::X11(h) => h.set_hub_fan(expanded, snap),
        }
    }

    pub fn show_chooser(&self, spec: HotkeyChooserSpec) -> bool {
        match self {
            Self::X11(h) => h.show_chooser(spec),
        }
    }

    pub fn hide_chooser(&self) {
        match self {
            Self::X11(h) => h.hide_chooser(),
        }
    }

    pub fn shutdown(&self) {
        match self {
            Self::X11(h) => h.shutdown(),
        }
    }
}
