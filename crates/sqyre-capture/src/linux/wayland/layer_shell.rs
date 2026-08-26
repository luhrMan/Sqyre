//! wlr-layer-shell availability (GPU Screen Recorder-style native Wayland overlays).
//!
//! GSR uses `ZWLR_LAYER_SHELL_V1_LAYER_OVERLAY` with exclusive keyboard interactivity
//! on wlroots compositors so the recorder UI can be shown/hidden without unmapping
//! fullscreen games. Sqyre probes the global here; outline/grab surfaces will bind it.

use crate::CaptureError;
use wayland_client::protocol::wl_registry;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1;

#[derive(Default)]
struct ProbeState {
    found: bool,
}

/// True when the compositor advertises `zwlr_layer_shell_v1`.
pub fn layer_shell_available() -> Result<(), CaptureError> {
    let conn = Connection::connect_to_env()
        .map_err(|e| CaptureError::Message(format!("wayland connect: {e}")))?;
    let mut queue = conn.new_event_queue();
    let qh = queue.handle();
    let _registry = conn.display().get_registry(&qh, ());
    let mut state = ProbeState::default();
    queue
        .roundtrip(&mut state)
        .map_err(|e| CaptureError::Message(format!("layer-shell probe: {e}")))?;
    if state.found {
        Ok(())
    } else {
        Err(CaptureError::Message(
            "compositor does not advertise zwlr_layer_shell_v1".into(),
        ))
    }
}

/// GSR enables native layer-shell overlay UI on these sessions (env-only; no connect).
pub fn prefers_layer_shell_overlay_session() -> bool {
    let Some(desktop) = std::env::var("XDG_CURRENT_DESKTOP")
        .ok()
        .filter(|s| !s.is_empty())
    else {
        return false;
    };
    desktop.contains("Hyprland")
        || desktop.contains("niri")
        || desktop.contains("river")
        || desktop.contains("sway")
}

impl Dispatch<wl_registry::WlRegistry, ()> for ProbeState {
    fn event(
        state: &mut Self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global { interface, .. } = event {
            if interface == zwlr_layer_shell_v1::ZwlrLayerShellV1::interface().name {
                state.found = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_layer_shell_overlay_session_from_env() {
        let prev = std::env::var("XDG_CURRENT_DESKTOP").ok();
        std::env::set_var("XDG_CURRENT_DESKTOP", "Hyprland");
        assert!(prefers_layer_shell_overlay_session());
        std::env::set_var("XDG_CURRENT_DESKTOP", "GNOME");
        assert!(!prefers_layer_shell_overlay_session());
        match prev {
            Ok(v) => std::env::set_var("XDG_CURRENT_DESKTOP", v),
            Err(_) => std::env::remove_var("XDG_CURRENT_DESKTOP"),
        }
    }
}
