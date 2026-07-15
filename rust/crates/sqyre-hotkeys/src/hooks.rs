//! rdev-based Esc stop + Esc+Ctrl+Shift failsafe (Linux X11 / non-root).

use crate::{HotkeyCallbacks, HotkeyService};
use parking_lot::Mutex;
use rdev::{listen, Event, EventType, Key};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

#[derive(Default)]
pub struct RdevHotkeys {
    stop: Arc<AtomicBool>,
    join: Mutex<Option<JoinHandle<()>>>,
}

impl HotkeyService for RdevHotkeys {
    fn start(&mut self, callbacks: HotkeyCallbacks) -> Result<(), String> {
        self.stop();
        let stop = Arc::clone(&self.stop);
        stop.store(false, Ordering::SeqCst);
        let handle = thread::Builder::new()
            .name("sqyre-hotkeys".into())
            .spawn(move || {
                let mut ctrl = false;
                let mut shift = false;
                let mut esc = false;
                let _ = listen(move |event: Event| {
                    if stop.load(Ordering::SeqCst) {
                        return;
                    }
                    match event.event_type {
                        EventType::KeyPress(Key::ControlLeft)
                        | EventType::KeyPress(Key::ControlRight) => ctrl = true,
                        EventType::KeyRelease(Key::ControlLeft)
                        | EventType::KeyRelease(Key::ControlRight) => ctrl = false,
                        EventType::KeyPress(Key::ShiftLeft)
                        | EventType::KeyPress(Key::ShiftRight) => shift = true,
                        EventType::KeyRelease(Key::ShiftLeft)
                        | EventType::KeyRelease(Key::ShiftRight) => shift = false,
                        EventType::KeyPress(Key::Escape) => {
                            esc = true;
                            if ctrl && shift {
                                (callbacks.on_failsafe)();
                            } else if !ctrl && !shift {
                                (callbacks.on_escape_stop)();
                            }
                        }
                        EventType::KeyRelease(Key::Escape) => esc = false,
                        _ => {}
                    }
                    let _ = esc;
                });
            })
            .map_err(|e| format!("hotkey thread: {e}"))?;
        *self.join.lock() = Some(handle);
        Ok(())
    }

    fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // rdev::listen blocks forever; we cannot join cleanly without process exit.
        // Drop the join handle so the OS reaps on process end.
        let _ = self.join.lock().take();
    }
}
