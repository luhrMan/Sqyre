//! Macro recording: capture moves/clicks/keys/waits until Esc, then review/copy.

use crate::action_tooltip::{self, TooltipState};
use crate::hotkey_record::HotkeyRecordUi;
use crate::icon_cache::IconCache;
use crate::key_record::KeyRecordUi;
use crate::paint_ctx::{CatalogPaint, RecordBridges, TipUiCtx, VarTheme};
use crate::preview_tooltip::PreviewTooltipCache;
use crate::tree_chrome::{self, RowAction, RowHighlight, RowInteraction};
use eframe::egui;
use sqyre_domain::{
    collect_known_variable_names, root_loop, Action, ActionId, ActionKind, CoordinateRef, Macro,
    MouseButton, PressState, ScalarValue, PROGRAM_DELIMITER,
};
use sqyre_hotkeys::{
    MacroHotkeyBridge, MacroRecordBridge, MacroRecordEvent, RecordMouseButton, ScreenClickBridge,
};
use sqyre_persist::{ProgramCatalog, ProgramPoint, GENERAL_PROGRAM, TEMPORARY_PROGRAM};
use sqyre_ui_model::SummaryPill;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// Minimum gap (ms) before inserting a Wait between recorded actions.
const WAIT_THRESHOLD_MS: i64 = 40;
/// Collapse mouse-move samples closer than this (pixels).
const MOVE_MIN_DISTANCE: i32 = 8;
/// Collapse Tap when press→release is shorter than this.
const TAP_MAX_MS: u128 = 200;

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

/// Max Chebyshev distance (px) to surface a nearby saved-point recommendation.
const NEARBY_POINT_PX: i32 = 24;

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
}

