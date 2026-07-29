//! Global hotkey service with injectable no-op stub.

mod continue_wait;
mod error;
mod macro_hotkeys;
mod macro_record;
mod screen_click;

#[cfg(all(feature = "hooks", target_os = "windows"))]
pub use continue_wait::vk_key_name;
pub use continue_wait::ContinueWaitBridge;
pub use macro_hotkeys::{
    chord_all_pressed, chord_fully_released, format_hotkey, parse_hotkey, HotkeyTrigger,
    MacroHotkeyBinding, MacroHotkeyBridge,
};
pub use macro_record::{MacroRecordBridge, MacroRecordEvent, RecordMouseButton};
pub use screen_click::ScreenClickBridge;
pub use sqyre_domain::{
    failsafe_modifiers_held, is_failsafe_chord, normalize_key_name, normalize_keys,
    validate_continue_key, validate_not_failsafe, FAILSAFE_KEYS, FAILSAFE_LABEL,
};

pub use error::HotkeyError;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Callbacks invoked from the hook thread (keep short).
#[derive(Clone)]
pub struct HotkeyCallbacks {
    pub on_escape_stop: Arc<dyn Fn() + Send + Sync>,
    pub on_failsafe: Arc<dyn Fn() + Send + Sync>,
    /// Fired with the macro name when a registered chord matches.
    pub on_macro_hotkey: Arc<dyn Fn(String) + Send + Sync>,
}

impl Default for HotkeyCallbacks {
    fn default() -> Self {
        Self {
            on_escape_stop: Arc::new(|| {}),
            on_failsafe: Arc::new(|| {}),
            on_macro_hotkey: Arc::new(|_| {}),
        }
    }
}

/// Shared stop flag used by app + executor.
#[derive(Clone, Default)]
pub struct StopFlag(Arc<AtomicBool>);

impl StopFlag {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
    pub fn request_stop(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    pub fn clear(&self) {
        self.0.store(false, Ordering::SeqCst);
    }
    pub fn is_stopped(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
    pub fn raw(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.0)
    }
}

pub trait HotkeyService: Send {
    fn start(&mut self, callbacks: HotkeyCallbacks) -> Result<(), HotkeyError>;
    fn stop(&mut self);
}

/// Always-available stub (CI / tests).
#[derive(Debug, Default)]
pub struct NullHotkeys {
    running: bool,
}

impl HotkeyService for NullHotkeys {
    fn start(&mut self, _callbacks: HotkeyCallbacks) -> Result<(), HotkeyError> {
        self.running = true;
        Ok(())
    }
    fn stop(&mut self) {
        self.running = false;
    }
}

#[cfg(all(feature = "hooks", not(target_os = "windows")))]
mod hooks;

#[cfg(all(feature = "hooks", not(target_os = "windows")))]
pub use hooks::RdevHotkeys;

#[cfg(all(feature = "hooks", target_os = "linux"))]
mod wayland_hooks;

#[cfg(all(feature = "hooks", target_os = "linux"))]
pub use wayland_hooks::WaylandHotkeys;

#[cfg(all(feature = "hooks", target_os = "windows"))]
mod win_hooks;

#[cfg(all(feature = "hooks", target_os = "windows"))]
pub use win_hooks::WinHotkeys;

/// Default hotkeys + bridges (continue-wait, screen-click, macro record, chords).
pub fn default_hotkeys() -> (
    Box<dyn HotkeyService>,
    ContinueWaitBridge,
    ScreenClickBridge,
    MacroRecordBridge,
    MacroHotkeyBridge,
) {
    let screen_click = ScreenClickBridge::new();
    let macro_record = MacroRecordBridge::new();
    let macro_hotkeys = MacroHotkeyBridge::new();
    #[cfg(all(feature = "hooks", target_os = "windows"))]
    {
        let bridge = ContinueWaitBridge::new(true);
        (
            Box::new(WinHotkeys::new(
                bridge.clone(),
                screen_click.clone(),
                macro_record.clone(),
                macro_hotkeys.clone(),
            )),
            bridge,
            screen_click,
            macro_record,
            macro_hotkeys,
        )
    }
    #[cfg(all(feature = "hooks", target_os = "linux"))]
    {
        let bridge = ContinueWaitBridge::new(true);
        if sqyre_capture::is_wayland_backend() {
            (
                Box::new(WaylandHotkeys::new(
                    bridge.clone(),
                    screen_click.clone(),
                    macro_record.clone(),
                    macro_hotkeys.clone(),
                )),
                bridge,
                screen_click,
                macro_record,
                macro_hotkeys,
            )
        } else {
            (
                Box::new(RdevHotkeys::new(
                    bridge.clone(),
                    screen_click.clone(),
                    macro_record.clone(),
                    macro_hotkeys.clone(),
                )),
                bridge,
                screen_click,
                macro_record,
                macro_hotkeys,
            )
        }
    }
    #[cfg(all(feature = "hooks", not(target_os = "windows"), not(target_os = "linux")))]
    {
        let bridge = ContinueWaitBridge::new(true);
        (
            Box::new(RdevHotkeys::new(
                bridge.clone(),
                screen_click.clone(),
                macro_record.clone(),
                macro_hotkeys.clone(),
            )),
            bridge,
            screen_click,
            macro_record,
            macro_hotkeys,
        )
    }
    #[cfg(not(feature = "hooks"))]
    {
        (
            Box::new(NullHotkeys::default()),
            ContinueWaitBridge::new(false),
            screen_click,
            macro_record,
            macro_hotkeys,
        )
    }
}

/// Press-latch helpers for macro chord logic.
pub mod latch {
    use parking_lot::Mutex;
    use std::time::Duration;

    pub fn try_acquire(mu: &Mutex<bool>) -> bool {
        let mut g = mu.lock();
        if *g {
            return false;
        }
        *g = true;
        true
    }

    pub fn clear(mu: &Mutex<bool>) {
        *mu.lock() = false;
    }

    /// Poll until `all_pressed` is false.
    pub fn wait_while_all_pressed(mut all_pressed: impl FnMut() -> bool, poll: Duration) {
        while all_pressed() {
            std::thread::sleep(poll);
        }
    }

    /// Fire after leaving the chord, then full release.
    pub fn run_after_chord_then_full_release(
        mut all_pressed: impl FnMut() -> bool,
        mut fully_released: impl FnMut() -> bool,
        poll: Duration,
        on_fire: impl FnOnce(),
    ) {
        while all_pressed() {
            std::thread::sleep(poll);
        }
        while !fully_released() {
            std::thread::sleep(poll);
        }
        on_fire();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_flag_roundtrip() {
        let f = StopFlag::new();
        assert!(!f.is_stopped());
        f.request_stop();
        assert!(f.is_stopped());
        f.clear();
        assert!(!f.is_stopped());
    }

    #[test]
    fn null_hotkeys_start_stop() {
        let mut h = NullHotkeys::default();
        h.start(HotkeyCallbacks::default()).unwrap();
        h.stop();
    }

    #[test]
    fn hotkey_error_display() {
        let err = HotkeyError::ThreadSpawn("spawn failed".into());
        assert!(err.to_string().contains("hotkey thread"));
    }

    #[test]
    fn latch_blocks_repeat() {
        let m = parking_lot::Mutex::new(false);
        assert!(latch::try_acquire(&m));
        assert!(!latch::try_acquire(&m));
        latch::clear(&m);
        assert!(latch::try_acquire(&m));
    }
}
