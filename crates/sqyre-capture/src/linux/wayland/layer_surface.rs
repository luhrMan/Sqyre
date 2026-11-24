//! Shared wlr-layer-shell + wl_shm helpers for kick, outline, grab, and overlays.

use sqyre_ports::{CaptureError, DesktopRect};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::os::fd::{AsFd, AsRawFd, FromRawFd, IntoRawFd, OwnedFd};
use std::time::Duration;
use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_keyboard, wl_output, wl_pointer, wl_region, wl_registry, wl_seat,
    wl_shm, wl_shm_pool, wl_surface,
};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

/// Output rectangle in Sqyre desktop pixels.
///
/// COSMIC's portal places a stream at the logical `wl_output` position and
/// fills a PipeWire frame at the physical mode size. Sqyre's desktop is that
/// physical pixel space, so the output origin is `logical * scale`. Other
/// compositors already use one space for both.
pub fn output_desktop_rect(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    scale: i32,
    cosmic: bool,
) -> DesktopRect {
    let scale = scale.max(1);
    if cosmic && scale > 1 {
        DesktopRect {
            x: x.saturating_mul(scale),
            y: y.saturating_mul(scale),
            w,
            h,
        }
    } else {
        DesktopRect { x, y, w, h }
    }
}

/// Layer-shell size and margins are logical pixels. `rect` is in desktop space.
/// Returns `(margin_left, margin_top, width, height)`.
pub fn layer_shell_logical(
    rect: DesktopRect,
    desktop: DesktopRect,
    scale: i32,
    cosmic: bool,
) -> (i32, i32, u32, u32) {
    let scale = scale.max(1);
    if cosmic && scale > 1 {
        let left = (rect.x - desktop.x).max(0) / scale;
        let top = (rect.y - desktop.y).max(0) / scale;
        let w = (rect.w / scale).max(1) as u32;
        let h = (rect.h / scale).max(1) as u32;
        (left, top, w, h)
    } else {
        (
            (rect.x - desktop.x).max(0),
            (rect.y - desktop.y).max(0),
            rect.w.max(1) as u32,
            rect.h.max(1) as u32,
        )
    }
}

pub fn rects_overlap(a: DesktopRect, b: DesktopRect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

/// Anonymous memfd sized for an ARGB8888 shm pool.
pub fn shm_memfd(bytes: usize, name: &std::ffi::CStr) -> Option<File> {
    if bytes == 0 {
        return None;
    }
    // SAFETY: `name` is NUL-terminated; flags are valid `MFD_*` constants.
    let fd =
        unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING) };
    if fd < 0 {
        return None;
    }
    // SAFETY: fresh open fd from memfd_create; OwnedFd becomes sole owner.
    let owned = unsafe { OwnedFd::from_raw_fd(fd) };
    // SAFETY: `owned` keeps the fd open for ftruncate.
    if unsafe { libc::ftruncate(owned.as_raw_fd(), bytes as libc::off_t) } != 0 {
        return None;
    }
    let raw = owned.into_raw_fd();
    // SAFETY: into_raw_fd released ownership without closing.
    Some(unsafe { File::from_raw_fd(raw) })
}

/// Zero-filled transparent ARGB memfd.
pub fn transparent_shm_file(bytes: usize) -> Option<File> {
    shm_memfd(bytes, c"sqyre-layer")
}

