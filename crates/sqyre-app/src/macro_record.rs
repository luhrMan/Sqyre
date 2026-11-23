//! Macro recording: capture moves/clicks/keys/waits until Esc, then review/copy.

use crate::action_tooltip::{self, TooltipState};
use crate::help_text as ht;
use crate::hotkey_record::HotkeyRecordUi;
use crate::icon_cache::IconCache;
use crate::key_record::KeyRecordUi;
use crate::paint_ctx::{CatalogPaint, RecordBridges, TipUiCtx, VarTheme};
use crate::preview_tooltip::PreviewTooltipCache;
use crate::status_banner::StatusBanner;
use crate::tree_chrome::{self, RowAction, RowHighlight, RowInteraction};
use eframe::egui;
use sqyre_domain::{
    collect_known_variable_names, root_loop, Action, ActionId, ActionKind, CoordinateRef, Macro,
    MouseButton, PressState, ScalarValue, GENERAL_PROGRAM, PROGRAM_DELIMITER, TEMPORARY_PROGRAM,
};
use sqyre_hotkeys::{
    MacroHotkeyBridge, MacroRecordBridge, MacroRecordEvent, RecordMouseButton, ScreenClickBridge,
};
use sqyre_persist::{ProgramCatalog, ProgramPoint};
use sqyre_ui_model::SummaryPill;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// Recording thresholds from Settings → Recording.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RecordThresholds {
    /// Minimum gap (ms) before inserting a Wait between recorded actions.
    pub wait_ms: i64,
    /// Collapse mouse-move samples closer than this (pixels).
    pub move_min_px: i32,
    /// Collapse Tap when press→release is at most this (ms).
    pub tap_max_ms: i64,
    /// Max Chebyshev distance (px) to surface a nearby saved-point recommendation.
    pub snap_point_px: i32,
}

