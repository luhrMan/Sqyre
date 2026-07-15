//! egui shell: load macros from `~/.sqyre`, Run/Stop with live backends.

mod action_tooltip;
mod action_logs_ui;
mod assets;
mod catalog;
mod collection_capture;
mod data_editor;
mod icon_cache;
mod pickers;
mod preview_tooltip;
mod single_instance;
mod tray;
mod tree_chrome;
mod tree_dnd;
mod tree_history;

use action_logs_ui::LogsImageCache;
use action_tooltip::TooltipState;
use catalog::{CatalogIcons, CatalogResolver, SnapshotMacros};
use data_editor::DataEditor;
use eframe::egui;
use egui_ltreeview::{Action as TreeAction, NodeBuilder, TreeView, TreeViewBuilder};
use icon_cache::IconCache;
use preview_tooltip::PreviewTooltipCache;
use sqyre_capture::{X11Capturer, X11WindowFocuser};
use sqyre_domain::{Action, ActionId, InsertSlot, Macro};
use sqyre_executor::{
    execute_macro_with, ContinueKeyWaiter, ExecDeps, MatchFacade, OcrEngine, OcrResult,
    SharedActionLog, SharedHighlighter,
};
use sqyre_hotkeys::{default_hotkeys, ContinueWaitBridge, HotkeyCallbacks, HotkeyService, StopFlag};
use sqyre_input::OsAutomation;
use sqyre_match::ImageBuf;
use sqyre_persist::{variables_path, Database, ProgramCatalog};
use sqyre_vision::LeptessOcr;
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use tree_chrome::{RowAction, RowHighlight, RowInteraction};
use tree_history::TreeHistory;

struct BridgeContinueWait(ContinueWaitBridge);

struct AppOcr(LeptessOcr);

impl OcrEngine for AppOcr {
    fn recognize(&self, image: &ImageBuf) -> Result<OcrResult, String> {
        let r = self.0.recognize(image)?;
        Ok(OcrResult {
            text: r.text,
            words: r.words,
        })
    }
}

impl ContinueKeyWaiter for BridgeContinueWait {
    fn wait_for_continue(
        &self,
        keys: &[String],
        pass_through: bool,
        stop: &AtomicBool,
    ) -> Result<(), String> {
        self.0.wait_for_continue(keys, pass_through, stop)
    }
}

fn main() -> eframe::Result<()> {
    let _instance_lock = match single_instance::try_acquire() {
        Ok(Some(lock)) => lock,
        Ok(None) => {
            eprintln!("Sqyre is already running");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("failed to acquire instance lock: {e}");
            std::process::exit(1);
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 640.0])
            .with_title("Sqyre (Rust)")
            .with_icon(assets::app_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Sqyre",
        options,
        Box::new(|cc| {
            let mut app = SqyreApp::load();
            app.tray = tray::SystemTray::install(cc.egui_ctx.clone());
            Ok(Box::new(app))
        }),
    )
}

struct RunState {
    stop: StopFlag,
    running: Arc<AtomicBool>,
    status: Arc<Mutex<String>>,
}

impl Default for RunState {
    fn default() -> Self {
        Self {
            stop: StopFlag::new(),
            running: Arc::new(AtomicBool::new(false)),
            status: Arc::new(Mutex::new(String::new())),
        }
    }
}

struct SqyreApp {
    db: Database,
    macros: Vec<Macro>,
    catalog: ProgramCatalog,
    load_error: Option<String>,
    selected_macro: usize,
    selected_node: Option<u64>,
    /// ActionId selection restored after undo/redo (mapped to node id after tree build).
    selected_action: Option<ActionId>,
    node_actions: HashMap<u64, ActionId>,
    run: RunState,
    hotkeys: Box<dyn HotkeyService>,
    continue_wait: ContinueWaitBridge,
    action_log: SharedActionLog,
    highlighter: SharedHighlighter,
    /// Per-macro undo/redo stacks keyed by macro name.
    tree_histories: HashMap<String, TreeHistory>,
    logs_window: Option<ActionId>,
    logs_image_cache: LogsImageCache,
    icon_cache: IconCache,
    preview_tooltips: PreviewTooltipCache,
    tooltip: TooltipState,
    data_editor: DataEditor,
    tray: tray::SystemTray,
}

