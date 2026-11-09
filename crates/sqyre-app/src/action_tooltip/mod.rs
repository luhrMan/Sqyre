//! In-tree action tooltip: view on hover, pinned edit with Save/Cancel.

mod edit;
mod edit_header;
pub(crate) mod help;
mod sections;

use crate::pickers::{self, ActivePicker, PickerResult};
use crate::tree_chrome::{self, RowInteraction};
use eframe::egui::{self, Key, Modifiers, Order, Vec2};
use sqyre_domain::{
    action_type_description, action_type_label, Action, ActionId, ActionKind, CoordinateRef, Macro,
};
use sqyre_persist::ProgramCatalog;
use sqyre_ui_model::{action_pastel_color, split_display_params, ActionDisplay};
use sqyre_validate::validate_action_persist;

use crate::paint_ctx::{CatalogPaint, EditFieldsCtx, RecordBridges, TipUiCtx, VarTheme};
use crate::var_pills;
use crate::widgets::SaveCancel;

pub use edit::apply_draft_preserving_children;
pub(crate) use edit::paint_edit_fields;
pub(crate) use edit_header::paint_action_edit_header;

/// Pinned edit payload (boxed so [`TooltipState`] stays small).
#[derive(Debug, Clone)]
pub struct TooltipEdit {
    pub action_id: ActionId,
    pub draft: Action,
    /// Snapshot at open; Save enables when `draft` differs (or provisional insert).
    baseline: Action,
    pub error: Option<String>,
    /// Screen position when edit opened (pinned, not mouse-follow).
    pub anchor: egui::Pos2,
    pub picker: ActivePicker,
    /// When true, Cancel / Escape / close removes this action from the tree
    /// (used for freshly inserted blank actions that were never saved).
    pub discard_on_cancel: bool,
    /// Extra ids removed with the provisional action (e.g. auto-paired key/click Up).
    pub discard_companions: Vec<ActionId>,
    /// Unclipped height of the fields column from the last frame.
    fields_height: f32,
    /// While true, size the window height to content (capped at screen max).
    /// Cleared after height settles, or when the user resizes. Re-enabled when
    /// the Advanced section is toggled so newly shown widgets grow the window.
    auto_fit: bool,
}

impl TooltipEdit {
    /// Provisional inserts are always saveable; otherwise require a draft change.
    fn save_enabled(&self) -> bool {
        self.discard_on_cancel || self.draft != self.baseline
    }
}

/// Tooltip lifecycle (editing flag + hover ownership).
#[derive(Debug, Clone, Default)]
pub enum TooltipState {
    #[default]
    Hidden,
    View {
        action_id: ActionId,
    },
    Edit(Box<TooltipEdit>),
}

impl TooltipState {
    pub fn is_editing(&self) -> bool {
        matches!(self, Self::Edit { .. })
    }

    pub fn action_id(&self) -> Option<ActionId> {
        match self {
            Self::Hidden => None,
            Self::View { action_id } => Some(*action_id),
            Self::Edit(edit) => Some(edit.action_id),
        }
    }

    pub fn open_view(&mut self, action_id: ActionId) {
        if self.is_editing() {
            return;
        }
        *self = Self::View { action_id };
    }

    pub fn open_edit(&mut self, action: &Action, anchor: egui::Pos2) {
        *self = Self::Edit(Box::new(TooltipEdit {
            action_id: action.id,
            draft: action.clone(),
            baseline: action.clone(),
            error: None,
            anchor,
            picker: ActivePicker::None,
            discard_on_cancel: false,
            discard_companions: Vec::new(),
            fields_height: 0.0,
            auto_fit: true,
        }));
    }

    /// Edit a freshly inserted blank action; Cancel removes it (and companions) from the tree.
    pub fn open_edit_new(
        &mut self,
        action: &Action,
        anchor: egui::Pos2,
        discard_companions: Vec<ActionId>,
    ) {
        *self = Self::Edit(Box::new(TooltipEdit {
            action_id: action.id,
            draft: action.clone(),
            baseline: action.clone(),
            error: None,
            anchor,
            picker: ActivePicker::None,
            discard_on_cancel: true,
            discard_companions,
            fields_height: 0.0,
            auto_fit: true,
        }));
    }

    /// Close the tooltip. Returns action ids that should be removed from the tree
    /// when a provisional new-action edit was cancelled.
    pub fn cancel(&mut self) -> Vec<ActionId> {
        let discard = match self {
            Self::Edit(edit) if edit.discard_on_cancel => {
                let mut ids = vec![edit.action_id];
                ids.append(&mut edit.discard_companions);
                ids
            }
            _ => Vec::new(),
        };
        *self = Self::Hidden;
        discard
    }

    pub fn dismiss_view(&mut self) {
        if matches!(self, Self::View { .. }) {
            *self = Self::Hidden;
        }
    }

    /// Validate draft (with live children) then apply. On failure keeps Edit + error.
    /// Image search may persist with no target items or search area; the macro
    /// tree shows that incompleteness via [`sqyre_validate::validate_action`].
    /// `before_mutate` runs after validation succeeds and before the tree is changed
    /// (for undo snapshots); it receives the pre-mutation root.
    pub fn try_save_validated(
        &mut self,
        root: &mut Action,
        macro_: Option<&Macro>,
        mut before_mutate: impl FnMut(&Action),
    ) -> bool {
        let Self::Edit(edit) = self else {
            return false;
        };
        let action_id = edit.action_id;
        let draft = edit.draft.clone();

        let Some(live) = root.find_by_id(action_id) else {
            edit.error = Some(format!("action {action_id} not found"));
            return false;
        };

        let mut candidate = draft;
        let preserved_then = live.children().to_vec();
        let preserved_else = live.else_children().map(|c| c.to_vec()).unwrap_or_default();
        if let Some(kids) = candidate.children_mut() {
            *kids = preserved_then;
        }
        if let Some(kids) = candidate.else_children_mut() {
            *kids = preserved_else;
        }
        candidate.id = live.id;

        if let Err(e) = validate_action_persist(&candidate, macro_) {
            edit.error = Some(e.to_string());
            return false;
        }

        before_mutate(root);

        let Some(live) = root.find_by_id_mut(action_id) else {
            edit.error = Some(format!("action {action_id} not found"));
            return false;
        };
        if let Err(e) = apply_draft_preserving_children(live, candidate) {
            edit.error = Some(e);
            return false;
        }

        *self = Self::View { action_id };
        true
    }

    /// Apply a key captured by [`KeyRecordUi`] onto a Key-action draft.
    pub fn apply_recorded_key(&mut self, recorded: String) {
        let Self::Edit(edit) = self else {
            return;
        };
        crate::recorded_action::apply_recorded_key(&mut edit.draft.kind, recorded);
    }

    /// Apply a chord from [`HotkeyRecordUi`] onto a Pause continue-key draft.
    /// Returns true when the draft was a Pause action.
    pub fn apply_recorded_chord(&mut self, recorded: Vec<String>) -> bool {
        let Self::Edit(edit) = self else {
            return false;
        };
        crate::recorded_action::apply_recorded_chord(&mut edit.draft.kind, recorded)
    }

    /// Apply a hex color from the Find Pixel screen dropper onto the draft.
    #[cfg(feature = "native-runtime")]
    pub fn apply_recorded_color(&mut self, recorded: String) {
        let Self::Edit(edit) = self else {
            return;
        };
        crate::recorded_action::apply_recorded_color(&mut edit.draft.kind, recorded);
    }

