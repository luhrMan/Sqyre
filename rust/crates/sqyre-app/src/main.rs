//! egui shell: load macros from `~/.sqyre`, Run/Stop with live backends.

mod action_tooltip;
mod action_logs_ui;
mod assets;
mod catalog;
mod collection_capture;
mod data_editor;
mod file_dialogs;
mod hotkey_record;
mod icon_cache;
mod icon_variants;
mod macro_meta;
mod pickers;
mod preview_tooltip;
mod settings;
mod single_instance;
mod theme;
mod tray;
mod tree_chrome;
mod tree_clipboard;
mod tree_dnd;
mod tree_history;
mod var_pills;

use action_logs_ui::LogsImageCache;
use action_tooltip::TooltipState;
use catalog::{CatalogIcons, CatalogResolver, SnapshotMacros};
use data_editor::DataEditor;
use eframe::egui;
use egui_ltreeview::{
    Action as TreeAction, NodeBuilder, TreeView, TreeViewBuilder, TreeViewState,
};
use hotkey_record::HotkeyRecordUi;
use icon_cache::IconCache;
use macro_meta::{collect_all_macro_tags, MacroMetaUi};
use preview_tooltip::PreviewTooltipCache;
use settings::SettingsUi;
use sqyre_capture::{X11Capturer, X11WindowFocuser};
use sqyre_domain::{
    collect_known_variable_names, Action, ActionId, InsertSlot, Macro,
};
use sqyre_executor::{
    execute_macro_with, ContinueKeyWaiter, ExecDeps, MatchFacade, OcrEngine, OcrResult,
    SharedActionLog, SharedHighlighter,
};
use sqyre_hotkeys::{
    default_hotkeys, format_hotkey, ContinueWaitBridge, HotkeyCallbacks, HotkeyService,
    HotkeyTrigger, MacroHotkeyBinding, MacroHotkeyBridge, ScreenClickBridge, StopFlag,
};
use sqyre_input::OsAutomation;
use sqyre_match::ImageBuf;
use sqyre_persist::{variables_path, Database, ProgramCatalog, UserSettings};
use sqyre_vision::LeptessOcr;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use tree_chrome::{RowAction, RowHighlight, RowInteraction};
use tree_history::TreeHistory;

struct BridgeContinueWait {
    continue_wait: ContinueWaitBridge,
    macro_hotkeys: MacroHotkeyBridge,
}

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
        self.macro_hotkeys.suspend();
        let result = self
            .continue_wait
            .wait_for_continue(keys, pass_through, stop);
        self.macro_hotkeys.resume();
        result
    }

    fn wait_for_any_chord(
        &self,
        chords: &[Vec<String>],
        hold_repeat: &[bool],
        pass_through: bool,
        stop: &AtomicBool,
    ) -> Result<usize, String> {
        self.macro_hotkeys.suspend();
        let result = self.continue_wait.wait_for_any_chord(
            chords,
            hold_repeat,
            pass_through,
            stop,
        );
        self.macro_hotkeys.resume();
        result
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
            SettingsUi::install_fonts(&cc.egui_ctx);
            SettingsUi::apply_appearance(&cc.egui_ctx, app.settings_ui.settings());
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
    /// Currently selected action in the macro tree (also the egui tree node id).
    selected_action: Option<ActionId>,
    run: RunState,
    hotkeys: Box<dyn HotkeyService>,
    continue_wait: ContinueWaitBridge,
    screen_click: ScreenClickBridge,
    macro_hotkeys: MacroHotkeyBridge,
    /// Macro names requested by the hotkey thread (drained each frame).
    pending_hotkey_macros: Arc<Mutex<Vec<String>>>,
    hotkey_record: HotkeyRecordUi,
    macro_meta: MacroMetaUi,
    action_log: SharedActionLog,
    highlighter: SharedHighlighter,
    /// Branches that were collapsed before execution expand (Go `preExecClosedBranches`).
    pre_exec_closed: HashSet<ActionId>,
    /// True while branches are force-opened for the active run (Go `execFullyExpanded`).
    exec_fully_expanded: bool,
    /// Last action scrolled into view for execution highlight follow.
    last_exec_follow: Option<ActionId>,
    /// Per-macro undo/redo stacks keyed by macro name.
    tree_histories: HashMap<String, TreeHistory>,
    /// Process-local action clipboard (YAML map without UIDs).
    action_clipboard: Option<serde_yaml::Mapping>,
    logs_window: Option<ActionId>,
    logs_image_cache: LogsImageCache,
    icon_cache: IconCache,
    preview_tooltips: PreviewTooltipCache,
    tooltip: TooltipState,
    data_editor: DataEditor,
    settings_ui: SettingsUi,
    /// Window was hidden because a point/search-area recording is armed.
    hidden_for_recording: bool,
    /// Left macro-list side panel visibility.
    macro_list_open: bool,
    tray: tray::SystemTray,
}

