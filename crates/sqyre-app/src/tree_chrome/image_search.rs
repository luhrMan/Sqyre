//! Image-search thumbnails, overflow pill, and view-tip helpers for tree rows.

use super::{
    extend_drag_handle, paint_pill, rgba, PILL_MARGIN_X, TARGET_THUMB_GAP, TARGET_THUMB_MAX_H,
    TARGET_THUMB_MAX_W,
};
use crate::icon_cache::IconCache;
use crate::pickers::attach_item_icon_tooltip;
use eframe::egui::{self, Color32, Sense, Vec2};
use sqyre_domain::{Action, ActionKind};
use sqyre_persist::CatalogView;
use sqyre_ui_model::action_pastel_color;
use std::collections::HashMap;

/// How many thumbs fit in `available`, leaving room for a `+N` pill when not all fit.
fn image_search_visible_thumbs(
    total: usize,
    available: f32,
    thumb_w: f32,
    gap: f32,
    overflow_pill_w: f32,
) -> usize {
    if total == 0 || thumb_w <= 0.0 {
        return 0;
    }
    let all = thumb_w * total as f32 + gap * (total - 1) as f32;
    if all <= available {
        return total;
    }
    let n = ((available - overflow_pill_w).max(0.0) / (thumb_w + gap)).floor() as usize;
    n.min(total - 1)
}

fn overflow_pill_width(ui: &egui::Ui, hidden: usize) -> f32 {
    let font = egui::TextStyle::Small.resolve(ui.style());
    let galley = ui
        .painter()
        .layout_no_wrap(format!("+{hidden}"), font, Color32::WHITE);
    galley.size().x + PILL_MARGIN_X as f32 * 2.0
}

fn target_thumb_size(
    ui: &egui::Ui,
    catalog: CatalogView<'_>,
    icons: &mut IconCache,
    target: &str,
) -> Vec2 {
    crate::icon_cache::stable_icon_thumb_size(
        ui.ctx(),
        catalog,
        icons,
        target,
        TARGET_THUMB_MAX_W,
        TARGET_THUMB_MAX_H,
    )
}

fn paint_target_thumb(
    ui: &mut egui::Ui,
    catalog: CatalogView<'_>,
    icons: &mut IconCache,
    target: &str,
) -> egui::Response {
    let size = target_thumb_size(ui, catalog, icons, target);
    let (slot_rect, slot_resp) = ui.allocate_exact_size(size, Sense::hover());
    if let Some(tex) = icons.for_target(ui.ctx(), catalog, target) {
        crate::icon_cache::paint_icon_thumb_at(
            ui,
            &tex,
            slot_rect.center(),
            size.x,
            size.y,
            3.0,
            Some(crate::theme::frame_fill()),
        );
    } else {
        let chrome = ui.visuals().widgets.noninteractive;
        ui.painter().rect(
            slot_rect,
            3.0,
            chrome.weak_bg_fill,
            chrome.bg_stroke,
            egui::StrokeKind::Outside,
        );
    }
    attach_item_icon_tooltip(&slot_resp, catalog, icons, target);
    slot_resp
}