impl SqyreApp {
    fn load() -> Self {
        let (mut hotkeys, continue_wait) = default_hotkeys();
        let run = RunState::default();
        let stop = run.stop.clone();
        let _ = hotkeys.start(HotkeyCallbacks {
            on_escape_stop: Arc::new(move || stop.request_stop()),
            on_failsafe: Arc::new(|| {
                eprintln!("failsafe Esc+Ctrl+Shift — exiting");
                std::process::exit(0);
            }),
        });

        match Database::load_default() {
            Ok(db) => {
                let catalog = db.program_catalog().unwrap_or_default();
                let mut macros: Vec<_> = db.macros.values().cloned().collect();
                macros.sort_by(|a, b| a.name.cmp(&b.name));
                Self {
                    db,
                    macros,
                    catalog,
                    load_error: None,
                    selected_macro: 0,
                    selected_node: None,
                    selected_action: None,
                    node_actions: HashMap::new(),
                    run,
                    hotkeys,
                    continue_wait,
                    action_log: SharedActionLog::new(),
                    highlighter: SharedHighlighter::new(),
                    tree_histories: HashMap::new(),
                    logs_window: None,
                    logs_image_cache: LogsImageCache::default(),
                    icon_cache: IconCache::new(),
                    preview_tooltips: PreviewTooltipCache::new(),
                    tooltip: TooltipState::Hidden,
                    data_editor: DataEditor::default(),
                    tray: tray::SystemTray::default(),
                }
            }
            Err(e) => Self {
                db: Database::default(),
                macros: Vec::new(),
                catalog: ProgramCatalog::default(),
                load_error: Some(e.to_string()),
                selected_macro: 0,
                selected_node: None,
                selected_action: None,
                node_actions: HashMap::new(),
                run,
                hotkeys,
                continue_wait,
                action_log: SharedActionLog::new(),
                highlighter: SharedHighlighter::new(),
                tree_histories: HashMap::new(),
                logs_window: None,
                logs_image_cache: LogsImageCache::default(),
                icon_cache: IconCache::new(),
                preview_tooltips: PreviewTooltipCache::new(),
                tooltip: TooltipState::Hidden,
                data_editor: DataEditor::default(),
                tray: tray::SystemTray::default(),
            },
        }
    }

    fn selected_action_id(&self) -> Option<ActionId> {
        self.selected_action.or_else(|| {
            self.selected_node
                .and_then(|nid| self.node_actions.get(&nid).copied())
        })
    }

