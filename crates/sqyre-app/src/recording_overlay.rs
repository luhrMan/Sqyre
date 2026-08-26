//! Screen-click selection outline, mouse-owning grab, and recording coords HUD.
//!
//! Driven by [`sqyre_hotkeys::ScreenClickBridge`] and tooltip preview requests:
//! - A fullscreen OS grab ([`sqyre_capture::SelectionGrab`]) that takes the pointer
//!   while Point / Color / SearchArea recording is armed, so games that confine or
//!   relative-capture the mouse cannot block selection. Do **not** use an eframe
//!   fullscreen viewport for this on GNOME/Wayland: Mutter un-redirects those
//!   surfaces and they paint as opaque black.
//! - On GNOME/Wayland, a frozen screenshot cover ([`sqyre_capture::FrozenSelectionOverlay`])
//!   sits over XWayland windows, owns pointer events, and paints the gold rubber-band
//!   onto that freeze (separate edge windows would sit under the cover).
//! - Native X11 uses the grab + edge windows without a snapshot.
//! - A small always-on-top egui viewport for live coords / status while recording
//!   (needed when the main window is hidden via `hide_app_during_recording`).
//!   The HUD sits on the opposite vertical edge of the monitor from the cursor so
//!   it stays out of the way while pointing / selecting.
//!
//! Outline / grab HWNDs and X11 windows are updated on the UI thread only. A short
//! poller only `request_repaint`s while recording is armed so the HUD keeps updating
//! when the root viewport is `Visible(false)`. On Wayland the main window is unmapped
//! during recording (GSR-style) so portal captures exclude Sqyre; selection uses the
//! frozen snapshot cover and a deferred HUD viewport.

use crate::theme;
use eframe::egui::{self, Pos2, TextStyle, Vec2, ViewportBuilder, ViewportClass, ViewportId};
use sqyre_capture::{event_log, mark_site, SelectionGrab, SelectionOutline};
use sqyre_hotkeys::ScreenClickBridge;
use sqyre_ports::DesktopRect;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[cfg(target_os = "linux")]
use sqyre_capture::{FrozenFrame, FrozenSelectionOverlay};

const POLL_MS: u64 = 16;
const HUD_ID: &str = "sqyre_recording_coords_hud";
const HUD_MARGIN: f32 = 12.0;
const HUD_PAD_X: f32 = 24.0;
const HUD_PAD_Y: f32 = 16.0;
const HUD_MIN_W: f32 = 200.0;
const HUD_MIN_H: f32 = 36.0;
/// Frame stroke is drawn outside the content; include it in the OS window size.
const HUD_STROKE: f32 = 1.0;
/// Fraction of monitor height used as dead-band so the HUD does not flip every
/// frame when the cursor skims the vertical midline.
const HUD_FLIP_HYSTERESIS: f32 = 0.18;

type OutlineCorners = (i32, i32, i32, i32);

/// Owns the selection grab + outline (UI thread) and a repaint poller for recording HUD.
#[derive(Default)]
pub struct RecordingOverlay {
    stop: Option<Arc<AtomicBool>>,
    join: Option<JoinHandle<()>>,
    /// Created lazily on the UI thread; never touched from the wake poller.
    outline: Option<SelectionOutline>,
    outline_failed: bool,
    /// Fullscreen mouse-owning layer while Point / Color / SearchArea is armed.
    grab: Option<SelectionGrab>,
    grab_failed: bool,
    /// Sticky vertical edge (`true` = top). Cleared when recording ends.
    hud_at_top: Option<bool>,
    /// OS window size for the coords HUD. Frozen after first show so we do not
    /// patch `min_inner_size` while `resizable(false)` pins max — GNOME Mutter
    /// treats min > max as `wl_surface` error 4 ("Invalid min/max size").
    hud_window_size: Option<Vec2>,
    /// Cached so HUD placement does not lock the portal frame mutex every paint
    /// (PipeWire copies ~4K frames under that lock).
    monitor_rects: Vec<DesktopRect>,
    logged_outline_ptr: bool,
    last_x11_ptr: Option<(i32, i32)>,
    last_portal_ptr: Option<(i32, i32)>,
    /// Wayland: frozen screenshot cover over X11/XWayland windows.
    #[cfg(target_os = "linux")]
    snapshot: Option<FrozenSelectionOverlay>,
    #[cfg(target_os = "linux")]
    snapshot_failed: bool,
    /// Freeze kept after the cover unmaps so Find Pixel can sample it.
    #[cfg(target_os = "linux")]
    freeze: Option<FrozenFrame>,
}

