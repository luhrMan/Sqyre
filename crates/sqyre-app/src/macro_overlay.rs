//! Always-on-top floating buttons that enqueue macros by name.
//!
//! Uses deferred egui viewports (same pattern as [`crate::recording_overlay`]) so
//! buttons stay visible while the main window is tray-hidden. Clicks push into the
//! shared pending-macro queue drained by `SqyreApp` each frame.
//!
//! On X11, `with_taskbar(false)` is Windows-only in egui-winit. Overlay buttons use
//! Notification window type plus
//! [`sqyre_capture::skip_taskbar_for_overlay_windows`] (`SKIP_TASKBAR` / `SKIP_PAGER`).
//! Dock type is avoided: Mutter treats docks like panels and can autohide / unredirect
//! them away under fullscreen games on GNOME Wayland.
//!
//! Tooltips use a sibling deferred viewport (`OVERLAY_TIP_WM_TITLE`) toggled with
//! `Visible` — not button `InnerSize` (that fights `ViewportBuilder::patch` and
//! phases the button), and not the button WM title (geom sync would move tips).

use crate::overlay_icons::{self, OverlayIcon};
use eframe::egui::{self, Color32, Pos2, ViewportBuilder, ViewportClass, ViewportId};
use parking_lot::Mutex;
use sqyre_capture::{
    enable_overlay_window_transparency, get_active_window, mark_site, note,
    skip_taskbar_for_overlay_windows, sync_overlay_window_geometry, window_is_our_process,
    window_is_transient_shell_focus, window_matches_binding, window_matches_program, WindowInfo,
    OVERLAY_TIP_WM_TITLE, OVERLAY_WM_TITLE,
};
use sqyre_persist::{
    OverlayButtonConfig, ProgramCatalog, DEFAULT_OVERLAY_BUTTON_SIZE, GENERAL_PROGRAM,
    MAX_OVERLAY_BUTTON_SIZE, MIN_OVERLAY_BUTTON_SIZE,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use web_time::{Duration, Instant};

const VIEWPORT_PAD_MIN: f32 = 2.0;
/// How often to re-query OS focus while gated buttons may appear/disappear.
const FOCUS_POLL: Duration = Duration::from_millis(250);
/// Once a foreign (e.g. XWayland game) window owns focus and buttons are shown,
/// poll less often — X11 focus IPC under fullscreen games hitch the UI thread.
const FOCUS_POLL_WHILE_SHOWN: Duration = Duration::from_millis(1000);
/// Spinner wake interval from the background poller.
const OVERLAY_ANIM_POLL: Duration = Duration::from_millis(16);
/// Re-apply skip-taskbar / transparency rarely. Doing this every focus poll
/// floods Mutter with `_NET_WM_STATE` ClientMessages and disk `mark_site`s —
/// overlays feel snappy for ~1s then degrade.
const OVERLAY_HINT_EVERY: Duration = Duration::from_secs(5);
const FOCUS_ERR_LOG_EVERY: Duration = Duration::from_secs(5);
/// Log overlay phase timings at most this often (needs `SQYRE_DIAG=1` env).
const OVERLAY_TIMING_LOG_EVERY: Duration = Duration::from_secs(2);

const TIP_MAX_W: f32 = 280.0;
const TIP_PAD: f32 = 8.0;
const TIP_GAP: f32 = 6.0;
const TIP_STROKE: f32 = 1.0;

/// Draws enabled overlay buttons each frame.
pub struct MacroOverlay {
    /// Last focused non-Sqyre window. Kept while an overlay button steals focus so
    /// program-gated buttons are not torn down mid-click (flicker + missed activate).
    last_foreign: Option<WindowInfo>,
    /// Cached result of [`Self::resolve_focus`] — `get_active_window` is expensive on
    /// GNOME Wayland (AT-SPI / foreign-toplevel connect per call).
    cached_focus: Option<WindowInfo>,
    last_focus_poll: Option<Instant>,
    last_skip_taskbar: Option<Instant>,
    last_geom_sync: Option<Instant>,
    last_focus_err_log: Option<Instant>,
    last_timing_log: Option<Instant>,
    /// Last logged (shown, gated, preview) tuple — avoid flooding stderr notes.
    last_sync_sig: Option<(usize, bool, bool)>,
    /// Buttons visible last sync — stretch focus poll while an X11 game holds focus.
    last_shown: usize,
    /// Last geom hints that were applied — skip XMove when unchanged.
    last_geom_hints: Vec<(String, i32, i32, u32, u32)>,
    /// Wayland: match XWayland overlay windows to buttons after coordinate edits.
    overlay_last_positions: HashMap<String, (i32, i32)>,
    /// Background wake for busy overlay viewports (spinner must not wait on ROOT).
    busy_wake: Mutex<Option<(Arc<AtomicBool>, JoinHandle<()>)>>,
    busy_viewport_ids: Arc<Mutex<Vec<ViewportId>>>,
}

impl Default for MacroOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MacroOverlay {
    fn drop(&mut self) {
        self.stop_busy_wake();
    }
}

impl MacroOverlay {
    pub fn new() -> Self {
        Self {
            last_foreign: None,
            cached_focus: None,
            last_focus_poll: None,
            last_skip_taskbar: None,
            last_geom_sync: None,
            last_focus_err_log: None,
            last_timing_log: None,
            last_sync_sig: None,
            last_shown: 0,
            last_geom_hints: Vec::new(),
            overlay_last_positions: HashMap::new(),
            busy_wake: Mutex::new(None),
            busy_viewport_ids: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Register deferred always-on-top viewports for each configured button.
    ///
    /// Each enabled button is shown when its assigned catalog program matches the
    /// focused OS window. Empty program and [`GENERAL_PROGRAM`] buttons always show.
    /// Prefer the program's bound `process_path` when set; otherwise fall back to
    /// fuzzy catalog-name match.
    ///
    /// When `preview` is set (Data Editor Overlay tab), that button is always drawn
    /// with live form values so the user can see placement and look while editing.
    /// It replaces any saved button with the same id.
    ///
    /// `running_macro` is the name of the macro currently executing (if any); buttons
    /// bound to that name show a spinner over their icon.
    ///
    /// Callers should skip this while screen-click recording is armed.
    pub fn sync(
        &mut self,
        ctx: &egui::Context,
        buttons: &[OverlayButtonConfig],
        preview: Option<&OverlayButtonConfig>,
        catalog: &ProgramCatalog,
        pending_macros: &Arc<Mutex<Vec<String>>>,
        running_macro: Option<&str>,
    ) {
        let any_enabled = buttons
            .iter()
            .any(|b| b.enabled && !b.macro_name.trim().is_empty());
        if preview.is_none() && !any_enabled {
            self.stop_busy_wake();
            return;
        }
        let sync_t0 = Instant::now();
        let focus = self.resolve_focus();
        let focus_ms = sync_t0.elapsed().as_secs_f32() * 1000.0;
        let preview_id = preview.map(|b| b.id.as_str());
        let mut any_gated = false;
        let mut any_shown = false;
        let mut any_busy = false;
        let mut shown = 0usize;
        let mut busy_ids = Vec::new();
        let mut overlay_geom_hints: Vec<(String, i32, i32, u32, u32)> = Vec::new();
        let ppp = ctx.pixels_per_point().max(0.01);
        let wayland_overlay_geom = wayland_overlay_needs_x11_geometry();

        for btn in buttons {
            if preview_id == Some(btn.id.as_str()) {
                continue;
            }
            if !btn.enabled {
                continue;
            }
            if btn.macro_name.trim().is_empty() {
                continue;
            }
            if button_is_focus_gated(btn) {
                any_gated = true;
                if !program_owns_focus(catalog, &btn.program, focus.as_ref()) {
                    continue;
                }
            }
            let busy = button_is_busy(btn, running_macro);
            any_busy |= busy;
            let mut drawn = btn.clone();
            let (x, y) = btn.resolved_position(catalog);
            drawn.x = x;
            drawn.y = y;
            let id = show_button_viewport(
                ctx,
                &drawn,
                Arc::clone(pending_macros),
                busy,
                wayland_overlay_geom,
            );
            if busy {
                busy_ids.push(id);
            }
            push_overlay_geom_hint(&mut overlay_geom_hints, &drawn, ppp);
            any_shown = true;
            shown += 1;
        }

        if let Some(btn) = preview {
            let busy = button_is_busy(btn, running_macro);
            any_busy |= busy;
            let mut drawn = btn.clone();
            let (x, y) = btn.resolved_position(catalog);
            drawn.x = x;
            drawn.y = y;
            let id = show_button_viewport(
                ctx,
                &drawn,
                Arc::clone(pending_macros),
                busy,
                wayland_overlay_geom,
            );
            if busy {
                busy_ids.push(id);
            }
            push_overlay_geom_hint(&mut overlay_geom_hints, &drawn, ppp);
            any_shown = true;
            shown += 1;
            ctx.request_repaint();
        }

        self.set_busy_wake(ctx, busy_ids);
        self.last_shown = shown;

        if any_busy {
            // Re-register deferred callbacks on a short cadence; spinner frames come
            // from the busy-wake thread + the button viewport itself.
            ctx.request_repaint_after(FOCUS_POLL);
        }

        let sig = (shown, any_gated, preview.is_some());
        if self.last_sync_sig != Some(sig) {
            self.last_sync_sig = Some(sig);
            let focus_label = focus
                .as_ref()
                .map(|w| format!("{} ({})", w.process_name.trim(), w.process_path.trim()))
                .unwrap_or_else(|| "(none)".into());
            note(&format!(
                "overlay: sync shown={shown} gated={any_gated} preview={} focus={focus_label}",
                preview.is_some()
            ));
            if shown == 0 && any_gated {
                if let Some(lf) = &self.last_foreign {
                    note(&format!(
                        "overlay: gated hidden last_foreign={} ({})",
                        lf.process_name.trim(),
                        lf.process_path.trim()
                    ));
                } else {
                    note("overlay: gated hidden last_foreign=(none)");
                }
            }
        }

        let mut hint_ms = 0.0f32;
        let mut geom_ms = 0.0f32;
        if any_shown {
            let t = Instant::now();
            self.maybe_hint_overlay_windows();
            hint_ms = t.elapsed().as_secs_f32() * 1000.0;
            if wayland_overlay_geom {
                let t = Instant::now();
                self.maybe_sync_overlay_geometry(&overlay_geom_hints);
                geom_ms = t.elapsed().as_secs_f32() * 1000.0;
            }
        }

        let total_ms = sync_t0.elapsed().as_secs_f32() * 1000.0;
        // Only log when a phase is actually slow — periodic logging alone was noise.
        if any_shown && (total_ms >= 4.0 || hint_ms >= 2.0 || geom_ms >= 2.0 || focus_ms >= 2.0)
        {
            let should_log = self
                .last_timing_log
                .is_none_or(|t| sync_t0.duration_since(t) >= OVERLAY_TIMING_LOG_EVERY);
            if should_log {
                self.last_timing_log = Some(sync_t0);
                note(&format!(
                    "overlay: timing total={total_ms:.1}ms focus={focus_ms:.1}ms hint={hint_ms:.1}ms geom={geom_ms:.1}ms shown={shown} busy={any_busy}"
                ));
            }
        }

        if any_gated && !any_busy {
            // Do NOT request_repaint every frame — that flickers transparent X11 windows.
            let wake = if shown > 0 {
                FOCUS_POLL_WHILE_SHOWN
            } else {
                FOCUS_POLL
            };
            ctx.request_repaint_after(wake);
        }
    }

    fn set_busy_wake(&self, ctx: &egui::Context, ids: Vec<ViewportId>) {
        *self.busy_viewport_ids.lock() = ids;
        if self.busy_viewport_ids.lock().is_empty() {
            self.stop_busy_wake();
            return;
        }
        let mut slot = self.busy_wake.lock();
        if slot.is_some() {
            return;
        }
        let stop = Arc::new(AtomicBool::new(false));
        let wake = ctx.clone();
        let ids = Arc::clone(&self.busy_viewport_ids);
        let stop_flag = Arc::clone(&stop);
        let join = thread::spawn(move || {
            while !stop_flag.load(Ordering::Relaxed) {
                let snapshot = ids.lock().clone();
                if snapshot.is_empty() {
                    break;
                }
                for id in snapshot {
                    wake.request_repaint_of(id);
                }
                thread::sleep(OVERLAY_ANIM_POLL);
            }
        });
        *slot = Some((stop, join));
    }

    fn stop_busy_wake(&self) {
        let mut slot = self.busy_wake.lock();
        if let Some((stop, join)) = slot.take() {
            stop.store(true, Ordering::Relaxed);
            let _ = join.join();
        }
        self.busy_viewport_ids.lock().clear();
    }

    fn resolve_focus(&mut self) -> Option<WindowInfo> {
        let now = Instant::now();
        let poll = if self.last_shown > 0 {
            FOCUS_POLL_WHILE_SHOWN
        } else {
            FOCUS_POLL
        };
        if self
            .last_focus_poll
            .is_some_and(|t| now.duration_since(t) < poll)
        {
            return self.cached_focus.clone();
        }
        self.last_focus_poll = Some(now);
        let t0 = Instant::now();
        let focus = match get_active_window() {
            Ok(Some(active))
                if window_is_our_process(&active) || window_is_transient_shell_focus(&active) =>
            {
                self.last_foreign.clone()
            }
            Ok(Some(active)) => {
                self.last_foreign = Some(active.clone());
                Some(active)
            }
            Ok(None) => self.last_foreign.clone(),
            Err(e) => {
                let should_log = self
                    .last_focus_err_log
                    .map(|t| now.duration_since(t) >= FOCUS_ERR_LOG_EVERY)
                    .unwrap_or(true);
                if should_log {
                    self.last_focus_err_log = Some(now);
                    note(&format!("overlay: get_active_window failed: {e}"));
                }
                self.last_foreign.clone()
            }
        };
        let focus_ms = t0.elapsed().as_secs_f32() * 1000.0;
        if focus_ms >= 5.0 {
            note(&format!("overlay: focus_poll slow={focus_ms:.1}ms"));
        }
        self.cached_focus = focus.clone();
        focus
    }

    fn maybe_hint_overlay_windows(&mut self) {
        let now = Instant::now();
        if self
            .last_skip_taskbar
            .is_some_and(|t| now.duration_since(t) < OVERLAY_HINT_EVERY)
        {
            return;
        }
        self.last_skip_taskbar = Some(now);
        // No mark_site here — disk flush every few hundred ms was starving spinner/hover.
        if let Err(e) = skip_taskbar_for_overlay_windows() {
            note(&format!("overlay: skip_taskbar failed: {e}"));
        }
        if let Err(e) = enable_overlay_window_transparency() {
            note(&format!("overlay: enable transparency failed: {e}"));
        }
    }

    fn maybe_sync_overlay_geometry(&mut self, hints: &[(String, i32, i32, u32, u32)]) {
        // Only nudge when button positions change (or first apply).
        if hints == self.last_geom_hints.as_slice() && self.last_geom_sync.is_some() {
            return;
        }
        self.last_geom_sync = Some(Instant::now());
        self.last_geom_hints = hints.to_vec();
        if let Err(e) = sync_overlay_window_geometry(hints, &mut self.overlay_last_positions) {
            note(&format!("overlay: sync geometry failed: {e}"));
        }
    }
}

/// Empty program and General stay on screen; other programs require focus match.
fn button_is_focus_gated(btn: &OverlayButtonConfig) -> bool {
    let program = btn.program.trim();
    !program.is_empty() && program != GENERAL_PROGRAM
}

fn program_owns_focus(catalog: &ProgramCatalog, program: &str, focus: Option<&WindowInfo>) -> bool {
    let program = program.trim();
    if program.is_empty() || program == GENERAL_PROGRAM {
        return true;
    }
    let Some(win) = focus else {
        return false;
    };
    if let Some(data) = catalog.get(program) {
        let path = data.process_path.trim();
        if !path.is_empty() {
            return window_matches_binding(win, path, &data.window_title);
        }
    }
    window_matches_program(win, program)
}

fn button_is_busy(btn: &OverlayButtonConfig, running_macro: Option<&str>) -> bool {
    let Some(running) = running_macro.map(str::trim).filter(|s| !s.is_empty()) else {
        return false;
    };
    btn.macro_name.trim() == running
}

fn wayland_overlay_needs_x11_geometry() -> bool {
    #[cfg(target_os = "linux")]
    {
        sqyre_capture::linux::LinuxSessionInfo::detect().has_wayland
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

fn push_overlay_geom_hint(
    hints: &mut Vec<(String, i32, i32, u32, u32)>,
    btn: &OverlayButtonConfig,
    ppp: f32,
) {
    let size = if btn.size > 0.0 {
        btn.size
    } else {
        DEFAULT_OVERLAY_BUTTON_SIZE
    }
    .clamp(MIN_OVERLAY_BUTTON_SIZE, MAX_OVERLAY_BUTTON_SIZE);
    let style = overlay_icons::OverlayPaintStyle::from_config(btn);
    let pad = VIEWPORT_PAD_MIN.max(style.border_width * (2.0 / 1.5) + 1.0);
    let outer = size + pad * 2.0;
    let phys_outer = (outer * ppp).round().max(1.0);
    hints.push((
        btn.id.clone(),
        btn.x.round() as i32,
        btn.y.round() as i32,
        phys_outer as u32,
        phys_outer as u32,
    ));
}

fn show_button_viewport(
    ctx: &egui::Context,
    btn: &OverlayButtonConfig,
    pending: Arc<Mutex<Vec<String>>>,
    busy: bool,
    wayland_overlay_geom: bool,
) -> ViewportId {
    let id = ViewportId::from_hash_of(format!("sqyre_macro_overlay_{}", btn.id));
    let icon = overlay_icons::resolve(&btn.icon);
    let macro_name = btn.macro_name.clone();
    let label = btn.label.clone();
    let btn_id = btn.id.clone();
    let size = if btn.size > 0.0 {
        btn.size
    } else {
        DEFAULT_OVERLAY_BUTTON_SIZE
    }
    .clamp(MIN_OVERLAY_BUTTON_SIZE, MAX_OVERLAY_BUTTON_SIZE);
    let style = overlay_icons::OverlayPaintStyle::from_config(btn);
    let pad = VIEWPORT_PAD_MIN.max(style.border_width * (2.0 / 1.5) + 1.0);
    let outer = size + pad * 2.0;
    let ppp = ctx.pixels_per_point().max(0.01);
    let btn_pos = Pos2::new(btn.x / ppp, btn.y / ppp);
    let builder = ViewportBuilder::default()
        .with_title(OVERLAY_WM_TITLE)
        .with_decorations(false)
        .with_resizable(false)
        .with_always_on_top()
        .with_taskbar(false)
        .with_window_type(egui::X11WindowType::Notification)
        .with_transparent(true)
        .with_inner_size([outer, outer])
        .with_min_inner_size([outer, outer])
        .with_position(btn_pos);

    ctx.show_viewport_deferred(id, builder, move |ui, class| {
        paint_button(
            ui,
            class,
            icon,
            size,
            style,
            pad,
            btn_pos,
            &macro_name,
            &label,
            &btn_id,
            &pending,
            busy,
        );
    });

    if busy {
        ctx.request_repaint_of(id);
    }

    if !wayland_overlay_geom {
        ctx.send_viewport_cmd_to(id, egui::ViewportCommand::OuterPosition(btn_pos));
    }
    id
}

fn overlay_tip_text(macro_name: &str, label: &str) -> String {
    if macro_name.trim().is_empty() {
        if label.is_empty() {
            "Overlay button (no macro yet)".to_string()
        } else {
            format!("{label}\n(no macro yet)")
        }
    } else if label.is_empty() {
        format!("Run macro: {macro_name}")
    } else {
        format!("{label}\nRun macro: {macro_name}")
    }
}

fn overlay_tip_viewport_id(btn_id: &str) -> ViewportId {
    ViewportId::from_hash_of(format!("sqyre_macro_overlay_tip_{btn_id}"))
}

/// Sibling tip window. Kept registered while the button is shown; visibility toggles
/// without create/destroy. Size stays stable (cached) so `ViewportBuilder::patch`
/// does not thrash `InnerSize` on every hover edge.
fn show_overlay_tip_viewport(
    ctx: &egui::Context,
    btn_id: &str,
    tip: &str,
    button_pos: Pos2,
    button_size: f32,
    visible: bool,
) {
    let id = overlay_tip_viewport_id(btn_id);
    let size_key = egui::Id::new(("sqyre_overlay_tip_size", btn_id));
    let vis_key = egui::Id::new(("sqyre_overlay_tip_vis", btn_id));

    let (tip_w, tip_h) = if visible {
        let style = ctx.global_style();
        let font_id = egui::TextStyle::Body.resolve(&style);
        let color = style.visuals.text_color();
        let galley = ctx.fonts_mut(|f| f.layout(tip.to_owned(), font_id, color, TIP_MAX_W));
        let tip_w = (galley.size().x + TIP_PAD * 2.0 + TIP_STROKE * 2.0)
            .ceil()
            .max(48.0);
        let tip_h = (galley.size().y + TIP_PAD * 2.0 + TIP_STROKE * 2.0)
            .ceil()
            .max(28.0);
        ctx.data_mut(|d| d.insert_temp(size_key, (tip_w, tip_h)));
        (tip_w, tip_h)
    } else {
        ctx.data(|d| d.get_temp(size_key).unwrap_or((120.0, 40.0)))
    };

    let tip_pos = Pos2::new(
        button_pos.x + button_size + VIEWPORT_PAD_MIN * 2.0 + TIP_GAP,
        button_pos.y,
    );

    let tip = tip.to_owned();
    let tip_window_id = format!("overlay-tip-{btn_id}");
    let builder = ViewportBuilder::default()
        // Distinct from button title so X11 geom sync never moves tip windows.
        .with_title(OVERLAY_TIP_WM_TITLE)
        .with_decorations(false)
        .with_resizable(false)
        .with_always_on_top()
        .with_taskbar(false)
        .with_active(false)
        .with_mouse_passthrough(true)
        .with_visible(visible)
        .with_window_type(egui::X11WindowType::Tooltip)
        .with_transparent(true)
        .with_inner_size([tip_w, tip_h])
        .with_min_inner_size([tip_w, tip_h])
        .with_position(tip_pos);

    ctx.show_viewport_deferred(id, builder, move |ui, class| {
        if !visible {
            return;
        }
        let frame = egui::Frame::NONE
            .fill(crate::theme::overlay_panel_fill())
            .stroke(egui::Stroke::new(TIP_STROKE, crate::theme::PRIMARY))
            .corner_radius(egui::CornerRadius::same(4))
            .inner_margin(egui::Margin::same(TIP_PAD as i8));

        if class == ViewportClass::EmbeddedWindow {
            egui::Window::new(tip_window_id.clone())
                .collapsible(false)
                .resizable(false)
                .title_bar(false)
                .frame(frame)
                .show(ui.ctx(), |ui| {
                    ui.set_max_width(TIP_MAX_W);
                    ui.label(&tip);
                });
            return;
        }

        frame.show(ui, |ui| {
            ui.set_max_width(TIP_MAX_W);
            ui.label(&tip);
        });
    });

    // Only emit Visible when it changes — every-frame Visible(false) while spinning
    // was thrashing XWayland map/unmap and making overlays degrade within seconds.
    let was_visible = ctx.data(|d| d.get_temp::<bool>(vis_key).unwrap_or(false));
    if was_visible != visible {
        ctx.data_mut(|d| d.insert_temp(vis_key, visible));
        ctx.send_viewport_cmd_to(id, egui::ViewportCommand::Visible(visible));
        if visible {
            ctx.request_repaint_of(id);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_button(
    ui: &mut egui::Ui,
    class: ViewportClass,
    icon: &OverlayIcon,
    size: f32,
    style: overlay_icons::OverlayPaintStyle,
    pad: f32,
    button_pos: Pos2,
    macro_name: &str,
    label: &str,
    btn_id: &str,
    pending: &Arc<Mutex<Vec<String>>>,
    busy: bool,
) {
    let tip = if busy {
        let base = overlay_tip_text(macro_name, label);
        format!("{base}\n(running…)")
    } else {
        overlay_tip_text(macro_name, label)
    };

    let paint = |ui: &mut egui::Ui| {
        let resp = overlay_icons::paint_glyph_bare(ui, icon, size, busy, &style);
        let clicked = resp.clicked();
        let hovered = resp.hovered();

        let hover_key = egui::Id::new(("sqyre_overlay_hover", btn_id));
        let was_hovered = ui
            .ctx()
            .data(|d| d.get_temp::<bool>(hover_key).unwrap_or(false));
        ui.ctx().data_mut(|d| d.insert_temp(hover_key, hovered));

        if busy || hovered || hovered != was_hovered {
            // Busy: wake this viewport only (ROOT drain is on its own cadence).
            ui.ctx().request_repaint();
        }

        if class == ViewportClass::EmbeddedWindow {
            resp.on_hover_text(&tip);
        } else if !busy {
            // Skip tip registration while spinning — builder/Visible work starved frames.
            show_overlay_tip_viewport(ui.ctx(), btn_id, &tip, button_pos, size, hovered);
        }

        if clicked && !busy && !macro_name.trim().is_empty() {
            enqueue(pending, btn_id, macro_name);
            ui.ctx().request_repaint_of(ViewportId::ROOT);
        }
    };

    if class == ViewportClass::EmbeddedWindow {
        egui::Window::new(format!("overlay-{macro_name}"))
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .frame(egui::Frame::NONE)
            .show(ui.ctx(), |ui| paint(ui));
        return;
    }

    egui::Frame::NONE
        .fill(Color32::TRANSPARENT)
        .inner_margin(egui::Margin::same(pad.round() as i8))
        .show(ui, |ui| {
            ui.set_min_size(egui::vec2(size, size));
            paint(ui);
        });
}

fn enqueue(pending: &Arc<Mutex<Vec<String>>>, btn_id: &str, macro_name: &str) {
    mark_site(&format!("overlay:click:{btn_id}"));
    pending.lock().push(macro_name.to_string());
    note(&format!(
        "overlay: click id={btn_id} enqueue macro={macro_name}"
    ));
}
