//! Layered topmost buttons on a dedicated thread.
//!
//! The window thread owns every HWND. Clicks enqueue a macro or request stop
//! without waiting for an egui frame. Detection captures exclude the windows
//! with `WDA_EXCLUDEFROMCAPTURE` when that setting is off, so they stay visible
//! and clickable while GDI reads the pixels underneath.

use crate::brand_s::SWEEP_PER_SEC;
use crate::raster::{self, ButtonPaint, TIP_CORNER_PX};
use crate::OverlayButtonMove;
use parking_lot::Mutex;
use sqyre_capture::{self, event_log, mark_site, note};
use std::cell::Cell;
use std::collections::HashMap;
use std::mem::size_of;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use windows::core::w;
use windows::Win32::Foundation::{
    GetLastError, COLORREF, ERROR_CLASS_ALREADY_EXISTS, HINSTANCE, HWND, LPARAM, LRESULT, POINT,
    SIZE, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateRoundRectRgn, DeleteDC, DeleteObject, SelectObject,
    SetWindowRgn, AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    DIB_RGB_COLORS, HGDIOBJ,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetCapture, ReleaseCapture, SetCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos, LoadCursorW,
    PeekMessageW, RegisterClassW, SetCursor, SetWindowDisplayAffinity, ShowWindow,
    TranslateMessage, UpdateLayeredWindow, CS_HREDRAW, CS_VREDRAW, HCURSOR, HTCLIENT,
    HTTRANSPARENT, IDC_ARROW, IDC_SIZEALL, MA_NOACTIVATE, MSG, PM_REMOVE, SW_HIDE,
    SW_SHOWNOACTIVATE, ULW_ALPHA, WDA_EXCLUDEFROMCAPTURE, WDA_NONE, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_NCHITTEST, WM_SETCURSOR, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

const CLASS_NAME: windows::core::PCWSTR = w!("SqyreOverlayButton");
const TIP_GAP_PX: i32 = 6;
const POLL_MS: u64 = 16;
/// `WM_MOUSELEAVE` is declared under `Win32_UI_Controls` in the windows crate.
const WM_MOUSELEAVE: u32 = 675;

/// Window handle shared with the capture thread.
///
/// `HWND` is a raw pointer, so the windows crate does not implement `Send`.
/// The capture thread only passes the value to `SetWindowDisplayAffinity`.
#[derive(Clone, Copy)]
struct ShareHwnd(HWND);

// SAFETY: the value is an opaque handle. Callers never dereference it.
unsafe impl Send for ShareHwnd {}
// SAFETY: copies of the handle are plain integers. The mutex orders the list.
unsafe impl Sync for ShareHwnd {}

thread_local! {
    static HOST: Cell<*mut HostState> = const { Cell::new(std::ptr::null_mut()) };
}

/// Desired on-screen button (physical pixels, virtual-desktop coordinates).
#[derive(Debug, Clone, PartialEq)]
pub struct WinButtonSpec {
    pub id: String,
    pub macro_name: String,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub bg: [u8; 4],
    pub border: [u8; 4],
    pub border_width: f32,
    pub corner_radius: f32,
    pub icon_glyph: char,
    pub icon: [u8; 4],
    pub icon_hover: [u8; 4],
    pub tip: String,
    pub busy: bool,
    pub visibility_gated: bool,
}

enum HostCmd {
    SetButtons(Vec<WinButtonSpec>),
    SetRelocate(bool),
    SetVisibility(Arc<Mutex<HashMap<String, bool>>>),
    SetWake(egui::Context),
    Shutdown,
}

/// Owns the window thread. Clicks enqueue into `pending` immediately.
pub struct WinButtonHost {
    cmd_tx: Sender<HostCmd>,
    stop: Arc<AtomicBool>,
    join: Mutex<Option<JoinHandle<()>>>,
}

impl Drop for WinButtonHost {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl WinButtonHost {
    pub fn start(
        pending: Arc<Mutex<Vec<String>>>,
        pending_moves: Arc<Mutex<Vec<OverlayButtonMove>>>,
        running_macro: Arc<Mutex<Option<String>>>,
        visibility: Arc<Mutex<HashMap<String, bool>>>,
        run_stop: Arc<Mutex<sqyre_hotkeys::StopFlag>>,
        press_stops: Arc<AtomicBool>,
    ) -> Option<Self> {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_t = Arc::clone(&stop);
        let hwnds = Arc::new(Mutex::new(Vec::new()));
        let hwnds_hook = Arc::clone(&hwnds);
        let join = match thread::Builder::new()
            .name("sqyre-win-overlay".into())
            .spawn(move || {
                host_loop(
                    cmd_rx,
                    pending,
                    pending_moves,
                    running_macro,
                    visibility,
                    run_stop,
                    press_stops,
                    hwnds,
                    stop_t,
                );
            }) {
            Ok(join) => join,
            Err(e) => {
                note(&format!("overlay: windows host failed to start: {e}"));
                return None;
            }
        };
        sqyre_capture::set_overlay_omit_hook(Some(Arc::new(move |omit| {
            apply_affinity(&hwnds_hook.lock(), omit);
        })));
        Some(Self {
            cmd_tx,
            stop,
            join: Mutex::new(Some(join)),
        })
    }

    pub fn set_buttons(&self, buttons: Vec<WinButtonSpec>) {
        let _ = self.cmd_tx.send(HostCmd::SetButtons(buttons));
    }

    pub fn set_relocate(&self, enabled: bool) {
        let _ = self.cmd_tx.send(HostCmd::SetRelocate(enabled));
    }

    pub fn set_visibility_map(&self, map: Arc<Mutex<HashMap<String, bool>>>) {
        let _ = self.cmd_tx.send(HostCmd::SetVisibility(map));
    }

    pub fn set_wake(&self, ctx: egui::Context) {
        let _ = self.cmd_tx.send(HostCmd::SetWake(ctx));
    }

    pub fn shutdown(&self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.cmd_tx.send(HostCmd::Shutdown);
        sqyre_capture::set_overlay_omit_hook(None);
        if let Some(join) = self.join.lock().take() {
            if sqyre_capture::process_exiting() {
                note("overlay-win: shutdown abandon join");
                std::mem::forget(join);
            } else {
                let _ = join.join();
            }
        }
    }
}

struct LiveButton {
    spec: WinButtonSpec,
    hwnd: HWND,
    mapped: bool,
    hovered: bool,
    phase: f32,
    bgra: Vec<u8>,
}

struct TipWindow {
    hwnd: HWND,
    for_id: String,
    text: String,
    mapped: bool,
}

struct Drag {
    id: String,
    grab_x: i32,
    grab_y: i32,
}

struct HostState {
    buttons: HashMap<String, LiveButton>,
    tip: Option<TipWindow>,
    pending: Arc<Mutex<Vec<String>>>,
    pending_moves: Arc<Mutex<Vec<OverlayButtonMove>>>,
    running_macro: Arc<Mutex<Option<String>>>,
    visibility: Arc<Mutex<HashMap<String, bool>>>,
    wake: Option<egui::Context>,
    run_stop: Arc<Mutex<sqyre_hotkeys::StopFlag>>,
    press_stops: Arc<AtomicBool>,
    hwnds: Arc<Mutex<Vec<ShareHwnd>>>,
    relocate: bool,
    drag: Option<Drag>,
    arrow: HCURSOR,
    size_all: HCURSOR,
    module: HINSTANCE,
    last_busy: Instant,
}

#[expect(
    clippy::too_many_arguments,
    reason = "the host thread owns the queues, stop flag, and window list together"
)]
fn host_loop(
    cmd_rx: Receiver<HostCmd>,
    pending: Arc<Mutex<Vec<String>>>,
    pending_moves: Arc<Mutex<Vec<OverlayButtonMove>>>,
    running_macro: Arc<Mutex<Option<String>>>,
    visibility: Arc<Mutex<HashMap<String, bool>>>,
    run_stop: Arc<Mutex<sqyre_hotkeys::StopFlag>>,
    press_stops: Arc<AtomicBool>,
    hwnds: Arc<Mutex<Vec<ShareHwnd>>>,
    stop: Arc<AtomicBool>,
) {
    if ensure_class().is_err() {
        note("overlay-win: RegisterClassW failed");
        return;
    }
    // SAFETY: module handle is the process image; cursors are shared stock objects.
    let (module, arrow, size_all) = unsafe {
        let Ok(instance) = GetModuleHandleW(None) else {
            note("overlay-win: GetModuleHandleW failed");
            return;
        };
        let arrow = LoadCursorW(None, IDC_ARROW).unwrap_or_default();
        let size_all = LoadCursorW(None, IDC_SIZEALL).unwrap_or(arrow);
        (HINSTANCE::from(instance), arrow, size_all)
    };
    let mut state = HostState {
        buttons: HashMap::new(),
        tip: None,
        pending,
        pending_moves,
        running_macro,
        visibility,
        wake: None,
        run_stop,
        press_stops,
        hwnds,
        relocate: false,
        drag: None,
        arrow,
        size_all,
        module,
        last_busy: Instant::now(),
    };
    HOST.with(|slot| slot.set(std::ptr::from_mut(&mut state)));
    note("overlay: using native Win32 button host");
    while !stop.load(Ordering::Relaxed) {
        loop {
            match cmd_rx.try_recv() {
                Ok(HostCmd::SetButtons(specs)) => apply_specs(&mut state, specs),
                Ok(HostCmd::SetRelocate(enabled)) => {
                    state.relocate = enabled;
                    if !enabled {
                        state.drag = None;
                    }
                    hide_tip(&mut state);
                    apply_visibility(&mut state);
                }
                Ok(HostCmd::SetVisibility(map)) => {
                    state.visibility = map;
                    apply_visibility(&mut state);
                }
                Ok(HostCmd::SetWake(ctx)) => state.wake = Some(ctx),
                Ok(HostCmd::Shutdown) => {
                    stop.store(true, Ordering::Relaxed);
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    stop.store(true, Ordering::Relaxed);
                    break;
                }
            }
        }
        if stop.load(Ordering::Relaxed) {
            break;
        }
        // SAFETY: messages belong to this thread; HOST is set so the wndproc can see `state`.
        unsafe { drain_messages() };
        tick_busy(&mut state);
        apply_visibility(&mut state);
        thread::sleep(Duration::from_millis(POLL_MS));
    }
    destroy_all(&mut state);
    HOST.with(|slot| slot.set(std::ptr::null_mut()));
    note("overlay: windows button host stopped");
}

