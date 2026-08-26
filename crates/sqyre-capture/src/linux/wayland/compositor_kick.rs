//! Force compositor stage damage so portal ScreenCast emits a fresh frame.
//!
//! Mutter (and other emit-on-damage streams) may leave the PipeWire cache idle
//! after a click. Kick backends, in order:
//! 1. `zwlr_layer_shell_v1` transparent overlay (wlroots / Cosmic / …)
//! 2. `xdg_toplevel` fullscreen on the target `wl_output` (GNOME / Mutter)
//! 3. X11 ARGB overlay via XWayland (last resort)

use crate::cap_log;
use crate::x11_capture::CompositorKick as X11Kick;
use sqyre_ports::DesktopRect;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::os::fd::{AsFd, AsRawFd, FromRawFd, IntoRawFd, OwnedFd};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_output, wl_region, wl_registry, wl_shm, wl_shm_pool, wl_surface,
};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1, zwlr_layer_surface_v1,
};

/// Damage kick used by portal fresh-capture waits.
pub(crate) enum DamageKick {
    Wayland(WaylandKick),
    X11(X11Kick),
}

impl DamageKick {
    /// Prefer native Wayland (layer-shell, then xdg fullscreen); then X11.
    pub(crate) fn open() -> Option<Self> {
        if let Some(w) = WaylandKick::open() {
            return Some(Self::Wayland(w));
        }
        X11Kick::open().map(Self::X11)
    }

    pub(crate) fn pulse_rect(&mut self, rect: DesktopRect) {
        match self {
            Self::Wayland(w) => w.pulse_rect(rect),
            Self::X11(x) => x.pulse_rect(rect),
        }
    }
}

enum KickCmd {
    Pulse {
        rect: DesktopRect,
        done: SyncSender<()>,
    },
    Shutdown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WaylandKickKind {
    LayerShell,
    /// GNOME/Mutter: fullscreen transparent xdg_toplevel on the search output.
    XdgFullscreen,
}

/// Persistent Wayland connection on a dedicated thread (EventQueue is !Send).
pub(crate) struct WaylandKick {
    tx: Sender<KickCmd>,
    thread: Option<JoinHandle<()>>,
}

impl WaylandKick {
    pub(crate) fn open() -> Option<Self> {
        let (ready_tx, ready_rx) = mpsc::sync_channel::<Option<WaylandKickKind>>(1);
        let (cmd_tx, cmd_rx) = mpsc::channel::<KickCmd>();
        let thread = thread::Builder::new()
            .name("sqyre-wl-kick".into())
            .spawn(move || kick_thread(ready_tx, cmd_rx))
            .ok()?;
        let kind = ready_rx
            .recv_timeout(Duration::from_secs(2))
            .ok()
            .flatten();
        let Some(kind) = kind else {
            let _ = cmd_tx.send(KickCmd::Shutdown);
            let _ = thread.join();
            return None;
        };
        let backend = match kind {
            WaylandKickKind::LayerShell => "wayland-layer-shell",
            WaylandKickKind::XdgFullscreen => "wayland-xdg-fullscreen",
        };
        cap_log("PORTAL", "kick", &format!("backend={backend}"));
        Some(Self {
            tx: cmd_tx,
            thread: Some(thread),
        })
    }

