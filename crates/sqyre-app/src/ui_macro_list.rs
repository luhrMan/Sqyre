//! Left macro list panel and delete confirmation.

use crate::help_text as ht;
use crate::icon_cache::{IconCache, PROCESS_ICON_SIDE};
use crate::pickers;
use crate::status_banner::{StatusBanner, PREFIX_LOAD_ERROR, PREFIX_SAVE_ERROR};
use crate::theme::SPACE_12;
use crate::widgets::elide::{elide_to_width, text_hover_if};
use crate::widgets::tags::{
    filters_cover_path, normalize_tag_path, program_icon_for_tag, tag_pill, tag_pill_width,
};
use crate::SqyreApp;
use eframe::egui::{self, TextureHandle};
use sqyre_domain::Macro;
use sqyre_hotkeys::format_hotkey;
use sqyre_persist::ProgramCatalog;
use std::collections::{BTreeMap, HashMap};

/// Empty-string group key for macros with no tags.
const UNTAGGED_KEY: &str = "";
/// Horizontal offset for one level of nested tag groups (fixed — density gaps
/// change row spacing, not tree indentation).
const NEST_INDENT: f32 = SPACE_12;

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

/// Macro name on the first line; hotkey as a weak small hint below when set.
/// Each line is shown in full when it fits `max_text_width`, otherwise elided with `…`.
/// When `validation_error` is set, the name is tinted and callers should attach hover text.
fn macro_list_item_text(
    ui: &egui::Ui,
    m: &Macro,
    max_text_width: f32,
    validation_error: Option<&str>,
) -> (egui::WidgetText, bool) {
    let style = ui.style();
    let name_font = egui::FontSelection::Default.resolve(style);
    let name = elide_to_width(ui, &m.name, max_text_width, name_font);
    let name_color = if validation_error.is_some() {
        crate::theme::error_fg()
    } else {
        style.visuals.text_color()
    };

    if m.hotkey.is_empty() {
        return (
            egui::RichText::new(name.text).color(name_color).into(),
            name.elided,
        );
    }

    let hotkey_font = egui::TextStyle::Small.resolve(style);
    let hotkey = elide_to_width(ui, &format_hotkey(&m.hotkey), max_text_width, hotkey_font);
    let elided = name.elided || hotkey.elided;

    let mut job = egui::text::LayoutJob::default();
    egui::RichText::new(name.text).color(name_color).append_to(
        &mut job,
        style,
        egui::FontSelection::Default,
        egui::Align::LEFT,
    );
    egui::RichText::new(format!("\n{}", hotkey.text))
        .small()
        .color(style.visuals.weak_text_color())
        .append_to(
            &mut job,
            style,
            egui::FontSelection::Default,
            egui::Align::LEFT,
        );
    (job.into(), elided)
}

fn tag_header_label(tag: &str) -> &str {
    if tag.is_empty() {
        "Untagged"
    } else {
        tag
    }
}

/// Normalized tag path → process icons of every Program whose Macro tags include it.
/// Programs without a process icon are skipped.
fn program_tag_icons(
    ctx: &egui::Context,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
) -> HashMap<String, Vec<TextureHandle>> {
    let mut out: HashMap<String, Vec<TextureHandle>> = HashMap::new();
    let names: Vec<String> = catalog.program_names().cloned().collect();
    for prog in names {
        let tags = catalog.program_macro_tags(&prog);
        let Some(tex) = icons.for_program(ctx, catalog, &prog) else {
            continue;
        };
        for tag in tags {
            let path = normalize_tag_path(&tag);
            if !path.is_empty() {
                out.entry(path).or_default().push(tex.clone());
            }
        }
    }
    out
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

/// What a macro row asked for this frame (click or right-click menu entry).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MacroRowAction {
    Select(usize),
    Duplicate(usize),
    Delete(usize),
}

/// One-shot keyboard effects targeting the selected row this frame.
#[derive(Debug, Default, Clone, Copy)]
struct RowNav {
    /// Scroll the selected row into view.
    scroll: bool,
    /// Move egui focus onto the selected row (Enter from the filter field).
    focus: bool,
}