impl RecordingOverlay {
    pub fn new() -> Self {
        Self::default()
    }

    /// Call every frame while the app is running.
    ///
    /// Updates the desktop outline for recording selection or tooltip preview,
    /// starts the repaint poller while recording is armed, and shows the coords HUD.
    pub fn sync_with_macro_record(
        &mut self,
        ctx: &egui::Context,
        screen_click: &ScreenClickBridge,
        macro_record: Option<&sqyre_hotkeys::MacroRecordBridge>,
        preview_outline: Option<OutlineCorners>,
        main_window_hidden: bool,
    ) {
        let macro_armed = macro_record.is_some_and(|b| b.is_armed());
        let recording = screen_click.is_armed() || macro_armed;
        self.sync_selection_grab(screen_click);
        #[cfg(target_os = "linux")]
        {
            if !self.sync_linux_snapshot(screen_click) {
                self.sync_linux_pointer(screen_click);
            }
        }
        let selection = screen_click.peek_search_area_selection();
        let rect = selection.or(preview_outline);

        #[cfg(target_os = "linux")]
        if self.snapshot.is_some() {
            self.apply_snapshot_rect(selection);
        } else if rect.is_some() || self.outline.is_some() {
            self.apply_outline(rect);
        }
        #[cfg(not(target_os = "linux"))]
        if rect.is_some() || self.outline.is_some() {
            self.apply_outline(rect);
        }

        if recording {
            self.ensure_wake_poller(ctx.clone(), screen_click.clone(), macro_record.cloned());
            self.show_coords_hud(ctx, screen_click, macro_record, main_window_hidden);
        } else {
            self.hud_at_top = None;
            self.hud_window_size = None;
            self.monitor_rects.clear();
            self.logged_outline_ptr = false;
            self.last_x11_ptr = None;
            self.last_portal_ptr = None;
            #[cfg(target_os = "linux")]
            {
                self.close_snapshot();
                self.snapshot_failed = false;
            }
        }
    }

    /// Arm / poll / disarm the fullscreen grab for any screen-click recording mode.
    fn sync_selection_grab(&mut self, screen_click: &ScreenClickBridge) {
        if !screen_click.is_armed() {
            self.release_selection_grab(screen_click);
            return;
        }

        if skip_x11_pointer_grab() {
            // XGrabPointer on an XWayland InputOnly window stalls GNOME and does
            // not own the Wayland cursor. The frozen snapshot cover owns input
            // when it maps; until then hooks still deliver clicks.
            screen_click.set_grab_owns_input(true);
            #[cfg(target_os = "linux")]
            if self.snapshot.is_none() {
                screen_click.allow_hook_clicks();
            }
            return;
        }

        if self.grab.is_none() && !self.grab_failed {
            mark_site("grab:open");
            match SelectionGrab::open() {
                Ok(mut grab) => match grab.arm() {
                    Ok(()) => {
                        screen_click.set_grab_owns_input(true);
                        self.grab = Some(grab);
                    }
                    Err(e) => {
                        self.grab_failed = true;
                        crate::log::warn(format_args!("selection grab arm failed: {e}"));
                    }
                },
                Err(e) => {
                    self.grab_failed = true;
                    crate::log::warn(format_args!("selection grab unavailable: {e}"));
                }
            }
        }

        let rearm_failed = {
            let Some(grab) = self.grab.as_mut() else {
                return;
            };
            if !grab.is_armed() {
                if let Err(e) = grab.arm() {
                    crate::log::warn(format_args!("selection grab re-arm failed: {e}"));
                    true
                } else {
                    screen_click.set_grab_owns_input(true);
                    false
                }
            } else {
                false
            }
        };
        if rearm_failed {
            self.release_selection_grab(screen_click);
            return;
        }

        let poll = {
            let Some(grab) = self.grab.as_mut() else {
                return;
            };
            grab.poll()
        };
        if poll.moved {
            screen_click.on_mouse_move(poll.x, poll.y);
        }
        for _ in 0..poll.left_clicks {
            screen_click.on_left_click_at(poll.x, poll.y);
        }
        if poll.escape {
            let _ = screen_click.on_escape();
        }

        // Recording completed or cancelled by the poll above.
        if !screen_click.is_armed() {
            self.release_selection_grab(screen_click);
        }
    }

