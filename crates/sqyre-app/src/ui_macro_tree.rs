//! Macro action tree: TreeView, DnD/scroll gestures, row chrome wiring, highlights.

use crate::action_tooltip;
use crate::action_tooltip::help;
use crate::paint_ctx::{CatalogPaint, RecordBridges, TipUiCtx, TreePaint, VarTheme};
use crate::tree_chrome::{self, RowAction, RowHighlight, RowInteraction};
use crate::tree_dnd;
use crate::tree_history::{TreeHistory, TreeSnapshot};
use crate::SqyreApp;
use eframe::egui;
use egui_ltreeview::{Action as TreeAction, NodeBuilder, TreeView, TreeViewBuilder, TreeViewState};
use sqyre_domain::{collect_known_variable_names, Action, ActionId, InsertSlot};
use std::collections::HashSet;
use std::sync::atomic::Ordering;

/// Pointer gesture over the macro tree: reorder only from icon/pill handles;
/// dragging elsewhere scrolls the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TreeDragMode {
    #[default]
    Idle,
    Reorder,
    Scroll,
}

/// Match egui `ScrollArea` kinetic scrolling (points / second).
const TREE_SCROLL_STOP_SPEED: f32 = 20.0;
/// Match egui `ScrollArea` friction (points / second²).
const TREE_SCROLL_FRICTION: f32 = 1000.0;

