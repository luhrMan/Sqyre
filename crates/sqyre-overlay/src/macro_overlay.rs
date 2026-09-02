//! Floating overlay buttons that enqueue macros by name.
//!
//! **Linux:** native X11 override-redirect windows on a dedicated event thread
//! ([`crate::x11_buttons`]). Clicks are handled there — they do not wait for the
//! egui ROOT frame (which starved under fullscreen XWayland GameThread on GNOME).
//!
//! **Other OS:** no-op for now (reintroduce per-OS later).

use crate::icons::{self as overlay_icons};
use egui::{self};
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

#[cfg(target_os = "linux")]
pub use crate::x11_buttons::OverlayButtonMove;

/// Desktop position committed after a relocate-mode drag (root coordinates).
#[cfg(not(target_os = "linux"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayButtonMove {
    pub id: String,
    pub x: i32,
    pub y: i32,
}

const FOCUS_POLL: Duration = Duration::from_millis(500);
const FOCUS_ERR_LOG_EVERY: Duration = Duration::from_secs(5);
/// Fullscreen XWayland often reports no focus briefly; hide only after it sticks.
const NONE_HIDE_AFTER: Duration = Duration::from_millis(1500);
/// Overlay click steals focus to Sqyre briefly; hide only if the user stays on Sqyre.
const OUR_HIDE_AFTER: Duration = Duration::from_millis(1500);

/// Draws enabled overlay buttons; Linux uses a native X11 host thread.
pub struct MacroOverlay {
    focus_slot: Arc<Mutex<FocusSlot>>,
    focus_poller: Mutex<Option<(Arc<AtomicBool>, JoinHandle<()>)>>,
    last_focus_err_log: Option<Instant>,
    last_sync_sig: Option<(usize, bool, bool, usize, usize)>,
    /// Shared with the X11 host + run worker so busy does not wait on egui frames.
    running_macro: Arc<Mutex<Option<String>>>,
    #[cfg(target_os = "linux")]
    x11_host: Option<X11ButtonHost>,
    #[cfg(target_os = "linux")]
    wake_sent: bool,
    #[cfg(target_os = "linux")]
    last_native_specs: Option<Vec<NativeButtonSpec>>,
    #[cfg(target_os = "linux")]
    pending_moves: Arc<Mutex<Vec<OverlayButtonMove>>>,
    #[cfg(target_os = "linux")]
    last_relocate: Option<bool>,
}