    /// Surface a dropper/sample failure on the open edit dialog (no-op otherwise).
    #[cfg(feature = "native-runtime")]
    pub fn note_recorded_color_error(&mut self, msg: String) {
        if let Self::Edit(edit) = self {
            edit.error = Some(msg);
        }
    }

    /// Escape handling. Returns `(consumed, discard_ids)` — discard_ids is set when a
    /// provisional new-action edit is fully cancelled.
    pub fn handle_escape(&mut self) -> (bool, Vec<ActionId>) {
        match self {
            Self::Hidden => (false, Vec::new()),
            Self::Edit(edit)
                if matches!(
                    edit.picker,
                    ActivePicker::Coord {
                        cell_pick: Some(_),
                        ..
                    }
                ) =>
            {
                if let ActivePicker::Coord { cell_pick, .. } = &mut edit.picker {
                    *cell_pick = None;
                }
                (true, Vec::new())
            }
            Self::Edit(edit)
                if matches!(
                    edit.picker,
                    ActivePicker::Items { .. }
                        | ActivePicker::Coord { .. }
                        | ActivePicker::Macro { .. }
                        | ActivePicker::Window { .. }
                ) =>
            {
                edit.picker = ActivePicker::None;
                (true, Vec::new())
            }
            Self::View { .. } | Self::Edit { .. } => (true, self.cancel()),
        }
    }
}

/// Process one row's interaction against the tooltip state.
pub fn ingest_row(
    state: &mut TooltipState,
    action: &Action,
    interaction: RowInteraction,
    pointer: Option<egui::Pos2>,
) {
    let open_edit = match interaction.action {
        tree_chrome::RowAction::Edit => true,
        tree_chrome::RowAction::None => interaction.double_clicked || interaction.middle_clicked,
        tree_chrome::RowAction::Logs | tree_chrome::RowAction::Delete => return,
    };

    if open_edit {
        let anchor = pointer.unwrap_or(egui::pos2(40.0, 40.0));
        state.open_edit(action, anchor);
        return;
    }

    if state.is_editing() {
        return;
    }

    if interaction.primary_clicked {
        state.dismiss_view();
        return;
    }

    if interaction.hovered {
        state.open_view(action.id);
    }
}

/// After all rows: if nothing hovered and we are in View, hide.
pub fn end_hover_pass(state: &mut TooltipState, any_view_hover: bool) {
    if matches!(state, TooltipState::View { .. }) && !any_view_hover {
        state.dismiss_view();
    }
}

/// Paint view or edit tooltip for the current frame.
///
fn edit_tip_area_id(action_id: ActionId) -> egui::Id {
    // Bump salt when changing default/min/max sizing so persisted locked sizes are discarded.
    egui::Id::new(("action_edit_tip", "grow_v12", action_id))
}

/// View tips float at tooltip order; the edit window (or its picker) must be on top.
fn tip_owns_escape(state: &TooltipState, ctx: &egui::Context) -> bool {
    match state {
        TooltipState::Hidden => false,
        TooltipState::View { .. } => true,
        TooltipState::Edit(edit) => {
            !matches!(edit.picker, ActivePicker::None)
                || crate::focus_nav::is_top_layer(
                    ctx,
                    egui::LayerId::new(Order::Middle, edit_tip_area_id(edit.action_id)),
                )
        }
    }
}

/// Returns action ids that should be removed when a provisional new-action
/// edit was cancelled without saving.
pub fn show(
    state: &mut TooltipState,
    ctx: &egui::Context,
    macro_: &mut Macro,
    macros: &[(String, Vec<String>)],
    ui: &mut TipUiCtx<'_>,
    mut before_mutate: impl FnMut(&Action),
) -> Vec<ActionId> {
    let TipUiCtx {
        paint,
        theme,
        bridges,
        ..
    } = ui;
    if !bridges.key_record.is_open()
        && !bridges.hotkey_record.is_open()
        && !bridges.screen_click.is_armed()
        && tip_owns_escape(state, ctx)
        && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
    {
        let (consumed, discard) = state.handle_escape();
        if consumed {
            return discard;
        }
    }

    match state {
        TooltipState::Hidden => Vec::new(),
        TooltipState::View { action_id } => {
            let action_id = *action_id;
            let Some(action) = macro_.root.find_by_id(action_id) else {
                *state = TooltipState::Hidden;
                return Vec::new();
            };
            show_view_tip(ctx, action, paint, *theme);
            Vec::new()
        }
        TooltipState::Edit { .. } => {
            show_edit_window(state, ctx, macro_, macros, ui, &mut before_mutate)
        }
    }
}

/// Largest fraction of the constrain width a tip may cover.
/// Tips shrink to their content below this cap.
const TIP_MAX_WIDTH_FRACTION: f32 = 0.50;

/// Shared width cap for view and edit tips so mode switches don't jump size.
///
/// Half the egui constrain rect (window/viewport), with `FLOATER_MIN_COMPACT`
/// as the floor on narrow windows.
fn tip_max_width(screen_w: f32) -> f32 {
    (screen_w * TIP_MAX_WIDTH_FRACTION).max(crate::widgets::FLOATER_MIN_COMPACT[0])
}

/// Comfortable prose column inside a view tip. Wrap-capable content
/// (descriptions, captions, status text) claims at most this — letting it
/// measure against `tip_max_width` made every tip balloon to half the window.
const TIP_WRAP_W: f32 = 380.0;

/// Floor for tips whose content is only a short line (avoids a sliver tip).
const TIP_MIN_W: f32 = 200.0;

/// Icon + chip padding allowance when measuring `extra` param rows as text.
const PARAM_ICON_PAD: f32 = 36.0;

/// Width the view-tip body wants: wrap-capable text capped at [`TIP_WRAP_W`],
/// plus intrinsic room for content that cannot wrap (param rows, target grids,
/// previews). Always ≤ `max_w`.
fn tip_preferred_width(
    ctx: &egui::Context,
    action: &Action,
    extra: &[&sqyre_ui_model::DisplayParam],
    catalog: &ProgramCatalog,
    max_w: f32,
) -> f32 {
    let type_key = action.type_key();
    let font = egui::TextStyle::Small.resolve(ctx.global_style().as_ref());
    let text_w = |s: String| {
        ctx.fonts_mut(|f| f.layout_no_wrap(s, font.clone(), egui::Color32::WHITE))
            .size()
            .x
    };
    let mut w = text_w(action_type_description(type_key).to_owned())
        .max(text_w(action_type_label(type_key).to_owned()) + 40.0)
        .clamp(TIP_MIN_W, TIP_WRAP_W);
    for p in extra {
        w = w.max(text_w(format!("{}: {}", p.label, p.minimal())) + PARAM_ICON_PAD);
    }
    if crate::preview_tooltip::coordinate_ref_for_preview(action).is_some() {
        w = w.max(crate::preview_tooltip::DISPLAY_MAX_W);
    }
    if let ActionKind::ImageSearch(d) = &action.kind {
        if !d.targets.is_empty() || !d.target_tags.is_empty() {
            let display = tree_chrome::cached_sorted_image_search_targets(
                ctx,
                catalog,
                &d.targets,
                &d.target_tags,
                d.sort_by,
                d.sort_then,
                &d.tag_priority,
            );
            // Widest grid the section paints: tag grids size as `display` when
            // it exists, else as the largest tag group; the items grid uses
            // `display.len()`. Mirror `paint_image_search_tooltip_thumbs_pub`.
            let count = if display.is_empty() {
                tree_chrome::image_search_tip_tag_groups(
                    catalog,
                    &d.target_tags,
                    d.sort_by,
                    d.sort_then,
                    &d.tag_priority,
                )
                .iter()
                .map(|g| g.members.len())
                .max()
                .unwrap_or(0)
            } else {
                display.len()
            };
            if count > 0 {
                let cell = crate::pickers::adaptive_icon_cell(
                    count,
                    max_w,
                    crate::pickers::EDIT_CELL,
                    crate::pickers::EDIT_CELL_MAX,
                    crate::pickers::EDIT_GAP,
                );
                let cols = crate::pickers::grid_column_count_for_width(
                    max_w,
                    cell,
                    crate::pickers::EDIT_GAP,
                )
                .min(count);
                w = w.max(
                    cols as f32 * cell + cols.saturating_sub(1) as f32 * crate::pickers::EDIT_GAP,
                );
            }
        }
    }
    w.clamp(TIP_MIN_W, max_w)
}

