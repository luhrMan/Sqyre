//! Native X11 overlay buttons on a dedicated event thread.
//!
//! egui deferred/immediate viewports starve under fullscreen XWayland games on
//! GNOME (ROOT re-registration ~1 Hz). This path owns an override-redirect
//! window per button and reads ButtonPress/Release on its own `Display`, so
//! clicks do not wait for the egui ROOT frame.
//!
//! # X11 safety (`HostState`)
//!
//! All Xlib/`xfixes` use on the host thread assumes:
//! - `display` is non-null, opened with `XOpenDisplay` on this thread only
//! - it is registered via [`register_secondary_x_display`] for its lifetime
//! - `gc` / `root` / `xfd` come from that display and stay valid until close
//! - every `Window` in `buttons` / `tip` was created on that display and is
//!   destroyed before `unregister_secondary_x_display` + `XCloseDisplay`
//!
//! Thin `x_*` helpers carry a single `SAFETY` contract; larger blocks document
//! the same invariant at the call boundary (see `x11_focus` for the same style).

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
    CWBorderPixel, CWEventMask, CWOverrideRedirect, CurrentTime, Display, EnterNotify,
    EnterWindowMask, Expose, ExposureMask, False, GrabModeAsync, InputOnly, InputOutput,
    LeaveNotify, LeaveWindowMask, MotionNotify, PointerMotionMask, StructureNotifyMask, Success,
    True, WhenMapped, Window, XAllocColor, XCloseDisplay, XColor, XConnectionNumber,
    XCreateFontCursor, XCreateGC, XCreateImage, XCreateWindow, XDefaultColormap, XDefaultDepth,
    XDefaultRootWindow, XDefaultScreen, XDefaultVisual, XDefineCursor, XDestroyImage,
    XDestroyWindow, XEvent, XFlush, XFreeCursor, XFreeGC, XGrabPointer, XInternAtom, XMapRaised,
    XMapWindow, XMoveResizeWindow, XNextEvent, XOpenDisplay, XPending, XPutImage, XRectangle,
    XSelectInput, XSetWindowAttributes, XStoreName, XSync, XUndefineCursor, XUngrabPointer,
    XUnmapWindow, XConfigureWindow, XWindowChanges, ZPixmap, LSBFirst, Below, CWSibling,
    CWStackMode,
};

/// `ShapeBounding` / `ShapeInput` from `X11/extensions/shapeconst.h`.
const SHAPE_BOUNDING: c_int = 0;
const SHAPE_INPUT: c_int = 2;

const POLL_IDLE_MS: u64 = 16;
const BUSY_TICK_MS: u64 = 50;
const TIP_GAP_PX: i32 = 6;
/// X11 cursorfont fleur (four-way move arrow).
const XC_FLEUR: c_uint = 52;
const BUTTON_LEFT: c_uint = 1;

/// Desired on-screen button (physical pixels, root coordinates).
#[derive(Debug, Clone, PartialEq)]
pub struct NativeButtonSpec {
    pub id: String,
    pub macro_name: String,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    /// Fill RGBA. Alpha 0 = no fill (Shape punch-out from chrome ink only).
    pub bg: [u8; 4],
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

/// Desktop position committed after a relocate-mode drag (root coordinates).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayButtonMove {
    pub id: String,
    pub x: i32,
    pub y: i32,
}

