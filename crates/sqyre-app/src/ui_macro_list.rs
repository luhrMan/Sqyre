//! Left macro list panel and delete confirmation.

use crate::pickers;
use crate::status_banner::{StatusBanner, PREFIX_LOAD_ERROR, PREFIX_SAVE_ERROR};
use crate::theme::{SPACE_12, SPACE_4, SPACE_8};
use crate::widgets::tags::{filters_cover_path, normalize_tag_path};
use crate::SqyreApp;
use eframe::egui;
use sqyre_domain::Macro;
use sqyre_hotkeys::format_hotkey;
use std::collections::BTreeMap;

/// Empty-string group key for macros with no tags.
const UNTAGGED_KEY: &str = "";

/// One node in the `/`-nested tag tree for the macrolist.
#[derive(Debug, Default)]
struct TagTreeNode {
    /// Macro indices with this exact normalized path (leaf placement only).
    macros: Vec<usize>,
    /// Child segments → nodes (BTreeMap keeps alphabetical order).
    children: BTreeMap<String, TagTreeNode>,
}

impl TagTreeNode {
    fn subtree_macro_count(&self) -> usize {
        self.macros.len()
            + self
                .children
                .values()
                .map(TagTreeNode::subtree_macro_count)
                .sum::<usize>()
    }

    fn insert_macro(&mut self, segments: &[&str], macro_idx: usize) {
        let Some((head, rest)) = segments.split_first() else {
            self.macros.push(macro_idx);
            return;
        };
        self.children
            .entry((*head).to_string())
            .or_default()
            .insert_macro(rest, macro_idx);
    }
}

/// Elide `text` to a single line that fits `max_width`, appending `…` only when needed.
fn elide_to_width(ui: &egui::Ui, text: &str, max_width: f32, font_id: egui::FontId) -> String {
    if text.is_empty() {
        return String::new();
    }
    let full = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font_id.clone(), egui::Color32::WHITE);
    if full.size().x <= max_width {
        return text.to_owned();
    }

    const ELLIPSIS: char = '…';
    let ellipsis_w = ui
        .painter()
        .layout_no_wrap(ELLIPSIS.to_string(), font_id.clone(), egui::Color32::WHITE)
        .size()
        .x;
    let budget = (max_width - ellipsis_w).max(0.0);
    if budget <= 0.0 {
        return ELLIPSIS.to_string();
    }

    let char_count = text.chars().count();
    let mut lo = 0usize;
    let mut hi = char_count;
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let candidate: String = text.chars().take(mid).collect();
        let w = ui
            .painter()
            .layout_no_wrap(candidate, font_id.clone(), egui::Color32::WHITE)
            .size()
            .x;
        if w <= budget {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    let mut out: String = text.chars().take(lo).collect();
    out.push(ELLIPSIS);
    out
}

/// Macro name on the first line; hotkey as a weak small hint below when set.
/// Each line is shown in full when it fits `max_text_width`, otherwise elided with `…`.
/// When `validation_error` is set, the name is tinted and callers should attach hover text.
fn macro_list_item_text(
    ui: &egui::Ui,
    m: &Macro,
    max_text_width: f32,
    validation_error: Option<&str>,
) -> egui::WidgetText {
    let style = ui.style();
    let name_font = egui::FontSelection::Default.resolve(style);
    let name = elide_to_width(ui, &m.name, max_text_width, name_font);
    let name_color = if validation_error.is_some() {
        crate::theme::error_fg()
    } else {
        style.visuals.text_color()
    };

    if m.hotkey.is_empty() {
        return egui::RichText::new(name).color(name_color).into();
    }

    let hotkey_font = egui::TextStyle::Small.resolve(style);
    let hotkey = elide_to_width(ui, &format_hotkey(&m.hotkey), max_text_width, hotkey_font);

    let mut job = egui::text::LayoutJob::default();
    egui::RichText::new(name).color(name_color).append_to(
        &mut job,
        style,
        egui::FontSelection::Default,
        egui::Align::LEFT,
    );
    egui::RichText::new(format!("\n{hotkey}"))
        .small()
        .color(style.visuals.weak_text_color())
        .append_to(
            &mut job,
            style,
            egui::FontSelection::Default,
            egui::Align::LEFT,
        );
    job.into()
}

