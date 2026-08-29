//! Floating overlay buttons that enqueue macros by name.
//!
//! **Linux:** native X11 override-redirect windows on a dedicated event thread
//! ([`crate::x11_buttons`]). Clicks are handled there — they do not wait for the
//! egui ROOT frame (which starved under fullscreen XWayland GameThread on GNOME).
//!
//! **Other OS:** no-op for now (reintroduce per-OS later).

use crate::icons::{self as overlay_icons};
use egui::{self, Color32};
use parking_lot::Mutex;
use sqyre_capture::{
    get_active_window, note, window_is_our_process, window_is_transient_shell_focus,
    window_matches_binding, window_matches_program, WindowInfo,
};
use sqyre_persist::{
    OverlayButtonConfig, ProgramCatalog, DEFAULT_OVERLAY_BUTTON_SIZE, GENERAL_PROGRAM,
    MAX_OVERLAY_BUTTON_SIZE, MIN_OVERLAY_BUTTON_SIZE,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use web_time::{Duration, Instant};

#[cfg(target_os = "linux")]
use crate::x11_buttons::{NativeButtonSpec, X11ButtonHost};

const FOCUS_POLL: Duration = Duration::from_millis(500);
const FOCUS_ERR_LOG_EVERY: Duration = Duration::from_secs(5);

/// Draws enabled overlay buttons; Linux uses a native X11 host thread.
pub struct MacroOverlay {
    focus_slot: Arc<Mutex<FocusSlot>>,
    focus_poller: Mutex<Option<(Arc<AtomicBool>, JoinHandle<()>)>>,
    last_focus_err_log: Option<Instant>,
    last_sync_sig: Option<(usize, bool, bool)>,
    #[cfg(target_os = "linux")]
    x11_host: Option<X11ButtonHost>,
    #[cfg(target_os = "linux")]
    wake_sent: bool,
    #[cfg(target_os = "linux")]
    last_native_specs: Option<Vec<NativeButtonSpec>>,
}

struct FocusSlot {
    cached: Option<WindowInfo>,
    last_foreign: Option<WindowInfo>,
    last_err: Option<String>,
    last_err_at: Option<Instant>,
}

impl Default for MacroOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MacroOverlay {
    fn drop(&mut self) {
        self.stop_focus_poller();
        #[cfg(target_os = "linux")]
        if let Some(host) = self.x11_host.take() {
            host.shutdown();
        }
    }
}

impl MacroOverlay {
    pub fn new() -> Self {
        Self {
            focus_slot: Arc::new(Mutex::new(FocusSlot {
                cached: None,
                last_foreign: None,
                last_err: None,
                last_err_at: None,
            })),
            focus_poller: Mutex::new(None),
            last_focus_err_log: None,
            last_sync_sig: None,
            #[cfg(target_os = "linux")]
            x11_host: None,
            #[cfg(target_os = "linux")]
            wake_sent: false,
            #[cfg(target_os = "linux")]
            last_native_specs: None,
        }
    }

    /// Show/hide buttons from settings + focus gate; drain native clicks into `pending_macros`.
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
            self.stop_focus_poller();
            #[cfg(target_os = "linux")]
            {
                if let Some(host) = &self.x11_host {
                    host.set_buttons(Vec::new());
                }
                self.last_native_specs = Some(Vec::new());
            }
            return;
        }

        self.ensure_focus_poller(ctx);
        let focus = self.resolve_focus();
        let preview_id = preview.map(|b| b.id.as_str());
        let mut any_gated = false;
        let mut shown = 0usize;
        let mut specs: Vec<ButtonDraw> = Vec::new();

        for btn in buttons {
            if preview_id == Some(btn.id.as_str()) {
                continue;
            }
            if !btn.enabled || btn.macro_name.trim().is_empty() {
                continue;
            }
            let gated = button_is_focus_gated(btn);
            if gated {
                any_gated = true;
                if !program_owns_focus(catalog, &btn.program, focus.as_ref()) {
                    continue;
                }
            }
            let busy = button_is_busy(btn, running_macro);
            let mut drawn = btn.clone();
            let (x, y) = btn.resolved_position(catalog);
            drawn.x = x;
            drawn.y = y;
            specs.push(ButtonDraw {
                cfg: drawn,
                busy,
            });
            shown += 1;
        }

        if let Some(btn) = preview {
            let busy = button_is_busy(btn, running_macro);
            let mut drawn = btn.clone();
            let (x, y) = btn.resolved_position(catalog);
            drawn.x = x;
            drawn.y = y;
            specs.push(ButtonDraw {
                cfg: drawn,
                busy,
            });
            shown += 1;
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
        }

        #[cfg(target_os = "linux")]
        {
            self.ensure_x11_host(pending_macros);
            if let Some(host) = &self.x11_host {
                if !self.wake_sent {
                    host.set_wake(ctx.clone());
                    self.wake_sent = true;
                }
                let native: Vec<NativeButtonSpec> = specs
                    .iter()
                    .map(|d| spec_from_config(&d.cfg, d.busy, ctx.pixels_per_point()))
                    .collect();
                if self.last_native_specs.as_ref() != Some(&native) {
                    host.set_buttons(native.clone());
                    self.last_native_specs = Some(native);
                }
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = (ctx, pending_macros, specs);
        }
    }

    #[cfg(target_os = "linux")]
    fn ensure_x11_host(&mut self, pending: &Arc<Mutex<Vec<String>>>) {
        if self.x11_host.is_some() {
            return;
        }
        match X11ButtonHost::start(Arc::clone(pending)) {
            Ok(host) => {
                note("overlay: using native X11 button host (direct enqueue)");
                self.x11_host = Some(host);
                self.wake_sent = false;
                self.last_native_specs = None;
            }
            Err(e) => note(&format!("overlay: X11 host failed: {e}")),
        }
    }

    fn ensure_focus_poller(&self, ctx: &egui::Context) {
        let mut slot = self.focus_poller.lock();
        if slot.is_some() {
            return;
        }
        let stop = Arc::new(AtomicBool::new(false));
        let focus_slot = Arc::clone(&self.focus_slot);
        let stop_t = Arc::clone(&stop);
        let ctx_t = ctx.clone();
        let join = thread::Builder::new()
            .name("sqyre-overlay-focus".into())
            .spawn(move || focus_poll_loop(focus_slot, stop_t, ctx_t))
            .ok();
        if let Some(join) = join {
            *slot = Some((stop, join));
        }
    }

    fn stop_focus_poller(&self) {
        let mut slot = self.focus_poller.lock();
        if let Some((stop, join)) = slot.take() {
            stop.store(true, Ordering::Relaxed);
            let _ = join.join();
        }
    }

    fn resolve_focus(&mut self) -> Option<WindowInfo> {
        let mut g = self.focus_slot.lock();
        if let Some(err) = g.last_err.take() {
            let now = Instant::now();
            let should_log = self
                .last_focus_err_log
                .map(|t| now.duration_since(t) >= FOCUS_ERR_LOG_EVERY)
                .unwrap_or(true);
            if should_log {
                self.last_focus_err_log = Some(now);
                note(&format!("overlay: get_active_window failed: {err}"));
            }
        }
        g.cached.clone()
    }
}

