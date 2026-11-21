//! Keep overlay buttons out of detection captures when the user turns that off.
//!
//! Each search-area grab calls [`detection_omit_guard`]. The app also holds a
//! [`sqyre_ports::OverlayMacroHide`] from macro start until the run thread exits.
//! A grab alone restores after a short idle, which is shorter than the gap
//! between attempts, so the buttons flash back on during the macro. The macro
//! span keeps them hidden the whole run.
//!
//! Gate transitions live in [`crate::overlay_omit_gate`]; this module owns the
//! mutex, omit hook, and restore worker.

use crate::overlay_omit_gate::{OmitEffect, OmitGate};
use parking_lot::{Condvar, Mutex};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use web_time::Instant;

type Hook = Arc<dyn Fn(bool) + Send + Sync>;

struct State {
    gate: OmitGate,
    hook: Option<Hook>,
}

/// Serializes hook(true) / hook(false) with the state change that decides them.
static GATE: Mutex<()> = Mutex::new(());
static STATE: Mutex<State> = Mutex::new(State {
    gate: OmitGate {
        include: true,
        omitted: false,
        depth: 0,
        generation: 0,
        restore_at: None,
    },
    hook: None,
});
static WAKE: Condvar = Condvar::new();
static WORKER: std::sync::Once = std::sync::Once::new();

fn apply_effect(effect: OmitEffect, hook: &Option<Hook>) {
    match effect {
        OmitEffect::Hide => {
            if let Some(hook) = hook {
                hook(true);
            }
        }
        OmitEffect::Show => {
            if let Some(hook) = hook {
                hook(false);
            }
        }
        OmitEffect::ScheduleRestore => {
            WAKE.notify_one();
        }
        OmitEffect::None => {}
    }
}

/// Publish the user setting. `true` means detection captures include the buttons.
pub fn set_overlay_buttons_in_detection(include: bool) {
    ensure_span_hooks();
    let _gate = GATE.lock();
    let (effect, hook) = {
        let mut state = STATE.lock();
        let effect = state
            .gate
            .set_include(include, sqyre_ports::overlay_macro_hide_held());
        (effect, state.hook.clone())
    };
    apply_effect(effect, &hook);
}

pub fn overlay_buttons_in_detection() -> bool {
    STATE.lock().gate.include
}

/// Buttons are currently left out of the next detection grab.
pub fn overlay_capture_omitted() -> bool {
    STATE.lock().gate.omitted
}

/// `hook(true)` omits buttons; `hook(false)` shows them again. Replaces any previous hook.
pub fn set_overlay_omit_hook(hook: Option<Hook>) {
    ensure_span_hooks();
    STATE.lock().hook = hook;
}

/// Omit overlay buttons for this detection grab when the setting is off.
///
/// Dropping the guard schedules a restore 150 ms later unless an
/// [`sqyre_ports::OverlayMacroHide`] is still open. That span keeps the buttons
/// hidden for the whole macro instead of flashing them between attempts.
pub fn detection_omit_guard() -> DetectionOmitGuard {
    ensure_span_hooks();
    ensure_worker();
    let _gate = GATE.lock();
    let mut state = STATE.lock();
    if state.gate.include {
        return DetectionOmitGuard { active: false };
    }
    let effect = state.gate.enter_detection();
    let hook = state.hook.clone();
    drop(state);
    apply_effect(effect, &hook);
    DetectionOmitGuard { active: true }
}

/// Portal and Android grabs must wait for a frame taken after the buttons were removed.
/// Windows display affinity does not change the pixels already on screen, and GDI reads live.
#[cfg(target_os = "windows")]
pub fn detection_omit_needs_fresh_frame() -> bool {
    false
}

/// Portal and Android grabs must wait for a frame taken after the buttons were removed.
#[cfg(not(target_os = "windows"))]
pub fn detection_omit_needs_fresh_frame() -> bool {
    let state = STATE.lock();
    state.gate.omitted && !state.gate.include
}

pub struct DetectionOmitGuard {
    active: bool,
}

impl Drop for DetectionOmitGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        let _gate = GATE.lock();
        let (effect, hook) = {
            let mut state = STATE.lock();
            let effect = state
                .gate
                .leave_detection(sqyre_ports::overlay_macro_hide_held(), Instant::now());
            (effect, state.hook.clone())
        };
        apply_effect(effect, &hook);
    }
}

fn ensure_span_hooks() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        sqyre_ports::set_overlay_macro_hide_hooks(
            Some(Arc::new(enter_macro_hide)),
            Some(Arc::new(leave_macro_hide)),
        );
    });
}

