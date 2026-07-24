//! Pure helpers for mapping tree drag-and-drop positions onto domain insert slots.

use egui_ltreeview::DirPosition;
use sqyre_domain::{Action, ActionId, InsertSlot};

/// Map an `egui_ltreeview` drop position to an [`InsertSlot`].
/// Tree node ids are stable [`ActionId`] values, so no separate lookup map is needed.
pub(crate) fn insert_slot_from_dir_position(position: DirPosition<ActionId>) -> Option<InsertSlot> {
    match position {
        DirPosition::First => Some(InsertSlot::First),
        DirPosition::Last => Some(InsertSlot::Last),
        DirPosition::Before(aid) => Some(InsertSlot::Before(aid)),
        DirPosition::After(aid) => Some(InsertSlot::After(aid)),
    }
}

/// True when dropping any of `srcs` onto `target` would nest a node into itself / a descendant.
pub(crate) fn is_invalid_tree_drop_any(root: &Action, srcs: &[ActionId], target: ActionId) -> bool {
    srcs.iter()
        .any(|&src| is_invalid_tree_drop(root, src, target))
}

/// True when dropping `src` onto `target` would nest a node into itself / a descendant.
pub(crate) fn is_invalid_tree_drop(root: &Action, src: ActionId, target: ActionId) -> bool {
    if src == target {
        return true;
    }
    let target_for_check = match root.resolve_tree_id(target) {
        Some(sqyre_domain::TreeNodeRef::ElseFolder { parent_id }) => parent_id,
        _ => target,
    };
    root.find_by_id(src)
        .is_some_and(|s| s.contains_id(target_for_check))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::{root_loop, ActionKind, ScalarValue};

    #[test]
    fn maps_dir_positions() {
        let a = ActionId::new();
        let b = ActionId::new();
        assert_eq!(
            insert_slot_from_dir_position(DirPosition::First),
            Some(InsertSlot::First)
        );
        assert_eq!(
            insert_slot_from_dir_position(DirPosition::Last),
            Some(InsertSlot::Last)
        );
        assert_eq!(
            insert_slot_from_dir_position(DirPosition::Before(b)),
            Some(InsertSlot::Before(b))
        );
        assert_eq!(
            insert_slot_from_dir_position(DirPosition::After(a)),
            Some(InsertSlot::After(a))
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
        assert!(is_invalid_tree_drop_any(
            &root,
            &[grandchild_id, child_id],
            grandchild_id
        ));
        assert!(!is_invalid_tree_drop_any(&root, &[grandchild_id], child_id));
    }
}