pub(super) fn paint_image_search_extras(
    ui: &mut egui::Ui,
    action: &Action,
    catalog: CatalogView<'_>,
    icons: &mut IconCache,
    is_dark: bool,
    drag_handle: &mut egui::Rect,
) -> bool {
    let ActionKind::ImageSearch(d) = &action.kind else {
        return false;
    };
    let targets = cached_sorted_image_search_targets(
        ui.ctx(),
        catalog,
        &d.targets,
        &d.target_tags,
        d.sort_by,
        d.sort_then,
        &d.tag_priority,
    );
    let mut tip_hovered = false;
    let pastel = rgba(action_pastel_color(action.type_key(), is_dark));
    let prev_gap = ui.spacing().item_spacing.x;
    ui.spacing_mut().item_spacing.x = TARGET_THUMB_GAP;
    let count_pill = paint_pill(
        ui,
        &format!(
            "{} {}",
            egui_phosphor::regular::MAGNIFYING_GLASS,
            targets.len()
        ),
        pastel,
        false,
    );
    extend_drag_handle(drag_handle, count_pill.rect);
    if count_pill.hovered() {
        tip_hovered = true;
    }
    let thumb_w = targets
        .first()
        .map(|target| target_thumb_size(ui, catalog, icons, target).x)
        .unwrap_or(TARGET_THUMB_MAX_H);
    // `available_width` can include space past the pane's clip (row max_rect
    // wider than the viewport): count thumbs against the visible width so the
    // "+N" overflow pill lands inside the clip instead of clipping itself.
    let visible_w = ui
        .available_width()
        .min(ui.clip_rect().right() - ui.cursor().left())
        .max(0.0);
    let visible = image_search_visible_thumbs(
        targets.len(),
        visible_w,
        thumb_w,
        TARGET_THUMB_GAP,
        overflow_pill_width(ui, targets.len()),
    );
    for target in targets.iter().take(visible) {
        let thumb = paint_target_thumb(ui, catalog, icons, target);
        extend_drag_handle(drag_handle, thumb.rect);
        if thumb.hovered() {
            tip_hovered = true;
        }
    }
    let hidden = targets.len().saturating_sub(visible);
    // When even the overflow pill can't fit the remaining visible width, skip
    // it — painting it would clip against the pane edge.
    if hidden > 0
        && overflow_pill_width(ui, hidden) <= (ui.clip_rect().right() - ui.cursor().left()).max(0.0)
    {
        let overflow = paint_pill(ui, &format!("+{hidden}"), pastel, false);
        extend_drag_handle(drag_handle, overflow.rect);
        if overflow.hovered() {
            tip_hovered = true;
        }
    }
    ui.spacing_mut().item_spacing.x = prev_gap;
    tip_hovered
}

pub(crate) fn paint_image_search_tooltip_thumbs_pub(
    ui: &mut egui::Ui,
    action: &Action,
    catalog: CatalogView<'_>,
    icons: &mut IconCache,
) {
    let ActionKind::ImageSearch(d) = &action.kind else {
        return;
    };
    if d.targets.is_empty() && d.target_tags.is_empty() {
        return;
    }

    // Same expansion + sort as the edit tip Items grid / search order.
    let display = cached_sorted_image_search_targets(
        ui.ctx(),
        catalog,
        &d.targets,
        &d.target_tags,
        d.sort_by,
        d.sort_then,
        &d.tag_priority,
    );
    ui.add(
        egui::Label::new(format!(
            "{} {}",
            egui_phosphor::regular::MAGNIFYING_GLASS,
            display.len()
        ))
        .wrap(),
    );

    if !d.target_tags.is_empty() {
        crate::widgets::title_with_count(
            ui,
            egui::RichText::new("Tags").small().strong(),
            d.target_tags.len(),
        );
        let groups = image_search_tip_tag_groups(
            catalog,
            &d.target_tags,
            d.sort_by,
            d.sort_then,
            &d.tag_priority,
        );
        if groups.is_empty() {
            ui.label(egui::RichText::new(sqyre_domain::EMPTY_NONE).small().weak());
        } else {
            let sizing_count = if display.is_empty() {
                groups.iter().map(|g| g.members.len()).max().unwrap_or(0)
            } else {
                display.len()
            };
            for group in &groups {
                let fill = if group.include {
                    crate::theme::tag_include_fill()
                } else {
                    crate::theme::tag_exclude_fill()
                };
                egui::Frame::NONE
                    .fill(fill)
                    .corner_radius(egui::CornerRadius::same(4))
                    .inner_margin(egui::Margin::same(crate::theme::SPACE_4 as i8))
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(egui::RichText::new(group.polarity_label()).small().strong());
                            ui.label(egui::RichText::new(group.name.as_str()).small());
                        });
                        if group.members.is_empty() {
                            ui.label(egui::RichText::new(sqyre_domain::EMPTY_NONE).small().weak());
                        } else {
                            ui.push_id(("is_tip_tag", group.include, group.name.as_str()), |ui| {
                                crate::pickers::paint_even_icon_grid(
                                    ui,
                                    catalog,
                                    icons,
                                    &group.members,
                                    |_| false,
                                    crate::pickers::IconGridKind::TargetsSizedAs {
                                        count: sizing_count,
                                    },
                                    crate::pickers::IconGridOps::default(),
                                    None,
                                );
                            });
                        }
                    });
            }
        }
        if display.is_empty() {
            return;
        }
    }

    crate::widgets::title_with_count(
        ui,
        egui::RichText::new("Items").small().strong(),
        display.len(),
    );
    if display.is_empty() {
        ui.label(egui::RichText::new(sqyre_domain::EMPTY_NONE).small().weak());
        return;
    }
    crate::pickers::paint_even_icon_grid(
        ui,
        catalog,
        icons,
        &display,
        |_| false,
        crate::pickers::IconGridKind::Targets { removable: false },
        crate::pickers::IconGridOps::default(),
        None,
    );
}

