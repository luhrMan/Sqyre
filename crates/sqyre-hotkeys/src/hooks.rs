//! rdev-based Esc stop + Esc+Ctrl+Alt+Shift failsafe + per-macro chords (Linux X11 / non-root).

use crate::continue_wait::{rdev_key_name, ContinueWaitBridge};
use crate::macro_hotkeys::MacroHotkeyBridge;
use crate::macro_record::{MacroRecordBridge, RecordMouseButton};
use crate::screen_click::ScreenClickBridge;
use crate::{HotkeyCallbacks, HotkeyError, HotkeyService};
use parking_lot::Mutex;
use rdev::{listen, Button, Event, EventType, Key};
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

fn record_button(button: Button) -> Option<RecordMouseButton> {
    match button {
        Button::Left => Some(RecordMouseButton::Left),
        Button::Right => Some(RecordMouseButton::Right),
        Button::Middle => Some(RecordMouseButton::Middle),
        _ => None,
    }
}

impl HotkeyService for RdevHotkeys {
    fn start(&mut self, callbacks: HotkeyCallbacks) -> Result<(), HotkeyError> {
        self.stop();
        let stop = Arc::clone(&self.stop);
        stop.store(false, Ordering::SeqCst);
        let continue_wait = self.continue_wait.clone();
        let screen_click = self.screen_click.clone();
        let macro_record = self.macro_record.clone();
        let macro_hotkeys = self.macro_hotkeys.clone();
        let handle = thread::Builder::new()
            .name("sqyre-hotkeys".into())
            .spawn(move || {
                let mut pressed: HashSet<&'static str> = HashSet::new();
                let _ = listen(move |event: Event| {
                    if stop.load(Ordering::SeqCst) {
                        return;
                    }
                    match event.event_type {
                        EventType::MouseMove { x, y } => {
                            screen_click.on_mouse_move(x as i32, y as i32);
                            macro_record.on_mouse_move(x as i32, y as i32);
                        }
                        EventType::ButtonPress(button) => {
                            if let Some(btn) = record_button(button) {
                                macro_record.on_button(btn, true);
                            }
                            if matches!(button, Button::Left) && screen_click.is_armed() {
                                screen_click.on_left_click();
                            }
                        }
                        EventType::ButtonRelease(button) => {
                            if let Some(btn) = record_button(button) {
                                macro_record.on_button(btn, false);
                            }
                        }
                        EventType::KeyPress(key) => {
                            if let Some(name) = rdev_key_name(key) {
                                pressed.insert(name);
                            }
                            continue_wait.on_pressed_keys(&pressed);
                            let on_fire = &*callbacks.on_macro_hotkey;
                            macro_hotkeys.on_pressed_keys(&pressed, on_fire);
                            // Record keys on the hook thread so presses are not lost
                            // while the recording HUD / focus settles (UI sync alone
                            // needed a click before keys started registering).
                            if macro_record.is_armed() {
                                let keys: HashSet<&str> = pressed.iter().copied().collect();
                                macro_record.sync_pressed_keys(&keys);
                            }

                            let ctrl = pressed.contains("ctrl");
                            let shift = pressed.contains("shift") || pressed.contains("rshift");
                            if matches!(key, Key::Escape) {
                                if macro_record.on_escape() {
                                    // Macro recording takes Esc.
                                } else if screen_click.on_escape() {
                                    // Point/area recording takes Esc; don't also stop macros.
                                } else if crate::failsafe_modifiers_held(&pressed) {
                                    (callbacks.on_failsafe)();
                                } else if !ctrl && !shift && !continue_wait.continue_is_escape() {
                                    (callbacks.on_escape_stop)();
                                }
                            }
                        }
                        EventType::KeyRelease(key) => {
                            if let Some(name) = rdev_key_name(key) {
                                pressed.remove(&name);
                            }
                            continue_wait.on_pressed_keys(&pressed);
                            let on_fire = &*callbacks.on_macro_hotkey;
                            macro_hotkeys.on_pressed_keys(&pressed, on_fire);
                            if macro_record.is_armed() {
                                let keys: HashSet<&str> = pressed.iter().copied().collect();
                                macro_record.sync_pressed_keys(&keys);
                            }
                        }
                        _ => {}
                    }
                });
            })
            .map_err(|e| HotkeyError::ThreadSpawn(e.to_string()))?;
        *self.join.lock() = Some(handle);
        Ok(())
    }

    fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // rdev::listen blocks forever; we cannot join cleanly without process exit.
        let _ = self.join.lock().take();
    }
}
