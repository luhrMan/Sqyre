//! Native Wayland layer-shell overlay buttons on a dedicated event thread.

use crate::host_common::{OverlayButtonMove, OverlayButtonSpec};
use crate::hub_fan::HubFanState;
use crate::linux_host::{HotkeyChooserResult, HotkeyChooserSpec};
use crate::raster::{self, ButtonPaint};
use egui::Context as EguiContext;
use parking_lot::Mutex;
use sqyre_capture::{
    attach_argb_pixels, output_desktop_rect, set_overlay_omit_hook, LayerShellConn, OutputGeom,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use thiserror::Error;
use wayland_client::protocol::{wl_pointer, wl_surface};
use wayland_client::Proxy;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1;

mod host_loop;

enum HostCmd {
    SetButtons(Vec<OverlayButtonSpec>),
    SetRelocateMode(bool),
    SetRelocateKeepAbove(bool),
    SetVisibilityMap(Arc<Mutex<HashMap<String, bool>>>),
    SetWake(EguiContext),
    SetHubFanDismiss(bool),
    SetHubFan { expanded: bool, snap: bool },
    ShowChooser(HotkeyChooserSpec),
    HideChooser,
    SetCaptureOmit(bool, Sender<()>),
    Shutdown,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum OverlayError {
    #[error("spawn wayland overlay thread: {0}")]
    ThreadSpawn(String),
    #[error("wayland overlay: {0}")]
    Wayland(String),
}

/// Owns the layer-shell overlay thread.
pub struct LayerButtonHost {
    cmd_tx: Sender<HostCmd>,
    stop: Arc<AtomicBool>,
    join: Mutex<Option<JoinHandle<()>>>,
}

impl LayerButtonHost {
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
    ) -> Result<Self, OverlayError> {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_t = Arc::clone(&stop);
        let join = thread::Builder::new()
            .name("sqyre-wl-overlay".into())
            .spawn(move || {
                host_loop::host_loop(
                    cmd_rx,
                    pending,
                    pending_moves,
                    running_macro,
                    pending_chooser,
                    pending_context,
                    visibility,
                    stop_t,
                    run_stop,
                    press_stops,
                );
            })
            .map_err(|e| OverlayError::ThreadSpawn(e.to_string()))?;
        let hook_tx = cmd_tx.clone();
        set_overlay_omit_hook(Some(Arc::new(move |omit| {
            let (ack_tx, ack_rx) = mpsc::channel();
            if hook_tx.send(HostCmd::SetCaptureOmit(omit, ack_tx)).is_err() {
                return;
            }
            let _ = ack_rx.recv_timeout(Duration::from_millis(500));
        })));
        Ok(Self {
            cmd_tx,
            stop,
            join: Mutex::new(Some(join)),
        })
    }

    pub fn set_buttons(&self, buttons: Vec<OverlayButtonSpec>) {
        let _ = self.cmd_tx.send(HostCmd::SetButtons(buttons));
    }
    pub fn set_relocate_mode(&self, enabled: bool) {
        let _ = self.cmd_tx.send(HostCmd::SetRelocateMode(enabled));
    }
    pub fn set_relocate_keep_above(&self, enabled: bool) {
        let _ = self.cmd_tx.send(HostCmd::SetRelocateKeepAbove(enabled));
    }
    pub fn set_visibility_map(&self, map: Arc<Mutex<HashMap<String, bool>>>) {
        let _ = self.cmd_tx.send(HostCmd::SetVisibilityMap(map));
    }
    pub fn set_wake(&self, ctx: EguiContext) {
        let _ = self.cmd_tx.send(HostCmd::SetWake(ctx));
    }
    pub fn set_hub_fan_dismiss(&self, enabled: bool) {
        let _ = self.cmd_tx.send(HostCmd::SetHubFanDismiss(enabled));
    }
    pub fn set_hub_fan(&self, expanded: bool, snap: bool) {
        let _ = self.cmd_tx.send(HostCmd::SetHubFan { expanded, snap });
    }
    pub fn show_chooser(&self, spec: HotkeyChooserSpec) -> bool {
        self.cmd_tx.send(HostCmd::ShowChooser(spec)).is_ok()
    }
    pub fn hide_chooser(&self) {
        let _ = self.cmd_tx.send(HostCmd::HideChooser);
    }
    pub fn shutdown(&self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.cmd_tx.send(HostCmd::Shutdown);
        set_overlay_omit_hook(None);
        if let Some(join) = self.join.lock().take() {
            let _ = join.join();
        }
    }
}

impl Drop for LayerButtonHost {
    fn drop(&mut self) {
        self.shutdown();
    }
}

struct LiveButton {
    id: String,
    spec: OverlayButtonSpec,
    surface: wl_surface::WlSurface,
    layer: zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
    mapped: bool,
    hovered: bool,
}

struct TipSurface {
    for_id: String,
    surface: wl_surface::WlSurface,
    layer: zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
}

struct ChooserSurface {
    spec: HotkeyChooserSpec,
    surface: wl_surface::WlSurface,
    layer: zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
    row_h: i32,
    title_h: i32,
    w: i32,
    h: i32,
}

struct HostState {
    conn: LayerShellConn,
    pointer: Option<wl_pointer::WlPointer>,
    buttons: HashMap<String, LiveButton>,
    /// wl_surface id → button id
    surface_to_button: HashMap<u32, String>,
    tip: Option<TipSurface>,
    chooser: Option<ChooserSurface>,
    visibility: Arc<Mutex<HashMap<String, bool>>>,
    pending: Arc<Mutex<Vec<String>>>,
    pending_moves: Arc<Mutex<Vec<OverlayButtonMove>>>,
    running_macro: Arc<Mutex<Option<String>>>,
    pending_chooser: Arc<Mutex<Option<HotkeyChooserResult>>>,
    #[expect(
        dead_code,
        reason = "context menu enqueue wired with right-click seat events"
    )]
    pending_context: Arc<Mutex<Vec<crate::OverlayButtonContextAction>>>,
    run_stop: Arc<Mutex<sqyre_hotkeys::StopFlag>>,
    press_stops: Arc<AtomicBool>,
    wake: Option<EguiContext>,
    relocate_mode: bool,
    relocate_keep_above: bool,
    hub_fan: HubFanState,
    hub_fan_dismiss: bool,
    capture_omit: bool,
    /// Button id armed for left release fire.
    armed_id: Option<String>,
    drag_id: Option<String>,
    drag_off: (i32, i32),
    last_ptr: (i32, i32),
    outputs: Vec<OutputGeom>,
}