    fn release_selection_grab(&mut self, screen_click: &ScreenClickBridge) {
        if let Some(mut grab) = self.grab.take() {
            mark_site("grab:release");
            grab.disarm();
        }
        screen_click.set_grab_owns_input(false);
        // Allow a later recording session to retry after a transient failure.
        if !screen_click.is_armed() {
            self.grab_failed = false;
        }
    }

    #[cfg(target_os = "linux")]
    fn sync_linux_snapshot(&mut self, screen_click: &ScreenClickBridge) -> bool {
        if !skip_x11_pointer_grab() {
            self.close_snapshot();
            return false;
        }
        if !screen_click.is_armed() {
            self.close_snapshot();
            return false;
        }
        if self.snapshot.is_none() && !self.snapshot_failed {
            mark_site("snapshot:open");
            match FrozenSelectionOverlay::capture_and_open() {
                Ok(overlay) => {
                    self.freeze = None;
                    self.snapshot = Some(overlay);
                    screen_click.set_grab_owns_input(true);
                }
                Err(e) if FrozenSelectionOverlay::capture_retryable(&e) => return false,
                Err(e) => {
                    self.snapshot_failed = true;
                    crate::log::warn(format_args!("frozen snapshot overlay unavailable: {e}"));
                    return false;
                }
            }
        }
        screen_click.set_grab_owns_input(true);
        let poll = {
            let Some(snapshot) = self.snapshot.as_mut() else {
                return false;
            };
            snapshot.poll()
        };
        if poll.moved {
            screen_click.on_mouse_move(poll.x, poll.y);
        }
        for _ in 0..poll.left_clicks {
            screen_click.on_left_click_at(poll.x, poll.y);
        }
        if poll.escape {
            let _ = screen_click.on_escape();
        }
        if self.monitor_rects.is_empty() {
            if let Some(snapshot) = self.snapshot.as_ref() {
                self.monitor_rects = snapshot.virtual_rects();
            }
        }
        true
    }

    #[cfg(target_os = "linux")]
    fn close_snapshot(&mut self) {
        if let Some(overlay) = self.snapshot.take() {
            mark_site("snapshot:close");
            self.freeze = Some(overlay.into_frame());
        }
    }

    #[cfg(target_os = "linux")]
    fn apply_snapshot_rect(&mut self, rect: Option<OutlineCorners>) {
        let Some(snapshot) = self.snapshot.as_mut() else {
            return;
        };
        match rect {
            Some((lx, ty, rx, by)) => snapshot.set_rect(lx, ty, rx, by),
            None => snapshot.clear_rect(),
        }
    }

    /// Sample Find Pixel from the freeze (cover or kept frame). `None` if no freeze.
    #[cfg(target_os = "linux")]
    pub(crate) fn sample_frozen_pixel_hex(&self, x: i32, y: i32) -> Option<String> {
        if let Some(snapshot) = self.snapshot.as_ref() {
            return snapshot.sample_hex(x, y);
        }
        self.freeze.as_ref().and_then(|f| f.sample_hex(x, y))
    }