fn apply_specs(state: &mut HostState, specs: Vec<WinButtonSpec>) {
    let mut keep = std::collections::HashSet::new();
    for spec in specs {
        let id = spec.id.clone();
        keep.insert(id.clone());
        if state.buttons.contains_key(&id) {
            update_button(state, spec);
        } else if let Some(live) = create_button(state, spec) {
            let id = live.spec.id.clone();
            state.buttons.insert(id, live);
        }
    }
    let stale: Vec<String> = state
        .buttons
        .keys()
        .filter(|id| !keep.contains(*id))
        .cloned()
        .collect();
    for id in stale {
        if state.drag.as_ref().is_some_and(|d| d.id == id) {
            state.drag = None;
        }
        if state.tip.as_ref().is_some_and(|t| t.for_id == id) {
            hide_tip(state);
        }
        if let Some(live) = state.buttons.remove(&id) {
            destroy_hwnd(state, live.hwnd);
        }
    }
    sync_hwnd_list(state);
    apply_visibility(state);
}

fn update_button(state: &mut HostState, spec: WinButtonSpec) {
    let id = spec.id.clone();
    let Some(live) = state.buttons.get_mut(&id) else {
        return;
    };
    let dragging = state.drag.as_ref().is_some_and(|d| d.id == id);
    let (x, y) = if dragging {
        (live.spec.x, live.spec.y)
    } else {
        (spec.x, spec.y)
    };
    let geom = live.spec.w != spec.w
        || live.spec.h != spec.h
        || (live.spec.corner_radius - spec.corner_radius).abs() > f32::EPSILON
        || live.spec.x != x
        || live.spec.y != y;
    let style = live.spec.bg != spec.bg
        || live.spec.border != spec.border
        || live.spec.icon != spec.icon
        || live.spec.icon_glyph != spec.icon_glyph
        || live.spec.busy != spec.busy;
    live.spec = spec;
    live.spec.x = x;
    live.spec.y = y;
    if geom {
        // Region uses the new size. Recreate it before the next paint.
        let hwnd = live.hwnd;
        let w = live.spec.w;
        let h = live.spec.h;
        let radius = live.spec.corner_radius;
        set_round_region(hwnd, w, h, radius);
    }
    if geom || style {
        let id = id.clone();
        repaint_button(state, &id);
    }
}