fn rgba_to_wayland_argb(rgba: &[u8]) -> Vec<u8> {
    let mut out = rgba.to_vec();
    for px in out.as_chunks_mut::<4>().0 {
        px.swap(0, 2);
    }
    out
}

fn paint_button_pixels(spec: &OverlayButtonSpec, hovered: bool, busy: bool) -> (u32, u32, Vec<u8>) {
    let paint = ButtonPaint::from_face(&spec.face, hovered, busy, 0.0);
    let w = paint.w;
    let h = paint.h;
    let rgba = raster::rasterize_argb(&paint);
    (w, h, rgba_to_wayland_argb(&rgba))
}

fn attach_face(
    conn: &mut LayerShellConn,
    surface: &wl_surface::WlSurface,
    geom: &OutputGeom,
    logical_w: u32,
    logical_h: u32,
    pixels_argb: &[u8],
) -> Result<(), String> {
    let scale = geom.scale.max(1);
    let buf_w = (logical_w as i32).saturating_mul(scale).max(1);
    let buf_h = (logical_h as i32).saturating_mul(scale).max(1);
    // Scale nearest-neighbor if needed.
    let scaled = if scale == 1 {
        pixels_argb.to_vec()
    } else {
        scale_argb_nn(pixels_argb, logical_w, logical_h, scale as u32)
    };
    let shm = conn
        .state
        .shm
        .clone()
        .ok_or_else(|| "shm gone".to_string())?;
    let qh = conn.queue.handle();
    let attached = attach_argb_pixels(surface, &shm, &qh, buf_w, buf_h, scale, &scaled)
        .ok_or_else(|| "attach failed".to_string())?;
    let _ = conn.roundtrip();
    attached.destroy();
    Ok(())
}

fn scale_argb_nn(src: &[u8], w: u32, h: u32, scale: u32) -> Vec<u8> {
    let bw = w * scale;
    let bh = h * scale;
    let mut out = vec![0u8; (bw * bh * 4) as usize];
    for y in 0..bh {
        for x in 0..bw {
            let sx = x / scale;
            let sy = y / scale;
            let si = ((sy * w + sx) * 4) as usize;
            let di = ((y * bw + x) * 4) as usize;
            out[di..di + 4].copy_from_slice(&src[si..si + 4]);
        }
    }
    out
}
