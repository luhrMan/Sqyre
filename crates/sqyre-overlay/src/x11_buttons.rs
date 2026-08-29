//! Native X11 overlay buttons on a dedicated event thread.
//!
//! egui deferred/immediate viewports starve under fullscreen XWayland games on
//! GNOME (ROOT re-registration ~1 Hz). This path owns an override-redirect
//! window per button and reads ButtonPress/Release on its own `Display`, so
//! clicks do not wait for the egui ROOT frame.

use crate::raster::{self, ButtonPaint, TIP_BG_RGB, TIP_CORNER_PX};
use egui::Context as EguiContext;
use parking_lot::Mutex;
use sqyre_capture::{
    mark_site, note, register_secondary_x_display, unregister_secondary_x_display,
    OVERLAY_TIP_WM_TITLE, OVERLAY_WM_TITLE,
};
use std::collections::HashMap;
use std::os::fd::RawFd;
use std::os::raw::{c_char, c_int, c_long, c_uint, c_ulong};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use x11::xfixes::{
    XFixesCreateRegion, XFixesDestroyRegion, XFixesQueryExtension, XFixesQueryVersion,
    XFixesSetWindowShapeRegion,
};
use x11::xlib::{
    ButtonPress, ButtonPressMask, ButtonRelease, ButtonReleaseMask, CWBackPixel, CWBackingStore,
    CWBorderPixel, CWEventMask, CWOverrideRedirect, Display, EnterNotify, EnterWindowMask, Expose,
    ExposureMask, False, InputOutput, LeaveNotify, LeaveWindowMask, StructureNotifyMask, True,
    WhenMapped, Window, XAllocColor, XCloseDisplay, XColor, XConnectionNumber, XCreateGC,
    XCreateImage, XCreateWindow, XDefaultColormap, XDefaultDepth, XDefaultRootWindow,
    XDefaultScreen, XDefaultVisual, XDestroyImage, XDestroyWindow, XEvent, XFlush, XFreeGC,
    XInternAtom, XMapRaised, XMoveResizeWindow, XNextEvent, XOpenDisplay, XPending, XPutImage,
    XRectangle, XSelectInput, XSetWindowAttributes, XStoreName, XSync, XUnmapWindow, ZPixmap,
    LSBFirst,
};

/// `ShapeBounding` / `ShapeInput` from `X11/extensions/shapeconst.h`.
const SHAPE_BOUNDING: c_int = 0;
const SHAPE_INPUT: c_int = 2;

const POLL_IDLE_MS: u64 = 16;
const BUSY_TICK_MS: u64 = 50;
const TIP_GAP_PX: i32 = 6;

/// Desired on-screen button (physical pixels, root coordinates).
#[derive(Debug, Clone, PartialEq)]
pub struct NativeButtonSpec {
    pub id: String,
    pub macro_name: String,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub bg: [u8; 3],
    pub border: [u8; 4],
    pub border_width: f32,
    pub corner_radius: f32,
    pub icon_glyph: char,
    pub icon: [u8; 4],
    pub icon_hover: [u8; 4],
    /// Hover tip text (label / display name). Empty = no tip.
    pub tip: String,
    pub busy: bool,
}

enum HostCmd {
    SetButtons(Vec<NativeButtonSpec>),
    /// egui context used to wake ROOT immediately after a click (busy UI / drain).
    SetWake(EguiContext),
    Shutdown,
}

/// Owns the X11 thread. Clicks enqueue into `pending` immediately (do not wait for ROOT).
pub struct X11ButtonHost {
    cmd_tx: Sender<HostCmd>,
    stop: Arc<AtomicBool>,
    join: Mutex<Option<JoinHandle<()>>>,
}

impl X11ButtonHost {
    pub fn start(pending: Arc<Mutex<Vec<String>>>) -> Result<Self, String> {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_t = Arc::clone(&stop);
        let join = thread::Builder::new()
            .name("sqyre-x11-overlay".into())
            .spawn(move || host_loop(cmd_rx, pending, stop_t))
            .map_err(|e| format!("spawn x11 overlay thread: {e}"))?;
        Ok(Self {
            cmd_tx,
            stop,
            join: Mutex::new(Some(join)),
        })
    }

    pub fn set_buttons(&self, buttons: Vec<NativeButtonSpec>) {
        let _ = self.cmd_tx.send(HostCmd::SetButtons(buttons));
    }