/// True when the user is dragging any window edge/corner resize handle.
fn edit_window_user_resized(ctx: &egui::Context, area_id: egui::Id) -> bool {
    let dragged = |id: egui::Id| {
        ctx.read_response(id)
            .is_some_and(|r| r.dragged() || r.drag_stopped())
    };
    for salt in [
        "left",
        "right",
        "top",
        "bottom",
        "left_top",
        "right_top",
        "left_bottom",
        "right_bottom",
    ] {
        if dragged(area_id.with(salt)) {
            return true;
        }
    }
    // Interior Resize corner widget used by `Window`.
    dragged(area_id.with("resize").with("__resize_corner"))
}

/// Primary click outside the edit window, once it has already been on screen.
///
/// `blocked` skips the check while a record modal or screen click owns the
/// pointer. Layers above the window (combo menus, suggestion lists, pickers)
/// and sublayers of the window itself are not outside.
fn edit_tip_clicked_outside(ctx: &egui::Context, area_id: egui::Id, blocked: bool) -> bool {
    if blocked {
        return false;
    }
    let layer = egui::LayerId::new(Order::Middle, area_id);
    let was_open = ctx.memory(|m| m.areas().visible_last_frame(&layer));
    if !was_open {
        return false;
    }
    let Some(pos) = ctx.input(|i| {
        if i.pointer.button_clicked(egui::PointerButton::Primary) {
            i.pointer.interact_pos()
        } else {
            None
        }
    }) else {
        return false;
    };
    if ctx
        .memory(|m| m.area_rect(area_id))
        .is_some_and(|rect| rect.contains(pos))
    {
        return false;
    }
    if let Some(hit) = ctx.layer_id_at(pos) {
        if hit.order > Order::Middle {
            return false;
        }
        if hit == layer || ctx.memory(|m| m.areas().parent_layer(hit) == Some(layer)) {
            return false;
        }
    }
    true
}

/// Vertical popup chrome around the view tip body (inner margin + stroke).
const VIEW_TIP_CHROME_H: f32 = 20.0;

/// Scroll position of the pointer-following view tip, kept across frames.
#[derive(Debug, Clone, Copy)]
struct ViewTipScroll {
    tip_id: egui::Id,
    offset: f32,
    max_offset: f32,
    pass: u64,
}

fn view_tip_scroll_key() -> egui::Id {
    egui::Id::new("action_view_tip_scroll")
}

/// Route the mouse wheel to a view tip that was cut off last frame.
///
/// The tip follows the pointer and is not interactable, so its own
/// `ScrollArea` never sees wheel input. Must run before any other
/// `ScrollArea` this frame so the tree underneath does not scroll too.
pub(crate) fn claim_view_tip_scroll(ctx: &egui::Context) {
    let key = view_tip_scroll_key();
    let Some(mut scroll) = ctx.data(|d| d.get_temp::<ViewTipScroll>(key)) else {
        return;
    };
    if scroll.max_offset <= 0.0 || scroll.pass + 1 != ctx.cumulative_pass_nr() {
        return;
    }
    let dy = ctx.input(|i| i.smooth_scroll_delta.y);
    if dy == 0.0 {
        return;
    }
    scroll.offset = (scroll.offset - dy).clamp(0.0, scroll.max_offset);
    ctx.input_mut(|i| i.smooth_scroll_delta.y = 0.0);
    ctx.data_mut(|d| d.insert_temp(key, scroll));
}