fn tag_header_label(tag: &str) -> &str {
    if tag.is_empty() {
        "Untagged"
    } else {
        tag
    }
}

/// Build a nested tag tree from filtered macros. Untagged macros are a separate root entry.
fn build_tag_tree(macros: &[Macro], filter: &str) -> (TagTreeNode, Vec<usize>) {
    let mut root = TagTreeNode::default();
    let mut untagged = Vec::new();
    for (i, m) in macros.iter().enumerate() {
        if !pickers::query_matches_name_or_tags(filter, &m.name, &m.tags) {
            continue;
        }
        if m.tags.is_empty() {
            untagged.push(i);
            continue;
        }
        for tag in &m.tags {
            let path = normalize_tag_path(tag);
            if path.is_empty() {
                continue;
            }
            let segments: Vec<&str> = path.split('/').collect();
            root.insert_macro(&segments, i);
        }
    }
    (root, untagged)
}

fn paint_macro_rows(
    ui: &mut egui::Ui,
    app: &SqyreApp,
    list_w: f32,
    indices: &[usize],
    clicked_macro: &mut Option<usize>,
) {
    for &i in indices {
        let Some(m) = app.workspace.macros.get(i) else {
            continue;
        };
        let width = ui.available_width().min(list_w).max(0.0);
        let text_width = (width - ui.spacing().button_padding.x * 2.0).max(0.0);
        let validation_err = sqyre_validate::validate_macro(m)
            .err()
            .map(|e| e.to_string());
        let label = macro_list_item_text(ui, m, text_width, validation_err.as_deref());
        let mut resp = ui.add(
            egui::Button::selectable(app.workspace.selected_macro == i, label)
                .wrap_mode(egui::TextWrapMode::Extend)
                .min_size(egui::vec2(width, 0.0)),
        );
        if let Some(err) = validation_err.as_deref() {
            resp = resp.on_hover_text(format!("Validation: {err}"));
        }
        if resp.clicked() {
            *clicked_macro = Some(i);
        }
    }
}

struct PaintTagCtx<'a> {
    app: &'a SqyreApp,
    list_w: f32,
    clicked_macro: &'a mut Option<usize>,
    clicked_tag: &'a mut Option<String>,
}