/// One Image Search tag filter for the view tip: polarity, display name, members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImageSearchTipTagGroup {
    pub include: bool,
    pub name: String,
    pub members: Vec<String>,
}

impl ImageSearchTipTagGroup {
    /// User-facing polarity glyph (`+` include / `−` exclude). Never a wire key.
    pub fn polarity_label(&self) -> &'static str {
        if self.include {
            "+"
        } else {
            "−"
        }
    }
}

/// Resolve searched tag filters → display names and catalog member targets.
///
/// Include tags expand to items that match the **full** filter set (every `+`
/// and none of the `−`). Exclude tags list every item carrying that tag.
/// Member order matches the action's Search sorting (`sort_by` / `sort_then` /
/// `tag_priority`) so the view tip agrees with the edit tip.
pub(crate) fn image_search_tip_tag_groups(
    catalog: CatalogView<'_>,
    target_tags: &[String],
    sort_by: sqyre_domain::ItemSortBy,
    sort_then: sqyre_domain::ItemSortThen,
    tag_priority: &[String],
) -> Vec<ImageSearchTipTagGroup> {
    use sqyre_domain::{item_matches_tag_filters, parse_tag_filter, ItemSortInfo};

    let catalog_refs = catalog_item_refs(catalog);

    let mut groups = Vec::new();
    for filter in target_tags {
        let Some((include, name)) = parse_tag_filter(filter) else {
            continue;
        };
        let infos: Vec<ItemSortInfo> = catalog_refs
            .iter()
            .filter(|item| {
                item.tags.iter().any(|t| t.trim() == name)
                    && (!include || item_matches_tag_filters(&item.tags, target_tags))
            })
            .map(|item| {
                let (display, tags) = crate::pickers::item_tooltip_parts(catalog, &item.target);
                let (rows, cols) = catalog
                    .item_meta(&item.target)
                    .map(|m| (m.rows, m.cols))
                    .unwrap_or((1, 1));
                ItemSortInfo::from_parts(item.target.clone(), display, rows, cols, tags)
            })
            .collect();
        groups.push(ImageSearchTipTagGroup {
            include,
            name,
            members: sqyre_domain::ordered_item_targets(&infos, sort_by, sort_then, tag_priority),
        });
    }
    groups
}

/// `sorted_image_search_targets` memoized on `(inputs, catalog generation)`.
///
/// Tree rows repaint every frame and `sorted_image_search_targets` expands tag
/// filters against the whole catalog each call — a hotspot for jank on large
/// catalogs. Identical inputs share one entry; catalog mutations invalidate it.
pub(crate) fn cached_sorted_image_search_targets(
    ctx: &egui::Context,
    catalog: CatalogView<'_>,
    targets: &[String],
    target_tags: &[String],
    sort_by: sqyre_domain::ItemSortBy,
    sort_then: sqyre_domain::ItemSortThen,
    tag_priority: &[String],
) -> Vec<String> {
    let sig = egui::Id::new((
        "sorted_image_search_targets",
        targets,
        target_tags,
        tag_priority,
        sort_by.as_str(),
        sort_then.as_str(),
    ));
    let gen = catalog.generation();
    let cache_id = egui::Id::new("sorted_image_search_targets_cache");
    ctx.data_mut(|d| {
        let map = d.get_persisted_mut_or_insert_with::<HashMap<egui::Id, (u64, Vec<String>)>>(
            cache_id,
            HashMap::new,
        );
        map.retain(|_, (g, _)| *g == gen);
        if let Some((_, cached)) = map.get(&sig) {
            return cached.clone();
        }
        let sorted = sorted_image_search_targets(
            catalog,
            targets,
            target_tags,
            sort_by,
            sort_then,
            tag_priority,
        );
        map.insert(sig, (gen, sorted.clone()));
        sorted
    })
}

