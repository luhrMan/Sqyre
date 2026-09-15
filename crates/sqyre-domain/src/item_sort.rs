//! Item list ordering for Image Search (display + search).

use crate::{ItemSortBy, ItemSortThen, PROGRAM_DELIMITER};
use std::cmp::Ordering;

/// Catalog-facing fields used to order Image Search targets.
#[derive(Debug, Clone)]
pub struct ItemSortInfo {
    pub target: String,
    pub name: String,
    pub footprint: i32,
    pub tags: Vec<String>,
}

impl ItemSortInfo {
    /// Build sort info from a target ref and optional catalog fields.
    ///
    /// Missing meta uses the item key as the name and a 1×1 footprint.
    pub fn from_parts(
        target: impl Into<String>,
        name: impl Into<String>,
        rows: i32,
        cols: i32,
        tags: Vec<String>,
    ) -> Self {
        let target = target.into();
        let name = {
            let n = name.into();
            if n.is_empty() {
                item_key_from_target(&target).to_string()
            } else {
                n
            }
        };
        let footprint = rows.max(1).saturating_mul(cols.max(1));
        Self {
            target,
            name,
            footprint,
            tags,
        }
    }
}

fn item_key_from_target(target: &str) -> &str {
    target
        .split_once(PROGRAM_DELIMITER)
        .map(|(_, rest)| {
            rest.split_once(PROGRAM_DELIMITER)
                .map(|(base, _)| base)
                .unwrap_or(rest)
        })
        .unwrap_or(target)
}

fn name_key(name: &str) -> String {
    name.to_ascii_lowercase()
}

/// Earliest index in `tag_priority` that appears on the item, or [`usize::MAX`].
fn tag_rank(tags: &[String], tag_priority: &[String]) -> usize {
    let mut best = usize::MAX;
    for tag in tags {
        if let Some(i) = tag_priority.iter().position(|p| p == tag) {
            best = best.min(i);
        }
    }
    best
}

fn cmp_name(a: &ItemSortInfo, b: &ItemSortInfo, desc: bool) -> Ordering {
    let ord = name_key(&a.name).cmp(&name_key(&b.name));
    if desc {
        ord.reverse()
    } else {
        ord
    }
}

fn cmp_footprint(a: &ItemSortInfo, b: &ItemSortInfo, large_first: bool) -> Ordering {
    if large_first {
        b.footprint.cmp(&a.footprint)
    } else {
        a.footprint.cmp(&b.footprint)
    }
}

fn cmp_then(
    a: &ItemSortInfo,
    b: &ItemSortInfo,
    then: ItemSortThen,
    ia: usize,
    ib: usize,
) -> Ordering {
    match then {
        ItemSortThen::NameAsc => cmp_name(a, b, false).then_with(|| ia.cmp(&ib)),
        ItemSortThen::NameDesc => cmp_name(a, b, true).then_with(|| ia.cmp(&ib)),
        ItemSortThen::FootprintLarge => cmp_footprint(a, b, true)
            .then_with(|| cmp_name(a, b, false))
            .then_with(|| ia.cmp(&ib)),
        ItemSortThen::FootprintSmall => cmp_footprint(a, b, false)
            .then_with(|| cmp_name(a, b, false))
            .then_with(|| ia.cmp(&ib)),
        ItemSortThen::ListOrder => ia.cmp(&ib),
    }
}