fn create_button(state: &mut HostState, spec: WinButtonSpec) -> Option<LiveButton> {
    let w = spec.w.max(1);
    let h = spec.h.max(1);
    // SAFETY: class is registered; the popup is owned by this thread until `destroy_hwnd`.
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
            CLASS_NAME,
            w!("Sqyre"),
            WS_POPUP,
            spec.x,
            spec.y,
            w as i32,
            h as i32,
            None,
            None,
            Some(state.module),
            None,
        )
        .ok()?
    };
    set_round_region(hwnd, w, h, spec.corner_radius);
    if sqyre_capture::overlay_capture_omitted() {
        apply_affinity(&[ShareHwnd(hwnd)], true);
    }
    let mut live = LiveButton {
        spec,
        hwnd,
        mapped: false,
        hovered: false,
        phase: 0.0,
        bgra: Vec::new(),
    };
    paint_live(&mut live);
    Some(live)
}

fn tick_busy(state: &mut HostState) {
    let dt = state.last_busy.elapsed().as_secs_f32();
    if dt < 0.05 {
        return;
    }
    state.last_busy = Instant::now();
    let dt = dt.clamp(0.0, 0.25);
    let ids: Vec<String> = state.buttons.keys().cloned().collect();
    for id in ids {
        let busy = state
            .buttons
            .get(&id)
            .is_some_and(|b| b.spec.busy && b.mapped);
        if !busy {
            if let Some(btn) = state.buttons.get_mut(&id) {
                btn.phase = 0.0;
            }
            continue;
        }
        if let Some(btn) = state.buttons.get_mut(&id) {
            btn.phase = (btn.phase + SWEEP_PER_SEC * dt) % 1.0;
        }
        repaint_button(state, &id);
    }
}