impl RecordThresholds {
    pub(crate) fn from_settings(s: &sqyre_persist::UserSettings) -> Self {
        Self {
            wait_ms: s.record_wait_threshold_ms,
            move_min_px: s.record_move_min_distance,
            tap_max_ms: s.record_tap_max_ms,
            snap_point_px: s.record_snap_point_px,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TempPoint {
    /// Name used when the Move action was created (for rename → ref rewrite).
    pub original_name: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    /// When true, Copy / Save points will upsert into the catalog and draw on screen.
    pub save: bool,
    /// When set, Move actions link to this existing catalog point (`Program~Name`)
    /// instead of `{TEMPORARY_PROGRAM}~{name}`.
    pub link_to: Option<String>,
}

/// Horizontal offset aligning the "Nearby" hint under the point fields above it.
const SUB_ROW_INDENT: f32 = 24.0;

/// Result of Copy from the review window.
pub(crate) struct MacroRecordCopy {
    pub maps: Vec<serde_yaml::Mapping>,
    pub yaml: String,
}

/// Outcome of one [`MacroRecordUi::show`] frame.
pub(crate) struct MacroRecordShowResult {
    pub copy: Option<MacroRecordCopy>,
    /// Catalog was mutated (temporary program wipe / point upsert) — persist `db.yaml`.
    pub catalog_changed: bool,
}

/// Borrowed UI deps for the recorded-actions review popup.
pub(crate) struct MacroRecordShow<'a> {
    pub ctx: &'a egui::Context,
    pub macro_hotkeys: &'a MacroHotkeyBridge,
    pub bridge: &'a MacroRecordBridge,
    pub catalog: &'a mut ProgramCatalog,
    pub icons: &'a mut IconCache,
    pub previews: &'a mut PreviewTooltipCache,
    pub key_record: &'a mut KeyRecordUi,
    pub hotkey_record: &'a mut HotkeyRecordUi,
    pub screen_click: &'a ScreenClickBridge,
    pub macros: &'a [(String, Vec<String>)],
    pub compact_program_headers: bool,
    pub pending_scale: Option<&'a crate::widgets::ViewportScaleEvent>,
    pub thresholds: RecordThresholds,
}

#[derive(Debug, Clone)]
pub(crate) struct ReviewState {
    /// Synthetic macro whose root children are the recorded actions.
    draft: Macro,
    points: Vec<TempPoint>,
    status: StatusBanner,
    tooltip: TooltipState,
    pills_cache: HashMap<ActionId, (u64, Vec<SummaryPill>)>,
    paint_revision: u64,
}

#[derive(Debug, Clone, Default)]
pub(crate) enum MacroRecordUi {
    #[default]
    Closed,
    /// Capturing input via [`MacroRecordBridge`].
    Recording,
    /// Esc finished capture; user can edit / save points / copy.
    Review(Box<ReviewState>),
}

impl MacroRecordUi {
    pub fn open(
        &mut self,
        macro_hotkeys: &MacroHotkeyBridge,
        bridge: &MacroRecordBridge,
        catalog: &mut ProgramCatalog,
    ) -> bool {
        if !matches!(self, Self::Closed) {
            return false;
        }
        macro_hotkeys.suspend();
        bridge.arm();
        *self = Self::Recording;
        // Each new recording overwrites the scratch `temporary` program.
        reset_temporary_program(catalog).is_ok()
    }

    pub fn is_open(&self) -> bool {
        !matches!(self, Self::Closed)
    }

    /// Draw recording chrome / review popup.
    ///
    /// On Copy / Save points: replaces [`TEMPORARY_PROGRAM`] points with the checked set.
    pub fn show(&mut self, ui: MacroRecordShow<'_>) -> MacroRecordShowResult {
        let MacroRecordShow {
            ctx,
            macro_hotkeys,
            bridge,
            catalog,
            icons,
            previews,
            key_record,
            hotkey_record,
            screen_click,
            macros,
            compact_program_headers,
            pending_scale,
            thresholds,
        } = ui;
        match self {
            Self::Closed => MacroRecordShowResult {
                copy: None,
                catalog_changed: false,
            },
            Self::Recording => {
                if bridge.take_cancelled() {
                    macro_hotkeys.resume();
                    *self = Self::Closed;
                    return MacroRecordShowResult {
                        copy: None,
                        catalog_changed: false,
                    };
                }
                // Pull keys from the hotkey bridge each frame (same source as Key
                // Record). Avoids missing presses when the HUD briefly steals focus
                // or OS hooks suppress LL keyboard until another window is clicked.
                {
                    let pressed = macro_hotkeys.pressed_keys();
                    let set: std::collections::HashSet<&str> =
                        pressed.iter().map(String::as_str).collect();
                    bridge.sync_pressed_keys(&set);
                }
                if let Some((started, events)) = bridge.take_finished() {
                    macro_hotkeys.resume();
                    let (actions, points) =
                        events_to_actions(&events, started, catalog, &thresholds);
                    *self = Self::Review(Box::new(ReviewState {
                        draft: Macro {
                            name: "Recorded".into(),
                            root: root_loop(actions),
                            ..Macro::new("Recorded", 0, Vec::new())
                        },
                        points,
                        status: StatusBanner::default(),
                        tooltip: TooltipState::default(),
                        pills_cache: HashMap::new(),
                        paint_revision: 0,
                    }));
                    // Bring the main window back before the review popup.
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    ctx.request_repaint();
                    return MacroRecordShowResult {
                        copy: None,
                        catalog_changed: false,
                    };
                }
                // Point squares are OS windows (see recording overlay). Deferred
                // egui viewports ignore position on Wayland and pile up mid-screen.
                // Status lives on the recording HUD (main window may be hidden).
                ctx.request_repaint_after(Duration::from_millis(16));
                MacroRecordShowResult {
                    copy: None,
                    catalog_changed: false,
                }
            }
            Self::Review(review) => match paint_review(
                review,
                MacroRecordShow {
                    ctx,
                    macro_hotkeys,
                    bridge,
                    catalog,
                    icons,
                    previews,
                    key_record,
                    hotkey_record,
                    screen_click,
                    macros,
                    compact_program_headers,
                    pending_scale,
                    thresholds,
                },
            ) {
                ReviewFrame::Continue { catalog_changed } => MacroRecordShowResult {
                    copy: None,
                    catalog_changed,
                },
                ReviewFrame::Copied(copy) => MacroRecordShowResult {
                    copy: Some(copy),
                    catalog_changed: true,
                },
                ReviewFrame::Close => {
                    *self = Self::Closed;
                    MacroRecordShowResult {
                        copy: None,
                        catalog_changed: false,
                    }
                }
            },
        }
    }
}

enum ReviewFrame {
    Continue { catalog_changed: bool },
    Copied(MacroRecordCopy),
    Close,
}

fn paint_review(review: &mut ReviewState, ui: MacroRecordShow<'_>) -> ReviewFrame {
    let MacroRecordShow {
        ctx,
        macro_hotkeys,
        bridge: _,
        catalog,
        icons,
        previews,
        key_record,
        hotkey_record,
        screen_click,
        macros,
        compact_program_headers,
        pending_scale,
        thresholds,
    } = ui;
    let ReviewState {
        draft,
        points,
        status,
        tooltip,
        pills_cache,
        paint_revision,
    } = review;

    let mut close = false;
    let mut copy = false;
    let mut save_points = false;
    let mut row_events: Vec<(ActionId, RowInteraction)> = Vec::new();
    let mut delete_id: Option<ActionId> = None;
    let mut is_dark = false;
    // Adopt nearby: (temp index, full CoordinateRef string, display name).
    let mut adopt_nearby: Option<(usize, String, String)> = None;
    let known_vars = collect_known_variable_names(draft);

    let screen = crate::widgets::dialog_constrain_rect(ctx);
    let default_w = (screen.width() * 0.75).max(480.0);
    let default_h = (screen.height() * 0.50).max(320.0);
    let default_pos = egui::pos2(
        screen.center().x - default_w * 0.5,
        screen.center().y - default_h * 0.5,
    );

    crate::widgets::fit_dialog_window(
        egui::Window::new("Recorded actions")
            .collapsible(false)
            .resizable(true)
            .default_size([default_w, default_h])
            .default_pos(default_pos)
            .min_size(crate::widgets::FLOATER_MIN_PICKER),
        ctx,
        egui::Id::new("sqyre_macro_record_review"),
        pending_scale,
    )
    .show(ctx, |ui| {
            is_dark = ui.visuals().dark_mode;
            ui.label(
                "Hover for view tip, middle-click or double-click to edit, right-click for Edit / Delete. Copy and paste into a macro.",
            );
            crate::widgets::section_separator(ui);

            if !points.is_empty() {
                crate::widgets::heading_with_count(ui, "Temporary points", points.len());
                ui.label(format!(
                    "Saved into program “{TEMPORARY_PROGRAM}” at the current resolution (replaced each recording). Enabled points are drawn on screen."
                ));
                let points_h = (ui.available_height() * 0.28).clamp(96.0, 220.0);
                let points_w = crate::widgets::visible_width(ui);
                crate::widgets::show_dialog_scroll_mod(ui, points_w, points_h, |s| s.id_salt("macro_record_points"), |ui| {
                        ui.set_max_width(points_w);
                        for (i, pt) in points.iter_mut().enumerate() {
                            let nearby =
                                nearby_saved_point(catalog, pt, thresholds.snap_point_px);
                            ui.horizontal(|ui| {
                                let enabled = ui
                                    .checkbox(&mut pt.save, "")
                                    .on_hover_text(ht::REC_SAVE_CATALOG);
                                if enabled.changed() && pt.save {
                                    pt.link_to = None;
                                }
                                let name_w = crate::widgets::name_edit_width(ui, &pt.name, "");
                                let name = ui.add(
                                    egui::TextEdit::singleline(&mut pt.name).desired_width(name_w),
                                );
                                // Editing the name after linking drops the link so refs
                                // follow temporary~name again.
                                if name.changed() {
                                    pt.link_to = None;
                                }
                                ui.label("X");
                                ui.add(crate::widgets::num_entry(&mut pt.x).speed(1).suffix(" px"))
                                    .on_hover_text(ht::REC_SCREEN_X);
                                ui.label("Y");
                                ui.add(crate::widgets::num_entry(&mut pt.y).speed(1).suffix(" px"))
                                    .on_hover_text(ht::REC_SCREEN_Y);
                            });
                            if let Some(near) = nearby {
                                ui.horizontal(|ui| {
                                    ui.add_space(SUB_ROW_INDENT);
                                    ui.weak(format!(
                                        "Nearby: {} ({} px)",
                                        near.display, near.dist
                                    ));
                                    let linked = pt.link_to.as_deref() == Some(near.coord_ref.as_str());
                                    if linked {
                                        ui.weak("linked");
                                    } else if ui
                                        .small_button("Use")
                                        .on_hover_text(format!(
                                            "Link Move to {} and skip saving a new point",
                                            near.display
                                        ))
                                        .clicked()
                                    {
                                        adopt_nearby =
                                            Some((i, near.coord_ref.clone(), near.name.clone()));
                                    }
                                });
                            }
                        }
                    });
                if crate::widgets::icon_button(
                    ui,
                    egui_phosphor::regular::FLOPPY_DISK,
                    "Save selected points",
                )
                .clicked()
                {
                    save_points = true;
                }
                crate::widgets::section_separator(ui);
            }

            let action_count = draft.root.children().len();
            crate::widgets::heading_with_count(ui, "Actions", action_count);
            // Leave room for Copy/Close (+ status) so the list grows with the window.
            let footer_reserve = if status.is_set() { 60.0 } else { 40.0 };
            let actions_h = (ui.available_height() - footer_reserve).max(80.0);
            let actions_w = crate::widgets::visible_width(ui);
            crate::widgets::show_dialog_scroll_mod(ui, actions_w, actions_h, |s| s.id_salt("macro_record_actions"), |ui| {
                    ui.set_max_width(actions_w);
                    let actions: Vec<Action> = draft.root.children().to_vec();
                    for action in &actions {
                        let interaction = tree_chrome::paint_action_row(
                            ui,
                            action,
                            catalog,
                            icons,
                            &known_vars,
                            is_dark,
                            RowHighlight::None,
                            pills_cache,
                            *paint_revision,
                            None,
                            false,
                        );
                        if interaction.action == RowAction::Delete {
                            delete_id = Some(action.id);
                        }
                        row_events.push((action.id, interaction));
                    }
                });

            crate::widgets::section_separator(ui);
            ui.horizontal(|ui| {
                if ui.small_button("Copy").clicked() {
                    copy = true;
                }
                if crate::widgets::cancel_icon_button(ui, "Close").clicked() {
                    close = true;
                }
            });
            status.paint(ui);
            // Esc closes when no widget inside the review window owns keys.
            if !close
                && !crate::focus_nav::focused_in_layer(ui.ctx(), ui.layer_id())
                && crate::widgets::consume_escape(ui)
            {
                close = true;
            }
        });

    if let Some((i, coord_ref, name)) = adopt_nearby {
        if let Some(pt) = points.get_mut(i) {
            pt.link_to = Some(coord_ref);
            pt.name = name;
            pt.save = false;
        }
        if let Some(kids) = draft.root.children_mut() {
            sync_move_refs_to_points(kids, points);
        }
        *paint_revision = paint_revision.wrapping_add(1);
    }

    if let Some(aid) = delete_id {
        let _ = draft.root.remove_by_id(aid);
        if tooltip.action_id() == Some(aid) {
            tooltip.cancel();
        }
        *paint_revision = paint_revision.wrapping_add(1);
    }

    let pointer = ctx.pointer_interact_pos();
    let mut any_view_hover = false;
    for (aid, interaction) in &row_events {
        if interaction.hovered || interaction.pointer_in_row {
            any_view_hover = true;
        }
        if let Some(action) = draft.root.find_by_id(*aid) {
            let action = action.clone();
            action_tooltip::ingest_row(tooltip, &action, *interaction, pointer);
        }
    }
    action_tooltip::end_hover_pass(tooltip, any_view_hover);

    let was_editing = tooltip.is_editing();
    {
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
                key_record,
                hotkey_record,
                macro_hotkeys,
                screen_click,
            },
            compact_program_headers,
            pending_scale,
        };
        let _ = action_tooltip::show(tooltip, ctx, draft, macros, &mut tip_ui, |_| {});
    }
    if was_editing && !tooltip.is_editing() {
        *paint_revision = paint_revision.wrapping_add(1);
    }