pub fn show(app: &mut SqyreApp, ui: &mut egui::Ui, force_openness: Option<bool>) {
    let running = app.run.running.load(Ordering::SeqCst);
    let idx = app.selected_macro.min(app.macros.len() - 1);
    app.selected_macro = idx;

    let mut open_logs: Option<ActionId> = None;
    let mut delete_action: Option<ActionId> = None;
    let mut row_events: Vec<(ActionId, RowInteraction)> = Vec::new();
    let is_dark = ui.visuals().dark_mode;
    let root_aid = app.macros[idx].root.id;
    let macro_name = app.macros[idx].name.clone();
    let hl_snap = app.highlighter.snapshot();
    let id = ui.make_persistent_id(("macro_tree", idx));
    let mut state = TreeViewState::<ActionId>::load(ui, id).unwrap_or_default();
    if let Some(open) = force_openness {
        set_all_branches_openness(&app.macros[idx].root, &mut state, open);
    }
    sync_execution_expand(
        running,
        &app.macros[idx].root,
        &mut state,
        &mut app.exec_fully_expanded,
        &mut app.pre_exec_closed,
        &mut app.last_exec_follow,
    );
    let follow = highlight_follow_target(&hl_snap);
    let scroll_to = follow.filter(|id| app.last_exec_follow != Some(*id));
    let mut scrolled_follow = false;
    // Floating bars allocate 0 by default and cover the row chrome; reserve
    // bar_width so logs/delete stay clear when the scrollbar appears.
    let actions = ui
        .scope(|ui| {
            ui.spacing_mut().scroll.floating_allocated_width = ui.spacing().scroll.bar_width;
            let scroll_out = egui::ScrollArea::vertical()
                .id_salt("macro_tree_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // Decide reorder vs drag-scroll before TreeView so
                    // allow_drag_and_drop can suppress a non-handle drag.
                    let (primary_down, primary_released, pointer_delta, pointer_vel_y, dt) = ui
                        .input(|i| {
                            (
                                i.pointer.primary_down(),
                                i.pointer.primary_released(),
                                i.pointer.delta(),
                                i.pointer.velocity().y,
                                i.stable_dt.min(0.1),
                            )
                        });
                    if primary_released && app.tree_drag_mode == TreeDragMode::Scroll {
                        // Hand off to kinetic coast (same as egui ScrollArea).
                        app.tree_scroll_vel = pointer_vel_y;
                    }
                    if !primary_down {
                        app.tree_drag_mode = TreeDragMode::Idle;
                    } else if app.tree_drag_mode == TreeDragMode::Idle {
                        let become_drag = ui.input(|i| !i.pointer.could_any_button_be_click());
                        if become_drag {
                            app.tree_scroll_vel = 0.0;
                            // Only claim the gesture when press started on this
                            // scroll surface — edit tooltips / other Windows sit
                            // above and must keep drag-move and resize without
                            // scrolling the tree underneath. View tips use
                            // interactable(false) so layer_id_at still hits us.
                            let tree_layer = ui.layer_id();
                            let scroll_clip = ui.clip_rect();
                            let press_on_tree = ui
                                .ctx()
                                .input(|i| i.pointer.press_origin())
                                .is_some_and(|p| {
                                    scroll_clip.contains(p)
                                        && ui.ctx().layer_id_at(p) == Some(tree_layer)
                                });
                            if press_on_tree {
                                let on_handle = ui.input(|i| {
                                    i.pointer.press_origin().is_some_and(|p| {
                                        app.tree_drag_handles.iter().any(|r| r.contains(p))
                                    })
                                });
                                app.tree_drag_mode = if on_handle {
                                    TreeDragMode::Reorder
                                } else {
                                    TreeDragMode::Scroll
                                };
                            }
                        }
                    }
                    if app.tree_drag_mode == TreeDragMode::Scroll {
                        ui.scroll_with_delta_animation(
                            pointer_delta,
                            egui::style::ScrollAnimation::none(),
                        );
                    } else if app.tree_scroll_vel.abs() >= TREE_SCROLL_STOP_SPEED {
                        ui.scroll_with_delta_animation(
                            egui::vec2(0.0, app.tree_scroll_vel * dt),
                            egui::style::ScrollAnimation::none(),
                        );
                        let friction = TREE_SCROLL_FRICTION * dt;
                        if friction > app.tree_scroll_vel.abs() {
                            app.tree_scroll_vel = 0.0;
                        } else {
                            app.tree_scroll_vel -= friction * app.tree_scroll_vel.signum();
                            ui.ctx().request_repaint();
                        }
                    } else {
                        app.tree_scroll_vel = 0.0;
                    }
                    let allow_dnd = !running && app.tree_drag_mode != TreeDragMode::Scroll;

                    let catalog = &app.catalog;
                    let icons = &mut app.icon_cache;
                    let root = &app.macros[idx].root;
                    let root_children = root.children();
                    let known_vars = collect_known_variable_names(&app.macros[idx]);
                    let interact_y = ui.spacing().interact_size.y;
                    let mut tree_paint = TreePaint {
                        catalog,
                        icons,
                        theme: VarTheme {
                            known_vars: &known_vars,
                            is_dark,
                        },
                        macro_name: &macro_name,
                        hl_snap: &hl_snap,
                        selected: app.selected_action,
                    };
                    let (_, tree_actions) = TreeView::new(id)
                        .allow_drag_and_drop(allow_dnd)
                        .default_node_height(Some(tree_chrome::default_row_height(interact_y)))
                        .show_state(
                            ui,
                            &mut state,
                            |builder: &mut TreeViewBuilder<'_, ActionId>| {
                                // Invisible flattened root so top-level rows have a parent for DnD
                                // Root loop is not painted.
                                builder.node(
                                    NodeBuilder::dir(root_aid)
                                        .flatten(true)
                                        .drop_allowed(true)
                                        .default_open(true),
                                );
                                for child in root_children {
                                    build_tree(
                                        builder,
                                        child,
                                        &mut open_logs,
                                        &mut delete_action,
                                        &mut row_events,
                                        &mut tree_paint,
                                        scroll_to,
                                        &mut scrolled_follow,
                                        interact_y,
                                    );
                                }
                                builder.close_dir();
                            },
                        );
                    // Off-clip rows skip label_ui — estimate Y so ScrollArea can still follow.
                    if let Some(target) = scroll_to {
                        if !scrolled_follow {
                            if let Some(row_i) = flattened_visible_index(root, target) {
                                let row_h =
                                    tree_chrome::default_row_height(ui.spacing().interact_size.y)
                                        + ui.spacing().item_spacing.y;
                                let y = ui.min_rect().top() + row_i as f32 * row_h;
                                let rect = egui::Rect::from_min_size(
                                    egui::pos2(ui.min_rect().left(), y),
                                    egui::vec2(ui.available_width().max(1.0), row_h),
                                );
                                ui.scroll_to_rect(rect, Some(egui::Align::Center));
                                scrolled_follow = true;
                            }
                        }
                    }
                    tree_actions
                });
            // Match egui ScrollArea: kill kinetic coast when offset clamps.
            // Positive scroll_with_delta.y decreases offset (toward top).
            let max_offset_y =
                (scroll_out.content_size.y - scroll_out.inner_rect.height()).max(0.0);
            let y = scroll_out.state.offset.y;
            if (y <= 0.0 && app.tree_scroll_vel > 0.0)
                || (y >= max_offset_y && app.tree_scroll_vel < 0.0)
            {
                app.tree_scroll_vel = 0.0;
            }
            scroll_out.inner
        })
        .inner;
    if scrolled_follow {
        app.last_exec_follow = follow;
    }

    app.tree_drag_handles.clear();
    for (_, interaction) in &row_events {
        let r = interaction.drag_handle_rect;
        if r.width() > 0.0 && r.height() > 0.0 {
            app.tree_drag_handles.push(r);
        }
    }

    // Row overlay is clickthrough (Sense::hover + geometric clicks). When a
    // view tip covers the row, TreeView never sees the click — select here.
    for (aid, interaction) in &row_events {
        if interaction.primary_clicked {
            state.set_one_selected(*aid);
            app.selected_action = Some(*aid);
        }
    }
    state.store(ui, id);

    if let Some(aid) = open_logs {
        app.logs_window = Some(aid);
    }
    if let Some(aid) = delete_action {
        if !aid.is_root()
            && !matches!(
                app.macros[idx].root.resolve_tree_id(aid),
                Some(sqyre_domain::TreeNodeRef::ElseFolder { .. })
            )
        {
            app.record_tree_mutation();
            let cleared_sel = app.selected_action_id() == Some(aid);
            let _ = app.macros[idx].root.remove_by_id(aid);
            if cleared_sel {
                app.selected_action = None;
            }
            if app.logs_window == Some(aid) {
                app.logs_window = None;
                app.logs_image_cache.clear();
            }
            if app.tooltip.action_id() == Some(aid) {
                app.tooltip.cancel();
            }
            app.persist_macro_at(idx);
        }
    }

    let pointer = ui.ctx().pointer_interact_pos();
    let mut any_view_hover = false;
    // Prefer edit-open from any row; otherwise last hovered wins for view.
    for (aid, interaction) in &row_events {
        if interaction.hovered || interaction.pointer_in_row {
            any_view_hover = true;
        }
        if let Some(action) = app.macros[idx].root.find_by_id(*aid) {
            let action = action.clone();
            action_tooltip::ingest_row(&mut app.tooltip, &action, *interaction, pointer);
        }
    }
    action_tooltip::end_hover_pass(&mut app.tooltip, any_view_hover);

    {
        let selected = app.selected_action_id();
        let name = app.macros[idx].name.clone();
        let catalog = &app.catalog;
        let icons = &mut app.icon_cache;
        let previews = &mut app.preview_tooltips;
        let macros: Vec<(String, Vec<String>)> = app
            .macros
            .iter()
            .map(|m| (m.name.clone(), m.tags.clone()))
            .collect();
        // Snapshot before tooltip may mutate; record via borrow-split.
        let mut pending_record: Option<TreeSnapshot> = None;
        let known_vars = collect_known_variable_names(&app.macros[idx]);
        let discarded = {
            let macro_ = &mut app.macros[idx];
            let mut tip_ui = TipUiCtx {
                paint: CatalogPaint {
                    catalog,
                    icons,
                    previews,
                },
                theme: VarTheme {
                    known_vars: &known_vars,
                    is_dark,
                },
                bridges: RecordBridges {
                    key_record: &mut app.key_record,
                    hotkey_record: &mut app.hotkey_record,
                    macro_hotkeys: &app.macro_hotkeys,
                    screen_click: &app.screen_click,
                },
            };
            action_tooltip::show(
                &mut app.tooltip,
                ui.ctx(),
                macro_,
                &macros,
                &mut tip_ui,
                |root_before| {
                    if pending_record.is_none() {
                        if let Ok(snap) = TreeHistory::take_snapshot(root_before, selected) {
                            pending_record = Some(snap);
                        }
                    }
                },
            )
        };
        if let Some(snap) = pending_record {
            app.tree_histories
                .entry(name.clone())
                .or_default()
                .push_snapshot(snap);
            // Saved an edit (including first save of a provisional insert).
            app.persist_macro_at(idx);
        }
        if let Some(aid) = discarded {
            app.discard_provisional_action(aid);
        }
    }

    let mut pending_moves: Vec<(ActionId, ActionId, InsertSlot)> = Vec::new();
    for action in actions {
        match action {
            TreeAction::SetSelected(sel) => {
                app.selected_action = sel.into_iter().next();
            }
            TreeAction::Move(dnd) => {
                if running || app.tree_drag_mode == TreeDragMode::Scroll {
                    continue;
                }
                let target_aid = dnd.target;
                let Some(slot) = tree_dnd::insert_slot_from_dir_position(dnd.position) else {
                    continue;
                };
                for src_aid in &dnd.source {
                    pending_moves.push((*src_aid, target_aid, slot));
                }
            }
            TreeAction::Drag(dnd) => {
                if app.tree_drag_mode == TreeDragMode::Scroll {
                    dnd.remove_drop_marker(ui);
                    continue;
                }
                // Disallow dropping a node into itself / a descendant while dragging.
                let target_aid = dnd.target;
                if let Some(src_aid) = dnd.source.first() {
                    if tree_dnd::is_invalid_tree_drop(&app.macros[idx].root, *src_aid, target_aid) {
                        dnd.remove_drop_marker(ui);
                    }
                }
            }
            _ => {}
        }
    }
    if !pending_moves.is_empty() {
        app.record_tree_mutation();
    }
    let moved = !pending_moves.is_empty();
    for (src, parent, slot) in pending_moves {
        let _ = app.macros[idx].root.move_action(src, parent, slot);
    }
    if moved {
        app.persist_macro_at(idx);
    }

    if let Some(aid) = app.selected_action {
        let root = &app.macros[idx].root;
        if let Some(sqyre_domain::TreeNodeRef::ElseFolder { .. }) = root.resolve_tree_id(aid) {
            ui.separator();
            ui.label("Selected: Else (runs when not found / condition false)");
        } else {
            let action = if aid.is_root() {
                Some(root)
            } else {
                root.find_by_id(aid)
            };
            if let Some(action) = action {
                ui.separator();
                ui.label(format!(
                    "Selected: {} ({})",
                    action.display_name(),
                    action.type_key()
                ));
            }
        }
    }
}

