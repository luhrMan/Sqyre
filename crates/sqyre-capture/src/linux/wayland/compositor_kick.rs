//! Force compositor stage damage so portal ScreenCast emits a fresh frame.
//!
//! Backends (first that connects):
//! 1. `zwlr_layer_shell_v1` — transparent overlay pulse (wlroots / Cosmic / …)
//! 2. `xdg_toplevel` **windowed** (not fullscreen) — Mutter draws fullscreen
//!    surfaces opaque black (alpha ignored). Windowed ARGB with an empty opaque
//!    region stays transparent; we keep a small surface mapped for the fresh
//!    wait and damage-flip it, then unmap on [`DamageKick::release_stage`].
//!    Mapping steals activation on GNOME — callers must refocus the prior
//!    window after `release_stage` before injecting EIS clicks.

use crate::{cap_log, mark_site};
use sqyre_ports::DesktopRect;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::os::fd::{AsFd, AsRawFd, FromRawFd, IntoRawFd, OwnedFd};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_output, wl_region, wl_registry, wl_shm, wl_shm_pool, wl_surface,
};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum};
use wayland_protocols::xdg::decoration::zv1::client::{
    zxdg_decoration_manager_v1, zxdg_toplevel_decoration_v1,
};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

/// Cap windowed kick size so a failed-alpha path cannot cover the desk.
const XDG_KICK_MAX: i32 = 64;

/// Layer-shell or windowed-xdg damage kick for portal fresh-capture waits.
pub(crate) struct DamageKick {
    tx: Sender<KickCmd>,
    thread: Option<JoinHandle<()>>,
    /// Unprocessed `Pulse`/`Release` cmds (Shutdown not counted). Grows when
    /// fire-and-forget enqueue outpaces Wayland roundtrips — Drop joins drain.
    pending: Arc<AtomicUsize>,
    /// When set, the kick thread skips remaining Pulse work and exits on Shutdown.
    shutting_down: Arc<std::sync::atomic::AtomicBool>,
}

enum KickCmd {
    Pulse {
        rect: DesktopRect,
        done: SyncSender<()>,
    },
    Release {
        done: SyncSender<()>,
    },
    Shutdown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KickKind {
    LayerShell,
    XdgWindowed,
}

impl DamageKick {
    /// `None` when neither layer-shell nor xdg-shell is available.
    pub(crate) fn open() -> Option<Self> {
        let (ready_tx, ready_rx) = mpsc::sync_channel::<Option<KickKind>>(1);
        let (cmd_tx, cmd_rx) = mpsc::channel::<KickCmd>();
        let pending = Arc::new(AtomicUsize::new(0));
        let shutting_down = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let pending_thread = Arc::clone(&pending);
        let shutdown_thread = Arc::clone(&shutting_down);
        let thread = thread::Builder::new()
            .name("sqyre-wl-kick".into())
            .spawn(move || kick_thread(ready_tx, cmd_rx, pending_thread, shutdown_thread))
            .ok()?;
        let kind = ready_rx.recv_timeout(Duration::from_secs(2)).ok().flatten();
        let Some(kind) = kind else {
            shutting_down.store(true, Ordering::SeqCst);
            let _ = cmd_tx.send(KickCmd::Shutdown);
            let _ = thread.join();
            return None;
        };
        let backend = match kind {
            KickKind::LayerShell => "wayland-layer-shell",
            KickKind::XdgWindowed => "wayland-xdg-windowed",
        };
        cap_log("PORTAL", "kick", &format!("backend={backend}"));
        Some(Self {
            tx: cmd_tx,
            thread: Some(thread),
            pending,
            shutting_down,
        })
    }