    /// Merge compositor-absolute pointer sources.
    ///
    /// Portal ScreenCast cursor metadata is compositor-absolute for both native
    /// Wayland and XWayland. `XQueryPointer` over a fullscreen XWayland game is a
    /// blocking round-trip that delays the first outline by seconds and makes
    /// every later frame hitch. Only query X11 when portal position is missing.
    #[cfg(target_os = "linux")]
    fn sync_linux_pointer(&mut self, screen_click: &ScreenClickBridge) {
        if !screen_click.is_armed() {
            return;
        }
        let portal_raw = sqyre_capture::portal_cursor_position();
        let wayland = skip_x11_pointer_grab();
        // Create (and pre-map) edge windows on arm — while the user is still in
        // Sqyre — so the first click on an XWayland game only ConfigureWindow.
        if self.outline.is_none() && !self.outline_failed {
            mark_site("outline:open");
            match SelectionOutline::open() {
                Ok(o) => self.outline = Some(o),
                Err(e) => {
                    self.outline_failed = true;
                    crate::log::warn(format_args!("selection outline unavailable: {e}"));
                }
            }
        }
        if self.monitor_rects.is_empty() {
            if let Some(outline) = self.outline.as_ref() {
                // Xinerama was cached during outline open. Do not call
                // preferred_monitor_rects() here — it locks the PipeWire frame mutex
                // (4K copy) and starves the rubber-band for seconds.
                self.monitor_rects = outline.virtual_rects();
            }
        }
        let portal = portal_raw.map(|(x, y)| clamp_to_desktop(&self.monitor_rects, x, y));
        let x11 = if wayland || portal.is_some() {
            None
        } else if let Some(outline) = self.outline.as_ref() {
            if outline.has_separate_pointer_conn() || self.last_x11_ptr.is_none() {
                outline
                    .query_pointer()
                    .map(|(x, y, _)| clamp_to_desktop(&self.monitor_rects, x, y))
            } else {
                self.last_x11_ptr
            }
        } else {
            None
        };
        if let Some((x, y)) = linux_pointer_snap(portal, self.last_x11_ptr, x11) {
            screen_click.on_mouse_move(x, y);
        }
        self.last_portal_ptr = portal;
        if let Some(pos) = x11 {
            self.last_x11_ptr = Some(pos);
        }
        let (x, y) = screen_click.last_pos();
        let clamped = clamp_to_desktop(&self.monitor_rects, x, y);
        if clamped != (x, y) {
            screen_click.on_mouse_move(clamped.0, clamped.1);
        }
        if !self.logged_outline_ptr {
            self.logged_outline_ptr = true;
            let ptr = screen_click.last_pos();
            let portal_s = self
                .last_portal_ptr
                .map(|(x, y)| format!("{x},{y}"))
                .unwrap_or_else(|| "none".into());
            let (root, x11n, ptr_conn, input) = match self.outline.as_ref() {
                Some(outline) => (
                    outline
                        .root_size()
                        .map(|(w, h)| format!("{w}x{h}"))
                        .unwrap_or_else(|| "unknown".into()),
                    outline.virtual_rects().len(),
                    if outline.has_separate_pointer_conn() {
                        "separate"
                    } else {
                        "shared"
                    },
                    if outline.input_passthrough() {
                        "passthrough"
                    } else {
                        "opaque"
                    },
                ),
                None => ("pending".into(), 0, "none", "pending"),
            };
            event_log(
                "SQYRE_OUTLINE",
                &[
                    ("ptr", &format!("{},{}", ptr.0, ptr.1)),
                    ("portal", &portal_s),
                    ("root", &root),
                    ("x11_outputs", &x11n.to_string()),
                    ("desktop_outputs", &self.monitor_rects.len().to_string()),
                    (
                        "grab",
                        if portal.is_some() {
                            "portal"
                        } else if wayland {
                            "portal-wait"
                        } else {
                            "x11"
                        },
                    ),
                    ("ptr_conn", ptr_conn),
                    ("input", input),
                ],
            );
        }
    }