fn set_all_branches_openness(root: &Action, state: &mut TreeViewState<ActionId>, open: bool) {
    root.walk(&mut |action| {
        if action.is_branch() && !action.id.is_root() {
            state.set_openness(action.id, open);
            if action.has_else_folder() {
                state.set_openness(ActionId::else_folder(action.id), open);
            }
        }
    });
}

/// Open every branch for the run.
fn sync_execution_expand(
    running: bool,
    root: &Action,
    state: &mut TreeViewState<ActionId>,
    exec_fully_expanded: &mut bool,
    pre_exec_closed: &mut HashSet<ActionId>,
    last_exec_follow: &mut Option<ActionId>,
) {
    if running && !*exec_fully_expanded {
        pre_exec_closed.clear();
        root.walk(&mut |action| {
            if action.is_branch() && !action.id.is_root() {
                // Match NodeBuilder::dir default_open(true) when unset.
                let open = state.is_open(&action.id).unwrap_or(true);
                if !open {
                    pre_exec_closed.insert(action.id);
                }
            }
        });
        set_all_branches_openness(root, state, true);
        *exec_fully_expanded = true;
        *last_exec_follow = None;
    } else if !running && *exec_fully_expanded {
        for id in pre_exec_closed.drain() {
            state.set_openness(id, false);
        }
        *exec_fully_expanded = false;
        *last_exec_follow = None;
    }
}

