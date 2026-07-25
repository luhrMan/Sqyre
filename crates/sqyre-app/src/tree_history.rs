//! Macro tree undo/redo.

use serde_yaml::Mapping;
use sqyre_domain::{Action, ActionId};
use sqyre_serialize::{action_from_map, action_to_map_with_uid};
use sqyre_validate::validate_action_tree;

/// Per-macro undo/redo depth cap. Keeps memory bounded for long editing
/// sessions since each entry is a full YAML snapshot of the tree.
const MAX_TREE_HISTORY_ENTRIES: usize = 100;

#[derive(Debug, Clone)]
pub struct TreeSnapshot {
    root_map: Mapping,
    selected: Vec<ActionId>,
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
    pub fn take_snapshot(root: &Action, selected: Vec<ActionId>) -> Result<TreeSnapshot, String> {
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

    /// Undo the last tree mutation. On failure (corrupt/invalid snapshot) the
    /// bad entry is dropped from the stack and `root`/`selected` are left
    /// untouched, so a later undo can still reach earlier, good snapshots.
    pub fn undo(&mut self, root: &mut Action, selected: &mut Vec<ActionId>) -> Result<(), String> {
        if !self.can_undo() {
            return Err("nothing to undo".into());
        }
        let current = snapshot_tree(root, selected.clone())
            .map_err(|e| format!("snapshot current state: {e}"))?;
        let Some(prev) = self.undo.pop() else {
            return Err("nothing to undo".into());
        };
        apply_snapshot(root, selected, prev, &mut self.applying)
            .map_err(|e| format!("restore previous state: {e}"))?;
        self.push_redo(current);
        Ok(())
    }

    /// Redo the last undone mutation. Same drop-bad-entry behavior as [`Self::undo`].
    pub fn redo(&mut self, root: &mut Action, selected: &mut Vec<ActionId>) -> Result<(), String> {
        if !self.can_redo() {
            return Err("nothing to redo".into());
        }
        let current = snapshot_tree(root, selected.clone())
            .map_err(|e| format!("snapshot current state: {e}"))?;
        let Some(next) = self.redo.pop() else {
            return Err("nothing to redo".into());
        };
        apply_snapshot(root, selected, next, &mut self.applying)
            .map_err(|e| format!("restore next state: {e}"))?;
        self.push_undo_only(current);
        Ok(())
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

fn snapshot_tree(root: &Action, selected: Vec<ActionId>) -> Result<TreeSnapshot, String> {
    let root_map = action_to_map_with_uid(root).map_err(|e| e.to_string())?;
    Ok(TreeSnapshot { root_map, selected })
}

fn selection_still_valid(root: &Action, id: ActionId) -> bool {
    root.find_by_id(id).is_some()
        || root.id == id
        || matches!(
            root.resolve_tree_id(id),
            Some(sqyre_domain::TreeNodeRef::ElseFolder { .. })
        )
}

fn apply_snapshot(
    root: &mut Action,
    selected: &mut Vec<ActionId>,
    snap: TreeSnapshot,
    applying: &mut bool,
) -> Result<(), String> {
    // Same nest-depth/type-key decode gates as clipboard paste, plus a
    // semantic re-validate — history is process-local but this still
    // guards against a corrupted snapshot silently landing in the tree.
    let restored = action_from_map(&snap.root_map).map_err(|e| e.to_string())?;
    validate_action_tree(&restored, None).map_err(|e| e.to_string())?;
    *applying = true;
    *root = restored;
    *selected = snap
        .selected
        .into_iter()
        .filter(|&id| selection_still_valid(root, id))
        .collect();
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

    fn record(history: &mut TreeHistory, root: &Action, selected: Vec<ActionId>) {
        let snap = TreeHistory::take_snapshot(root, selected).unwrap();
        history.push_snapshot(snap);
    }

    #[test]
    fn undo_redo_insert_and_remove() {
        let a = wait(1);
        let b = wait(2);
        let mut root = root_loop(vec![a, b]);
        let mut history = TreeHistory::default();
        let mut selected = Vec::new();

        record(&mut history, &root, selected.clone());
        let c = wait(3);
        let c_id = c.id;
        root.children_mut().unwrap().push(c);
        assert_eq!(child_ids(&root).len(), 3);

        assert!(history.undo(&mut root, &mut selected).is_ok());
        assert_eq!(child_ids(&root).len(), 2);

        assert!(history.redo(&mut root, &mut selected).is_ok());
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
        let snap = snapshot_tree(&root, vec![uid_b]).unwrap();
        let restored = action_from_map(&snap.root_map).unwrap();
        let ids = child_ids(&restored);
        assert_eq!(ids, vec![uid_a, uid_b]);
    }

    #[test]
    fn applying_history_does_not_record() {
        let mut root = root_loop(vec![wait(1)]);
        let mut history = TreeHistory::default();
        record(&mut history, &root, Vec::new());
        root.children_mut().unwrap().push(wait(2));
        assert_eq!(history.undo.len(), 1);

        history.applying = true;
        record(&mut history, &root, Vec::new());
        history.applying = false;
        assert_eq!(history.undo.len(), 1);
    }

    #[test]
    fn redo_cleared_on_new_mutation() {
        let mut root = root_loop(vec![wait(1)]);
        let mut history = TreeHistory::default();
        let mut selected = Vec::new();
        record(&mut history, &root, selected.clone());
        root.children_mut().unwrap().push(wait(2));
        assert!(history.undo(&mut root, &mut selected).is_ok());
        assert!(history.can_redo());
        record(&mut history, &root, selected.clone());
        root.children_mut().unwrap().push(wait(3));
        assert!(!history.can_redo());
    }

    #[test]
    fn undo_stack_capped_at_max_entries() {
        let mut root = root_loop(vec![wait(0)]);
        let mut history = TreeHistory::default();
        let selected = Vec::new();

        for i in 0..MAX_TREE_HISTORY_ENTRIES + 10 {
            record(&mut history, &root, selected.clone());
            root.children_mut().unwrap().push(wait(i as i64));
        }

        assert_eq!(history.undo.len(), MAX_TREE_HISTORY_ENTRIES);
    }

    #[test]
    fn redo_stack_capped_at_max_entries() {
        let mut root = root_loop(vec![wait(0)]);
        let mut history = TreeHistory::default();
        let mut selected = Vec::new();

        for i in 0..MAX_TREE_HISTORY_ENTRIES + 10 {
            record(&mut history, &root, selected.clone());
            root.children_mut().unwrap().push(wait(i as i64));
        }
        for _ in 0..MAX_TREE_HISTORY_ENTRIES + 10 {
            let _ = history.undo(&mut root, &mut selected);
        }

        assert_eq!(history.redo.len(), MAX_TREE_HISTORY_ENTRIES);
    }

    #[test]
    fn undo_with_corrupt_snapshot_drops_entry_and_leaves_root_untouched() {
        let mut root = root_loop(vec![wait(1)]);
        let mut history = TreeHistory::default();
        let mut selected = Vec::new();

        record(&mut history, &root, selected.clone());
        // Corrupt the pushed snapshot so decode fails on undo.
        history.undo[0].root_map.insert(
            serde_yaml::Value::String("type".into()),
            serde_yaml::Value::String("not-a-real-type".into()),
        );
        root.children_mut().unwrap().push(wait(2));

        let before = child_ids(&root);
        let err = history.undo(&mut root, &mut selected).unwrap_err();
        assert!(!err.is_empty());
        // Root is untouched and the bad entry is gone, not left to fail again.
        assert_eq!(child_ids(&root), before);
        assert!(!history.can_undo());
    }
}
