//! Stable per-row ids for repeatable list editors.
//!
//! `ui.push_id(i)` scopes row widgets by position: deleting a row shifts every
//! later row's id, so an open popover or focused field "migrates" onto the row
//! that slid into its place. Persist one id per slot instead, and drop the id
//! at the same index as the removed element via [`remove_row_id`].

use eframe::egui;

#[derive(Clone, Default)]
struct RowIdSet {
    next: u64,
    ids: Vec<u64>,
}

fn set_id(ui: &egui::Ui, salt: &str) -> egui::Id {
    ui.id().with(("stable_row_ids", salt))
}

/// One persistent [`egui::Id`] per row slot, sized to `len`.
///
/// Fresh ids are appended when the list grows; trailing extras are dropped when
/// it shrinks. Pair with [`remove_row_id`] so deletion keeps ids aligned with
/// the surviving elements.
pub fn stable_row_ids(ui: &egui::Ui, salt: &str, len: usize) -> Vec<egui::Id> {
    let key = set_id(ui, salt);
    ui.ctx().data_mut(|d| {
        let set = d.get_persisted_mut_or_insert_with::<RowIdSet>(key, RowIdSet::default);
        while set.ids.len() < len {
            set.ids.push(set.next);
            set.next += 1;
        }
        set.ids.truncate(len);
        set.ids.iter().map(|n| egui::Id::new(*n)).collect()
    })
}

/// Drop the id at `index` after removing that element from the data list.
pub fn remove_row_id(ui: &egui::Ui, salt: &str, index: usize) {
    let key = set_id(ui, salt);
    ui.ctx().data_mut(|d| {
        if let Some(mut set) = d.get_persisted::<RowIdSet>(key) {
            if index < set.ids.len() {
                set.ids.remove(index);
            }
            d.insert_persisted(key, set);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(ctx: &egui::Context, len: usize) -> Vec<egui::Id> {
        let mut out = Vec::new();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            out = stable_row_ids(ui, "rows", len);
        })
        .drop_without_applying_deltas();
        out
    }

    #[test]
    fn deletion_keeps_surviving_row_ids() {
        let ctx = egui::Context::default();
        let first = ids(&ctx, 3);
        ctx.run_ui(egui::RawInput::default(), |ui| {
            remove_row_id(ui, "rows", 0);
        })
        .drop_without_applying_deltas();
        let after = ids(&ctx, 2);
        assert_eq!(after, first[1..]);
    }

    #[test]
    fn growth_appends_fresh_ids() {
        let ctx = egui::Context::default();
        let first = ids(&ctx, 2);
        let after = ids(&ctx, 3);
        assert_eq!(after[..2], first[..]);
        assert_ne!(after[2], first[0]);
        assert_ne!(after[2], first[1]);
    }
}