/// Paint a read-only hover tip for an action (tree rows + Add Action defaults preview).
pub(crate) fn show_action_view_tip(
    ctx: &egui::Context,
    tip_id: egui::Id,
    action: &Action,
    paint: &mut CatalogPaint<'_>,
    theme: VarTheme<'_>,
) {
    let CatalogPaint {
        catalog,
        icons,
        previews,
    } = paint;
    let VarTheme {
        known_vars,
        is_dark,
    } = theme;
    let Some(pointer) = ctx.pointer_interact_pos() else {
        return;
    };
    let type_key = action.type_key();
    let label = action_type_label(type_key);
    let description = action_type_description(type_key);
    let params = action.display_params_for_tree();
    let (_, extra) = split_display_params(&params);
    let pastel = tree_chrome::rgba_pub(action_pastel_color(type_key, is_dark));
    let coord_preview = crate::preview_tooltip::coordinate_ref_for_preview(action);
    let summary_pills = action.tree_summary_pills();

    // Prefer growing left when near the right edge so constrain() is less likely
    // to slide the tip over the hovered row (which would steal hover).
    let screen = crate::widgets::dialog_constrain_rect(ctx);
    let max_w = tip_max_width(screen.width());
    let preferred_w = tip_preferred_width(ctx, action, &extra, catalog, max_w);
    let near_right = pointer.x + 14.0 + max_w > screen.right();
    let (pivot, pos) = if near_right {
        (egui::Align2::RIGHT_TOP, pointer + Vec2::new(-8.0, 14.0))
    } else {
        (egui::Align2::LEFT_TOP, pointer + Vec2::new(14.0, 14.0))
    };
    let scroll_key = view_tip_scroll_key();
    let pass = ctx.cumulative_pass_nr();
    let offset = ctx
        .data(|d| d.get_temp::<ViewTipScroll>(scroll_key))
        .filter(|s| s.tip_id == tip_id && s.pass + 1 >= pass)
        .map_or(0.0, |s| s.offset);
    let max_body_h = (screen.height() - VIEW_TIP_CHROME_H).max(40.0);
    // interactable(false): clicks pass through to the control underneath.
    egui::Area::new(tip_id)
        .order(Order::Tooltip)
        .pivot(pivot)
        .fixed_pos(pos)
        .interactable(false)
        .sense(egui::Sense::hover())
        .constrain_to(screen)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .inner_margin(egui::Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    // Area content is capped at last frame's size; lift both axes so
                    // the tip can grow up to the constrain rect before scrolling.
                    // Width uses the content-derived preferred width, not `max_w`,
                    // so wrap-capable children (prose, captions, error labels)
                    // cannot inflate the tip to the half-viewport cap.
                    ui.set_max_width(preferred_w);
                    ui.set_max_height(max_body_h);
                    // V-only: tip body wraps prose and icon grids to the body width.
                    // Wheel claiming in `claim_view_tip_scroll` tracks Y offset.
                    let out = crate::widgets::show_scroll_edge_fit(
                        ui,
                        crate::widgets::scroll_vertical()
                            .id_salt("view_tip_body")
                            .max_height(max_body_h)
                            .vertical_scroll_offset(offset),
                        |ui| {
                            tree_chrome::paint_pill_tip(ui, label, pastel);
                            ui.label(egui::RichText::new(description).small().weak());
                            if !summary_pills.is_empty() {
                                crate::theme::add_section_gap(ui);
                                sections::tip_wrapped_section(ui, |ui| {
                                    ui.spacing_mut().item_spacing =
                                        Vec2::splat(crate::theme::SPACE_2);
                                    let band_h = var_pills::summary_pill_height(ui);
                                    for pill in &summary_pills {
                                        tree_chrome::paint_summary_pill_cell(
                                            ui,
                                            action,
                                            pill,
                                            catalog,
                                            icons,
                                            VarTheme {
                                                known_vars,
                                                is_dark,
                                            },
                                            band_h,
                                            true,
                                        );
                                    }
                                });
                            }
                            if let Some((coord_ref, kind)) = coord_preview {
                                sections::tip_section(ui, |ui| {
                                    previews.paint_for_coordinate_ref(
                                        ui, catalog, &coord_ref, kind, false, false,
                                    );
                                });
                            }
                            if let ActionKind::ImageSearch(d) = &action.kind {
                                if !d.targets.is_empty() || !d.target_tags.is_empty() {
                                    sections::tip_section(ui, |ui| {
                                        tree_chrome::paint_image_search_tooltip_thumbs_pub(
                                            ui, action, catalog, icons,
                                        );
                                    });
                                }
                            }
                            if !extra.is_empty() {
                                sections::tip_section(ui, |ui| {
                                    for p in &extra {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(format!("{}:", p.label))
                                                    .small()
                                                    .strong(),
                                            );
                                            if p.label.eq_ignore_ascii_case("Program") {
                                                crate::icon_cache::paint_program_icon(
                                                    ui,
                                                    catalog,
                                                    icons,
                                                    p.minimal(),
                                                );
                                            } else if p.label.eq_ignore_ascii_case("App") {
                                                if let ActionKind::FocusWindow {
                                                    process_path,
                                                    window_title,
                                                    ..
                                                } = &action.kind
                                                {
                                                    crate::icon_cache::paint_leading_process_icon(
                                                        ui,
                                                        icons,
                                                        process_path,
                                                        window_title,
                                                    );
                                                }
                                            }
                                            var_pills::paint_var_ref_content(
                                                ui,
                                                p.minimal(),
                                                known_vars,
                                                is_dark,
                                                if p.minimal() == CoordinateRef::UNSET_LABEL {
                                                    crate::theme::warn_fg()
                                                } else {
                                                    ui.visuals().text_color()
                                                },
                                            );
                                        });
                                    }
                                });
                            }
                        },
                    );
                    let max_offset = (out.content_size.y - out.inner_rect.height()).max(0.0);
                    ui.ctx().data_mut(|d| {
                        d.insert_temp(
                            scroll_key,
                            ViewTipScroll {
                                tip_id,
                                offset: out.state.offset.y.min(max_offset),
                                max_offset,
                                pass,
                            },
                        )
                    });
                });
        });
}

fn show_view_tip(
    ctx: &egui::Context,
    action: &Action,
    paint: &mut CatalogPaint<'_>,
    theme: VarTheme<'_>,
) {
    show_action_view_tip(
        ctx,
        egui::Id::new(("action_hover_tip", action.id)),
        action,
        paint,
        theme,
    );
}

