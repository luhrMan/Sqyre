//! egui shell: load macros from `~/.sqyre`, Run/Stop with live backends.

mod action_logs_ui;
mod action_tooltip;
mod add_action;
mod assets;
mod catalog;
mod collection_capture;
mod data_editor;
mod data_editor_preview;
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
mod paint_ctx;
mod pickers;
mod pixel_color;
mod preview_tooltip;
mod recorded_action;
mod recording_overlay;
mod settings;
mod single_instance;
mod status_banner;
pub mod theme;
mod tray;
mod tree_chrome;
mod tree_clipboard;
mod tree_dnd;
mod tree_history;
mod ui_macro_list;
mod ui_macro_tree;
mod ui_overlays;
mod ui_toolbar;
mod var_pills;
mod variables_panel;

pub use settings::SettingsUi;

use action_logs_ui::LogsImageCache;
use action_tooltip::TooltipState;
use add_action::AddActionPicker;
use catalog::{apply_main_monitor_resolution, CatalogIcons, CatalogResolver, SnapshotMacros};
use data_editor::DataEditor;
use eframe::egui;
use hotkey_record::HotkeyRecordUi;
use icon_cache::IconCache;
use key_record::KeyRecordUi;
use macro_meta::MacroMetaUi;
use macro_overlay::MacroOverlay;
use parking_lot::Mutex;
use preview_tooltip::PreviewTooltipCache;
use recording_overlay::RecordingOverlay;
use sqyre_capture::{shared_capturer, SharedRunCapturer, X11WindowFocuser};
use sqyre_domain::{Action, ActionId, Macro};
use sqyre_executor::{
    execute_macro_with, ContinueKeyWaiter, ExecDeps, OcrEngine, OcrResult, SharedActionLog,
    SharedHighlighter, SharedRuntimeVars,
};
use sqyre_hotkeys::{
    default_hotkeys, ContinueWaitBridge, HotkeyCallbacks, HotkeyService, HotkeyTrigger,
    MacroHotkeyBinding, MacroHotkeyBridge, NullHotkeys, ScreenClickBridge, StopFlag,
};
use sqyre_input::OsAutomation;
use sqyre_match::ImageBuf;
use sqyre_persist::{variables_path, Database, ProgramCatalog, UserSettings};
use sqyre_vision::LeptessOcr;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use tree_history::TreeHistory;
use ui_macro_tree::TreeDragMode;

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
        let result = self
            .continue_wait
            .wait_for_any_chord(chords, hold_repeat, pass_through, stop);
        self.macro_hotkeys.resume();
        result
    }
}