fn enter_macro_hide() {
    ensure_worker();
    let _gate = GATE.lock();
    let (effect, hook) = {
        let mut state = STATE.lock();
        let effect = state.gate.enter_macro_hide();
        (effect, state.hook.clone())
    };
    apply_effect(effect, &hook);
}

fn leave_macro_hide() {
    let _gate = GATE.lock();
    if sqyre_ports::overlay_macro_hide_held() {
        return;
    }
    let (effect, hook) = {
        let mut state = STATE.lock();
        let effect = state.gate.leave_macro_hide(Instant::now());
        (effect, state.hook.clone())
    };
    apply_effect(effect, &hook);
}

fn ensure_worker() {
    WORKER.call_once(|| {
        let _ = thread::Builder::new()
            .name("sqyre-overlay-omit".into())
            .spawn(|| loop {
                let mut state = STATE.lock();
                let wait = match state.gate.restore_at {
                    Some(at) => at.saturating_duration_since(Instant::now()),
                    None => Duration::from_secs(3600),
                };
                let generation = state.gate.generation;
                let _timed_out = WAKE.wait_for(&mut state, wait);
                drop(state);
                let _gate = GATE.lock();
                let (effect, hook) = {
                    let mut state = STATE.lock();
                    let effect = state.gate.try_idle_restore(
                        generation,
                        sqyre_ports::overlay_macro_hide_held(),
                        Instant::now(),
                    );
                    (effect, state.hook.clone())
                };
                apply_effect(effect, &hook);
            });
    });
}

#[cfg(test)]
pub fn restore_overlay_omit_now() {
    let _gate = GATE.lock();
    let (effect, hook) = {
        let mut state = STATE.lock();
        let effect = if state.gate.omitted
            && state.gate.depth == 0
            && !sqyre_ports::overlay_macro_hide_held()
        {
            state.gate.begin_restore();
            OmitEffect::Show
        } else {
            OmitEffect::None
        };
        (effect, state.hook.clone())
    };
    apply_effect(effect, &hook);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lock_state() -> parking_lot::MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        LOCK.lock()
    }

    #[test]
    fn omit_guard_calls_the_hook_and_restore_shows_buttons() {
        let _lock = lock_state();
        set_overlay_buttons_in_detection(true);
        set_overlay_omit_hook(None);
        let log = Arc::new(Mutex::new(Vec::new()));
        let log_hook = Arc::clone(&log);
        set_overlay_omit_hook(Some(Arc::new(move |omit| {
            log_hook.lock().push(omit);
        })));

        {
            let _guard = detection_omit_guard();
            assert!(
                log.lock().is_empty(),
                "default-on setting does not omit buttons"
            );
        }

        set_overlay_buttons_in_detection(false);
        {
            let outer = detection_omit_guard();
            assert_eq!(log.lock().as_slice(), &[true]);
            let inner = detection_omit_guard();
            assert_eq!(
                log.lock().as_slice(),
                &[true],
                "a nested grab stays omitted"
            );
            drop(inner);
            drop(outer);
        }
        assert!(overlay_capture_omitted());
        restore_overlay_omit_now();
        assert_eq!(log.lock().as_slice(), &[true, false]);
        assert!(!overlay_capture_omitted());

        set_overlay_buttons_in_detection(true);
        set_overlay_omit_hook(None);
    }

    #[test]
    fn macro_hide_keeps_buttons_down_for_the_whole_run() {
        let _lock = lock_state();
        set_overlay_buttons_in_detection(true);
        let log = Arc::new(Mutex::new(Vec::new()));
        let log_hook = Arc::clone(&log);
        set_overlay_omit_hook(Some(Arc::new(move |omit| {
            log_hook.lock().push(omit);
        })));

        let shown = sqyre_ports::OverlayMacroHide::enter();
        assert!(
            log.lock().is_empty(),
            "buttons stay up when detection includes them"
        );
        drop(shown);

        set_overlay_buttons_in_detection(false);
        let hidden = sqyre_ports::OverlayMacroHide::enter();
        assert_eq!(log.lock().as_slice(), &[true]);
        {
            let grab = detection_omit_guard();
            drop(grab);
        }
        assert_eq!(
            log.lock().as_slice(),
            &[true],
            "a grab must not show the buttons again while the macro is running"
        );
        assert!(overlay_capture_omitted());
        drop(hidden);
        assert!(overlay_capture_omitted());
        restore_overlay_omit_now();
        assert_eq!(log.lock().as_slice(), &[true, false]);
        assert!(!overlay_capture_omitted());

        set_overlay_buttons_in_detection(true);
        set_overlay_omit_hook(None);
    }
}