impl SqyreApp {
    fn load() -> Self {
        let settings = UserSettings::load_default().unwrap_or_else(|e| {
            eprintln!("sqyre: failed to load settings: {e}");
            UserSettings::default()
        });
        settings.apply_sqyre_dir_override();
        SettingsUi::apply_action_colors(&settings);

        let (mut hotkeys, continue_wait, screen_click, macro_hotkeys) = default_hotkeys();
        let run = RunState::default();
        let stop = run.stop.clone();
        let pending_hotkey_macros = Arc::new(Mutex::new(Vec::new()));
        let pending_for_cb = Arc::clone(&pending_hotkey_macros);
        let _ = hotkeys.start(HotkeyCallbacks {
            on_escape_stop: Arc::new(move || stop.request_stop()),
            on_failsafe: Arc::new(|| {
                eprintln!("failsafe Esc+Ctrl+Shift — exiting");
                std::process::exit(0);
            }),
            on_macro_hotkey: Arc::new(move |name| {
                if let Ok(mut q) = pending_for_cb.lock() {
                    q.push(name);
                }
            }),
        });

        let highlighter = SharedHighlighter::new();
        highlighter.set_enabled(settings.highlight_active_action);
        let settings_ui = SettingsUi::from_settings(settings);

        match Database::load_default() {
            Ok(db) => {
                let catalog = db.program_catalog().unwrap_or_default();
                let mut macros: Vec<_> = db.macros.values().cloned().collect();
                macros.sort_by(|a, b| a.name.cmp(&b.name));
                let app = Self {
                    db,
                    macros,
                    catalog,
                    load_error: None,
                    selected_macro: 0,
                    selected_action: None,
                    run,
                    hotkeys,
                    continue_wait,
                    screen_click,
                    macro_hotkeys,
                    pending_hotkey_macros,
                    hotkey_record: HotkeyRecordUi::default(),
                    macro_meta: MacroMetaUi::default(),
                    action_log: SharedActionLog::new(),
                    highlighter,
                    pre_exec_closed: HashSet::new(),
                    exec_fully_expanded: false,
                    last_exec_follow: None,
                    tree_histories: HashMap::new(),
                    action_clipboard: None,
                    logs_window: None,
                    logs_image_cache: LogsImageCache::default(),
                    icon_cache: IconCache::new(),
                    preview_tooltips: PreviewTooltipCache::new(),
                    tooltip: TooltipState::Hidden,
                    data_editor: DataEditor::default(),
                    settings_ui,
                    hidden_for_recording: false,
                    macro_list_open: true,
                    tray: tray::SystemTray::default(),
                };
                app.refresh_macro_hotkey_bindings();
                app
            }
            Err(e) => Self {
                db: Database::default(),
                macros: Vec::new(),
                catalog: ProgramCatalog::default(),
                load_error: Some(e.to_string()),
                selected_macro: 0,
                selected_action: None,
                run,
                hotkeys,
                continue_wait,
                screen_click,
                macro_hotkeys,
                pending_hotkey_macros,
                hotkey_record: HotkeyRecordUi::default(),
                macro_meta: MacroMetaUi::default(),
                action_log: SharedActionLog::new(),
                highlighter,
                pre_exec_closed: HashSet::new(),
                exec_fully_expanded: false,
                last_exec_follow: None,
                tree_histories: HashMap::new(),
                action_clipboard: None,
                logs_window: None,
                logs_image_cache: LogsImageCache::default(),
                icon_cache: IconCache::new(),
                preview_tooltips: PreviewTooltipCache::new(),
                tooltip: TooltipState::Hidden,
                data_editor: DataEditor::default(),
                settings_ui,
                hidden_for_recording: false,
                macro_list_open: true,
                tray: tray::SystemTray::default(),
            },
        }
    }

