//! Selection outline via four wlr-layer-shell edge surfaces.

use super::layer_surface::{attach_argb_pixels, output_desktop_rect, LayerShellConn, OutputGeom};
use crate::outline_geometry::{
    edge_placements, outline_should_clear, EDGE_PX, STROKE_B, STROKE_G, STROKE_R,
};
use crate::OutlineRect;
use sqyre_ports::{cap_log, event_log, CaptureError, DesktopRect};
use std::time::Instant;
use wayland_client::protocol::wl_surface;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1;

struct EdgeSurface {
    surface: wl_surface::WlSurface,
    layer: zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
    output_name: u32,
    last_place: Option<(i32, i32, i32, i32)>,
}

/// Hollow gold rectangle drawn with four click-through overlay layer surfaces.
pub struct LayerSelectionOutline {
    conn: LayerShellConn,
    edges: [EdgeSurface; 4],
    last: Option<OutlineRect>,
    cached_rects: Vec<DesktopRect>,
}

// SAFETY: `conn` and edge proxies are owned exclusively by this value; only
// `&mut self` methods and `Drop` touch them.
unsafe impl Send for LayerSelectionOutline {}

impl LayerSelectionOutline {
    pub fn open() -> Result<Self, CaptureError> {
        let mut conn = LayerShellConn::connect()?;
        let outputs = conn.outputs();
        let Some(primary) = outputs.first().cloned() else {
            return Err(CaptureError::Message("no wl_output for outline".into()));
        };
        let cosmic = sqyre_ports::cosmic_desktop_session();
        let cached_rects: Vec<DesktopRect> = outputs
            .iter()
            .map(|o| output_desktop_rect(o.x, o.y, o.width, o.height, o.scale, cosmic))
            .collect();

        let mut edges = Vec::with_capacity(4);
        for i in 0..4 {
            let (surface, layer) = conn.create_overlay_surface(
                &primary.output,
                &format!("sqyre-outline-{i}"),
                zwlr_layer_surface_v1::KeyboardInteractivity::None,
            )?;
            conn.set_input_empty(&surface)?;
            let park = DesktopRect {
                x: primary.x,
                y: primary.y,
                w: 1,
                h: 1,
            };
            let (lw, lh) = LayerShellConn::position_layer(&layer, park, &primary);
            conn.state.configure_serial = None;
            conn.state.layer_closed = false;
            surface.commit();
            if !conn.wait_configure(8) {
                layer.destroy();
                surface.destroy();
                return Err(CaptureError::Message(
                    "outline layer configure timed out".into(),
                ));
            }
            let Some(serial) = conn.state.configure_serial.take() else {
                layer.destroy();
                surface.destroy();
                return Err(CaptureError::Message(
                    "outline layer missing configure serial".into(),
                ));
            };
            layer.ack_configure(serial);
            paint_edge(&mut conn, &surface, &primary, lw, lh)?;
            edges.push(EdgeSurface {
                surface,
                layer,
                output_name: primary.name,
                last_place: Some((primary.x, primary.y, 1, 1)),
            });
        }
        let edges: [EdgeSurface; 4] = edges
            .try_into()
            .map_err(|_| CaptureError::Message("outline edge count".into()))?;
        conn.flush();
        event_log(
            "SQYRE_OUTLINE",
            &[
                ("input", "passthrough"),
                ("backend", "wayland-layer-shell"),
                ("outputs", &cached_rects.len().to_string()),
            ],
        );
        Ok(Self {
            conn,
            edges,
            last: None,
            cached_rects,
        })
    }

    pub fn set_rect(&mut self, left: i32, top: i32, right: i32, bottom: i32) {
        let rect = OutlineRect::normalize(left, top, right, bottom);
        if outline_should_clear(rect) {
            self.clear();
            return;
        }
        if self.last == Some(rect) {
            return;
        }
        let placements = edge_placements(rect);
        let coming_from_clear = self.last.is_none();
        if coming_from_clear {
            event_log(
                "SQYRE_OUTLINE",
                &[
                    ("op", "show"),
                    (
                        "rect",
                        &format!("{},{}-{},{}", rect.left, rect.top, rect.right, rect.bottom),
                    ),
                    ("backend", "wayland-layer-shell"),
                ],
            );
        }
        let start = Instant::now();
        for (edge, place) in self.edges.iter_mut().zip(placements.iter()) {
            if edge.last_place == Some(*place) {
                continue;
            }
            if let Err(e) = place_edge(&mut self.conn, edge, *place, &self.cached_rects) {
                cap_log("OUTLINE", "layer", &format!("place_edge: {e}"));
            }
        }
        let _ = self.conn.roundtrip();
        self.conn.flush();
        let ms = start.elapsed().as_millis();
        if ms >= 16 {
            cap_log("OUTLINE", "slow", &format!("set_rect_ms={ms}"));
        }
        self.last = Some(rect);
    }

    pub fn input_passthrough(&self) -> bool {
        true
    }

    pub fn has_separate_pointer_conn(&self) -> bool {
        false
    }