fn highlight_follow_target(snap: &sqyre_executor::HighlightSnapshot) -> Option<ActionId> {
    if let Some(id) = snap.cursor {
        return Some(id);
    }
    snap.fills
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(id, _)| *id)
}

/// 0-based index among painted tree rows with all branches treated as open (exec expand).
fn flattened_visible_index(root: &Action, target: ActionId) -> Option<usize> {
    let mut index = 0usize;
    let mut found = None;
    fn walk(action: &Action, target: ActionId, index: &mut usize, found: &mut Option<usize>) {
        if action.id.is_root() {
            for child in action.children() {
                walk(child, target, index, found);
            }
            return;
        }
        if action.id == target {
            *found = Some(*index);
        }
        *index += 1;
        if action.is_branch() {
            for child in action.children() {
                walk(child, target, index, found);
            }
            if action.has_else_folder() {
                let else_id = ActionId::else_folder(action.id);
                if else_id == target {
                    *found = Some(*index);
                }
                *index += 1;
                if let Some(else_kids) = action.else_children() {
                    for child in else_kids {
                        walk(child, target, index, found);
                    }
                }
            }
        }
    }
    walk(root, target, &mut index, &mut found);
    found
}

#[allow(clippy::too_many_arguments)]
fn build_tree(
    builder: &mut TreeViewBuilder<'_, ActionId>,
    action: &Action,
    open_logs: &mut Option<ActionId>,
    delete_action: &mut Option<ActionId>,
    row_events: &mut Vec<(ActionId, RowInteraction)>,
    tree: &mut TreePaint<'_>,
    scroll_to: Option<ActionId>,
    scrolled_follow: &mut bool,
    interact_y: f32,
) {
    let action_id = action.id;
    let highlight = row_highlight_for(tree, action_id);
    let should_scroll = scroll_to == Some(action_id);
    let row_h = tree_chrome::action_row_height(action, interact_y);

    let mut handle_row = |ui: &mut egui::Ui,
                          open_logs: &mut Option<ActionId>,
                          delete_action: &mut Option<ActionId>,
                          row_events: &mut Vec<(ActionId, RowInteraction)>,
                          scrolled_follow: &mut bool| {
        let interaction = tree_chrome::paint_action_row(
            ui,
            action,
            tree.catalog,
            tree.icons,
            tree.theme.known_vars,
            tree.theme.is_dark,
            highlight,
        );
        if should_scroll {
            ui.scroll_to_rect(interaction.row_rect, Some(egui::Align::Center));
            *scrolled_follow = true;
        }
        match interaction.action {
            RowAction::Logs => *open_logs = Some(action_id),
            RowAction::Delete => *delete_action = Some(action_id),
            RowAction::None => {}
        }
        row_events.push((action_id, interaction));
    };

    if action.is_branch() {
        let is_open = builder.node(
            NodeBuilder::dir(action_id)
                .drop_allowed(true)
                .height(row_h)
                .label_ui(|ui| {
                    handle_row(ui, open_logs, delete_action, row_events, scrolled_follow);
                }),
        );
        if is_open {
            for child in action.children() {
                build_tree(
                    builder,
                    child,
                    open_logs,
                    delete_action,
                    row_events,
                    tree,
                    scroll_to,
                    scrolled_follow,
                    interact_y,
                );
            }
            if action.has_else_folder() {
                build_else_dir(
                    builder,
                    action,
                    open_logs,
                    delete_action,
                    row_events,
                    tree,
                    scroll_to,
                    scrolled_follow,
                    interact_y,
                );
            }
        }
        builder.close_dir();
    } else {
        builder.node(NodeBuilder::leaf(action_id).height(row_h).label_ui(|ui| {
            handle_row(ui, open_logs, delete_action, row_events, scrolled_follow);
        }));
    }
}

