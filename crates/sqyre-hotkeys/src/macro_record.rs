//! Global input capture for macro recording (move / click / key / Esc stop).

use parking_lot::Mutex;
use std::collections::HashSet;
#[cfg(feature = "hooks")]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

type AbsolutePosFn = Arc<dyn Fn() -> Option<(i32, i32)> + Send + Sync>;
type CoordinateSourceFn = Arc<dyn Fn(&'static str) + Send + Sync>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HookCoordinates {
    #[default]
    Absolute,
    Relative,
}

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
    PositionUnavailable,
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

#[derive(Default)]
struct Inner {
    armed: bool,
    last_pos: Option<(i32, i32)>,
    hook_coordinates: HookCoordinates,
    coordinate_source: Option<&'static str>,
    on_coordinate_source: Option<CoordinateSourceFn>,
    position_unavailable: bool,
    events: Vec<MacroRecordEvent>,
    /// Set when Esc ends an armed session (UI takes finished events).
    finished: bool,
    cancelled: bool,
    started_at: Option<Instant>,
    /// Last focused-key set (Windows Raw Input path).
    prev_keys: HashSet<String>,
    /// Compositor-absolute pointer (portal cursor on Wayland). Sampled on arm /
    /// move / click — Wayland evdev REL moves are raw counts, not screen pixels.
    absolute_pos: Option<AbsolutePosFn>,
}

impl std::fmt::Debug for Inner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inner")
            .field("armed", &self.armed)
            .field("events", &self.events)
            .field("finished", &self.finished)
            .field("cancelled", &self.cancelled)
            .field("absolute_pos", &self.absolute_pos.is_some())
            .finish_non_exhaustive()
    }
}

