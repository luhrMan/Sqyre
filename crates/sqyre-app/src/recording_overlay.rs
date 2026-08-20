//! Screen-click selection outline, mouse-owning grab, and recording coords HUD.
//!
//! Driven by [`sqyre_hotkeys::ScreenClickBridge`] and tooltip preview requests:
//! - A fullscreen OS grab ([`sqyre_capture::SelectionGrab`]) that takes the pointer
//!   while Point / Color / SearchArea recording is armed, so games that confine or
//!   relative-capture the mouse cannot block selection. Do **not** use an eframe
//!   fullscreen viewport for this on GNOME/Wayland: Mutter un-redirects those
//!   surfaces and they paint as opaque black.
//! - OS edge windows ([`sqyre_capture::SelectionOutline`]) for the live search-area
//!   rect or a hovered point / search-area preview — not a fullscreen desktop
//!   snapshot (X11 on Linux, Win32 popups on Windows).
//! - A small always-on-top egui viewport for live coords / status while recording
//!   (needed when the main window is hidden via `hide_app_during_recording`).
//!   The HUD sits on the opposite vertical edge of the monitor from the cursor so
//!   it stays out of the way while pointing / selecting.
//!
//! Outline / grab HWNDs and X11 windows are updated on the UI thread only. A short
//! poller only `request_repaint`s while recording is armed so the HUD keeps updating
//! when the root viewport is `Visible(false)`. Driving Win32 outline windows from a
//! background thread while glow paints preview textures hard-crashed on Windows
//! (no Rust panic / `crash.log`).

use crate::theme;
use eframe::egui::{self, Pos2, TextStyle, Vec2, ViewportBuilder, ViewportClass, ViewportId};
use sqyre_capture::{event_log, mark_site, SelectionGrab, SelectionOutline};
use sqyre_hotkeys::ScreenClickBridge;
use sqyre_ports::DesktopRect;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

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
    ) {
        let macro_armed = macro_record.is_some_and(|b| b.is_armed());
        let recording = screen_click.is_armed() || macro_armed;
        self.sync_selection_grab(screen_click);
        #[cfg(target_os = "linux")]
        self.sync_x11_pointer(screen_click);
        let rect = screen_click
            .peek_search_area_selection()
            .or(preview_outline);

        if rect.is_some() || self.outline.is_some() {
            self.apply_outline(rect);
        }

        if recording {
            self.ensure_wake_poller(ctx.clone(), screen_click.clone(), macro_record.cloned());
            self.show_coords_hud(ctx, screen_click, macro_record);
        } else {
            self.hud_at_top = None;
            self.hud_window_size = None;
            self.monitor_rects.clear();
            self.logged_outline_ptr = false;
            self.last_x11_ptr = None;
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
            // not own the Wayland cursor. Track via XQueryPointer + evdev deltas.
            screen_click.set_grab_owns_input(true);
            screen_click.allow_hook_clicks();
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
            screen_click.on_left_click();
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

    /// Sample the X11 root pointer (same connection as the outline).
    #[cfg(target_os = "linux")]
    fn sync_x11_pointer(&mut self, screen_click: &ScreenClickBridge) {
        if !screen_click.is_armed() {
            return;
        }
        if self.outline.is_none() && !self.outline_failed {
            mark_site("outline:open");
            match SelectionOutline::open() {
                Ok(o) => self.outline = Some(o),
                Err(e) => {
                    self.outline_failed = true;
                    crate::log::warn(format_args!("selection outline unavailable: {e}"));
                    return;
                }
            }
        }
        let Some(outline) = self.outline.as_mut() else {
            return;
        };
        let Some((x, y, _left)) = outline.query_pointer() else {
            return;
        };
        // Only trust X11 when it actually moved. On the other GNOME output the
        // cursor is not in XWayland, so QueryPointer sticks and evdev deltas win.
        if self.last_x11_ptr != Some((x, y)) {
            screen_click.on_mouse_move(x, y);
            self.last_x11_ptr = Some((x, y));
        }
        if self.monitor_rects.is_empty() {
            let mut rects = outline.virtual_rects();
            let portal = sqyre_capture::preferred_monitor_rects();
            if portal.iter().map(|r| r.w).sum::<i32>() > rects.iter().map(|r| r.w).sum::<i32>() {
                rects = portal;
            }
            self.monitor_rects = rects;
        }
        if !self.logged_outline_ptr {
            self.logged_outline_ptr = true;
            let root = outline
                .root_size()
                .map(|(w, h)| format!("{w}x{h}"))
                .unwrap_or_else(|| "unknown".into());
            let x11n = outline.virtual_rects().len();
            event_log(
                "SQYRE_OUTLINE",
                &[
                    ("ptr", &format!("{x},{y}")),
                    ("root", &root),
                    ("x11_outputs", &x11n.to_string()),
                    ("desktop_outputs", &self.monitor_rects.len().to_string()),
                    (
                        "grab",
                        if skip_x11_pointer_grab() {
                            "xquery+evdev"
                        } else {
                            "x11"
                        },
                    ),
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
        // GNOME embeds deferred viewports in the root window and ignores
        // `with_position`. Prefer an in-window banner over a second OS surface.
        if skip_x11_pointer_grab() {
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
}
