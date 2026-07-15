//! Pure helpers for mapping tree drag-and-drop positions onto domain insert slots.

use egui_ltreeview::DirPosition;
use sqyre_domain::{Action, ActionId, InsertSlot};
use std::collections::HashMap;

/// Map an `egui_ltreeview` drop position to an [`InsertSlot`] using the node→action map.
pub(crate) fn insert_slot_from_dir_position(
    position: DirPosition<u64>,
    node_actions: &HashMap<u64, ActionId>,
) -> Option<InsertSlot> {
    match position {
        DirPosition::First => Some(InsertSlot::First),
        DirPosition::Last => Some(InsertSlot::Last),
        DirPosition::Before(nid) => node_actions.get(&nid).copied().map(InsertSlot::Before),
        DirPosition::After(nid) => node_actions.get(&nid).copied().map(InsertSlot::After),
    }
}

/// True when dropping `src` onto `target` would nest a node into itself / a descendant.
pub(crate) fn is_invalid_tree_drop(root: &Action, src: ActionId, target: ActionId) -> bool {
    src == target || root.find_by_id(src).is_some_and(|s| s.contains_id(target))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::{root_loop, ActionKind, ScalarValue};

    #[test]
    fn maps_dir_positions() {
        let a = ActionId::new();
        let b = ActionId::new();
        let map = HashMap::from([(1u64, a), (2u64, b)]);
        assert_eq!(
            insert_slot_from_dir_position(DirPosition::First, &map),
            Some(InsertSlot::First)
        );
        assert_eq!(
            insert_slot_from_dir_position(DirPosition::Last, &map),
            Some(InsertSlot::Last)
        );
        assert_eq!(
            insert_slot_from_dir_position(DirPosition::Before(2), &map),
            Some(InsertSlot::Before(b))
        );
        assert_eq!(
            insert_slot_from_dir_position(DirPosition::After(1), &map),
            Some(InsertSlot::After(a))
        );
        assert_eq!(
            insert_slot_from_dir_position(DirPosition::Before(99), &map),
            None
        );
    }

    #[test]
    fn rejects_drop_into_self_or_descendant() {
        let child_id = ActionId::new();
        let grandchild_id = ActionId::new();
        let child = Action {
            id: child_id,
            kind: ActionKind::Loop {
                name: "inner".into(),
                count: ScalarValue::Int(1),
                subactions: vec![Action {
                    id: grandchild_id,
                    kind: ActionKind::Wait {
                        time: ScalarValue::Int(1),
                    },
                }],
            },
        };
        let root = root_loop(vec![child]);
        assert!(is_invalid_tree_drop(&root, child_id, child_id));
        assert!(is_invalid_tree_drop(&root, child_id, grandchild_id));
        assert!(!is_invalid_tree_drop(&root, grandchild_id, child_id));
    }
}
