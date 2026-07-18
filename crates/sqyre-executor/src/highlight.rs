//! Active-action highlighting during macro execution.

use parking_lot::Mutex;
use sqyre_domain::ActionId;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// How an action should be highlighted during execution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HighlightKind {
    /// Clears a highlight. Empty `action_id` clears every highlight.
    None,
    /// Moving "cursor" on the action currently running.
    Simple,
    /// Horizontal progress fill (0..1) for container actions.
    Fill,
}

/// Event sent to the UI to update execution highlight of an action.
#[derive(Debug, Clone, PartialEq)]
pub struct HighlightEvent {
    pub macro_name: String,
    pub action_id: Option<ActionId>,
    pub kind: HighlightKind,
    /// 0..1, only meaningful for [`HighlightKind::Fill`].
    pub fill: f64,
}

/// Receives highlight events from the executor (UI marshals onto its thread).
pub trait ActionHighlighter: Send + Sync {
    fn emit(&self, event: HighlightEvent);
}

/// Thread-safe highlight state for the egui shell.
#[derive(Clone, Default)]
pub struct SharedHighlighter {
    enabled: Arc<AtomicBool>,
    inner: Arc<Mutex<HighlightState>>,
}

#[derive(Debug, Clone, Default)]
struct HighlightState {
    macro_name: String,
    cursor: Option<ActionId>,
    fills: HashMap<ActionId, f64>,
}

/// Snapshot consumed by the UI each frame.
#[derive(Debug, Clone, Default)]
pub struct HighlightSnapshot {
    pub macro_name: String,
    pub cursor: Option<ActionId>,
    pub fills: HashMap<ActionId, f64>,
}

impl SharedHighlighter {
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(true)),
            inner: Arc::new(Mutex::new(HighlightState::default())),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
        if !enabled {
            self.clear_all();
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    pub fn snapshot(&self) -> HighlightSnapshot {
        let g = self.inner.lock();
        HighlightSnapshot {
            macro_name: g.macro_name.clone(),
            cursor: g.cursor,
            fills: g.fills.clone(),
        }
    }

    pub fn clear_all(&self) {
        let mut g = self.inner.lock();
        g.macro_name.clear();
        g.cursor = None;
        g.fills.clear();
    }

    fn apply(&self, event: HighlightEvent) {
        let mut g = self.inner.lock();
        match event.kind {
            HighlightKind::None => {
                if event.action_id.is_none() {
                    g.macro_name.clear();
                    g.cursor = None;
                    g.fills.clear();
                    return;
                }
                if let Some(id) = event.action_id {
                    g.fills.remove(&id);
                    if g.cursor == Some(id) {
                        g.cursor = None;
                    }
                }
            }
            HighlightKind::Simple => {
                g.macro_name = event.macro_name;
                g.cursor = event.action_id;
            }
            HighlightKind::Fill => {
                let Some(id) = event.action_id else {
                    return;
                };
                g.macro_name = event.macro_name;
                let fill = event.fill.clamp(0.0, 1.0);
                g.fills.insert(id, fill);
            }
        }
    }
}

impl ActionHighlighter for SharedHighlighter {
    fn emit(&self, event: HighlightEvent) {
        // Clear-all bypasses the enabled flag (macro completion / feature off).
        let clear_all = event.kind == HighlightKind::None && event.action_id.is_none();
        if !clear_all && !self.enabled.load(Ordering::SeqCst) {
            return;
        }
        self.apply(event);
    }
}

/// Emit helpers used by the executor.
pub fn highlight_cursor(
    highlighter: Option<&dyn ActionHighlighter>,
    macro_name: &str,
    action_id: Option<ActionId>,
) {
    let Some(h) = highlighter else {
        return;
    };
    h.emit(HighlightEvent {
        macro_name: macro_name.to_string(),
        action_id,
        kind: HighlightKind::Simple,
        fill: 0.0,
    });
}

pub fn highlight_fill(
    highlighter: Option<&dyn ActionHighlighter>,
    macro_name: &str,
    action_id: ActionId,
    fill: f64,
) {
    let Some(h) = highlighter else {
        return;
    };
    h.emit(HighlightEvent {
        macro_name: macro_name.to_string(),
        action_id: Some(action_id),
        kind: HighlightKind::Fill,
        fill,
    });
}

pub fn highlight_clear(
    highlighter: Option<&dyn ActionHighlighter>,
    macro_name: &str,
    action_id: ActionId,
) {
    let Some(h) = highlighter else {
        return;
    };
    h.emit(HighlightEvent {
        macro_name: macro_name.to_string(),
        action_id: Some(action_id),
        kind: HighlightKind::None,
        fill: 0.0,
    });
}

pub fn clear_highlights(highlighter: Option<&dyn ActionHighlighter>) {
    let Some(h) = highlighter else {
        return;
    };
    h.emit(HighlightEvent {
        macro_name: String::new(),
        action_id: None,
        kind: HighlightKind::None,
        fill: 0.0,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_fill_and_clear() {
        let h = SharedHighlighter::new();
        let a = ActionId::new();
        let b = ActionId::new();
        highlight_cursor(Some(&h), "m", Some(a));
        let snap = h.snapshot();
        assert_eq!(snap.macro_name, "m");
        assert_eq!(snap.cursor, Some(a));

        highlight_fill(Some(&h), "m", b, 0.5);
        let snap = h.snapshot();
        assert_eq!(snap.fills.get(&b).copied(), Some(0.5));
        assert_eq!(snap.cursor, Some(a));

        highlight_clear(Some(&h), "m", b);
        assert!(!h.snapshot().fills.contains_key(&b));

        clear_highlights(Some(&h));
        let snap = h.snapshot();
        assert!(snap.cursor.is_none());
        assert!(snap.fills.is_empty());
    }

    #[test]
    fn disabled_skips_emit_but_clear_all_works() {
        let h = SharedHighlighter::new();
        let a = ActionId::new();
        highlight_cursor(Some(&h), "m", Some(a));
        h.set_enabled(false);
        assert!(h.snapshot().cursor.is_none());

        highlight_cursor(Some(&h), "m", Some(a));
        assert!(h.snapshot().cursor.is_none());

        h.set_enabled(true);
        highlight_cursor(Some(&h), "m", Some(a));
        assert_eq!(h.snapshot().cursor, Some(a));
        clear_highlights(Some(&h));
        assert!(h.snapshot().cursor.is_none());
    }
}
