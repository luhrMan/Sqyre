//! rdev / evdev Esc stop + failsafe + per-macro chords (Linux).
//!
//! X11 sessions use [`rdev::listen`]. Wayland sessions read `/dev/input` **without**
//! exclusive grab — `rdev::grab` freezes the pointer and makes portal dialogs ignore clicks.

use crate::continue_wait::{rdev_key_name, ContinueWaitBridge};
use crate::macro_hotkeys::MacroHotkeyBridge;
use crate::macro_record::{MacroRecordBridge, RecordMouseButton};
use crate::screen_click::ScreenClickBridge;
use crate::{HotkeyCallbacks, HotkeyError, HotkeyService};
use parking_lot::Mutex;
use rdev::{listen, Button, Event, EventType, Key};
use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

pub struct RdevHotkeys {
    stop: Arc<AtomicBool>,
    join: Mutex<Option<JoinHandle<()>>>,
    continue_wait: ContinueWaitBridge,
    screen_click: ScreenClickBridge,
    macro_record: MacroRecordBridge,
    macro_hotkeys: MacroHotkeyBridge,
}

impl RdevHotkeys {
    pub fn new(
        continue_wait: ContinueWaitBridge,
        screen_click: ScreenClickBridge,
        macro_record: MacroRecordBridge,
        macro_hotkeys: MacroHotkeyBridge,
    ) -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            join: Mutex::new(None),
            continue_wait,
            screen_click,
            macro_record,
            macro_hotkeys,
        }
    }
}

/// True when the hook thread should use evdev (Wayland) instead of X11 listen.
///
/// The evdev path does **not** exclusive-grab devices (see [`crate::linux_evdev`]).
pub fn linux_uses_evdev_grab() -> bool {
    if std::env::var("XDG_SESSION_TYPE").is_ok_and(|s| s.eq_ignore_ascii_case("wayland")) {
        return true;
    }
    std::env::var("WAYLAND_DISPLAY")
        .ok()
        .is_some_and(|s| !s.is_empty())
}

/// Whether the current user can access evdev (`input` group on Wayland).
#[cfg(target_os = "linux")]
pub fn linux_in_input_group() -> bool {
    let Ok(out) = std::process::Command::new("id").arg("-Gn").output() else {
        return false;
    };
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .any(|g| g == "input")
}

#[cfg(not(target_os = "linux"))]
pub fn linux_in_input_group() -> bool {
    true
}

fn record_button(button: Button) -> Option<RecordMouseButton> {
    match button {
        Button::Left => Some(RecordMouseButton::Left),
        Button::Right => Some(RecordMouseButton::Right),
        Button::Middle => Some(RecordMouseButton::Middle),
        _ => None,
    }
}

struct HookCtx {
    stop: Arc<AtomicBool>,
    continue_wait: ContinueWaitBridge,
    screen_click: ScreenClickBridge,
    macro_record: MacroRecordBridge,
    macro_hotkeys: MacroHotkeyBridge,
    callbacks: HotkeyCallbacks,
    pressed: HashSet<&'static str>,
}