fn apply_visibility(state: &mut HostState) {
    let found = state.visibility.lock().clone();
    let relocate = state.relocate;
    let ids: Vec<String> = state.buttons.keys().cloned().collect();
    for id in ids {
        let Some(btn) = state.buttons.get(&id) else {
            continue;
        };
        let show =
            relocate || !btn.spec.visibility_gated || found.get(&id).copied().unwrap_or(false);
        if show == btn.mapped {
            continue;
        }
        let hwnd = btn.hwnd;
        // SAFETY: `hwnd` was created on this thread and not yet destroyed.
        unsafe {
            let _ = ShowWindow(hwnd, if show { SW_SHOWNOACTIVATE } else { SW_HIDE });
        }
        if let Some(btn) = state.buttons.get_mut(&id) {
            btn.mapped = show;
        }
        if !show && state.tip.as_ref().is_some_and(|t| t.for_id == id) {
            hide_tip(state);
        }
    }
}

fn repaint_button(state: &mut HostState, id: &str) {
    let Some(btn) = state.buttons.get_mut(id) else {
        return;
    };
    paint_live(btn);
}

fn paint_live(btn: &mut LiveButton) {
    let w = btn.spec.w.max(1);
    let h = btn.spec.h.max(1);
    let rgba = raster::rasterize_argb(&ButtonPaint {
        w,
        h,
        bg: btn.spec.bg,
        border: btn.spec.border,
        border_width: btn.spec.border_width,
        corner_radius: btn.spec.corner_radius,
        icon_glyph: btn.spec.icon_glyph,
        icon: btn.spec.icon,
        icon_hover: btn.spec.icon_hover,
        hovered: btn.hovered,
        busy: btn.spec.busy,
        busy_phase: btn.phase,
    });
    btn.bgra = rgba_to_bgra(&rgba);
    blit_layered(btn.hwnd, btn.spec.x, btn.spec.y, w, h, &btn.bgra);
}