enum HostCmd {
    SetButtons(Vec<NativeButtonSpec>),
    /// When true, left-drag relocates buttons instead of enqueueing macros.
    SetRelocateMode(bool),
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
    pub fn start(
        pending: Arc<Mutex<Vec<String>>>,
        pending_moves: Arc<Mutex<Vec<OverlayButtonMove>>>,
        running_macro: Arc<Mutex<Option<String>>>,
    ) -> Result<Self, String> {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_t = Arc::clone(&stop);
        let join = thread::Builder::new()
            .name("sqyre-x11-overlay".into())
            .spawn(move || host_loop(cmd_rx, pending, pending_moves, running_macro, stop_t))
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

    pub fn set_relocate_mode(&self, enabled: bool) {
        let _ = self.cmd_tx.send(HostCmd::SetRelocateMode(enabled));
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
    /// Painted face (`InputOutput`). Empty ShapeInput — clicks go to [`Self::hit`].
    win: Window,
    /// Invisible full hit target (`InputOnly`). Owns pointer events for the button disk.
    hit: Window,
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

struct DragState {
    id: String,
    /// Pointer offset from button top-left in root coordinates.
    grab_dx: i32,
    grab_dy: i32,
    start_x: i32,
    start_y: i32,
}

/// X11 resources owned exclusively by the overlay host thread.
///
/// See module-level **X11 safety** for the invariants every `unsafe` call relies on.
struct HostState {
    display: *mut Display,
    screen: c_int,
    root: Window,
    gc: *mut x11::xlib::_XGC,
    buttons: HashMap<String, LiveButton>,
    tip: Option<TipWindow>,
    xfd: RawFd,
    pending: Arc<Mutex<Vec<String>>>,
    pending_moves: Arc<Mutex<Vec<OverlayButtonMove>>>,
    wake: Option<EguiContext>,
    relocate_mode: bool,
    drag: Option<DragState>,
    move_cursor: c_ulong,
    /// Skip Enter/Leave tip logic while mapping/stacking tip (avoids leave↔show loops).
    suppress_crossing: bool,
}

fn host_loop(
    cmd_rx: Receiver<HostCmd>,
    pending: Arc<Mutex<Vec<String>>>,
    pending_moves: Arc<Mutex<Vec<OverlayButtonMove>>>,
    running_macro: Arc<Mutex<Option<String>>>,
    stop: Arc<AtomicBool>,
) {
    // SAFETY: connects to the default display; pointer is owned by this thread
    // until unregister + x_close_display below (or the early-error path).
    let display = unsafe { XOpenDisplay(std::ptr::null()) };
    if display.is_null() {
        note("overlay-x11: XOpenDisplay failed");
        return;
    }
    register_secondary_x_display(display.cast());

    // SAFETY: `display` just opened and non-null.
    let screen = unsafe { XDefaultScreen(display) };
    let root = unsafe { XDefaultRootWindow(display) };
    let gc = unsafe { XCreateGC(display, root, 0, std::ptr::null_mut()) };
    if gc.is_null() {
        note("overlay-x11: XCreateGC failed");
        unregister_secondary_x_display(display.cast());
        // SAFETY: display still live; nothing else holds it.
        unsafe { x_close_display(display) };
        return;
    }
    // SAFETY: `display` non-null; fd is valid for poll until XCloseDisplay.
    let xfd = unsafe { XConnectionNumber(display) };
    // SAFETY: host-thread display; cursor freed in destroy_all before close.
    let move_cursor = unsafe { XCreateFontCursor(display, XC_FLEUR) };

    let mut state = HostState {
        display,
        screen,
        root,
        gc,
        buttons: HashMap::new(),
        tip: None,
        xfd,
        pending,
        pending_moves,
        wake: None,
        relocate_mode: false,
        drag: None,
        move_cursor,
        suppress_crossing: false,
    };

    note("overlay-x11: host thread started (override-redirect buttons)");
    let mut last_busy_tick = Instant::now();
    let mut last_x_growth = Instant::now();
    let mut stall_raised = false;
    let mut last_running: Option<String> = None;

    while !stop.load(Ordering::Relaxed) {
        loop {
            match cmd_rx.try_recv() {
                Ok(HostCmd::SetButtons(specs)) => {
                    apply_specs(&mut state, specs);
                    apply_running_busy(&mut state, last_running.as_deref());
                }
                Ok(HostCmd::SetRelocateMode(enabled)) => set_relocate_mode(&mut state, enabled),
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

        // Busy must not wait on egui sync — ROOT starves under fullscreen games.
        let running_now = running_macro.lock().clone();
        if running_now != last_running {
            apply_running_busy(&mut state, running_now.as_deref());
            last_running = running_now;
            stall_raised = false;
            last_x_growth = Instant::now();
        }

        let before_pending = unsafe { XPending(state.display) };
        let _got_pointer = drain_x_events(&mut state);
        if before_pending > 0 {
            last_x_growth = Instant::now();
            stall_raised = false;
        }

        // After macros, the game often restacks above OR hits. Re-raise when the
        // X queue goes quiet (no events at all — not only pointer).
        let any_busy = state.buttons.values().any(|b| b.spec.busy);
        if !state.buttons.is_empty()
            && !any_busy
            && !stall_raised
            && last_x_growth.elapsed() >= Duration::from_millis(800)
        {
            raise_all_hits(&state);
            stall_raised = true;
        }

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
                    // Pixels only — reapplying Shape* every spinner frame made the
                    // face eat clicks (empty ShapeInput regresses on XWayland/Mutter).
                    paint_button(&state, btn, false);
                }
            }
            // SAFETY: HostState display invariant.
            unsafe { x_flush(state.display) };
        }

        wait_x_or_timeout(state.xfd, if any_busy { BUSY_TICK_MS } else { POLL_IDLE_MS });
    }

    destroy_all(&mut state);
    // SAFETY: HostState display/gc still live; windows already destroyed.
    unsafe {
        x_free_gc(state.display, state.gc);
    }
    unregister_secondary_x_display(state.display.cast());
    // SAFETY: unregistered; no further Xlib use of this pointer.
    unsafe {
        x_close_display(state.display);
    }
    note("overlay-x11: host thread stopped");
}

fn apply_specs(state: &mut HostState, specs: Vec<NativeButtonSpec>) {
    let mut keep = std::collections::HashSet::new();
    for spec in specs {
        let id = spec.id.clone();
        keep.insert(id.clone());
        let dragging = state.drag.as_ref().is_some_and(|d| d.id == id);
        let updated = if let Some(live) = state.buttons.get_mut(&id) {
            let geom_changed = live.spec.x != spec.x
                || live.spec.y != spec.y
                || live.spec.w != spec.w
                || live.spec.h != spec.h;
            let radius_changed =
                (live.spec.corner_radius - spec.corner_radius).abs() > f32::EPSILON;
            let style_changed = live.spec.bg != spec.bg
                || live.spec.border != spec.border
                || (live.spec.border_width - spec.border_width).abs() > f32::EPSILON
                || radius_changed
                || live.spec.icon_glyph != spec.icon_glyph
                || live.spec.icon != spec.icon
                || live.spec.icon_hover != spec.icon_hover
                || live.spec.tip != spec.tip
                || live.spec.busy != spec.busy;
            let was_filled = live.spec.bg[3] > 0;
            let win = live.win;
            let hit = live.hit;
            // While relocating, keep the live drag position — app specs may still be stale.
            if geom_changed && !dragging {
                // SAFETY: HostState display invariant; face+hit created on this display.
                unsafe {
                    x_move_resize(state.display, hit, spec.x, spec.y, spec.w, spec.h);
                    x_move_resize(state.display, win, spec.x, spec.y, spec.w, spec.h);
                }
            }
            let (keep_x, keep_y) = (live.spec.x, live.spec.y);
            live.spec = spec;
            if dragging {
                live.spec.x = keep_x;
                live.spec.y = keep_y;
            }
            let filled = live.spec.bg[3] > 0;
            let geom_applied = geom_changed && !dragging;
            Some((
                // Opaque: rounded face Bounding (+ empty Input). Transparent: paint
                // reapplies chrome Bounding. Hit window keeps the full rounded Input.
                filled && (geom_applied || radius_changed || !was_filled),
                style_changed || geom_applied,
                live.spec.w,
                live.spec.h,
                live.spec.corner_radius,
            ))
        } else {
            // Create here so `spec` is not used after the update arm moves it.
            if let Ok(live) = create_button(state, spec) {
                if live.spec.bg[3] > 0 {
                    apply_face_rounded_bounding(
                        state.display,
                        live.win,
                        live.spec.w,
                        live.spec.h,
                        live.spec.corner_radius,
                    );
                }
                apply_hit_rounded_input(
                    state.display,
                    live.hit,
                    live.spec.w,
                    live.spec.h,
                    live.spec.corner_radius,
                );
                paint_button(state, &live, true);
                apply_button_cursor(state, live.hit, live.hovered);
                state.buttons.insert(live.spec.id.clone(), live);
            }
            None
        };
        if let Some((need_shape, need_paint, w, h, radius)) = updated {
            if need_shape {
                if let Some(live) = state.buttons.get(&id) {
                    apply_face_rounded_bounding(state.display, live.win, w, h, radius);
                    apply_hit_rounded_input(state.display, live.hit, w, h, radius);
                }
            }
            if need_paint {
                if let Some(live) = state.buttons.get(&id) {
                    paint_button(state, live, true);
                    if live.spec.bg[3] == 0 {
                        apply_hit_rounded_input(
                            state.display,
                            live.hit,
                            live.spec.w,
                            live.spec.h,
                            live.spec.corner_radius,
                        );
                    }
                    // Do not XMapRaised here — restacking after busy/style updates
                    // broke hit-testing until windows were remapped (focus cycle).
                }
                // Tips disabled (see EnterNotify); never re-show from apply_specs.
            }
        }
    }
    let drop_ids: Vec<String> = state
        .buttons
        .keys()
        .filter(|id| !keep.contains(*id))
        .cloned()
        .collect();
    for id in drop_ids {
        if state.drag.as_ref().is_some_and(|d| d.id == id) {
            cancel_drag(state);
        }
        if let Some(live) = state.buttons.remove(&id) {
            if state.tip.as_ref().is_some_and(|t| t.for_id == id) {
                hide_tip(state);
            }
            // SAFETY: HostState display invariant; face+hit created on this display.
            unsafe {
                x_destroy_window(state.display, live.win);
                x_destroy_window(state.display, live.hit);
            }
        }
    }
    // SAFETY: HostState display invariant.
    unsafe {
        x_flush(state.display);
    }
}

fn set_relocate_mode(state: &mut HostState, enabled: bool) {
    if state.relocate_mode == enabled {
        return;
    }
    if !enabled {
        cancel_drag(state);
    }
    state.relocate_mode = enabled;
    let wins: Vec<(Window, Window, bool)> = state
        .buttons
        .values()
        .map(|b| (b.hit, b.win, b.hovered))
        .collect();
    for (hit, face, hovered) in wins {
        // SAFETY: HostState display invariant; `hit`/`face` are live for this button.
        unsafe {
            x_select_button_input(state.display, hit, enabled);
            x_select_face_input(state.display, face, enabled);
        }
        apply_button_cursor(state, hit, hovered);
        raise_hit_above_face(state.display, hit, face);
    }
    // SAFETY: HostState display invariant.
    unsafe {
        x_flush(state.display);
    }
    note(&format!("overlay-x11: relocate_mode={enabled}"));
}

fn button_event_mask(relocate: bool) -> c_long {
    let mut mask = ButtonPressMask
        | ButtonReleaseMask
        | EnterWindowMask
        | LeaveWindowMask;
    if relocate {
        mask |= PointerMotionMask;
    }
    mask
}

fn button_id_for_event_win(state: &HostState, win: Window) -> Option<String> {
    state
        .buttons
        .values()
        .find(|b| b.hit == win || b.win == win)
        .map(|b| b.spec.id.clone())
}

fn apply_button_cursor(state: &HostState, win: Window, hovered: bool) {
    // SAFETY: HostState display invariant; cursor created on this display.
    unsafe {
        if state.relocate_mode && hovered && state.move_cursor != 0 {
            XDefineCursor(state.display, win, state.move_cursor);
        } else {
            XUndefineCursor(state.display, win);
        }
    }
}

fn cancel_drag(state: &mut HostState) {
    if state.drag.take().is_some() {
        // SAFETY: HostState display invariant; matches a prior XGrabPointer on this display.
        unsafe {
            XUngrabPointer(state.display, CurrentTime);
            x_flush(state.display);
        }
    }
}

fn commit_drag(state: &mut HostState) {
    let Some(drag) = state.drag.take() else {
        return;
    };
    // SAFETY: HostState display invariant; matches a prior XGrabPointer on this display.
    unsafe {
        XUngrabPointer(state.display, CurrentTime);
    }
    let Some(btn) = state.buttons.get(&drag.id) else {
        unsafe {
            x_flush(state.display);
        }
        return;
    };
    if btn.spec.x == drag.start_x && btn.spec.y == drag.start_y {
        // Click without move — keep catalog point bindings intact.
        unsafe {
            x_flush(state.display);
        }
        return;
    }
    let mv = OverlayButtonMove {
        id: drag.id.clone(),
        x: btn.spec.x,
        y: btn.spec.y,
    };
    mark_site(&format!("overlay-x11:move:{}", mv.id));
    note(&format!(
        "overlay-x11: relocate id={} -> {},{}",
        mv.id, mv.x, mv.y
    ));
    state.pending_moves.lock().push(mv);
    if let Some(ctx) = &state.wake {
        ctx.request_repaint();
    }
    // SAFETY: HostState display invariant.
    unsafe {
        x_flush(state.display);
    }
}

fn create_button(state: &HostState, spec: NativeButtonSpec) -> Result<LiveButton, String> {
    // Transparent fill still needs a backing pixel for any unshaped flecks / Expose.
    let bg_rgb = if spec.bg[3] > 0 {
        [spec.bg[0], spec.bg[1], spec.bg[2]]
    } else {
        [0, 0, 0]
    };
    let bg_pixel = alloc_color(state, bg_rgb);
    let w = spec.w.max(1);
    let h = spec.h.max(1);
    // SAFETY: HostState display invariant — create/configure/map on this thread's Display.
    // Face paints chrome; InputOnly hit sits above it (invisible, owns pointer). Putting
    // face on top + empty ShapeInput used to work until busy-spinner shape churn made the
    // face eat clicks on XWayland/Mutter with no ButtonPress selected.
    let (hit, win) = unsafe {
        let mut hit_attrs: XSetWindowAttributes = std::mem::zeroed();
        hit_attrs.override_redirect = True;
        hit_attrs.event_mask = button_event_mask(state.relocate_mode);
        let hit = XCreateWindow(
            state.display,
            state.root,
            spec.x,
            spec.y,
            w,
            h,
            0,
            0,
            InputOnly as c_uint,
            std::ptr::null_mut(),
            CWOverrideRedirect | CWEventMask,
            &mut hit_attrs,
        );
        if hit == 0 {
            return Err("XCreateWindow hit failed".into());
        }

        let mut face_attrs: XSetWindowAttributes = std::mem::zeroed();
        face_attrs.background_pixel = bg_pixel;
        face_attrs.border_pixel = bg_pixel;
        face_attrs.override_redirect = True;
        face_attrs.backing_store = WhenMapped;
        // Button events as fallback if stacking ever puts face above hit.
        face_attrs.event_mask =
            ExposureMask | StructureNotifyMask | button_event_mask(state.relocate_mode);
        let win = XCreateWindow(
            state.display,
            state.root,
            spec.x,
            spec.y,
            w,
            h,
            0,
            XDefaultDepth(state.display, state.screen),
            InputOutput as c_uint,
            XDefaultVisual(state.display, state.screen),
            CWBackPixel | CWBorderPixel | CWOverrideRedirect | CWBackingStore | CWEventMask,
            &mut face_attrs,
        );
        if win == 0 {
            x_destroy_window(state.display, hit);
            return Err("XCreateWindow face failed".into());
        }
        let c_title = std::ffi::CString::new(OVERLAY_WM_TITLE).unwrap_or_default();
        XStoreName(state.display, win, c_title.as_ptr());
        set_net_wm_type_notification(state.display, win);
        set_skip_taskbar_state(state.display, state.root, win);
        apply_empty_input_shape(state.display, win);
        x_select_button_input(state.display, hit, state.relocate_mode);
        // Face first, then hit on top — InputOnly does not obscure the face pixels.
        XMapRaised(state.display, win);
        XMapRaised(state.display, hit);
        XFlush(state.display);
        (hit, win)
    };
    mark_site(&format!("overlay-x11:map:{}", spec.id));
    note(&format!(
        "overlay-x11: mapped id={} {}x{}+{}+{} (face+hit)",
        spec.id, spec.w, spec.h, spec.x, spec.y
    ));
    Ok(LiveButton {
        spec,
        win,
        hit,
        armed: false,
        hovered: false,
        busy_phase: 0.0,
    })
}

fn paint_button(state: &HostState, btn: &LiveButton, update_shape: bool) {
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
    if update_shape && paint.bg[3] == 0 {
        apply_face_chrome_bounding(
            state.display,
            btn.win,
            paint.w,
            paint.h,
            &rgba,
        );
    }
    blit_rgba(state, btn.win, paint.w, paint.h, &rgba);
}

fn raise_hit_above_face(display: *mut Display, hit: Window, face: Window) {
    // SAFETY: caller passes host-thread display and live face+hit for one button.
    unsafe {
        XMapRaised(display, face);
        XMapRaised(display, hit);
    }
}

fn raise_all_hits(state: &HostState) {
    for btn in state.buttons.values() {
        raise_hit_above_face(state.display, btn.hit, btn.win);
    }
    if state.tip.as_ref().is_some_and(|t| t.mapped) {
        stack_tip_below_buttons(state);
    }
    // SAFETY: HostState display invariant.
    unsafe {
        x_flush(state.display);
    }
}

/// Apply busy from the shared running-macro slot (not egui sync).
///
/// While busy: empty the hit ShapeInput so macro mouse clicks reach the game.
/// When cleared: restore hit shape and raise above the game.
fn apply_running_busy(state: &mut HostState, running: Option<&str>) {
    let running = running.map(str::trim).filter(|s| !s.is_empty());
    let mut flipped = false;
    let mut became_idle = false;
    let ids: Vec<String> = state.buttons.keys().cloned().collect();
    for id in ids {
        let Some(btn) = state.buttons.get_mut(&id) else {
            continue;
        };
        let want = running.is_some_and(|n| btn.spec.macro_name.eq_ignore_ascii_case(n));
        if btn.spec.busy == want {
            continue;
        }
        if btn.spec.busy && !want {
            became_idle = true;
        }
        btn.spec.busy = want;
        btn.armed = false;
        flipped = true;
        let (hit, w, h, radius) = (btn.hit, btn.spec.w, btn.spec.h, btn.spec.corner_radius);
        if want {
            apply_empty_input_shape(state.display, hit);
        } else {
            apply_hit_rounded_input(state.display, hit, w, h, radius);
        }
    }
    if !flipped {
        return;
    }
    hide_tip(state);
    let paint_ids: Vec<String> = state.buttons.keys().cloned().collect();
    for id in paint_ids {
        if let Some(btn) = state.buttons.get(&id) {
            paint_button(state, btn, true);
        }
    }
    if became_idle || running.is_none() {
        raise_all_hits(state);
    }
    // SAFETY: HostState display invariant.
    unsafe {
        x_flush(state.display);
    }
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
    // Center under the button with a gap. Tip must stay *below* button hit windows —
    // a raised tip over a neighbor button eats clicks if ShapeInput pass-through fails.
    let tip_x = btn_x + (btn_w - tw as i32) / 2;
    let tip_y = btn_y + btn_h + TIP_GAP_PX;

    let need_new = state
        .tip
        .as_ref()
        .map(|t| t.w != tw || t.h != th)
        .unwrap_or(true);
    if need_new {
        if let Some(old) = state.tip.take() {
            // SAFETY: HostState display invariant; tip win created on this display.
            unsafe {
                x_destroy_window(state.display, old.win);
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
        // SAFETY: HostState display invariant; tip.win is live on this display.
        unsafe {
            x_move_resize(state.display, tip.win, tip_x, tip_y, tw, th);
            XMapWindow(state.display, tip.win);
        }
        tip.mapped = true;
    }
    if let Some(tip) = state.tip.as_ref() {
        apply_tip_shape(state.display, tip.win, tip.w, tip.h);
        blit_rgba(state, tip.win, tip.w, tip.h, &rgba);
        // Keep tip under button hits. ConfigureWindow can synthesize Leave/Enter —
        // suppress crossing and discard those events before re-enabling tip logic.
        state.suppress_crossing = true;
        stack_tip_below_buttons(state);
        // SAFETY: HostState display invariant.
        unsafe {
            x_flush(state.display);
        }
        discard_pending_crossing(state);
        state.suppress_crossing = false;
    }
}

fn stack_tip_below_buttons(state: &HostState) {
    let Some(tip) = state.tip.as_ref() else {
        return;
    };
    let Some(sibling) = state.buttons.values().next().map(|b| b.hit) else {
        return;
    };
    // SAFETY: tip + sibling are live windows on this display.
    unsafe {
        let mut changes: XWindowChanges = std::mem::zeroed();
        changes.sibling = sibling;
        changes.stack_mode = Below as c_int;
        XConfigureWindow(
            state.display,
            tip.win,
            (CWSibling | CWStackMode) as c_uint,
            &mut changes,
        );
    }
}

/// Drop Enter/Leave already queued after tip map/stack (prevents tip show↔hide loops).
fn discard_pending_crossing(state: &mut HostState) {
    loop {
        // SAFETY: HostState display invariant.
        let pending = unsafe { XPending(state.display) };
        if pending <= 0 {
            break;
        }
        let mut event: XEvent = unsafe { std::mem::zeroed() };
        // SAFETY: event written by XNextEvent before type check.
        unsafe {
            XNextEvent(state.display, &mut event);
        }
        let ty = event.get_type();
        if ty == EnterNotify || ty == LeaveNotify {
            continue;
        }
        // Put non-crossing events back by handling them inline is hard; re-queue via
        // XPutBackEvent so ButtonPress/etc. are not lost.
        // SAFETY: event was just received on this display.
        unsafe {
            x11::xlib::XPutBackEvent(state.display, &mut event);
        }
        break;
    }
}

fn hide_tip(state: &mut HostState) {
    let Some(tip) = state.tip.as_mut() else {
        return;
    };
    if !tip.mapped {
        tip.for_id.clear();
        tip.text.clear();
        return;
    }
    let tip_win = tip.win;
    tip.mapped = false;
    tip.for_id.clear();
    tip.text.clear();
    state.suppress_crossing = true;
    // SAFETY: HostState display invariant; tip.win is mapped on this display.
    unsafe {
        x_unmap(state.display, tip_win);
        x_flush(state.display);
    }
    discard_pending_crossing(state);
    state.suppress_crossing = false;
}

fn create_tip_window(
    state: &HostState,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
) -> Result<Window, String> {
    let bg_pixel = alloc_color(state, TIP_BG_RGB);
    // SAFETY: HostState display invariant — tip window lifecycle matches buttons.
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
        // Map without raising — show_tip stacks tip below button hits.
        XMapWindow(state.display, win);
        XFlush(state.display);
        Ok(win)
    }
}

/// Rounded bounding + empty input so the tip never steals hover from the button.
fn apply_tip_shape(display: *mut Display, win: Window, w: u32, h: u32) {
    apply_rounded_shape(display, win, w, h, TIP_CORNER_PX);
}

fn blit_rgba(state: &HostState, win: Window, w: u32, h: u32, rgba: &[u8]) {
    // SAFETY: HostState display/gc invariant; `win` is a button/tip on this display.
    // `XCreateImage` borrows `packed` until we null `data` and destroy the image.
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

fn drain_x_events(state: &mut HostState) -> bool {
    let mut saw_pointer = false;
    loop {
        // SAFETY: HostState display invariant.
        let pending = unsafe { XPending(state.display) };
        if pending <= 0 {
            break;
        }
        // SAFETY: `event` is written by XNextEvent before any field reads below.
        let mut event: XEvent = unsafe { std::mem::zeroed() };
        unsafe {
            XNextEvent(state.display, &mut event);
        }
        let ty = event.get_type();
        if ty == EnterNotify
            || ty == LeaveNotify
            || ty == MotionNotify
            || ty == ButtonPress
            || ty == ButtonRelease
        {
            saw_pointer = true;
        }
        if ty == Expose {
            // SAFETY: event type is Expose; expose union member is initialized.
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
            let id = button_id_for_event_win(state, win);
            if let Some(id) = id {
                if let Some(b) = state.buttons.get(&id) {
                    paint_button(state, b, true);
                }
            }
            continue;
        }
        if ty == EnterNotify || ty == LeaveNotify {
            // SAFETY: event type is Enter/Leave; crossing union member is initialized.
            let win = unsafe { event.crossing.window };
            let enter = ty == EnterNotify;
            // Ignore leave/enter chatter while a grab drag is active or tip restack.
            if state.drag.is_some() || state.suppress_crossing {
                continue;
            }
            let id = button_id_for_event_win(state, win);
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
                        // Do not raise/restack here — that synthesizes more Leave/Enter
                        // and used to loop tip show↔hide (~1ms) freezing the host.
                        paint_button(state, b, true);
                        apply_button_cursor(state, b.hit, b.hovered);
                    }
                }
                if let Some((show, tip_id)) = tip_action {
                    if show && !state.relocate_mode {
                        show_tip_for(state, &tip_id);
                    } else if state.tip.as_ref().is_some_and(|t| t.for_id == tip_id) {
                        hide_tip(state);
                    }
                }
            }
            continue;
        }
        if ty == MotionNotify {
            // SAFETY: event type is MotionNotify; XMotionEvent overlay is valid.
            let (root_x, root_y) = unsafe {
                let m = &*( &event as *const XEvent as *const x11::xlib::XMotionEvent);
                (m.x_root, m.y_root)
            };
            if let Some(drag) = state.drag.as_ref() {
                let nx = root_x - drag.grab_dx;
                let ny = root_y - drag.grab_dy;
                let id = drag.id.clone();
                if let Some(btn) = state.buttons.get_mut(&id) {
                    if btn.spec.x != nx || btn.spec.y != ny {
                        btn.spec.x = nx;
                        btn.spec.y = ny;
                        let (hit, face, w, h) = (btn.hit, btn.win, btn.spec.w, btn.spec.h);
                        // SAFETY: HostState display invariant; face+hit are this button.
                        unsafe {
                            x_move_resize(state.display, hit, nx, ny, w, h);
                            x_move_resize(state.display, face, nx, ny, w, h);
                        }
                    }
                }
            }
            continue;
        }
        if ty == ButtonPress {
            // SAFETY: event type is ButtonPress; XButtonEvent overlay is valid.
            let (win, button, root_x, root_y) = unsafe {
                let b = &*( &event as *const XEvent as *const x11::xlib::XButtonEvent);
                (b.window, b.button, b.x_root, b.y_root)
            };
            if button != BUTTON_LEFT {
                continue;
            }
            // Tip can sit over a neighboring button; hide before hit-test routing.
            hide_tip(state);
            let id = button_id_for_event_win(state, win);
            let Some(id) = id else {
                continue;
            };
            if state.relocate_mode {
                hide_tip(state);
                let (bx, by, bhit) = {
                    let Some(btn) = state.buttons.get(&id) else {
                        continue;
                    };
                    (btn.spec.x, btn.spec.y, btn.hit)
                };
                // SAFETY: HostState display invariant; grab on this button's hit cover.
                let grab_ok = unsafe {
                    XGrabPointer(
                        state.display,
                        bhit,
                        False,
                        (ButtonPressMask | ButtonReleaseMask | PointerMotionMask) as c_uint,
                        GrabModeAsync,
                        GrabModeAsync,
                        0,
                        state.move_cursor,
                        CurrentTime,
                    ) == Success as c_int
                };
                if grab_ok {
                    state.drag = Some(DragState {
                        id,
                        grab_dx: root_x - bx,
                        grab_dy: root_y - by,
                        start_x: bx,
                        start_y: by,
                    });
                }
                continue;
            }
            if let Some(btn) = state.buttons.get_mut(&id) {
                if btn.spec.busy {
                    mark_site(&format!("overlay-x11:press-busy:{id}"));
                    note(&format!("overlay-x11: press-busy-ignore id={id}"));
                } else {
                    btn.armed = true;
                    mark_site(&format!("overlay-x11:press:{id}"));
                    note(&format!("overlay-x11: press id={id}"));
                }
            }
            continue;
        }
        if ty == ButtonRelease {
            // SAFETY: event type is ButtonRelease; XButtonEvent overlay is valid.
            let (win, button) = unsafe {
                let b = &*( &event as *const XEvent as *const x11::xlib::XButtonEvent);
                (b.window, b.button)
            };
            if button != BUTTON_LEFT {
                continue;
            }
            if state.drag.is_some() {
                commit_drag(state);
                continue;
            }
            let id = button_id_for_event_win(state, win);
            if let Some(id) = id {
                if let Some(btn) = state.buttons.get_mut(&id) {
                    if btn.armed && !btn.spec.busy {
                        btn.armed = false;
                        let id = btn.spec.id.clone();
                        let macro_name = btn.spec.macro_name.clone();
                        mark_site(&format!("overlay-x11:click:{id}"));
                        note(&format!("overlay-x11: click id={id} macro={macro_name}"));
                        state.pending.lock().push(macro_name);
                        // Macro mouse/focus often restacks the game above us — re-raise now.
                        raise_all_hits(state);
                        if let Some(ctx) = &state.wake {
                            ctx.request_repaint();
                        }
                    } else {
                        mark_site(&format!(
                            "overlay-x11:release-ign:{}:a{}b{}",
                            id,
                            btn.armed as u8,
                            btn.spec.busy as u8
                        ));
                        note(&format!(
                            "overlay-x11: release-ignored id={id} armed={} busy={}",
                            btn.armed, btn.spec.busy
                        ));
                        btn.armed = false;
                    }
                }
            } else {
                note("overlay-x11: release on unknown window");
            }
        }
    }
    // SAFETY: HostState display invariant.
    unsafe {
        x_flush(state.display);
    }
    saw_pointer
}

fn destroy_all(state: &mut HostState) {
    cancel_drag(state);
    hide_tip(state);
    if let Some(tip) = state.tip.take() {
        // SAFETY: HostState display invariant; tip win created on this display.
        unsafe {
            x_destroy_window(state.display, tip.win);
        }
    }
    for (_, live) in state.buttons.drain() {
        // SAFETY: HostState display invariant; face+hit created on this display.
        unsafe {
            x_destroy_window(state.display, live.win);
            x_destroy_window(state.display, live.hit);
        }
    }
    if state.move_cursor != 0 {
        // SAFETY: cursor from XCreateFontCursor on this display; not used after.
        unsafe {
            XFreeCursor(state.display, state.move_cursor);
        }
        state.move_cursor = 0;
    }
    // SAFETY: HostState display invariant; flushes outstanding requests before close.
    unsafe {
        x_sync(state.display);
    }
}

fn alloc_color(state: &HostState, rgb: [u8; 3]) -> c_ulong {
    // SAFETY: HostState display invariant.
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

// --- Thin Xlib wrappers (HostState display / window invariants) ---------------

// SAFETY: `display` live; `win` was created on it and not yet destroyed.
unsafe fn x_select_button_input(display: *mut Display, win: Window, relocate: bool) {
    XSelectInput(display, win, button_event_mask(relocate));
}

// SAFETY: face is InputOutput — keep Expose/StructureNotify with button masks.
unsafe fn x_select_face_input(display: *mut Display, win: Window, relocate: bool) {
    XSelectInput(
        display,
        win,
        ExposureMask | StructureNotifyMask | button_event_mask(relocate),
    );
}

// SAFETY: `display` is a live host-thread connection (see module docs).
unsafe fn x_flush(display: *mut Display) {
    XFlush(display);
}

// SAFETY: `display` live; `win` was created on it and not yet destroyed.
unsafe fn x_destroy_window(display: *mut Display, win: Window) {
    XDestroyWindow(display, win);
}

// SAFETY: `display` live; `win` was created on it and not yet destroyed.
unsafe fn x_move_resize(display: *mut Display, win: Window, x: i32, y: i32, w: u32, h: u32) {
    XMoveResizeWindow(display, win, x, y, w, h);
}

// SAFETY: `display` live; `win` was created on it and not yet destroyed.
unsafe fn x_unmap(display: *mut Display, win: Window) {
    XUnmapWindow(display, win);
}

// SAFETY: `display` live for this thread.
unsafe fn x_sync(display: *mut Display) {
    XSync(display, False);
}

// SAFETY: `display` live; `gc` was created with XCreateGC on it and not freed.
unsafe fn x_free_gc(display: *mut Display, gc: *mut x11::xlib::_XGC) {
    XFreeGC(display, gc);
}

// SAFETY: `display` from XOpenDisplay on this thread; no further use after return.
unsafe fn x_close_display(display: *mut Display) {
    XCloseDisplay(display);
}

// SAFETY: callers pass a live host-thread `display` and a `win` created on it.
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

// SAFETY: callers pass a live host-thread `display`, its root, and a `win` on it.
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
    // SAFETY: `xfd` is XConnectionNumber for the live host Display; poll only reads readiness.
    unsafe {
        libc::poll(&mut fds, 1, timeout_ms as c_int);
    }
}

/// Opaque face: rounded Bounding + empty Input (hit cover owns clicks).
fn apply_face_rounded_bounding(
    display: *mut Display,
    win: Window,
    w: u32,
    h: u32,
    radius_px: f32,
) {
    if w == 0 || h == 0 || w > u32::from(u16::MAX) || h > u32::from(u16::MAX) {
        return;
    }
    let radius = radius_px.round().max(0.0) as i32;
    let mut rects = rounded_rect_xrectangles(w as i32, h as i32, radius);
    apply_shape_region(display, win, SHAPE_BOUNDING, &mut rects);
    apply_empty_input_shape(display, win);
}

/// Transparent face: chrome-only Bounding + empty Input (hollow stays clickable via hit).
fn apply_face_chrome_bounding(display: *mut Display, win: Window, w: u32, h: u32, rgba: &[u8]) {
    if w == 0 || h == 0 || w > u32::from(u16::MAX) || h > u32::from(u16::MAX) {
        return;
    }
    let runs = raster::shape_rects_from_alpha(rgba, w, h);
    let mut bound: Vec<XRectangle> = runs
        .into_iter()
        .map(|(x, y, width, height)| XRectangle {
            x,
            y,
            width,
            height,
        })
        .collect();
    if bound.is_empty() {
        bound.push(XRectangle {
            x: (w / 2) as i16,
            y: (h / 2) as i16,
            width: 1,
            height: 1,
        });
    }
    apply_shape_region(display, win, SHAPE_BOUNDING, &mut bound);
    apply_empty_input_shape(display, win);
}

/// Hit cover: rounded ShapeInput matching the button disk.
fn apply_hit_rounded_input(
    display: *mut Display,
    hit: Window,
    w: u32,
    h: u32,
    radius_px: f32,
) {
    if w == 0 || h == 0 || w > u32::from(u16::MAX) || h > u32::from(u16::MAX) {
        return;
    }
    let radius = radius_px.round().max(0.0) as i32;
    let mut rects = rounded_rect_xrectangles(w as i32, h as i32, radius);
    if rects.is_empty() {
        rects.push(XRectangle {
            x: 0,
            y: 0,
            width: w as u16,
            height: h as u16,
        });
    }
    apply_shape_region(display, hit, SHAPE_INPUT, &mut rects);
}

/// Tip / face: no pointer events (visual-only).
fn apply_empty_input_shape(display: *mut Display, win: Window) {
    // SAFETY: callers pass the host-thread display and a window created on it.
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

/// Opaque tip panel: rounded Bounding + empty Input.
fn apply_rounded_shape(display: *mut Display, win: Window, w: u32, h: u32, radius_px: f32) {
    if w == 0 || h == 0 || w > u32::from(u16::MAX) || h > u32::from(u16::MAX) {
        return;
    }
    let radius = radius_px.round().max(0.0) as i32;
    let mut rects = rounded_rect_xrectangles(w as i32, h as i32, radius);
    apply_shape_region(display, win, SHAPE_BOUNDING, &mut rects);
    apply_empty_input_shape(display, win);
}

fn apply_shape_region(
    display: *mut Display,
    win: Window,
    kind: c_int,
    rects: &mut [XRectangle],
) {
    if rects.is_empty() {
        return;
    }
    // SAFETY: callers pass the host-thread display and a window created on it.
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
        let region = XFixesCreateRegion(display, rects.as_mut_ptr(), rects.len() as c_int);
        if region == 0 {
            return;
        }
        XFixesSetWindowShapeRegion(display, win, kind, 0, 0, region);
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