/// Macro indices in painted order — groups render rows only when expanded.
///
/// Header ids must match `paint_tag_node` (`ui.make_persistent_id`), or the
/// open-state lookup always misses and the order silently collapses to empty.
fn visible_macro_order(ui: &egui::Ui, tree: &TagTreeNode, untagged: &[usize]) -> Vec<usize> {
    let mut out = Vec::new();
    for (seg, child) in &tree.children {
        collect_open_group(ui, seg, child, &mut out);
    }
    if group_is_open(ui, UNTAGGED_KEY) {
        out.extend_from_slice(untagged);
    }
    out
}

fn group_is_open(ui: &egui::Ui, path: &str) -> bool {
    let id = ui.make_persistent_id(("macro_list_tag", path));
    egui::collapsing_header::CollapsingState::load(ui.ctx(), id).is_some_and(|s| s.is_open())
}

fn collect_open_group(ui: &egui::Ui, path: &str, node: &TagTreeNode, out: &mut Vec<usize>) {
    if !group_is_open(ui, path) {
        return;
    }
    out.extend_from_slice(&node.macros);
    for (seg, child) in &node.children {
        let child_path = if path.is_empty() {
            seg.clone()
        } else {
            format!("{path}/{seg}")
        };
        collect_open_group(ui, &child_path, child, out);
    }
}

fn paint_macro_rows(
    ui: &mut egui::Ui,
    app: &SqyreApp,
    list_w: f32,
    indices: &[usize],
    row_action: &mut Option<MacroRowAction>,
    nav: &mut RowNav,
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
        let (label, label_elided) =
            macro_list_item_text(ui, m, text_width, validation_err.as_deref());
        let mut resp = ui.add(
            egui::Button::selectable(app.workspace.selected_macro == i, label)
                .wrap_mode(egui::TextWrapMode::Extend)
                .min_size(egui::vec2(width, 0.0)),
        );
        let full_text = if m.hotkey.is_empty() {
            m.name.clone()
        } else {
            format!("{}\n{}", m.name, format_hotkey(&m.hotkey))
        };
        let hover_text = match (label_elided, validation_err.as_deref()) {
            (true, Some(err)) => Some(format!("{full_text}\n\nValidation: {err}")),
            (true, None) => Some(full_text.clone()),
            (false, Some(err)) => Some(format!("Validation: {err}")),
            (false, None) => None,
        };
        if let Some(hover_text) = hover_text {
            resp = resp.on_hover_text(hover_text);
        } else {
            resp = text_hover_if(resp, &full_text, label_elided);
        }
        if validation_err.is_some() {
            crate::widgets::error_badge(ui, resp.rect);
        }
        if i == app.workspace.selected_macro {
            if nav.scroll {
                ui.scroll_to_rect(resp.rect, Some(egui::Align::Center));
                nav.scroll = false;
            }
            if nav.focus {
                resp.request_focus();
                nav.focus = false;
            }
        }
        if resp.clicked() {
            *row_action = Some(MacroRowAction::Select(i));
        }
        // Multi-tag macros paint once per tag; the row id keeps each copy's menu separate.
        if let Some(action) = crate::widgets::response_context_menu(ui, &resp, |ui| {
            if crate::widgets::menu_item(ui, "Duplicate", true) {
                return Some(MacroRowAction::Duplicate(i));
            }
            if crate::widgets::menu_item_danger(ui, "Delete", true) {
                return Some(MacroRowAction::Delete(i));
            }
            None
        })
        .flatten()
        {
            *row_action = Some(action);
        }
    }
}

struct PaintTagCtx<'a> {
    app: &'a SqyreApp,
    list_w: f32,
    row_action: &'a mut Option<MacroRowAction>,
    clicked_tag: &'a mut Option<String>,
    /// One-shot keyboard effects toward the selected row.
    nav: &'a mut RowNav,
    /// Program icons per tag path (see [`program_tag_icons`]).
    tag_icons: &'a HashMap<String, Vec<TextureHandle>>,
    /// Appearance → compact program headers: icon-only when a tag has program icons.
    compact: bool,
}

fn paint_tag_node(
    ui: &mut egui::Ui,
    ctx: &mut PaintTagCtx<'_>,
    path: &str,
    label: &str,
    node: &TagTreeNode,
) {
    let id = ui.make_persistent_id(("macro_list_tag", path));
    let open =
        egui::collapsing_header::CollapsingState::load(ui.ctx(), id).is_some_and(|s| s.is_open());
    if open {
        crate::widgets::tinted_section(ui, |ui| paint_tag_group(ui, ctx, id, path, label, node));
    } else {
        paint_tag_group(ui, ctx, id, path, label, node);
    }
}