fn paint_tag_node(
    ui: &mut egui::Ui,
    ctx: &mut PaintTagCtx<'_>,
    path: &str,
    label: &str,
    node: &TagTreeNode,
    is_first_root: bool,
) {
    if !is_first_root {
        ui.add_space(SPACE_8);
        ui.separator();
        ui.add_space(SPACE_4);
    }

    let id = ui.make_persistent_id(("macro_list_tag", path));
    let count = node.subtree_macro_count();
    let filters = ctx.app.workspace.hotkey_tag_filters.as_slice();
    let exact_selected = filters.iter().any(|t| t == path);
    let covered = filters_cover_path(filters, path);
    let parent_covered = covered && !exact_selected;
    // body_unindented: show_body_indented calls expand_to_include_x which widens the panel.
    // Chevron expands/collapses only; Hotkeys is a key icon toggle (outline/fill).
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
        .show_header(ui, |ui| {
            let hover = if exact_selected {
                "Hotkeys enabled for this tag (multiselect). Click again to remove."
            } else if parent_covered {
                "Hotkeys covered by a parent tag. Deselect the parent to pick this tag alone."
            } else {
                "Include this tag in the hotkey filter (multiselect). Parents include nested tags."
            };
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut hotkeys_on = covered;
                let hotkey_resp = ui
                    .add_enabled_ui(!parent_covered, |ui| {
                        crate::widgets::icon_toggle(
                            ui,
                            &mut hotkeys_on,
                            "Hotkeys",
                            egui_phosphor::regular::KEY,
                            egui_phosphor::fill::KEY,
                        )
                    })
                    .inner
                    .on_hover_text(hover)
                    .on_disabled_hover_text(hover);
                if hotkey_resp.changed() {
                    *ctx.clicked_tag = Some(path.to_string());
                }

                // Remaining width: non-selectable title + count (expand via chevron only).
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.set_max_width(ui.available_width());
                    let font = egui::FontSelection::Default.resolve(ui.style());
                    let count_text = format!("({count})");
                    let count_w = ui
                        .painter()
                        .layout_no_wrap(count_text.clone(), font.clone(), egui::Color32::WHITE)
                        .size()
                        .x;
                    let name_budget =
                        (ui.available_width() - count_w - ui.spacing().item_spacing.x).max(0.0);
                    let header_text = elide_to_width(ui, label, name_budget, font);
                    ui.add(
                        egui::Label::new(egui::RichText::new(header_text).strong())
                            .selectable(false),
                    );
                    ui.label(egui::RichText::new(count_text).weak());
                });
            });
        })
        .body_unindented(|ui| {
            ui.set_max_width(ctx.list_w);
            paint_macro_rows(ui, ctx.app, ctx.list_w, &node.macros, ctx.clicked_macro);
            for (seg, child) in &node.children {
                let child_path = if path.is_empty() {
                    seg.clone()
                } else {
                    format!("{path}/{seg}")
                };
                // Nested headers: no root separators; slight indent without expand_to_include_x.
                ui.add_space(SPACE_4);
                ui.horizontal(|ui| {
                    ui.add_space(SPACE_12);
                    ui.vertical(|ui| {
                        let nested_w = (ctx.list_w - SPACE_12).max(0.0);
                        ui.set_max_width(nested_w);
                        let mut nested = PaintTagCtx {
                            app: ctx.app,
                            list_w: nested_w,
                            clicked_macro: ctx.clicked_macro,
                            clicked_tag: ctx.clicked_tag,
                        };
                        paint_tag_node(
                            ui,
                            &mut nested,
                            &child_path,
                            seg,
                            child,
                            true, // suppress root separators inside nest
                        );
                    });
                });
            }
        });
}