    pub(crate) fn pulse_rect(&mut self, rect: DesktopRect) {
        let (done_tx, done_rx) = mpsc::sync_channel(1);
        if self
            .tx
            .send(KickCmd::Pulse {
                rect,
                done: done_tx,
            })
            .is_err()
        {
            return;
        }
        let _ = done_rx.recv_timeout(Duration::from_millis(250));
    }
}

impl Drop for WaylandKick {
    fn drop(&mut self) {
        let _ = self.tx.send(KickCmd::Shutdown);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn kick_thread(ready: SyncSender<Option<WaylandKickKind>>, cmds: Receiver<KickCmd>) {
    let mut conn = match KickConn::connect() {
        Ok(c) => {
            let _ = ready.send(Some(c.kind));
            c
        }
        Err(_) => {
            let _ = ready.send(None);
            return;
        }
    };
    while let Ok(cmd) = cmds.recv() {
        match cmd {
            KickCmd::Pulse { rect, done } => {
                conn.pulse(rect);
                let _ = done.send(());
            }
            KickCmd::Shutdown => break,
        }
    }
}

struct OutputInfo {
    output: wl_output::WlOutput,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    scale: i32,
    pending_x: i32,
    pending_y: i32,
    pending_w: i32,
    pending_h: i32,
    pending_scale: i32,
}

struct KickState {
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    layer_shell: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    wm_base: Option<xdg_wm_base::XdgWmBase>,
    outputs: HashMap<u32, OutputInfo>,
    configure_serial: Option<u32>,
    configure_w: i32,
    configure_h: i32,
    layer_closed: bool,
}

struct KickConn {
    conn: Connection,
    queue: EventQueue<KickState>,
    state: KickState,
    kind: WaylandKickKind,
}

impl KickConn {
    fn connect() -> Result<Self, ()> {
        let conn = Connection::connect_to_env().map_err(|_| ())?;
        let mut queue = conn.new_event_queue();
        let qh = queue.handle();
        let _registry = conn.display().get_registry(&qh, ());
        let mut state = KickState {
            compositor: None,
            shm: None,
            layer_shell: None,
            wm_base: None,
            outputs: HashMap::new(),
            configure_serial: None,
            configure_w: 0,
            configure_h: 0,
            layer_closed: false,
        };
        queue.roundtrip(&mut state).map_err(|_| ())?;
        for _ in 0..4 {
            queue.roundtrip(&mut state).map_err(|_| ())?;
        }
        if state.compositor.is_none() || state.shm.is_none() || state.outputs.is_empty() {
            return Err(());
        }
        let kind = if state.layer_shell.is_some() {
            WaylandKickKind::LayerShell
        } else if state.wm_base.is_some() {
            WaylandKickKind::XdgFullscreen
        } else {
            return Err(());
        };
        Ok(Self {
            conn,
            queue,
            state,
            kind,
        })
    }

    fn pulse(&mut self, rect: DesktopRect) {
        if rect.w <= 1 || rect.h <= 1 {
            return;
        }
        match self.kind {
            WaylandKickKind::LayerShell => self.pulse_layer_shell(rect),
            WaylandKickKind::XdgFullscreen => self.pulse_xdg_fullscreen(rect),
        }
    }

    fn pulse_layer_shell(&mut self, rect: DesktopRect) {
        let Some(out_id) = overlapping_output_id(&self.state, rect) else {
            return;
        };
        let (out_x, out_y, out_w, out_h, scale, output) = {
            let o = self.state.outputs.get(&out_id).expect("output id");
            (
                o.x,
                o.y,
                o.width.max(1),
                o.height.max(1),
                o.scale.max(1),
                o.output.clone(),
            )
        };
        let kick = clamp_kick_rect(
            rect,
            DesktopRect {
                x: out_x,
                y: out_y,
                w: out_w,
                h: out_h,
            },
        );
        if kick.w <= 1 || kick.h <= 1 {
            return;
        }

        let qh = self.queue.handle();
        let compositor = self.state.compositor.as_ref().expect("compositor").clone();
        let shm = self.state.shm.as_ref().expect("shm").clone();
        let layer_shell = self.state.layer_shell.as_ref().expect("layer_shell").clone();

        let surface = compositor.create_surface(&qh, ());
        set_empty_input(&compositor, &surface, &qh);

        let layer = layer_shell.get_layer_surface(
            &surface,
            Some(&output),
            zwlr_layer_shell_v1::Layer::Overlay,
            "sqyre-kick".to_string(),
            &qh,
            (),
        );
        layer.set_size(kick.w as u32, kick.h as u32);
        layer.set_anchor(
            zwlr_layer_surface_v1::Anchor::Top | zwlr_layer_surface_v1::Anchor::Left,
        );
        let margin_top = (kick.y - out_y).max(0);
        let margin_left = (kick.x - out_x).max(0);
        layer.set_margin(margin_top, 0, 0, margin_left);
        layer.set_exclusive_zone(0);
        layer.set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::None);

        self.state.configure_serial = None;
        self.state.layer_closed = false;
        surface.commit();
        if !self.wait_configure() {
            layer.destroy();
            surface.destroy();
            let _ = self.conn.flush();
            return;
        }
        let serial = self.state.configure_serial.take().expect("configure");
        layer.ack_configure(serial);

        let mapped = attach_transparent(&surface, &shm, &qh, kick.w, kick.h, scale);
        let _ = self.queue.roundtrip(&mut self.state);

        surface.attach(None, 0, 0);
        surface.commit();
        let _ = self.queue.roundtrip(&mut self.state);

        if let Some((buffer, pool, file)) = mapped {
            buffer.destroy();
            pool.destroy();
            drop(file);
        }
        layer.destroy();
        surface.destroy();
        let _ = self.conn.flush();
    }

    /// Mutter has no layer-shell; fullscreen a transparent xdg_toplevel on the
    /// overlapping output so that monitor's ScreenCast stream sees stage damage.
    fn pulse_xdg_fullscreen(&mut self, rect: DesktopRect) {
        let Some(out_id) = overlapping_output_id(&self.state, rect) else {
            return;
        };
        let (scale, output, out_w, out_h) = {
            let o = self.state.outputs.get(&out_id).expect("output id");
            (
                o.scale.max(1),
                o.output.clone(),
                o.width.max(1),
                o.height.max(1),
            )
        };

        let qh = self.queue.handle();
        let compositor = self.state.compositor.as_ref().expect("compositor").clone();
        let shm = self.state.shm.as_ref().expect("shm").clone();
        let wm_base = self.state.wm_base.as_ref().expect("wm_base").clone();

        let surface = compositor.create_surface(&qh, ());
        set_empty_input(&compositor, &surface, &qh);

        let xdg_surf = wm_base.get_xdg_surface(&surface, &qh, ());
        let toplevel = xdg_surf.get_toplevel(&qh, ());
        toplevel.set_app_id("sqyre-kick".to_string());
        toplevel.set_title(String::new());
        toplevel.set_fullscreen(Some(&output));

        self.state.configure_serial = None;
        self.state.configure_w = 0;
        self.state.configure_h = 0;
        surface.commit();
        if !self.wait_configure() {
            toplevel.destroy();
            xdg_surf.destroy();
            surface.destroy();
            let _ = self.conn.flush();
            return;
        }
        let serial = self.state.configure_serial.take().expect("configure");
        xdg_surf.ack_configure(serial);

        // Tiny transparent buffer: the window is still fullscreen on `output`
        // (that is what damages the stream); avoid a full-desktop memfd.
        let bump_w = self.state.configure_w.max(2).min(out_w).min(64);
        let bump_h = self.state.configure_h.max(2).min(out_h).min(64);
        let mapped = attach_transparent(&surface, &shm, &qh, bump_w, bump_h, scale);
        let _ = self.queue.roundtrip(&mut self.state);

        // Destroy unmaps; do not leave a fullscreen surface up during the wait.
        toplevel.destroy();
        xdg_surf.destroy();
        surface.destroy();
        if let Some((buffer, pool, file)) = mapped {
            buffer.destroy();
            pool.destroy();
            drop(file);
        }
        let _ = self.queue.roundtrip(&mut self.state);
        let _ = self.conn.flush();
    }

    fn wait_configure(&mut self) -> bool {
        for _ in 0..8 {
            if self.state.configure_serial.is_some() || self.state.layer_closed {
                break;
            }
            if self.queue.roundtrip(&mut self.state).is_err() {
                return false;
            }
        }
        self.state.configure_serial.is_some()
    }
}

fn set_empty_input(
    compositor: &wl_compositor::WlCompositor,
    surface: &wl_surface::WlSurface,
    qh: &QueueHandle<KickState>,
) {
    let empty = compositor.create_region(qh, ());
    surface.set_input_region(Some(&empty));
    empty.destroy();
}

fn attach_transparent(
    surface: &wl_surface::WlSurface,
    shm: &wl_shm::WlShm,
    qh: &QueueHandle<KickState>,
    width: i32,
    height: i32,
    scale: i32,
) -> Option<(wl_buffer::WlBuffer, wl_shm_pool::WlShmPool, File)> {
    let width = width.max(2);
    let height = height.max(2);
    let scale = scale.max(1);
    let buf_w = width.saturating_mul(scale);
    let buf_h = height.saturating_mul(scale);
    let stride = buf_w.saturating_mul(4);
    let bytes = stride.saturating_mul(buf_h) as usize;
    let mut file = transparent_shm_file(bytes)?;
    let _ = file.flush();
    let pool = shm.create_pool(file.as_fd(), bytes as i32, qh, ());
    let buffer = pool.create_buffer(
        0,
        buf_w,
        buf_h,
        stride,
        wl_shm::Format::Argb8888,
        qh,
        (),
    );
    surface.set_buffer_scale(scale);
    surface.attach(Some(&buffer), 0, 0);
    surface.damage_buffer(0, 0, buf_w, buf_h);
    surface.commit();
    Some((buffer, pool, file))
}

fn overlapping_output_id(state: &KickState, rect: DesktopRect) -> Option<u32> {
    state.outputs.iter().find_map(|(id, o)| {
        if o.width <= 0 || o.height <= 0 {
            return None;
        }
        let dest = DesktopRect {
            x: o.x,
            y: o.y,
            w: o.width,
            h: o.height,
        };
        if rects_overlap(dest, rect) {
            Some(*id)
        } else {
            None
        }
    })
}

fn clamp_kick_rect(rect: DesktopRect, dest: DesktopRect) -> DesktopRect {
    let left = rect.x.max(dest.x);
    let top = rect.y.max(dest.y);
    let right = (rect.x + rect.w).min(dest.x + dest.w);
    let bottom = (rect.y + rect.h).min(dest.y + dest.h);
    if right - left < 2 || bottom - top < 2 {
        DesktopRect {
            x: dest.x,
            y: dest.y,
            w: 2.min(dest.w),
            h: 2.min(dest.h),
        }
    } else {
        DesktopRect {
            x: left,
            y: top,
            w: right - left,
            h: bottom - top,
        }
    }
}

fn rects_overlap(a: DesktopRect, b: DesktopRect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

fn transparent_shm_file(bytes: usize) -> Option<File> {
    if bytes == 0 {
        return None;
    }
    let fd = unsafe {
        libc::memfd_create(
            c"sqyre-kick".as_ptr(),
            libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING,
        )
    };
    if fd < 0 {
        return None;
    }
    let owned = unsafe { OwnedFd::from_raw_fd(fd) };
    if unsafe { libc::ftruncate(owned.as_raw_fd(), bytes as libc::off_t) } != 0 {
        return None;
    }
    let raw = owned.into_raw_fd();
    Some(unsafe { File::from_raw_fd(raw) })
}

impl Dispatch<wl_registry::WlRegistry, ()> for KickState {
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
                state.shm = Some(registry.bind::<wl_shm::WlShm, _, _>(
                    name,
                    version.min(1),
                    qh,
                    (),
                ));
            }
            "wl_output" => {
                let output = registry.bind::<wl_output::WlOutput, _, _>(
                    name,
                    version.min(4),
                    qh,
                    name,
                );
                state.outputs.insert(
                    name,
                    OutputInfo {
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
            "xdg_wm_base" if state.wm_base.is_none() => {
                state.wm_base = Some(registry.bind::<xdg_wm_base::XdgWmBase, _, _>(
                    name,
                    version.min(4),
                    qh,
                    (),
                ));
            }
            i if i == zwlr_layer_shell_v1::ZwlrLayerShellV1::interface().name
                && state.layer_shell.is_none() =>
            {
                state.layer_shell =
                    Some(registry.bind::<zwlr_layer_shell_v1::ZwlrLayerShellV1, _, _>(
                        name,
                        version.min(4),
                        qh,
                        (),
                    ));
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_output::WlOutput, u32> for KickState {
    fn event(
        state: &mut Self,
        _: &wl_output::WlOutput,
        event: wl_output::Event,
        name: &u32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let Some(out) = state.outputs.get_mut(name) else {
            return;
        };
        match event {
            wl_output::Event::Geometry { x, y, .. } => {
                out.pending_x = x;
                out.pending_y = y;
            }
            wl_output::Event::Mode {
                flags,
                width,
                height,
                ..
            } => {
                let current = match flags {
                    WEnum::Value(f) => f.contains(wl_output::Mode::Current),
                    WEnum::Unknown(_) => false,
                };
                if current {
                    out.pending_w = width;
                    out.pending_h = height;
                }
            }
            wl_output::Event::Scale { factor } => {
                out.pending_scale = factor.max(1);
            }
            wl_output::Event::Done => {
                out.x = out.pending_x;
                out.y = out.pending_y;
                if out.pending_w > 0 && out.pending_h > 0 {
                    out.width = out.pending_w;
                    out.height = out.pending_h;
                }
                out.scale = out.pending_scale.max(1);
            }
            _ => {}
        }
    }
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for KickState {
    fn event(
        state: &mut Self,
        _: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure { serial, .. } => {
                state.configure_serial = Some(serial);
            }
            zwlr_layer_surface_v1::Event::Closed => {
                state.layer_closed = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for KickState {
    fn event(
        _: &mut Self,
        wm_base: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            wm_base.pong(serial);
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, ()> for KickState {
    fn event(
        state: &mut Self,
        _: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            state.configure_serial = Some(serial);
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for KickState {
    fn event(
        state: &mut Self,
        _: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_toplevel::Event::Configure { width, height, .. } = event {
            if width > 0 {
                state.configure_w = width;
            }
            if height > 0 {
                state.configure_h = height;
            }
        }
    }
}

impl Dispatch<wl_compositor::WlCompositor, ()> for KickState {
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

impl Dispatch<wl_shm::WlShm, ()> for KickState {
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

impl Dispatch<wl_shm_pool::WlShmPool, ()> for KickState {
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

impl Dispatch<wl_buffer::WlBuffer, ()> for KickState {
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

impl Dispatch<wl_surface::WlSurface, ()> for KickState {
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

impl Dispatch<wl_region::WlRegion, ()> for KickState {
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

impl Dispatch<zwlr_layer_shell_v1::ZwlrLayerShellV1, ()> for KickState {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_kick_rect_intersects_monitor() {
        let dest = DesktopRect {
            x: 100,
            y: 50,
            w: 800,
            h: 600,
        };
        let search = DesktopRect {
            x: 80,
            y: 40,
            w: 100,
            h: 80,
        };
        let kick = clamp_kick_rect(search, dest);
        assert_eq!(kick.x, 100);
        assert_eq!(kick.y, 50);
        assert_eq!(kick.w, 80);
        assert_eq!(kick.h, 70);
    }

    #[test]
    fn clamp_kick_rect_tiny_falls_back() {
        let dest = DesktopRect {
            x: 0,
            y: 0,
            w: 1920,
            h: 1080,
        };
        let search = DesktopRect {
            x: 10,
            y: 10,
            w: 1,
            h: 1,
        };
        let kick = clamp_kick_rect(search, dest);
        assert_eq!(kick.w, 2);
        assert_eq!(kick.h, 2);
    }
}