fn paint_tag_group(
    ui: &mut egui::Ui,
    ctx: &mut PaintTagCtx<'_>,
    id: egui::Id,
    path: &str,
    label: &str,
    node: &TagTreeNode,
) {
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
                    .add_enabled(
                        !parent_covered,
                        crate::widgets::IconToggle::new(
                            &mut hotkeys_on,
                            "Hotkeys",
                            egui_phosphor::regular::KEY,
                            egui_phosphor::fill::KEY,
                        ),
                    )
                    .on_hover_text(hover)
                    .on_disabled_hover_text(hover);
                if hotkey_resp.changed() {
                    *ctx.clicked_tag = Some(path.to_string());
                }

                // Remaining width: [program icons] + non-selectable title + count
                // (expand via chevron only).
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.set_max_width(ui.available_width());
                    let prog_icons = ctx.tag_icons.get(path).map_or(&[][..], Vec::as_slice);
                    // Same rule as program headers: hide the name only when an icon shows.
                    let hide_name = ctx.compact && !prog_icons.is_empty();
                    for tex in prog_icons {
                        let icon = ui.add(
                            egui::Image::new((tex.id(), egui::Vec2::splat(PROCESS_ICON_SIDE)))
                                .fit_to_exact_size(egui::Vec2::splat(PROCESS_ICON_SIDE))
                                .maintain_aspect_ratio(true)
                                .sense(egui::Sense::hover()),
                        );
                        if hide_name {
                            icon.on_hover_text(label);
                        }
                    }
                    let font = egui::FontSelection::Default.resolve(ui.style());
                    let count_text = crate::widgets::count_text(count);
                    if !hide_name {
                        let count_w = ui
                            .painter()
                            .layout_no_wrap(count_text.clone(), font.clone(), egui::Color32::WHITE)
                            .size()
                            .x;
                        let name_budget =
                            (ui.available_width() - count_w - ui.spacing().item_spacing.x).max(0.0);
                        let header_text = elide_to_width(ui, label, name_budget, font);
                        let resp = ui.add(
                            egui::Label::new(egui::RichText::new(header_text.text).strong())
                                .selectable(false),
                        );
                        text_hover_if(resp, label, header_text.elided);
                    }
                    crate::widgets::count_suffix(ui, count);
                });
            });
        })
        .body_unindented(|ui| {
            ui.set_max_width(ctx.list_w);
            paint_macro_rows(
                ui,
                ctx.app,
                ctx.list_w,
                &node.macros,
                ctx.row_action,
                ctx.nav,
            );
            for (seg, child) in &node.children {
                let child_path = if path.is_empty() {
                    seg.clone()
                } else {
                    format!("{path}/{seg}")
                };
                // Nested headers: slight indent without expand_to_include_x.
                ui.horizontal(|ui| {
                    ui.add_space(NEST_INDENT);
                    // Item spacing sits between the indent and the column; leaving it
                    // out overflows the viewport and enables horizontal drag-scroll.
                    let nested_w =
                        (ctx.list_w - NEST_INDENT - ui.spacing().item_spacing.x).max(0.0);
                    ui.vertical(|ui| {
                        ui.set_max_width(nested_w);
                        let mut nested = PaintTagCtx {
                            app: ctx.app,
                            list_w: nested_w,
                            row_action: ctx.row_action,
                            clicked_tag: ctx.clicked_tag,
                            nav: ctx.nav,
                            tag_icons: ctx.tag_icons,
                            compact: ctx.compact,
                        };
                        paint_tag_node(ui, &mut nested, &child_path, seg, child);
                    });
                });
            }
        });
}