/// Write ARGB8888 little-endian pixels into a new memfd (`pixels` length = w*h*4).
pub fn argb_shm_file(width: i32, height: i32, pixels: &[u8]) -> Option<File> {
    let width = width.max(1);
    let height = height.max(1);
    let bytes = (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(4);
    if pixels.len() < bytes {
        return None;
    }
    let mut file = shm_memfd(bytes, c"sqyre-layer-argb")?;
    if file.write_all(&pixels[..bytes]).is_err() {
        return None;
    }
    let _ = file.flush();
    Some(file)
}

/// Attached shm buffer + pool + backing file (caller destroys on drop path).
pub struct AttachedShm {
    pub buffer: wl_buffer::WlBuffer,
    pub pool: wl_shm_pool::WlShmPool,
    pub file: File,
    pub buf_w: i32,
    pub buf_h: i32,
}

/// Attach a transparent ARGB buffer (optional single-pixel flip for damage kick).
pub fn attach_transparent<D>(
    surface: &wl_surface::WlSurface,
    shm: &wl_shm::WlShm,
    qh: &QueueHandle<D>,
    logical_w: i32,
    logical_h: i32,
    scale: i32,
    flip: u8,
) -> Option<AttachedShm>
where
    D: Dispatch<wl_shm_pool::WlShmPool, ()> + Dispatch<wl_buffer::WlBuffer, ()> + 'static,
{
    let logical_w = logical_w.max(2);
    let logical_h = logical_h.max(2);
    let scale = scale.max(1);
    let buf_w = logical_w.saturating_mul(scale);
    let buf_h = logical_h.saturating_mul(scale);
    let stride = buf_w.saturating_mul(4);
    let bytes = stride.saturating_mul(buf_h) as usize;
    let mut file = transparent_shm_file(bytes)?;
    // ARGB8888 LE: A in high byte. Keep A=0 so the pixel stays transparent.
    let px = [flip, 0u8, 0u8, 0u8];
    use std::io::{Seek, SeekFrom};
    let _ = file.seek(SeekFrom::Start(0));
    let _ = file.write_all(&px);
    let _ = file.flush();
    let pool = shm.create_pool(file.as_fd(), bytes as i32, qh, ());
    let buffer = pool.create_buffer(0, buf_w, buf_h, stride, wl_shm::Format::Argb8888, qh, ());
    surface.set_buffer_scale(scale);
    surface.attach(Some(&buffer), 0, 0);
    surface.damage_buffer(0, 0, buf_w, buf_h);
    surface.commit();
    Some(AttachedShm {
        buffer,
        pool,
        file,
        buf_w,
        buf_h,
    })
}

/// Attach caller-provided ARGB8888 pixels (length must be `w*h*4` in buffer pixels).
pub fn attach_argb_pixels<D>(
    surface: &wl_surface::WlSurface,
    shm: &wl_shm::WlShm,
    qh: &QueueHandle<D>,
    buf_w: i32,
    buf_h: i32,
    scale: i32,
    pixels: &[u8],
) -> Option<AttachedShm>
where
    D: Dispatch<wl_shm_pool::WlShmPool, ()> + Dispatch<wl_buffer::WlBuffer, ()> + 'static,
{
    let buf_w = buf_w.max(1);
    let buf_h = buf_h.max(1);
    let scale = scale.max(1);
    let file = argb_shm_file(buf_w, buf_h, pixels)?;
    let stride = buf_w.saturating_mul(4);
    let bytes = stride.saturating_mul(buf_h);
    let pool = shm.create_pool(file.as_fd(), bytes, qh, ());
    let buffer = pool.create_buffer(0, buf_w, buf_h, stride, wl_shm::Format::Argb8888, qh, ());
    surface.set_buffer_scale(scale);
    surface.attach(Some(&buffer), 0, 0);
    surface.damage_buffer(0, 0, buf_w, buf_h);
    surface.commit();
    Some(AttachedShm {
        buffer,
        pool,
        file,
        buf_w,
        buf_h,
    })
}

impl AttachedShm {
    pub fn destroy(self) {
        self.buffer.destroy();
        self.pool.destroy();
        drop(self.file);
    }
}

/// Snapshot of one `wl_output` in compositor space.
#[derive(Clone, Debug)]
pub struct OutputGeom {
    pub name: u32,
    pub output: wl_output::WlOutput,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub scale: i32,
}

/// Long-lived Wayland connection with layer-shell globals for outline/grab/overlay.
pub struct LayerShellConn {
    pub conn: Connection,
    pub queue: EventQueue<LayerShellState>,
    pub state: LayerShellState,
}

/// Accumulated pointer/keyboard samples for a layer-shell grab (drained by poll).
#[derive(Debug, Default, Clone, Copy)]
pub struct GrabAccum {
    pub x: i32,
    pub y: i32,
    pub moved: bool,
    pub left_clicks: u32,
    pub left_releases: u32,
    pub escape: bool,
}

#[derive(Default)]
pub struct LayerShellState {
    pub compositor: Option<wl_compositor::WlCompositor>,
    pub shm: Option<wl_shm::WlShm>,
    pub layer_shell: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    pub seat: Option<wl_seat::WlSeat>,
    pub outputs: HashMap<u32, OutputSlot>,
    pub configure_serial: Option<u32>,
    pub layer_closed: bool,
    /// Desktop origin + scale of the active grab surface (for local→desktop coords).
    pub grab_origin: (i32, i32),
    pub grab_scale: i32,
    pub grab: GrabAccum,
}

pub struct OutputSlot {
    pub output: wl_output::WlOutput,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub scale: i32,
    pub pending_x: i32,
    pub pending_y: i32,
    pub pending_w: i32,
    pub pending_h: i32,
    pub pending_scale: i32,
}

impl LayerShellConn {
    /// Connect and bind compositor/shm/layer-shell/outputs. Fails when layer-shell is missing.
    pub fn connect() -> Result<Self, CaptureError> {
        let conn = Connection::connect_to_env()
            .map_err(|e| CaptureError::Message(format!("wayland connect: {e}")))?;
        let mut queue = conn.new_event_queue();
        let qh = queue.handle();
        let _registry = conn.display().get_registry(&qh, ());
        let mut state = LayerShellState::default();
        queue
            .roundtrip(&mut state)
            .map_err(|e| CaptureError::Message(format!("layer-shell registry: {e}")))?;
        for _ in 0..4 {
            queue
                .roundtrip(&mut state)
                .map_err(|e| CaptureError::Message(format!("layer-shell outputs: {e}")))?;
        }
        if state.compositor.is_none() || state.shm.is_none() {
            return Err(CaptureError::Message(
                "wayland missing compositor or shm".into(),
            ));
        }
        if state.layer_shell.is_none() {
            return Err(CaptureError::Message(
                "compositor does not advertise zwlr_layer_shell_v1".into(),
            ));
        }
        if state.outputs.is_empty() {
            return Err(CaptureError::Message("no wl_output advertised".into()));
        }
        Ok(Self { conn, queue, state })
    }

    pub fn flush(&self) {
        let _ = self.conn.flush();
    }

    pub fn roundtrip(&mut self) -> Result<(), CaptureError> {
        self.queue
            .roundtrip(&mut self.state)
            .map(|_| ())
            .map_err(|e| CaptureError::Message(format!("wayland roundtrip: {e}")))
    }

    pub fn outputs(&self) -> Vec<OutputGeom> {
        self.state
            .outputs
            .iter()
            .filter(|(_, o)| o.width > 0 && o.height > 0)
            .map(|(name, o)| OutputGeom {
                name: *name,
                output: o.output.clone(),
                x: o.x,
                y: o.y,
                width: o.width,
                height: o.height,
                scale: o.scale.max(1),
            })
            .collect()
    }

    /// First output whose desktop rect overlaps `rect`.
    pub fn overlapping_output(&self, rect: DesktopRect) -> Option<OutputGeom> {
        let cosmic = sqyre_ports::cosmic_desktop_session();
        self.outputs().into_iter().find(|o| {
            let dest = output_desktop_rect(o.x, o.y, o.width, o.height, o.scale, cosmic);
            rects_overlap(dest, rect)
        })
    }

    pub fn wait_configure(&mut self, rounds: usize) -> bool {
        for _ in 0..rounds {
            if self.state.configure_serial.is_some() || self.state.layer_closed {
                break;
            }
            if self.roundtrip().is_err() {
                return false;
            }
        }
        self.state.configure_serial.is_some()
    }

    /// Create an overlay layer surface; caller commits and waits for configure.
    pub fn create_overlay_surface(
        &self,
        output: &wl_output::WlOutput,
        namespace: &str,
        keyboard: zwlr_layer_surface_v1::KeyboardInteractivity,
    ) -> Result<
        (
            wl_surface::WlSurface,
            zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        ),
        CaptureError,
    > {
        let qh = self.queue.handle();
        let compositor = self
            .state
            .compositor
            .as_ref()
            .ok_or_else(|| CaptureError::Message("compositor gone".into()))?;
        let layer_shell = self
            .state
            .layer_shell
            .as_ref()
            .ok_or_else(|| CaptureError::Message("layer_shell gone".into()))?;
        let surface = compositor.create_surface(&qh, ());
        let layer = layer_shell.get_layer_surface(
            &surface,
            Some(output),
            zwlr_layer_shell_v1::Layer::Overlay,
            namespace.to_string(),
            &qh,
            (),
        );
        layer.set_anchor(zwlr_layer_surface_v1::Anchor::Top | zwlr_layer_surface_v1::Anchor::Left);
        layer.set_exclusive_zone(0);
        layer.set_keyboard_interactivity(keyboard);
        Ok((surface, layer))
    }

    /// Empty input region (click-through).
    pub fn set_input_empty(&self, surface: &wl_surface::WlSurface) -> Result<(), CaptureError> {
        let qh = self.queue.handle();
        let compositor = self
            .state
            .compositor
            .as_ref()
            .ok_or_else(|| CaptureError::Message("compositor gone".into()))?;
        let empty = compositor.create_region(&qh, ());
        surface.set_input_region(Some(&empty));
        empty.destroy();
        Ok(())
    }

    /// Full-surface input region.
    pub fn set_input_full(&self, surface: &wl_surface::WlSurface) -> Result<(), CaptureError> {
        surface.set_input_region(None);
        Ok(())
    }

    /// Position a layer surface for a desktop-pixel rect on `output`.
    pub fn position_layer(
        layer: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        rect: DesktopRect,
        output: &OutputGeom,
    ) -> (u32, u32) {
        let cosmic = sqyre_ports::cosmic_desktop_session();
        let desktop = output_desktop_rect(
            output.x,
            output.y,
            output.width,
            output.height,
            output.scale,
            cosmic,
        );
        let (left, top, w, h) = layer_shell_logical(rect, desktop, output.scale, cosmic);
        layer.set_size(w, h);
        layer.set_margin(top, 0, 0, left);
        (w, h)
    }
}

/// True when a short-lived overlay layer surface can be created and configured.
pub fn layer_shell_surface_works() -> Result<(), CaptureError> {
    let mut conn = LayerShellConn::connect()?;
    let Some(out) = conn.outputs().into_iter().next() else {
        return Err(CaptureError::Message("no usable wl_output".into()));
    };
    let (surface, layer) = conn.create_overlay_surface(
        &out.output,
        "sqyre-probe",
        zwlr_layer_surface_v1::KeyboardInteractivity::None,
    )?;
    let _ = conn.set_input_empty(&surface);
    let (w, h) = LayerShellConn::position_layer(
        &layer,
        DesktopRect {
            x: out.x,
            y: out.y,
            w: 2,
            h: 2,
        },
        &out,
    );
    conn.state.configure_serial = None;
    conn.state.layer_closed = false;
    surface.commit();
    if !conn.wait_configure(8) {
        layer.destroy();
        surface.destroy();
        conn.flush();
        return Err(CaptureError::Message(
            "layer-shell configure timed out".into(),
        ));
    }
    let Some(serial) = conn.state.configure_serial.take() else {
        layer.destroy();
        surface.destroy();
        conn.flush();
        return Err(CaptureError::Message(
            "layer-shell missing configure serial".into(),
        ));
    };
    layer.ack_configure(serial);
    let shm = conn
        .state
        .shm
        .clone()
        .ok_or_else(|| CaptureError::Message("shm gone".into()))?;
    let qh = conn.queue.handle();
    let mapped = attach_transparent(&surface, &shm, &qh, w as i32, h as i32, out.scale, 0);
    let _ = conn.roundtrip();
    surface.attach(None, 0, 0);
    surface.commit();
    let _ = conn.roundtrip();
    if let Some(attached) = mapped {
        attached.destroy();
    }
    layer.destroy();
    surface.destroy();
    conn.flush();
    // Brief settle so compositors release the probe surface before the next open.
    std::thread::sleep(Duration::from_millis(1));
    Ok(())
}

impl Dispatch<wl_registry::WlRegistry, ()> for LayerShellState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        else {
            return;
        };
        match &interface[..] {
            "wl_compositor" if state.compositor.is_none() => {
                state.compositor = Some(registry.bind::<wl_compositor::WlCompositor, _, _>(
                    name,
                    version.min(4),
                    qh,
                    (),
                ));
            }
            "wl_shm" if state.shm.is_none() => {
                state.shm =
                    Some(registry.bind::<wl_shm::WlShm, _, _>(name, version.min(1), qh, ()));
            }
            "wl_seat" if state.seat.is_none() => {
                state.seat =
                    Some(registry.bind::<wl_seat::WlSeat, _, _>(name, version.min(8), qh, ()));
            }
            "wl_output" => {
                let output =
                    registry.bind::<wl_output::WlOutput, _, _>(name, version.min(4), qh, name);
                state.outputs.insert(
                    name,
                    OutputSlot {
                        output,
                        x: 0,
                        y: 0,
                        width: 0,
                        height: 0,
                        scale: 1,
                        pending_x: 0,
                        pending_y: 0,
                        pending_w: 0,
                        pending_h: 0,
                        pending_scale: 1,
                    },
                );
            }
            i if i == zwlr_layer_shell_v1::ZwlrLayerShellV1::interface().name
                && state.layer_shell.is_none() =>
            {
                state.layer_shell = Some(
                    registry.bind::<zwlr_layer_shell_v1::ZwlrLayerShellV1, _, _>(
                        name,
                        version.min(4),
                        qh,
                        (),
                    ),
                );
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_output::WlOutput, u32> for LayerShellState {
    fn event(
        state: &mut Self,
        _: &wl_output::WlOutput,
        event: wl_output::Event,
        name: &u32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let Some(slot) = state.outputs.get_mut(name) else {
            return;
        };
        match event {
            wl_output::Event::Geometry {
                x,
                y,
                physical_width: _,
                physical_height: _,
                subpixel: _,
                make: _,
                model: _,
                transform: _,
            } => {
                slot.pending_x = x;
                slot.pending_y = y;
            }
            wl_output::Event::Mode {
                flags,
                width,
                height,
                refresh: _,
            } => {
                let current = match flags {
                    WEnum::Value(f) => f.contains(wl_output::Mode::Current),
                    WEnum::Unknown(_) => false,
                };
                if current {
                    slot.pending_w = width;
                    slot.pending_h = height;
                }
            }
            wl_output::Event::Scale { factor } => {
                slot.pending_scale = factor.max(1);
            }
            wl_output::Event::Done => {
                slot.x = slot.pending_x;
                slot.y = slot.pending_y;
                slot.width = slot.pending_w;
                slot.height = slot.pending_h;
                slot.scale = slot.pending_scale.max(1);
            }
            _ => {}
        }
    }
}

impl Dispatch<zwlr_layer_shell_v1::ZwlrLayerShellV1, ()> for LayerShellState {
    fn event(
        _: &mut Self,
        _: &zwlr_layer_shell_v1::ZwlrLayerShellV1,
        _: zwlr_layer_shell_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for LayerShellState {
    fn event(
        state: &mut Self,
        surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure { serial, .. } => {
                state.configure_serial = Some(serial);
                let _ = surface;
            }
            zwlr_layer_surface_v1::Event::Closed => {
                state.layer_closed = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_compositor::WlCompositor, ()> for LayerShellState {
    fn event(
        _: &mut Self,
        _: &wl_compositor::WlCompositor,
        _: wl_compositor::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_shm::WlShm, ()> for LayerShellState {
    fn event(
        _: &mut Self,
        _: &wl_shm::WlShm,
        _: wl_shm::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_shm_pool::WlShmPool, ()> for LayerShellState {
    fn event(
        _: &mut Self,
        _: &wl_shm_pool::WlShmPool,
        _: wl_shm_pool::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_buffer::WlBuffer, ()> for LayerShellState {
    fn event(
        _: &mut Self,
        _: &wl_buffer::WlBuffer,
        _: wl_buffer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_surface::WlSurface, ()> for LayerShellState {
    fn event(
        _: &mut Self,
        _: &wl_surface::WlSurface,
        _: wl_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_region::WlRegion, ()> for LayerShellState {
    fn event(
        _: &mut Self,
        _: &wl_region::WlRegion,
        _: wl_region::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for LayerShellState {
    fn event(
        _: &mut Self,
        _: &wl_seat::WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // Capabilities are optional; grab calls get_pointer/get_keyboard directly.
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for LayerShellState {
    fn event(
        state: &mut Self,
        _: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let scale = state.grab_scale.max(1);
        let (ox, oy) = state.grab_origin;
        match event {
            wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                let x = ox + (surface_x as i32).saturating_mul(scale);
                let y = oy + (surface_y as i32).saturating_mul(scale);
                state.grab.x = x;
                state.grab.y = y;
                state.grab.moved = true;
            }
            wl_pointer::Event::Button {
                button,
                state: btn_state,
                ..
            } => {
                // Linux BTN_LEFT = 0x110
                if button == 0x110 {
                    match btn_state {
                        WEnum::Value(wl_pointer::ButtonState::Pressed) => {
                            state.grab.left_clicks = state.grab.left_clicks.saturating_add(1);
                        }
                        WEnum::Value(wl_pointer::ButtonState::Released) => {
                            state.grab.left_releases = state.grab.left_releases.saturating_add(1);
                        }
                        _ => {}
                    }
                }
            }
            wl_pointer::Event::Enter {
                surface_x,
                surface_y,
                ..
            } => {
                let x = ox + (surface_x as i32).saturating_mul(scale);
                let y = oy + (surface_y as i32).saturating_mul(scale);
                state.grab.x = x;
                state.grab.y = y;
                state.grab.moved = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for LayerShellState {
    fn event(
        state: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_keyboard::Event::Key {
            key,
            state: key_state,
            ..
        } = event
        {
            // Evdev KEY_ESC = 1
            if key == 1 && matches!(key_state, WEnum::Value(wl_keyboard::KeyState::Pressed)) {
                state.grab.escape = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_shell_logical_identity_scale_one() {
        let desktop = DesktopRect {
            x: 100,
            y: 200,
            w: 1920,
            h: 1080,
        };
        let rect = DesktopRect {
            x: 140,
            y: 260,
            w: 40,
            h: 20,
        };
        assert_eq!(
            layer_shell_logical(rect, desktop, 1, false),
            (40, 60, 40, 20)
        );
    }

    #[test]
    fn layer_shell_logical_cosmic_hi_dpi() {
        let desktop = DesktopRect {
            x: 0,
            y: 0,
            w: 3840,
            h: 2160,
        };
        let rect = DesktopRect {
            x: 200,
            y: 400,
            w: 100,
            h: 50,
        };
        assert_eq!(
            layer_shell_logical(rect, desktop, 2, true),
            (100, 200, 50, 25)
        );
    }

    #[test]
    fn output_desktop_rect_cosmic_scales_origin() {
        assert_eq!(
            output_desktop_rect(10, 20, 3840, 2160, 2, true),
            DesktopRect {
                x: 20,
                y: 40,
                w: 3840,
                h: 2160
            }
        );
    }
}