    fn apply_outline(&mut self, rect: Option<OutlineCorners>) {
        if rect.is_none() {
            if let Some(outline) = self.outline.as_mut() {
                mark_site("outline:clear");
                outline.clear();
            }
            return;
        }
        let Some((lx, ty, rx, by)) = rect else {
            return;
        };
        if self.outline.is_none() && !self.outline_failed {
            mark_site("outline:open");
            match SelectionOutline::open() {
                Ok(o) => self.outline = Some(o),
                Err(e) => {
                    self.outline_failed = true;
                    crate::log::warn(format_args!("selection outline unavailable: {e}"));
                }
            }
        }
        if let Some(outline) = self.outline.as_mut() {
            outline.set_rect(lx, ty, rx, by);
        }
    }

    fn ensure_wake_poller(
        &mut self,
        ctx: egui::Context,
        bridge: ScreenClickBridge,
        macro_record: Option<sqyre_hotkeys::MacroRecordBridge>,
    ) {
        if self.join.is_some() {
            return;
        }
        let stop = Arc::new(AtomicBool::new(false));
        self.stop = Some(Arc::clone(&stop));
        self.join = Some(thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                // Wake the UI loop so outline + HUD update while the main window
                // is hidden for recording. Outline Win32/X11 calls stay on the UI
                // thread via `sync_with_macro_record`.
                let macro_armed = macro_record.as_ref().is_some_and(|b| b.is_armed());
                if bridge.is_armed() || macro_armed {
                    ctx.request_repaint();
                }
                thread::sleep(Duration::from_millis(POLL_MS));
            }
        }));
    }

    fn show_coords_hud(
        &mut self,
        ctx: &egui::Context,
        screen_click: &ScreenClickBridge,
        macro_record: Option<&sqyre_hotkeys::MacroRecordBridge>,
        main_window_hidden: bool,
    ) {
        let text = macro_record
            .and_then(|b| b.status_label())
            .or_else(|| screen_click.status_label());
        let Some(text) = text else {
            return;
        };
        let (mx, my) = match macro_record {
            Some(b) if b.is_armed() => b.last_pos(),
            _ => screen_click.last_pos(),
        };
        if self.monitor_rects.is_empty() {
            self.monitor_rects = sqyre_capture::preferred_monitor_rects();
        }
        let monitor = monitor_for_pointer_in(&self.monitor_rects, mx, my);
        let hud_at_top = pick_hud_edge(self.hud_at_top, my, monitor);
        self.hud_at_top = Some(hud_at_top);

        // Pointer / monitor rects are physical pixels; egui viewport position/size
        // are logical points (`physical / pixels_per_point`).
        let ppp = ctx.pixels_per_point().max(0.01);
        let mon_w_pts = monitor.w as f32 / ppp;
        let max_w = (mon_w_pts - HUD_MARGIN * 2.0).max(HUD_MIN_W);

        let (hud_w, hud_h, panel) = if let Some(size) = self.hud_window_size {
            (
                size.x,
                size.y,
                Vec2::new(
                    (size.x - HUD_STROKE * 2.0).max(1.0),
                    (size.y - HUD_STROKE * 2.0).max(1.0),
                ),
            )
        } else {
            let style = ctx.global_style();
            let font = TextStyle::Body.resolve(&style);
            let color = theme::PRIMARY;
            let text_max_w = (max_w - HUD_PAD_X - HUD_STROKE * 2.0).max(1.0);
            let galley = ctx.fonts_mut(|f| f.layout(text.clone(), font, color, text_max_w));
            let panel_w = (galley.size().x + HUD_PAD_X)
                .ceil()
                .clamp(HUD_MIN_W, (max_w - HUD_STROKE * 2.0).max(HUD_MIN_W));
            let panel_h = (galley.size().y + HUD_PAD_Y).ceil().max(HUD_MIN_H);
            let needed_w = panel_w + HUD_STROKE * 2.0;
            let needed_h = panel_h + HUD_STROKE * 2.0;
            let size = freeze_hud_size(None, Vec2::new(needed_w, needed_h), max_w);
            self.hud_window_size = Some(size);
            mark_site("hud:open");
            event_log(
                "SQYRE_HUD",
                &[
                    ("op", "open"),
                    ("size", &format!("{}x{}", size.x, size.y)),
                    ("ppp", &format!("{ppp:.3}")),
                ],
            );
            (
                size.x,
                size.y,
                Vec2::new(
                    (size.x - HUD_STROKE * 2.0).max(1.0),
                    (size.y - HUD_STROKE * 2.0).max(1.0),
                ),
            )
        };
        // When the main window stays visible on Wayland, paint the HUD in-window
        // (GNOME embeds deferred viewports in the root and ignores `with_position`).
        // After GSR-style hide (`Visible(false)`), use a separate deferred viewport.
        if skip_x11_pointer_grab() && !main_window_hidden {
            paint_hud_window(ctx, &text, hud_at_top);
            return;
        }

        let pos = hud_position(monitor, hud_at_top, hud_w, hud_h, ppp);

        let text_owned = text;
        let id = ViewportId::from_hash_of(HUD_ID);
        let builder = ViewportBuilder::default()
            .with_title("Sqyre recording")
            .with_decorations(false)
            .with_resizable(false)
            .with_always_on_top()
            .with_taskbar(false)
            // Must not steal keyboard focus — otherwise keys are not delivered to the
            // global hook / focused-key feed until the user clicks another window.
            .with_active(false)
            .with_mouse_passthrough(true)
            .with_inner_size([hud_w, hud_h])
            .with_position(pos);

        // Deferred: independent of the (possibly hidden) root viewport paint cycle,
        // as long as the parent keeps registering it each frame via request_repaint.
        ctx.show_viewport_deferred(id, builder, move |ui, class| {
            paint_hud_label(ui, class, &text_owned, hud_at_top, panel);
        });
    }
}