struct FocusSlot {
    cached: Option<WindowInfo>,
    last_foreign: Option<WindowInfo>,
    last_err: Option<String>,
    last_err_at: Option<Instant>,
    none_since: Option<Instant>,
    our_since: Option<Instant>,
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
                none_since: None,
                our_since: None,
            })),
            focus_poller: Mutex::new(None),
            last_focus_err_log: None,
            last_sync_sig: None,
            running_macro: Arc::new(Mutex::new(None)),
            #[cfg(target_os = "linux")]
            x11_host: None,
            #[cfg(target_os = "linux")]
            wake_sent: false,
            #[cfg(target_os = "linux")]
            last_native_specs: None,
            #[cfg(target_os = "linux")]
            pending_moves: Arc::new(Mutex::new(Vec::new())),
            #[cfg(target_os = "linux")]
            last_relocate: None,
        }
    }

    /// Positions committed by drag-relocate while the Overlay editor tab is open.
    pub fn drain_moves(&self) -> Vec<OverlayButtonMove> {
        #[cfg(target_os = "linux")]
        {
            std::mem::take(&mut *self.pending_moves.lock())
        }
        #[cfg(not(target_os = "linux"))]
        {
            Vec::new()
        }
    }

    /// Shared slot for the running macro name (X11 host polls this for busy).
    pub fn running_macro_slot(&self) -> Arc<Mutex<Option<String>>> {
        Arc::clone(&self.running_macro)
    }

    /// Mark which macro is running so overlay hits pass through until it finishes.
    pub fn set_running_macro(&self, name: Option<String>) {
        *self.running_macro.lock() = name;
    }

    /// Enable/disable drag-to-relocate (move cursor + no macro enqueue on click).
    pub fn set_relocate_mode(&mut self, enabled: bool) {
        #[cfg(target_os = "linux")]
        {
            if self.last_relocate == Some(enabled) {
                return;
            }
            if let Some(host) = &self.x11_host {
                host.set_relocate_mode(enabled);
            }
            self.last_relocate = Some(enabled);
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = enabled;
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
        relocate: bool,
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
            self.set_relocate_mode(false);
            return;
        }

        self.ensure_focus_poller(ctx);
        let focus = self.resolve_focus();
        let preview_id = preview.map(|b| b.id.as_str());
        let mut any_gated = false;
        let mut gated_skips = 0usize;
        let mut shown = 0usize;
        let mut busy_shown = 0usize;
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
                    gated_skips += 1;
                    continue;
                }
            }
            let busy = button_is_busy(btn, running_macro);
            if busy {
                busy_shown += 1;
            }
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
            if busy {
                busy_shown += 1;
            }
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

        let sig = (shown, any_gated, preview.is_some(), busy_shown, gated_skips);
        if self.last_sync_sig != Some(sig) {
            self.last_sync_sig = Some(sig);
            let focus_label = focus
                .as_ref()
                .map(|w| format!("{} ({})", w.process_name.trim(), w.process_path.trim()))
                .unwrap_or_else(|| "(none)".into());
            note(&format!(
                "overlay: sync shown={shown} busy={busy_shown} gated={any_gated} skips={gated_skips} preview={} focus={focus_label}",
                preview.is_some()
            ));
        }

        #[cfg(target_os = "linux")]
        {
            self.ensure_x11_host(pending_macros);
            self.set_relocate_mode(relocate);
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
            let _ = (ctx, pending_macros, specs, relocate);
        }
    }

    #[cfg(target_os = "linux")]
    fn ensure_x11_host(&mut self, pending: &Arc<Mutex<Vec<String>>>) {
        if self.x11_host.is_some() {
            return;
        }
        match X11ButtonHost::start(
            Arc::clone(pending),
            Arc::clone(&self.pending_moves),
            Arc::clone(&self.running_macro),
        ) {
            Ok(host) => {
                note("overlay: using native X11 button host (direct enqueue)");
                self.x11_host = Some(host);
                self.wake_sent = false;
                self.last_native_specs = None;
                self.last_relocate = None;
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
                let repaint = apply_active_focus(&mut g, &active);
                g.last_err = None;
                drop(g);
                if repaint {
                    ctx.request_repaint();
                }
            }
            Ok(None) => {
                let mut g = focus_slot.lock();
                g.our_since = None;
                let started = *g.none_since.get_or_insert_with(Instant::now);
                let repaint = if started.elapsed() >= NONE_HIDE_AFTER {
                    // Sustained no-focus (alt-tab to Wayland / desktop) — hide gated buttons.
                    let had = g.cached.is_some();
                    g.cached = None;
                    had
                } else {
                    // Brief None blip (fullscreen flicker) — keep last foreign.
                    let before = g.cached.as_ref().map(focus_identity_key);
                    g.cached = g.last_foreign.clone();
                    before != g.cached.as_ref().map(focus_identity_key)
                };
                g.last_err = None;
                drop(g);
                if repaint {
                    ctx.request_repaint();
                }
            }
            Err(e) => {
                let mut g = focus_slot.lock();
                g.last_err = Some(e.to_string());
                g.last_err_at = Some(Instant::now());
            }
        }
        // Short slices so stop_focus_poller can join quickly on quit.
        let deadline = Instant::now() + FOCUS_POLL;
        while Instant::now() < deadline {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            thread::sleep(std::time::Duration::from_millis(50));
        }
    }
}

