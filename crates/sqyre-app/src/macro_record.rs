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
use sqyre_persist::{ProgramCatalog, ProgramPoint, GENERAL_PROGRAM};
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
    /// Editable text for X (committed to `x` when a valid integer).
    pub x_text: String,
    /// Editable text for Y (committed to `y` when a valid integer).
    pub y_text: String,
    /// When true, Copy / Save points will upsert into the catalog and draw on screen.
    pub save: bool,
}

/// Result of Copy from the review window.
pub(crate) struct MacroRecordCopy {
    pub maps: Vec<serde_yaml::Mapping>,
    pub yaml: String,
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
    pub fn open(&mut self, macro_hotkeys: &MacroHotkeyBridge, bridge: &MacroRecordBridge) {
        if !matches!(self, Self::Closed) {
            return;
        }
        macro_hotkeys.suspend();
        bridge.arm();
        *self = Self::Recording;
    }

    pub fn is_open(&self) -> bool {
        !matches!(self, Self::Closed)
    }

    /// Draw recording chrome / review popup.
    ///
    /// On Copy: upserts checked temp points and returns clipboard payload.
    pub fn show(&mut self, ui: MacroRecordShow<'_>) -> Option<MacroRecordCopy> {
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
        } = ui;
        match self {
            Self::Closed => None,
            Self::Recording => {
                if bridge.take_cancelled() {
                    macro_hotkeys.resume();
                    *self = Self::Closed;
                    return None;
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
                    return None;
                }
                // Status lives on the recording HUD (main window may be hidden).
                ctx.request_repaint_after(Duration::from_millis(16));
                None
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
            ) {
                ReviewFrame::Continue => None,
                ReviewFrame::Copied(copy) => Some(copy),
                ReviewFrame::Close => {
                    *self = Self::Closed;
                    None
                }
            },
        }
    }
}

enum ReviewFrame {
    Continue,
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
    let known_vars = collect_known_variable_names(draft);

    let screen = ctx.content_rect();
    let default_w = (screen.width() * 0.75).max(480.0);
    let default_h = (screen.height() * 0.50).max(320.0);
    let default_pos = egui::pos2(
        screen.center().x - default_w * 0.5,
        screen.center().y - default_h * 0.5,
    );