    fn selected_action_id(&self) -> Option<ActionId> {
        self.selected_action
    }

    fn refresh_macro_hotkey_bindings(&self) {
        let bindings = self
            .macros
            .iter()
            .filter(|m| !m.hotkey.is_empty())
            .map(|m| {
                MacroHotkeyBinding::new(
                    m.name.clone(),
                    m.hotkey.clone(),
                    HotkeyTrigger::parse(&m.hotkey_trigger),
                )
            })
            .collect();
        self.macro_hotkeys.set_bindings(bindings);
    }

    fn persist_macro_at(&mut self, idx: usize) {
        if idx >= self.macros.len() {
            return;
        }
        let m = self.macros[idx].clone();
        self.db.macros.insert(m.name.clone(), m);
        if let Err(e) = self.db.save_default() {
            eprintln!("sqyre: save macro: {e}");
        }
        self.refresh_macro_hotkey_bindings();
    }

    /// Rename the selected macro, drop the old db key, and rewrite Run Macro refs.
    fn rename_selected_macro(&mut self, new_name: String) {
        if self.macros.is_empty() {
            return;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let old_name = self.macros[idx].name.clone();
        if old_name == new_name {
            return;
        }

        self.macros[idx].name = new_name.clone();
        for m in &mut self.macros {
            m.rename_macro_reference(&old_name, &new_name);
        }
        if let Some(hist) = self.tree_histories.remove(&old_name) {
            self.tree_histories.insert(new_name.clone(), hist);
        }
        self.db.macros.remove(&old_name);
        self.db.replace_macros(self.macros.iter().cloned());
        if let Err(e) = self.db.save_default() {
            eprintln!("sqyre: rename macro: {e}");
        }
        self.refresh_macro_hotkey_bindings();

        self.macros.sort_by(|a, b| a.name.cmp(&b.name));
        if let Some(i) = self.macros.iter().position(|m| m.name == new_name) {
            self.selected_macro = i;
        }
        self.macro_meta.sync_selection(self.selected_macro, &self.macros[self.selected_macro]);
    }

    fn apply_hotkey_to_selected(
        &mut self,
        chord: Vec<String>,
        trigger: Option<HotkeyTrigger>,
    ) {
        if self.macros.is_empty() {
            return;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let trigger = trigger.unwrap_or_else(|| {
            HotkeyTrigger::parse(&self.macros[idx].hotkey_trigger)
        });
        let binding = MacroHotkeyBinding::new(self.macros[idx].name.clone(), chord, trigger);
        self.macros[idx].hotkey = binding.chord;
        self.macros[idx].hotkey_trigger = trigger.as_str().to_string();
        self.persist_macro_at(idx);
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

    fn can_copy_selection(&self) -> bool {
        if self.macros.is_empty() {
            return false;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let Some(aid) = self.selected_action.filter(|a| !a.is_root()) else {
            return false;
        };
        self.macros[idx].root.find_by_id(aid).is_some()
    }

    fn can_paste_clipboard(&self) -> bool {
        self.action_clipboard.is_some() && !self.macros.is_empty()
    }

    fn copy_selection(&mut self) -> bool {
        if self.macros.is_empty() {
            return false;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let Some(aid) = self.selected_action.filter(|a| !a.is_root()) else {
            return false;
        };
        let Some(action) = self.macros[idx].root.find_by_id(aid) else {
            return false;
        };
        let Ok(map) = sqyre_serialize::action_to_map(action) else {
            return false;
        };
        self.action_clipboard = Some(map);
        true
    }

    fn paste_clipboard(&mut self) -> bool {
        if self.macros.is_empty() {
            return false;
        }
        let Some(clip) = self.action_clipboard.clone() else {
            return false;
        };
        let Ok(new_action) = sqyre_serialize::action_from_map(&clip) else {
            return false;
        };
        let new_id = new_action.id;
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let selected = self.selected_action_id();
        let Some((parent, slot)) =
            tree_clipboard::insert_location_below_selection(&self.macros[idx].root, selected)
        else {
            return false;
        };
        self.record_tree_mutation();
        if self.macros[idx]
            .root
            .insert_at(parent, slot, new_action)
            .is_err()
        {
            return false;
        }
        self.selected_action = Some(new_id);
        self.tooltip.cancel();
        true
    }

    fn cut_selection(&mut self) -> bool {
        if !self.copy_selection() {
            return false;
        }
        if self.macros.is_empty() {
            return false;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let Some(aid) = self.selected_action.filter(|a| !a.is_root()) else {
            return false;
        };
        self.record_tree_mutation();
        let _ = self.macros[idx].root.remove_by_id(aid);
        self.selected_action = None;
        if self.logs_window == Some(aid) {
            self.logs_window = None;
            self.logs_image_cache.clear();
        }
        self.tooltip.cancel();
        true
    }

    fn start_macro(&mut self, ctx: &egui::Context) {
        if self.macros.is_empty() {
            return;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let name = self.macros[idx].name.clone();
        self.start_macro_by_name(&name, ctx);
    }

    fn start_macro_by_name(&mut self, name: &str, ctx: &egui::Context) {
        if self.run.running.load(Ordering::SeqCst) {
            return;
        }
        let Some(idx) = self.macros.iter().position(|m| m.name == name) else {
            return;
        };
        // Show the running macro's tree so highlight overlays have matching rows.
        self.selected_macro = idx;
        let mut macro_ = self.macros[idx].clone();
        let catalog = self.catalog.clone();
        let stop_flag = self.run.stop.clone();
        stop_flag.clear();
        let running = Arc::clone(&self.run.running);
        let status = Arc::clone(&self.run.status);
        self.action_log.clear();
        self.logs_image_cache.clear();
        self.highlighter.clear_all();
        self.last_exec_follow = None;
        // Expand happens on the next UI frame via `sync_execution_expand`.
        let action_log = self.action_log.clone();
        let highlighter = self.highlighter.clone();
        let continue_wait = BridgeContinueWait {
            continue_wait: self.continue_wait.clone(),
            macro_hotkeys: self.macro_hotkeys.clone(),
        };
        let close_matches = self
            .settings_ui
            .settings()
            .image_search_close_matches_distance;
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
                let matcher = MatchFacade {
                    close_matches_distance: close_matches,
                };
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

    fn drain_pending_hotkey_macros(&mut self, ctx: &egui::Context) {
        let pending: Vec<String> = self
            .pending_hotkey_macros
            .lock()
            .map(|mut q| std::mem::take(&mut *q))
            .unwrap_or_default();
        for name in pending {
            self.start_macro_by_name(&name, ctx);
        }
    }

    fn request_stop(&mut self) {
        self.run.stop.request_stop();
        *self.run.status.lock().unwrap() = "Stop requested…".into();
    }

    /// Hide the main window while a screen-click recording is armed (Go hide-during-recording).
    fn update_recording_visibility(&mut self, ctx: &egui::Context) {
        let should_hide = self.settings_ui.settings().hide_app_during_recording
            && self.screen_click.is_armed();
        if should_hide && !self.hidden_for_recording {
            self.hidden_for_recording = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        } else if !should_hide && self.hidden_for_recording {
            self.hidden_for_recording = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
    }

    /// Always-on-top HUD so live coordinates stay visible when the main window is hidden.
    fn show_recording_hud(&self, ctx: &egui::Context) {
        let Some(msg) = self.screen_click.status_label() else {
            return;
        };
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("sqyre_recording_hud"),
            egui::ViewportBuilder::default()
                .with_title("Recording")
                .with_inner_size([520.0, 56.0])
                .with_resizable(false)
                .with_always_on_top()
                .with_decorations(true),
            |ctx, _class| {
                // Root panel for an immediate viewport (no parent `Ui`).
                #[allow(deprecated)]
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.colored_label(theme::PRIMARY, msg.as_str());
                });
                ctx.request_repaint();
            },
        );
        ctx.request_repaint();
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
            &self.screen_click,
        );
        self.settings_ui.show(
            ui.ctx(),
            &mut self.db,
            &mut self.macros,
            &mut self.catalog,
        );
        // Keep highlighter enable flag in sync with the preference.
        let highlight_on = self.settings_ui.settings().highlight_active_action;
        if self.highlighter.is_enabled() != highlight_on {
            self.highlighter.set_enabled(highlight_on);
        }
        if self.settings_ui.reload_requested {
            self.settings_ui.reload_requested = false;
            self.selected_macro = 0;
            self.selected_action = None;
            self.tree_histories.clear();
            self.tooltip.cancel();
            self.icon_cache = IconCache::new();
            self.preview_tooltips = PreviewTooltipCache::new();
            self.refresh_macro_hotkey_bindings();
        }
        self.update_recording_visibility(ui.ctx());
        self.show_recording_hud(ui.ctx());
        self.drain_pending_hotkey_macros(ui.ctx());

        if let Some(chord) = self.hotkey_record.show(ui.ctx(), &self.macro_hotkeys) {
            self.apply_hotkey_to_selected(chord, None);
        }

        let running = self.run.running.load(Ordering::SeqCst);
        if running || self.hotkey_record.is_open() || self.screen_click.is_armed() {
            ui.ctx().request_repaint();
        }

        // Ctrl+C / Ctrl+X / Ctrl+V / Ctrl+Z / Ctrl+Y — skip while editing an action.
        if !self.tooltip.is_editing() && !self.hotkey_record.is_open() {
            let (copy, cut, paste, undo, redo) = ui.ctx().input(|i| {
                let mod_key = i.modifiers.command;
                let copy = mod_key && i.key_pressed(egui::Key::C);
                let cut = mod_key && i.key_pressed(egui::Key::X);
                let paste = mod_key && i.key_pressed(egui::Key::V);
                let undo = mod_key && !i.modifiers.shift && i.key_pressed(egui::Key::Z);
                let redo = mod_key
                    && (i.key_pressed(egui::Key::Y)
                        || (i.modifiers.shift && i.key_pressed(egui::Key::Z)));
                (copy, cut, paste, undo, redo)
            });
            if cut {
                self.cut_selection();
            } else if copy {
                self.copy_selection();
            } else if paste {
                self.paste_clipboard();
            } else if undo {
                self.undo_tree();
            } else if redo {
                self.redo_tree();
            }
        }

        egui::Panel::left("macro_list")
            .default_size(220.0)
            .show_animated_inside(ui, self.macro_list_open, |ui| {
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
                    let width = ui.available_width();
                    let text_width =
                        (width - ui.spacing().button_padding.x * 2.0).max(0.0);
                    for (i, m) in self.macros.iter().enumerate() {
                        let label = macro_list_item_text(ui, m, text_width);
                        if ui
                            .add(
                                egui::Button::selectable(self.selected_macro == i, label)
                                    .wrap_mode(egui::TextWrapMode::Extend)
                                    .min_size(egui::vec2(width, 0.0)),
                            )
                            .clicked()
                        {
                            self.selected_macro = i;
                            self.selected_action = None;
                            self.tooltip.cancel();
                        }
                    }
                });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("Sqyre");
            let mut force_openness: Option<bool> = None;
            let running = self.run.running.load(Ordering::SeqCst);
            ui.horizontal(|ui| {
                let (list_glyph, list_tip) = if self.macro_list_open {
                    ("◁", "Hide macro list")
                } else {
                    ("☰", "Show macro list")
                };
                if toolbar_icon(ui, list_glyph, list_tip, true).clicked() {
                    self.macro_list_open = !self.macro_list_open;
                }
                ui.separator();
                if toolbar_icon(ui, "▶", "Run", !running && !self.macros.is_empty()).clicked() {
                    self.start_macro(ui.ctx());
                }
                if toolbar_icon(ui, "⏹", "Stop", running).clicked() {
                    self.request_stop();
                }
                if toolbar_icon(ui, "📁", "Data Editor", true).clicked() {
                    self.data_editor.open = true;
                }
                if toolbar_icon(ui, "⚙", "Settings", true).clicked() {
                    self.settings_ui.open = true;
                }
                let status = self.run.status.lock().unwrap().clone();
                if !status.is_empty() {
                    ui.label(status);
                }
            });
            ui.small("Esc stops the running macro; Esc+Ctrl+Shift exits (failsafe). Macro hotkeys launch from anywhere.");
            ui.separator();

            if self.macros.is_empty() {
                ui.label("No macros loaded. Place a db.yaml under ~/.sqyre.");
                return;
            }
            let idx = self.selected_macro.min(self.macros.len() - 1);
            self.selected_macro = idx;
            let meta_enabled = !running;
            self.macro_meta
                .sync_selection(idx, &self.macros[idx]);
            let other_names: Vec<String> =
                self.macros.iter().map(|m| m.name.clone()).collect();
            let all_tags = collect_all_macro_tags(&self.macros);
            let meta = {
                let m = &mut self.macros[idx];
                self.macro_meta
                    .show(ui, m, &other_names, &all_tags, meta_enabled)
            };
            if let Some(new_name) = meta.rename_to {
                self.rename_selected_macro(new_name);
            } else if meta.persist {
                self.persist_macro_at(idx);
            }
            // Selection / length may have changed after rename.
            let idx = self.selected_macro.min(self.macros.len().saturating_sub(1));
            self.selected_macro = idx;
            if self.macros.is_empty() {
                return;
            }

            ui.horizontal(|ui| {
                ui.label("Hotkey:");
                let hk_label = {
                    let m = &self.macros[idx];
                    if m.hotkey.is_empty() {
                        "—".to_string()
                    } else {
                        format_hotkey(&m.hotkey)
                    }
                };
                ui.monospace(&hk_label);

                let mut trigger = HotkeyTrigger::parse(&self.macros[idx].hotkey_trigger);
                let mut trigger_changed = false;
                if ui
                    .selectable_label(trigger == HotkeyTrigger::Press, "On press")
                    .clicked()
                {
                    trigger = HotkeyTrigger::Press;
                    trigger_changed = true;
                }
                if ui
                    .selectable_label(trigger == HotkeyTrigger::Release, "On release")
                    .clicked()
                {
                    trigger = HotkeyTrigger::Release;
                    trigger_changed = true;
                }
                if trigger_changed {
                    let chord = self.macros[idx].hotkey.clone();
                    self.apply_hotkey_to_selected(chord, Some(trigger));
                }

                if theme::record_icon_button(ui, "Record a global hotkey chord", !running)
                    .clicked()
                {
                    self.hotkey_record.open(&self.macro_hotkeys);
                }
                if ui
                    .add_enabled(
                        !running && !self.macros[idx].hotkey.is_empty(),
                        egui::Button::new("Clear"),
                    )
                    .clicked()
                {
                    self.apply_hotkey_to_selected(Vec::new(), None);
                }
            });
            ui.separator();

            ui.horizontal(|ui| {
                let can_copy = self.can_copy_selection();
                let can_paste = self.can_paste_clipboard();
                let can_undo = self.can_undo();
                let can_redo = self.can_redo();
                if toolbar_icon(ui, "📄", "Copy (Ctrl+C)", can_copy && !running).clicked() {
                    self.copy_selection();
                }
                if toolbar_icon(ui, "✂", "Cut (Ctrl+X)", can_copy && !running).clicked() {
                    self.cut_selection();
                }
                if toolbar_icon(ui, "📋", "Paste (Ctrl+V)", can_paste && !running).clicked() {
                    self.paste_clipboard();
                }
                if toolbar_icon(ui, "↺", "Undo (Ctrl+Z)", can_undo && !running).clicked() {
                    self.undo_tree();
                }
                if toolbar_icon(ui, "↻", "Redo (Ctrl+Y)", can_redo && !running).clicked() {
                    self.redo_tree();
                }
                if toolbar_icon(ui, "⬇⬇", "Expand all branches", true).clicked() {
                    force_openness = Some(true);
                }
                if toolbar_icon(ui, "⬆⬆", "Collapse all branches", true).clicked() {
                    force_openness = Some(false);
                }
            });

            let mut open_logs: Option<ActionId> = None;
            let mut delete_action: Option<ActionId> = None;
            let mut row_events: Vec<(ActionId, RowInteraction)> = Vec::new();
            let is_dark = ui.visuals().dark_mode;
            let root_aid = self.macros[idx].root.id;
            let macro_name = self.macros[idx].name.clone();
            let hl_snap = self.highlighter.snapshot();
            let id = ui.make_persistent_id(("macro_tree", idx));
            let mut state = TreeViewState::<ActionId>::load(ui, id).unwrap_or_default();
            if let Some(open) = force_openness {
                set_all_branches_openness(&self.macros[idx].root, &mut state, open);
            }
            sync_execution_expand(
                running,
                &self.macros[idx].root,
                &mut state,
                &mut self.exec_fully_expanded,
                &mut self.pre_exec_closed,
                &mut self.last_exec_follow,
            );
            let follow = highlight_follow_target(&hl_snap);
            let scroll_to = follow.filter(|id| self.last_exec_follow != Some(*id));
            let mut scrolled_follow = false;
            let actions = egui::ScrollArea::vertical()
                .id_salt("macro_tree_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let catalog = &self.catalog;
                    let icons = &mut self.icon_cache;
                    let root = &self.macros[idx].root;
                    let root_children = root.children();
                    let known_vars = collect_known_variable_names(&self.macros[idx]);
                    let (_, tree_actions) = TreeView::new(id)
                        .allow_drag_and_drop(!running)
                        .show_state(ui, &mut state, |builder: &mut TreeViewBuilder<'_, ActionId>| {
                            // Invisible flattened root so top-level rows have a parent for DnD
                            // (matches Go: root loop not painted).
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
                                    catalog,
                                    icons,
                                    &known_vars,
                                    is_dark,
                                    &macro_name,
                                    &hl_snap,
                                    scroll_to,
                                    &mut scrolled_follow,
                                );
                            }
                            builder.close_dir();
                        });
                    // Off-clip rows skip label_ui — estimate Y so ScrollArea can still follow.
                    if let Some(target) = scroll_to {
                        if !scrolled_follow {
                            if let Some(row_i) = flattened_visible_index(root, target) {
                                let row_h = ui.spacing().interact_size.y
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
                })
                .inner;
            if scrolled_follow {
                self.last_exec_follow = follow;
            }
            state.store(ui, id);

            if let Some(aid) = open_logs {
                self.logs_window = Some(aid);
            }
            if let Some(aid) = delete_action {
                if !aid.is_root() {
                    self.record_tree_mutation();
                    let cleared_sel = self.selected_action_id() == Some(aid);
                    let _ = self.macros[idx].root.remove_by_id(aid);
                    if cleared_sel {
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
            let tip_aid = self.tooltip.action_id();
            // Prefer edit-open from any row; otherwise last hovered wins for view.
            for (aid, interaction) in &row_events {
                if interaction.hovered {
                    any_view_hover = true;
                }
                // Keep open while pointer remains over the tipped row even if a
                // constrained tooltip covers it and clears `.hovered()`.
                if tip_aid == Some(*aid) && interaction.pointer_in_row {
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
                let macros: Vec<(String, Vec<String>)> = self
                    .macros
                    .iter()
                    .map(|m| (m.name.clone(), m.tags.clone()))
                    .collect();
                // Snapshot before tooltip may mutate; record via borrow-split.
                let mut pending_record: Option<tree_history::TreeSnapshot> = None;
                let known_vars = collect_known_variable_names(&self.macros[idx]);
                {
                    let root = &mut self.macros[idx].root;
                    action_tooltip::show(
                        &mut self.tooltip,
                        ui.ctx(),
                        root,
                        catalog,
                        icons,
                        previews,
                        &macros,
                        &known_vars,
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
                        self.selected_action = sel.into_iter().next();
                    }
                    TreeAction::Move(dnd) => {
                        if running {
                            continue;
                        }
                        let target_aid = dnd.target;
                        let Some(slot) = tree_dnd::insert_slot_from_dir_position(dnd.position)
                        else {
                            continue;
                        };
                        for src_aid in &dnd.source {
                            pending_moves.push((*src_aid, target_aid, slot));
                        }
                    }
                    TreeAction::Drag(dnd) => {
                        // Disallow dropping a node into itself / a descendant while dragging.
                        let target_aid = dnd.target;
                        if let Some(src_aid) = dnd.source.first() {
                            if tree_dnd::is_invalid_tree_drop(
                                &self.macros[idx].root,
                                *src_aid,
                                target_aid,
                            ) {
                                dnd.remove_drop_marker(ui);
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

            if let Some(aid) = self.selected_action {
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
        });
    }
}

impl Drop for SqyreApp {
    fn drop(&mut self) {
        self.hotkeys.stop();
    }
}

/// Elide `text` to a single line that fits `max_width`, appending `…` only when needed.
fn elide_to_width(ui: &egui::Ui, text: &str, max_width: f32, font_id: egui::FontId) -> String {
    if text.is_empty() {
        return String::new();
    }
    let full = ui.painter().layout_no_wrap(
        text.to_owned(),
        font_id.clone(),
        egui::Color32::WHITE,
    );
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
        let mid = (lo + hi + 1) / 2;
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
fn macro_list_item_text(ui: &egui::Ui, m: &Macro, max_text_width: f32) -> egui::WidgetText {
    let style = ui.style();
    let name_font = egui::FontSelection::Default.resolve(style);
    let name = elide_to_width(ui, &m.name, max_text_width, name_font);

    if m.hotkey.is_empty() {
        return name.into();
    }

    let hotkey_font = egui::TextStyle::Small.resolve(style);
    let hotkey = elide_to_width(
        ui,
        &format_hotkey(&m.hotkey),
        max_text_width,
        hotkey_font,
    );

    let mut job = egui::text::LayoutJob::default();
    egui::RichText::new(name)
        .color(style.visuals.text_color())
        .append_to(
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

fn set_all_branches_openness(
    root: &Action,
    state: &mut TreeViewState<ActionId>,
    open: bool,
) {
    root.walk(&mut |action| {
        if action.is_branch() && !action.id.is_root() {
            state.set_openness(action.id, open);
        }
    });
}

/// Open every branch for the run (Go `beginExecutionExpand` / `endExecutionExpand`).
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
        }
    }
    walk(root, target, &mut index, &mut found);
    found
}

fn build_tree(
    builder: &mut TreeViewBuilder<'_, ActionId>,
    action: &Action,
    open_logs: &mut Option<ActionId>,
    delete_action: &mut Option<ActionId>,
    row_events: &mut Vec<(ActionId, RowInteraction)>,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    known_vars: &HashSet<String>,
    is_dark: bool,
    macro_name: &str,
    hl_snap: &sqyre_executor::HighlightSnapshot,
    scroll_to: Option<ActionId>,
    scrolled_follow: &mut bool,
) {
    let action_id = action.id;
    let highlight = row_highlight(macro_name, action_id, hl_snap);
    let should_scroll = scroll_to == Some(action_id);

    let mut handle_row = |ui: &mut egui::Ui,
                          open_logs: &mut Option<ActionId>,
                          delete_action: &mut Option<ActionId>,
                          row_events: &mut Vec<(ActionId, RowInteraction)>,
                          scrolled_follow: &mut bool| {
        let interaction = tree_chrome::paint_action_row(
            ui,
            action,
            catalog,
            icons,
            known_vars,
            is_dark,
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
        let is_open = builder.node(NodeBuilder::dir(action_id).drop_allowed(true).label_ui(|ui| {
            handle_row(ui, open_logs, delete_action, row_events, scrolled_follow);
        }));
        if is_open {
            for child in action.children() {
                build_tree(
                    builder,
                    child,
                    open_logs,
                    delete_action,
                    row_events,
                    catalog,
                    icons,
                    known_vars,
                    is_dark,
                    macro_name,
                    hl_snap,
                    scroll_to,
                    scrolled_follow,
                );
            }
        }
        builder.close_dir();
    } else {
        builder.node(NodeBuilder::leaf(action_id).label_ui(|ui| {
            handle_row(ui, open_logs, delete_action, row_events, scrolled_follow);
        }));
    }
}

/// Compact toolbar control: icon glyph + hover label (mirrors Go icon-only buttons).
fn toolbar_icon(ui: &mut egui::Ui, glyph: &str, tip: &str, enabled: bool) -> egui::Response {
    ui.add_enabled(
        enabled,
        egui::Button::new(egui::RichText::new(glyph).size(16.0)),
    )
    .on_hover_text(tip)
}

fn row_highlight(
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
        assert!(matches!(
            row_highlight("m", id, &snap),
            RowHighlight::None
        ));
    }
}