fn show_tip(state: &mut HostState, id: &str) {
    if state.relocate || state.drag.is_some() {
        hide_tip(state);
        return;
    }
    let Some(btn) = state.buttons.get(id) else {
        return;
    };
    if !btn.mapped {
        hide_tip(state);
        return;
    }
    let text = btn.spec.tip.trim().to_string();
    if text.is_empty() {
        hide_tip(state);
        return;
    }
    let (x, y, h) = (btn.spec.x, btn.spec.y, btn.spec.h);
    if state
        .tip
        .as_ref()
        .is_some_and(|t| t.mapped && t.for_id == id && t.text == text)
    {
        return;
    }
    let (tw, th, rgba) = raster::rasterize_tip(&text);
    if tw == 0 || th == 0 || rgba.is_empty() {
        hide_tip(state);
        return;
    }
    let bgra = rgba_to_bgra(&rgba);
    let tip_x = x;
    let tip_y = y + h as i32 + TIP_GAP_PX;
    if state.tip.is_none() {
        // SAFETY: same class and module as the buttons; destroyed in `destroy_all`.
        let hwnd = unsafe {
            match CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TOPMOST
                    | WS_EX_NOACTIVATE
                    | WS_EX_TOOLWINDOW
                    | WS_EX_TRANSPARENT,
                CLASS_NAME,
                w!("Sqyre tip"),
                WS_POPUP,
                tip_x,
                tip_y,
                tw as i32,
                th as i32,
                None,
                None,
                Some(state.module),
                None,
            ) {
                Ok(hwnd) => hwnd,
                Err(_) => return,
            }
        };
        set_round_region(hwnd, tw, th, TIP_CORNER_PX);
        if sqyre_capture::overlay_capture_omitted() {
            apply_affinity(&[ShareHwnd(hwnd)], true);
        }
        state.tip = Some(TipWindow {
            hwnd,
            for_id: id.to_string(),
            text: text.clone(),
            mapped: false,
        });
        sync_hwnd_list(state);
    }
    let Some(tip) = state.tip.as_mut() else {
        return;
    };
    tip.for_id = id.to_string();
    tip.text = text;
    blit_layered(tip.hwnd, tip_x, tip_y, tw, th, &bgra);
    if !tip.mapped {
        // SAFETY: tip HWND is live on this thread.
        unsafe {
            let _ = ShowWindow(tip.hwnd, SW_SHOWNOACTIVATE);
        }
        tip.mapped = true;
    }
}

fn hide_tip(state: &mut HostState) {
    let Some(tip) = state.tip.as_mut() else {
        return;
    };
    if tip.mapped {
        // SAFETY: tip HWND is live on this thread.
        unsafe {
            let _ = ShowWindow(tip.hwnd, SW_HIDE);
        }
        tip.mapped = false;
    }
}