impl Inner {
    fn sample_position(&mut self, hook: Option<(i32, i32)>) -> Option<(i32, i32)> {
        let absolute = self.absolute_pos.as_ref().and_then(|f| f());
        let pos = absolute.or_else(|| {
            (self.hook_coordinates == HookCoordinates::Absolute)
                .then(|| hook.or(self.last_pos))
                .flatten()
        });
        let source = if absolute.is_some() {
            "portal"
        } else if pos.is_some() {
            "hook"
        } else {
            "unavailable"
        };
        if self.armed && self.coordinate_source != Some(source) {
            if let Some(f) = &self.on_coordinate_source {
                f(source);
            }
            if pos.is_none() {
                self.events.push(MacroRecordEvent::PositionUnavailable);
            }
            self.coordinate_source = Some(source);
        }
        if self.armed && pos.is_none() {
            self.position_unavailable = true;
        }
        self.last_pos = pos;
        pos
    }
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
        let absolute_pos = g.absolute_pos.clone();
        let hook_coordinates = g.hook_coordinates;
        let on_coordinate_source = g.on_coordinate_source.clone();
        let pos = g
            .last_pos
            .or_else(|| (hook_coordinates == HookCoordinates::Absolute).then_some((0, 0)));
        *g = Inner {
            armed: true,
            last_pos: pos,
            events: Vec::new(),
            finished: false,
            cancelled: false,
            started_at: Some(Instant::now()),
            prev_keys: HashSet::new(),
            absolute_pos,
            hook_coordinates,
            on_coordinate_source,
            coordinate_source: None,
            position_unavailable: false,
        };
        g.sample_position(None);
        sync_hook_wants_moves(true);
    }

    /// Compositor-absolute pointer sampled on each move/click (portal cursor on
    /// Wayland, where hook mouse coordinates are relative counts, not pixels).
    pub fn set_absolute_pos(&self, f: impl Fn() -> Option<(i32, i32)> + Send + Sync + 'static) {
        self.inner.lock().absolute_pos = Some(Arc::new(f));
    }

    pub fn set_hook_coordinates(&self, coordinates: HookCoordinates) {
        let mut g = self.inner.lock();
        g.hook_coordinates = coordinates;
        g.last_pos = None;
    }

    pub fn set_coordinate_source_logger(&self, f: impl Fn(&'static str) + Send + Sync + 'static) {
        self.inner.lock().on_coordinate_source = Some(Arc::new(f));
    }

    pub fn position_unavailable(&self) -> bool {
        self.inner.lock().position_unavailable
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
        let position = g.last_pos.map_or_else(
            || "cursor position unavailable".to_string(),
            |(x, y)| format!("({x}, {y})"),
        );
        let n = g.events.len();
        Some(format!(
            "Recording macro — {position} — {n} events — Esc to finish"
        ))
    }

    pub fn last_pos(&self) -> Option<(i32, i32)> {
        self.inner.lock().last_pos
    }

    pub fn started_at(&self) -> Option<Instant> {
        self.inner.lock().started_at
    }

    /// Update pointer without recording a move event (e.g. click coordinates).
    pub fn set_last_pos(&self, x: i32, y: i32) {
        let mut g = self.inner.lock();
        if g.hook_coordinates == HookCoordinates::Absolute {
            g.last_pos = Some((x, y));
        }
    }

    /// Hotkey thread: track pointer (always, so arm starts with a real position).
    pub fn on_mouse_move(&self, x: i32, y: i32) {
        let mut g = self.inner.lock();
        let Some(pos) = g.sample_position(Some((x, y))) else {
            return;
        };
        if g.armed {
            g.events.push(MacroRecordEvent::MouseMove {
                x: pos.0,
                y: pos.1,
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
        let Some((x, y)) = g.sample_position(None) else {
            return;
        };
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

    #[test]
    fn mouse_move_uses_absolute_pos_when_available() {
        let b = MacroRecordBridge::new();
        b.set_absolute_pos(|| Some((500, 600)));
        b.arm();
        // Wayland evdev REL counts are not screen coordinates.
        b.on_mouse_move(3, 4);
        assert_eq!(b.last_pos(), Some((500, 600)));
        assert!(b.on_escape());
        let (_, events) = b.take_finished().expect("finished");
        assert!(events
            .iter()
            .any(|e| matches!(e, MacroRecordEvent::MouseMove { x: 500, y: 600, .. })));
    }

    #[test]
    fn button_stamps_absolute_pos_without_prior_move() {
        let b = MacroRecordBridge::new();
        b.set_absolute_pos(|| Some((3212, 528)));
        b.arm();
        b.on_button(RecordMouseButton::Left, true);
        assert!(b.on_escape());
        let (_, events) = b.take_finished().expect("finished");
        assert!(events.iter().any(|e| matches!(
            e,
            MacroRecordEvent::Button {
                x: 3212,
                y: 528,
                ..
            }
        )));
    }

    #[test]
    fn arm_seeds_last_pos_from_absolute_pos() {
        let b = MacroRecordBridge::new();
        b.set_absolute_pos(|| Some((40, 50)));
        b.arm();
        assert_eq!(b.last_pos(), Some((40, 50)));
    }

    #[test]
    fn trusted_hook_coords_used_when_absolute_pos_missing() {
        let b = MacroRecordBridge::new();
        b.set_absolute_pos(|| None);
        b.arm();
        b.on_mouse_move(7, 9);
        b.on_button(RecordMouseButton::Left, true);
        assert_eq!(b.last_pos(), Some((7, 9)));
        assert!(!b.position_unavailable());
        assert!(matches!(
            b.peek_events().last(),
            Some(MacroRecordEvent::Button { x: 7, y: 9, .. })
        ));
    }

    #[test]
    fn relative_hook_never_records_positions_without_provider() {
        let b = MacroRecordBridge::new();
        b.set_hook_coordinates(HookCoordinates::Relative);
        b.set_absolute_pos(|| None);
        b.set_last_pos(99, 88);
        b.arm();
        b.on_mouse_move(7, 9);
        b.on_button(RecordMouseButton::Left, true);
        b.on_button(RecordMouseButton::Left, false);
        assert_eq!(b.last_pos(), None);
        assert!(b.position_unavailable());
        assert!(b.status_label().unwrap().contains("position unavailable"));
        assert!(b.on_escape());
        let (_, events) = b.take_finished().unwrap();
        assert!(matches!(
            events.as_slice(),
            [MacroRecordEvent::PositionUnavailable]
        ));
        assert!(b.position_unavailable());
    }

    #[test]
    fn relative_hook_recovers_and_logs_source_transitions_once() {
        let b = MacroRecordBridge::new();
        b.set_hook_coordinates(HookCoordinates::Relative);
        let position = Arc::new(Mutex::new(None));
        let provider = position.clone();
        b.set_absolute_pos(move || *provider.lock());
        let sources = Arc::new(Mutex::new(Vec::new()));
        let logged = sources.clone();
        b.set_coordinate_source_logger(move |source| logged.lock().push(source));
        b.arm();
        b.on_mouse_move(7, 9);
        b.on_button(RecordMouseButton::Left, true);
        *position.lock() = Some((100, 200));
        b.on_mouse_move(999, 999);
        b.on_button(RecordMouseButton::Left, true);
        b.on_button(RecordMouseButton::Left, false);
        assert_eq!(b.last_pos(), Some((100, 200)));
        *position.lock() = None;
        b.on_mouse_move(123, 456);
        b.on_button(RecordMouseButton::Right, true);
        assert_eq!(b.last_pos(), None);
        assert_eq!(*sources.lock(), ["unavailable", "portal", "unavailable"]);
        assert!(b.position_unavailable());
        b.on_escape();
        let (_, events) = b.take_finished().unwrap();
        assert!(matches!(
            events.as_slice(),
            [
                MacroRecordEvent::PositionUnavailable,
                MacroRecordEvent::MouseMove { x: 100, y: 200, .. },
                MacroRecordEvent::Button {
                    x: 100,
                    y: 200,
                    pressed: true,
                    ..
                },
                MacroRecordEvent::Button {
                    x: 100,
                    y: 200,
                    pressed: false,
                    ..
                },
                MacroRecordEvent::PositionUnavailable,
            ]
        ));
        *position.lock() = Some((10, 20));
        b.arm();
        assert!(!b.position_unavailable());
    }
}
