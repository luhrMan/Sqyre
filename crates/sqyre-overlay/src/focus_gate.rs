//! OS-neutral overlay focus-gate policy helpers.
//!
//! Shared by [`crate::macro_overlay`] across Linux / Windows / Android hosts.

use sqyre_capture::{
    window_is_our_process, window_is_transient_shell_focus, window_matches_binding,
    window_matches_program, WindowInfo,
};
use sqyre_domain::GENERAL_PROGRAM;
use sqyre_persist::{OverlayButtonConfig, ProgramCatalog};
use sqyre_ports::note;
use web_time::{Duration, Instant};

/// Fullscreen XWayland often reports no focus for the whole session.
/// Clear `cached` after this so focus-only buttons hide, but keep `last_foreign`
/// so image-gated overlays keep specs and keep polling.
pub(crate) const NONE_HIDE_AFTER: Duration = Duration::from_millis(1500);
/// Overlay click steals focus to Sqyre briefly; hide only if the user stays on Sqyre.
pub(crate) const OUR_HIDE_AFTER: Duration = Duration::from_millis(1500);

pub(crate) struct FocusSlot {
    pub(crate) cached: Option<WindowInfo>,
    pub(crate) last_foreign: Option<WindowInfo>,
    pub(crate) last_err: Option<String>,
    pub(crate) last_err_at: Option<Instant>,
    pub(crate) none_since: Option<Instant>,
    pub(crate) our_since: Option<Instant>,
    /// Sqyre’s main window is in front (past the click-grace when a game was focused).
    pub(crate) sqyre_focused: bool,
}

impl FocusSlot {
    pub(crate) fn idle() -> Self {
        Self {
            cached: None,
            last_foreign: None,
            last_err: None,
            last_err_at: None,
            none_since: None,
            our_since: None,
            sqyre_focused: false,
        }
    }
}

/// Update focus slot from a fresh active window. Returns true when callers should repaint.
///
/// - Overlay chrome / portal / Steam helper: leave `cached` alone (do not poison
///   `last_foreign`, and do not revive a gate already cleared by hide grace).
/// - Sqyre main UI: keep `last_foreign` briefly (overlay click wake); hide after
///   [`OUR_HIDE_AFTER`] so alt-tabbing to Sqyre clears gated buttons. With no
///   foreign window to hold, mark Sqyre in front immediately.
/// - Other apps: store as `last_foreign` (identity includes title for shared Wine/Proton exes).
pub(crate) fn apply_active_focus(g: &mut FocusSlot, active: &WindowInfo) -> bool {
    if window_is_transient_shell_focus(active) {
        // Includes overlay button / tip WM titles — do not poison last_foreign.
        // Do not copy last_foreign into an empty cached: after hide grace that
        // would flash gated buttons on every GNOME overview / portal blip.
        g.none_since = None;
        g.our_since = None;
        return false;
    }
    if window_is_our_process(active) {
        g.none_since = None;
        let started = *g.our_since.get_or_insert_with(Instant::now);
        let parked = started.elapsed() >= OUR_HIDE_AFTER;
        // Clicking a button focuses Sqyre for a moment while a game is still the gate.
        let cold = g.last_foreign.is_none();
        if parked || cold {
            let became = !g.sqyre_focused;
            g.sqyre_focused = true;
            if parked {
                return clear_focus_gate(g) || became;
            }
            return became;
        }
        // Brief Sqyre focus (overlay click / macro wake) — keep game gate.
        let was = g.sqyre_focused;
        g.sqyre_focused = false;
        return hold_last_foreign(g) || was;
    }
    g.none_since = None;
    g.our_since = None;
    let was_sqyre = g.sqyre_focused;
    g.sqyre_focused = false;
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
        was_sqyre
    }
}

/// Drop gated focus after hide grace. Clears `last_foreign` so later None / Sqyre /
/// shell blips cannot resurrect buttons for a window that is no longer focused.
pub(crate) fn clear_focus_gate(g: &mut FocusSlot) -> bool {
    let had = g.cached.is_some() || g.last_foreign.is_some();
    g.cached = None;
    g.last_foreign = None;
    had
}

/// No active window. A brief blip keeps the previous gate; a sustained one
/// drops Sqyre-front tracking so buttons can show over a fullscreen game.
pub(crate) fn apply_no_focus(g: &mut FocusSlot) -> bool {
    g.our_since = None;
    let started = *g.none_since.get_or_insert_with(Instant::now);
    if started.elapsed() >= NONE_HIDE_AFTER {
        let was = g.sqyre_focused;
        g.sqyre_focused = false;
        // Sustained no-focus: hide focus-only overlays, but keep last_foreign
        // so image-gated buttons stay on the X11 host (fullscreen games often
        // report None for the whole session).
        hide_cached_keep_foreign(g) || was
    } else if g.sqyre_focused {
        false
    } else {
        // Brief None blip (fullscreen flicker) — keep last foreign while gate is live.
        hold_last_foreign(g)
    }
}

/// Hide buttons while Sqyre is in front, unless the user opted in or is placing them.
pub(crate) fn suppress_over_sqyre(show_over: bool, sqyre_focused: bool, relocate: bool) -> bool {
    sqyre_focused && !show_over && !relocate
}

/// Hide focus-only overlays after sustained None, but keep `last_foreign` so
/// image-gated buttons can stay mapped and keep polling.
pub(crate) fn hide_cached_keep_foreign(g: &mut FocusSlot) -> bool {
    let had = g.cached.take().is_some();
    if had {
        note("overlay: none-focus cleared cached (keep last_foreign for image gate)");
        sqyre_ports::mark_site("overlay:none-hide-cached");
    }
    had
}

/// Keep showing the last real foreign window while a grace timer is active.
/// No-op when the gate was already cleared (`last_foreign` is None).
pub(crate) fn hold_last_foreign(g: &mut FocusSlot) -> bool {
    let before = g.cached.as_ref().map(focus_identity_key);
    g.cached = g.last_foreign.clone();
    before != g.cached.as_ref().map(focus_identity_key)
}

/// Image-gated buttons keep the last game through fullscreen "no focus" reports.
pub(crate) fn focus_for_overlay_button<'a>(
    btn: &OverlayButtonConfig,
    cached: Option<&'a WindowInfo>,
    last_foreign: Option<&'a WindowInfo>,
) -> Option<&'a WindowInfo> {
    if button_is_focus_gated(btn) && btn.visibility_gate.is_active() {
        cached.or(last_foreign)
    } else {
        cached
    }
}

/// Focus-gated overlay: show when the bound program owns focus.
///
/// Fullscreen XWayland often reports no active window for the whole session
/// (`win` is None). Image-gated buttons stay hosted so the found-map can
/// map/unmap without waiting for an alt-tab to populate `last_foreign`.
pub(crate) fn keep_focus_gated_button(
    btn: &OverlayButtonConfig,
    catalog: &ProgramCatalog,
    win: Option<&WindowInfo>,
) -> bool {
    if program_owns_focus(catalog, &btn.program, win) {
        return true;
    }
    btn.visibility_gate.is_active() && win.is_none()
}

/// Process + title so shared Proton `wine-preloader` windows still distinguish games.
pub(crate) fn focus_identity_key(w: &WindowInfo) -> String {
    format!(
        "{}|{}|{}",
        w.process_path.trim(),
        w.process_name.trim(),
        w.title.trim()
    )
}

pub(crate) fn button_is_focus_gated(btn: &OverlayButtonConfig) -> bool {
    let p = btn.program.trim();
    !p.is_empty() && p != GENERAL_PROGRAM
}

pub(crate) fn program_owns_focus(
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