fn on_button_down(state: &mut HostState, id: &str, client_x: i32, client_y: i32) {
    if state.relocate {
        state.drag = Some(Drag {
            id: id.to_string(),
            grab_x: client_x,
            grab_y: client_y,
        });
        if let Some(btn) = state.buttons.get(id) {
            // SAFETY: button HWND is live on this thread. Capture keeps move/up flowing.
            unsafe {
                let _ = SetCapture(btn.hwnd);
            }
        }
        hide_tip(state);
        return;
    }
    let running = state
        .running_macro
        .lock()
        .as_deref()
        .is_some_and(|n| !n.trim().is_empty());
    if running && state.press_stops.load(Ordering::Relaxed) {
        mark_site(&format!("overlay-win:press-stop:{id}"));
        note(&format!("overlay-win: press-stop id={id}"));
        state.run_stop.lock().request_stop();
        return;
    }
    let Some(btn) = state.buttons.get(id) else {
        return;
    };
    if btn.spec.busy {
        note(&format!("overlay-win: press-busy-ignore id={id}"));
        return;
    }
    let name = btn.spec.macro_name.clone();
    if name.trim().is_empty() {
        return;
    }
    mark_site(&format!("overlay-win:click:{id}"));
    note(&format!("overlay-win: click id={id} macro={name}"));
    event_log(
        "SQYRE_OVERLAY",
        &[("click", "press"), ("id", id), ("name", name.as_str())],
    );
    state.pending.lock().push(name);
    if let Some(ctx) = &state.wake {
        ctx.request_repaint();
    }
}

fn on_drag_move(state: &mut HostState) {
    let Some(drag) = state.drag.as_ref() else {
        return;
    };
    let id = drag.id.clone();
    let (grab_x, grab_y) = (drag.grab_x, drag.grab_y);
    let mut pt = POINT::default();
    // SAFETY: `pt` is a valid out-param for the cursor position.
    if unsafe { GetCursorPos(&mut pt) }.is_err() {
        return;
    }
    let x = pt.x - grab_x;
    let y = pt.y - grab_y;
    let Some(btn) = state.buttons.get_mut(&id) else {
        return;
    };
    btn.spec.x = x;
    btn.spec.y = y;
    let (hwnd, w, h, bgra) = (
        btn.hwnd,
        btn.spec.w.max(1),
        btn.spec.h.max(1),
        btn.bgra.clone(),
    );
    blit_layered(hwnd, x, y, w, h, &bgra);
}

fn on_button_up(state: &mut HostState) {
    // SAFETY: release only matters while this thread holds the capture.
    unsafe {
        if !GetCapture().is_invalid() {
            let _ = ReleaseCapture();
        }
    }
    let Some(drag) = state.drag.take() else {
        return;
    };
    let Some(btn) = state.buttons.get(&drag.id) else {
        return;
    };
    state.pending_moves.lock().push(OverlayButtonMove {
        id: drag.id,
        x: btn.spec.x,
        y: btn.spec.y,
    });
    if let Some(ctx) = &state.wake {
        ctx.request_repaint();
    }
}

fn on_hover(state: &mut HostState, id: &str, hovered: bool) {
    let changed = state.buttons.get(id).is_some_and(|b| b.hovered != hovered);
    if let Some(btn) = state.buttons.get_mut(id) {
        btn.hovered = hovered;
    }
    if hovered {
        if let Some(btn) = state.buttons.get(id) {
            // SAFETY: button HWND is live. One leave event ends the hover.
            unsafe {
                let mut track = TRACKMOUSEEVENT {
                    cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: btn.hwnd,
                    dwHoverTime: 0,
                };
                let _ = TrackMouseEvent(&mut track);
            }
        }
        show_tip(state, id);
    } else if state.tip.as_ref().is_some_and(|t| t.for_id == id) {
        hide_tip(state);
    }
    if changed {
        let id = id.to_string();
        repaint_button(state, &id);
    }
}

fn button_at(state: &HostState, hwnd: HWND) -> Option<String> {
    state
        .buttons
        .iter()
        .find(|(_, b)| b.hwnd == hwnd)
        .map(|(id, _)| id.clone())
}

