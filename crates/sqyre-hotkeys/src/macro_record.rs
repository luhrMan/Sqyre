//! Global input capture for macro recording (move / click / key / Esc stop).

use parking_lot::Mutex;
use std::collections::HashSet;
#[cfg(feature = "hooks")]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Fast path for OS hooks: skip mouse-move work unless macro recording is armed.
#[cfg(feature = "hooks")]
static HOOK_WANTS_MOVES: AtomicBool = AtomicBool::new(false);

#[cfg(feature = "hooks")]
pub(crate) fn hook_wants_mouse_moves() -> bool {
    HOOK_WANTS_MOVES.load(Ordering::Relaxed)
}

fn sync_hook_wants_moves(armed: bool) {
    #[cfg(feature = "hooks")]
    HOOK_WANTS_MOVES.store(armed, Ordering::Relaxed);
    #[cfg(not(feature = "hooks"))]
    let _ = armed;
}

/// Mouse button for recorded click events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordMouseButton {
    Left,
    Right,
    Middle,
}

impl RecordMouseButton {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Middle => "middle",
        }
    }
}

/// One raw input sample while macro recording is armed.
#[derive(Debug, Clone)]
pub enum MacroRecordEvent {
    MouseMove {
        x: i32,
        y: i32,
        at: Instant,
    },
    Button {
        button: RecordMouseButton,
        pressed: bool,
        x: i32,
        y: i32,
        at: Instant,
    },
    Key {
        name: String,
        pressed: bool,
        at: Instant,
    },
}

#[derive(Debug, Default)]
struct Inner {
    armed: bool,
    last_pos: (i32, i32),
    events: Vec<MacroRecordEvent>,
    /// Set when Esc ends an armed session (UI takes finished events).
    finished: bool,
    cancelled: bool,
    started_at: Option<Instant>,
    /// Last focused-key set (Windows Raw Input path).
    prev_keys: HashSet<String>,
}

/// Shared bridge between the hotkey thread and the macro-record UI.
#[derive(Clone, Default)]
pub struct MacroRecordBridge {
    inner: Arc<Mutex<Inner>>,
}