fn show_edit_window(
    state: &mut TooltipState,
    ctx: &egui::Context,
    macro_: &mut Macro,
    macros: &[(String, Vec<String>)],
    ui: &mut TipUiCtx<'_>,
    before_mutate: &mut dyn FnMut(&Action),
) -> Vec<ActionId> {
    let TipUiCtx {
        paint,
        theme,
        bridges,
        compact_program_headers,
        pending_scale,
    } = ui;
    let VarTheme { is_dark, .. } = *theme;
    let (action_id, anchor, type_key) = match state {
        TooltipState::Edit(edit) => (edit.action_id, edit.anchor, edit.draft.type_key()),
        _ => return Vec::new(),
    };

    let label = action_type_label(type_key);
    let pastel = tree_chrome::rgba_pub(action_pastel_color(type_key, is_dark));
    let screen = crate::widgets::dialog_constrain_rect(ctx);
    let max_w = tip_max_width(screen.width());
    // Grow with content from a modest default; ScrollArea caps at the window height.
    const EDIT_CHROME: f32 = 72.0; // type pill + Save/Cancel + separator + margins
    let max_scroll_h = (screen.height() - EDIT_CHROME).max(100.0);

    // Picker modal first (foreground); apply result onto draft.
    if let TooltipState::Edit(edit) = state {
        let result = pickers::show_active_picker(
            ctx,
            &mut edit.picker,
            paint,
            macros,
            *compact_program_headers,
            *pending_scale,
        );
        apply_picker_result(&mut edit.draft, result);
    }

    let CatalogPaint {
        catalog,
        icons,
        previews,
    } = paint;

    let mut save = false;
    let mut cancel = false;
    let mut open = true;

    // Stable Area id (also keys egui's resize state as `area_id.with("resize")`).
    let area_id = edit_tip_area_id(action_id);
    let (fitting, fit_fields_h) = match state {
        TooltipState::Edit(edit) => (edit.auto_fit, edit.fields_height),
        _ => (false, 0.0),
    };
    // While fitting, raise min height to the last measured fields column so
    // egui's sticky Resize `desired_size` grows. Content is pinned with
    // `fill_resize_body` so sections cannot lock/ratchet horizontal size.
    let fit_min_h = if fitting && fit_fields_h > 0.0 {
        (EDIT_CHROME + fit_fields_h.min(max_scroll_h)).clamp(48.0, screen.height())
    } else {
        48.0
    };

    // Popup chrome (no title bar). Height is driven by `fit_min_h` while
    // `auto_fit` is set. Default width matches the view tip; user can drag
    // wider (up to the dialog edge margin) or narrower (down to min_size).
    // `fit_dialog_popup` already caps max_size to `dialog_constrain_rect`.
    crate::widgets::fit_dialog_popup(
        egui::Window::new(label)
            .open(&mut open)
            .title_bar(false)
            .collapsible(false)
            .resizable(true)
            .default_pos(anchor + Vec2::new(crate::theme::SPACE_12, crate::theme::SPACE_12))
            .default_size([max_w, 1.0])
            .min_size([crate::widgets::FLOATER_MIN_COMPACT[0], fit_min_h])
            .frame(
                egui::Frame::popup(ctx.global_style().as_ref())
                    .inner_margin(egui::Margin::symmetric(10, 6)),
            ),
        ctx,
        area_id,
        *pending_scale,
    )
    .show(ctx, |ui| {
        crate::widgets::fill_resize_body(ui, |ui| {
            let (err, save_enabled) = match state {
                TooltipState::Edit(edit) => (edit.error.as_deref(), edit.save_enabled()),
                _ => (None, false),
            };
            match paint_action_edit_header(ui, label, pastel, None, err, save_enabled, true) {
                SaveCancel::Cancel => cancel = true,
                SaveCancel::Save => save = true,
                SaveCancel::None => {}
            }
            if let TooltipState::Edit(edit) = state {
                let mut fields = EditFieldsCtx {
                    paint: CatalogPaint {
                        catalog,
                        icons,
                        previews,
                    },
                    bridges: RecordBridges {
                        key_record: bridges.key_record,
                        hotkey_record: bridges.hotkey_record,
                        macro_hotkeys: bridges.macro_hotkeys,
                        screen_click: bridges.screen_click,
                    },
                    theme: *theme,
                    macros,
                    active_macro: Some(&*macro_),
                };
                let scroll_h = ui.available_height().max(40.0).min(max_scroll_h);
                // No x-shrink: the window is user-resized, not content-sized, so
                // a shrunk viewport would float the V-bar left of the frame edge.
                let measured = crate::widgets::show_dialog_scroll_mod(
                    ui,
                    ui.available_width(),
                    scroll_h,
                    |s| s.id_salt("edit_fields"),
                    |ui| {
                        edit::paint_edit_fields(
                            ui,
                            &mut edit.draft,
                            &mut edit.picker,
                            &mut fields,
                        );
                    })
                    .content_size
                    .y;
                let prev_h = edit.fields_height;
                let height_stable = (measured - prev_h).abs() <= 1.0;
                if !height_stable {
                    ui.ctx().request_repaint();
                }
                edit.fields_height = measured;

                let refit = sections::take_edit_tip_refit(ui.ctx());
                if refit {
                    edit.auto_fit = true;
                    #[cfg(feature = "native-runtime")]
                    if sqyre_capture::disk_logging_enabled() {
                        sqyre_capture::event_log(
                            "SQYRE_EDIT_TIP",
                            &[
                                ("refit", "advanced"),
                                ("measured", &format!("{measured:.0}")),
                                ("prev", &format!("{prev_h:.0}")),
                                ("fit_min", &format!("{fit_min_h:.0}")),
                            ],
                        );
                    }
                    ui.ctx().request_repaint();
                } else if edit.auto_fit {
                    if measured > 0.0 && height_stable && fitting {
                        // Settle only after a frame that applied fit_min_h.
                        edit.auto_fit = false;
                        #[cfg(feature = "native-runtime")]
                        if sqyre_capture::disk_logging_enabled() {
                            sqyre_capture::event_log(
                                "SQYRE_EDIT_TIP",
                                &[
                                    ("fit", "settle"),
                                    ("measured", &format!("{measured:.0}")),
                                    ("fit_min", &format!("{fit_min_h:.0}")),
                                ],
                            );
                        }
                    } else {
                        ui.ctx().request_repaint();
                    }
                }
            }

            if ui.input(|i| i.key_pressed(Key::Enter))
                && !ui.input(|i| i.modifiers.shift)
            {
                // Don't steal Enter while a picker is open, a focused widget in
                // this window owns the key, or another window is on top (same
                // layering rule as Escape).
                if matches!(state, TooltipState::Edit(edit) if matches!(edit.picker, ActivePicker::None))
                    && crate::focus_nav::is_top_layer(ui.ctx(), ui.layer_id())
                    && !crate::focus_nav::focused_in_layer(ui.ctx(), ui.layer_id())
                {
                    save = true;
                }
            }
        });
    });

    if edit_window_user_resized(ctx, area_id) {
        if let TooltipState::Edit(edit) = state {
            edit.auto_fit = false;
        }
    }

    // Click outside the window cancels, same as the Close button and Escape.
    // The click that opened the editor is still in this frame's input, and the
    // window was not visible last frame, so that click does not dismiss it.
    // Clicks on a popup, menu, or picker (layers above the window) stay with
    // that surface. Record modals and an armed screen click own the pointer.
    let record_busy = bridges.key_record.is_open()
        || bridges.hotkey_record.is_open()
        || bridges.screen_click.is_armed();
    if !cancel && edit_tip_clicked_outside(ctx, area_id, record_busy) {
        cancel = true;
    }

    if !open || cancel {
        return state.cancel();
    }
    if save {
        // Snapshot for expression checks without conflicting with `&mut root`.
        let snap = Macro {
            name: macro_.name.clone(),
            root: macro_.root.clone(),
            global_delay: macro_.global_delay,
            keyboard_delay: macro_.keyboard_delay,
            mouse_delay: macro_.mouse_delay,
            hotkey: macro_.hotkey.clone(),
            hotkey_trigger: macro_.hotkey_trigger.clone(),
            tags: macro_.tags.clone(),
            variable_decls: macro_.variable_decls.clone(),
            variables: macro_.variables.clone(),
        };
        let _ = state.try_save_validated(&mut macro_.root, Some(&snap), before_mutate);
    }
    Vec::new()
}