    pub fn set_wake(&self, ctx: EguiContext) {
        let _ = self.cmd_tx.send(HostCmd::SetWake(ctx));
    }

    pub fn shutdown(&self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.cmd_tx.send(HostCmd::Shutdown);
        if let Some(join) = self.join.lock().take() {
            let _ = join.join();
        }
    }
}

impl Drop for X11ButtonHost {
    fn drop(&mut self) {
        self.shutdown();
    }
}

struct LiveButton {
    spec: NativeButtonSpec,
    win: Window,
    armed: bool,
    hovered: bool,
    busy_phase: f32,
}

struct TipWindow {
    win: Window,
    for_id: String,
    text: String,
    w: u32,
    h: u32,
    mapped: bool,
}

struct HostState {
    display: *mut Display,
    screen: c_int,
    root: Window,
    gc: *mut x11::xlib::_XGC,
    buttons: HashMap<String, LiveButton>,
    tip: Option<TipWindow>,
    xfd: RawFd,
    pending: Arc<Mutex<Vec<String>>>,
    wake: Option<EguiContext>,
}

fn host_loop(
    cmd_rx: Receiver<HostCmd>,
    pending: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
) {
    // SAFETY: own Display for this thread only.
    let display = unsafe { XOpenDisplay(std::ptr::null()) };
    if display.is_null() {
        note("overlay-x11: XOpenDisplay failed");
        return;
    }
    register_secondary_x_display(display.cast());

    let screen = unsafe { XDefaultScreen(display) };
    let root = unsafe { XDefaultRootWindow(display) };
    let gc = unsafe { XCreateGC(display, root, 0, std::ptr::null_mut()) };
    if gc.is_null() {
        note("overlay-x11: XCreateGC failed");
        unregister_secondary_x_display(display.cast());
        unsafe { XCloseDisplay(display) };
        return;
    }
    let xfd = unsafe { XConnectionNumber(display) };

    let mut state = HostState {
        display,
        screen,
        root,
        gc,
        buttons: HashMap::new(),
        tip: None,
        xfd,
        pending,
        wake: None,
    };

    note("overlay-x11: host thread started (override-redirect buttons)");
    let mut last_busy_tick = Instant::now();

    while !stop.load(Ordering::Relaxed) {
        loop {
            match cmd_rx.try_recv() {
                Ok(HostCmd::SetButtons(specs)) => apply_specs(&mut state, specs),
                Ok(HostCmd::SetWake(ctx)) => {
                    state.wake = Some(ctx);
                }
                Ok(HostCmd::Shutdown) => {
                    stop.store(true, Ordering::Relaxed);
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    stop.store(true, Ordering::Relaxed);
                    break;
                }
            }
        }
        if stop.load(Ordering::Relaxed) {
            break;
        }

        drain_x_events(&mut state);

        let any_busy = state.buttons.values().any(|b| b.spec.busy);
        if any_busy && last_busy_tick.elapsed() >= Duration::from_millis(BUSY_TICK_MS) {
            last_busy_tick = Instant::now();
            let busy_ids: Vec<String> = state
                .buttons
                .iter()
                .filter(|(_, b)| b.spec.busy)
                .map(|(id, _)| id.clone())
                .collect();
            for id in busy_ids {
                if let Some(btn) = state.buttons.get_mut(&id) {
                    btn.busy_phase = (btn.busy_phase + 0.12) % std::f32::consts::TAU;
                }
                if let Some(btn) = state.buttons.get(&id) {
                    paint_button(&state, btn);
                }
            }
            unsafe { XFlush(state.display) };
        }

        wait_x_or_timeout(state.xfd, if any_busy { BUSY_TICK_MS } else { POLL_IDLE_MS });
    }

    destroy_all(&mut state);
    unsafe {
        XFreeGC(state.display, state.gc);
    }
    unregister_secondary_x_display(state.display.cast());
    unsafe {
        XCloseDisplay(state.display);
    }
    note("overlay-x11: host thread stopped");
}

