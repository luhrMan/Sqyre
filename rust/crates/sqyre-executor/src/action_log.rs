//! Per-action execution log sink (keyed by [`ActionId`]).

use sqyre_domain::ActionId;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Max lines retained per action (oldest dropped).
pub const MAX_LINES_PER_ACTION: usize = 200;

/// Receives log lines tagged with the action that produced them.
pub trait ActionLogger: Send + Sync {
    fn log(&self, action_id: ActionId, message: String);
}

/// Thread-safe per-action line buffer for the UI.
#[derive(Clone, Default)]
pub struct SharedActionLog {
    inner: Arc<Mutex<HashMap<ActionId, Vec<String>>>>,
}

impl SharedActionLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&self) {
        self.inner.lock().unwrap().clear();
    }

    pub fn lines_for(&self, action_id: ActionId) -> Vec<String> {
        self.inner
            .lock()
            .unwrap()
            .get(&action_id)
            .cloned()
            .unwrap_or_default()
    }
}

impl ActionLogger for SharedActionLog {
    fn log(&self, action_id: ActionId, message: String) {
        let mut map = self.inner.lock().unwrap();
        let lines = map.entry(action_id).or_default();
        lines.push(message);
        if lines.len() > MAX_LINES_PER_ACTION {
            let drop = lines.len() - MAX_LINES_PER_ACTION;
            lines.drain(0..drop);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_lines_per_action() {
        let log = SharedActionLog::new();
        let id = ActionId::new();
        for i in 0..(MAX_LINES_PER_ACTION + 50) {
            log.log(id, format!("line-{i}"));
        }
        let lines = log.lines_for(id);
        assert_eq!(lines.len(), MAX_LINES_PER_ACTION);
        assert_eq!(lines[0], format!("line-{}", 50));
        assert_eq!(
            lines.last().unwrap(),
            &format!("line-{}", MAX_LINES_PER_ACTION + 49)
        );
    }

    #[test]
    fn isolates_actions_and_clear_wipes_all() {
        let log = SharedActionLog::new();
        let a = ActionId::new();
        let b = ActionId::new();
        log.log(a, "from-a".into());
        log.log(b, "from-b".into());
        assert_eq!(log.lines_for(a), vec!["from-a".to_string()]);
        assert_eq!(log.lines_for(b), vec!["from-b".to_string()]);
        log.clear();
        assert!(log.lines_for(a).is_empty());
        assert!(log.lines_for(b).is_empty());
    }
}