    egui::Window::new("Recorded actions")
        .collapsible(false)
        .resizable(true)
        .default_size([default_w, default_h])
        .default_pos(default_pos)
        .min_size([400.0, 280.0])
        .constrain(true)
        .show(ctx, |ui| {
            is_dark = ui.visuals().dark_mode;
            ui.label(
                "Hover for view tip, right-click or double-click to edit. Copy and paste into a macro.",
            );
            ui.separator();

            if !points.is_empty() {
                ui.heading("Temporary points");
                ui.label(format!(
                    "Saved into program “{GENERAL_PROGRAM}” at the current resolution. Enabled points are drawn on screen."
                ));
                let points_h = (ui.available_height() * 0.28).clamp(96.0, 220.0);
                egui::ScrollArea::vertical()
                    .id_salt("macro_record_points")
                    .max_height(points_h)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (i, pt) in points.iter_mut().enumerate() {
                            let mut row_hovered = false;
                            let mut row_editing = false;
                            ui.horizontal(|ui| {
                                let enabled = ui.checkbox(&mut pt.save, "");
                                let name = ui.add(
                                    egui::TextEdit::singleline(&mut pt.name).desired_width(120.0),
                                );
                                ui.label("X");
                                let x_valid = pt.x_text.trim().parse::<i32>().ok();
                                let mut x_edit = egui::TextEdit::singleline(&mut pt.x_text)
                                    .desired_width(56.0);
                                if x_valid.is_none() {
                                    x_edit = x_edit.text_color(crate::theme::MACRO_STOP);
                                }
                                let x_resp = ui.add(x_edit).on_hover_text("Integer screen X");
                                ui.label("Y");
                                let y_valid = pt.y_text.trim().parse::<i32>().ok();
                                let mut y_edit = egui::TextEdit::singleline(&mut pt.y_text)
                                    .desired_width(56.0);
                                if y_valid.is_none() {
                                    y_edit = y_edit.text_color(crate::theme::MACRO_STOP);
                                }
                                let y_resp = ui.add(y_edit).on_hover_text("Integer screen Y");

                                if let Some(v) = x_valid {
                                    pt.x = v;
                                }
                                if let Some(v) = y_valid {
                                    pt.y = v;
                                }

                                row_hovered = enabled.contains_pointer()
                                    || name.contains_pointer()
                                    || x_resp.contains_pointer()
                                    || y_resp.contains_pointer();
                                row_editing = name.has_focus()
                                    || x_resp.has_focus()
                                    || y_resp.has_focus();
                            });
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

            ui.heading("Actions");
            // Leave room for Copy/Close (+ status) so the list grows with the window.
            let footer_reserve = if status.is_empty() { 40.0 } else { 60.0 };
            let actions_h = (ui.available_height() - footer_reserve).max(80.0);
            egui::ScrollArea::vertical()
                .id_salt("macro_record_actions")
                .max_height(actions_h)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
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
        };
        let _ = action_tooltip::show(tooltip, ctx, draft, macros, &mut tip_ui, |_| {});
    }
    if was_editing && !tooltip.is_editing() {
        *paint_revision = paint_revision.wrapping_add(1);
    }

    if save_points {
        if let Some(kids) = draft.root.children_mut() {
            sync_move_refs_to_points(kids, points);
        }
        match upsert_selected_points(catalog, points) {
            Ok(n) => *status = format!("Saved {n} point(s)"),
            Err(e) => *status = format!("Save points failed: {e}"),
        }
    }

    if copy {
        if let Some(kids) = draft.root.children_mut() {
            sync_move_refs_to_points(kids, points);
        }
        if let Err(e) = upsert_selected_points(catalog, points) {
            *status = format!("Copy failed (points): {e}");
            return ReviewFrame::Continue;
        }
        let actions = draft.root.children();
        let maps = match actions_to_clipboard(actions) {
            Ok(m) => m,
            Err(e) => {
                *status = format!("Copy failed: {e}");
                return ReviewFrame::Continue;
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
        ReviewFrame::Continue
    }
}

fn upsert_selected_points(
    catalog: &mut ProgramCatalog,
    points: &[TempPoint],
) -> Result<usize, String> {
    let mut n = 0;
    let saving: Vec<_> = points.iter().filter(|p| p.save).collect();
    if saving.is_empty() {
        return Ok(0);
    }
    if catalog.get(GENERAL_PROGRAM).is_none() {
        catalog
            .create_program(GENERAL_PROGRAM)
            .map_err(|e| e.to_string())?;
    }
    for pt in saving {
        let name = pt.name.trim();
        if name.is_empty() {
            return Err("point name cannot be empty".into());
        }
        if pt.x_text.trim().parse::<i32>().is_err() {
            return Err(format!("point “{name}”: X must be an integer"));
        }
        if pt.y_text.trim().parse::<i32>().is_err() {
            return Err(format!("point “{name}”: Y must be an integer"));
        }
        catalog
            .upsert_point(
                GENERAL_PROGRAM,
                ProgramPoint {
                    name: name.to_string(),
                    x: ScalarValue::Int(pt.x as i64),
                    y: ScalarValue::Int(pt.y as i64),
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
        for pt in points.iter() {
            if entity == pt.original_name || entity == pt.name.trim() {
                *point = CoordinateRef(format!(
                    "{GENERAL_PROGRAM}{PROGRAM_DELIMITER}{}",
                    pt.name.trim()
                ));
                break;
            }
        }
    }
    for pt in points.iter_mut() {
        pt.original_name = pt.name.trim().to_string();
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
            x_text: x.to_string(),
            y_text: y.to_string(),
            save: true,
        });
        actions.push(Action {
            id: ActionId::new(),
            kind: ActionKind::Move {
                point: CoordinateRef(format!("{GENERAL_PROGRAM}{PROGRAM_DELIMITER}{name}")),
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
    if let Some(prog) = catalog.get(GENERAL_PROGRAM) {
        for bucket in prog.points.values() {
            for name in bucket.keys() {
                names.insert(name.clone());
            }
        }
    }
    names
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

/// Default / hover / editing colors for on-screen temp-point markers.
const MARKER_DEFAULT: egui::Color32 = crate::theme::PRIMARY;
const MARKER_HOVER: egui::Color32 = crate::theme::MACRO_START;
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
        if !pt.save {
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

    const CROSS: f32 = 18.0;
    const ARM: f32 = 7.0;
    const LABEL_H: f32 = 16.0;
    const PAD: f32 = 4.0;

    let label = if pt.name.trim().is_empty() {
        format!("#{index}")
    } else {
        pt.name.trim().to_string()
    };
    let font = FontId::proportional(11.0);
    let galley = ctx.fonts_mut(|f| f.layout_no_wrap(label.clone(), font.clone(), color));
    let label_w = galley.size().x.max(CROSS);
    let outer_w = label_w + PAD * 2.0;
    let outer_h = LABEL_H + CROSS + PAD;
    // Position so the crosshair center sits on (x, y).
    let pos = Pos2::new(
        pt.x as f32 - outer_w * 0.5,
        pt.y as f32 - (LABEL_H + CROSS * 0.5),
    );

    let id = ViewportId::from_hash_of(format!("sqyre_temp_point_marker_{index}"));
    let builder = ViewportBuilder::default()
        .with_title(OVERLAY_WM_TITLE)
        .with_decorations(false)
        .with_resizable(false)
        .with_always_on_top()
        .with_taskbar(false)
        .with_mouse_passthrough(true)
        .with_window_type(egui::X11WindowType::Dock)
        .with_transparent(true)
        .with_inner_size([outer_w, outer_h])
        .with_min_inner_size([outer_w, outer_h])
        .with_position(pos);

    ctx.show_viewport_deferred(id, builder, move |ui, _class| {
        let fill = crate::theme::overlay_panel_fill();
        Frame::NONE
            .fill(fill)
            .stroke(Stroke::new(1.0, color))
            .corner_radius(egui::CornerRadius::same(3))
            .inner_margin(Margin::ZERO)
            .show(ui, |ui| {
                ui.set_min_size(Vec2::new(outer_w, outer_h));
                let (rect, _) = ui.allocate_exact_size(Vec2::new(outer_w, outer_h), Sense::hover());
                let painter = ui.painter();
                let galley = painter.layout_no_wrap(label.clone(), font.clone(), color);
                let label_pos =
                    Pos2::new(rect.center().x - galley.size().x * 0.5, rect.top() + 1.0);
                painter.galley(label_pos, galley, color);
                let c = Pos2::new(rect.center().x, rect.top() + LABEL_H + CROSS * 0.5);
                let stroke = Stroke::new(2.0, color);
                painter.line_segment(
                    [Pos2::new(c.x - ARM, c.y), Pos2::new(c.x + ARM, c.y)],
                    stroke,
                );
                painter.line_segment(
                    [Pos2::new(c.x, c.y - ARM), Pos2::new(c.x, c.y + ARM)],
                    stroke,
                );
                painter.circle_stroke(c, 3.0, stroke);
            });
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