/// Display / search order for Image Search targets (explicit + tag expansion).
pub(crate) fn sorted_image_search_targets(
    catalog: CatalogView<'_>,
    targets: &[String],
    target_tags: &[String],
    sort_by: sqyre_domain::ItemSortBy,
    sort_then: sqyre_domain::ItemSortThen,
    tag_priority: &[String],
) -> Vec<String> {
    use sqyre_domain::expand_image_search_targets;
    let catalog_refs = catalog_item_refs(catalog);
    let expanded = expand_image_search_targets(targets, target_tags, &catalog_refs);
    let infos: Vec<_> = expanded
        .iter()
        .map(|target| {
            let (name, tags) = crate::pickers::item_tooltip_parts(catalog, target);
            let (rows, cols) = catalog
                .item_meta(target)
                .map(|m| (m.rows, m.cols))
                .unwrap_or((1, 1));
            sqyre_domain::ItemSortInfo::from_parts(target.clone(), name, rows, cols, tags)
        })
        .collect();
    sqyre_domain::ordered_item_targets(&infos, sort_by, sort_then, tag_priority)
}

fn catalog_item_refs(catalog: CatalogView<'_>) -> Vec<sqyre_domain::CatalogItemRef> {
    use sqyre_domain::{CatalogItemRef, PROGRAM_DELIMITER};
    catalog
        .program_names()
        .filter_map(|prog| catalog.get(prog).map(|pdata| (prog.clone(), pdata)))
        .flat_map(|(prog, pdata)| {
            pdata
                .items
                .iter()
                .map(move |(name, item)| CatalogItemRef {
                    target: format!("{prog}{PROGRAM_DELIMITER}{name}"),
                    tags: item.tags.clone(),
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_search_tip_tag_groups_lists_names_and_members() {
        use sqyre_domain::{ItemSortBy, ItemSortThen};
        use sqyre_persist::{CatalogView, ProgramCatalog, ProgramData, ProgramItem};
        use std::collections::BTreeMap;

        let mut cat = ProgramCatalog::default();
        cat.programs_mut().insert(
            "Game".into(),
            ProgramData {
                name: "Game".into(),
                items: BTreeMap::from([
                    (
                        "Flask".into(),
                        ProgramItem {
                            name: "Health Flask".into(),
                            tags: vec!["healing".into(), "consumable".into()],
                            ..Default::default()
                        },
                    ),
                    (
                        "Mana".into(),
                        ProgramItem {
                            name: "Mana Flask".into(),
                            tags: vec!["mana".into(), "consumable".into()],
                            ..Default::default()
                        },
                    ),
                    (
                        "Junk".into(),
                        ProgramItem {
                            name: "Scrap".into(),
                            tags: vec!["consumable".into(), "junk".into()],
                            ..Default::default()
                        },
                    ),
                ]),
                ..Default::default()
            },
        );

        let groups = image_search_tip_tag_groups(
            CatalogView::from(&cat),
            &["+consumable".into(), "-junk".into()],
            ItemSortBy::Name,
            ItemSortThen::NameAsc,
            &[],
        );
        assert_eq!(groups.len(), 2);
        assert!(groups[0].include);
        assert_eq!(groups[0].name, "consumable");
        assert_eq!(
            groups[0].members,
            vec!["Game~Flask".to_string(), "Game~Mana".to_string()]
        );
        assert!(!groups[1].include);
        assert_eq!(groups[1].name, "junk");
        assert_eq!(groups[1].members, vec!["Game~Junk".to_string()]);
        assert_eq!(groups[0].polarity_label(), "+");
        assert_eq!(groups[1].polarity_label(), "−");
    }

    /// View tip Items / tag grids must use the same Search sorting as the edit tip.
    #[test]
    fn image_search_view_tip_sort_matches_edit_order() {
        use sqyre_domain::{ItemSortBy, ItemSortThen};
        use sqyre_persist::{CatalogView, ProgramCatalog, ProgramData, ProgramItem};
        use std::collections::BTreeMap;

        let mut cat = ProgramCatalog::default();
        cat.programs_mut().insert(
            "Demo".into(),
            ProgramData {
                name: "Demo".into(),
                items: BTreeMap::from([
                    (
                        "Zebra".into(),
                        ProgramItem {
                            name: "Zebra".into(),
                            tags: vec!["animal".into()],
                            grid_cols: 1,
                            grid_rows: 1,
                            ..Default::default()
                        },
                    ),
                    (
                        "Ant".into(),
                        ProgramItem {
                            name: "Ant".into(),
                            tags: vec!["animal".into()],
                            grid_cols: 2,
                            grid_rows: 2,
                            ..Default::default()
                        },
                    ),
                    (
                        "Moose".into(),
                        ProgramItem {
                            name: "Moose".into(),
                            tags: vec!["animal".into()],
                            grid_cols: 1,
                            grid_rows: 2,
                            ..Default::default()
                        },
                    ),
                ]),
                ..Default::default()
            },
        );

        let targets = vec!["Demo~Zebra".into(), "Demo~Ant".into(), "Demo~Moose".into()];
        let tags = vec!["+animal".into()];
        let sort_by = ItemSortBy::Name;
        let sort_then = ItemSortThen::NameDesc;

        let edit_order = sorted_image_search_targets(
            CatalogView::from(&cat),
            &targets,
            &tags,
            sort_by,
            sort_then,
            &[],
        );
        assert_eq!(
            edit_order,
            vec![
                "Demo~Zebra".to_string(),
                "Demo~Moose".to_string(),
                "Demo~Ant".to_string()
            ]
        );

        // Old view tip Items passed `&[]` for tags: tags-only actions showed an
        // empty Items grid. Expansion must match edit/search order.
        assert!(sorted_image_search_targets(
            CatalogView::from(&cat),
            &[],
            &[],
            sort_by,
            sort_then,
            &[]
        )
        .is_empty());
        assert_eq!(
            sorted_image_search_targets(
                CatalogView::from(&cat),
                &[],
                &tags,
                sort_by,
                sort_then,
                &[]
            ),
            edit_order
        );

        // Old tag groups always sorted A→Z; they must follow Search sorting.
        let groups =
            image_search_tip_tag_groups(CatalogView::from(&cat), &tags, sort_by, sort_then, &[]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].members, edit_order);
        let alpha_groups = image_search_tip_tag_groups(
            CatalogView::from(&cat),
            &tags,
            ItemSortBy::Name,
            ItemSortThen::NameAsc,
            &[],
        );
        assert_ne!(alpha_groups[0].members, edit_order);

        let footprint = sorted_image_search_targets(
            CatalogView::from(&cat),
            &targets,
            &tags,
            ItemSortBy::Footprint,
            ItemSortThen::FootprintLarge,
            &[],
        );
        assert_eq!(
            footprint,
            vec![
                "Demo~Ant".to_string(),
                "Demo~Moose".to_string(),
                "Demo~Zebra".to_string()
            ]
        );
        let fp_groups = image_search_tip_tag_groups(
            CatalogView::from(&cat),
            &tags,
            ItemSortBy::Footprint,
            ItemSortThen::FootprintLarge,
            &[],
        );
        assert_eq!(fp_groups[0].members, footprint);
    }

    #[test]
    fn visible_thumbs_fill_available_width() {
        const THUMB: f32 = 18.0;
        const GAP: f32 = 2.0;
        const PILL: f32 = 30.0;
        assert_eq!(image_search_visible_thumbs(3, 200.0, THUMB, GAP, PILL), 3);
        assert_eq!(image_search_visible_thumbs(2, 38.0, THUMB, GAP, PILL), 2);
        assert_eq!(image_search_visible_thumbs(8, 100.0, THUMB, GAP, PILL), 3);
        assert_eq!(image_search_visible_thumbs(0, 100.0, THUMB, GAP, PILL), 0);
        assert_eq!(image_search_visible_thumbs(10, 10.0, THUMB, GAP, PILL), 0);
        // Old 8-wide 30px slots wasted space; 400px should show far more than 8.
        assert!(image_search_visible_thumbs(30, 400.0, THUMB, GAP, PILL) > 8);
    }
}