fn apply_specs(state: &mut HostState, specs: Vec<NativeButtonSpec>) {
    let mut keep = std::collections::HashSet::new();
    for spec in specs {
        let id = spec.id.clone();
        keep.insert(id.clone());
        if state.buttons.contains_key(&id) {
            let geom_changed = {
                let live = state.buttons.get(&id).unwrap();
                live.spec.x != spec.x
                    || live.spec.y != spec.y
                    || live.spec.w != spec.w
                    || live.spec.h != spec.h
            };
            let radius_changed = {
                let live = state.buttons.get(&id).unwrap();
                (live.spec.corner_radius - spec.corner_radius).abs() > f32::EPSILON
            };
            let style_changed = {
                let live = state.buttons.get(&id).unwrap();
                live.spec.bg != spec.bg
                    || live.spec.border != spec.border
                    || (live.spec.border_width - spec.border_width).abs() > f32::EPSILON
                    || radius_changed
                    || live.spec.icon_glyph != spec.icon_glyph
                    || live.spec.icon != spec.icon
                    || live.spec.icon_hover != spec.icon_hover
                    || live.spec.tip != spec.tip
                    || live.spec.busy != spec.busy
            };
            let win = state.buttons.get(&id).unwrap().win;
            if geom_changed {
                unsafe {
                    XMoveResizeWindow(state.display, win, spec.x, spec.y, spec.w, spec.h);
                }
            }
            if let Some(live) = state.buttons.get_mut(&id) {
                live.spec = spec;
            }
            if geom_changed || radius_changed {
                if let Some(live) = state.buttons.get(&id) {
                    apply_rounded_shape(
                        state.display,
                        live.win,
                        live.spec.w,
                        live.spec.h,
                        live.spec.corner_radius,
                    );
                }
            }
            if style_changed || geom_changed {
                if let Some(live) = state.buttons.get(&id) {
                    paint_button(state, live);
                }
                if state
                    .tip
                    .as_ref()
                    .is_some_and(|t| t.mapped && t.for_id == id)
                {
                    show_tip_for(state, &id);
                }
            }
        } else if let Ok(live) = create_button(state, spec) {
            apply_rounded_shape(
                state.display,
                live.win,
                live.spec.w,
                live.spec.h,
                live.spec.corner_radius,
            );
            paint_button(state, &live);
            state.buttons.insert(live.spec.id.clone(), live);
        }
    }
    let drop_ids: Vec<String> = state
        .buttons
        .keys()
        .filter(|id| !keep.contains(*id))
        .cloned()
        .collect();
    for id in drop_ids {
        if let Some(live) = state.buttons.remove(&id) {
            if state.tip.as_ref().is_some_and(|t| t.for_id == id) {
                hide_tip(state);
            }
            unsafe {
                XDestroyWindow(state.display, live.win);
            }
        }
    }
    unsafe {
        XFlush(state.display);
    }
}

fn create_button(state: &HostState, spec: NativeButtonSpec) -> Result<LiveButton, String> {
    let bg_pixel = alloc_color(state, spec.bg);
    // SAFETY: state.display is live for this thread.
    let win = unsafe {
        let mut attrs: XSetWindowAttributes = std::mem::zeroed();
        attrs.background_pixel = bg_pixel;
        attrs.border_pixel = bg_pixel;
        attrs.override_redirect = True;
        attrs.backing_store = WhenMapped;
        attrs.event_mask = ButtonPressMask
            | ButtonReleaseMask
            | ExposureMask
            | StructureNotifyMask
            | EnterWindowMask
            | LeaveWindowMask;
        let win = XCreateWindow(
            state.display,
            state.root,
            spec.x,
            spec.y,
            spec.w.max(1),
            spec.h.max(1),
            0,
            XDefaultDepth(state.display, state.screen),
            InputOutput as c_uint,
            XDefaultVisual(state.display, state.screen),
            CWBackPixel | CWBorderPixel | CWOverrideRedirect | CWBackingStore | CWEventMask,
            &mut attrs,
        );
        if win == 0 {
            return Err("XCreateWindow failed".into());
        }
        let c_title = std::ffi::CString::new(OVERLAY_WM_TITLE).unwrap_or_default();
        XStoreName(state.display, win, c_title.as_ptr());
        set_net_wm_type_notification(state.display, win);
        set_skip_taskbar_state(state.display, state.root, win);
        XSelectInput(
            state.display,
            win,
            ButtonPressMask
                | ButtonReleaseMask
                | ExposureMask
                | EnterWindowMask
                | LeaveWindowMask,
        );
        XMapRaised(state.display, win);
        XFlush(state.display);
        win
    };
    mark_site(&format!("overlay-x11:map:{}", spec.id));
    note(&format!(
        "overlay-x11: mapped id={} {}x{}+{}+{}",
        spec.id, spec.w, spec.h, spec.x, spec.y
    ));
    Ok(LiveButton {
        spec,
        win,
        armed: false,
        hovered: false,
        busy_phase: 0.0,
    })
}

