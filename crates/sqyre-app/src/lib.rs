//! egui shell: load macros from `~/.sqyre`, Run/Stop with live backends.

mod action_tooltip;
mod action_logs_ui;
mod add_action;
mod assets;
mod catalog;
mod collection_capture;
mod data_editor;
mod diag;
pub mod docs_fixture;
mod file_dialogs;
mod hotkey_record;
mod icon_cache;
mod icon_variants;
mod image_view;
mod key_record;
mod macro_meta;
mod macro_overlay;
mod overlay_icons;
mod pickers;
mod pixel_color;
mod preview_tooltip;
mod recording_overlay;
mod settings;
mod single_instance;
pub mod theme;
mod tray;
mod tree_chrome;
mod tree_clipboard;
mod tree_dnd;
mod tree_history;
mod var_pills;
mod variables_panel;

pub use settings::SettingsUi;

use action_logs_ui::LogsImageCache;
use action_tooltip::TooltipState;
use add_action::AddActionPicker;
use catalog::{apply_main_monitor_resolution, CatalogIcons, CatalogResolver, SnapshotMacros};
use data_editor::DataEditor;
use eframe::egui;
use egui_ltreeview::{
    Action as TreeAction, NodeBuilder, TreeView, TreeViewBuilder, TreeViewState,
};
use hotkey_record::HotkeyRecordUi;
use icon_cache::IconCache;
use key_record::KeyRecordUi;
use macro_meta::{collect_all_macro_tags, MacroMetaUi};
use macro_overlay::MacroOverlay;
use preview_tooltip::PreviewTooltipCache;
use recording_overlay::RecordingOverlay;
use sqyre_capture::{X11Capturer, X11WindowFocuser};
use sqyre_domain::{
    collect_known_variable_names, Action, ActionId, InsertSlot, Macro,
};
use sqyre_executor::{
    execute_macro_with, ContinueKeyWaiter, ExecDeps, MatchFacade, OcrEngine, OcrResult,
    SharedActionLog, SharedHighlighter, SharedRuntimeVars,
};
use sqyre_hotkeys::{
    default_hotkeys, format_hotkey, ContinueWaitBridge, HotkeyCallbacks, HotkeyService,
    HotkeyTrigger, MacroHotkeyBinding, MacroHotkeyBridge, NullHotkeys, ScreenClickBridge,
    StopFlag,
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

/// Pointer gesture over the macro tree: reorder only from icon/pill handles;
/// dragging elsewhere scrolls the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum TreeDragMode {
    #[default]
    Idle,
    Reorder,
    Scroll,
}

/// Match egui `ScrollArea` kinetic scrolling (points / second).
const TREE_SCROLL_STOP_SPEED: f32 = 20.0;
/// Match egui `ScrollArea` friction (points / second²).
const TREE_SCROLL_FRICTION: f32 = 1000.0;

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