    let mut catalog_changed = false;

    if save_points {
        if let Some(kids) = draft.root.children_mut() {
            sync_move_refs_to_points(kids, points);
        }
        match replace_temporary_points(catalog, points) {
            Ok(n) => {
                status.set_ok(format!("Saved {n} point(s) into “{TEMPORARY_PROGRAM}”"));
                catalog_changed = true;
            }
            Err(e) => status.set_err(format!(
                "Save points failed: {}",
                crate::status_banner::user_error_detail(&e)
            )),
        }
    }

    if copy {
        if let Some(kids) = draft.root.children_mut() {
            sync_move_refs_to_points(kids, points);
        }
        if let Err(e) = replace_temporary_points(catalog, points) {
            status.set_err(format!(
                "Copy failed (points): {}",
                crate::status_banner::user_error_detail(&e)
            ));
            return ReviewFrame::Continue { catalog_changed };
        }
        catalog_changed = true;
        let actions = draft.root.children();
        let maps = match actions_to_clipboard(actions) {
            Ok(m) => m,
            Err(e) => {
                status.set_err(format!(
                    "Copy failed: {}",
                    crate::status_banner::user_error_detail(&e)
                ));
                return ReviewFrame::Continue { catalog_changed };
            }
        };
        let yaml = actions_to_yaml_text(actions).unwrap_or_default();
        status.set_ok(format!(
            "Copied {} action(s) — paste into the macro tree",
            maps.len()
        ));
        return ReviewFrame::Copied(MacroRecordCopy { maps, yaml });
    }