impl Drop for RecordingOverlay {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            stop.store(true, Ordering::Relaxed);
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
        if let Some(mut grab) = self.grab.take() {
            grab.disarm();
        }
        if let Some(mut outline) = self.outline.take() {
            outline.clear();
        }
        #[cfg(target_os = "linux")]
        self.close_snapshot();
    }
}

fn skip_x11_pointer_grab() -> bool {
    #[cfg(target_os = "linux")]
    {
        sqyre_capture::linux::LinuxSessionInfo::detect().has_wayland
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// Absolute compositor snap for this frame.
///
/// Portal wins whenever it exists (XWayland games included). XQueryPointer is
/// only a fallback for when ScreenCast cursor metadata is missing.
fn linux_pointer_snap(
    portal: Option<(i32, i32)>,
    last_x11: Option<(i32, i32)>,
    x11: Option<(i32, i32)>,
) -> Option<(i32, i32)> {
    if portal.is_some() {
        return portal;
    }
    if let Some(pos) = x11 {
        if last_x11 != Some(pos) {
            return Some(pos);
        }
    }
    None
}

/// Clamp to the bounding box of the known outputs so a stuck edge drag cannot
/// walk `last_pos` to ±∞.
fn clamp_to_desktop(rects: &[DesktopRect], x: i32, y: i32) -> (i32, i32) {
    let Some(first) = rects.first() else {
        return (x.max(0), y.max(0));
    };
    let mut min_x = first.x;
    let mut min_y = first.y;
    let mut max_x = first.x.saturating_add(first.w.saturating_sub(1));
    let mut max_y = first.y.saturating_add(first.h.saturating_sub(1));
    for r in rects.iter().skip(1) {
        min_x = min_x.min(r.x);
        min_y = min_y.min(r.y);
        max_x = max_x.max(r.x.saturating_add(r.w.saturating_sub(1)));
        max_y = max_y.max(r.y.saturating_add(r.h.saturating_sub(1)));
    }
    (x.clamp(min_x, max_x), y.clamp(min_y, max_y))
}

fn monitor_for_pointer_in(rects: &[DesktopRect], x: i32, y: i32) -> DesktopRect {
    if let Some(r) = rects
        .iter()
        .find(|r| x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h)
        .copied()
    {
        return r;
    }
    if let Some(r) = rects.first().copied() {
        return r;
    }
    DesktopRect {
        x: 0,
        y: 0,
        w: 1920,
        h: 1080,
    }
}

/// Prefer the edge opposite the cursor, with hysteresis so midline motion is stable.
fn pick_hud_edge(current: Option<bool>, pointer_y: i32, monitor: DesktopRect) -> bool {
    let mid = monitor.y as f32 + monitor.h as f32 * 0.5;
    let band = (monitor.h as f32 * HUD_FLIP_HYSTERESIS).max(1.0);
    // `true` = banner on top (cursor is in the lower portion of the monitor).
    match current {
        Some(true) => (pointer_y as f32) >= mid - band,
        Some(false) => (pointer_y as f32) >= mid + band,
        None => (pointer_y as f32) >= mid,
    }
}

/// First-frame HUD size, then frozen. Growing `min_inner_size` each mouse-move
/// against `resizable(false)` max is a GNOME Mutter protocol error.
fn freeze_hud_size(existing: Option<Vec2>, needed: Vec2, max_w: f32) -> Vec2 {
    if let Some(s) = existing {
        return s;
    }
    Vec2::new(
        (needed.x + 80.0).clamp(HUD_MIN_W, max_w).round().max(1.0),
        needed.y.round().max(HUD_MIN_H),
    )
}

/// Convert a physical-pixel monitor placement into egui points.
fn hud_position(monitor: DesktopRect, at_top: bool, hud_w: f32, hud_h: f32, ppp: f32) -> Pos2 {
    let mon_x = monitor.x as f32 / ppp;
    let mon_y = monitor.y as f32 / ppp;
    let mon_w = monitor.w as f32 / ppp;
    let mon_h = monitor.h as f32 / ppp;
    let x = mon_x + (mon_w - hud_w).max(0.0) * 0.5;
    let y = if at_top {
        mon_y + HUD_MARGIN
    } else {
        mon_y + (mon_h - hud_h - HUD_MARGIN).max(0.0)
    };
    Pos2::new(x, y)
}

fn hud_frame() -> egui::Frame {
    egui::Frame::NONE
        .fill(crate::theme::overlay_panel_fill())
        .stroke(egui::Stroke::new(HUD_STROKE, theme::PRIMARY))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(12, 8))
}

fn paint_hud_window(ctx: &egui::Context, text: &str, at_top: bool) {
    let anchor = if at_top {
        egui::Align2::CENTER_TOP
    } else {
        egui::Align2::CENTER_BOTTOM
    };
    let offset = if at_top {
        [0.0, HUD_MARGIN]
    } else {
        [0.0, -HUD_MARGIN]
    };
    egui::Window::new("Recording")
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .anchor(anchor, offset)
        .frame(hud_frame())
        .show(ctx, |ui| {
            ui.label(egui::RichText::new(text).color(theme::PRIMARY).strong());
        });
}

fn paint_hud_label(ui: &mut egui::Ui, class: ViewportClass, text: &str, at_top: bool, panel: Vec2) {
    let frame = hud_frame();

    if class == ViewportClass::EmbeddedWindow {
        paint_hud_window(ui.ctx(), text, at_top);
        return;
    }

    frame.show(ui, |ui| {
        // Content area inside margins — not the full OS window (stroke lives outside).
        ui.set_min_size(Vec2::new(
            (panel.x - HUD_PAD_X).max(1.0),
            (panel.y - HUD_PAD_Y).max(1.0),
        ));
        ui.centered_and_justified(|ui| {
            ui.label(egui::RichText::new(text).color(theme::PRIMARY).strong());
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hud_position_converts_physical_to_points() {
        let mon = DesktopRect {
            x: 100,
            y: 50,
            w: 1920,
            h: 1080,
        };
        let ppp = 1.7;
        let hud_w = 400.0;
        let hud_h = 40.0;
        let top = hud_position(mon, true, hud_w, hud_h, ppp);
        assert!((top.x - (100.0 / ppp + (1920.0 / ppp - hud_w) * 0.5)).abs() < 0.01);
        assert!((top.y - (50.0 / ppp + HUD_MARGIN)).abs() < 0.01);

        let bottom = hud_position(mon, false, hud_w, hud_h, ppp);
        assert!((bottom.x - top.x).abs() < 0.01);
        let expect_y = 50.0 / ppp + (1080.0 / ppp - hud_h - HUD_MARGIN);
        assert!((bottom.y - expect_y).abs() < 0.01);
    }

    #[test]
    fn freeze_hud_size_does_not_grow_after_first_frame() {
        let first = freeze_hud_size(None, Vec2::new(240.0, 40.0), 800.0);
        assert!((first.x - 320.0).abs() < f32::EPSILON);
        let second = freeze_hud_size(Some(first), Vec2::new(500.0, 40.0), 800.0);
        assert_eq!(second, first);
    }

    #[test]
    fn monitor_for_pointer_in_picks_containing_display() {
        let rects = [
            DesktopRect {
                x: 0,
                y: 0,
                w: 2560,
                h: 1440,
            },
            DesktopRect {
                x: 2560,
                y: 0,
                w: 1920,
                h: 1080,
            },
        ];
        assert_eq!(monitor_for_pointer_in(&rects, 100, 100).w, 2560);
        assert_eq!(monitor_for_pointer_in(&rects, 3000, 10).w, 1920);
        assert_eq!(monitor_for_pointer_in(&[], 0, 0).w, 1920);
    }

    #[test]
    fn pick_hud_edge_uses_hysteresis() {
        let mon = DesktopRect {
            x: 0,
            y: 0,
            w: 1000,
            h: 1000,
        };
        // Mid = 500, band = 180.
        assert!(pick_hud_edge(None, 600, mon)); // lower → top
        assert!(!pick_hud_edge(None, 400, mon)); // upper → bottom

        // Sticky top until cursor moves well into the upper half.
        assert!(pick_hud_edge(Some(true), 400, mon)); // still within band
        assert!(!pick_hud_edge(Some(true), 300, mon)); // past band → flip

        // Sticky bottom until cursor moves well into the lower half.
        assert!(!pick_hud_edge(Some(false), 600, mon));
        assert!(pick_hud_edge(Some(false), 700, mon));
    }

    #[test]
    fn clamp_to_desktop_rejects_negatives_and_past_edge() {
        let rects = [
            DesktopRect {
                x: 0,
                y: 146,
                w: 1920,
                h: 1080,
            },
            DesktopRect {
                x: 1920,
                y: 0,
                w: 2560,
                h: 1440,
            },
        ];
        assert_eq!(clamp_to_desktop(&rects, -400, -20), (0, 0));
        assert_eq!(clamp_to_desktop(&rects, 9000, 50), (4479, 50));
        assert_eq!(clamp_to_desktop(&rects, 3327, 967), (3327, 967));
        assert_eq!(clamp_to_desktop(&[], -3, 10), (0, 10));
    }

    #[test]
    fn linux_pointer_snap_prefers_portal_over_x11() {
        assert_eq!(
            linux_pointer_snap(None, None, Some((10, 20))),
            Some((10, 20))
        );
        assert_eq!(
            linux_pointer_snap(None, Some((10, 20)), Some((10, 20))),
            None
        );
        assert_eq!(
            linux_pointer_snap(Some((30, 40)), Some((10, 20)), Some((10, 20))),
            Some((30, 40))
        );
        assert_eq!(
            linux_pointer_snap(Some((30, 40)), Some((10, 20)), Some((11, 20))),
            Some((30, 40))
        );
    }
}