/// Launch the desktop shell (single-instance lock, tray, fonts).
pub fn run() -> eframe::Result<()> {
    let _ = sqyre_persist::initialize_directories();
    diag::install(sqyre_persist::sqyre_dir());

    let instance_lock = match single_instance::try_acquire() {
        Ok(Some(lock)) => Some(lock),
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
            .with_min_inner_size([100.0, 100.0])
            .with_title("Sqyre (Rust)")
            .with_icon(assets::app_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Sqyre",
        options,
        Box::new(move |cc| {
            let mut app = SqyreApp::load();
            app.instance_lock = instance_lock;
            SettingsUi::install_fonts(&cc.egui_ctx);
            SettingsUi::apply_appearance(&cc.egui_ctx, app.settings_ui.settings());
            app.bind_hotkey_repaint(cc.egui_ctx.clone());
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

pub struct SqyreApp {
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
    /// egui context for waking the UI when a hotkey queues a macro while idle/unfocused.
    hotkey_repaint: Arc<Mutex<Option<egui::Context>>>,
    hotkey_record: HotkeyRecordUi,
    key_record: KeyRecordUi,
    macro_meta: MacroMetaUi,
    action_log: SharedActionLog,
    runtime_vars: SharedRuntimeVars,
    highlighter: SharedHighlighter,
    /// Branches that were collapsed before execution expand.
    pre_exec_closed: HashSet<ActionId>,
    /// True while branches are force-opened for the active run.
    exec_fully_expanded: bool,
    /// Last action scrolled into view for execution highlight follow.
    last_exec_follow: Option<ActionId>,
    /// Prior-frame icon/pill rects; used to decide reorder vs drag-scroll.
    tree_drag_handles: Vec<egui::Rect>,
    /// Active pointer gesture on the macro tree (idle / reorder / drag-scroll).
    tree_drag_mode: TreeDragMode,
    /// Vertical coast velocity after a drag-scroll release (points/sec).
    tree_scroll_vel: f32,
    /// Per-macro undo/redo stacks keyed by macro name.
    tree_histories: HashMap<String, TreeHistory>,
    /// Process-local action clipboard (YAML map without UIDs).
    action_clipboard: Option<serde_yaml::Mapping>,
    logs_window: Option<ActionId>,
    logs_image_cache: LogsImageCache,
    icon_cache: IconCache,
    preview_tooltips: PreviewTooltipCache,
    tooltip: TooltipState,
    add_action_picker: AddActionPicker,
    data_editor: DataEditor,
    settings_ui: SettingsUi,
    variables_panel: variables_panel::VariablesPanelUi,
    /// Window was hidden because a point/search-area recording is armed.
    hidden_for_recording: bool,
    /// X11 outline windows for live search-area selection rect.
    recording_overlay: RecordingOverlay,
    /// Always-on-top floating buttons that start macros.
    macro_overlay: MacroOverlay,
    /// Left macro-list side panel visibility.
    macro_list_open: bool,
    /// Filter text for the macro list (name / tags fuzzy match).
    macro_list_filter: String,
    tray: tray::SystemTray,
    /// Process-wide data-dir lock (re-acquired after relocate).
    instance_lock: Option<single_instance::InstanceLock>,
    /// Confirm dialog for deleting the selected macro.
    pending_delete_macro: Option<String>,
}

impl SqyreApp {
    /// In-memory app for README / docs screenshots (no tray, disk, or real hotkeys).
    pub fn for_docs() -> Self {
        let settings = UserSettings::default();
        SettingsUi::apply_action_colors(&settings);

        let hotkeys: Box<dyn HotkeyService> = Box::new(NullHotkeys::default());
        let continue_wait = ContinueWaitBridge::new(false);
        let screen_click = ScreenClickBridge::new();
        let macro_hotkeys = MacroHotkeyBridge::new();
        let run = RunState::default();
        let pending_hotkey_macros = Arc::new(Mutex::new(Vec::new()));
        let hotkey_repaint = Arc::new(Mutex::new(None::<egui::Context>));

        let highlighter = SharedHighlighter::new();
        highlighter.set_enabled(settings.highlight_active_action);
        let settings_ui = SettingsUi::from_settings(settings);
        let action_log = SharedActionLog::new();
        action_log.set_log_images(settings_ui.settings().save_meta_images);
        let mut add_action_picker = AddActionPicker::default();
        add_action_picker.load_from_settings(settings_ui.settings());

        let catalog = docs_fixture::demo_catalog();
        let macro_ = docs_fixture::demo_macro();
        let macros = vec![macro_];
        let db = docs_fixture::demo_database(&macros, &catalog);

        let mut app = Self {
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
            hotkey_repaint,
            hotkey_record: HotkeyRecordUi::default(),
            key_record: KeyRecordUi::default(),
            macro_meta: MacroMetaUi::default(),
            action_log,
            runtime_vars: SharedRuntimeVars::new(),
            highlighter,
            pre_exec_closed: HashSet::new(),
            exec_fully_expanded: true,
            last_exec_follow: None,
            tree_drag_handles: Vec::new(),
            tree_drag_mode: TreeDragMode::Idle,
            tree_scroll_vel: 0.0,
            tree_histories: HashMap::new(),
            action_clipboard: None,
            logs_window: None,
            logs_image_cache: LogsImageCache::default(),
            icon_cache: IconCache::new(),
            preview_tooltips: PreviewTooltipCache::new(),
            tooltip: TooltipState::Hidden,
            add_action_picker,
            data_editor: DataEditor::default(),
            settings_ui,
            variables_panel: variables_panel::VariablesPanelUi::default(),
            hidden_for_recording: false,
            recording_overlay: RecordingOverlay::new(),
            macro_overlay: MacroOverlay::new(),
            macro_list_open: false,
            macro_list_filter: String::new(),
            tray: tray::SystemTray::default(),
            instance_lock: None,
            pending_delete_macro: None,
        };
        if let Some(m) = app.macros.first() {
            app.macro_meta.sync_selection(0, m);
        }
        app
    }

    pub fn open_add_action_picker(&mut self) {
        self.add_action_picker.open();
    }

    pub fn open_data_editor(&mut self) {
        self.data_editor.open = true;
        self.data_editor
            .select_program_for_docs("Demo Program", &self.catalog);
    }

    pub fn select_action(&mut self, id: ActionId) {
        self.selected_action = Some(id);
    }

    /// First top-level action under the demo macro root (skips the root loop).
    pub fn demo_first_action_id(&self) -> Option<ActionId> {
        self.macros
            .first()?
            .root
            .children()
            .first()
            .map(|a| a.id)
    }

    /// First image-search action in the selected macro tree.
    pub fn demo_image_search_id(&self) -> Option<ActionId> {
        let m = self.macros.get(self.selected_macro)?;
        let mut found = None;
        m.root.walk(&mut |a| {
            if found.is_none() && a.type_key() == "imagesearch" {
                found = Some(a.id);
            }
        });
        found
    }

    /// Force all branch nodes open (same as run-time expand).
    pub fn expand_all_branches_for_docs(&mut self) {
        self.exec_fully_expanded = true;
    }

    /// Settings used for docs appearance (fonts / scale).
    pub fn docs_settings(&self) -> &UserSettings {
        self.settings_ui.settings()
    }

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
        let hotkey_repaint = Arc::new(Mutex::new(None::<egui::Context>));
        let repaint_for_cb = Arc::clone(&hotkey_repaint);
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
                // Idle/unfocused eframe may not paint until woken — without this,
                // queued macros only start when Sqyre regains focus.
                if let Ok(guard) = repaint_for_cb.lock() {
                    if let Some(ctx) = guard.as_ref() {
                        ctx.request_repaint();
                    }
                }
            }),
        });

        let highlighter = SharedHighlighter::new();
        highlighter.set_enabled(settings.highlight_active_action);
        let settings_ui = SettingsUi::from_settings(settings);
        let action_log = SharedActionLog::new();
        action_log.set_log_images(settings_ui.settings().save_meta_images);
        let mut add_action_picker = AddActionPicker::default();
        add_action_picker.load_from_settings(settings_ui.settings());

        match Database::load_default() {
            Ok(db) => {
                let mut catalog = db.program_catalog().unwrap_or_default();
                apply_main_monitor_resolution(&mut catalog);
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
                    hotkey_repaint,
                    hotkey_record: HotkeyRecordUi::default(),
                    key_record: KeyRecordUi::default(),
                    macro_meta: MacroMetaUi::default(),
                    action_log,
                    runtime_vars: SharedRuntimeVars::new(),
                    highlighter,
                    pre_exec_closed: HashSet::new(),
                    exec_fully_expanded: false,
                    last_exec_follow: None,
                    tree_drag_handles: Vec::new(),
                    tree_drag_mode: TreeDragMode::Idle,
                    tree_scroll_vel: 0.0,
                    tree_histories: HashMap::new(),
                    action_clipboard: None,
                    logs_window: None,
                    logs_image_cache: LogsImageCache::default(),
                    icon_cache: IconCache::new(),
                    preview_tooltips: PreviewTooltipCache::new(),
                    tooltip: TooltipState::Hidden,
                    add_action_picker,
                    data_editor: DataEditor::default(),
                    settings_ui,
                    variables_panel: variables_panel::VariablesPanelUi::default(),
                    hidden_for_recording: false,
                    recording_overlay: RecordingOverlay::new(),
                    macro_overlay: MacroOverlay::new(),
                    macro_list_open: true,
                    macro_list_filter: String::new(),
                    tray: tray::SystemTray::default(),
                    instance_lock: None,
                    pending_delete_macro: None,
                };
                app.refresh_macro_hotkey_bindings();
                app
            }
            Err(e) => {
                let mut catalog = ProgramCatalog::default();
                apply_main_monitor_resolution(&mut catalog);
                Self {
                    db: Database::default(),
                    macros: Vec::new(),
                    catalog,
                    load_error: Some(e.to_string()),
                    selected_macro: 0,
                    selected_action: None,
                    run,
                    hotkeys,
                    continue_wait,
                    screen_click,
                    macro_hotkeys,
                    pending_hotkey_macros,
                    hotkey_repaint,
                    hotkey_record: HotkeyRecordUi::default(),
                    key_record: KeyRecordUi::default(),
                    macro_meta: MacroMetaUi::default(),
                    action_log,
                    runtime_vars: SharedRuntimeVars::new(),
                    highlighter,
                    pre_exec_closed: HashSet::new(),
                    exec_fully_expanded: false,
                    last_exec_follow: None,
                    tree_drag_handles: Vec::new(),
                    tree_drag_mode: TreeDragMode::Idle,
                    tree_scroll_vel: 0.0,
                    tree_histories: HashMap::new(),
                    action_clipboard: None,
                    logs_window: None,
                    logs_image_cache: LogsImageCache::default(),
                    icon_cache: IconCache::new(),
                    preview_tooltips: PreviewTooltipCache::new(),
                    tooltip: TooltipState::Hidden,
                    add_action_picker,
                    data_editor: DataEditor::default(),
                    settings_ui,
                    variables_panel: variables_panel::VariablesPanelUi::default(),
                    hidden_for_recording: false,
                    recording_overlay: RecordingOverlay::new(),
                    macro_overlay: MacroOverlay::new(),
                    macro_list_open: true,
                    macro_list_filter: String::new(),
                    tray: tray::SystemTray::default(),
                    instance_lock: None,
                    pending_delete_macro: None,
                }
            },
        }
    }

    /// Provide egui context so background hotkey fires can wake an idle UI frame.
    fn bind_hotkey_repaint(&self, ctx: egui::Context) {
        if let Ok(mut slot) = self.hotkey_repaint.lock() {
            *slot = Some(ctx);
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

    fn unique_macro_name(&self, base: &str) -> String {
        if !self.macros.iter().any(|m| m.name == base) {
            return base.to_string();
        }
        for i in 2.. {
            let candidate = format!("{base} {i}");
            if !self.macros.iter().any(|m| m.name == candidate) {
                return candidate;
            }
        }
        unreachable!()
    }

    fn select_macro_by_name(&mut self, name: &str) {
        if let Some(i) = self.macros.iter().position(|m| m.name == name) {
            self.selected_macro = i;
            self.selected_action = None;
            self.tooltip.cancel();
            self.macro_meta.sync_selection(i, &self.macros[i]);
        }
    }

    fn create_macro(&mut self) {
        let name = self.unique_macro_name("new macro");
        let m = Macro::new(name.clone(), 0, vec![]);
        self.db.macros.insert(m.name.clone(), m.clone());
        if let Err(e) = self.db.save_default() {
            eprintln!("sqyre: create macro: {e}");
            return;
        }
        self.macros.push(m);
        self.macros.sort_by(|a, b| a.name.cmp(&b.name));
        self.refresh_macro_hotkey_bindings();
        self.select_macro_by_name(&name);
    }

    fn duplicate_selected_macro(&mut self) {
        if self.macros.is_empty() {
            return;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let src_name = self.macros[idx].name.clone();
        let mut dup = self.macros[idx].clone();
        dup.name = self.unique_macro_name(&format!("{src_name} copy"));
        // Clear hotkey so duplicate doesn't steal the source chord.
        dup.hotkey.clear();
        let name = dup.name.clone();
        self.db.macros.insert(name.clone(), dup.clone());
        if let Err(e) = self.db.save_default() {
            eprintln!("sqyre: duplicate macro: {e}");
            return;
        }
        self.macros.push(dup);
        self.macros.sort_by(|a, b| a.name.cmp(&b.name));
        self.refresh_macro_hotkey_bindings();
        self.select_macro_by_name(&name);
    }

    fn delete_macro_named(&mut self, name: &str) {
        self.db.macros.remove(name);
        self.tree_histories.remove(name);
        self.macros.retain(|m| m.name != name);
        if let Err(e) = self.db.save_default() {
            eprintln!("sqyre: delete macro: {e}");
        }
        self.refresh_macro_hotkey_bindings();
        if self.macros.is_empty() {
            self.selected_macro = 0;
            self.selected_action = None;
            self.tooltip.cancel();
            return;
        }
        self.selected_macro = self.selected_macro.min(self.macros.len() - 1);
        self.selected_action = None;
        self.tooltip.cancel();
        self.macro_meta
            .sync_selection(self.selected_macro, &self.macros[self.selected_macro]);
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

    /// Insert a blank action below the current selection.
    /// Opens a provisional edit tip — Cancel removes the action without keeping it.
    fn insert_blank_action(&mut self, action: Action, edit_anchor: egui::Pos2) -> bool {
        if self.macros.is_empty() {
            return false;
        }
        let new_id = action.id;
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
            .insert_at(parent, slot, action.clone())
            .is_err()
        {
            return false;
        }
        self.selected_action = Some(new_id);
        // Not persisted until Save; Cancel removes the provisional node.
        self.tooltip.open_edit_new(&action, edit_anchor);
        true
    }

    fn discard_provisional_action(&mut self, action_id: ActionId) {
        if self.macros.is_empty() {
            return;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let _ = self.macros[idx].root.remove_by_id(action_id);
        if self.selected_action == Some(action_id) {
            self.selected_action = None;
        }
        if self.logs_window == Some(action_id) {
            self.logs_window = None;
            self.logs_image_cache.clear();
        }
        // Drop the undo entry recorded for the provisional insert so Undo is a no-op.
        let name = self.macros[idx].name.clone();
        if let Some(hist) = self.tree_histories.get_mut(&name) {
            hist.pop_last_undo();
        }
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
        self.runtime_vars.clear();
        self.logs_image_cache.clear();
        self.highlighter.clear_all();
        self.last_exec_follow = None;
        // Expand happens on the next UI frame via `sync_execution_expand`.
        let action_log = self.action_log.clone();
        let runtime_vars = self.runtime_vars.clone();
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
                        runtime_vars: Some(&runtime_vars),
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

    /// Hide the main window while a screen-click recording is armed.
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

    /// Live X11 selection outline + coords HUD while screen-click recording is armed.
    fn sync_recording_overlay(&mut self, ctx: &egui::Context) {
        self.recording_overlay.sync(ctx, &self.screen_click);
    }

    /// Always-on-top macro buttons (settings-backed); hidden while recording is armed.
    /// While the Data Editor Overlay tab is editing a button, that button is previewed
    /// live (even if overlays are globally disabled / focus-gated).
    fn sync_macro_overlay(&mut self, ctx: &egui::Context) {
        let enabled = self.settings_ui.settings().overlay_enabled;
        let buttons = self.settings_ui.settings().overlay_buttons.clone();
        let preview = self.data_editor.overlay_edit_preview();
        let hide = self.screen_click.is_armed();
        self.macro_overlay.sync(
            ctx,
            enabled,
            &buttons,
            preview.as_ref(),
            &self.catalog,
            &self.pending_hotkey_macros,
            hide,
        );
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
            self.selected_macro,
            &mut self.catalog,
            &mut self.icon_cache,
            &mut self.preview_tooltips,
            &self.screen_click,
            self.settings_ui.settings_mut(),
        );
        self.settings_ui.show(
            ui.ctx(),
            &mut self.db,
            &mut self.macros,
            &mut self.catalog,
        );
        if !self.macros.is_empty() {
            let idx = self.selected_macro.min(self.macros.len() - 1);
            let running = self.run.running.load(Ordering::SeqCst);
            if self
                .variables_panel
                .show(
                    ui.ctx(),
                    &mut self.macros[idx],
                    !running,
                    &self.runtime_vars,
                    running,
                )
            {
                self.persist_macro_at(idx);
            }
        }
        if let Some(action) = {
            let catalog = &self.catalog;
            let icons = &mut self.icon_cache;
            let previews = &mut self.preview_tooltips;
            let macros: Vec<(String, Vec<String>)> = self
                .macros
                .iter()
                .map(|m| (m.name.clone(), m.tags.clone()))
                .collect();
            let known_vars = if self.macros.is_empty() {
                HashSet::new()
            } else {
                let idx = self.selected_macro.min(self.macros.len() - 1);
                collect_known_variable_names(&self.macros[idx])
            };
            let mut defaults_to_persist = false;
            let picked = self.add_action_picker.show(
                ui.ctx(),
                catalog,
                icons,
                previews,
                &macros,
                &known_vars,
                &mut self.key_record,
                &mut self.hotkey_record,
                &self.macro_hotkeys,
                &self.screen_click,
                |_| {
                    defaults_to_persist = true;
                },
            );
            if defaults_to_persist {
                self.add_action_picker
                    .store_into_settings(self.settings_ui.settings_mut());
                if let Err(e) = self.settings_ui.save_settings() {
                    eprintln!("sqyre: save action defaults: {e}");
                }
            }
            picked
        } {
            let anchor = ui
                .ctx()
                .pointer_interact_pos()
                .unwrap_or_else(|| ui.ctx().content_rect().center());
            self.insert_blank_action(action, anchor);
        }
        // Keep highlighter enable flag in sync with the preference.
        let highlight_on = self.settings_ui.settings().highlight_active_action;
        if self.highlighter.is_enabled() != highlight_on {
            self.highlighter.set_enabled(highlight_on);
        }
        self.action_log
            .set_log_images(self.settings_ui.settings().save_meta_images);
        if self.settings_ui.reload_requested {
            self.settings_ui.reload_requested = false;
            apply_main_monitor_resolution(&mut self.catalog);
            match single_instance::reacquire(self.instance_lock.take()) {
                Ok(lock) => self.instance_lock = lock,
                Err(e) => eprintln!("sqyre: re-acquire instance lock: {e}"),
            }
            if self.instance_lock.is_none() {
                eprintln!(
                    "sqyre: warning: could not lock {} (another instance may be using this data dir)",
                    sqyre_persist::sqyre_dir().join("sqyre.lock").display()
                );
            }
            self.selected_macro = 0;
            self.selected_action = None;
            self.tree_histories.clear();
            self.tooltip.cancel();
            self.add_action_picker = AddActionPicker::default();
            self.add_action_picker
                .load_from_settings(self.settings_ui.settings());
            let editor_open = self.data_editor.open;
            self.data_editor = DataEditor::default();
            self.data_editor.open = editor_open;
            let vars_open = self.variables_panel.open;
            self.variables_panel = variables_panel::VariablesPanelUi::default();
            self.variables_panel.open = vars_open;
            self.pending_delete_macro = None;
            self.icon_cache = IconCache::new();
            self.preview_tooltips = PreviewTooltipCache::new();
            self.refresh_macro_hotkey_bindings();
        }
        // Sample color before restoring visibility so the app isn't under the cursor.
        if let Some((x, y)) = self.screen_click.take_color_point() {
            match pixel_color::sample_pixel_hex(x, y) {
                Ok(hex) => {
                    self.tooltip.apply_recorded_color(hex.clone());
                    self.add_action_picker.apply_recorded_color(hex);
                }
                Err(e) => eprintln!("sqyre: sample pixel color: {e}"),
            }
        }
        self.update_recording_visibility(ui.ctx());
        self.sync_recording_overlay(ui.ctx());
        self.sync_macro_overlay(ui.ctx());
        self.drain_pending_hotkey_macros(ui.ctx());

        if let Some(chord) = self.hotkey_record.show(ui.ctx(), &self.macro_hotkeys) {
            if !self.tooltip.apply_recorded_chord(chord.clone())
                && !self.add_action_picker.apply_recorded_chord(chord.clone())
            {
                self.apply_hotkey_to_selected(chord, None);
            }
        }
        if let Some(key) = self.key_record.show(ui.ctx(), &self.macro_hotkeys) {
            self.tooltip.apply_recorded_key(key.clone());
            self.add_action_picker.apply_recorded_key(key);
        }

        let running = self.run.running.load(Ordering::SeqCst);
        if running
            || self.hotkey_record.is_open()
            || self.key_record.is_open()
            || self.screen_click.is_armed()
        {
            ui.ctx().request_repaint();
        } else if self.settings_ui.settings().overlay_enabled {
            // Overlay focus-gating polls on its own schedule; avoid per-frame
            // transparent window clears (flicker) while still draining click queue promptly.
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(250));
        }

        // Ctrl+C / Ctrl+X / Ctrl+V / Ctrl+Z / Ctrl+Y / Ctrl+A — skip while editing an action
        // or when a text field has keyboard focus (so Ctrl+A still selects-all in editors).
        if !self.tooltip.is_editing()
            && !self.hotkey_record.is_open()
            && !self.key_record.is_open()
            && !ui.ctx().egui_wants_keyboard_input()
        {
            let (copy, cut, paste, undo, redo, add_action) = ui.ctx().input(|i| {
                let mod_key = i.modifiers.command;
                let copy = mod_key && i.key_pressed(egui::Key::C);
                let cut = mod_key && i.key_pressed(egui::Key::X);
                let paste = mod_key && i.key_pressed(egui::Key::V);
                let undo = mod_key && !i.modifiers.shift && i.key_pressed(egui::Key::Z);
                let redo = mod_key
                    && (i.key_pressed(egui::Key::Y)
                        || (i.modifiers.shift && i.key_pressed(egui::Key::Z)));
                let add_action = mod_key && i.key_pressed(egui::Key::A);
                (copy, cut, paste, undo, redo, add_action)
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
            } else if add_action && !running && !self.macros.is_empty() {
                self.add_action_picker.open();
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
                ui.horizontal(|ui| {
                    // Use ASCII / NotoEmoji glyphs only — fullwidth/math symbols
                    // (＋, ⧉) render as tofu in egui's default font stack.
                    if ui
                        .button("+")
                        .on_hover_text("New macro")
                        .clicked()
                    {
                        self.create_macro();
                    }
                    let has_sel = !self.macros.is_empty();
                    if ui
                        .add_enabled(has_sel, egui::Button::new("📄"))
                        .on_hover_text("Duplicate selected macro")
                        .clicked()
                    {
                        self.duplicate_selected_macro();
                    }
                    if ui
                        .add_enabled(has_sel, egui::Button::new("🗑"))
                        .on_hover_text("Delete selected macro")
                        .clicked()
                    {
                        let idx = self.selected_macro.min(self.macros.len() - 1);
                        self.pending_delete_macro = Some(self.macros[idx].name.clone());
                    }
                });
                ui.add(
                    egui::TextEdit::singleline(&mut self.macro_list_filter)
                        .desired_width(f32::INFINITY)
                        .hint_text("Search macros or tags…"),
                );
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let width = ui.available_width();
                    let text_width =
                        (width - ui.spacing().button_padding.x * 2.0).max(0.0);
                    let filter = self.macro_list_filter.trim().to_string();
                    for (i, m) in self.macros.iter().enumerate() {
                        if !pickers::query_matches_name_or_tags(&filter, &m.name, &m.tags) {
                            continue;
                        }
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

        if let Some(name) = self.pending_delete_macro.clone() {
            let mut open = true;
            egui::Window::new("Delete Macro")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .order(egui::Order::Foreground)
                .open(&mut open)
                .show(ui.ctx(), |ui| {
                    ui.label(format!("Delete macro \"{name}\"?"));
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.pending_delete_macro = None;
                        }
                        if ui.button("Delete").clicked() {
                            self.pending_delete_macro = None;
                            self.delete_macro_named(&name);
                        }
                    });
                });
            if !open {
                self.pending_delete_macro = None;
            }
        }
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                let tex = self.icon_cache.sqyre_fallback(ui.ctx());
                let size = egui::vec2(28.0, 28.0);
                ui.add(
                    egui::Image::new((tex.id(), size))
                        .fit_to_exact_size(size)
                        .maintain_aspect_ratio(true),
                );
                ui.heading("Sqyre");
            });
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
                if toolbar_icon(ui, "+", "Add Action (Ctrl+A)", !running).clicked() {
                    self.add_action_picker.open();
                }
                if toolbar_icon(ui, "x", "Variables", true).clicked() {
                    self.variables_panel.open = true;
                }
                ui.separator();
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
            // Floating bars allocate 0 by default and cover the row chrome; reserve
            // bar_width so logs/delete stay clear when the scrollbar appears.
            let actions = ui
                .scope(|ui| {
                    ui.spacing_mut().scroll.floating_allocated_width =
                        ui.spacing().scroll.bar_width;
                    egui::ScrollArea::vertical()
                        .id_salt("macro_tree_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            // Decide reorder vs drag-scroll before TreeView so
                            // allow_drag_and_drop can suppress a non-handle drag.
                            let (primary_down, primary_released, pointer_delta, pointer_vel_y, dt) =
                                ui.input(|i| {
                                    (
                                        i.pointer.primary_down(),
                                        i.pointer.primary_released(),
                                        i.pointer.delta(),
                                        i.pointer.velocity().y,
                                        i.stable_dt.min(0.1),
                                    )
                                });
                            if primary_released && self.tree_drag_mode == TreeDragMode::Scroll {
                                // Hand off to kinetic coast (same as egui ScrollArea).
                                self.tree_scroll_vel = pointer_vel_y;
                            }
                            if !primary_down {
                                self.tree_drag_mode = TreeDragMode::Idle;
                            } else if self.tree_drag_mode == TreeDragMode::Idle {
                                let become_drag =
                                    ui.input(|i| !i.pointer.could_any_button_be_click());
                                if become_drag {
                                    self.tree_scroll_vel = 0.0;
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
                                                self.tree_drag_handles
                                                    .iter()
                                                    .any(|r| r.contains(p))
                                            })
                                        });
                                        self.tree_drag_mode = if on_handle {
                                            TreeDragMode::Reorder
                                        } else {
                                            TreeDragMode::Scroll
                                        };
                                    }
                                }
                            }
                            if self.tree_drag_mode == TreeDragMode::Scroll {
                                ui.scroll_with_delta_animation(
                                    pointer_delta,
                                    egui::style::ScrollAnimation::none(),
                                );
                            } else if self.tree_scroll_vel.abs() >= TREE_SCROLL_STOP_SPEED {
                                ui.scroll_with_delta_animation(
                                    egui::vec2(0.0, self.tree_scroll_vel * dt),
                                    egui::style::ScrollAnimation::none(),
                                );
                                let friction = TREE_SCROLL_FRICTION * dt;
                                if friction > self.tree_scroll_vel.abs() {
                                    self.tree_scroll_vel = 0.0;
                                } else {
                                    self.tree_scroll_vel -=
                                        friction * self.tree_scroll_vel.signum();
                                    ui.ctx().request_repaint();
                                }
                            } else {
                                self.tree_scroll_vel = 0.0;
                            }
                            let allow_dnd =
                                !running && self.tree_drag_mode != TreeDragMode::Scroll;

                            let catalog = &self.catalog;
                            let icons = &mut self.icon_cache;
                            let root = &self.macros[idx].root;
                            let root_children = root.children();
                            let known_vars = collect_known_variable_names(&self.macros[idx]);
                            let interact_y = ui.spacing().interact_size.y;
                            let (_, tree_actions) = TreeView::new(id)
                                .allow_drag_and_drop(allow_dnd)
                                .default_node_height(Some(tree_chrome::default_row_height(interact_y)))
                                .show_state(ui, &mut state, |builder: &mut TreeViewBuilder<'_, ActionId>| {
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
                                            catalog,
                                            icons,
                                            &known_vars,
                                            is_dark,
                                            &macro_name,
                                            &hl_snap,
                                            scroll_to,
                                            &mut scrolled_follow,
                                            interact_y,
                                        );
                                    }
                                    builder.close_dir();
                                });
                            // Off-clip rows skip label_ui — estimate Y so ScrollArea can still follow.
                            if let Some(target) = scroll_to {
                                if !scrolled_follow {
                                    if let Some(row_i) = flattened_visible_index(root, target) {
                                        let row_h = tree_chrome::default_row_height(
                                            ui.spacing().interact_size.y,
                                        ) + ui.spacing().item_spacing.y;
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
                        .inner
                })
                .inner;
            if scrolled_follow {
                self.last_exec_follow = follow;
            }

            self.tree_drag_handles.clear();
            for (_, interaction) in &row_events {
                let r = interaction.drag_handle_rect;
                if r.width() > 0.0 && r.height() > 0.0 {
                    self.tree_drag_handles.push(r);
                }
            }

            // Row overlay is clickthrough (Sense::hover + geometric clicks). When a
            // view tip covers the row, TreeView never sees the click — select here.
            for (aid, interaction) in &row_events {
                if interaction.primary_clicked {
                    state.set_one_selected(*aid);
                    self.selected_action = Some(*aid);
                }
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
            // Prefer edit-open from any row; otherwise last hovered wins for view.
            for (aid, interaction) in &row_events {
                if interaction.hovered || interaction.pointer_in_row {
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
                let discarded = {
                    let macro_ = &mut self.macros[idx];
                    action_tooltip::show(
                        &mut self.tooltip,
                        ui.ctx(),
                        macro_,
                        catalog,
                        icons,
                        previews,
                        &macros,
                        &known_vars,
                        is_dark,
                        &mut self.key_record,
                        &mut self.hotkey_record,
                        &self.macro_hotkeys,
                        &self.screen_click,
                        |root_before| {
                            if pending_record.is_none() {
                                if let Ok(snap) =
                                    TreeHistory::take_snapshot(root_before, selected)
                                {
                                    pending_record = Some(snap);
                                }
                            }
                        },
                    )
                };
                if let Some(snap) = pending_record {
                    self.tree_histories
                        .entry(name.clone())
                        .or_default()
                        .push_snapshot(snap);
                    // Saved an edit (including first save of a provisional insert).
                    self.persist_macro_at(idx);
                }
                if let Some(aid) = discarded {
                    self.discard_provisional_action(aid);
                }
            }

            let mut pending_moves: Vec<(ActionId, ActionId, InsertSlot)> = Vec::new();
            for action in actions {
                match action {
                    TreeAction::SetSelected(sel) => {
                        self.selected_action = sel.into_iter().next();
                    }
                    TreeAction::Move(dnd) => {
                        if running || self.tree_drag_mode == TreeDragMode::Scroll {
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
                        if self.tree_drag_mode == TreeDragMode::Scroll {
                            dnd.remove_drop_marker(ui);
                            continue;
                        }
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

    /// Fully transparent clear so deferred overlay viewports (`with_transparent(true)`)
    /// don't paint eframe's default dark plate behind the gold chrome.
    /// Opaque root window still fills solid via its framebuffer; UI panels supply their own fill.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
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
    interact_y: f32,
) {
    let action_id = action.id;
    let highlight = row_highlight(macro_name, action_id, hl_snap);
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
                    catalog,
                    icons,
                    known_vars,
                    is_dark,
                    macro_name,
                    hl_snap,
                    scroll_to,
                    scrolled_follow,
                    interact_y,
                );
            }
        }
        builder.close_dir();
    } else {
        builder.node(
            NodeBuilder::leaf(action_id)
                .height(row_h)
                .label_ui(|ui| {
                    handle_row(ui, open_logs, delete_action, row_events, scrolled_follow);
                }),
        );
    }
}

/// Compact toolbar control: icon glyph + hover label.
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
