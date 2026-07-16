//! Always-on-top floating buttons that enqueue macros by name.
//!
//! Uses deferred egui viewports (same pattern as [`crate::recording_overlay`]) so
//! buttons stay visible while the main window is tray-hidden. Clicks push into the
//! shared pending-macro queue drained by `SqyreApp` each frame.
//!
//! On X11, `with_taskbar(false)` is Windows-only in egui-winit; we use Utility
//! window type plus [`sqyre_capture::skip_taskbar_for_overlay_windows`] so Alt-Tab
//! / pagers omit these tool windows.

use crate::overlay_icons::{self, OverlayIcon};
use eframe::egui::{self, Color32, Pos2, ViewportBuilder, ViewportClass, ViewportId};
use sqyre_capture::{
    get_active_window, mark_site, note, skip_taskbar_for_overlay_windows, window_is_our_process,
    window_matches_binding, window_matches_program, WindowInfo, OVERLAY_WM_TITLE,
};
use sqyre_persist::{
    OverlayButtonConfig, ProgramCatalog, DEFAULT_OVERLAY_BUTTON_SIZE, MAX_OVERLAY_BUTTON_SIZE,
    MIN_OVERLAY_BUTTON_SIZE,
};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};

const VIEWPORT_PAD: f32 = 2.0;
const FOCUS_POLL: Duration = Duration::from_millis(250);
const SKIP_TASKBAR_EVERY: Duration = Duration::from_secs(2);
const FOCUS_ERR_LOG_EVERY: Duration = Duration::from_secs(5);

/// Draws enabled overlay buttons each frame.
pub struct MacroOverlay {
    /// Last focused non-Sqyre window. Kept while an overlay button steals focus so
    /// program-gated buttons are not torn down mid-click (flicker + missed activate).
    last_foreign: Option<WindowInfo>,
    last_skip_taskbar: Option<Instant>,
    last_focus_err_log: Option<Instant>,
    /// Last logged (shown, gated, preview) tuple — avoid flooding stderr notes.
    last_sync_sig: Option<(usize, bool, bool)>,
}

impl Default for MacroOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl MacroOverlay {
    pub fn new() -> Self {
        Self {
            last_foreign: None,
            last_skip_taskbar: None,
            last_focus_err_log: None,
            last_sync_sig: None,
        }
    }

    /// Register deferred always-on-top viewports for each configured button.
    ///
    /// Skipped while screen-click recording is armed. Each button is shown only
    /// when its assigned catalog program matches the focused OS window (empty
    /// program = always show). Prefer the program's bound `process_path` when set;
    /// otherwise fall back to fuzzy catalog-name match.
    ///
    /// When `preview` is set (Data Editor Overlay tab), that button is always drawn
    /// with live form values so the user can see placement and look while editing.
    /// It replaces any saved button with the same id.
    ///
    /// `running_macro` is the name of the macro currently executing (if any); buttons
    /// bound to that name show a spinner over their icon.
    pub fn sync(
        &mut self,
        ctx: &egui::Context,
        enabled: bool,
        buttons: &[OverlayButtonConfig],
        preview: Option<&OverlayButtonConfig>,
        catalog: &ProgramCatalog,
        pending_macros: &Arc<Mutex<Vec<String>>>,
        hide: bool,
        running_macro: Option<&str>,
    ) {
        if hide {
            return;
        }
        if !enabled && preview.is_none() {
            return;
        }

        let focus = self.resolve_focus();
        let preview_id = preview.map(|b| b.id.as_str());
        let mut any_gated = false;
        let mut any_shown = false;
        let mut any_busy = false;
        let mut shown = 0usize;

        if enabled {
            for btn in buttons {
                if preview_id == Some(btn.id.as_str()) {
                    continue;
                }
                if btn.macro_name.trim().is_empty() {
                    continue;
                }
                if !btn.program.trim().is_empty() {
                    any_gated = true;
                    if !program_owns_focus(catalog, &btn.program, focus.as_ref()) {
                        continue;
                    }
                }
                let busy = button_is_busy(btn, running_macro);
                any_busy |= busy;
                show_button_viewport(ctx, btn, Arc::clone(pending_macros), busy);
                any_shown = true;
                shown += 1;
            }
        }

        if let Some(btn) = preview {
            let busy = button_is_busy(btn, running_macro);
            any_busy |= busy;
            show_button_viewport(ctx, btn, Arc::clone(pending_macros), busy);
            any_shown = true;
            shown += 1;
            // Keep the preview viewport updating while the form is edited.
            ctx.request_repaint();
        }

        if any_busy {
            // Spinner animation; root already repaints while running, but keep
            // overlay viewports waking if the main window is tray-hidden.
            ctx.request_repaint();
        }

        let sig = (shown, any_gated, preview.is_some());
        if self.last_sync_sig != Some(sig) {
            self.last_sync_sig = Some(sig);
            let focus_label = focus
                .as_ref()
                .map(|w| {
                    format!(
                        "{} ({})",
                        w.process_name.trim(),
                        w.process_path.trim()
                    )
                })
                .unwrap_or_else(|| "(none)".into());
            note(&format!(
                "overlay: sync shown={shown} gated={any_gated} preview={} focus={focus_label}",
                preview.is_some()
            ));
        }

        if any_shown {
            self.maybe_skip_taskbar();
        }

        if any_gated {
            // Wake periodically so buttons appear when the required program gains focus.
            // Do NOT request_repaint every frame — that flickers transparent X11 windows
            // and was recreating focus-gated viewports aggressively.
            ctx.request_repaint_after(FOCUS_POLL);
        }
    }