    if close {
        ReviewFrame::Close
    } else {
        ReviewFrame::Continue { catalog_changed }
    }
}

fn reset_temporary_program(catalog: &mut ProgramCatalog) -> Result<(), String> {
    if catalog.get(TEMPORARY_PROGRAM).is_none() {
        catalog
            .create_program(TEMPORARY_PROGRAM)
            .map_err(|e| e.to_string())?;
    } else {
        catalog
            .clear_points(TEMPORARY_PROGRAM)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Wipe [`TEMPORARY_PROGRAM`] and upsert checked points (full replace).
fn replace_temporary_points(
    catalog: &mut ProgramCatalog,
    points: &[TempPoint],
) -> Result<usize, String> {
    reset_temporary_program(catalog)?;
    let live = crate::data_editor::helpers::sync_live_monitor_rects(catalog);
    let mut n = 0;
    for pt in points.iter().filter(|p| p.save) {
        let name = pt.name.trim();
        if name.is_empty() {
            return Err("point name cannot be empty".into());
        }
        let (monitor, rx, ry) = sqyre_persist::absolute_point_to_relative(&live, pt.x, pt.y);
        catalog
            .upsert_point(
                TEMPORARY_PROGRAM,
                ProgramPoint {
                    name: name.to_string(),
                    monitor,
                    x: ScalarValue::Int(rx as i64),
                    y: ScalarValue::Int(ry as i64),
                },
            )
            .map_err(|e| e.to_string())?;
        n += 1;
    }
    Ok(n)
}

fn sync_move_refs_to_points(actions: &mut [Action], points: &mut [TempPoint]) {
    for action in actions {
        let ActionKind::Move { point, .. } = &mut action.kind else {
            continue;
        };
        let entity = point.name().to_string();
        let full = point.as_str().to_string();
        for pt in points.iter() {
            let matches = entity == pt.original_name
                || entity == pt.name.trim()
                || full == pt.original_name
                || pt.link_to.as_deref() == Some(full.as_str());
            if !matches {
                continue;
            }
            let target = pt
                .link_to
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| {
                    format!("{TEMPORARY_PROGRAM}{PROGRAM_DELIMITER}{}", pt.name.trim())
                });
            *point = CoordinateRef(target);
            break;
        }
    }
    for pt in points.iter_mut() {
        if let Some(link) = pt.link_to.as_deref() {
            pt.original_name = link.to_string();
        } else {
            pt.original_name = pt.name.trim().to_string();
        }
    }
}

fn actions_to_clipboard(actions: &[Action]) -> Result<Vec<serde_yaml::Mapping>, String> {
    let mut maps = Vec::with_capacity(actions.len());
    for action in actions {
        let map = sqyre_serialize::action_to_map(action).map_err(|e| e.to_string())?;
        sqyre_serialize::action_from_map(&map).map_err(|e| e.to_string())?;
        maps.push(map);
    }
    Ok(maps)
}

fn actions_to_yaml_text(actions: &[Action]) -> Result<String, String> {
    let mut docs = Vec::with_capacity(actions.len());
    for action in actions {
        let map = sqyre_serialize::action_to_map(action).map_err(|e| e.to_string())?;
        let text =
            serde_yaml::to_string(&serde_yaml::Value::Mapping(map)).map_err(|e| e.to_string())?;
        docs.push(text.trim().to_string());
    }
    Ok(docs.join("\n"))
}

fn events_to_actions(
    events: &[MacroRecordEvent],
    started: Instant,
    catalog: &ProgramCatalog,
    t: &RecordThresholds,
) -> (Vec<Action>, Vec<TempPoint>) {
    let compressed = compress_events(events);
    let mut actions = Vec::new();
    let mut points = Vec::new();
    let mut used_names = existing_point_names(catalog);
    let mut last_t = started;
    let mut last_move: Option<(i32, i32)> = None;
    let mut pending_key: Option<(String, Instant)> = None;
    let mut pending_btn: Option<(RecordMouseButton, Instant)> = None;

    let flush_wait = |actions: &mut Vec<Action>, last_t: &mut Instant, at: Instant| {
        let ms = at.duration_since(*last_t).as_millis() as i64;
        if ms >= t.wait_ms {
            actions.push(Action::new(ActionKind::Wait {
                time: ScalarValue::Int(ms),
            }));
        }
        *last_t = at;
    };

    let ensure_move = |actions: &mut Vec<Action>,
                       points: &mut Vec<TempPoint>,
                       used_names: &mut HashSet<String>,
                       last_move: &mut Option<(i32, i32)>,
                       last_t: &mut Instant,
                       x: i32,
                       y: i32,
                       at: Instant| {
        if *last_move == Some((x, y)) {
            return;
        }
        flush_wait(actions, last_t, at);
        let name = next_temp_point_name(used_names);
        used_names.insert(name.clone());
        points.push(TempPoint {
            original_name: name.clone(),
            name: name.clone(),
            x,
            y,
            save: true,
            link_to: None,
        });
        actions.push(Action::new(ActionKind::Move {
            point: CoordinateRef(format!("{TEMPORARY_PROGRAM}{PROGRAM_DELIMITER}{name}")),
            smooth: true,
            smooth_low: 0.05,
            smooth_high: 0.20,
            smooth_delay_ms: 1,
        }));
        *last_move = Some((x, y));
        *last_t = at;
    };

    for ev in &compressed {
        match ev {
            MacroRecordEvent::MouseMove { x, y, at } => {
                if last_move.is_none_or(|(lx, ly)| {
                    (x - lx).abs() >= t.move_min_px || (y - ly).abs() >= t.move_min_px
                }) {
                    ensure_move(
                        &mut actions,
                        &mut points,
                        &mut used_names,
                        &mut last_move,
                        &mut last_t,
                        *x,
                        *y,
                        *at,
                    );
                }
            }
            MacroRecordEvent::Button {
                button,
                pressed,
                x,
                y,
                at,
            } => {
                if *pressed {
                    if let Some((prev_btn, prev_at)) = pending_btn.take() {
                        flush_wait(&mut actions, &mut last_t, prev_at);
                        actions.push(click_action(prev_btn, PressState::Down));
                        last_t = prev_at;
                    }
                    ensure_move(
                        &mut actions,
                        &mut points,
                        &mut used_names,
                        &mut last_move,
                        &mut last_t,
                        *x,
                        *y,
                        *at,
                    );
                    pending_btn = Some((*button, *at));
                } else if let Some((prev_btn, prev_at)) = pending_btn.take() {
                    if prev_btn == *button {
                        let held = at.duration_since(prev_at).as_millis();
                        flush_wait(&mut actions, &mut last_t, prev_at);
                        if held <= t.tap_max_ms.max(0) as u128 {
                            actions.push(click_action(*button, PressState::Tap));
                        } else {
                            actions.push(click_action(*button, PressState::Down));
                            actions.push(click_action(*button, PressState::Up));
                        }
                        last_t = *at;
                    } else {
                        flush_wait(&mut actions, &mut last_t, prev_at);
                        actions.push(click_action(prev_btn, PressState::Down));
                        flush_wait(&mut actions, &mut last_t, *at);
                        actions.push(click_action(*button, PressState::Up));
                        last_t = *at;
                    }
                } else {
                    flush_wait(&mut actions, &mut last_t, *at);
                    actions.push(click_action(*button, PressState::Up));
                    last_t = *at;
                }
            }
            MacroRecordEvent::Key { name, pressed, at } => {
                if *pressed {
                    if let Some((prev, _)) = pending_key.as_ref() {
                        if prev == name {
                            // Duplicate down (OS auto-repeat / dual sync) — keep hold.
                            continue;
                        }
                        let (prev, prev_at) = pending_key.take().expect("pending");
                        flush_wait(&mut actions, &mut last_t, prev_at);
                        actions.push(key_action(&prev, PressState::Down));
                        last_t = prev_at;
                    }
                    pending_key = Some((name.clone(), *at));
                } else if let Some((prev, prev_at)) = pending_key.take() {
                    if prev == *name {
                        let held = at.duration_since(prev_at).as_millis();
                        flush_wait(&mut actions, &mut last_t, prev_at);
                        if held <= t.tap_max_ms.max(0) as u128 {
                            actions.push(key_action(name, PressState::Tap));
                        } else {
                            actions.push(key_action(name, PressState::Down));
                            actions.push(key_action(name, PressState::Up));
                        }
                        last_t = *at;
                    } else {
                        flush_wait(&mut actions, &mut last_t, prev_at);
                        actions.push(key_action(&prev, PressState::Down));
                        flush_wait(&mut actions, &mut last_t, *at);
                        actions.push(key_action(name, PressState::Up));
                        last_t = *at;
                    }
                } else {
                    flush_wait(&mut actions, &mut last_t, *at);
                    actions.push(key_action(name, PressState::Up));
                    last_t = *at;
                }
            }
        }
    }

    if let Some((btn, at)) = pending_btn.take() {
        flush_wait(&mut actions, &mut last_t, at);
        actions.push(click_action(btn, PressState::Down));
    }
    if let Some((key, at)) = pending_key.take() {
        flush_wait(&mut actions, &mut last_t, at);
        actions.push(key_action(&key, PressState::Down));
    }

    (actions, points)
}

fn compress_events(events: &[MacroRecordEvent]) -> Vec<MacroRecordEvent> {
    let mut out = Vec::new();
    let mut last_move: Option<(i32, i32)> = None;
    for ev in events {
        match ev {
            MacroRecordEvent::MouseMove { x, y, at } => {
                if last_move == Some((*x, *y)) {
                    continue;
                }
                if let Some(MacroRecordEvent::MouseMove { .. }) = out.last() {
                    out.pop();
                }
                last_move = Some((*x, *y));
                out.push(MacroRecordEvent::MouseMove {
                    x: *x,
                    y: *y,
                    at: *at,
                });
            }
            MacroRecordEvent::Key {
                name,
                pressed: true,
                ..
            } => {
                // Drop OS auto-repeat downs while this key is still held.
                let already_down = out.iter().rev().find_map(|e| match e {
                    MacroRecordEvent::Key {
                        name: n, pressed, ..
                    } if n == name => Some(*pressed),
                    _ => None,
                }) == Some(true);
                if already_down {
                    continue;
                }
                out.push(ev.clone());
            }
            other => {
                out.push(other.clone());
            }
        }
    }
    out
}

fn existing_point_names(catalog: &ProgramCatalog) -> HashSet<String> {
    let mut names = HashSet::new();
    if let Some(prog) = catalog.get(TEMPORARY_PROGRAM) {
        for bucket in prog.points.values() {
            for name in bucket.keys() {
                names.insert(name.clone());
            }
        }
    }
    names
}

struct NearbyPoint {
    /// Full `Program~Name` coordinate ref.
    coord_ref: String,
    /// Catalog point key / name for the TempPoint name field.
    name: String,
    /// UI label, e.g. `General~Rec001` or just the name when in General.
    display: String,
    dist: i32,
}

/// Nearest saved catalog point within `snap_px` (Chebyshev), if any.
fn nearby_saved_point(
    catalog: &ProgramCatalog,
    pt: &TempPoint,
    snap_px: i32,
) -> Option<NearbyPoint> {
    let empty = Macro::new("", 0, vec![]);
    let mut best: Option<NearbyPoint> = None;
    for prog in catalog.program_names() {
        if prog == TEMPORARY_PROGRAM {
            continue;
        }
        let Some(pdata) = catalog.get(prog) else {
            continue;
        };
        let Some(bucket) = pdata
            .points
            .get(catalog.resolution_key())
            .or_else(|| pdata.points.values().next())
        else {
            continue;
        };
        for (key, saved) in bucket {
            let display_name = if saved.name.trim().is_empty() {
                key.as_str()
            } else {
                saved.name.as_str()
            };
            let coord = format!("{prog}{PROGRAM_DELIMITER}{key}");
            let Ok((sx, sy)) = catalog.resolve_point(&CoordinateRef(coord.clone()), &empty) else {
                continue;
            };
            let dist = (sx - pt.x).abs().max((sy - pt.y).abs());
            if dist > snap_px {
                continue;
            }
            let better = best
                .as_ref()
                .is_none_or(|b| dist < b.dist || (dist == b.dist && coord < b.coord_ref));
            if better {
                let display = if prog == GENERAL_PROGRAM {
                    display_name.to_string()
                } else {
                    format!("{prog}{PROGRAM_DELIMITER}{display_name}")
                };
                best = Some(NearbyPoint {
                    coord_ref: coord,
                    name: key.clone(),
                    display,
                    dist,
                });
            }
        }
    }
    best
}

fn next_temp_point_name(used: &HashSet<String>) -> String {
    for i in 1..10_000 {
        let name = format!("Rec{i:03}");
        if !used.contains(&name) {
            return name;
        }
    }
    format!("Rec{}", used.len() + 1)
}

fn click_action(button: RecordMouseButton, state: PressState) -> Action {
    Action::new(ActionKind::Click {
        button: MouseButton::parse(button.as_str()),
        state,
    })
}

fn key_action(key: &str, state: PressState) -> Action {
    Action::new(ActionKind::Key {
        key: key.to_string(),
        state,
    })
}

/// Move points that would be created from the live event stream (same distance
/// rules as [`events_to_actions`]), for on-screen crosshairs while recording.
#[cfg(any(test, feature = "native-runtime"))]
fn live_record_points(events: &[MacroRecordEvent], t: &RecordThresholds) -> Vec<TempPoint> {
    let compressed = compress_events(events);
    let mut out = Vec::new();
    let mut last_move: Option<(i32, i32)> = None;
    let push = |out: &mut Vec<TempPoint>, last_move: &mut Option<(i32, i32)>, x: i32, y: i32| {
        if *last_move == Some((x, y)) {
            return;
        }
        let name = format!("Rec{:03}", out.len() + 1);
        out.push(TempPoint {
            original_name: name.clone(),
            name,
            x,
            y,
            save: true,
            link_to: None,
        });
        *last_move = Some((x, y));
    };
    for ev in &compressed {
        match ev {
            MacroRecordEvent::MouseMove { x, y, .. }
                if last_move.is_none_or(|(lx, ly)| {
                    (x - lx).abs() >= t.move_min_px || (y - ly).abs() >= t.move_min_px
                }) =>
            {
                push(&mut out, &mut last_move, *x, *y);
            }
            MacroRecordEvent::Button {
                x,
                y,
                pressed: true,
                ..
            } => {
                push(&mut out, &mut last_move, *x, *y);
            }
            _ => {}
        }
    }
    out
}

/// Desktop pixels to mark while recording or reviewing.
///
/// Drawn by the recording overlay as override-redirect / topmost windows.
/// egui deferred viewports cannot be positioned on Wayland, so they stacked
/// in a cascade down the middle of the screen.
#[cfg(any(test, feature = "native-runtime"))]
pub(crate) fn desktop_marker_points(
    ui: &MacroRecordUi,
    bridge: &MacroRecordBridge,
    thresholds: &RecordThresholds,
) -> Vec<(i32, i32)> {
    match ui {
        MacroRecordUi::Closed => Vec::new(),
        MacroRecordUi::Recording => live_record_points(&bridge.peek_events(), thresholds)
            .into_iter()
            .map(|p| (p.x, p.y))
            .collect(),
        MacroRecordUi::Review(review) => review
            .points
            .iter()
            .filter(|p| p.save || p.link_to.is_some())
            .map(|p| (p.x, p.y))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compress_keeps_last_move_before_click() {
        let t0 = Instant::now();
        let events = vec![
            MacroRecordEvent::MouseMove { x: 1, y: 1, at: t0 },
            MacroRecordEvent::MouseMove {
                x: 2,
                y: 2,
                at: t0 + Duration::from_millis(5),
            },
            MacroRecordEvent::Button {
                button: RecordMouseButton::Left,
                pressed: true,
                x: 2,
                y: 2,
                at: t0 + Duration::from_millis(10),
            },
            MacroRecordEvent::Button {
                button: RecordMouseButton::Left,
                pressed: false,
                x: 2,
                y: 2,
                at: t0 + Duration::from_millis(20),
            },
        ];
        let compressed = compress_events(&events);
        assert_eq!(compressed.len(), 3);
        let cat = ProgramCatalog::default();
        let t = RecordThresholds::from_settings(&sqyre_persist::UserSettings::default());
        let (actions, points) = events_to_actions(&compressed, t0, &cat, &t);
        assert_eq!(points.len(), 1);
        assert!(matches!(actions[0].kind, ActionKind::Move { .. }));
        assert!(matches!(
            actions.last().map(|a| &a.kind),
            Some(ActionKind::Click {
                state: PressState::Tap,
                ..
            })
        ));
    }

    #[test]
    fn live_record_points_match_final_moves() {
        let t0 = Instant::now();
        let events = vec![
            MacroRecordEvent::MouseMove {
                x: 100,
                y: 100,
                at: t0,
            },
            MacroRecordEvent::MouseMove {
                x: 102,
                y: 100,
                at: t0 + Duration::from_millis(5),
            },
            MacroRecordEvent::MouseMove {
                x: 200,
                y: 200,
                at: t0 + Duration::from_millis(10),
            },
            MacroRecordEvent::Button {
                button: RecordMouseButton::Left,
                pressed: true,
                x: 200,
                y: 200,
                at: t0 + Duration::from_millis(15),
            },
            MacroRecordEvent::Button {
                button: RecordMouseButton::Left,
                pressed: false,
                x: 200,
                y: 200,
                at: t0 + Duration::from_millis(20),
            },
        ];
        let t = RecordThresholds::from_settings(&sqyre_persist::UserSettings::default());
        let live = live_record_points(&events, &t);
        let cat = ProgramCatalog::default();
        let (_, points) = events_to_actions(&events, t0, &cat, &t);
        assert_eq!(live.len(), points.len());
        for (a, b) in live.iter().zip(points.iter()) {
            assert_eq!((a.x, a.y), (b.x, b.y));
        }
    }

    #[test]
    fn held_key_records_one_down_and_up() {
        let t0 = Instant::now();
        let events = vec![
            MacroRecordEvent::Key {
                name: "a".into(),
                pressed: true,
                at: t0,
            },
            MacroRecordEvent::Key {
                name: "a".into(),
                pressed: true,
                at: t0 + Duration::from_millis(50),
            },
            MacroRecordEvent::Key {
                name: "a".into(),
                pressed: true,
                at: t0 + Duration::from_millis(100),
            },
            MacroRecordEvent::Key {
                name: "a".into(),
                pressed: false,
                at: t0 + Duration::from_millis(400),
            },
        ];
        let cat = ProgramCatalog::default();
        let t = RecordThresholds::from_settings(&sqyre_persist::UserSettings::default());
        let (actions, _) = events_to_actions(&events, t0, &cat, &t);
        let key_states: Vec<_> = actions
            .iter()
            .filter_map(|a| match &a.kind {
                ActionKind::Key { state, .. } => Some(*state),
                _ => None,
            })
            .collect();
        assert_eq!(key_states, vec![PressState::Down, PressState::Up]);
    }

    #[test]
    fn desktop_markers_follow_recorded_desktop_points() {
        let bridge = MacroRecordBridge::new();
        let t = RecordThresholds::from_settings(&sqyre_persist::UserSettings::default());
        assert!(desktop_marker_points(&MacroRecordUi::Closed, &bridge, &t).is_empty());

        bridge.arm();
        // Same-timestamp samples collapse to the latest desktop point.
        bridge.record_desktop_pos(10, 20);
        bridge.record_desktop_pos(400, 500);
        assert_eq!(
            desktop_marker_points(&MacroRecordUi::Recording, &bridge, &t),
            vec![(400, 500)]
        );

        let review = MacroRecordUi::Review(Box::new(ReviewState {
            draft: Macro::new("Recorded", 0, Vec::new()),
            points: vec![
                TempPoint {
                    original_name: "a".into(),
                    name: "a".into(),
                    x: 1,
                    y: 2,
                    save: false,
                    link_to: None,
                },
                TempPoint {
                    original_name: "b".into(),
                    name: "b".into(),
                    x: 9,
                    y: 8,
                    save: true,
                    link_to: None,
                },
                TempPoint {
                    original_name: "c".into(),
                    name: "c".into(),
                    x: 3,
                    y: 4,
                    save: false,
                    link_to: Some("Demo~Center".into()),
                },
            ],
            status: StatusBanner::default(),
            tooltip: TooltipState::default(),
            pills_cache: HashMap::new(),
            paint_revision: 0,
        }));
        assert_eq!(
            desktop_marker_points(&review, &bridge, &t),
            vec![(9, 8), (3, 4)]
        );
    }
}