/// Pinned "Hotkeys:" summary — one pill per selected tag. Pills that cannot
/// fit whole collapse into a `+N` overflow pill (single-row chip rule); each
/// pill carries the tag-chip affordance: right-click removes the tag.
fn paint_hotkey_tag_footer(app: &mut SqyreApp, ui: &mut egui::Ui) {
    if app.workspace.hotkey_tag_filters.is_empty() {
        ui.small("Hotkeys: off");
        return;
    }
    let program_icons = app
        .icon_cache
        .program_name_icons(ui.ctx(), &app.workspace.catalog);
    let filters = &app.workspace.hotkey_tag_filters;
    let pill_w = |ui: &egui::Ui, text: &str| {
        tag_pill_width(
            ui,
            text,
            program_icon_for_tag(Some(&program_icons), text).is_some(),
        )
    };
    let mut remove: Option<String> = None;
    ui.horizontal(|ui| {
        let header = ui.label(egui::RichText::new("Hotkeys:").small().weak());
        let row_right = ui.max_rect().right().min(ui.clip_rect().right());
        let spacing = ui.spacing().item_spacing.x;
        let widths: Vec<f32> = filters
            .iter()
            .map(|t| pill_w(ui, tag_header_label(t)))
            .collect();
        // Room the "+N" marker needs after the last painted pill. Unless all
        // tags fit, every painted tag must leave this room so hidden tags
        // always keep a visible affordance.
        let marker_w = pill_w(ui, &format!("+{}", filters.len())) + spacing;
        let fits = |reserve: f32| {
            let mut x = ui.cursor().left();
            let mut n = 0;
            for w in &widths {
                if x + w + reserve > row_right {
                    break;
                }
                n += 1;
                x += w + spacing;
            }
            n
        };
        let mut shown = fits(0.0);
        if shown < filters.len() {
            shown = fits(marker_w);
        }
        for (i, tag) in filters.iter().enumerate().take(shown) {
            let label = tag_header_label(tag);
            let resp = tag_pill(ui, label, program_icon_for_tag(Some(&program_icons), label))
                .on_hover_text(ht::MLIST_ROW_OPTIONS);
            if crate::widgets::rect_danger_menu(
                ui,
                ui.id().with(("hotkey_tag_pill", i, tag)),
                resp.rect,
                None,
                "Remove",
                true,
            ) {
                remove = Some(tag.clone());
            }
        }
        let overflow = filters.len() - shown;
        if overflow > 0 {
            let count = format!("+{overflow}");
            let hidden = filters[shown..]
                .iter()
                .map(|t| tag_header_label(t))
                .collect::<Vec<_>>()
                .join(", ");
            let tip = format!("{overflow} more: {hidden}");
            if ui.cursor().left() + tag_pill_width(ui, &count, false) <= row_right {
                tag_pill(ui, &count, None).on_hover_text(tip);
            } else {
                header.on_hover_text(tip);
            }
        }
    });
    if let Some(tag) = remove {
        app.toggle_hotkey_tag_filter(tag);
    }
}

fn select_macro_row(app: &mut SqyreApp, i: usize) {
    app.workspace.selected_macro = i;
    app.tree.selected_actions.clear();
    app.tree.tooltip.cancel();
}

