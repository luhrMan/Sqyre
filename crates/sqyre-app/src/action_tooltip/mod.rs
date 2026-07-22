//! In-tree action tooltip: view on hover, pinned edit with Save/Cancel.

mod edit;
mod edit_header;
pub(crate) mod help;
mod sections;

use crate::pickers::{self, ActivePicker, PickerResult};
use crate::tree_chrome::{self, RowInteraction};
use eframe::egui::{self, Key, Order, Vec2};
use sqyre_domain::{
    action_type_description, action_type_label, Action, ActionId, ActionKind, Macro, PressState,
};
use sqyre_ui_model::{action_pastel_color, split_display_params, ActionDisplay};
use sqyre_validate::validate_action;

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
    pub error: Option<String>,
    /// Screen position when edit opened (pinned, not mouse-follow).
    pub anchor: egui::Pos2,
    pub picker: ActivePicker,
    /// When true, Cancel / Escape / close removes this action from the tree
    /// (used for freshly inserted blank actions that were never saved).
    pub discard_on_cancel: bool,
    /// Unclipped height of the fields column from the last frame.
    fields_height: f32,
    /// While true, raise window height toward content (capped at screen max).
    /// Cleared after the initial fit settles, or when the user resizes.
    auto_fit: bool,
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
            error: None,
            anchor,
            picker: ActivePicker::None,
            discard_on_cancel: false,
            fields_height: 0.0,
            auto_fit: true,
        }));
    }

    /// Edit a freshly inserted blank action; Cancel removes it from the tree.
    pub fn open_edit_new(&mut self, action: &Action, anchor: egui::Pos2) {
        *self = Self::Edit(Box::new(TooltipEdit {
            action_id: action.id,
            draft: action.clone(),
            error: None,
            anchor,
            picker: ActivePicker::None,
            discard_on_cancel: true,
            fields_height: 0.0,
            auto_fit: true,
        }));
    }

    /// Close the tooltip. Returns an action id that should be removed from the tree
    /// when a provisional new-action edit was cancelled.
    pub fn cancel(&mut self) -> Option<ActionId> {
        let discard = match self {
            Self::Edit(edit) if edit.discard_on_cancel => Some(edit.action_id),
            _ => None,
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
        let preserved = live.children().to_vec();
        if let Some(kids) = candidate.children_mut() {
            *kids = preserved;
        }
        candidate.id = live.id;

        if let Err(e) = validate_action(&candidate, macro_) {
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
    pub fn apply_recorded_color(&mut self, recorded: String) {
        let Self::Edit(edit) = self else {
            return;
        };
        crate::recorded_action::apply_recorded_color(&mut edit.draft.kind, recorded);
    }

    /// Escape handling. Returns `(consumed, discard_id)` — discard_id is set when a
    /// provisional new-action edit is fully cancelled.
    pub fn handle_escape(&mut self) -> (bool, Option<ActionId>) {
        match self {
            Self::Hidden => (false, None),
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
                (true, None)
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
                (true, None)
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
    if interaction.action != tree_chrome::RowAction::None {
        return;
    }

    if interaction.secondary_clicked || interaction.double_clicked {
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
/// Returns an action id that should be removed when a provisional new-action
/// edit was cancelled without saving.
pub fn show(
    state: &mut TooltipState,
    ctx: &egui::Context,
    macro_: &mut Macro,
    macros: &[(String, Vec<String>)],
    ui: &mut TipUiCtx<'_>,
    mut before_mutate: impl FnMut(&Action),
) -> Option<ActionId> {
    let TipUiCtx {
        paint,
        theme,
        bridges,
    } = ui;
    // Esc while recording a key / chord / screen sample is captured by the recorder.
    if !bridges.key_record.is_open()
        && !bridges.hotkey_record.is_open()
        && !bridges.screen_click.is_armed()
        && ctx.input(|i| i.key_pressed(Key::Escape))
    {
        let (consumed, discard) = state.handle_escape();
        if consumed {
            return discard;
        }
    }

    match state.clone() {
        TooltipState::Hidden => None,
        TooltipState::View { action_id } => {
            let Some(action) = macro_.root.find_by_id(action_id).cloned() else {
                *state = TooltipState::Hidden;
                return None;
            };
            show_view_tip(ctx, &action, paint, *theme);
            None
        }
        TooltipState::Edit { .. } => {
            show_edit_window(state, ctx, macro_, macros, ui, &mut before_mutate)
        }
    }
}

/// Shared width budget for view and edit tips so mode switches don't jump size.
fn tip_max_width(has_coord_preview: bool) -> f32 {
    if has_coord_preview {
        280.0
    } else {
        340.0
    }
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
    let screen = ctx.content_rect();
    let max_w = tip_max_width(coord_preview.is_some());
    let near_right = pointer.x + 14.0 + max_w > screen.right();
    let (pivot, pos) = if near_right {
        (egui::Align2::RIGHT_TOP, pointer + Vec2::new(-8.0, 14.0))
    } else {
        (egui::Align2::LEFT_TOP, pointer + Vec2::new(14.0, 14.0))
    };
    // interactable(false): clicks pass through to the control underneath.
    egui::Area::new(tip_id)
        .order(Order::Tooltip)
        .pivot(pivot)
        .fixed_pos(pos)
        .interactable(false)
        .sense(egui::Sense::hover())
        .constrain(true)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .inner_margin(egui::Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    ui.set_max_width(max_w);
                    tree_chrome::paint_pill_pub(ui, label, pastel);
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(description).size(12.0).weak());
                    if !summary_pills.is_empty() {
                        ui.add_space(4.0);
                        sections::tip_wrapped_section(ui, |ui| {
                            ui.spacing_mut().item_spacing = Vec2::splat(3.0);
                            for pill in &summary_pills {
                                let _ = var_pills::paint_summary_pill(
                                    ui, type_key, pill, known_vars, is_dark,
                                );
                            }
                        });
                    }
                    if let Some((coord_ref, kind)) = coord_preview {
                        sections::tip_section(ui, |ui| {
                            previews.paint_for_coordinate_ref(ui, catalog, &coord_ref, kind, false);
                        });
                    }
                    if let ActionKind::ImageSearch { targets, .. } = &action.kind {
                        if !targets.is_empty() {
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
                                            .size(12.0)
                                            .strong(),
                                    );
                                    var_pills::paint_var_ref_content(
                                        ui,
                                        p.minimal(),
                                        known_vars,
                                        is_dark,
                                        ui.visuals().text_color(),
                                    );
                                });
                            }
                        });
                    }
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
) -> Option<ActionId> {
    let TipUiCtx {
        paint,
        theme,
        bridges,
    } = ui;
    let VarTheme { is_dark, .. } = *theme;
    let (action_id, anchor, type_key, has_coord_preview) = match state {
        TooltipState::Edit(edit) => (
            edit.action_id,
            edit.anchor,
            edit.draft.type_key(),
            crate::preview_tooltip::coordinate_ref_for_preview(&edit.draft).is_some(),
        ),
        _ => return None,
    };

    let label = action_type_label(type_key);
    let pastel = tree_chrome::rgba_pub(action_pastel_color(type_key, is_dark));
    let max_w = tip_max_width(has_coord_preview);
    let screen = ctx.content_rect();
    // Grow with content from a modest default; ScrollArea caps at the window height.
    const EDIT_CHROME: f32 = 72.0; // type pill + Save/Cancel + separator + margins
    let max_scroll_h = (screen.height() - EDIT_CHROME).max(100.0);

    // Picker modal first (foreground); apply result onto draft.
    if let TooltipState::Edit(edit) = state {
        let result = pickers::show_active_picker(ctx, &mut edit.picker, paint, macros);
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
    let area_id = egui::Id::new(("action_edit_tip", "grow_v3", action_id));

    // Popup chrome (no title bar). On open, auto-fit height to content up to
    // the screen max; afterward the user may freely shrink/grow (ScrollArea).
    egui::Window::new(label)
        .id(area_id)
        .open(&mut open)
        .title_bar(false)
        .collapsible(false)
        .resizable(true)
        .constrain(true)
        .default_pos(anchor + Vec2::new(12.0, 12.0))
        .default_size([max_w, 120.0])
        .min_size([220.0, 100.0])
        .max_size(screen.size())
        .frame(
            egui::Frame::popup(ctx.global_style().as_ref())
                .inner_margin(egui::Margin::symmetric(10, 8)),
        )
        .show(ctx, |ui| {
            let err = match state {
                TooltipState::Edit(edit) => edit.error.as_deref(),
                _ => None,
            };
            match paint_action_edit_header(ui, label, pastel, None, err) {
                SaveCancel::Cancel => cancel = true,
                SaveCancel::Save => save = true,
                SaveCancel::None => {}
            }
            // Always scroll so a user-shrunk window can still reach all fields.
            // While `auto_fit`, raise min height toward measured content (capped).
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
                let out = egui::ScrollArea::vertical()
                    .id_salt("edit_fields")
                    .auto_shrink([false, false])
                    .max_height(max_scroll_h)
                    .show(ui, |ui| {
                        edit::paint_edit_fields(
                            ui,
                            &mut edit.draft,
                            &mut edit.picker,
                            &mut fields,
                        );
                    });
                let measured = out.content_size.y;
                if (measured - edit.fields_height).abs() > 1.0 {
                    ui.ctx().request_repaint();
                }
                edit.fields_height = measured;

                if edit.auto_fit {
                    let want = measured.min(max_scroll_h);
                    if want > 1.0 {
                        // Expand the resizable window toward content for the
                        // next frame; does not block later user shrink once
                        // `auto_fit` clears.
                        ui.set_min_height(want);
                    }
                    if measured > 0.0 && out.inner_rect.height() >= want - 1.0 {
                        edit.auto_fit = false;
                    }
                }
            }

            if ui.input(|i| i.key_pressed(Key::Enter))
                && !ui.input(|i| i.modifiers.shift)
            {
                // Don't steal Enter while a picker is open.
                if matches!(state, TooltipState::Edit(edit) if matches!(edit.picker, ActivePicker::None))
                    && !ui.ctx().egui_wants_keyboard_input()
                {
                    save = true;
                }
            }
        });

    if edit_window_user_resized(ctx, area_id) {
        if let TooltipState::Edit(edit) = state {
            edit.auto_fit = false;
        }
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
    None
}

pub(crate) fn apply_picker_result(draft: &mut Action, result: PickerResult) {
    match result {
        PickerResult::None => {}
        PickerResult::Items(targets) => {
            if let ActionKind::ImageSearch { targets: t, .. } = &mut draft.kind {
                *t = targets;
            }
        }
        PickerResult::Point(coord) => {
            if let ActionKind::Move { point, .. } = &mut draft.kind {
                *point = coord;
            }
        }
        PickerResult::SearchArea(coord) => match &mut draft.kind {
            ActionKind::ImageSearch { search_area, .. }
            | ActionKind::Ocr { search_area, .. }
            | ActionKind::FindPixel { search_area, .. } => *search_area = coord,
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
    use sqyre_domain::{root_loop, ActionKind, CoordinateRef, DetectionBranch, ScalarValue};

    fn wait_action(time: i64) -> Action {
        Action {
            id: ActionId::new(),
            kind: ActionKind::Wait {
                time: ScalarValue::Int(time),
            },
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
        assert_eq!(state.cancel(), None);
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
        state.open_edit_new(&child, egui::pos2(0.0, 0.0));
        assert_eq!(state.cancel(), Some(id));
        assert!(matches!(state, TooltipState::Hidden));
    }

    #[test]
    fn validate_failure_keeps_edit() {
        let child = Action {
            id: ActionId::new(),
            kind: ActionKind::Key {
                key: "a".into(),
                state: PressState::Down,
            },
        };
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
    fn apply_recorded_key_updates_key_draft() {
        let child = Action {
            id: ActionId::new(),
            kind: ActionKind::Key {
                key: String::new(),
                state: PressState::Down,
            },
        };
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
    fn apply_recorded_color_updates_find_pixel_draft() {
        let child = Action {
            id: ActionId::new(),
            kind: ActionKind::FindPixel {
                name: String::new(),
                search_area: CoordinateRef::default(),
                target_color: "ffffff".into(),
                color_tolerance: 0,
                detection: DetectionBranch::default(),
            },
        };
        let mut state = TooltipState::Hidden;
        state.open_edit(&child, egui::pos2(0.0, 0.0));
        state.apply_recorded_color("ab12cd".into());
        match &state {
            TooltipState::Edit(edit) => match &edit.draft.kind {
                ActionKind::FindPixel { target_color, .. } => {
                    assert_eq!(target_color, "ab12cd")
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
            kind: ActionKind::ImageSearch {
                name: "find".into(),
                targets: vec!["P~A".into()],
                search_area: CoordinateRef("P~Box".into()),
                tolerance: 0.9,
                blur: 0,
                detection: DetectionBranch::default(),
            },
        }]);
        let mut draft = root.find_by_id(id).unwrap().clone();
        if let ActionKind::ImageSearch { targets, .. } = &mut draft.kind {
            *targets = vec!["P~B".into(), "P~C".into()];
        }
        apply_draft_preserving_children(root.find_by_id_mut(id).unwrap(), draft).unwrap();
        match &root.find_by_id(id).unwrap().kind {
            ActionKind::ImageSearch { targets, .. } => {
                assert_eq!(
                    targets.as_slice(),
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
        assert_eq!(state.handle_escape(), (true, None));
        assert!(matches!(state, TooltipState::Hidden));

        let draft = wait_action(1);
        state.open_edit(&draft, egui::pos2(0.0, 0.0));
        assert_eq!(state.handle_escape(), (true, None));
        assert!(matches!(state, TooltipState::Hidden));

        let draft = wait_action(2);
        let id = draft.id;
        state.open_edit_new(&draft, egui::pos2(0.0, 0.0));
        assert_eq!(state.handle_escape(), (true, Some(id)));
        assert!(matches!(state, TooltipState::Hidden));
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
}