/// Launch the desktop shell (single-instance lock, tray, fonts).
pub fn run() -> eframe::Result<()> {
    let _ = sqyre_persist::initialize_directories();
    diag::install(sqyre_persist::sqyre_dir());
    #[cfg(target_os = "linux")]
    install_x11_secondary_error_hook();

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

/// Keep winit from storing X errors that originate on Sqyre's secondary Displays.
///
/// Those Displays (focus / capture / outline) race destroyed windows during
/// overlay skip-taskbar and property reads. Without this hook, a BadWindow
/// GetProperty lands in winit's global error slot and later panics on IME
/// destroy (`Failed to destroy input context`).
#[cfg(target_os = "linux")]
fn install_x11_secondary_error_hook() {
    winit::platform::x11::register_xlib_error_hook(Box::new(|display, _event| {
        sqyre_capture::owns_secondary_x_display(display)
    }));
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
    /// Last failed macro/db save; shown in the macro list until a save succeeds.
    save_error: Option<String>,
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
            save_error: None,
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
        self.macros.first()?.root.children().first().map(|a| a.id)
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
                pending_for_cb.lock().push(name);
                // Idle/unfocused eframe may not paint until woken — without this,
                // queued macros only start when Sqyre regains focus.
                if let Some(ctx) = repaint_for_cb.lock().as_ref() {
                    ctx.request_repaint();
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
                    save_error: None,
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
                    save_error: None,
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
            }
        }
    }

    /// Provide egui context so background hotkey fires can wake an idle UI frame.
    fn bind_hotkey_repaint(&self, ctx: egui::Context) {
        *self.hotkey_repaint.lock() = Some(ctx);
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
            self.save_error = Some(e.to_string());
        } else {
            self.save_error = None;
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
            self.save_error = Some(e.to_string());
            return;
        }
        self.save_error = None;
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
            self.save_error = Some(e.to_string());
            return;
        }
        self.save_error = None;
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
            self.save_error = Some(e.to_string());
        } else {
            self.save_error = None;
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
            self.save_error = Some(e.to_string());
        } else {
            self.save_error = None;
        }
        self.refresh_macro_hotkey_bindings();

        self.macros.sort_by(|a, b| a.name.cmp(&b.name));
        if let Some(i) = self.macros.iter().position(|m| m.name == new_name) {
            self.selected_macro = i;
        }
        self.macro_meta
            .sync_selection(self.selected_macro, &self.macros[self.selected_macro]);
    }

    fn apply_hotkey_to_selected(&mut self, chord: Vec<String>, trigger: Option<HotkeyTrigger>) {
        if self.macros.is_empty() {
            return;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let trigger =
            trigger.unwrap_or_else(|| HotkeyTrigger::parse(&self.macros[idx].hotkey_trigger));
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
            self.persist_macro_at(idx);
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
            self.persist_macro_at(idx);
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
        self.persist_macro_at(idx);
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
        self.persist_macro_at(idx);
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
            let map: BTreeMap<String, Arc<Macro>> = self
                .macros
                .iter()
                .map(|m| (m.name.clone(), Arc::new(m.clone())))
                .collect();
            SnapshotMacros(Arc::new(map))
        };
        let ctx = ctx.clone();
        running.store(true, Ordering::SeqCst);
        *status.lock() = format!("Running {}…", macro_.name);

        thread::spawn(move || {
            let result = (|| -> Result<(), String> {
                let mut automation = OsAutomation::new().map_err(|e| format!("automation: {e}"))?;
                let capturer_arc = shared_capturer().map_err(|e| format!("capture: {e}"))?;
                let mut capturer = SharedRunCapturer(capturer_arc);
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
                        close_matches_distance: close_matches,
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

            // Drop blurred templates / masks retained during image search so RSS can fall.
            sqyre_vision::clear_search_cache();
            trim_process_heap();

            let msg = match result {
                Ok(()) if stop_flag.is_stopped() => "Stopped.".into(),
                Ok(()) => "Finished.".into(),
                Err(e) => format!("Error: {e}"),
            };
            *status.lock() = msg;
            running.store(false, Ordering::SeqCst);
            ctx.request_repaint();
        });
    }

    fn drain_pending_hotkey_macros(&mut self, ctx: &egui::Context) {
        let pending: Vec<String> = std::mem::take(&mut *self.pending_hotkey_macros.lock());
        for name in pending {
            self.start_macro_by_name(&name, ctx);
        }
    }

    fn request_stop(&mut self) {
        self.run.stop.request_stop();
        *self.run.status.lock() = "Stop requested…".into();
    }

    /// Hide the main window while a screen-click recording is armed.
    fn update_recording_visibility(&mut self, ctx: &egui::Context) {
        let should_hide =
            self.settings_ui.settings().hide_app_during_recording && self.screen_click.is_armed();
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
    fn type_char(&mut self, ch: char) {
        if !self.stop.is_stopped() {
            self.inner.type_char(ch);
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
        ui_overlays::handle_close_to_tray(self, ui.ctx());
        ui_overlays::show_floating_windows(self, ui.ctx());
        ui_overlays::sync_frame_state(self, ui.ctx());
        ui_overlays::handle_shortcuts(self, ui);

        ui_macro_list::show(self, ui);

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui_toolbar::brand_header(self, ui);
            ui_toolbar::main_toolbar(self, ui);
            if self.macros.is_empty() {
                ui.label("No macros loaded. Place a db.yaml under ~/.sqyre.");
                return;
            }
            if !ui_toolbar::show_meta_and_hotkey(self, ui) {
                return;
            }
            let force_openness = ui_toolbar::action_toolbar(self, ui);
            ui_macro_tree::show(self, ui, force_openness);
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

/// Ask glibc to return freed pages after large image/OCR allocations.
fn trim_process_heap() {
    #[cfg(target_os = "linux")]
    {
        unsafe {
            extern "C" {
                fn malloc_trim(pad: usize) -> i32;
            }
            let _ = malloc_trim(0);
        }
    }
}