#[allow(clippy::too_many_arguments)]
fn build_else_dir(
    builder: &mut TreeViewBuilder<'_, ActionId>,
    detection: &Action,
    open_logs: &mut Option<ActionId>,
    delete_action: &mut Option<ActionId>,
    row_events: &mut Vec<(ActionId, RowInteraction)>,
    tree: &mut TreePaint<'_>,
    scroll_to: Option<ActionId>,
    scrolled_follow: &mut bool,
    interact_y: f32,
) {
    let else_id = ActionId::else_folder(detection.id);
    let row_h = tree_chrome::default_row_height(interact_y);
    let should_scroll = scroll_to == Some(else_id);
    let is_open = builder.node(
        NodeBuilder::dir(else_id)
            .drop_allowed(true)
            .height(row_h)
            .label_ui(|ui| {
                let resp = help::tip(ui.strong("Else"), help::ELSE_BRANCH);
                let mut row_rect = resp.rect;
                row_rect.set_right(ui.max_rect().right());
                tree_chrome::paint_row_highlight(
                    ui,
                    row_rect,
                    else_folder_highlight(detection.id, tree.selected),
                );
                if should_scroll {
                    ui.scroll_to_rect(row_rect, Some(egui::Align::Center));
                    *scrolled_follow = true;
                }
                row_events.push((
                    else_id,
                    RowInteraction {
                        action: RowAction::None,
                        hovered: resp.hovered(),
                        pointer_in_row: resp.hovered(),
                        secondary_clicked: false,
                        double_clicked: false,
                        primary_clicked: resp.clicked(),
                        row_rect,
                        drag_handle_rect: row_rect,
                    },
                ));
            }),
    );
    if is_open {
        if let Some(else_kids) = detection.else_children() {
            for child in else_kids {
                build_tree(
                    builder,
                    child,
                    open_logs,
                    delete_action,
                    row_events,
                    tree,
                    scroll_to,
                    scrolled_follow,
                    interact_y,
                );
            }
        }
    }
    builder.close_dir();
}