    pub fn query_pointer(&self) -> Option<(i32, i32, bool)> {
        crate::portal_cursor_position().map(|(x, y)| (x, y, true))
    }

    pub fn root_size(&self) -> Option<(i32, i32)> {
        let mut max_r = 0;
        let mut max_b = 0;
        for r in &self.cached_rects {
            max_r = max_r.max(r.x + r.w);
            max_b = max_b.max(r.y + r.h);
        }
        if max_r > 0 && max_b > 0 {
            Some((max_r, max_b))
        } else {
            None
        }
    }

    pub fn virtual_rects(&self) -> Vec<DesktopRect> {
        self.cached_rects.clone()
    }

    pub fn clear(&mut self) {
        if self.last.is_none() {
            return;
        }
        let Some(primary) = self.conn.outputs().into_iter().next() else {
            self.last = None;
            return;
        };
        for edge in &mut self.edges {
            let place = (primary.x, primary.y, 1, 1);
            if edge.last_place == Some(place) {
                continue;
            }
            let _ = place_edge(
                &mut self.conn,
                edge,
                place,
                &[(DesktopRect {
                    x: primary.x,
                    y: primary.y,
                    w: primary.width,
                    h: primary.height,
                })],
            );
        }
        let _ = self.conn.roundtrip();
        self.conn.flush();
        self.last = None;
    }

    pub fn is_active(&self) -> bool {
        self.last.is_some()
    }
}

impl Drop for LayerSelectionOutline {
    fn drop(&mut self) {
        for edge in &self.edges {
            edge.layer.destroy();
            edge.surface.destroy();
        }
        self.conn.flush();
    }
}

fn place_edge(
    conn: &mut LayerShellConn,
    edge: &mut EdgeSurface,
    place: (i32, i32, i32, i32),
    outputs: &[DesktopRect],
) -> Result<(), CaptureError> {
    let (x, y, w, h) = place;
    let rect = DesktopRect { x, y, w, h };
    let geom = pick_output(conn, rect, outputs)?;
    if edge.output_name != geom.name {
        // Recreate on the correct output — destroy and make a new layer surface.
        edge.layer.destroy();
        edge.surface.destroy();
        let (surface, layer) = conn.create_overlay_surface(
            &geom.output,
            "sqyre-outline",
            zwlr_layer_surface_v1::KeyboardInteractivity::None,
        )?;
        conn.set_input_empty(&surface)?;
        edge.surface = surface;
        edge.layer = layer;
        edge.output_name = geom.name;
    }
    let (lw, lh) = LayerShellConn::position_layer(&edge.layer, rect, &geom);
    conn.state.configure_serial = None;
    conn.state.layer_closed = false;
    edge.surface.commit();
    if !conn.wait_configure(8) {
        return Err(CaptureError::Message(
            "outline edge configure timed out".into(),
        ));
    }
    let Some(serial) = conn.state.configure_serial.take() else {
        return Err(CaptureError::Message(
            "outline edge missing configure serial".into(),
        ));
    };
    edge.layer.ack_configure(serial);
    paint_edge(conn, &edge.surface, &geom, lw, lh)?;
    edge.last_place = Some(place);
    Ok(())
}

fn pick_output(
    conn: &LayerShellConn,
    rect: DesktopRect,
    _cached: &[DesktopRect],
) -> Result<OutputGeom, CaptureError> {
    conn.overlapping_output(rect)
        .or_else(|| conn.outputs().into_iter().next())
        .ok_or_else(|| CaptureError::Message("no output for outline edge".into()))
}

fn paint_edge(
    conn: &mut LayerShellConn,
    surface: &wl_surface::WlSurface,
    geom: &OutputGeom,
    logical_w: u32,
    logical_h: u32,
) -> Result<(), CaptureError> {
    let scale = geom.scale.max(1);
    let buf_w = (logical_w as i32).saturating_mul(scale).max(1);
    let buf_h = (logical_h as i32).saturating_mul(scale).max(1);
    let pixels = solid_argb(buf_w as usize, buf_h as usize);
    let shm = conn
        .state
        .shm
        .clone()
        .ok_or_else(|| CaptureError::Message("shm gone".into()))?;
    let qh = conn.queue.handle();
    let Some(attached) = attach_argb_pixels(surface, &shm, &qh, buf_w, buf_h, scale, &pixels)
    else {
        return Err(CaptureError::Message("outline shm attach failed".into()));
    };
    // Buffer stays attached until the next attach; destroy pool after commit ack.
    let _ = conn.roundtrip();
    attached.destroy();
    let _ = EDGE_PX; // thickness comes from edge_placements
    Ok(())
}

fn solid_argb(w: usize, h: usize) -> Vec<u8> {
    let mut out = vec![0u8; w.saturating_mul(h).saturating_mul(4)];
    for px in out.as_chunks_mut::<4>().0 {
        // wl_shm Argb8888 LE: B, G, R, A
        px[0] = STROKE_B;
        px[1] = STROKE_G;
        px[2] = STROKE_R;
        px[3] = 255;
    }
    out
}