/// Update focus slot from a fresh active window. Returns true when callers should repaint.
///
/// - Overlay chrome / portal / Steam helper: keep `last_foreign` (brief flashes).
/// - Sqyre main UI: keep `last_foreign` briefly (overlay click wake); hide after
///   [`OUR_HIDE_AFTER`] so alt-tabbing to Sqyre clears gated buttons.
/// - Other apps: store as `last_foreign` (identity includes title for shared Wine/Proton exes).
fn apply_active_focus(g: &mut FocusSlot, active: &WindowInfo) -> bool {
    if window_is_transient_shell_focus(active) {
        // Includes overlay button / tip WM titles — do not poison last_foreign.
        g.none_since = None;
        g.our_since = None;
        let before = g.cached.as_ref().map(focus_identity_key);
        g.cached = g.last_foreign.clone();
        return before != g.cached.as_ref().map(focus_identity_key);
    }
    if window_is_our_process(active) {
        g.none_since = None;
        let started = *g.our_since.get_or_insert_with(Instant::now);
        if started.elapsed() >= OUR_HIDE_AFTER {
            let before = g.cached.as_ref().map(focus_identity_key);
            g.cached = None;
            return before.is_some();
        }
        // Brief Sqyre focus (overlay click / macro wake) — keep game gate.
        let before = g.cached.as_ref().map(focus_identity_key);
        g.cached = g.last_foreign.clone();
        return before != g.cached.as_ref().map(focus_identity_key);
    }
    g.none_since = None;
    g.our_since = None;
    let key = focus_identity_key(active);
    let prev = g.last_foreign.as_ref().map(focus_identity_key);
    if prev.as_deref() != Some(key.as_str()) {
        g.last_foreign = Some(active.clone());
        g.cached = Some(active.clone());
        true
    } else {
        // Same identity — refresh fields in case path/title casing changed.
        g.last_foreign = Some(active.clone());
        g.cached = Some(active.clone());
        false
    }
}

struct ButtonDraw {
    cfg: OverlayButtonConfig,
    busy: bool,
}