pub fn show(app: &mut SqyreApp, ui: &mut egui::Ui) {
    // Local copy: `show_collapsible` borrows `&mut bool` for the whole call,
    // and the content closure also needs `&mut app`.
    let mut open = app.macro_list_open;
    let list_w = app.settings_ui.settings().macro_list_width.clamp(
        sqyre_persist::MIN_MACRO_LIST_WIDTH,
        sqyre_persist::MAX_MACRO_LIST_WIDTH,
    );
    egui::Panel::left("macro_list_tags")
        .default_size(list_w)
        .size_range(sqyre_persist::MIN_MACRO_LIST_WIDTH..=sqyre_persist::MAX_MACRO_LIST_WIDTH)
        .show_collapsible(ui, &mut open, |ui| {
            // Side panels persist last-frame content width; never let children
            // request more than the allocated pane or the panel grows every frame.
            let pane_w = ui.available_width();
            ui.set_max_width(pane_w);

            crate::widgets::heading_with_count(ui, "Macros", {
                let filter = app.macro_list_filter.trim();
                if filter.is_empty() {
                    app.workspace.macros.len()
                } else {
                    app.workspace
                        .macros
                        .iter()
                        .filter(|m| pickers::query_matches_name_or_tags(filter, &m.name, &m.tags))
                        .count()
                }
            });
            // True load failures only (corrupt db / undecodable macros). Per-macro
            // validation issues are shown on the macro rows and action tree.
            // List-scoped: same color/prefix rules as StatusBanner panel footers.
            if let Some(err) = &app.workspace.load_error {
                StatusBanner::paint_prefixed_error(ui, PREFIX_LOAD_ERROR, err);
            }
            if let Some(warn) = &app.workspace.platform_warning {
                StatusBanner::paint_warn(ui, warn);
            }
            if let Some(err) = &app.workspace.save_error {
                StatusBanner::paint_prefixed_error(ui, PREFIX_SAVE_ERROR, err);
            }
            ui.horizontal(|ui| {
                // Use ASCII / NotoEmoji glyphs only — fullwidth/math symbols
                // (＋, ⧉) render as tofu in egui's default font stack.
                let new_resp = crate::widgets::icon_button_colored(
                    ui,
                    "+",
                    "New macro",
                    Some(crate::theme::MACRO_START),
                );
                if new_resp.clicked() {
                    app.create_macro();
                }
                let has_sel = !app.workspace.macros.is_empty();
                if ui
                    .add_enabled_ui(has_sel, |ui| {
                        crate::widgets::icon_button(ui, "📄", "Duplicate selected macro")
                    })
                    .inner
                    .clicked()
                {
                    app.duplicate_selected_macro();
                }
                if ui
                    .add_enabled_ui(has_sel, |ui| {
                        crate::widgets::icon_button_colored(
                            ui,
                            "🗑",
                            "Delete selected macro",
                            Some(crate::theme::MACRO_STOP),
                        )
                    })
                    .inner
                    .clicked()
                {
                    let idx = app
                        .workspace
                        .selected_macro
                        .min(app.workspace.macros.len() - 1);
                    app.pending_delete_macro = Some(app.workspace.macros[idx].name.clone());
                }
            });
            ui.add(
                egui::TextEdit::singleline(&mut app.macro_list_filter)
                    .desired_width(pane_w)
                    .hint_text("Search macros or tags…"),
            )
            .on_hover_text("Filter by macro name or tag.");
            {
                let label = if app.workspace.hotkey_tag_filters.is_empty() {
                    "Hotkeys: off".to_string()
                } else {
                    let joined = app
                        .workspace
                        .hotkey_tag_filters
                        .iter()
                        .map(|t| tag_header_label(t))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("Hotkeys: {joined}")
                };
                let font = egui::TextStyle::Small.resolve(ui.style());
                ui.small(elide_to_width(ui, &label, pane_w, font));
            }
            ui.separator();
            let list_h = ui.available_height();
            // Exact slot + clipped child: overflow from ScrollArea/collapsing headers
            // must not widen the side panel's persisted size.
            let (list_rect, _) =
                ui.allocate_exact_size(egui::vec2(pane_w, list_h), egui::Sense::hover());
            let mut list_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(list_rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            list_ui.set_clip_rect(list_rect.intersect(ui.clip_rect()));
            crate::pickers::dialog_scroll(pane_w, list_h).show(&mut list_ui, |ui| {
                let list_w = ui.available_width().min(pane_w);
                ui.set_max_width(list_w);
                let filter = app.macro_list_filter.trim().to_string();
                let (tree, untagged) = build_tag_tree(&app.workspace.macros, &filter);
                let mut clicked_macro: Option<usize> = None;
                let mut clicked_tag: Option<String> = None;
                let has_visible = !tree.children.is_empty() || !untagged.is_empty();
                if !has_visible {
                    if app.workspace.macros.is_empty() {
                        let clicked = crate::widgets::empty_state(
                            ui,
                            "No macros yet",
                            Some("Create a macro to get started."),
                            Some("New macro"),
                            None,
                        );
                        if clicked == crate::widgets::EmptyStateAction::Primary {
                            app.create_macro();
                        }
                    } else {
                        crate::widgets::list_vacancy(ui, &filter, 0, "macros");
                    }
                }
                {
                    let mut ctx = PaintTagCtx {
                        app,
                        list_w,
                        clicked_macro: &mut clicked_macro,
                        clicked_tag: &mut clicked_tag,
                    };
                    let mut first = true;
                    for (seg, child) in &tree.children {
                        paint_tag_node(ui, &mut ctx, seg, seg, child, first);
                        first = false;
                    }
                    if !untagged.is_empty() {
                        let untagged_node = TagTreeNode {
                            macros: untagged,
                            children: BTreeMap::new(),
                        };
                        paint_tag_node(
                            ui,
                            &mut ctx,
                            UNTAGGED_KEY,
                            "Untagged",
                            &untagged_node,
                            first,
                        );
                    }
                }

                if let Some(tag) = clicked_tag {
                    app.toggle_hotkey_tag_filter(tag);
                }
                if let Some(i) = clicked_macro {
                    app.workspace.selected_macro = i;
                    app.tree.selected_actions.clear();
                    app.tree.tooltip.cancel();
                }
            });
        });
    app.macro_list_open = open;

    // Persist user-resized panel width once the drag ends.
    let panel_id = egui::Id::new("macro_list_tags");
    if !ui.ctx().input(|i| i.pointer.any_down()) {
        if let Some(state) = egui::containers::panel::PanelState::load(ui.ctx(), panel_id) {
            let w = state.size().x.clamp(
                sqyre_persist::MIN_MACRO_LIST_WIDTH,
                sqyre_persist::MAX_MACRO_LIST_WIDTH,
            );
            let cur = app.settings_ui.settings().macro_list_width;
            if (w - cur).abs() > 0.5 {
                app.settings_ui.settings_mut().macro_list_width = w;
                let _ = app.settings_ui.save_settings();
            }
        }
    }

    if let Some(name) = app.pending_delete_macro.clone() {
        let pending = app.pending_viewport_scale;
        let open =
            crate::widgets::confirm_window(ui.ctx(), "Delete Macro", pending.as_ref(), |ui| {
                ui.label(format!("Delete macro \"{name}\"?"));
                match crate::widgets::confirm_cancel_row(
                    ui,
                    "Delete",
                    crate::widgets::ConfirmKind::Destructive,
                ) {
                    crate::widgets::ConfirmCancel::Cancel => {
                        app.pending_delete_macro = None;
                    }
                    crate::widgets::ConfirmCancel::Confirm => {
                        app.pending_delete_macro = None;
                        app.delete_macro_named(&name);
                    }
                    crate::widgets::ConfirmCancel::None => {}
                }
            });
        if !open {
            app.pending_delete_macro = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::Macro;

    fn m(name: &str, tags: &[&str]) -> Macro {
        let mut macro_ = Macro::new(name, 0, Vec::new());
        macro_.tags = tags.iter().map(|s| (*s).to_string()).collect();
        macro_
    }

    #[test]
    fn nests_slash_tags_leaf_only() {
        let macros = vec![
            m("alpha", &["combat"]),
            m("beta", &["combat/pve"]),
            m("gamma", &["combat/pvp"]),
            m("delta", &["farm"]),
            m("epsilon", &[]),
        ];
        let (tree, untagged) = build_tag_tree(&macros, "");
        assert_eq!(untagged, vec![4]);
        let combat = tree.children.get("combat").expect("combat");
        assert_eq!(combat.macros, vec![0]);
        assert_eq!(combat.children.get("pve").unwrap().macros, vec![1]);
        assert_eq!(combat.children.get("pvp").unwrap().macros, vec![2]);
        assert_eq!(combat.subtree_macro_count(), 3);
        assert_eq!(tree.children.get("farm").unwrap().macros, vec![3]);
    }

    #[test]
    fn filter_hides_empty_branches() {
        let macros = vec![m("alpha", &["combat/pve"]), m("beta", &["farm"])];
        let (tree, untagged) = build_tag_tree(&macros, "farm");
        assert!(untagged.is_empty());
        assert!(!tree.children.contains_key("combat"));
        assert_eq!(tree.children.get("farm").unwrap().macros, vec![1]);
    }

    #[test]
    fn multi_tag_appears_under_each_leaf() {
        let macros = vec![m("beta", &["combat/pve", "farm"])];
        let (tree, _) = build_tag_tree(&macros, "");
        assert_eq!(
            tree.children
                .get("combat")
                .unwrap()
                .children
                .get("pve")
                .unwrap()
                .macros,
            vec![0]
        );
        assert_eq!(tree.children.get("farm").unwrap().macros, vec![0]);
    }
}