fn focus_poll_loop(
    focus_slot: Arc<Mutex<FocusSlot>>,
    stop: Arc<AtomicBool>,
    ctx: egui::Context,
) {
    while !stop.load(Ordering::Relaxed) {
        match get_active_window() {
            Ok(Some(active)) => {
                let mut g = focus_slot.lock();
                if window_is_our_process(&active) || window_is_transient_shell_focus(&active) {
                    // Keep last foreign target so clicking overlay chrome does not hide gated buttons.
                    g.cached = g.last_foreign.clone();
                } else {
                    let key = focus_identity_key(&active);
                    let prev = g.last_foreign.as_ref().map(focus_identity_key);
                    if prev.as_deref() != Some(key.as_str()) {
                        g.last_foreign = Some(active.clone());
                        g.cached = Some(active);
                        ctx.request_repaint();
                    } else {
                        g.cached = g.last_foreign.clone();
                    }
                }
                g.last_err = None;
            }
            Ok(None) => {
                let mut g = focus_slot.lock();
                g.cached = g.last_foreign.clone();
                g.last_err = None;
            }
            Err(e) => {
                let mut g = focus_slot.lock();
                g.last_err = Some(e.to_string());
                g.last_err_at = Some(Instant::now());
            }
        }
        thread::sleep(FOCUS_POLL);
    }
}

struct ButtonDraw {
    cfg: OverlayButtonConfig,
    busy: bool,
}

fn focus_identity_key(w: &WindowInfo) -> String {
    format!("{}|{}", w.process_path.trim(), w.process_name.trim())
}

fn button_is_focus_gated(btn: &OverlayButtonConfig) -> bool {
    let p = btn.program.trim();
    !p.is_empty() && p != GENERAL_PROGRAM
}

fn program_owns_focus(
    catalog: &ProgramCatalog,
    program: &str,
    focus: Option<&WindowInfo>,
) -> bool {
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

#[cfg(target_os = "linux")]
fn spec_from_config(btn: &OverlayButtonConfig, busy: bool, ppp: f32) -> NativeButtonSpec {
    let size = if btn.size > 0.0 {
        btn.size
    } else {
        DEFAULT_OVERLAY_BUTTON_SIZE
    }
    .clamp(MIN_OVERLAY_BUTTON_SIZE, MAX_OVERLAY_BUTTON_SIZE);
    let style = overlay_icons::OverlayPaintStyle::from_config(btn);
    let icon = overlay_icons::resolve(&btn.icon);
    let glyph = icon.glyph.chars().next().unwrap_or('\u{0}');
    let ppp = ppp.max(0.01);
    let phys = (size * ppp).round().max(1.0);
    NativeButtonSpec {
        id: btn.id.clone(),
        macro_name: btn.macro_name.clone(),
        x: btn.x.round() as i32,
        y: btn.y.round() as i32,
        w: phys as u32,
        h: phys as u32,
        bg: opaque_rgb(style.bg),
        border: [
            style.border.r(),
            style.border.g(),
            style.border.b(),
            style.border.a(),
        ],
        border_width: (style.border_width * ppp).max(0.0),
        corner_radius: (style.corner_radius * ppp).max(0.0),
        icon_glyph: glyph,
        icon: [
            style.icon.r(),
            style.icon.g(),
            style.icon.b(),
            style.icon.a(),
        ],
        icon_hover: [
            style.icon_hover.r(),
            style.icon_hover.g(),
            style.icon_hover.b(),
            style.icon_hover.a(),
        ],
        tip: {
            let label = btn.label.trim();
            if !label.is_empty() {
                label.to_string()
            } else {
                btn.macro_name.trim().to_string()
            }
        },
        busy,
    }
}

fn opaque_rgb(c: Color32) -> [u8; 3] {
    if c.a() == 0 {
        [20, 18, 14]
    } else {
        [c.r(), c.g(), c.b()]
    }
}