fn paint_button(state: &HostState, btn: &LiveButton) {
    let paint = ButtonPaint {
        w: btn.spec.w.max(1),
        h: btn.spec.h.max(1),
        bg: btn.spec.bg,
        border: btn.spec.border,
        border_width: btn.spec.border_width,
        corner_radius: btn.spec.corner_radius,
        icon_glyph: btn.spec.icon_glyph,
        icon: btn.spec.icon,
        icon_hover: btn.spec.icon_hover,
        hovered: btn.hovered,
        busy: btn.spec.busy,
        busy_phase: btn.busy_phase,
    };
    let rgba = raster::rasterize(&paint);
    blit_rgba(state, btn.win, paint.w, paint.h, &rgba);
}

fn show_tip_for(state: &mut HostState, button_id: &str) {
    let Some(btn) = state.buttons.get(button_id) else {
        return;
    };
    let tip_text = btn.spec.tip.trim().to_string();
    if tip_text.is_empty() {
        hide_tip(state);
        return;
    }
    let btn_x = btn.spec.x;
    let btn_y = btn.spec.y;
    let btn_w = btn.spec.w as i32;
    let btn_h = btn.spec.h as i32;
    let (tw, th, rgba) = raster::rasterize_tip(&tip_text);
    if tw == 0 || th == 0 || rgba.is_empty() {
        hide_tip(state);
        return;
    }
    // Center under the button with a gap (tip has empty ShapeInput — no steal/flicker).
    let tip_x = btn_x + (btn_w - tw as i32) / 2;
    let tip_y = btn_y + btn_h + TIP_GAP_PX;

    let need_new = state
        .tip
        .as_ref()
        .map(|t| t.w != tw || t.h != th)
        .unwrap_or(true);
    if need_new {
        if let Some(old) = state.tip.take() {
            unsafe {
                XDestroyWindow(state.display, old.win);
            }
        }
        match create_tip_window(state, tip_x, tip_y, tw, th) {
            Ok(win) => {
                state.tip = Some(TipWindow {
                    win,
                    for_id: button_id.to_string(),
                    text: tip_text.clone(),
                    w: tw,
                    h: th,
                    mapped: true,
                });
            }
            Err(e) => {
                note(&format!("overlay-x11: tip create failed: {e}"));
                return;
            }
        }
    } else if let Some(tip) = state.tip.as_mut() {
        tip.for_id = button_id.to_string();
        tip.text = tip_text.clone();
        unsafe {
            XMoveResizeWindow(state.display, tip.win, tip_x, tip_y, tw, th);
            XMapRaised(state.display, tip.win);
            tip.mapped = true;
        }
    }
    if let Some(tip) = state.tip.as_ref() {
        apply_tip_shape(state.display, tip.win, tip.w, tip.h);
        blit_rgba(state, tip.win, tip.w, tip.h, &rgba);
        unsafe {
            XFlush(state.display);
        }
    }
}

fn hide_tip(state: &mut HostState) {
    if let Some(tip) = state.tip.as_mut() {
        if tip.mapped {
            unsafe {
                XUnmapWindow(state.display, tip.win);
                XFlush(state.display);
            }
            tip.mapped = false;
        }
        tip.for_id.clear();
        tip.text.clear();
    }
}

fn create_tip_window(
    state: &HostState,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
) -> Result<Window, String> {
    let bg_pixel = alloc_color(state, TIP_BG_RGB);
    unsafe {
        let mut attrs: XSetWindowAttributes = std::mem::zeroed();
        attrs.background_pixel = bg_pixel;
        attrs.border_pixel = bg_pixel;
        attrs.override_redirect = True;
        attrs.backing_store = WhenMapped;
        // No pointer events — tip is visual-only (empty ShapeInput below).
        attrs.event_mask = ExposureMask;
        let win = XCreateWindow(
            state.display,
            state.root,
            x,
            y,
            w.max(1),
            h.max(1),
            0,
            XDefaultDepth(state.display, state.screen),
            InputOutput as c_uint,
            XDefaultVisual(state.display, state.screen),
            CWBackPixel | CWBorderPixel | CWOverrideRedirect | CWBackingStore | CWEventMask,
            &mut attrs,
        );
        if win == 0 {
            return Err("XCreateWindow tip failed".into());
        }
        let c_title = std::ffi::CString::new(OVERLAY_TIP_WM_TITLE).unwrap_or_default();
        XStoreName(state.display, win, c_title.as_ptr());
        set_net_wm_type_notification(state.display, win);
        set_skip_taskbar_state(state.display, state.root, win);
        apply_tip_shape(state.display, win, w, h);
        XMapRaised(state.display, win);
        XFlush(state.display);
        Ok(win)
    }
}