impl HookCtx {
    fn dispatch(&mut self, event: &Event) {
        if self.stop.load(Ordering::SeqCst) {
            return;
        }
        match &event.event_type {
            EventType::MouseMove { x, y } => {
                // rdev::grab sits on the evdev path and must return immediately.
                // Mutex work on every motion makes the pointer crawl (Windows already
                // skips WH_MOUSE_LL moves unless a recording is armed).
                if !crate::macro_record::hook_wants_mouse_moves()
                    && !crate::screen_click::hook_wants_mouse_moves()
                {
                    return;
                }
                if !self.screen_click.grab_owns_input() {
                    self.screen_click.on_mouse_move(*x as i32, *y as i32);
                }
                self.macro_record.on_mouse_move(*x as i32, *y as i32);
            }
            EventType::ButtonPress(button) => {
                if let Some(btn) = record_button(*button) {
                    self.macro_record.on_button(btn, true);
                }
                if matches!(button, Button::Left)
                    && self.screen_click.is_armed()
                    && !self.screen_click.grab_owns_input()
                {
                    self.screen_click.on_left_click();
                }
            }
            EventType::ButtonRelease(button) => {
                if let Some(btn) = record_button(*button) {
                    self.macro_record.on_button(btn, false);
                }
            }
            EventType::KeyPress(key) => {
                if let Some(name) = rdev_key_name(*key) {
                    self.pressed.insert(name);
                }
                self.continue_wait.on_pressed_keys(&self.pressed);
                let on_fire = &*self.callbacks.on_macro_hotkey;
                self.macro_hotkeys.on_pressed_keys(&self.pressed, on_fire);
                // Record keys on the hook thread so presses are not lost while the
                // recording HUD / focus settles (UI sync alone needed a click first).
                if self.macro_record.is_armed() {
                    let keys: HashSet<&str> = self.pressed.iter().copied().collect();
                    self.macro_record.sync_pressed_keys(&keys);
                }

                let ctrl = self.pressed.contains("ctrl");
                let shift = self.pressed.contains("shift") || self.pressed.contains("rshift");
                if matches!(key, Key::Escape) {
                    if self.macro_record.on_escape() {
                        // Macro recording takes Esc.
                    } else if self.screen_click.grab_owns_input() && self.screen_click.is_armed() {
                        // SelectionGrab delivers Esc; swallow so we don't stop macros.
                    } else if self.screen_click.on_escape() {
                        // Point/area recording takes Esc; don't also stop macros.
                    } else if crate::failsafe_modifiers_held(&self.pressed) {
                        (self.callbacks.on_failsafe)();
                    } else if !ctrl && !shift && !self.continue_wait.continue_is_escape() {
                        (self.callbacks.on_escape_stop)();
                    }
                }
            }
            EventType::KeyRelease(key) => {
                if let Some(name) = rdev_key_name(*key) {
                    self.pressed.remove(&name);
                }
                self.continue_wait.on_pressed_keys(&self.pressed);
                let on_fire = &*self.callbacks.on_macro_hotkey;
                self.macro_hotkeys.on_pressed_keys(&self.pressed, on_fire);
                if self.macro_record.is_armed() {
                    let keys: HashSet<&str> = self.pressed.iter().copied().collect();
                    self.macro_record.sync_pressed_keys(&keys);
                }
            }
            _ => {}
        }
    }
}

fn run_hook_loop(mut ctx: HookCtx) {
    #[cfg(target_os = "linux")]
    if linux_uses_evdev_grab() {
        if !linux_in_input_group() {
            eprintln!(
                "sqyre-hotkeys: evdev watch skipped (user not in 'input' group). \
                 Global hotkeys and macro recording need: sudo usermod -aG input $USER, then re-login."
            );
            return;
        }
        let stop = Arc::clone(&ctx.stop);
        let ctx = RefCell::new(ctx);
        match crate::linux_evdev::watch_events(stop, move |event: Event| {
            ctx.borrow_mut().dispatch(&event);
        }) {
            Ok(()) => {}
            Err(e) => {
                eprintln!(
                    "sqyre-hotkeys: evdev watch failed ({e}). \
                     Add your user to the 'input' group and re-login: sudo usermod -aG input $USER"
                );
            }
        }
        return;
    }

    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        listen(move |event: Event| {
            ctx.dispatch(&event);
        })
    })) {
        Ok(Ok(())) => {}
        Ok(Err(e)) => eprintln!("sqyre-hotkeys: X11 listen failed ({e:?})"),
        Err(_) => eprintln!("sqyre-hotkeys: X11 listen panicked"),
    }
}

impl HotkeyService for RdevHotkeys {
    fn start(&mut self, callbacks: HotkeyCallbacks) -> Result<(), HotkeyError> {
        self.stop();
        let stop = Arc::clone(&self.stop);
        stop.store(false, Ordering::SeqCst);
        let ctx = HookCtx {
            stop,
            continue_wait: self.continue_wait.clone(),
            screen_click: self.screen_click.clone(),
            macro_record: self.macro_record.clone(),
            macro_hotkeys: self.macro_hotkeys.clone(),
            callbacks,
            pressed: HashSet::new(),
        };
        let handle = thread::Builder::new()
            .name("sqyre-hotkeys".into())
            .spawn(move || run_hook_loop(ctx))
            .map_err(|e| HotkeyError::ThreadSpawn(e.to_string()))?;
        *self.join.lock() = Some(handle);
        Ok(())
    }

    fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wayland evdev watch uses a short epoll timeout and can join. X11 `listen` blocks forever.
        #[cfg(target_os = "linux")]
        if linux_uses_evdev_grab() {
            if let Some(handle) = self.join.lock().take() {
                let _ = handle.join();
            }
            return;
        }
        let _ = self.join.lock().take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wayland_detection_from_env() {
        // Do not mutate env in parallel tests; just ensure the helper is callable.
        let _ = linux_uses_evdev_grab();
        let _ = linux_in_input_group();
    }
}