pub(crate) fn row_highlight(
    macro_name: &str,
    action_id: ActionId,
    snap: &sqyre_executor::HighlightSnapshot,
) -> RowHighlight {
    if snap.macro_name != macro_name {
        return RowHighlight::None;
    }
    // Near-zero fill (e.g. image-search wait start) would paint an empty bar and hide the
    // cursor — prefer the simple overlay until progress is visible.
    if let Some(frac) = snap.fills.get(&action_id) {
        if *frac > 0.001 {
            return RowHighlight::Fill(*frac as f32);
        }
    }
    if snap.cursor == Some(action_id) {
        return RowHighlight::Cursor;
    }
    if snap.fills.contains_key(&action_id) {
        return RowHighlight::Cursor;
    }
    RowHighlight::None
}

fn row_highlight_for(tree: &TreePaint<'_>, action_id: ActionId) -> RowHighlight {
    let exec = row_highlight(tree.macro_name, action_id, tree.hl_snap);
    if !matches!(exec, RowHighlight::None) {
        return exec;
    }
    else_owner_highlight(action_id, tree.selected)
}

/// Soft highlight on a branch when its Else folder is the tree selection.
pub(crate) fn else_owner_highlight(
    action_id: ActionId,
    selected: Option<ActionId>,
) -> RowHighlight {
    if selected.is_some_and(|sel| ActionId::else_folder(action_id) == sel) {
        RowHighlight::Owner
    } else {
        RowHighlight::None
    }
}

/// Soft highlight on the Else folder when its owning branch is selected.
pub(crate) fn else_folder_highlight(
    owner_id: ActionId,
    selected: Option<ActionId>,
) -> RowHighlight {
    if selected == Some(owner_id) {
        RowHighlight::Owner
    } else {
        RowHighlight::None
    }
}

#[cfg(test)]
mod highlight_ui_tests {
    use super::*;
    use sqyre_executor::HighlightSnapshot;
    use std::collections::HashMap;

    #[test]
    fn row_highlight_prefers_cursor_over_zero_fill() {
        let id = ActionId::new();
        let mut fills = HashMap::new();
        fills.insert(id, 0.0);
        let snap = HighlightSnapshot {
            macro_name: "m".into(),
            cursor: Some(id),
            fills,
        };
        assert!(matches!(
            row_highlight("m", id, &snap),
            RowHighlight::Cursor
        ));
    }

    #[test]
    fn row_highlight_uses_fill_when_progressed() {
        let id = ActionId::new();
        let mut fills = HashMap::new();
        fills.insert(id, 0.4);
        let snap = HighlightSnapshot {
            macro_name: "m".into(),
            cursor: Some(id),
            fills,
        };
        assert!(matches!(
            row_highlight("m", id, &snap),
            RowHighlight::Fill(f) if (f - 0.4).abs() < f32::EPSILON
        ));
    }

    #[test]
    fn row_highlight_requires_matching_macro() {
        let id = ActionId::new();
        let snap = HighlightSnapshot {
            macro_name: "other".into(),
            cursor: Some(id),
            fills: HashMap::new(),
        };
        assert!(matches!(row_highlight("m", id, &snap), RowHighlight::None));
    }

    #[test]
    fn else_owner_highlight_when_else_selected() {
        let id = ActionId::new();
        assert_eq!(
            else_owner_highlight(id, Some(ActionId::else_folder(id))),
            RowHighlight::Owner
        );
        assert_eq!(
            else_owner_highlight(id, Some(ActionId::new())),
            RowHighlight::None
        );
        assert_eq!(else_owner_highlight(id, None), RowHighlight::None);
    }

    #[test]
    fn else_folder_highlight_when_owner_selected() {
        let id = ActionId::new();
        assert_eq!(else_folder_highlight(id, Some(id)), RowHighlight::Owner);
        assert_eq!(
            else_folder_highlight(id, Some(ActionId::else_folder(id))),
            RowHighlight::None
        );
        assert_eq!(else_folder_highlight(id, None), RowHighlight::None);
    }
}
