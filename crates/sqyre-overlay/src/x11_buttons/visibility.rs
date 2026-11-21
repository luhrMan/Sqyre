//! Gate map/unmap and busy-hit visibility for the X11 overlay host.

use super::*;
use super::paint::{apply_empty_input_shape, apply_hit_rounded_input, paint_button};

/// `XMapRaised` over a focused XWayland game freezes the next drag and drops
/// held mouse buttons on Mutter. Same gate as relocate `keep_above=false`.
pub(super) fn allow_map_raised(click_through: bool) -> bool {
    !click_through
}

pub(super) static MAP_RAISED_SKIP_NOTED: AtomicBool = AtomicBool::new(false);

pub(super) static MAP_RAISED_SKIP_COUNT: AtomicUsize = AtomicUsize::new(0);

pub(super) fn raise_all_hits(state: &HostState) {
    if !allow_map_raised(state.click_through) {
        MAP_RAISED_SKIP_COUNT.fetch_add(1, Ordering::Relaxed);
        if !MAP_RAISED_SKIP_NOTED.swap(true, Ordering::Relaxed) {
            note("overlay-x11: skip MapRaised while click-through");
            event_log("SQYRE_OVERLAY", &[("map_raised", "skip_click_through")]);
        }
        return;
    }
    for btn in state.buttons.values() {
        if !btn.mapped {
            continue;
        }
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

/// Map/unmap gated buttons from the image-search found map (no egui frame needed).
pub(super) fn apply_gate_visibility(state: &mut HostState) {
    let found = state.visibility.lock().clone();
    let ids: Vec<String> = state.buttons.keys().cloned().collect();
    let mut changed = false;
    let mut shown = 0usize;
    let mut hidden = 0usize;
    for id in ids {
        let Some(btn) = state.buttons.get(&id) else {
            continue;
        };
        // Relocate (Overlay editor open): keep hosted buttons mapped so the user
        // can drag them; do not let the found-map hide them mid-edit. Callers only
        // host the selected program's buttons in this mode.
        // Closed hub satellites stay unmapped (warm windows, no paint/hit).
        let fan_show =
            !crate::hub_fan::is_hub_satellite_id(&id) || state.hub_fan.open_or_traveling();
        let show = !state.capture_omit
            && fan_show
            && (state.relocate_mode
                || !btn.spec.visibility_gated
                || found.get(&id).copied().unwrap_or(false));
        let (hit, face) = (btn.hit, btn.win);
        if show {
            if btn.mapped {
                continue;
            }
            // MapRaised when relocate keep-above so new gated buttons are not stuck
            // under the Sqyre editor; plain Map otherwise (see create_button).
            // SAFETY: HostState display invariant; face+hit created on this display.
            unsafe {
                if state.relocate_mode && state.relocate_keep_above {
                    XMapRaised(state.display, face);
                    XMapRaised(state.display, hit);
                } else {
                    XMapWindow(state.display, face);
                    XMapWindow(state.display, hit);
                }
            }
            shown += 1;
            changed = true;
        } else {
            // Always Unmap when hidden — XUnmapWindow is idempotent. Skipping when
            // `mapped` was already false left ghosts after MapRaised without a
            // matching flag update (see relocate_mode comment above).
            let was_mapped = btn.mapped;
            if was_mapped {
                if state.drag.as_ref().is_some_and(|d| d.id == id) {
                    cancel_drag(state);
                }
                if state.tip.as_ref().is_some_and(|t| t.for_id == id) {
                    hide_tip(state);
                }
                hidden += 1;
                changed = true;
            }
            // SAFETY: HostState display invariant; face+hit created on this display.
            unsafe {
                x_unmap(state.display, hit);
                x_unmap(state.display, face);
            }
        }
        if let Some(btn) = state.buttons.get_mut(&id) {
            btn.mapped = show;
            if !show {
                btn.armed = false;
                btn.hovered = false;
            }
        }
    }
    if changed {
        note(&format!(
            "overlay-x11: gate-map show={shown} hide={hidden} relocate={} found={}",
            state.relocate_mode,
            found.len()
        ));
        // SAFETY: HostState display invariant.
        unsafe {
            x_flush(state.display);
        }
    }
}

/// Copy the press-to-stop flag onto the host and retarget busy hit shapes.
///
/// When the setting is on, a busy button stays a hit target so the press can
/// stop the run. When it is off, the hit shape is emptied so clicks reach the game.
pub(super) fn sync_press_stops(state: &mut HostState) {
    let enabled = state.press_stops_flag.load(Ordering::Relaxed);
    if enabled == state.press_stops {
        return;
    }
    state.press_stops = enabled;
    let ids: Vec<String> = state.buttons.keys().cloned().collect();
    let mut changed = false;
    for id in ids {
        let Some(btn) = state.buttons.get(&id) else {
            continue;
        };
        if !btn.spec.busy {
            continue;
        }
        changed = true;
        let (hit, w, h, radius) = (
            btn.hit,
            btn.spec.face.w,
            btn.spec.face.h,
            btn.spec.face.corner_radius,
        );
        apply_busy_hit_shape(
            state.display,
            state.press_stops,
            state.click_through,
            hit,
            w,
            h,
            radius,
        );
    }
    if changed {
        // SAFETY: HostState display invariant.
        unsafe {
            x_flush(state.display);
        }
    }
}

pub(super) fn apply_busy_hit_shape(
    display: *mut Display,
    press_stops: bool,
    click_through: bool,
    hit: Window,
    w: u32,
    h: u32,
    radius: f32,
) {
    // Click-through hits stay empty so a confined cursor can cross; polling
    // decides whether the press stops the run. Otherwise a busy button is a
    // target only when press-to-stop is on.
    if click_through || !press_stops {
        apply_empty_input_shape(display, hit);
    } else {
        apply_hit_rounded_input(display, hit, w, h, radius);
    }
}

/// Apply busy from the shared running-macro slot (not egui sync).
///
/// While busy and press-to-stop is off, empty the hit ShapeInput so macro mouse
/// clicks reach the game. When press-to-stop is on, keep the hit so a press can
/// stop the run. When cleared: restore hit shape and raise above the game.
pub(super) fn apply_running_busy(state: &mut HostState, running: Option<&str>) {
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
        // face_bounding is left alone: the repaint below reads whether the
        // union is still applied to order blit-then-shrink correctly.
        btn.armed = false;
        flipped = true;
        // Keep mid-drag hit empty (parked at grab start) until commit/cancel.
        if state.drag.as_ref().is_some_and(|d| d.id == id) {
            continue;
        }
        let (hit, w, h, radius, press_stops, input_enabled) = (
            btn.hit,
            btn.spec.face.w,
            btn.spec.face.h,
            btn.spec.face.corner_radius,
            state.press_stops,
            btn.spec.input_enabled,
        );
        if want {
            apply_busy_hit_shape(
                state.display,
                press_stops,
                state.click_through,
                hit,
                w,
                h,
                radius,
            );
        } else {
            sync_hit_input(state, hit, w, h, radius, input_enabled);
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