fn hit_test(state: &HostState, hwnd: HWND, screen_x: i32, screen_y: i32) -> LRESULT {
    if state.tip.as_ref().is_some_and(|t| t.hwnd == hwnd) {
        return LRESULT(HTTRANSPARENT as isize);
    }
    let Some((_, btn)) = state.buttons.iter().find(|(_, b)| b.hwnd == hwnd) else {
        return LRESULT(HTCLIENT as isize);
    };
    let running = state
        .running_macro
        .lock()
        .as_deref()
        .is_some_and(|n| !n.trim().is_empty());
    let pass = btn.spec.busy && running && !state.press_stops.load(Ordering::Relaxed);
    if pass {
        return LRESULT(HTTRANSPARENT as isize);
    }
    let local_x = screen_x - btn.spec.x;
    let local_y = screen_y - btn.spec.y;
    if inside_rounded(
        local_x,
        local_y,
        btn.spec.w,
        btn.spec.h,
        btn.spec.corner_radius,
    ) {
        LRESULT(HTCLIENT as isize)
    } else {
        LRESULT(HTTRANSPARENT as isize)
    }
}

fn inside_rounded(x: i32, y: i32, w: u32, h: u32, radius: f32) -> bool {
    if w == 0 || h == 0 {
        return false;
    }
    let xf = x as f32;
    let yf = y as f32;
    let wf = w as f32;
    let hf = h as f32;
    if xf < 0.0 || yf < 0.0 || xf >= wf || yf >= hf {
        return false;
    }
    let r = radius.clamp(0.0, wf.min(hf) * 0.5);
    let cx = if xf < r {
        r
    } else if xf > wf - r {
        wf - r
    } else {
        xf
    };
    let cy = if yf < r {
        r
    } else if yf > hf - r {
        hf - r
    } else {
        yf
    };
    let dx = xf - cx;
    let dy = yf - cy;
    dx * dx + dy * dy <= r * r
}

fn lparam_xy(lp: LPARAM) -> (i32, i32) {
    let v = lp.0 as u32;
    let x = (v & 0xffff) as i16 as i32;
    let y = ((v >> 16) & 0xffff) as i16 as i32;
    (x, y)
}

fn sync_hwnd_list(state: &HostState) {
    let mut list = Vec::new();
    list.extend(state.buttons.values().map(|b| ShareHwnd(b.hwnd)));
    if let Some(tip) = &state.tip {
        list.push(ShareHwnd(tip.hwnd));
    }
    *state.hwnds.lock() = list;
}

fn destroy_hwnd(state: &HostState, hwnd: HWND) {
    state.hwnds.lock().retain(|h| h.0 != hwnd);
    // SAFETY: `hwnd` was created on this thread.
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

fn destroy_all(state: &mut HostState) {
    hide_tip(state);
    if let Some(tip) = state.tip.take() {
        destroy_hwnd(state, tip.hwnd);
    }
    let buttons = std::mem::take(&mut state.buttons);
    for (_, live) in buttons {
        destroy_hwnd(state, live.hwnd);
    }
    state.hwnds.lock().clear();
}

fn apply_affinity(hwnds: &[ShareHwnd], omit: bool) {
    let mode = if omit {
        WDA_EXCLUDEFROMCAPTURE
    } else {
        WDA_NONE
    };
    for hwnd in hwnds {
        if hwnd.0.is_invalid() {
            continue;
        }
        // SAFETY: affinity is a window property; the HWND is either live or the call fails.
        unsafe {
            let _ = SetWindowDisplayAffinity(hwnd.0, mode);
        }
    }
}

fn set_round_region(hwnd: HWND, w: u32, h: u32, radius: f32) {
    let r = radius.round().max(0.0) as i32;
    if r <= 0 || w == 0 || h == 0 {
        return;
    }
    // SAFETY: the region is handed to the window, which owns it after a successful set.
    unsafe {
        let rgn = CreateRoundRectRgn(0, 0, w as i32, h as i32, r * 2, r * 2);
        if rgn.is_invalid() {
            return;
        }
        if SetWindowRgn(hwnd, Some(rgn), true) == 0 {
            let _ = DeleteObject(HGDIOBJ::from(rgn));
        }
    }
}

fn rgba_to_bgra(rgba: &[u8]) -> Vec<u8> {
    let mut out = rgba.to_vec();
    for px in out.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    out
}

fn blit_layered(hwnd: HWND, x: i32, y: i32, w: u32, h: u32, bgra: &[u8]) {
    if w == 0 || h == 0 || bgra.len() < (w * h * 4) as usize || hwnd.is_invalid() {
        return;
    }
    // SAFETY: the DIB and DC are released before return. `bgra` is tightly packed top-down BGRA.
    unsafe {
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w as i32,
                biHeight: -(h as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [Default::default()],
        };
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let Ok(bitmap) = CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut bits, None, 0) else {
            return;
        };
        if bits.is_null() {
            let _ = DeleteObject(HGDIOBJ::from(bitmap));
            return;
        }
        std::ptr::copy_nonoverlapping(bgra.as_ptr(), bits.cast::<u8>(), (w * h * 4) as usize);
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            let _ = DeleteObject(HGDIOBJ::from(bitmap));
            return;
        }
        let old = SelectObject(dc, HGDIOBJ::from(bitmap));
        let dst = POINT { x, y };
        let src = POINT { x: 0, y: 0 };
        let size = SIZE {
            cx: w as i32,
            cy: h as i32,
        };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let _ = UpdateLayeredWindow(
            hwnd,
            None,
            Some(&dst),
            Some(&size),
            Some(dc),
            Some(&src),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );
        SelectObject(dc, old);
        let _ = DeleteDC(dc);
        let _ = DeleteObject(HGDIOBJ::from(bitmap));
    }
}