/// Rounded bounding + empty input so the tip never steals hover from the button.
fn apply_tip_shape(display: *mut Display, win: Window, w: u32, h: u32) {
    apply_rounded_shape(display, win, w, h, TIP_CORNER_PX);
    unsafe {
        let mut event_base = 0;
        let mut error_base = 0;
        if XFixesQueryExtension(display, &mut event_base, &mut error_base) == 0 {
            return;
        }
        let empty = XFixesCreateRegion(display, std::ptr::null_mut(), 0);
        if empty == 0 {
            return;
        }
        XFixesSetWindowShapeRegion(display, win, SHAPE_INPUT, 0, 0, empty);
        XFixesDestroyRegion(display, empty);
    }
}

fn blit_rgba(state: &HostState, win: Window, w: u32, h: u32, rgba: &[u8]) {
    unsafe {
        let visual = XDefaultVisual(state.display, state.screen);
        let depth = XDefaultDepth(state.display, state.screen);
        let (red_mask, green_mask, blue_mask) = if visual.is_null() {
            (0x00FF_0000u64, 0x0000_FF00, 0x0000_00FF)
        } else {
            (
                (*visual).red_mask,
                (*visual).green_mask,
                (*visual).blue_mask,
            )
        };
        let mut packed = pack_truecolor(rgba, w, h, red_mask, green_mask, blue_mask);
        let ximage = XCreateImage(
            state.display,
            visual,
            depth as c_uint,
            ZPixmap,
            0,
            packed.as_mut_ptr().cast::<c_char>(),
            w,
            h,
            32,
            (w * 4) as c_int,
        );
        if ximage.is_null() {
            return;
        }
        (*ximage).byte_order = LSBFirst;
        (*ximage).bits_per_pixel = 32;
        XPutImage(
            state.display,
            win,
            state.gc,
            ximage,
            0,
            0,
            0,
            0,
            w,
            h,
        );
        (*ximage).data = std::ptr::null_mut();
        XDestroyImage(ximage);
    }
}

fn pack_truecolor(
    rgba: &[u8],
    w: u32,
    h: u32,
    red_mask: u64,
    green_mask: u64,
    blue_mask: u64,
) -> Vec<u8> {
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for px in rgba.chunks_exact(4) {
        let packed = place_channel(px[0], red_mask)
            | place_channel(px[1], green_mask)
            | place_channel(px[2], blue_mask);
        out.extend_from_slice(&packed.to_le_bytes());
    }
    out
}

fn place_channel(value: u8, mask: u64) -> u32 {
    if mask == 0 {
        return 0;
    }
    let shift = mask.trailing_zeros();
    (u32::from(value) << shift) & mask as u32
}