pub fn show(app: &mut SqyreApp, ui: &mut egui::Ui) {
    // Local copy: `show_collapsible` borrows `&mut bool` for the whole call,
    // and the content closure also needs `&mut app`.
    let mut open = app.macro_list_open;
    // Panel size is a fraction of the window body it splits, not absolute points,
    // so the 15%–75% limits track window resizes.
    let win_w = ui.max_rect().width().max(1.0);
    let frac = app.settings_ui.settings().macro_list_split.clamp(
        sqyre_persist::MIN_MACRO_LIST_FRAC,
        sqyre_persist::MAX_MACRO_LIST_FRAC,
    );
    let panel_id = egui::Id::new("macro_list_tags");
    // The saved fraction is authoritative: PanelState is absolute pixels and
    // egui skips storing it during divider drags and slide animations, so a
    // stale value (old session, window/WM resize) would otherwise override
    // `default_size`. Realign every frame — the >1px slack keeps the steady
    // state write-free — but skip implausibly narrow frames so a transient
    // small window can't squash the panel.
    if win_w >= 300.0 {
        if let Some(mut state) = egui::containers::panel::PanelState::load(ui.ctx(), panel_id) {
            let want_w = win_w * frac;
            if (state.size().x - want_w).abs() > 1.0 {
                state.outer_rect = egui::Rect::from_min_size(
                    state.outer_rect.min,
                    egui::vec2(want_w, state.outer_rect.height()),
                );
                ui.ctx().data_mut(|d| d.insert_persisted(panel_id, state));
            }
        }
    }
    egui::Panel::right("macro_list_tags")
        .default_size(win_w * frac)
        .size_range(
            (win_w * sqyre_persist::MIN_MACRO_LIST_FRAC)
                ..=(win_w * sqyre_persist::MAX_MACRO_LIST_FRAC),
        )
        // Default side margin is 2px tall, which the window border eats into.
        // Bottom gets the same 8px as the sides so the pinned Hotkeys footer
        // reads with real spacing instead of hugging the screen edge.
        .frame(
            egui::Frame::side_top_panel(ui.style()).inner_margin(egui::Margin {
                left: crate::theme::SPACE_8 as i8,
                right: crate::theme::SPACE_8 as i8,
                top: crate::theme::SPACE_2 as i8,
                bottom: crate::theme::SPACE_8 as i8,
            }),
        )
        .show_collapsible(ui, &mut open, |ui| {
            // Side panels persist last-frame content width; never let children
            // request more than the allocated pane or the panel grows every frame.
            let pane_w = ui.available_width();
            ui.set_max_width(pane_w);

            // New macro sits inside the search pair, after the clear eraser.
            let search = crate::widgets::search_field(
                ui,
                &mut app.macro_list_filter,
                crate::widgets::SearchFieldOpts {
                    hint: "Search macros or tags…",
                    focus_id: None,
                    clearable: true,
                    stable_id: Some(egui::Id::new("macro_list_search")),
                    trailing: Some(crate::widgets::SearchTrailing {
                        glyph: egui_phosphor::regular::PLUS,
                        tip: "New macro",
                        color: Some(crate::theme::MACRO_START),
                    }),
                },
            );
            let search_resp = search.response.on_hover_text(ht::MLIST_SEARCH_TIP);
            if search.trailing_clicked {
                app.create_macro();
            }
            // True load failures only (corrupt db / undecodable macros). Per-macro
            // validation issues are shown on the macro rows and action tree.
            // List-scoped: same color/prefix rules as StatusBanner panel footers.
            if let Some(err) = &app.workspace.load_error {
                StatusBanner::paint_prefixed_error(
                    ui,
                    PREFIX_LOAD_ERROR,
                    &crate::status_banner::user_error_detail(err.as_str()),
                );
            }
            if let Some(warn) = &app.workspace.platform_warning {
                StatusBanner::paint_warn(ui, warn);
            }
            if let Some(err) = &app.workspace.save_error {
                StatusBanner::paint_prefixed_error(
                    ui,
                    PREFIX_SAVE_ERROR,
                    &crate::status_banner::user_error_detail(err.as_str()),
                );
            }
            // Hotkeys summary is a small footer pinned under the list; tag pills
            // add their inner margin above/below the Small text line.
            let footer_h = ui.text_style_height(&egui::TextStyle::Small)
                + crate::theme::SPACE_2 * 2.0
                + crate::theme::element_gap(ui);
            let list_h = (ui.available_height() - footer_h).max(24.0);
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
            crate::widgets::show_dialog_scroll(&mut list_ui, pane_w, list_h, |ui| {
                let list_w = ui.available_width().min(pane_w);
                ui.set_max_width(list_w);
                let filter = app.macro_list_filter.trim().to_string();
                let (tree, untagged) = build_tag_tree(&app.workspace.macros, &filter);
                let tag_icons =
                    program_tag_icons(ui.ctx(), &app.workspace.catalog, &mut app.icon_cache);
                let compact = app.settings_ui.settings().compact_program_headers;
                let mut row_action: Option<MacroRowAction> = None;
                let mut clicked_tag: Option<String> = None;
                let mut row_nav = RowNav::default();

                // Search-then-navigate: while the filter field owns the keyboard,
                // ↑↓ move the selection across visible rows, Enter hands focus to
                // the row, Esc clears the filter (or releases the field).
                let order = visible_macro_order(ui, &tree, &untagged);
                if search_resp.has_focus() {
                    use pickers::ListNavAction;
                    let nav = pickers::poll_list_nav(ui);
                    match nav {
                        ListNavAction::Up | ListNavAction::Down => {
                            if !order.is_empty() {
                                let down = nav == ListNavAction::Down;
                                let next = match order
                                    .iter()
                                    .position(|&i| i == app.workspace.selected_macro)
                                {
                                    Some(p) if down => (p + 1).min(order.len() - 1),
                                    Some(p) => p.saturating_sub(1),
                                    None if down => 0,
                                    None => order.len() - 1,
                                };
                                select_macro_row(app, order[next]);
                                row_nav.scroll = true;
                            }
                        }
                        ListNavAction::Activate => {
                            let visible = order.contains(&app.workspace.selected_macro);
                            row_nav.scroll = visible;
                            row_nav.focus = visible;
                        }
                        ListNavAction::Cancel => {
                            if !app.macro_list_filter.is_empty() {
                                app.macro_list_filter.clear();
                            } else {
                                search_resp.surrender_focus();
                            }
                        }
                        ListNavAction::None => {}
                    }
                }
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
                        row_action: &mut row_action,
                        clicked_tag: &mut clicked_tag,
                        nav: &mut row_nav,
                        tag_icons: &tag_icons,
                        compact,
                    };
                    for (seg, child) in &tree.children {
                        paint_tag_node(ui, &mut ctx, seg, seg, child);
                    }
                    if !untagged.is_empty() {
                        let untagged_node = TagTreeNode {
                            macros: untagged,
                            children: BTreeMap::new(),
                        };
                        paint_tag_node(ui, &mut ctx, UNTAGGED_KEY, "Untagged", &untagged_node);
                    }
                }

                if let Some(tag) = clicked_tag {
                    app.toggle_hotkey_tag_filter(tag);
                }
                match row_action {
                    Some(MacroRowAction::Select(i)) => select_macro_row(app, i),
                    Some(MacroRowAction::Duplicate(i)) => {
                        select_macro_row(app, i);
                        app.duplicate_selected_macro();
                    }
                    Some(MacroRowAction::Delete(i)) => {
                        if let Some(m) = app.workspace.macros.get(i) {
                            app.pending_delete_macro = Some(m.name.clone());
                        }
                    }
                    None => {}
                }
            });
            paint_hotkey_tag_footer(app, ui);
        });
    app.macro_list_open = open;

    // Persist user-resized panel width only when a divider drag ends. Reading
    // PanelState on every idle frame would corrupt the fraction: it is absolute
    // pixels, so window resizes (maximize, tiling, WM startup sizing) change
    // size.x/win_w without the user touching the divider. "__resize" is egui's
    // resize-handle id suffix (panel.rs `resize_widget_id`).
    let divider_released = ui
        .ctx()
        .read_response(panel_id.with("__resize"))
        .is_some_and(|r| r.drag_stopped());
    if divider_released && win_w >= 300.0 {
        if let Some(state) = egui::containers::panel::PanelState::load(ui.ctx(), panel_id) {
            let frac = (state.size().x / win_w).clamp(
                sqyre_persist::MIN_MACRO_LIST_FRAC,
                sqyre_persist::MAX_MACRO_LIST_FRAC,
            );
            let cur = app.settings_ui.settings().macro_list_split;
            if (frac - cur).abs() > 0.005 {
                app.settings_ui.settings_mut().macro_list_split = frac;
                let _ = app.settings_ui.save_settings();
            }
        }
    }

    if let Some(name) = app.pending_delete_macro.clone() {
        let pending = app.pending_viewport_scale;
        let open =
            crate::widgets::confirm_window(ui.ctx(), "Delete Macro", pending.as_ref(), |ui| {
                ui.label(format!("Delete macro \"{name}\"?"));
                match crate::widgets::confirm_cancel_row(ui, "Delete") {
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

    /// Tags that would fill the row to the edge must still leave room for the
    /// "+N" pill — otherwise hidden filters vanish without any affordance.
    #[test]
    fn hotkey_tag_footer_keeps_overflow_pill_in_narrow_pane() {
        let mut app = crate::SqyreApp::for_docs();
        app.workspace.hotkey_tag_filters = vec!["alpha", "beta", "gamma", "delta", "epsilon"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let ctx = egui::Context::default();
        let mut plus = None;
        for _ in 0..3 {
            let out = ctx.run_ui(egui::RawInput::default(), |ui| {
                let pane = egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(120.0, 400.0));
                let mut pane_ui = ui.new_child(egui::UiBuilder::new().max_rect(pane));
                pane_ui.set_clip_rect(pane);
                paint_hotkey_tag_footer(&mut app, &mut pane_ui);
            });
            plus = plus.or_else(|| {
                out.shapes.iter().find_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) if text.galley.job.text.starts_with('+') => {
                        Some((text.galley.job.text.clone(), clipped.clip_rect))
                    }
                    _ => None,
                })
            });
            out.drop_without_applying_deltas();
        }
        let (text, clip) = plus.expect("+N overflow pill was not painted");
        assert!(
            text[1..].parse::<usize>().is_ok(),
            "unexpected pill text {text:?}"
        );
        assert!(
            clip.width() >= 110.0,
            "+N pill painted under a sliver clip: {clip:?}"
        );
    }

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