fn ensure_class() -> Result<(), ()> {
    use std::sync::OnceLock;
    static CLASS: OnceLock<Result<(), u32>> = OnceLock::new();
    let result = CLASS.get_or_init(|| {
        // SAFETY: process-lifetime window class; no background brush (layered paint).
        unsafe {
            let module = GetModuleHandleW(None).map_err(|e| e.code().0 as u32)?;
            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wnd_proc),
                hInstance: module.into(),
                lpszClassName: CLASS_NAME,
                ..Default::default()
            };
            if RegisterClassW(&wc) == 0 {
                let err = GetLastError();
                if err == ERROR_CLASS_ALREADY_EXISTS {
                    return Ok(());
                }
                return Err(err.0);
            }
            Ok(())
        }
    });
    result.map_err(|_| ())
}

/// # Safety
/// Called only from this thread's message pump while [`HOST`] points at the live state.
unsafe fn drain_messages() {
    // SAFETY: `msg` is a valid out-param; the queue belongs to this thread.
    unsafe {
        let mut msg = MSG::default();
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: the host thread sets HOST before pumping and clears it after destroy.
    let state = HOST.with(|slot| slot.get());
    if state.is_null() {
        // SAFETY: the system passes a live window of our class and its message params.
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    // SAFETY: the host thread sets HOST to its live state before pumping and clears
    // it only after every window of this class is destroyed.
    let state = unsafe { &mut *state };
    match msg {
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_NCHITTEST => {
            let mut pt = POINT::default();
            // SAFETY: `pt` is a valid out-param for the cursor position.
            let (x, y) = if unsafe { GetCursorPos(&mut pt) }.is_ok() {
                (pt.x, pt.y)
            } else {
                lparam_xy(lparam)
            };
            hit_test(state, hwnd, x, y)
        }
        WM_SETCURSOR => {
            let cursor = if state.relocate {
                state.size_all
            } else {
                state.arrow
            };
            if !cursor.is_invalid() {
                // SAFETY: stock cursor loaded at startup; still valid for the process.
                unsafe {
                    SetCursor(Some(cursor));
                }
                return LRESULT(1);
            }
            // SAFETY: unhandled cursor messages go to the default window procedure.
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_LBUTTONDOWN => {
            if let Some(id) = button_at(state, hwnd) {
                let (x, y) = lparam_xy(lparam);
                on_button_down(state, &id, x, y);
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            if state.drag.is_some() {
                on_drag_move(state);
            } else if let Some(id) = button_at(state, hwnd) {
                on_hover(state, &id, true);
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            on_button_up(state);
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            if let Some(id) = button_at(state, hwnd) {
                on_hover(state, &id, false);
            }
            LRESULT(0)
        }
        // SAFETY: every other message is forwarded unchanged to the default procedure.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
