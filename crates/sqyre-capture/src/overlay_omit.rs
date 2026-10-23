//! Keep overlay buttons out of detection captures when the user turns that off.
//!
//! The grab calls [`detection_omit_guard`] around one search-area capture. The
//! installed hook hides or excludes the buttons. A short idle keeps them omitted
//! across a wait/retry loop so they do not strobe once per interval.

use parking_lot::{Condvar, Mutex};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use web_time::Instant;

const RESTORE_IDLE: Duration = Duration::from_millis(150);

type Hook = Arc<dyn Fn(bool) + Send + Sync>;

struct State {
    /// When true, detection captures include overlay buttons.
    include: bool,
    hook: Option<Hook>,
    omitted: bool,
    depth: u32,
    /// Bumped whenever a restore is scheduled or cancelled.
    generation: u64,
    restore_at: Option<Instant>,
}

/// Serializes hook(true) / hook(false) with the state change that decides them.
static GATE: Mutex<()> = Mutex::new(());
static STATE: Mutex<State> = Mutex::new(State {
    include: true,
    hook: None,
    omitted: false,
    depth: 0,
    generation: 0,
    restore_at: None,
});
static WAKE: Condvar = Condvar::new();
static WORKER: std::sync::Once = std::sync::Once::new();

/// Publish the user setting. `true` means detection captures include the buttons.
pub fn set_overlay_buttons_in_detection(include: bool) {
    let _gate = GATE.lock();
    let hook = {
        let mut state = STATE.lock();
        if state.include == include {
            return;
        }
        state.include = include;
        if include && state.omitted && state.depth == 0 {
            begin_restore(&mut state)
        } else {
            None
        }
    };
    finish_restore(hook);
}

pub fn overlay_buttons_in_detection() -> bool {
    STATE.lock().include
}

/// Buttons are currently left out of the next detection grab.
pub fn overlay_capture_omitted() -> bool {
    STATE.lock().omitted
}

/// `hook(true)` omits buttons; `hook(false)` shows them again. Replaces any previous hook.
pub fn set_overlay_omit_hook(hook: Option<Hook>) {
    STATE.lock().hook = hook;
}

/// Omit overlay buttons for this detection grab when the setting is off.
///
/// Dropping the guard schedules a restore 150 ms later so a retry loop does not
/// flash the buttons between captures.
pub fn detection_omit_guard() -> DetectionOmitGuard {
    ensure_worker();
    let _gate = GATE.lock();
    let mut state = STATE.lock();
    if state.include {
        return DetectionOmitGuard { active: false };
    }
    state.restore_at = None;
    state.generation = state.generation.wrapping_add(1);
    state.depth += 1;
    let hook = if state.omitted {
        None
    } else {
        state.omitted = true;
        state.hook.clone()
    };
    drop(state);
    if let Some(hook) = hook {
        hook(true);
    }
    DetectionOmitGuard { active: true }
}

/// Portal and Android grabs must wait for a frame taken after the buttons were removed.
/// Windows display affinity does not change the pixels already on screen, and GDI reads live.
pub fn detection_omit_needs_fresh_frame() -> bool {
    if cfg!(target_os = "windows") {
        return false;
    }
    let state = STATE.lock();
    state.omitted && !state.include
}

pub struct DetectionOmitGuard {
    active: bool,
}

impl Drop for DetectionOmitGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        let mut state = STATE.lock();
        state.depth = state.depth.saturating_sub(1);
        if state.depth == 0 && state.omitted {
            state.generation = state.generation.wrapping_add(1);
            state.restore_at = Some(Instant::now() + RESTORE_IDLE);
            WAKE.notify_one();
        }
    }
}

/// Caller holds `STATE`. Marks a restore in progress and returns the hook to run
/// after the lock is dropped. `None` hook means there is nothing to show.
fn begin_restore(state: &mut State) -> Option<Hook> {
    state.omitted = false;
    state.restore_at = None;
    state.generation = state.generation.wrapping_add(1);
    state.hook.clone()
}

fn finish_restore(hook: Option<Hook>) {
    if let Some(hook) = hook {
        hook(false);
    }
}

fn ensure_worker() {
    WORKER.call_once(|| {
        let _ = thread::Builder::new()
            .name("sqyre-overlay-omit".into())
            .spawn(|| loop {
                let mut state = STATE.lock();
                let wait = match state.restore_at {
                    Some(at) => at.saturating_duration_since(Instant::now()),
                    None => Duration::from_secs(3600),
                };
                let generation = state.generation;
                let _timed_out = WAKE.wait_for(&mut state, wait);
                drop(state);
                let _gate = GATE.lock();
                let mut state = STATE.lock();
                let hook = if state.depth == 0
                    && state.omitted
                    && state.generation == generation
                    && state.restore_at.is_some_and(|at| Instant::now() >= at)
                {
                    begin_restore(&mut state)
                } else {
                    None
                };
                drop(state);
                finish_restore(hook);
            });
    });
}

#[cfg(test)]
pub fn restore_overlay_omit_now() {
    let _gate = GATE.lock();
    let hook = {
        let mut state = STATE.lock();
        if state.omitted && state.depth == 0 {
            begin_restore(&mut state)
        } else {
            None
        }
    };
    finish_restore(hook);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omit_guard_calls_the_hook_and_restore_shows_buttons() {
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
}