/// Return targets ordered for search and UI display.
///
/// `items` must be in the stored `targets` list order (Manual / list-order ties).
pub fn ordered_item_targets(
    items: &[ItemSortInfo],
    sort_by: ItemSortBy,
    sort_then: ItemSortThen,
    tag_priority: &[String],
) -> Vec<String> {
    let mut indexed: Vec<(usize, &ItemSortInfo)> = items.iter().enumerate().collect();
    match sort_by {
        ItemSortBy::Manual => {}
        ItemSortBy::Name => {
            let name_desc = matches!(sort_then, ItemSortThen::NameDesc);
            indexed.sort_by(|(ia, a), (ib, b)| {
                let primary = cmp_name(a, b, name_desc);
                if primary != Ordering::Equal {
                    return primary.then_with(|| a.target.cmp(&b.target));
                }
                // Same name (or Then was Name*): apply Then only when it adds another key.
                match sort_then {
                    ItemSortThen::NameAsc | ItemSortThen::NameDesc => {
                        a.target.cmp(&b.target).then_with(|| ia.cmp(ib))
                    }
                    other => cmp_then(a, b, other, *ia, *ib).then_with(|| a.target.cmp(&b.target)),
                }
            });
        }
        ItemSortBy::Footprint => {
            let large_first = !matches!(sort_then, ItemSortThen::FootprintSmall);
            indexed.sort_by(|(ia, a), (ib, b)| {
                let primary = cmp_footprint(a, b, large_first);
                if primary != Ordering::Equal {
                    return primary;
                }
                match sort_then {
                    ItemSortThen::FootprintLarge | ItemSortThen::FootprintSmall => {
                        cmp_name(a, b, false)
                            .then_with(|| a.target.cmp(&b.target))
                            .then_with(|| ia.cmp(ib))
                    }
                    other => cmp_then(a, b, other, *ia, *ib).then_with(|| a.target.cmp(&b.target)),
                }
            });
        }
        ItemSortBy::Tags => {
            indexed.sort_by(|(ia, a), (ib, b)| {
                let primary = tag_rank(&a.tags, tag_priority).cmp(&tag_rank(&b.tags, tag_priority));
                if primary != Ordering::Equal {
                    return primary;
                }
                cmp_then(a, b, sort_then, *ia, *ib).then_with(|| a.target.cmp(&b.target))
            });
        }
    }
    indexed
        .into_iter()
        .map(|(_, item)| item.target.clone())
        .collect()
}

/// Simple single-key order for Data Editor / catalog item grids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CatalogItemSort {
    #[default]
    NameAsc,
    NameDesc,
    FootprintLarge,
    FootprintSmall,
    /// Tag priority list, then name A→Z within each rank (see `tag_priority`).
    Tags,
}

impl CatalogItemSort {
    pub const ALL: &'static [Self] = &[
        Self::NameAsc,
        Self::NameDesc,
        Self::FootprintLarge,
        Self::FootprintSmall,
        Self::Tags,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::NameAsc => "Name A→Z",
            Self::NameDesc => "Name Z→A",
            Self::FootprintLarge => "Larger footprint",
            Self::FootprintSmall => "Smaller footprint",
            Self::Tags => "Tags",
        }
    }
}

/// Order catalog items for Data Editor / picker grids.
///
/// `tag_priority` is used when `sort` is [`CatalogItemSort::Tags`]; ignored otherwise.
pub fn ordered_catalog_items(
    items: &[ItemSortInfo],
    sort: CatalogItemSort,
    tag_priority: &[String],
) -> Vec<String> {
    match sort {
        CatalogItemSort::NameAsc => {
            ordered_item_targets(items, ItemSortBy::Name, ItemSortThen::NameAsc, &[])
        }
        CatalogItemSort::NameDesc => {
            ordered_item_targets(items, ItemSortBy::Name, ItemSortThen::NameDesc, &[])
        }
        CatalogItemSort::FootprintLarge => ordered_item_targets(
            items,
            ItemSortBy::Footprint,
            ItemSortThen::FootprintLarge,
            &[],
        ),
        CatalogItemSort::FootprintSmall => ordered_item_targets(
            items,
            ItemSortBy::Footprint,
            ItemSortThen::FootprintSmall,
            &[],
        ),
        CatalogItemSort::Tags => {
            ordered_item_targets(items, ItemSortBy::Tags, ItemSortThen::NameAsc, tag_priority)
        }
    }
}