pub(crate) fn apply_picker_result(draft: &mut Action, result: PickerResult) {
    match result {
        PickerResult::None => {}
        PickerResult::Items {
            targets,
            target_tags,
        } => {
            if let ActionKind::ImageSearch(d) = &mut draft.kind {
                d.targets = targets;
                if let Some(next_tags) = target_tags {
                    d.target_tags = next_tags;
                }
            }
        }
        PickerResult::Point(coord) => {
            if let ActionKind::Move { point, .. } = &mut draft.kind {
                *point = coord;
            }
        }
        PickerResult::SearchArea(coord) => match &mut draft.kind {
            ActionKind::ImageSearch(d) => d.search_area = coord,
            ActionKind::Ocr(d) => d.search_area = coord,
            ActionKind::FindPixel(d) => d.search_area = coord,
            ActionKind::ForEachCell { cells, .. } => *cells = coord,
            _ => {}
        },
        PickerResult::MacroName(name) => {
            if let ActionKind::RunMacro { macro_name } = &mut draft.kind {
                *macro_name = name;
            }
        }
        PickerResult::Window {
            process_path,
            window_title,
        } => {
            if let ActionKind::FocusWindow {
                process_path: path,
                window_title: title,
                ..
            } = &mut draft.kind
            {
                *path = process_path;
                *title = window_title;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::{root_loop, ActionKind, CoordinateRef, PressState, ScalarValue};

    fn wait_action(time: i64) -> Action {
        Action::new(ActionKind::Wait {
            time: ScalarValue::Int(time),
        })
    }

    #[test]
    fn middle_click_opens_edit_from_view() {
        let child = wait_action(100);
        let id = child.id;
        let mut state = TooltipState::Hidden;
        state.open_view(id);
        ingest_row(
            &mut state,
            &child,
            RowInteraction {
                middle_clicked: true,
                pointer_in_row: true,
                ..RowInteraction::default()
            },
            Some(egui::pos2(12.0, 8.0)),
        );
        match &state {
            TooltipState::Edit(edit) => {
                assert_eq!(edit.action_id, id);
                assert_eq!(edit.anchor, egui::pos2(12.0, 8.0));
            }
            other => panic!("expected Edit, got {other:?}"),
        }
    }

    #[test]
    fn view_to_edit_to_save_applies_draft() {
        let child = wait_action(100);
        let id = child.id;
        let mut root = root_loop(vec![child]);

        let mut state = TooltipState::Hidden;
        state.open_view(id);
        assert!(matches!(state, TooltipState::View { .. }));

        let live = root.find_by_id(id).unwrap().clone();
        state.open_edit(&live, egui::pos2(0.0, 0.0));
        if let TooltipState::Edit(edit) = &mut state {
            edit.draft.kind = ActionKind::Wait {
                time: ScalarValue::Int(250),
            };
        }
        assert!(state.try_save_validated(&mut root, None, |_| {}));
        assert!(matches!(state, TooltipState::View { action_id } if action_id == id));
        match &root.find_by_id(id).unwrap().kind {
            ActionKind::Wait { time } => assert_eq!(*time, ScalarValue::Int(250)),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn cancel_discards_draft() {
        let child = wait_action(100);
        let id = child.id;
        let root = root_loop(vec![child]);
        let live = root.find_by_id(id).unwrap().clone();
        let mut state = TooltipState::Hidden;
        state.open_edit(&live, egui::pos2(0.0, 0.0));
        if let TooltipState::Edit(edit) = &mut state {
            edit.draft.kind = ActionKind::Wait {
                time: ScalarValue::Int(999),
            };
        }
        assert_eq!(state.cancel(), Vec::<ActionId>::new());
        assert!(matches!(state, TooltipState::Hidden));
        match &root.find_by_id(id).unwrap().kind {
            ActionKind::Wait { time } => assert_eq!(*time, ScalarValue::Int(100)),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn cancel_provisional_new_returns_discard_id() {
        let child = wait_action(100);
        let id = child.id;
        let mut state = TooltipState::Hidden;
        state.open_edit_new(&child, egui::pos2(0.0, 0.0), Vec::new());
        assert_eq!(state.cancel(), vec![id]);
        assert!(matches!(state, TooltipState::Hidden));
    }

    #[test]
    fn validate_failure_keeps_edit() {
        let child = Action::new(ActionKind::Key {
            key: "a".into(),
            state: PressState::Down,
        });
        let id = child.id;
        let mut root = root_loop(vec![child]);
        let live = root.find_by_id(id).unwrap().clone();
        let mut state = TooltipState::Hidden;
        state.open_edit(&live, egui::pos2(0.0, 0.0));
        if let TooltipState::Edit(edit) = &mut state {
            edit.draft.kind = ActionKind::Key {
                key: String::new(),
                state: PressState::Down,
            };
        }
        assert!(!state.try_save_validated(&mut root, None, |_| {}));
        assert!(matches!(
            state,
            TooltipState::Edit(edit) if edit.error.is_some()
        ));
        match &root.find_by_id(id).unwrap().kind {
            ActionKind::Key { key, .. } => assert_eq!(key, "a"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn image_search_saves_without_targets() {
        let child = Action::new(ActionKind::ImageSearch(Box::new(
            sqyre_domain::ImageSearchData {
                targets: vec!["Game~Item".into()],
                search_area: Default::default(),
                tolerance: 0.95,
                blur: 5,
                ..Default::default()
            },
        )));
        let id = child.id;
        let mut root = root_loop(vec![child]);
        let live = root.find_by_id(id).unwrap().clone();
        let mut state = TooltipState::Hidden;
        state.open_edit(&live, egui::pos2(0.0, 0.0));
        if let TooltipState::Edit(edit) = &mut state {
            if let ActionKind::ImageSearch(d) = &mut edit.draft.kind {
                d.targets.clear();
            }
        }
        assert!(state.try_save_validated(&mut root, None, |_| {}));
        assert!(matches!(state, TooltipState::View { action_id } if action_id == id));
        match &root.find_by_id(id).unwrap().kind {
            ActionKind::ImageSearch(d) => assert!(d.targets.is_empty()),
            other => panic!("unexpected {other:?}"),
        }
        assert!(sqyre_validate::validate_action(root.find_by_id(id).unwrap(), None).is_err());
    }

    #[test]
    fn apply_recorded_key_updates_key_draft() {
        let child = Action::new(ActionKind::Key {
            key: String::new(),
            state: PressState::Down,
        });
        let mut state = TooltipState::Hidden;
        state.open_edit(&child, egui::pos2(0.0, 0.0));
        state.apply_recorded_key("f5".into());
        match &state {
            TooltipState::Edit(edit) => match &edit.draft.kind {
                ActionKind::Key { key, .. } => assert_eq!(key, "f5"),
                other => panic!("unexpected {other:?}"),
            },
            other => panic!("expected Edit, got {other:?}"),
        }
    }

    #[test]
    #[cfg(feature = "native-runtime")]
    fn apply_recorded_color_updates_find_pixel_draft() {
        let child = Action::new(ActionKind::FindPixel(Box::new(
            sqyre_domain::FindPixelData {
                target_color: "ffffff".into(),
                ..Default::default()
            },
        )));
        let mut state = TooltipState::Hidden;
        state.open_edit(&child, egui::pos2(0.0, 0.0));
        state.apply_recorded_color("ab12cd".into());
        match &state {
            TooltipState::Edit(edit) => match &edit.draft.kind {
                ActionKind::FindPixel(d) => {
                    assert_eq!(d.target_color, "ab12cd")
                }
                other => panic!("unexpected {other:?}"),
            },
            other => panic!("expected Edit, got {other:?}"),
        }
    }

    #[test]
    fn apply_preserves_subactions() {
        let inner_id = ActionId::new();
        let branch_id = ActionId::new();
        let mut wait = wait_action(1);
        wait.id = inner_id;
        let mut root = root_loop(vec![Action {
            id: branch_id,
            kind: ActionKind::Loop {
                name: "outer".into(),
                count: ScalarValue::Int(2),
                subactions: vec![wait],
            },
        }]);

        let mut draft = root.find_by_id(branch_id).unwrap().clone();
        draft.kind = ActionKind::Loop {
            name: "renamed".into(),
            count: ScalarValue::Int(5),
            subactions: vec![],
        };
        let live = root.find_by_id_mut(branch_id).unwrap();
        apply_draft_preserving_children(live, draft).unwrap();
        match &root.find_by_id(branch_id).unwrap().kind {
            ActionKind::Loop {
                name,
                count,
                subactions,
            } => {
                assert_eq!(name, "renamed");
                assert_eq!(*count, ScalarValue::Int(5));
                assert_eq!(subactions.len(), 1);
                assert_eq!(subactions[0].id, inner_id);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn image_search_targets_round_trip_via_apply() {
        let id = ActionId::new();
        let mut root = root_loop(vec![Action {
            id,
            kind: ActionKind::ImageSearch(Box::new(sqyre_domain::ImageSearchData {
                name: "find".into(),
                targets: vec!["P~A".into()],
                search_area: CoordinateRef("P~Box".into()),
                tolerance: 0.9,
                ..Default::default()
            })),
        }]);
        let mut draft = root.find_by_id(id).unwrap().clone();
        if let ActionKind::ImageSearch(d) = &mut draft.kind {
            d.targets = vec!["P~B".into(), "P~C".into()];
        }
        apply_draft_preserving_children(root.find_by_id_mut(id).unwrap(), draft).unwrap();
        match &root.find_by_id(id).unwrap().kind {
            ActionKind::ImageSearch(d) => {
                assert_eq!(
                    d.targets.as_slice(),
                    ["P~B".to_string(), "P~C".to_string()].as_slice()
                );
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn escape_dismisses_view_and_edit() {
        let mut state = TooltipState::View {
            action_id: ActionId::new(),
        };
        assert_eq!(state.handle_escape(), (true, Vec::new()));
        assert!(matches!(state, TooltipState::Hidden));

        let draft = wait_action(1);
        state.open_edit(&draft, egui::pos2(0.0, 0.0));
        assert_eq!(state.handle_escape(), (true, Vec::new()));
        assert!(matches!(state, TooltipState::Hidden));

        let draft = wait_action(2);
        let id = draft.id;
        state.open_edit_new(&draft, egui::pos2(0.0, 0.0), Vec::new());
        assert_eq!(state.handle_escape(), (true, vec![id]));
        assert!(matches!(state, TooltipState::Hidden));
    }

    #[test]
    fn tip_max_width_scales_with_viewport() {
        let floor = crate::widgets::FLOATER_MIN_COMPACT[0];
        // Narrow viewport: still at least the floater floor.
        assert_eq!(tip_max_width(300.0), floor);
        // Otherwise half the constrain width.
        assert!((tip_max_width(1200.0) - 600.0).abs() < 0.5);
        assert!((tip_max_width(4000.0) - 2000.0).abs() < 0.5);
    }

    /// The fields scroll's floating V-bar must sit on the window's right edge.
    /// Anything narrower (content edge, un-expanded bound) leaves a dead strip.
    #[test]
    fn edit_tip_scrollbar_stays_at_window_edge() {
        let ctx = egui::Context::default();
        crate::theme::apply(&ctx);

        let mut macro_ = crate::docs_fixture::demo_macro();
        let image_search = macro_
            .root
            .children()
            .iter()
            .find(|a| matches!(a.kind, ActionKind::ImageSearch(_)))
            .cloned()
            .expect("demo macro has an image search action");
        let mut state = TooltipState::Hidden;
        state.open_edit(&image_search, egui::pos2(80.0, 60.0));

        let catalog = sqyre_persist::ProgramCatalog::default();
        let mut icons = crate::icon_cache::IconCache::new();
        let mut previews = crate::preview_tooltip::PreviewTooltipCache::new();
        let mut key_record = crate::key_record::KeyRecordUi::default();
        let mut hotkey_record = crate::hotkey_record::HotkeyRecordUi::default();
        let macro_hotkeys = sqyre_hotkeys::MacroHotkeyBridge::new();
        let screen_click = sqyre_hotkeys::ScreenClickBridge::new();
        let known_vars = sqyre_domain::collect_known_variable_names(&macro_);
        let macros = vec![(macro_.name.clone(), macro_.tags.clone())];

        let mut gap = f32::NAN;
        let mut bar_seen = false;
        for frame in 0..12 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::pos2(0.0, 0.0),
                    egui::vec2(640.0, 360.0),
                )),
                time: Some(frame as f64 * 0.1),
                predicted_dt: 0.1,
                ..Default::default()
            };
            let out = ctx.run_ui(input, |ui| {
                let mut tip = TipUiCtx {
                    paint: CatalogPaint {
                        catalog: &catalog,
                        icons: &mut icons,
                        previews: &mut previews,
                    },
                    theme: VarTheme {
                        known_vars: &known_vars,
                        is_dark: true,
                    },
                    bridges: RecordBridges {
                        key_record: &mut key_record,
                        hotkey_record: &mut hotkey_record,
                        macro_hotkeys: &macro_hotkeys,
                        screen_click: &screen_click,
                    },
                    compact_program_headers: false,
                    pending_scale: None,
                };
                let _ = show(&mut state, ui.ctx(), &mut macro_, &macros, &mut tip, |_| {});
            });
            let win = ctx.memory(|m| m.area_rect(edit_tip_area_id(image_search.id)));
            // The floating V-bar (background + handle) paints as tall thin
            // rects; the theme leaves their fills transparent, so match on
            // shape alone and take the rightmost one inside the window.
            if let Some(win) = win {
                let bar_right = out
                    .shapes
                    .iter()
                    .filter_map(|c| match &c.shape {
                        egui::Shape::Rect(r)
                            if r.rect.width() <= 12.0
                                && r.rect.height() > 120.0
                                && r.rect.min.x >= win.left() =>
                        {
                            Some(r.rect.max.x)
                        }
                        _ => None,
                    })
                    .fold(f32::NEG_INFINITY, f32::max);
                if bar_right.is_finite() {
                    bar_seen = true;
                    gap = win.right() - bar_right;
                }
            }
            out.drop_without_applying_deltas();
        }
        assert!(bar_seen, "fields scroll never showed a vertical bar");
        assert!(
            gap <= 20.0,
            "scrollbar floats {gap:.1}pt left of the window edge"
        );
    }

    #[test]
    fn move_field_apply() {
        let id = ActionId::new();
        let mut root = root_loop(vec![Action {
            id,
            kind: ActionKind::Move {
                point: CoordinateRef("P~A".into()),
                smooth: false,
                smooth_low: 0.05,
                smooth_high: 0.2,
                smooth_delay_ms: 1,
            },
        }]);
        let mut draft = root.find_by_id(id).unwrap().clone();
        if let ActionKind::Move { point, smooth, .. } = &mut draft.kind {
            point.0 = "P~B".into();
            *smooth = true;
        }
        apply_draft_preserving_children(root.find_by_id_mut(id).unwrap(), draft).unwrap();
        match &root.find_by_id(id).unwrap().kind {
            ActionKind::Move { point, smooth, .. } => {
                assert_eq!(point.0, "P~B");
                assert!(*smooth);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    /// Hosts one edit tooltip across frames so pointer events can dismiss it.
    struct EditTipHost {
        ctx: egui::Context,
        state: TooltipState,
        macro_: sqyre_domain::Macro,
        catalog: sqyre_persist::ProgramCatalog,
        icons: crate::icon_cache::IconCache,
        previews: crate::preview_tooltip::PreviewTooltipCache,
        key_record: crate::key_record::KeyRecordUi,
        hotkey_record: crate::hotkey_record::HotkeyRecordUi,
        macro_hotkeys: sqyre_hotkeys::MacroHotkeyBridge,
        screen_click: sqyre_hotkeys::ScreenClickBridge,
        known_vars: sqyre_domain::KnownVariableNames,
        macros: Vec<(String, Vec<String>)>,
        time: f64,
        discarded: Vec<ActionId>,
        /// Foreground shield at the origin, covering clicks that must not dismiss.
        shield: bool,
    }

    impl EditTipHost {
        fn open(action: &Action, anchor: egui::Pos2, provisional: bool) -> Self {
            let macro_ = sqyre_domain::Macro {
                name: "m".into(),
                root: root_loop(vec![action.clone()]),
                global_delay: 0,
                keyboard_delay: 25,
                mouse_delay: 25,
                hotkey: Vec::new(),
                hotkey_trigger: String::new(),
                tags: Vec::new(),
                variable_decls: Vec::new(),
                variables: Default::default(),
            };
            let known_vars = sqyre_domain::collect_known_variable_names(&macro_);
            let macros = vec![(macro_.name.clone(), macro_.tags.clone())];
            let mut state = TooltipState::Hidden;
            if provisional {
                state.open_edit_new(action, anchor, Vec::new());
            } else {
                state.open_edit(action, anchor);
            }
            let ctx = egui::Context::default();
            crate::theme::apply(&ctx);
            Self {
                ctx,
                state,
                macro_,
                catalog: sqyre_persist::ProgramCatalog::default(),
                icons: crate::icon_cache::IconCache::new(),
                previews: crate::preview_tooltip::PreviewTooltipCache::new(),
                key_record: crate::key_record::KeyRecordUi::default(),
                hotkey_record: crate::hotkey_record::HotkeyRecordUi::default(),
                macro_hotkeys: sqyre_hotkeys::MacroHotkeyBridge::new(),
                screen_click: sqyre_hotkeys::ScreenClickBridge::new(),
                known_vars,
                macros,
                time: 0.0,
                discarded: Vec::new(),
                shield: false,
            }
        }

        fn frame(&mut self, events: Vec<egui::Event>) {
            self.time += 0.1;
            let ctx = self.ctx.clone();
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900.0, 700.0),
                )),
                time: Some(self.time),
                predicted_dt: 0.1,
                events,
                ..Default::default()
            };
            let shield = self.shield;
            let out = ctx.run_ui(input, |ui| {
                if shield {
                    egui::Area::new(egui::Id::new("edit_tip_test_shield"))
                        .order(egui::Order::Foreground)
                        .fixed_pos(egui::Pos2::ZERO)
                        .interactable(true)
                        .show(ui.ctx(), |ui| {
                            ui.allocate_exact_size(egui::vec2(64.0, 64.0), egui::Sense::click());
                        });
                }
                let mut tip = TipUiCtx {
                    paint: CatalogPaint {
                        catalog: &self.catalog,
                        icons: &mut self.icons,
                        previews: &mut self.previews,
                    },
                    theme: VarTheme {
                        known_vars: &self.known_vars,
                        is_dark: true,
                    },
                    bridges: RecordBridges {
                        key_record: &mut self.key_record,
                        hotkey_record: &mut self.hotkey_record,
                        macro_hotkeys: &self.macro_hotkeys,
                        screen_click: &self.screen_click,
                    },
                    compact_program_headers: false,
                    pending_scale: None,
                };
                self.discarded = show(
                    &mut self.state,
                    ui.ctx(),
                    &mut self.macro_,
                    &self.macros,
                    &mut tip,
                    |_| {},
                );
            });
            out.drop_without_applying_deltas();
        }

        fn pointer(pos: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        }

        /// Press and release in one frame — the shape of the click that opens the editor.
        fn click_same_frame(&mut self, pos: egui::Pos2) {
            let mut events = Self::pointer(pos, true);
            events.extend(Self::pointer(pos, false));
            self.frame(events);
        }

        fn click(&mut self, pos: egui::Pos2) {
            self.frame(Self::pointer(pos, true));
            self.frame(Self::pointer(pos, false));
        }

        fn editing(&self) -> bool {
            self.state.is_editing()
        }

        fn window_rect(&self) -> egui::Rect {
            let id = self.state.action_id().expect("edit open");
            self.ctx
                .memory(|m| m.area_rect(edit_tip_area_id(id)))
                .expect("edit window rect")
        }

        fn close_button_center(&self) -> egui::Pos2 {
            let id = self.state.action_id().expect("edit open");
            let layer = egui::LayerId::new(egui::Order::Middle, edit_tip_area_id(id));
            let win = self.window_rect();
            // Header band: the Close icon is the rightmost roughly-square button.
            // Resize grips are thin strips and sit outside this filter.
            let header_bottom = win.top() + 64.0;
            self.ctx
                .viewport(|v| {
                    v.prev_pass
                        .widgets
                        .get_layer(layer)
                        .filter(|w| {
                            let size = w.rect.size();
                            w.sense.senses_click()
                                && w.enabled
                                && w.rect.center().y <= header_bottom
                                && size.x >= 12.0
                                && size.y >= 12.0
                                && (size.x - size.y).abs() < 16.0
                        })
                        .max_by(|a, b| a.rect.right().total_cmp(&b.rect.right()))
                        .map(|w| w.rect.center())
                })
                .expect("close button in the edit header")
        }
    }

    #[test]
    fn edit_tip_close_button_discards_provisional_edit() {
        let action = wait_action(40);
        let id = action.id;
        let mut host = EditTipHost::open(&action, egui::pos2(280.0, 160.0), true);
        host.frame(Vec::new());
        host.frame(Vec::new());
        let at = host.close_button_center();
        assert!(
            host.window_rect().contains(at),
            "close button sits inside the edit window"
        );
        host.click(at);
        assert!(!host.editing(), "Close dismisses the editor");
        assert_eq!(host.discarded, vec![id]);
    }

    #[test]
    fn edit_tip_click_outside_closes_after_it_is_open() {
        let action = wait_action(40);
        let id = action.id;
        let outside = egui::pos2(8.0, 8.0);
        let mut host = EditTipHost::open(&action, egui::pos2(280.0, 160.0), true);

        // The opening click arrives on the first frame the window is shown.
        host.click_same_frame(outside);
        assert!(
            host.editing(),
            "the click that opens the editor must not close it"
        );

        host.frame(Vec::new());
        let win = host.window_rect();
        assert!(
            !win.contains(outside),
            "test click {outside:?} landed inside the window {win:?}"
        );
        // Same press+release shape as the opening click, now that the window
        // was on screen last frame.
        host.click_same_frame(outside);
        assert!(!host.editing(), "a later click outside closes the editor");
        assert_eq!(host.discarded, vec![id]);

        host.state.open_edit(&action, egui::pos2(280.0, 160.0));
        host.frame(Vec::new());
        host.frame(Vec::new());
        let inside = host.window_rect().center();
        host.click(inside);
        assert!(host.editing(), "a click inside the editor leaves it open");
    }

    #[test]
    fn edit_tip_click_on_foreground_popup_stays_open() {
        let action = wait_action(40);
        let mut host = EditTipHost::open(&action, egui::pos2(400.0, 240.0), false);
        host.shield = true;
        host.frame(Vec::new());
        host.frame(Vec::new());
        let on_popup = egui::pos2(16.0, 16.0);
        let win = host.window_rect();
        assert!(
            !win.contains(on_popup),
            "popup click {on_popup:?} should be outside the editor {win:?}"
        );
        host.click(on_popup);
        assert!(
            host.editing(),
            "a click on a popup above the editor does not close it"
        );

        let outside = egui::pos2(120.0, 16.0);
        assert!(!host.window_rect().contains(outside));
        host.click(outside);
        assert!(
            !host.editing(),
            "a click outside both surfaces still closes"
        );
    }

    #[test]
    fn edit_tip_click_outside_ignored_while_recording_a_key() {
        let action = wait_action(40);
        let mut host = EditTipHost::open(&action, egui::pos2(280.0, 160.0), false);
        let hotkeys = sqyre_hotkeys::MacroHotkeyBridge::new();
        host.key_record.open(&hotkeys);
        host.frame(Vec::new());
        host.frame(Vec::new());
        host.click(egui::pos2(8.0, 8.0));
        assert!(
            host.editing(),
            "an outside click while a key is being recorded leaves the editor open"
        );
    }
}