#[derive(Debug, Clone)]
pub(crate) struct ReviewState {
    /// Synthetic macro whose root children are the recorded actions.
    draft: Macro,
    points: Vec<TempPoint>,
    status: String,
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
                    let (actions, points) = events_to_actions(&events, started, catalog);
                    *self = Self::Review(Box::new(ReviewState {
                        draft: Macro {
                            name: "Recorded".into(),
                            root: root_loop(actions),
                            ..Macro::new("Recorded", 0, Vec::new())
                        },
                        points,
                        status: String::new(),
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
                // Live crosshairs for move points captured so far.
                #[cfg(feature = "native-runtime")]
                {
                    let live = live_record_points(&bridge.peek_events());
                    sync_temp_point_markers(ctx, &live, None, None);
                }
                // Status lives on the recording HUD (main window may be hidden).
                ctx.request_repaint_after(Duration::from_millis(16));
                MacroRecordShowResult {
                    copy: None,
                    catalog_changed: false,
                }
            }
            Self::Review(review) => match paint_review(
                review,
                ctx,
                macro_hotkeys,
                catalog,
                icons,
                previews,
                key_record,
                hotkey_record,
                screen_click,
                macros,
                compact_program_headers,
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

#[allow(clippy::too_many_arguments)]
fn paint_review(
    review: &mut ReviewState,
    ctx: &egui::Context,
    macro_hotkeys: &MacroHotkeyBridge,
    catalog: &mut ProgramCatalog,
    icons: &mut IconCache,
    previews: &mut PreviewTooltipCache,
    key_record: &mut KeyRecordUi,
    hotkey_record: &mut HotkeyRecordUi,
    screen_click: &ScreenClickBridge,
    macros: &[(String, Vec<String>)],
    compact_program_headers: bool,
) -> ReviewFrame {
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
    let mut hovered_point: Option<usize> = None;
    let mut editing_point: Option<usize> = None;
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
            .min_size([400.0, 280.0]),
        ctx,
    )
    .show(ctx, |ui| {
            is_dark = ui.visuals().dark_mode;
            ui.label(
                "Hover for view tip, right-click or double-click to edit. Copy and paste into a macro.",
            );
            ui.separator();

            if !points.is_empty() {
                crate::widgets::heading_with_count(ui, "Temporary points", points.len());
                ui.label(format!(
                    "Saved into program “{TEMPORARY_PROGRAM}” at the current resolution (replaced each recording). Enabled points are drawn on screen."
                ));
                let points_h = (ui.available_height() * 0.28).clamp(96.0, 220.0);
                crate::pickers::scroll_vertical()
                    .id_salt("macro_record_points")
                    .max_height(points_h)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (i, pt) in points.iter_mut().enumerate() {
                            let mut row_hovered = false;
                            let mut row_editing = false;
                            let nearby = nearby_saved_point(catalog, pt);
                            ui.horizontal(|ui| {
                                let enabled = ui
                                    .checkbox(&mut pt.save, "")
                                    .on_hover_text("Save into catalog on Copy / Save selected");
                                if enabled.changed() && pt.save {
                                    pt.link_to = None;
                                }
                                let name = ui.add(
                                    egui::TextEdit::singleline(&mut pt.name).desired_width(120.0),
                                );
                                // Editing the name after linking drops the link so refs
                                // follow temporary~name again.
                                if name.changed() {
                                    pt.link_to = None;
                                }
                                ui.label("X");
                                let x_resp = ui
                                    .add(egui::DragValue::new(&mut pt.x).speed(1).suffix(" px"))
                                    .on_hover_text("Screen X (physical pixels)");
                                ui.label("Y");
                                let y_resp = ui
                                    .add(egui::DragValue::new(&mut pt.y).speed(1).suffix(" px"))
                                    .on_hover_text("Screen Y (physical pixels)");

                                row_hovered = enabled.contains_pointer()
                                    || name.contains_pointer()
                                    || x_resp.contains_pointer()
                                    || y_resp.contains_pointer();
                                row_editing = name.has_focus()
                                    || x_resp.has_focus()
                                    || y_resp.has_focus();
                            });
                            if let Some(near) = nearby {
                                ui.horizontal(|ui| {
                                    ui.add_space(24.0);
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
                            if row_editing {
                                editing_point = Some(i);
                            }
                            if row_hovered {
                                hovered_point = Some(i);
                            }
                        }
                    });
                if ui.button("Save selected points").clicked() {
                    save_points = true;
                }
                ui.separator();
            }

            let action_count = draft.root.children().len();
            crate::widgets::heading_with_count(ui, "Actions", action_count);
            // Leave room for Copy/Close (+ status) so the list grows with the window.
            let footer_reserve = if status.is_empty() { 40.0 } else { 60.0 };
            let actions_h = (ui.available_height() - footer_reserve).max(80.0);
            crate::pickers::scroll_vertical()
                .id_salt("macro_record_actions")
                .max_height(actions_h)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_max_width(crate::widgets::visible_width(ui));
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

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Copy").clicked() {
                    copy = true;
                }
                if ui.button("Close").clicked() {
                    close = true;
                }
            });
            if !status.is_empty() {
                ui.colored_label(egui::Color32::LIGHT_GREEN, status.as_str());
            }
        });

    #[cfg(feature = "native-runtime")]
    sync_temp_point_markers(ctx, points, hovered_point, editing_point);

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
                *status = format!("Saved {n} point(s) into “{TEMPORARY_PROGRAM}”");
                catalog_changed = true;
            }
            Err(e) => *status = format!("Save points failed: {e}"),
        }
    }

    if copy {
        if let Some(kids) = draft.root.children_mut() {
            sync_move_refs_to_points(kids, points);
        }
        if let Err(e) = replace_temporary_points(catalog, points) {
            *status = format!("Copy failed (points): {e}");
            return ReviewFrame::Continue { catalog_changed };
        }
        catalog_changed = true;
        let actions = draft.root.children();
        let maps = match actions_to_clipboard(actions) {
            Ok(m) => m,
            Err(e) => {
                *status = format!("Copy failed: {e}");
                return ReviewFrame::Continue { catalog_changed };
            }
        };
        let yaml = actions_to_yaml_text(actions).unwrap_or_default();
        *status = format!(
            "Copied {} action(s) — paste into the macro tree",
            maps.len()
        );
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
    let mut n = 0;
    for pt in points.iter().filter(|p| p.save) {
        let name = pt.name.trim();
        if name.is_empty() {
            return Err("point name cannot be empty".into());
        }
        let (monitor, rx, ry) =
            sqyre_persist::absolute_point_to_relative(catalog.monitor_rects(), pt.x, pt.y);
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
        if ms >= WAIT_THRESHOLD_MS {
            actions.push(Action {
                id: ActionId::new(),
                kind: ActionKind::Wait {
                    time: ScalarValue::Int(ms),
                },
            });
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
        actions.push(Action {
            id: ActionId::new(),
            kind: ActionKind::Move {
                point: CoordinateRef(format!("{TEMPORARY_PROGRAM}{PROGRAM_DELIMITER}{name}")),
                smooth: true,
                smooth_low: 0.05,
                smooth_high: 0.20,
                smooth_delay_ms: 1,
            },
        });
        *last_move = Some((x, y));
        *last_t = at;
    };

    for ev in &compressed {
        match ev {
            MacroRecordEvent::MouseMove { x, y, at } => {
                if last_move.is_none_or(|(lx, ly)| {
                    (x - lx).abs() >= MOVE_MIN_DISTANCE || (y - ly).abs() >= MOVE_MIN_DISTANCE
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
                        if held <= TAP_MAX_MS {
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
                        if held <= TAP_MAX_MS {
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

/// Nearest saved catalog point within [`NEARBY_POINT_PX`] (Chebyshev), if any.
fn nearby_saved_point(catalog: &ProgramCatalog, pt: &TempPoint) -> Option<NearbyPoint> {
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
            if dist > NEARBY_POINT_PX {
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
    Action {
        id: ActionId::new(),
        kind: ActionKind::Click {
            button: MouseButton::parse(button.as_str()),
            state,
        },
    }
}

fn key_action(key: &str, state: PressState) -> Action {
    Action {
        id: ActionId::new(),
        kind: ActionKind::Key {
            key: key.to_string(),
            state,
        },
    }
}

/// Move points that would be created from the live event stream (same distance
/// rules as [`events_to_actions`]), for on-screen crosshairs while recording.
#[cfg(any(test, feature = "native-runtime"))]
fn live_record_points(events: &[MacroRecordEvent]) -> Vec<TempPoint> {
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
            MacroRecordEvent::MouseMove { x, y, .. } => {
                if last_move.is_none_or(|(lx, ly)| {
                    (x - lx).abs() >= MOVE_MIN_DISTANCE || (y - ly).abs() >= MOVE_MIN_DISTANCE
                }) {
                    push(&mut out, &mut last_move, *x, *y);
                }
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

/// Default / hover / editing colors for on-screen temp-point markers.
#[cfg(feature = "native-runtime")]
const MARKER_DEFAULT: egui::Color32 = crate::theme::PRIMARY;
#[cfg(feature = "native-runtime")]
const MARKER_HOVER: egui::Color32 = crate::theme::MACRO_START;
#[cfg(feature = "native-runtime")]
const MARKER_EDIT: egui::Color32 = crate::theme::MACRO_STOP;

#[cfg(feature = "native-runtime")]
fn sync_temp_point_markers(
    ctx: &egui::Context,
    points: &[TempPoint],
    hovered: Option<usize>,
    editing: Option<usize>,
) {
    use sqyre_capture::{enable_overlay_window_transparency, skip_taskbar_for_overlay_windows};
    use std::sync::atomic::{AtomicU64, Ordering};

    static LAST_HINTS_MS: AtomicU64 = AtomicU64::new(0);

    let mut any = false;
    for (i, pt) in points.iter().enumerate() {
        // Enabled saves and linked (reused) points get markers.
        if !pt.save && pt.link_to.is_none() {
            continue;
        }
        any = true;
        let color = if editing == Some(i) {
            MARKER_EDIT
        } else if hovered == Some(i) {
            MARKER_HOVER
        } else {
            MARKER_DEFAULT
        };
        show_temp_point_marker(ctx, i, pt, color);
    }

    if !any {
        return;
    }

    let tick = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let prev = LAST_HINTS_MS.load(Ordering::Relaxed);
    if tick.saturating_sub(prev) >= 250 {
        LAST_HINTS_MS.store(tick, Ordering::Relaxed);
        let _ = skip_taskbar_for_overlay_windows();
        let _ = enable_overlay_window_transparency();
    }
}

#[cfg(feature = "native-runtime")]
fn show_temp_point_marker(ctx: &egui::Context, index: usize, pt: &TempPoint, color: egui::Color32) {
    use eframe::egui::{
        FontId, Frame, Margin, Pos2, Sense, Stroke, Vec2, ViewportBuilder, ViewportId,
    };
    use sqyre_capture::OVERLAY_WM_TITLE;

    const DIAMETER: f32 = 12.0;
    const ARM: f32 = 3.5;
    const CROSS_STROKE: f32 = 1.25;
    const RING_STROKE: f32 = 1.25;
    const LABEL_GAP: f32 = 1.0;
    const LABEL_H: f32 = 10.0;

    let label = if pt.name.trim().is_empty() {
        format!("#{index}")
    } else {
        pt.name.trim().to_string()
    };
    let font = FontId::proportional(9.0);
    let galley = ctx.fonts_mut(|f| f.layout_no_wrap(label.clone(), font.clone(), color));
    let radius = DIAMETER * 0.5;
    let content_w = galley.size().x.max(DIAMETER).ceil();
    let content_h = (LABEL_H + LABEL_GAP + DIAMETER).ceil();
    // Ring stroke is centered on the circle edge — pad so it stays inside the window.
    let outer_w = (content_w + RING_STROKE).ceil();
    let outer_h = (content_h + RING_STROKE).ceil();

    // Recorded coords are physical desktop pixels; egui viewport position is logical
    // points (`physical / pixels_per_point`). Without this divide, markers land at
    // `recorded * ppp` and only near-origin points remain on-screen.
    let ppp = ctx.pixels_per_point().max(0.01);
    let cx = pt.x as f32 / ppp;
    let cy = pt.y as f32 / ppp;
    // Circle center sits on the recorded point (below the label).
    let cross_y_offset = RING_STROKE * 0.5 + LABEL_H + LABEL_GAP + radius;
    let pos = Pos2::new(cx - outer_w * 0.5, cy - cross_y_offset);

    let id = ViewportId::from_hash_of(format!("sqyre_temp_point_marker_{index}"));
    let builder = ViewportBuilder::default()
        .with_title(OVERLAY_WM_TITLE)
        .with_decorations(false)
        .with_resizable(false)
        .with_always_on_top()
        .with_taskbar(false)
        .with_mouse_passthrough(true)
        .with_window_type(egui::X11WindowType::Notification)
        .with_transparent(true)
        .with_inner_size([outer_w, outer_h])
        .with_min_inner_size([outer_w, outer_h])
        .with_position(pos);

    ctx.show_viewport_deferred(id, builder, move |ui, _class| {
        Frame::NONE
            .fill(egui::Color32::TRANSPARENT)
            .inner_margin(Margin::ZERO)
            .show(ui, |ui| {
                ui.set_min_size(Vec2::new(outer_w, outer_h));
                let (rect, _) = ui.allocate_exact_size(Vec2::new(outer_w, outer_h), Sense::hover());
                let painter = ui.painter();
                let galley = painter.layout_no_wrap(label.clone(), font.clone(), color);
                let label_pos = Pos2::new(
                    rect.center().x - galley.size().x * 0.5,
                    rect.top() + RING_STROKE * 0.5,
                );
                painter.galley(label_pos, galley, color);
                let c = Pos2::new(
                    rect.center().x,
                    rect.top() + RING_STROKE * 0.5 + LABEL_H + LABEL_GAP + radius,
                );
                painter.circle_filled(c, radius, crate::theme::overlay_panel_fill());
                painter.circle_stroke(c, radius, Stroke::new(RING_STROKE, color));
                let stroke = Stroke::new(CROSS_STROKE, color);
                painter.line_segment(
                    [Pos2::new(c.x - ARM, c.y), Pos2::new(c.x + ARM, c.y)],
                    stroke,
                );
                painter.line_segment(
                    [Pos2::new(c.x, c.y - ARM), Pos2::new(c.x, c.y + ARM)],
                    stroke,
                );
            });
        ui.ctx().request_repaint();
    });
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
        let (actions, points) = events_to_actions(&compressed, t0, &cat);
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
        let live = live_record_points(&events);
        let cat = ProgramCatalog::default();
        let (_, points) = events_to_actions(&events, t0, &cat);
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
        let (actions, _) = events_to_actions(&events, t0, &cat);
        let key_states: Vec<_> = actions
            .iter()
            .filter_map(|a| match &a.kind {
                ActionKind::Key { state, .. } => Some(*state),
                _ => None,
            })
            .collect();
        assert_eq!(key_states, vec![PressState::Down, PressState::Up]);
    }
}