    fn resolve_focus(&mut self) -> Option<WindowInfo> {
        match get_active_window() {
            Ok(Some(active)) if window_is_our_process(&active) => self.last_foreign.clone(),
            Ok(Some(active)) => {
                self.last_foreign = Some(active.clone());
                Some(active)
            }
            Ok(None) => self.last_foreign.clone(),
            Err(e) => {
                let now = Instant::now();
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
        }
    }

    fn maybe_skip_taskbar(&mut self) {
        let now = Instant::now();
        if self
            .last_skip_taskbar
            .is_some_and(|t| now.duration_since(t) < SKIP_TASKBAR_EVERY)
        {
            return;
        }
        self.last_skip_taskbar = Some(now);
        mark_site("overlay:skip_taskbar");
        if let Err(e) = skip_taskbar_for_overlay_windows() {
            note(&format!("overlay: skip_taskbar failed: {e}"));
        }
    }
}

fn program_owns_focus(
    catalog: &ProgramCatalog,
    program: &str,
    focus: Option<&WindowInfo>,
) -> bool {
    let program = program.trim();
    if program.is_empty() {
        return true;
    }
    let Some(win) = focus else {
        return false;
    };
    if let Some(data) = catalog.get(program) {
        let path = data.process_path.trim();
        if !path.is_empty() {
            // Path + title when title is bound — shared exes (e.g. GameThread) need both.
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

fn show_button_viewport(
    ctx: &egui::Context,
    btn: &OverlayButtonConfig,
    pending: Arc<Mutex<Vec<String>>>,
    busy: bool,
) {
    // Stable id from settings — must not change per frame or OS windows pile up in Alt-Tab.
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
    let outer = size + VIEWPORT_PAD * 2.0;
    let builder = ViewportBuilder::default()
        // Fixed title so X11 skip-taskbar can find these; avoids N distinct "Sqyre: …" Alt-Tab entries.
        .with_title(OVERLAY_WM_TITLE)
        .with_decorations(false)
        .with_resizable(false)
        .with_always_on_top()
        // Windows: omit from taskbar. X11: see skip_taskbar_for_overlay_windows.
        .with_taskbar(false)
        .with_window_type(egui::X11WindowType::Utility)
        .with_transparent(true)
        .with_inner_size([outer, outer])
        .with_min_inner_size([outer, outer])
        .with_position(Pos2::new(btn.x, btn.y));

    let btn_pos = Pos2::new(btn.x, btn.y);
    ctx.show_viewport_deferred(id, builder, move |ui, class| {
        paint_button(
            ui,
            class,
            icon,
            size,
            btn_pos,
            &macro_name,
            &label,
            &btn_id,
            &pending,
            busy,
        );
    });
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

/// egui's built-in tooltip is clipped to the tiny button OS window; use a sibling viewport.
fn show_overlay_tip_viewport(
    ctx: &egui::Context,
    btn_id: &str,
    tip: &str,
    button_pos: Pos2,
    button_size: f32,
) {
    const TIP_MAX_W: f32 = 280.0;
    const TIP_PAD: f32 = 8.0;
    const TIP_GAP: f32 = 6.0;

    mark_site(&format!("overlay:tip:{btn_id}"));

    let style = ctx.global_style();
    let font_id = egui::TextStyle::Body.resolve(&style);
    let color = style.visuals.text_color();
    let galley = ctx.fonts_mut(|f| f.layout(tip.to_owned(), font_id, color, TIP_MAX_W));
    let tip_w = (galley.size().x + TIP_PAD * 2.0).ceil().max(48.0);
    let tip_h = (galley.size().y + TIP_PAD * 2.0).ceil().max(28.0);
    let tip_pos = Pos2::new(
        button_pos.x + button_size + VIEWPORT_PAD * 2.0 + TIP_GAP,
        button_pos.y,
    );

    let id = ViewportId::from_hash_of(format!("sqyre_macro_overlay_tip_{btn_id}"));
    let tip = tip.to_owned();
    let tip_window_id = format!("overlay-tip-{btn_id}");
    let builder = ViewportBuilder::default()
        .with_title(OVERLAY_WM_TITLE)
        .with_decorations(false)
        .with_resizable(false)
        .with_always_on_top()
        .with_taskbar(false)
        .with_window_type(egui::X11WindowType::Tooltip)
        .with_transparent(true)
        .with_inner_size([tip_w, tip_h])
        .with_min_inner_size([tip_w, tip_h])
        .with_position(tip_pos);

    ctx.show_viewport_deferred(id, builder, move |ui, class| {
        let frame = egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(20, 18, 14, 230))
            .stroke(egui::Stroke::new(1.0, crate::theme::PRIMARY))
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
}

fn paint_button(
    ui: &mut egui::Ui,
    class: ViewportClass,
    icon: &OverlayIcon,
    size: f32,
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
        let resp = overlay_icons::paint_glyph_bare(ui, icon, size, busy);
        let clicked = resp.clicked();
        // Wake the root viewport so pending macros are drained (child request_repaint
        // alone does not run App::update).
        if class == ViewportClass::EmbeddedWindow {
            // Embedded in the root window — built-in tooltip has room to draw.
            resp.on_hover_text(&tip);
        } else if resp.hovered() {
            show_overlay_tip_viewport(ui.ctx(), btn_id, &tip, button_pos, size);
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
        .inner_margin(egui::Margin::same(VIEWPORT_PAD as i8))
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