fn drain_x_events(state: &mut HostState) {
    loop {
        let pending = unsafe { XPending(state.display) };
        if pending <= 0 {
            break;
        }
        let mut event: XEvent = unsafe { std::mem::zeroed() };
        unsafe {
            XNextEvent(state.display, &mut event);
        }
        let ty = event.get_type();
        if ty == Expose {
            let win = unsafe { event.expose.window };
            if state.tip.as_ref().is_some_and(|t| t.win == win && t.mapped) {
                if let Some(tip) = state.tip.as_ref() {
                    let (tw, th, rgba) = raster::rasterize_tip(&tip.text);
                    if tw > 0 && th > 0 && !rgba.is_empty() {
                        blit_rgba(state, tip.win, tw, th, &rgba);
                    }
                }
                continue;
            }
            let id = state
                .buttons
                .values()
                .find(|b| b.win == win)
                .map(|b| b.spec.id.clone());
            if let Some(id) = id {
                if let Some(b) = state.buttons.get(&id) {
                    paint_button(state, b);
                }
            }
            continue;
        }
        if ty == EnterNotify || ty == LeaveNotify {
            let win = unsafe { event.crossing.window };
            let enter = ty == EnterNotify;
            let id = state
                .buttons
                .values()
                .find(|b| b.win == win)
                .map(|b| b.spec.id.clone());
            if let Some(id) = id {
                let mut need_paint = false;
                let mut tip_action: Option<(bool, String)> = None;
                if let Some(b) = state.buttons.get_mut(&id) {
                    if b.hovered != enter {
                        b.hovered = enter;
                        need_paint = true;
                        tip_action = Some((enter, id.clone()));
                    }
                }
                if need_paint {
                    if let Some(b) = state.buttons.get(&id) {
                        paint_button(state, b);
                    }
                }
                if let Some((show, tip_id)) = tip_action {
                    if show {
                        show_tip_for(state, &tip_id);
                    } else if state.tip.as_ref().is_some_and(|t| t.for_id == tip_id) {
                        hide_tip(state);
                    }
                }
            }
            continue;
        }
        if ty == ButtonPress {
            let win = unsafe {
                let b = &*( &event as *const XEvent as *const x11::xlib::XButtonEvent);
                b.window
            };
            if let Some(btn) = state.buttons.values_mut().find(|b| b.win == win) {
                if !btn.spec.busy {
                    btn.armed = true;
                }
            }
            continue;
        }
        if ty == ButtonRelease {
            let win = unsafe {
                let b = &*( &event as *const XEvent as *const x11::xlib::XButtonEvent);
                b.window
            };
            if let Some(btn) = state.buttons.values_mut().find(|b| b.win == win) {
                if btn.armed && !btn.spec.busy {
                    btn.armed = false;
                    let id = btn.spec.id.clone();
                    let macro_name = btn.spec.macro_name.clone();
                    mark_site(&format!("overlay-x11:click:{id}"));
                    note(&format!("overlay-x11: click id={id} macro={macro_name}"));
                    // Enqueue immediately — do not wait for the next egui ROOT sync
                    // (that was ~1s under GameThread while the spinner already ran fine).
                    state.pending.lock().push(macro_name);
                    if let Some(ctx) = &state.wake {
                        ctx.request_repaint();
                    }
                } else {
                    btn.armed = false;
                }
            }
        }
    }
    unsafe {
        XFlush(state.display);
    }
}

fn destroy_all(state: &mut HostState) {
    hide_tip(state);
    if let Some(tip) = state.tip.take() {
        unsafe {
            XDestroyWindow(state.display, tip.win);
        }
    }
    for (_, live) in state.buttons.drain() {
        unsafe {
            XDestroyWindow(state.display, live.win);
        }
    }
    unsafe {
        XSync(state.display, False);
    }
}

fn alloc_color(state: &HostState, rgb: [u8; 3]) -> c_ulong {
    unsafe {
        let cmap = XDefaultColormap(state.display, state.screen);
        let mut color = XColor {
            pixel: 0,
            red: (rgb[0] as u16) << 8,
            green: (rgb[1] as u16) << 8,
            blue: (rgb[2] as u16) << 8,
            flags: 0,
            pad: 0,
        };
        if XAllocColor(state.display, cmap, &mut color) != 0 {
            color.pixel
        } else {
            // Fallback: pack assuming 24-bit TrueColor common on XWayland.
            ((rgb[0] as c_ulong) << 16) | ((rgb[1] as c_ulong) << 8) | (rgb[2] as c_ulong)
        }
    }
}

unsafe fn set_net_wm_type_notification(display: *mut Display, win: Window) {
    let ty = XInternAtom(
        display,
        b"_NET_WM_WINDOW_TYPE\0".as_ptr().cast::<c_char>(),
        False,
    );
    let notification = XInternAtom(
        display,
        b"_NET_WM_WINDOW_TYPE_NOTIFICATION\0"
            .as_ptr()
            .cast::<c_char>(),
        False,
    );
    if ty == 0 || notification == 0 {
        return;
    }
    let mut val: c_ulong = notification as c_ulong;
    x11::xlib::XChangeProperty(
        display,
        win,
        ty,
        x11::xlib::XA_ATOM,
        32,
        x11::xlib::PropModeReplace,
        (&mut val as *mut c_ulong).cast::<u8>(),
        1,
    );
}