    pub(crate) fn pulse_rect(&mut self, rect: DesktopRect) {
        if self.shutting_down.load(Ordering::SeqCst) {
            return;
        }
        // Fire-and-forget during the PipeWire wait; [`release_stage`] blocks until
        // this pulse (and later cmds) finish so Move/Click cannot race the toplevel.
        let (done_tx, _done_rx) = mpsc::sync_channel(1);
        self.pending.fetch_add(1, Ordering::Relaxed);
        if self
            .tx
            .send(KickCmd::Pulse {
                rect,
                done: done_tx,
            })
            .is_err()
        {
            self.pending.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// Allow another xdg pulse on the next fresh-wait.
    ///
    /// Blocks until this `Release` (and any earlier `Pulse` ahead of it) finishes.
    /// Fresh-capture used to return while the kick toplevel was still mapping —
    /// on GNOME that intermittently stole focus / ate the following EIS click.
    pub(crate) fn release_stage(&mut self) {
        if self.shutting_down.load(Ordering::SeqCst) {
            return;
        }
        let (done_tx, done_rx) = mpsc::sync_channel(1);
        self.pending.fetch_add(1, Ordering::Relaxed);
        if self.tx.send(KickCmd::Release { done: done_tx }).is_err() {
            self.pending.fetch_sub(1, Ordering::Relaxed);
            return;
        }
        let t0 = Instant::now();
        match done_rx.recv_timeout(Duration::from_millis(750)) {
            Ok(()) => {
                let ms = t0.elapsed().as_millis();
                if ms >= 20 {
                    cap_log("PORTAL", "kick", &format!("release_wait_ms={ms}"));
                }
            }
            Err(_) => {
                cap_log(
                    "PORTAL",
                    "kick",
                    &format!(
                        "release_wait=timeout pending={}",
                        self.pending.load(Ordering::Relaxed)
                    ),
                );
            }
        }
    }
}

impl Drop for DamageKick {
    fn drop(&mut self) {
        let pending = self.pending.load(Ordering::Relaxed);
        mark_site("kick:drop:before_join");
        let t0 = Instant::now();
        // Skip remaining Wayland map/unmap work — join used to drain the whole backlog.
        self.shutting_down.store(true, Ordering::SeqCst);
        let _ = self.tx.send(KickCmd::Shutdown);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        let ms = t0.elapsed().as_millis();
        cap_log(
            "PORTAL",
            "kick-drop",
            &format!("pending={pending} join_ms={ms}"),
        );
        mark_site("kick:drop:after_join");
    }
}

fn kick_thread(
    ready: SyncSender<Option<KickKind>>,
    cmds: Receiver<KickCmd>,
    pending: Arc<AtomicUsize>,
    shutting_down: Arc<std::sync::atomic::AtomicBool>,
) {
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
                if !shutting_down.load(Ordering::SeqCst) {
                    conn.pulse(rect);
                }
                let _ = done.send(());
                pending.fetch_sub(1, Ordering::Relaxed);
            }
            KickCmd::Release { done } => {
                if !shutting_down.load(Ordering::SeqCst) {
                    conn.release_stage();
                }
                let _ = done.send(());
                pending.fetch_sub(1, Ordering::Relaxed);
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

struct StagedXdg {
    surface: wl_surface::WlSurface,
    xdg_surface: xdg_surface::XdgSurface,
    toplevel: xdg_toplevel::XdgToplevel,
    decoration: Option<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>,
    buffer: wl_buffer::WlBuffer,
    pool: wl_shm_pool::WlShmPool,
    file: File,
    buf_w: i32,
    buf_h: i32,
    flip: u8,
}

struct KickState {
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    layer_shell: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    xdg_wm_base: Option<xdg_wm_base::XdgWmBase>,
    decoration_manager: Option<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
    outputs: HashMap<u32, OutputInfo>,
    configure_serial: Option<u32>,
    layer_closed: bool,
    xdg_configured: bool,
}

struct KickConn {
    conn: Connection,
    queue: EventQueue<KickState>,
    state: KickState,
    kind: KickKind,
    staged: Option<StagedXdg>,
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
            xdg_wm_base: None,
            decoration_manager: None,
            outputs: HashMap::new(),
            configure_serial: None,
            layer_closed: false,
            xdg_configured: false,
        };
        queue.roundtrip(&mut state).map_err(|_| ())?;
        for _ in 0..4 {
            queue.roundtrip(&mut state).map_err(|_| ())?;
        }
        if state.compositor.is_none() || state.shm.is_none() || state.outputs.is_empty() {
            return Err(());
        }
        let kind = if state.layer_shell.is_some() {
            KickKind::LayerShell
        } else if state.xdg_wm_base.is_some() {
            KickKind::XdgWindowed
        } else {
            return Err(());
        };
        Ok(Self {
            conn,
            queue,
            state,
            kind,
            staged: None,
        })
    }

    fn pulse(&mut self, rect: DesktopRect) {
        match self.kind {
            KickKind::LayerShell => self.pulse_layer(rect),
            KickKind::XdgWindowed => self.pulse_xdg(rect),
        }
    }

    fn release_stage(&mut self) {
        if self.staged.is_some() {
            self.destroy_staged();
        }
    }

    fn pulse_layer(&mut self, rect: DesktopRect) {
        if rect.w <= 1 || rect.h <= 1 {
            return;
        }
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
        let layer_shell = self
            .state
            .layer_shell
            .as_ref()
            .expect("layer_shell")
            .clone();

        let surface = compositor.create_surface(&qh, ());
        let empty = compositor.create_region(&qh, ());
        surface.set_input_region(Some(&empty));
        empty.destroy();

        let layer = layer_shell.get_layer_surface(
            &surface,
            Some(&output),
            zwlr_layer_shell_v1::Layer::Overlay,
            "sqyre-kick".to_string(),
            &qh,
            (),
        );
        layer.set_size(kick.w as u32, kick.h as u32);
        layer.set_anchor(zwlr_layer_surface_v1::Anchor::Top | zwlr_layer_surface_v1::Anchor::Left);
        layer.set_margin((kick.y - out_y).max(0), 0, 0, (kick.x - out_x).max(0));
        layer.set_exclusive_zone(0);
        layer.set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::None);

        self.state.configure_serial = None;
        self.state.layer_closed = false;
        surface.commit();
        if !self.wait_layer_configure() {
            layer.destroy();
            surface.destroy();
            let _ = self.conn.flush();
            return;
        }
        let serial = self.state.configure_serial.take().expect("configure");
        layer.ack_configure(serial);

        let mapped = attach_transparent(&surface, &shm, &qh, kick.w, kick.h, scale, 0);
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

    fn pulse_xdg(&mut self, rect: DesktopRect) {
        if self.staged.is_some() {
            self.damage_staged();
            return;
        }
        if rect.w <= 1 || rect.h <= 1 {
            return;
        }
        let Some(out_id) = overlapping_output_id(&self.state, rect) else {
            return;
        };
        let (out_w, out_h, scale) = {
            let o = self.state.outputs.get(&out_id).expect("output id");
            (o.width.max(1), o.height.max(1), o.scale.max(1))
        };
        let kick = clamp_kick_rect(
            rect,
            DesktopRect {
                x: 0,
                y: 0,
                w: out_w,
                h: out_h,
            },
        );
        // Small windowed surface: alpha works on Mutter for non-fullscreen.
        let w = kick.w.min(XDG_KICK_MAX).max(2);
        let h = kick.h.min(XDG_KICK_MAX).max(2);

        let qh = self.queue.handle();
        let compositor = self.state.compositor.as_ref().expect("compositor").clone();
        let shm = self.state.shm.as_ref().expect("shm").clone();
        let wm = self
            .state
            .xdg_wm_base
            .as_ref()
            .expect("xdg_wm_base")
            .clone();

        let surface = compositor.create_surface(&qh, ());
        let empty_in = compositor.create_region(&qh, ());
        surface.set_input_region(Some(&empty_in));
        empty_in.destroy();
        let empty_op = compositor.create_region(&qh, ());
        surface.set_opaque_region(Some(&empty_op));
        empty_op.destroy();

        let xdg_surface = wm.get_xdg_surface(&surface, &qh, ());
        let toplevel = xdg_surface.get_toplevel(&qh, ());
        toplevel.set_title("".into());
        toplevel.set_app_id("sqyre-kick".into());
        toplevel.set_min_size(w, h);
        toplevel.set_max_size(w, h);
        // Do NOT set_fullscreen — Mutter paints fullscreen opaque black.

        let decoration = self.state.decoration_manager.as_ref().map(|mgr| {
            let deco = mgr.get_toplevel_decoration(&toplevel, &qh, ());
            deco.set_mode(zxdg_toplevel_decoration_v1::Mode::ClientSide);
            deco
        });

        self.state.configure_serial = None;
        self.state.xdg_configured = false;
        surface.commit();
        if !self.wait_xdg_configure() {
            if let Some(d) = decoration {
                d.destroy();
            }
            toplevel.destroy();
            xdg_surface.destroy();
            surface.destroy();
            let _ = self.conn.flush();
            return;
        }
        let serial = self.state.configure_serial.take().expect("xdg configure");
        xdg_surface.ack_configure(serial);
        xdg_surface.set_window_geometry(0, 0, w, h);

        let Some((buffer, pool, file)) = attach_transparent(&surface, &shm, &qh, w, h, scale, 0)
        else {
            if let Some(d) = decoration {
                d.destroy();
            }
            toplevel.destroy();
            xdg_surface.destroy();
            surface.destroy();
            let _ = self.conn.flush();
            return;
        };
        let _ = self.queue.roundtrip(&mut self.state);
        let _ = self.conn.flush();

        self.staged = Some(StagedXdg {
            surface,
            xdg_surface,
            toplevel,
            decoration,
            buffer,
            pool,
            file,
            buf_w: w.saturating_mul(scale),
            buf_h: h.saturating_mul(scale),
            flip: 0,
        });
    }

    fn damage_staged(&mut self) {
        let Some(staged) = self.staged.as_mut() else {
            return;
        };
        staged.flip = staged.flip.wrapping_add(1);
        // Re-write one pixel so the shm contents change (damage alone can be ignored).
        let _ = write_flip_pixel(&mut staged.file, staged.flip);
        staged
            .surface
            .damage_buffer(0, 0, staged.buf_w, staged.buf_h);
        staged.surface.commit();
        let _ = self.queue.roundtrip(&mut self.state);
        let _ = self.conn.flush();
    }

    fn destroy_staged(&mut self) {
        let Some(staged) = self.staged.take() else {
            return;
        };
        staged.surface.attach(None, 0, 0);
        staged.surface.commit();
        let _ = self.queue.roundtrip(&mut self.state);
        if let Some(d) = staged.decoration {
            d.destroy();
        }
        staged.toplevel.destroy();
        staged.xdg_surface.destroy();
        staged.surface.destroy();
        staged.buffer.destroy();
        staged.pool.destroy();
        drop(staged.file);
        let _ = self.conn.flush();
    }

    fn wait_layer_configure(&mut self) -> bool {
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

    fn wait_xdg_configure(&mut self) -> bool {
        for _ in 0..12 {
            if self.state.configure_serial.is_some() && self.state.xdg_configured {
                break;
            }
            if self.queue.roundtrip(&mut self.state).is_err() {
                return false;
            }
        }
        self.state.configure_serial.is_some()
    }
}

fn attach_transparent(
    surface: &wl_surface::WlSurface,
    shm: &wl_shm::WlShm,
    qh: &QueueHandle<KickState>,
    width: i32,
    height: i32,
    scale: i32,
    flip: u8,
) -> Option<(wl_buffer::WlBuffer, wl_shm_pool::WlShmPool, File)> {
    let width = width.max(2);
    let height = height.max(2);
    let scale = scale.max(1);
    let buf_w = width.saturating_mul(scale);
    let buf_h = height.saturating_mul(scale);
    let stride = buf_w.saturating_mul(4);
    let bytes = stride.saturating_mul(buf_h) as usize;
    let mut file = transparent_shm_file(bytes)?;
    let _ = write_flip_pixel(&mut file, flip);
    let _ = file.flush();
    let pool = shm.create_pool(file.as_fd(), bytes as i32, qh, ());
    let buffer = pool.create_buffer(0, buf_w, buf_h, stride, wl_shm::Format::Argb8888, qh, ());
    surface.set_buffer_scale(scale);
    surface.attach(Some(&buffer), 0, 0);
    surface.damage_buffer(0, 0, buf_w, buf_h);
    surface.commit();
    Some((buffer, pool, file))
}

fn write_flip_pixel(file: &mut File, flip: u8) -> std::io::Result<()> {
    use std::io::{Seek, SeekFrom};
    // ARGB8888 little-endian: A in high byte. Keep A=0 so the pixel stays transparent.
    let px = [flip, 0u8, 0u8, 0u8];
    file.seek(SeekFrom::Start(0))?;
    file.write_all(&px)?;
    file.flush()?;
    Ok(())
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
                state.shm =
                    Some(registry.bind::<wl_shm::WlShm, _, _>(name, version.min(1), qh, ()));
            }
            "wl_output" => {
                let output =
                    registry.bind::<wl_output::WlOutput, _, _>(name, version.min(4), qh, name);
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
            i if i == xdg_wm_base::XdgWmBase::interface().name && state.xdg_wm_base.is_none() => {
                state.xdg_wm_base = Some(registry.bind::<xdg_wm_base::XdgWmBase, _, _>(
                    name,
                    version.min(6),
                    qh,
                    (),
                ));
            }
            i if i == zxdg_decoration_manager_v1::ZxdgDecorationManagerV1::interface().name
                && state.decoration_manager.is_none() =>
            {
                state.decoration_manager = Some(
                    registry.bind::<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, _, _>(
                        name,
                        version.min(1),
                        qh,
                        (),
                    ),
                );
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
        wm: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            wm.pong(serial);
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
            state.xdg_configured = true;
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for KickState {
    fn event(
        _: &mut Self,
        _: &xdg_toplevel::XdgToplevel,
        _: xdg_toplevel::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, ()> for KickState {
    fn event(
        _: &mut Self,
        _: &zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
        _: zxdg_decoration_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, ()> for KickState {
    fn event(
        _: &mut Self,
        _: &zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
        _: zxdg_toplevel_decoration_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
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
