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

/// Catalog entry used when expanding Image Search tag targets.
#[derive(Debug, Clone)]
pub struct CatalogItemRef {
    /// `program~item` target string.
    pub target: String,
    pub tags: Vec<String>,
}

/// Parse a stored Image Search tag filter into `(include, tag)`.
///
/// Leading `+` / `-` set polarity; a bare tag is an include. Empty / whitespace-only
/// entries (and a lone `+` / `-`) are ignored.
pub fn parse_tag_filter(raw: &str) -> Option<(bool, String)> {
    let t = raw.trim();
    if t.is_empty() {
        return None;
    }
    let (include, rest) = if let Some(r) = t.strip_prefix('+') {
        (true, r.trim())
    } else if let Some(r) = t.strip_prefix('-') {
        (false, r.trim())
    } else {
        (true, t)
    };
    if rest.is_empty() {
        return None;
    }
    Some((include, rest.to_string()))
}

/// Wire / chip form for a tag filter (`+tag` or `-tag`).
pub fn format_tag_filter(include: bool, tag: &str) -> String {
    let tag = tag.trim();
    if include {
        format!("+{tag}")
    } else {
        format!("-{tag}")
    }
}

/// Bare tag name from a stored filter (`"+weapon"` → `"weapon"`).
pub fn tag_filter_name(raw: &str) -> Option<String> {
    parse_tag_filter(raw).map(|(_, tag)| tag)
}

/// True when the item's tags satisfy every include and none of the excludes.
///
/// Requires at least one include filter; exclude-only lists never match.
pub fn item_matches_tag_filters(item_tags: &[String], filters: &[String]) -> bool {
    let mut includes: Vec<String> = Vec::new();
    let mut excludes: Vec<String> = Vec::new();
    for f in filters {
        let Some((include, tag)) = parse_tag_filter(f) else {
            continue;
        };
        if include {
            if !includes.iter().any(|t| t == &tag) {
                includes.push(tag);
            }
        } else if !excludes.iter().any(|t| t == &tag) {
            excludes.push(tag);
        }
    }
    if includes.is_empty() {
        return false;
    }
    let has = |want: &str| item_tags.iter().any(|t| t.trim() == want);
    includes.iter().all(|t| has(t)) && excludes.iter().all(|t| !has(t))
}

/// Union explicit Image Search targets with catalog items matching `target_tags`.
///
/// Tag filters use exact string equality with `+` / `-` polarity:
/// an item must have **every** include tag and **none** of the exclude tags.
/// Explicit targets keep their stored order first; matching catalog items are
/// appended in enumeration order (deduped).
pub fn expand_image_search_targets(
    explicit: &[String],
    target_tags: &[String],
    catalog: &[CatalogItemRef],
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::<String>::new();

    for target in explicit {
        let t = target.trim();
        if t.is_empty() {
            continue;
        }
        if seen.insert(t.to_string()) {
            out.push(t.to_string());
        }
    }

    let has_include = target_tags.iter().any(|f| {
        parse_tag_filter(f)
            .map(|(include, _)| include)
            .unwrap_or(false)
    });
    if !has_include {
        return out;
    }

    for item in catalog {
        if !item_matches_tag_filters(&item.tags, target_tags) {
            continue;
        }
        let t = item.target.trim();
        if t.is_empty() {
            continue;
        }
        if seen.insert(t.to_string()) {
            out.push(t.to_string());
        }
    }
    out
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

    fn catalog(target: &str, tags: &[&str]) -> CatalogItemRef {
        CatalogItemRef {
            target: target.into(),
            tags: tags.iter().map(|t| (*t).to_string()).collect(),
        }
    }

    #[test]
    fn expand_tags_exact_across_programs() {
        let catalog = vec![
            catalog("Game~Sword", &["weapon", "melee"]),
            catalog("Game~Potion", &["heal"]),
            catalog("Other~Axe", &["weapon"]),
            catalog("Game~Shield", &["armor"]),
        ];
        assert_eq!(
            expand_image_search_targets(&[], &["+weapon".into()], &catalog),
            vec!["Game~Sword", "Other~Axe"]
        );
    }

    #[test]
    fn expand_tags_include_and_exclude() {
        let catalog = vec![
            catalog("P~a", &["construction material", "holy"]),
            catalog("P~b", &["construction material"]),
            catalog("P~c", &["construction material", "legendary"]),
            catalog("P~d", &["other"]),
        ];
        assert_eq!(
            expand_image_search_targets(
                &[],
                &[
                    "+construction material".into(),
                    "-holy".into(),
                    "-legendary".into()
                ],
                &catalog
            ),
            vec!["P~b"]
        );
    }

    #[test]
    fn expand_tags_requires_all_includes() {
        let catalog = vec![
            catalog("P~a", &["weapon", "melee"]),
            catalog("P~b", &["weapon"]),
        ];
        assert_eq!(
            expand_image_search_targets(&[], &["+weapon".into(), "+melee".into()], &catalog),
            vec!["P~a"]
        );
    }

    #[test]
    fn expand_tags_union_dedupes_explicit() {
        let catalog = vec![
            catalog("P~a", &["rare", "heal"]),
            catalog("P~b", &["heal"]),
            catalog("P~c", &["other"]),
        ];
        let explicit = vec!["P~b".into(), "P~extra".into()];
        // Single include: only items with heal (AND semantics for multiple includes).
        assert_eq!(
            expand_image_search_targets(&explicit, &["+heal".into()], &catalog),
            vec!["P~b", "P~extra", "P~a"]
        );
    }

    #[test]
    fn expand_tags_ignores_hierarchical_prefix() {
        let catalog = vec![catalog("P~a", &["combat/pve"]), catalog("P~b", &["combat"])];
        assert_eq!(
            expand_image_search_targets(&[], &["+combat".into()], &catalog),
            vec!["P~b"]
        );
    }

    #[test]
    fn expand_exclude_only_does_not_match_catalog() {
        let catalog = vec![catalog("P~a", &["holy"]), catalog("P~b", &["other"])];
        assert!(expand_image_search_targets(&[], &["-holy".into()], &catalog).is_empty());
    }

    #[test]
    fn bare_tag_filter_is_include() {
        assert_eq!(parse_tag_filter("weapon"), Some((true, "weapon".into())));
        assert_eq!(parse_tag_filter("+weapon"), Some((true, "weapon".into())));
        assert_eq!(parse_tag_filter("-holy"), Some((false, "holy".into())));
    }

    #[test]
    fn expand_then_sort_by_name() {
        let catalog = vec![catalog("P~z", &["t"]), catalog("P~a", &["t"])];
        let expanded = expand_image_search_targets(&["P~m".into()], &["+t".into()], &catalog);
        let infos: Vec<_> = expanded
            .iter()
            .map(|t| {
                let name = t.rsplit_once('~').map(|(_, n)| n).unwrap_or(t);
                info(t, name, 1, 1, &[])
            })
            .collect();
        assert_eq!(
            ordered_item_targets(&infos, ItemSortBy::Name, ItemSortThen::NameAsc, &[]),
            vec!["P~a", "P~m", "P~z"]
        );
    }
}