/// Process + title so shared Proton `wine-preloader` windows still distinguish games.
fn focus_identity_key(w: &WindowInfo) -> String {
    format!(
        "{}|{}|{}",
        w.process_path.trim(),
        w.process_name.trim(),
        w.title.trim()
    )
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
    btn.macro_name.trim().eq_ignore_ascii_case(running)
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
        bg: [
            style.bg.r(),
            style.bg.g(),
            style.bg.b(),
            style.bg.a(),
        ],
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
        tip: btn.macro_name.trim().to_string(),
        busy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_capture::OVERLAY_WM_TITLE;
    use sqyre_persist::ProgramCatalog;

    fn wine_win(title: &str) -> WindowInfo {
        WindowInfo {
            title: title.into(),
            process_name: "wine-preloader".into(),
            process_path: "/opt/proton/files/lib/wine/x86_64-unix/wine-preloader".into(),
            icon: None,
        }
    }

    #[test]
    fn focus_identity_includes_title_for_shared_exe() {
        let a = wine_win("Mistfall Hunter");
        let b = wine_win("Other Game");
        assert_ne!(focus_identity_key(&a), focus_identity_key(&b));
        assert_eq!(focus_identity_key(&a), focus_identity_key(&wine_win("Mistfall Hunter")));
    }

    #[test]
    fn apply_focus_switches_between_shared_proton_titles() {
        let mut g = FocusSlot {
            cached: None,
            last_foreign: None,
            last_err: None,
            last_err_at: None,
            none_since: None,
            our_since: None,
        };
        let mist = wine_win("Mistfall Hunter");
        assert!(apply_active_focus(&mut g, &mist));
        assert_eq!(g.cached.as_ref().map(|w| w.title.as_str()), Some("Mistfall Hunter"));

        let other = wine_win("Other Game");
        assert!(apply_active_focus(&mut g, &other));
        assert_eq!(g.cached.as_ref().map(|w| w.title.as_str()), Some("Other Game"));
        assert_eq!(
            g.last_foreign.as_ref().map(|w| w.title.as_str()),
            Some("Other Game")
        );
    }

    #[test]
    fn apply_focus_overlay_chrome_keeps_last_foreign() {
        let mut g = FocusSlot {
            cached: None,
            last_foreign: None,
            last_err: None,
            last_err_at: None,
            none_since: None,
            our_since: None,
        };
        let mist = wine_win("Mistfall Hunter");
        apply_active_focus(&mut g, &mist);
        let chrome = WindowInfo {
            title: OVERLAY_WM_TITLE.into(),
            process_name: "sqyre".into(),
            process_path: "/bin/sqyre".into(),
            icon: None,
        };
        assert!(!apply_active_focus(&mut g, &chrome));
        assert_eq!(g.cached.as_ref().map(|w| w.title.as_str()), Some("Mistfall Hunter"));
    }

    #[test]
    fn apply_focus_steam_helper_keeps_last_foreign() {
        let mut g = FocusSlot {
            cached: None,
            last_foreign: None,
            last_err: None,
            last_err_at: None,
            none_since: None,
            our_since: None,
        };
        apply_active_focus(&mut g, &wine_win("Mistfall Hunter"));
        let steam = WindowInfo {
            title: "Steam".into(),
            process_name: "steamwebhelper".into(),
            process_path: "/home/x/.local/share/Steam/ubuntu12_64/steamwebhelper".into(),
            icon: None,
        };
        assert!(!apply_active_focus(&mut g, &steam));
        assert_eq!(
            g.last_foreign.as_ref().map(|w| w.title.as_str()),
            Some("Mistfall Hunter")
        );
        assert_eq!(
            g.cached.as_ref().map(|w| w.title.as_str()),
            Some("Mistfall Hunter")
        );
    }

    #[test]
    fn apply_focus_our_main_ui_keeps_last_foreign_for_overlays() {
        let exe = std::env::current_exe().expect("test exe");
        let mut g = FocusSlot {
            cached: None,
            last_foreign: None,
            last_err: None,
            last_err_at: None,
            none_since: None,
            our_since: None,
        };
        apply_active_focus(&mut g, &wine_win("Mistfall Hunter"));
        let main = WindowInfo {
            title: "Sqyre".into(),
            process_name: "sqyre".into(),
            process_path: exe.to_string_lossy().into_owned(),
            icon: None,
        };
        // Sqyre focus must not destroy gated overlays (overlay click / macro wake).
        assert!(!apply_active_focus(&mut g, &main));
        assert_eq!(
            g.cached.as_ref().map(|w| w.title.as_str()),
            Some("Mistfall Hunter")
        );
        assert_eq!(
            g.last_foreign.as_ref().map(|w| w.title.as_str()),
            Some("Mistfall Hunter")
        );
    }

    #[test]
    fn program_owns_focus_requires_bound_title() {
        let mut catalog = ProgramCatalog::default();
        catalog.create_program("Mistfall Hunter").unwrap();
        catalog
            .set_process_binding(
                "Mistfall Hunter",
                "/opt/proton/files/lib/wine/x86_64-unix/wine-preloader",
                "Mistfall Hunter",
            )
            .unwrap();
        assert!(program_owns_focus(
            &catalog,
            "Mistfall Hunter",
            Some(&wine_win("Mistfall Hunter"))
        ));
        assert!(!program_owns_focus(
            &catalog,
            "Mistfall Hunter",
            Some(&wine_win("Other Game"))
        ));
        assert!(!program_owns_focus(&catalog, "Mistfall Hunter", None));
    }

    #[test]
    fn general_buttons_are_not_focus_gated() {
        let mut btn = OverlayButtonConfig::new("g", GENERAL_PROGRAM);
        btn.enabled = true;
        btn.macro_name = "x".into();
        assert!(!button_is_focus_gated(&btn));
        btn.program = "Mistfall Hunter".into();
        assert!(button_is_focus_gated(&btn));
    }
}