    fn record_tree_mutation(&mut self) {
        if self.macros.is_empty() {
            return;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let selected = self.selected_action_id();
        let name = self.macros[idx].name.clone();
        let Ok(snap) = TreeHistory::take_snapshot(&self.macros[idx].root, selected) else {
            return;
        };
        self.tree_histories
            .entry(name)
            .or_default()
            .push_snapshot(snap);
    }

    fn undo_tree(&mut self) {
        if self.macros.is_empty() {
            return;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let name = self.macros[idx].name.clone();
        let mut selected = self.selected_action_id();
        let mut history = self.tree_histories.remove(&name).unwrap_or_default();
        let ok = history.undo(&mut self.macros[idx].root, &mut selected);
        self.tree_histories.insert(name, history);
        if ok {
            self.selected_action = selected;
            self.selected_node = None;
            self.tooltip.cancel();
        }
    }

    fn redo_tree(&mut self) {
        if self.macros.is_empty() {
            return;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let name = self.macros[idx].name.clone();
        let mut selected = self.selected_action_id();
        let mut history = self.tree_histories.remove(&name).unwrap_or_default();
        let ok = history.redo(&mut self.macros[idx].root, &mut selected);
        self.tree_histories.insert(name, history);
        if ok {
            self.selected_action = selected;
            self.selected_node = None;
            self.tooltip.cancel();
        }
    }

    fn can_undo(&self) -> bool {
        if self.macros.is_empty() {
            return false;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        self.tree_histories
            .get(&self.macros[idx].name)
            .is_some_and(|h| h.can_undo())
    }

    fn can_redo(&self) -> bool {
        if self.macros.is_empty() {
            return false;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        self.tree_histories
            .get(&self.macros[idx].name)
            .is_some_and(|h| h.can_redo())
    }

    fn start_macro(&mut self, ctx: &egui::Context) {
        if self.macros.is_empty() || self.run.running.load(Ordering::SeqCst) {
            return;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let mut macro_ = self.macros[idx].clone();
        let catalog = self.catalog.clone();
        let stop_flag = self.run.stop.clone();
        stop_flag.clear();
        let running = Arc::clone(&self.run.running);
        let status = Arc::clone(&self.run.status);
        self.action_log.clear();
        self.logs_image_cache.clear();
        self.highlighter.clear_all();
        let action_log = self.action_log.clone();
        let highlighter = self.highlighter.clone();
        let continue_wait = BridgeContinueWait(self.continue_wait.clone());
        let macro_lookup = {
            let map: BTreeMap<String, Macro> =
                self.macros.iter().map(|m| (m.name.clone(), m.clone())).collect();
            SnapshotMacros(Arc::new(map))
        };
        let ctx = ctx.clone();
        running.store(true, Ordering::SeqCst);
        *status.lock().unwrap() = format!("Running {}…", macro_.name);

        thread::spawn(move || {
            let result = (|| -> Result<(), String> {
                let mut automation =
                    OsAutomation::new().map_err(|e| format!("automation: {e}"))?;
                let mut capturer = X11Capturer::open().map_err(|e| format!("capture: {e}"))?;
                let matcher = MatchFacade::new();
                let resolver = CatalogResolver(&catalog);
                let icons = CatalogIcons(&catalog);
                let focuser = X11WindowFocuser;
                let ocr_engine = LeptessOcr::from_env_or_system()
                    .map_err(|e| {
                        eprintln!("sqyre: {e}");
                        e
                    })
                    .ok()
                    .map(AppOcr);
                let stop_raw = stop_flag.raw();
                let mut watched = StopWatchAutomation {
                    inner: &mut automation,
                    stop: &stop_flag,
                };
                let vars_dir = variables_path();
                execute_macro_with(
                    &mut macro_,
                    ExecDeps {
                        automation: &mut watched,
                        capturer: Some(&mut capturer),
                        matcher: Some(&matcher),
                        resolver: Some(&resolver),
                        icons: Some(&icons),
                        macros: Some(&macro_lookup),
                        continue_waiter: Some(&continue_wait),
                        window_focuser: Some(&focuser),
                        ocr: ocr_engine.as_ref().map(|e| e as &dyn OcrEngine),
                        stop_flag: Some(stop_raw.as_ref()),
                        logger: Some(&action_log),
                        highlighter: Some(&highlighter),
                        variables_dir: Some(vars_dir.as_path()),
                    },
                )
                .map_err(|e| e.to_string())
            })();

            let msg = match result {
                Ok(()) if stop_flag.is_stopped() => "Stopped.".into(),
                Ok(()) => "Finished.".into(),
                Err(e) => format!("Error: {e}"),
            };
            *status.lock().unwrap() = msg;
            running.store(false, Ordering::SeqCst);
            ctx.request_repaint();
        });
    }

    fn request_stop(&mut self) {
        self.run.stop.request_stop();
        *self.run.status.lock().unwrap() = "Stop requested…".into();
    }

    fn action_display_name(&self, action_id: ActionId) -> String {
        if self.macros.is_empty() {
            return action_id.as_str();
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let root = &self.macros[idx].root;
        let action = if action_id.is_root() {
            Some(root)
        } else {
            root.find_by_id(action_id)
        };
        action
            .map(|a| a.display_name())
            .unwrap_or_else(|| action_id.as_str())
    }

    fn show_logs_window(&mut self, ctx: &egui::Context) {
        let Some(action_id) = self.logs_window else {
            return;
        };
        let title = format!("Logs — {}", self.action_display_name(action_id));
        if action_logs_ui::show_logs_window(
            ctx,
            action_id,
            &title,
            &self.action_log,
            &mut self.logs_image_cache,
        ) {
            self.logs_window = None;
        }
    }
}

/// Forwards automation but surfaces stop via milli_sleep / between calls by setting
/// a flag the executor polls — here we map stop into a short sleep so UI stays responsive.
struct StopWatchAutomation<'a> {
    inner: &'a mut OsAutomation,
    stop: &'a StopFlag,
}

impl sqyre_executor::AutomationBackend for StopWatchAutomation<'_> {
    fn milli_sleep(&mut self, ms: i32) {
        let mut left = ms.max(0);
        while left > 0 {
            if self.stop.is_stopped() {
                return;
            }
            let chunk = left.min(50);
            self.inner.milli_sleep(chunk);
            left -= chunk;
        }
    }
    fn move_to(&mut self, x: i32, y: i32, opts: sqyre_executor::MoveOptions) {
        if !self.stop.is_stopped() {
            self.inner.move_to(x, y, opts);
        }
    }
    fn click(&mut self, button: &str, down: bool) -> Result<(), String> {
        if self.stop.is_stopped() {
            return Ok(());
        }
        self.inner.click(button, down)
    }
    fn scroll(&mut self, up: bool) -> Result<(), String> {
        if self.stop.is_stopped() {
            return Ok(());
        }
        self.inner.scroll(up)
    }
    fn key_down(&mut self, key: &str) -> Result<(), String> {
        if self.stop.is_stopped() {
            return Ok(());
        }
        self.inner.key_down(key)
    }
    fn key_up(&mut self, key: &str) -> Result<(), String> {
        if self.stop.is_stopped() {
            return Ok(());
        }
        self.inner.key_up(key)
    }
    fn type_char(&mut self, s: &str) {
        if !self.stop.is_stopped() {
            self.inner.type_char(s);
        }
    }
    fn write_clipboard(&mut self, s: &str) -> Result<(), String> {
        if self.stop.is_stopped() {
            return Ok(());
        }
        self.inner.write_clipboard(s)
    }
}

impl eframe::App for SqyreApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Close → hide to tray when available; Quit from tray allows real exit.
        if self.tray.is_active()
            && !self.tray.quit_requested()
            && ui.ctx().input(|i| i.viewport().close_requested())
        {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        self.show_logs_window(ui.ctx());
        self.data_editor.show(
            ui.ctx(),
            &mut self.db,
            &mut self.macros,
            &mut self.catalog,
            &mut self.icon_cache,
            &mut self.preview_tooltips,
        );

        let running = self.run.running.load(Ordering::SeqCst);
        if running {
            ui.ctx().request_repaint();
        }

        // Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z — skip while editing an action.
        if !self.tooltip.is_editing() {
            let (undo, redo) = ui.ctx().input(|i| {
                let mod_key = i.modifiers.command;
                let undo = mod_key && !i.modifiers.shift && i.key_pressed(egui::Key::Z);
                let redo = mod_key
                    && (i.key_pressed(egui::Key::Y)
                        || (i.modifiers.shift && i.key_pressed(egui::Key::Z)));
                (undo, redo)
            });
            if undo {
                self.undo_tree();
            } else if redo {
                self.redo_tree();
            }
        }

        egui::Panel::left("macro_list")
            .default_size(220.0)
            .show_inside(ui, |ui| {
                ui.heading("Macros");
                if let Some(err) = &self.load_error {
                    ui.colored_label(egui::Color32::RED, format!("Load error: {err}"));
                } else {
                    ui.small(format!(
                        "{} from {}",
                        self.macros.len(),
                        sqyre_persist::db_path().display()
                    ));
                }
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (i, m) in self.macros.iter().enumerate() {
                        if ui
                            .selectable_label(self.selected_macro == i, &m.name)
                            .clicked()
                        {
                            self.selected_macro = i;
                            self.selected_node = None;
                            self.selected_action = None;
                            self.tooltip.cancel();
                        }
                    }
                });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("Sqyre");
            ui.horizontal(|ui| {
                let running = self.run.running.load(Ordering::SeqCst);
                if ui
                    .add_enabled(!running && !self.macros.is_empty(), egui::Button::new("Run"))
                    .clicked()
                {
                    self.start_macro(ui.ctx());
                }
                if ui
                    .add_enabled(running, egui::Button::new("Stop"))
                    .clicked()
                {
                    self.request_stop();
                }
                let can_undo = self.can_undo();
                let can_redo = self.can_redo();
                if ui
                    .add_enabled(can_undo && !running, egui::Button::new("Undo"))
                    .on_hover_text("Ctrl+Z")
                    .clicked()
                {
                    self.undo_tree();
                }
                if ui
                    .add_enabled(can_redo && !running, egui::Button::new("Redo"))
                    .on_hover_text("Ctrl+Y")
                    .clicked()
                {
                    self.redo_tree();
                }
                if ui.button("Data Editor").clicked() {
                    self.data_editor.open = true;
                }
                let status = self.run.status.lock().unwrap().clone();
                if !status.is_empty() {
                    ui.label(status);
                }
            });
            ui.small("Esc stops the running macro; Esc+Ctrl+Shift exits (failsafe).");
            ui.separator();

            if self.macros.is_empty() {
                ui.label("No macros loaded. Place a db.yaml under ~/.sqyre.");
                return;
            }
            let idx = self.selected_macro.min(self.macros.len() - 1);
            self.selected_macro = idx;

            let summary = {
                let m = &self.macros[idx];
                format!(
                    "{} — delay {}ms — {} tags",
                    m.name,
                    m.global_delay,
                    m.tags.len()
                )
            };
            ui.label(summary);
            ui.separator();

            let mut next_id = 0u64;
            let mut node_actions = HashMap::new();
            let mut open_logs: Option<ActionId> = None;
            let mut delete_action: Option<ActionId> = None;
            let mut row_events: Vec<(ActionId, RowInteraction)> = Vec::new();
            let is_dark = ui.visuals().dark_mode;
            let root_aid = self.macros[idx].root.id;
            let macro_name = self.macros[idx].name.clone();
            let hl_snap = self.highlighter.snapshot();
            let id = ui.make_persistent_id(("macro_tree", idx));
            let running = self.run.running.load(Ordering::SeqCst);
            let (_, actions) = {
                let catalog = &self.catalog;
                let icons = &mut self.icon_cache;
                let root_children = self.macros[idx].root.children();
                TreeView::new(id)
                    .allow_drag_and_drop(!running)
                    .show(ui, |builder: &mut TreeViewBuilder<'_, u64>| {
                        // Invisible flattened root so top-level rows have a parent for DnD
                        // (matches Go: root loop not painted).
                        let root_nid = next_id;
                        next_id += 1;
                        node_actions.insert(root_nid, root_aid);
                        builder.node(
                            NodeBuilder::dir(root_nid)
                                .flatten(true)
                                .drop_allowed(true)
                                .default_open(true),
                        );
                        for child in root_children {
                            build_tree(
                                builder,
                                child,
                                &mut next_id,
                                &mut node_actions,
                                &mut open_logs,
                                &mut delete_action,
                                &mut row_events,
                                catalog,
                                icons,
                                is_dark,
                                &macro_name,
                                &hl_snap,
                            );
                        }
                        builder.close_dir();
                    })
            };
            self.node_actions = node_actions;

            // Restore selection after undo/redo once node map is rebuilt.
            if let Some(aid) = self.selected_action {
                self.selected_node = self
                    .node_actions
                    .iter()
                    .find(|(_, a)| **a == aid)
                    .map(|(nid, _)| *nid);
            }

            if let Some(aid) = open_logs {
                self.logs_window = Some(aid);
            }
            if let Some(aid) = delete_action {
                if !aid.is_root() {
                    self.record_tree_mutation();
                    let cleared_sel = self.selected_action_id() == Some(aid);
                    let _ = self.macros[idx].root.remove_by_id(aid);
                    if cleared_sel {
                        self.selected_node = None;
                        self.selected_action = None;
                    }
                    if self.logs_window == Some(aid) {
                        self.logs_window = None;
                        self.logs_image_cache.clear();
                    }
                    if self.tooltip.action_id() == Some(aid) {
                        self.tooltip.cancel();
                    }
                }
            }

            let pointer = ui.ctx().pointer_interact_pos();
            let mut any_view_hover = false;
            // Prefer edit-open from any row; otherwise last hovered wins for view.
            for (aid, interaction) in &row_events {
                if interaction.hovered {
                    any_view_hover = true;
                }
                if let Some(action) = self.macros[idx].root.find_by_id(*aid) {
                    let action = action.clone();
                    action_tooltip::ingest_row(
                        &mut self.tooltip,
                        &action,
                        *interaction,
                        pointer,
                    );
                }
            }
            action_tooltip::end_hover_pass(&mut self.tooltip, any_view_hover);

            {
                let selected = self.selected_action_id();
                let name = self.macros[idx].name.clone();
                let catalog = &self.catalog;
                let icons = &mut self.icon_cache;
                let previews = &mut self.preview_tooltips;
                let macro_names: Vec<String> =
                    self.macros.iter().map(|m| m.name.clone()).collect();
                // Snapshot before tooltip may mutate; record via borrow-split.
                let mut pending_record: Option<tree_history::TreeSnapshot> = None;
                {
                    let root = &mut self.macros[idx].root;
                    action_tooltip::show(
                        &mut self.tooltip,
                        ui.ctx(),
                        root,
                        catalog,
                        icons,
                        previews,
                        &macro_names,
                        is_dark,
                        |root_before| {
                            if pending_record.is_none() {
                                if let Ok(snap) =
                                    TreeHistory::take_snapshot(root_before, selected)
                                {
                                    pending_record = Some(snap);
                                }
                            }
                        },
                    );
                }
                if let Some(snap) = pending_record {
                    self.tree_histories
                        .entry(name)
                        .or_default()
                        .push_snapshot(snap);
                }
            }

            let mut pending_moves: Vec<(ActionId, ActionId, InsertSlot)> = Vec::new();
            for action in actions {
                match action {
                    TreeAction::SetSelected(sel) => {
                        self.selected_node = sel.into_iter().next();
                        self.selected_action = self
                            .selected_node
                            .and_then(|nid| self.node_actions.get(&nid).copied());
                    }
                    TreeAction::Move(dnd) => {
                        if running {
                            continue;
                        }
                        let Some(target_aid) = self.node_actions.get(&dnd.target).copied() else {
                            continue;
                        };
                        let Some(slot) =
                            tree_dnd::insert_slot_from_dir_position(dnd.position, &self.node_actions)
                        else {
                            continue;
                        };
                        for src_nid in &dnd.source {
                            if let Some(src_aid) = self.node_actions.get(src_nid).copied() {
                                pending_moves.push((src_aid, target_aid, slot));
                            }
                        }
                    }
                    TreeAction::Drag(dnd) => {
                        // Disallow dropping a node into itself / a descendant while dragging.
                        if let (Some(target_aid), Some(src_nid)) =
                            (self.node_actions.get(&dnd.target), dnd.source.first())
                        {
                            if let Some(src_aid) = self.node_actions.get(src_nid).copied() {
                                if tree_dnd::is_invalid_tree_drop(
                                    &self.macros[idx].root,
                                    src_aid,
                                    *target_aid,
                                ) {
                                    dnd.remove_drop_marker(ui);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            if !pending_moves.is_empty() {
                self.record_tree_mutation();
            }
            for (src, parent, slot) in pending_moves {
                let _ = self.macros[idx].root.move_action(src, parent, slot);
            }

            if let Some(nid) = self.selected_node {
                if let Some(aid) = self.node_actions.get(&nid).copied() {
                    let root = &self.macros[idx].root;
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
        });
    }
}

impl Drop for SqyreApp {
    fn drop(&mut self) {
        self.hotkeys.stop();
    }
}

fn build_tree(
    builder: &mut TreeViewBuilder<'_, u64>,
    action: &Action,
    next_id: &mut u64,
    map: &mut HashMap<u64, ActionId>,
    open_logs: &mut Option<ActionId>,
    delete_action: &mut Option<ActionId>,
    row_events: &mut Vec<(ActionId, RowInteraction)>,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    is_dark: bool,
    macro_name: &str,
    hl_snap: &sqyre_executor::HighlightSnapshot,
) {
    let id = *next_id;
    *next_id += 1;
    map.insert(id, action.id);
    let action_id = action.id;
    let highlight = row_highlight(macro_name, action_id, hl_snap);

    let mut handle_row = |ui: &mut egui::Ui,
                          open_logs: &mut Option<ActionId>,
                          delete_action: &mut Option<ActionId>,
                          row_events: &mut Vec<(ActionId, RowInteraction)>| {
        let interaction =
            tree_chrome::paint_action_row(ui, action, catalog, icons, is_dark, highlight);
        match interaction.action {
            RowAction::Logs => *open_logs = Some(action_id),
            RowAction::Delete => *delete_action = Some(action_id),
            RowAction::None => {}
        }
        row_events.push((action_id, interaction));
    };

    if action.is_branch() {
        let is_open = builder.node(NodeBuilder::dir(id).drop_allowed(true).label_ui(|ui| {
            handle_row(ui, open_logs, delete_action, row_events);
        }));
        if is_open {
            for child in action.children() {
                build_tree(
                    builder,
                    child,
                    next_id,
                    map,
                    open_logs,
                    delete_action,
                    row_events,
                    catalog,
                    icons,
                    is_dark,
                    macro_name,
                    hl_snap,
                );
            }
        }
        builder.close_dir();
    } else {
        builder.node(NodeBuilder::leaf(id).label_ui(|ui| {
            handle_row(ui, open_logs, delete_action, row_events);
        }));
    }
}

fn row_highlight(
    macro_name: &str,
    action_id: ActionId,
    snap: &sqyre_executor::HighlightSnapshot,
) -> RowHighlight {
    if snap.macro_name != macro_name {
        return RowHighlight::None;
    }
    if let Some(frac) = snap.fills.get(&action_id) {
        return RowHighlight::Fill(*frac as f32);
    }
    if snap.cursor == Some(action_id) {
        return RowHighlight::Cursor;
    }
    RowHighlight::None
}
