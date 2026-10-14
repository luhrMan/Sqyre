//! One Storage Access Framework pick at a time: the shell copies the chosen document
//! into app cache and reports its path against the request id.

use parking_lot::Mutex;
use std::path::PathBuf;

#[derive(Debug, Default)]
struct State {
    next_id: i32,
    /// Id of the pick the app is still waiting for.
    pending: Option<i32>,
    /// Finished pick: `None` when the user cancelled.
    done: Option<Option<PathBuf>>,
}

/// Request ids and results for document picks.
#[derive(Debug)]
pub struct DocumentPicks(Mutex<State>);

impl Default for DocumentPicks {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentPicks {
    pub const fn new() -> Self {
        Self(Mutex::new(State {
            next_id: 0,
            pending: None,
            done: None,
        }))
    }

    /// Start a pick and return its id. A newer pick supersedes an unfinished one.
    pub fn begin(&self) -> i32 {
        let mut g = self.0.lock();
        g.next_id = g.next_id.wrapping_add(1);
        g.pending = Some(g.next_id);
        g.done = None;
        g.next_id
    }

    /// Record the shell's answer. An empty path means cancelled; stale ids are dropped.
    pub fn complete(&self, id: i32, path: &str) {
        let mut g = self.0.lock();
        if g.pending != Some(id) {
            return;
        }
        g.pending = None;
        g.done = Some((!path.is_empty()).then(|| PathBuf::from(path)));
    }

    /// Whether a pick is still open in the shell.
    pub fn is_pending(&self) -> bool {
        self.0.lock().pending.is_some()
    }

    /// Take a finished pick: `Some(None)` is a cancel, `None` means nothing finished.
    pub fn take(&self) -> Option<Option<PathBuf>> {
        self.0.lock().done.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_pick_is_taken_once() {
        let picks = DocumentPicks::new();
        let id = picks.begin();
        assert!(picks.is_pending());
        assert_eq!(picks.take(), None);
        picks.complete(id, "/cache/picked/a.png");
        assert!(!picks.is_pending());
        assert_eq!(
            picks.take(),
            Some(Some(PathBuf::from("/cache/picked/a.png")))
        );
        assert_eq!(picks.take(), None);
    }

    #[test]
    fn empty_path_is_a_cancel() {
        let picks = DocumentPicks::new();
        let id = picks.begin();
        picks.complete(id, "");
        assert_eq!(picks.take(), Some(None));
    }

    #[test]
    fn stale_answers_are_dropped() {
        let picks = DocumentPicks::new();
        let old = picks.begin();
        let new = picks.begin();
        assert_ne!(old, new);
        picks.complete(old, "/cache/old.png");
        assert!(picks.is_pending());
        assert_eq!(picks.take(), None);
        picks.complete(new, "/cache/new.png");
        assert_eq!(picks.take(), Some(Some(PathBuf::from("/cache/new.png"))));
    }

    #[test]
    fn answer_without_a_pick_is_dropped() {
        let picks = DocumentPicks::new();
        picks.complete(1, "/cache/x.png");
        assert_eq!(picks.take(), None);
    }

    #[test]
    fn a_new_pick_discards_an_untaken_result() {
        let picks = DocumentPicks::new();
        let id = picks.begin();
        picks.complete(id, "/cache/a.png");
        picks.begin();
        assert_eq!(picks.take(), None);
    }
}