/// Move `from` → `to` in display order, write back to `targets`, and force Manual.
///
/// Returns `true` when `targets` / `sort_by` changed.
pub fn apply_display_reorder(
    targets: &mut Vec<String>,
    sort_by: &mut ItemSortBy,
    sort_then: ItemSortThen,
    tag_priority: &[String],
    infos: &[ItemSortInfo],
    from: usize,
    to: usize,
) -> bool {
    if from == to || infos.len() != targets.len() {
        return false;
    }
    let mut order = ordered_item_targets(infos, *sort_by, sort_then, tag_priority);
    if from >= order.len() || to >= order.len() {
        return false;
    }
    if from < to {
        order[from..=to].rotate_left(1);
    } else {
        order[to..=from].rotate_right(1);
    }
    let changed = *targets != order || *sort_by != ItemSortBy::Manual;
    *targets = order;
    *sort_by = ItemSortBy::Manual;
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(target: &str, name: &str, rows: i32, cols: i32, tags: &[&str]) -> ItemSortInfo {
        ItemSortInfo::from_parts(
            target,
            name,
            rows,
            cols,
            tags.iter().map(|t| (*t).to_string()).collect(),
        )
    }

    #[test]
    fn name_then_asc() {
        let items = vec![
            info("P~b", "bravo", 1, 1, &[]),
            info("P~a", "Alpha", 1, 1, &[]),
            info("P~c", "charlie", 1, 1, &[]),
        ];
        assert_eq!(
            ordered_item_targets(&items, ItemSortBy::Name, ItemSortThen::NameAsc, &[]),
            vec!["P~a", "P~b", "P~c"]
        );
    }

    #[test]
    fn tags_then_name_asc() {
        let items = vec![
            info("P~c", "c", 1, 1, &["other"]),
            info("P~z", "z", 1, 1, &["heal"]),
            info("P~a", "a", 1, 1, &["heal"]),
            info("P~b", "b", 1, 1, &["rare"]),
        ];
        let priority = vec!["rare".into(), "heal".into()];
        assert_eq!(
            ordered_item_targets(&items, ItemSortBy::Tags, ItemSortThen::NameAsc, &priority),
            vec!["P~b", "P~a", "P~z", "P~c"]
        );
    }

    #[test]
    fn footprint_then_name() {
        let items = vec![
            info("P~small", "a", 1, 1, &[]),
            info("P~big", "z", 2, 2, &[]),
            info("P~mid", "m", 1, 2, &[]),
        ];
        assert_eq!(
            ordered_item_targets(
                &items,
                ItemSortBy::Footprint,
                ItemSortThen::FootprintLarge,
                &[]
            ),
            vec!["P~big", "P~mid", "P~small"]
        );
    }

    #[test]
    fn catalog_tags_priority_then_name() {
        let items = vec![
            info("P~b", "b", 1, 1, &["zeta"]),
            info("P~a", "a", 1, 1, &["alpha"]),
            info("P~c", "c", 1, 1, &["alpha"]),
        ];
        let priority = vec!["alpha".into()];
        assert_eq!(
            ordered_catalog_items(&items, CatalogItemSort::Tags, &priority),
            vec!["P~a", "P~c", "P~b"]
        );
    }

    #[test]
    fn drag_switches_to_manual() {
        let infos = vec![
            info("P~b", "bravo", 1, 1, &[]),
            info("P~a", "alpha", 1, 1, &[]),
        ];
        let mut targets = vec!["P~b".into(), "P~a".into()];
        let mut sort_by = ItemSortBy::Name;
        assert!(apply_display_reorder(
            &mut targets,
            &mut sort_by,
            ItemSortThen::NameAsc,
            &[],
            &infos,
            0,
            1
        ));
        assert_eq!(sort_by, ItemSortBy::Manual);
        assert_eq!(targets, vec!["P~b", "P~a"]);
    }
}
