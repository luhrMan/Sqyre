//! Macro tree undo/redo.

use serde_yaml::Mapping;
use sqyre_domain::{Action, ActionId};
use sqyre_serialize::{action_from_map, action_to_map_with_uid};

const MAX_TREE_HISTORY_ENTRIES: usize = 50;

#[derive(Debug, Clone)]
pub struct TreeSnapshot {
    root_map: Mapping,
    selected: Option<ActionId>,
}

/// Per-macro undo/redo stacks of UID-preserving tree snapshots.
#[derive(Debug, Default)]
pub struct TreeHistory {
    undo: Vec<TreeSnapshot>,
    redo: Vec<TreeSnapshot>,
    applying: bool,
}

impl TreeHistory {
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Build a snapshot without holding `&mut TreeHistory` (for borrow splitting).
    pub fn take_snapshot(
        root: &Action,
        selected: Option<ActionId>,
    ) -> Result<TreeSnapshot, String> {
        snapshot_tree(root, selected)
    }

    /// Push a pre-built snapshot (from [`Self::take_snapshot`]).
    pub fn push_snapshot(&mut self, snap: TreeSnapshot) {
        if self.applying {
            return;
        }
        self.push_undo_clearing_redo(snap);
    }

    /// Drop the most recent undo entry (used when discarding a provisional insert).
    pub fn pop_last_undo(&mut self) {
        let _ = self.undo.pop();
    }

    pub fn undo(
        &mut self,
        root: &mut Action,
        selected: &mut Option<ActionId>,
    ) -> bool {
        if !self.can_undo() {
            return false;
        }
        let Ok(current) = snapshot_tree(root, *selected) else {
            eprintln!("tree undo: snapshot current failed");
            return false;
        };
        let Some(prev) = self.undo.pop() else {
            return false;
        };
        self.push_redo(current);
        if let Err(e) = apply_snapshot(root, selected, prev, &mut self.applying) {
            eprintln!("tree undo: restore failed: {e}");
            return false;
        }
        true
    }

    pub fn redo(
        &mut self,
        root: &mut Action,
        selected: &mut Option<ActionId>,
    ) -> bool {
        if !self.can_redo() {
            return false;
        }
        let Ok(current) = snapshot_tree(root, *selected) else {
            eprintln!("tree redo: snapshot current failed");
            return false;
        };
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.push_undo_only(current);
        if let Err(e) = apply_snapshot(root, selected, next, &mut self.applying) {
            eprintln!("tree redo: restore failed: {e}");
            return false;
        }
        true
    }

    fn push_undo_clearing_redo(&mut self, snap: TreeSnapshot) {
        self.undo.push(snap);
        trim(&mut self.undo);
        self.redo.clear();
    }

    fn push_undo_only(&mut self, snap: TreeSnapshot) {
        self.undo.push(snap);
        trim(&mut self.undo);
    }

    fn push_redo(&mut self, snap: TreeSnapshot) {
        self.redo.push(snap);
        trim(&mut self.redo);
    }
}

fn trim(stack: &mut Vec<TreeSnapshot>) {
    if stack.len() > MAX_TREE_HISTORY_ENTRIES {
        let drop = stack.len() - MAX_TREE_HISTORY_ENTRIES;
        stack.drain(0..drop);
    }
}

fn snapshot_tree(root: &Action, selected: Option<ActionId>) -> Result<TreeSnapshot, String> {
    let root_map = action_to_map_with_uid(root).map_err(|e| e.to_string())?;
    Ok(TreeSnapshot {
        root_map,
        selected,
    })
}

fn apply_snapshot(
    root: &mut Action,
    selected: &mut Option<ActionId>,
    snap: TreeSnapshot,
    applying: &mut bool,
) -> Result<(), String> {
    let restored = action_from_map(&snap.root_map).map_err(|e| e.to_string())?;
    *applying = true;
    *root = restored;
    *selected = snap.selected.filter(|id| root.find_by_id(*id).is_some() || root.id == *id);
    *applying = false;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::{root_loop, ActionKind, ScalarValue};

    fn wait(ms: i64) -> Action {
        Action {
            id: ActionId::new(),
            kind: ActionKind::Wait {
                time: ScalarValue::Int(ms),
            },
        }
    }

    fn child_ids(root: &Action) -> Vec<ActionId> {
        root.children().iter().map(|c| c.id).collect()
    }

    fn record(history: &mut TreeHistory, root: &Action, selected: Option<ActionId>) {
        let snap = TreeHistory::take_snapshot(root, selected).unwrap();
        history.push_snapshot(snap);
    }

    #[test]
    fn undo_redo_insert_and_remove() {
        let a = wait(1);
        let b = wait(2);
        let mut root = root_loop(vec![a, b]);
        let mut history = TreeHistory::default();
        let mut selected = None;

        record(&mut history, &root, selected);
        let c = wait(3);
        let c_id = c.id;
        root.children_mut().unwrap().push(c);
        assert_eq!(child_ids(&root).len(), 3);

        assert!(history.undo(&mut root, &mut selected));
        assert_eq!(child_ids(&root).len(), 2);

        assert!(history.redo(&mut root, &mut selected));
        let ids = child_ids(&root);
        assert_eq!(ids.len(), 3);
        assert_eq!(ids[2], c_id);
    }

    #[test]
    fn snapshot_preserves_uids() {
        let a = wait(1);
        let b = wait(2);
        let uid_a = a.id;
        let uid_b = b.id;
        let root = root_loop(vec![a, b]);
        let snap = snapshot_tree(&root, Some(uid_b)).unwrap();
        let restored = action_from_map(&snap.root_map).unwrap();
        let ids = child_ids(&restored);
        assert_eq!(ids, vec![uid_a, uid_b]);
    }

    #[test]
    fn applying_history_does_not_record() {
        let mut root = root_loop(vec![wait(1)]);
        let mut history = TreeHistory::default();
        record(&mut history, &root, None);
        root.children_mut().unwrap().push(wait(2));
        assert_eq!(history.undo.len(), 1);

        history.applying = true;
        record(&mut history, &root, None);
        history.applying = false;
        assert_eq!(history.undo.len(), 1);
    }

    #[test]
    fn redo_cleared_on_new_mutation() {
        let mut root = root_loop(vec![wait(1)]);
        let mut history = TreeHistory::default();
        let mut selected = None;
        record(&mut history, &root, selected);
        root.children_mut().unwrap().push(wait(2));
        assert!(history.undo(&mut root, &mut selected));
        assert!(history.can_redo());
        record(&mut history, &root, selected);
        root.children_mut().unwrap().push(wait(3));
        assert!(!history.can_redo());
    }
}