unsafe fn set_skip_taskbar_state(display: *mut Display, root: Window, win: Window) {
    let state = XInternAtom(
        display,
        b"_NET_WM_STATE\0".as_ptr().cast::<c_char>(),
        False,
    );
    let skip_taskbar = XInternAtom(
        display,
        b"_NET_WM_STATE_SKIP_TASKBAR\0".as_ptr().cast::<c_char>(),
        False,
    );
    let skip_pager = XInternAtom(
        display,
        b"_NET_WM_STATE_SKIP_PAGER\0".as_ptr().cast::<c_char>(),
        False,
    );
    let above = XInternAtom(
        display,
        b"_NET_WM_STATE_ABOVE\0".as_ptr().cast::<c_char>(),
        False,
    );
    if state == 0 || skip_taskbar == 0 {
        return;
    }
    let mut atoms: [c_ulong; 3] = [
        skip_taskbar as c_ulong,
        skip_pager as c_ulong,
        above as c_ulong,
    ];
    x11::xlib::XChangeProperty(
        display,
        win,
        state,
        x11::xlib::XA_ATOM,
        32,
        x11::xlib::PropModeReplace,
        atoms.as_mut_ptr().cast::<u8>(),
        3,
    );
    // Also ClientMessage for WMs that ignore property until mapped.
    let mut ev: XEvent = std::mem::zeroed();
    {
        let c = &mut ev.client_message;
        c.type_ = x11::xlib::ClientMessage;
        c.window = win;
        c.message_type = state;
        c.format = 32;
        c.data.set_long(0, 1); // _NET_WM_STATE_ADD
        c.data.set_long(1, skip_taskbar as c_long);
        c.data.set_long(2, above as c_long);
        c.data.set_long(3, 1);
    }
    x11::xlib::XSendEvent(
        display,
        root,
        False,
        (x11::xlib::SubstructureNotifyMask | x11::xlib::SubstructureRedirectMask) as c_long,
        &mut ev,
    );
}

fn wait_x_or_timeout(xfd: RawFd, timeout_ms: u64) {
    let mut fds = libc::pollfd {
        fd: xfd,
        events: libc::POLLIN,
        revents: 0,
    };
    unsafe {
        libc::poll(&mut fds, 1, timeout_ms as c_int);
    }
}

/// Opaque window + XFixes rounded Bounding/Input — corners look punched out without ARGB lag.
fn apply_rounded_shape(display: *mut Display, win: Window, w: u32, h: u32, radius_px: f32) {
    unsafe {
        let mut event_base = 0;
        let mut error_base = 0;
        if XFixesQueryExtension(display, &mut event_base, &mut error_base) == 0 {
            return;
        }
        let mut major: c_int = 4;
        let mut minor: c_int = 0;
        if XFixesQueryVersion(
            display,
            &mut major,
            &mut minor as *mut c_int as *const c_int,
        ) == 0
        {
            return;
        }
        if w == 0 || h == 0 || w > u32::from(u16::MAX) || h > u32::from(u16::MAX) {
            return;
        }
        let radius = radius_px.round().max(0.0) as i32;
        let mut rects = rounded_rect_xrectangles(w as i32, h as i32, radius);
        if rects.is_empty() {
            return;
        }
        let region = XFixesCreateRegion(display, rects.as_mut_ptr(), rects.len() as c_int);
        if region == 0 {
            return;
        }
        XFixesSetWindowShapeRegion(display, win, SHAPE_BOUNDING, 0, 0, region);
        XFixesSetWindowShapeRegion(display, win, SHAPE_INPUT, 0, 0, region);
        XFixesDestroyRegion(display, region);
    }
}

fn rounded_rect_xrectangles(w: i32, h: i32, radius: i32) -> Vec<XRectangle> {
    if w <= 0 || h <= 0 {
        return Vec::new();
    }
    let r = radius.max(0).min(w / 2).min(h / 2);
    let mut out = Vec::with_capacity(h as usize);
    for y in 0..h {
        let inset = if r == 0 {
            0
        } else if y < r {
            circle_inset(r, r - 1 - y)
        } else if y >= h - r {
            circle_inset(r, y - (h - r))
        } else {
            0
        };
        let width = (w - 2 * inset).max(0);
        if width > 0 {
            out.push(XRectangle {
                x: inset as i16,
                y: y as i16,
                width: width as u16,
                height: 1,
            });
        }
    }
    out
}

fn circle_inset(r: i32, y_from_edge: i32) -> i32 {
    let r = r as f64;
    let y = y_from_edge.clamp(0, r as i32) as f64;
    let inside = (r * r - y * y).max(0.0).sqrt();
    (r - inside).ceil().max(0.0) as i32
}