impl MacroRecordBridge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn arm(&self) {
        let mut g = self.inner.lock();
        let pos = g.last_pos;
        *g = Inner {
            armed: true,
            last_pos: pos,
            events: Vec::new(),
            finished: false,
            cancelled: false,
            started_at: Some(Instant::now()),
            prev_keys: HashSet::new(),
        };
        sync_hook_wants_moves(true);
    }

    pub fn disarm(&self) {
        let mut g = self.inner.lock();
        g.armed = false;
        sync_hook_wants_moves(false);
    }

    pub fn is_armed(&self) -> bool {
        self.inner.lock().armed
    }

    pub fn is_active(&self) -> bool {
        let g = self.inner.lock();
        g.armed || g.finished
    }

    pub fn status_label(&self) -> Option<String> {
        let g = self.inner.lock();
        if !g.armed {
            return None;
        }
        let (x, y) = g.last_pos;
        let n = g.events.len();
        Some(format!(
            "Recording macro — ({x}, {y}) — {n} events — Esc to finish"
        ))
    }

    pub fn last_pos(&self) -> (i32, i32) {
        self.inner.lock().last_pos
    }

    pub fn started_at(&self) -> Option<Instant> {
        self.inner.lock().started_at
    }

    /// Update pointer without recording a move event (e.g. click coordinates).
    pub fn set_last_pos(&self, x: i32, y: i32) {
        self.inner.lock().last_pos = (x, y);
    }

    /// Hotkey thread: track pointer (always, so arm starts with a real position).
    pub fn on_mouse_move(&self, x: i32, y: i32) {
        let mut g = self.inner.lock();
        g.last_pos = (x, y);
        if g.armed {
            g.events.push(MacroRecordEvent::MouseMove {
                x,
                y,
                at: Instant::now(),
            });
        }
    }

    /// Hotkey thread: mouse button press/release while armed.
    pub fn on_button(&self, button: RecordMouseButton, pressed: bool) {
        let mut g = self.inner.lock();
        if !g.armed {
            return;
        }
        let (x, y) = g.last_pos;
        g.events.push(MacroRecordEvent::Button {
            button,
            pressed,
            x,
            y,
            at: Instant::now(),
        });
    }

    /// Hotkey thread / focused-key feed: key press/release while armed.
    ///
    /// OS key-repeat is ignored: a held key records at most one down until
    /// release. Returns `true` when Esc press finished the session (caller
    /// should not also treat Esc as macro-stop).
    pub fn on_key(&self, name: &str, pressed: bool) -> bool {
        let mut g = self.inner.lock();
        if !g.armed {
            return false;
        }
        if pressed && name == "esc" {
            g.armed = false;
            g.finished = true;
            sync_hook_wants_moves(false);
            return true;
        }
        // Do not record the Esc that stops recording.
        if name == "esc" {
            return false;
        }
        if pressed {
            if !g.prev_keys.insert(name.to_string()) {
                // Already down — ignore auto-repeat.
                return false;
            }
        } else if !g.prev_keys.remove(name) {
            // Spurious release.
            return false;
        }
        g.events.push(MacroRecordEvent::Key {
            name: name.to_string(),
            pressed,
            at: Instant::now(),
        });
        false
    }

    /// Hotkey thread: Esc while armed ends the session.
    pub fn on_escape(&self) -> bool {
        let mut g = self.inner.lock();
        if g.armed {
            g.armed = false;
            g.finished = true;
            sync_hook_wants_moves(false);
            true
        } else {
            false
        }
    }

    /// Diff `pressed` against the previous key snapshot and emit key events.
    ///
    /// Used by the UI each frame while recording (and on Windows via async key
    /// poll) so presses are not lost when OS hooks / focus are unreliable.
    pub fn sync_pressed_keys(&self, pressed: &HashSet<&str>) {
        let mut g = self.inner.lock();
        if !g.armed {
            g.prev_keys.clear();
            return;
        }
        let prev = g.prev_keys.clone();
        let now: HashSet<String> = pressed.iter().map(|s| (*s).to_string()).collect();
        for name in now.difference(&prev) {
            if name == "esc" {
                continue;
            }
            g.events.push(MacroRecordEvent::Key {
                name: name.clone(),
                pressed: true,
                at: Instant::now(),
            });
        }
        for name in prev.difference(&now) {
            if name == "esc" {
                continue;
            }
            g.events.push(MacroRecordEvent::Key {
                name: name.clone(),
                pressed: false,
                at: Instant::now(),
            });
        }
        g.prev_keys = now;
    }

    /// Cancel without producing a review list.
    pub fn cancel(&self) {
        let mut g = self.inner.lock();
        g.armed = false;
        g.finished = false;
        g.cancelled = true;
        g.events.clear();
        g.started_at = None;
        sync_hook_wants_moves(false);
    }

    /// Snapshot of events recorded so far (while armed or until taken).
    pub fn peek_events(&self) -> Vec<MacroRecordEvent> {
        self.inner.lock().events.clone()
    }

    pub fn take_cancelled(&self) -> bool {
        let mut g = self.inner.lock();
        let c = g.cancelled;
        g.cancelled = false;
        c
    }

    /// Take finished event stream (after Esc). Returns `None` until finished.
    pub fn take_finished(&self) -> Option<(Instant, Vec<MacroRecordEvent>)> {
        let mut g = self.inner.lock();
        if !g.finished {
            return None;
        }
        g.finished = false;
        let started = g.started_at.take().unwrap_or_else(Instant::now);
        let events = std::mem::take(&mut g.events);
        Some((started, events))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_keys_until_esc() {
        let b = MacroRecordBridge::new();
        b.arm();
        assert!(!b.on_key("a", true));
        assert!(!b.on_key("a", false));
        assert!(b.on_key("esc", true));
        assert!(!b.is_armed());
        let (_, events) = b.take_finished().expect("finished");
        assert_eq!(events.len(), 2);
        assert!(events.iter().all(|e| !matches!(
            e,
            MacroRecordEvent::Key { name, .. } if name == "esc"
        )));
    }

    #[test]
    fn key_hold_records_single_down() {
        let b = MacroRecordBridge::new();
        b.arm();
        assert!(!b.on_key("a", true));
        assert!(!b.on_key("a", true)); // OS auto-repeat
        assert!(!b.on_key("a", true));
        assert!(!b.on_key("a", false));
        assert!(b.on_escape());
        let (_, events) = b.take_finished().expect("finished");
        let keys: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                MacroRecordEvent::Key { name, pressed, .. } => Some((name.as_str(), *pressed)),
                _ => None,
            })
            .collect();
        assert_eq!(keys, vec![("a", true), ("a", false)]);
    }

    #[test]
    fn sync_pressed_keys_ignores_hold() {
        let b = MacroRecordBridge::new();
        b.arm();
        let mut set = HashSet::new();
        set.insert("a");
        b.sync_pressed_keys(&set);
        b.sync_pressed_keys(&set);
        b.sync_pressed_keys(&set);
        set.clear();
        b.sync_pressed_keys(&set);
        assert!(b.on_escape());
        let (_, events) = b.take_finished().expect("finished");
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn esc_stops_recording() {
        let b = MacroRecordBridge::new();
        b.arm();
        b.on_mouse_move(1, 2);
        assert!(b.on_escape());
        assert!(!b.is_armed());
        let (_, events) = b.take_finished().expect("finished");
        assert!(events
            .iter()
            .any(|e| matches!(e, MacroRecordEvent::MouseMove { .. })));
        assert!(b.take_finished().is_none());
    }

    #[test]
    fn status_while_armed() {
        let b = MacroRecordBridge::new();
        b.arm();
        b.on_mouse_move(5, 6);
        let msg = b.status_label().expect("armed");
        assert!(msg.contains("(5, 6)"), "{msg}");
        assert!(msg.contains("Esc to finish"), "{msg}");
    }
}
