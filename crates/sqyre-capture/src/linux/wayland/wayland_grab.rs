//! Fullscreen layer-shell pointer/keyboard grab for screen-click recording.

use super::layer_surface::{attach_transparent, output_desktop_rect, GrabAccum, LayerShellConn};
use crate::selection_grab::GrabPoll;
use sqyre_ports::{cap_log, CaptureError};
use wayland_client::protocol::{wl_keyboard, wl_pointer, wl_surface};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1;

/// Nearly invisible fullscreen overlay that receives pointer + Esc while armed.
pub struct LayerSelectionGrab {
    conn: LayerShellConn,
    surface: Option<wl_surface::WlSurface>,
    layer: Option<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
    pointer: Option<wl_pointer::WlPointer>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    armed: bool,
    last_pos: (i32, i32),
}

// SAFETY: Wayland proxies are owned exclusively by this value.
unsafe impl Send for LayerSelectionGrab {}

impl LayerSelectionGrab {
    pub fn open() -> Result<Self, CaptureError> {
        let conn = LayerShellConn::connect()?;
        if conn.state.seat.is_none() {
            return Err(CaptureError::Message(
                "no wl_seat for layer-shell grab".into(),
            ));
        }
        cap_log("GRAB", "open", "backend=wayland-layer-shell");
        Ok(Self {
            conn,
            surface: None,
            layer: None,
            pointer: None,
            keyboard: None,
            armed: false,
            last_pos: (0, 0),
        })
    }

    pub fn arm(&mut self) -> Result<(), CaptureError> {
        if self.armed {
            return Ok(());
        }
        let outputs = self.conn.outputs();
        let Some(out) = outputs.into_iter().next() else {
            return Err(CaptureError::Message("no wl_output for grab".into()));
        };
        let cosmic = sqyre_ports::cosmic_desktop_session();
        let desktop = output_desktop_rect(out.x, out.y, out.width, out.height, out.scale, cosmic);
        let (surface, layer) = self.conn.create_overlay_surface(
            &out.output,
            "sqyre-grab",
            zwlr_layer_surface_v1::KeyboardInteractivity::Exclusive,
        )?;
        self.conn.set_input_full(&surface)?;
        layer.set_size(out.width.max(1) as u32, out.height.max(1) as u32);
        layer.set_anchor(
            zwlr_layer_surface_v1::Anchor::Top
                | zwlr_layer_surface_v1::Anchor::Bottom
                | zwlr_layer_surface_v1::Anchor::Left
                | zwlr_layer_surface_v1::Anchor::Right,
        );
        layer.set_margin(0, 0, 0, 0);
        self.conn.state.configure_serial = None;
        self.conn.state.layer_closed = false;
        surface.commit();
        if !self.conn.wait_configure(12) {
            layer.destroy();
            surface.destroy();
            return Err(CaptureError::Message(
                "grab layer configure timed out".into(),
            ));
        }
        let Some(serial) = self.conn.state.configure_serial.take() else {
            layer.destroy();
            surface.destroy();
            return Err(CaptureError::Message(
                "grab layer missing configure serial".into(),
            ));
        };
        layer.ack_configure(serial);
        let shm = self
            .conn
            .state
            .shm
            .clone()
            .ok_or_else(|| CaptureError::Message("shm gone".into()))?;
        let qh = self.conn.queue.handle();
        let attached = attach_transparent(
            &surface,
            &shm,
            &qh,
            out.width.max(2),
            out.height.max(2),
            out.scale.max(1),
            0,
        )
        .ok_or_else(|| CaptureError::Message("grab shm attach failed".into()))?;
        let _ = self.conn.roundtrip();
        attached.destroy();

        self.conn.state.grab_origin = (desktop.x, desktop.y);
        self.conn.state.grab_scale = out.scale.max(1);
        self.conn.state.grab = GrabAccum::default();

        let seat = self
            .conn
            .state
            .seat
            .clone()
            .ok_or_else(|| CaptureError::Message("seat gone".into()))?;
        let qh = self.conn.queue.handle();
        self.pointer = Some(seat.get_pointer(&qh, ()));
        self.keyboard = Some(seat.get_keyboard(&qh, ()));
        let _ = self.conn.roundtrip();

        self.surface = Some(surface);
        self.layer = Some(layer);
        self.armed = true;
        self.conn.flush();
        Ok(())
    }

    pub fn disarm(&mut self) {
        if !self.armed {
            return;
        }
        if let Some(p) = self.pointer.take() {
            p.release();
        }
        if let Some(k) = self.keyboard.take() {
            k.release();
        }
        if let Some(layer) = self.layer.take() {
            layer.destroy();
        }
        if let Some(surface) = self.surface.take() {
            surface.destroy();
        }
        self.armed = false;
        let _ = self.conn.roundtrip();
        self.conn.flush();
    }

    pub fn is_armed(&self) -> bool {
        self.armed
    }

    pub fn poll(&mut self) -> GrabPoll {
        let mut out = GrabPoll {
            x: self.last_pos.0,
            y: self.last_pos.1,
            ..GrabPoll::default()
        };
        if !self.armed {
            return out;
        }
        let _ = self.conn.conn.flush();
        if let Some(guard) = self.conn.conn.prepare_read() {
            let _ = guard.read();
        }
        let _ = self.conn.queue.dispatch_pending(&mut self.conn.state);

        let g = &mut self.conn.state.grab;
        out.moved |= g.moved;
        out.left_clicks = out.left_clicks.saturating_add(g.left_clicks);
        out.left_releases = out.left_releases.saturating_add(g.left_releases);
        out.escape |= g.escape;
        if g.moved || g.left_clicks > 0 || g.left_releases > 0 {
            self.last_pos = (g.x, g.y);
        }
        out.x = self.last_pos.0;
        out.y = self.last_pos.1;
        *g = GrabAccum {
            x: self.last_pos.0,
            y: self.last_pos.1,
            ..GrabAccum::default()
        };
        out
    }
}

impl Drop for LayerSelectionGrab {
    fn drop(&mut self) {
        self.disarm();
    }
}
