//! egui shell: load macros from `~/.sqyre` (native) or in-memory / YAML import (WASM).

mod action_logs_ui;
mod action_tooltip;
mod add_action;
mod app_backends;
mod app_docs;
mod app_macro_ops;
mod app_run;
mod assets;
mod catalog;
mod chord_record;
mod collection_capture;
mod data_editor;
mod data_editor_preview;
mod demo_icons;
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
#[cfg(not(target_arch = "wasm32"))]
mod sound;
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
#[cfg(any(test, target_arch = "wasm32"))]
mod wasm_demo_seed;
mod wasm_io;
mod widgets;

pub use settings::SettingsUi;

use action_logs_ui::LogsImageCache;
use action_tooltip::TooltipState;
use add_action::AddActionPicker;
use app_backends::RunState;
use catalog::apply_main_monitor_resolution;
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
use sqyre_domain::{ActionId, Macro};
use sqyre_executor::{SharedActionLog, SharedHighlighter, SharedRuntimeVars};
use sqyre_hotkeys::{
    default_hotkeys, ContinueWaitBridge, HotkeyCallbacks, HotkeyService, MacroHotkeyBridge,
    ScreenClickBridge,
};
use sqyre_persist::{Database, ProgramCatalog, UserSettings};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tree_history::TreeHistory;
use ui_macro_tree::TreeDragMode;
use wasm_io::PendingImport;

/// Launch the desktop shell (single-instance lock, tray, fonts).
#[cfg(not(target_arch = "wasm32"))]
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
            .with_title("Sqyre")
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
#[cfg(all(not(target_arch = "wasm32"), target_os = "linux"))]
fn install_x11_secondary_error_hook() {
    winit::platform::x11::register_xlib_error_hook(Box::new(|display, _event| {
        sqyre_capture::owns_secondary_x_display(display)
    }));
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
    /// Outline windows for live search-area selection rect.
    recording_overlay: RecordingOverlay,
    /// Always-on-top floating buttons that start macros.
    macro_overlay: MacroOverlay,
    /// Left macro-list side panel visibility.
    macro_list_open: bool,
    /// Filter text for the macro list (name / tags fuzzy match).
    macro_list_filter: String,
    tray: tray::SystemTray,
    /// Process-wide single-instance lock (held for the app lifetime).
    instance_lock: Option<single_instance::InstanceLock>,
    /// Confirm dialog for deleting the selected macro.
    pending_delete_macro: Option<String>,
    /// WASM async YAML import result (unused on native).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pending_import: PendingImport,
    /// In-flight automatic backup (native only).
    #[cfg(not(target_arch = "wasm32"))]
    backup_task: Option<std::sync::mpsc::Receiver<Result<std::path::PathBuf, String>>>,
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
        let hotkey_repaint = Arc::new(Mutex::new(None::<egui::Context>));
        let repaint_for_cb = Arc::clone(&hotkey_repaint);

        #[cfg(not(target_arch = "wasm32"))]
        let _ = hotkeys.start(HotkeyCallbacks {
            on_escape_stop: Arc::new(move || stop.request_stop()),
            on_failsafe: Arc::new(|| {
                eprintln!("failsafe Esc+Ctrl+Shift — exiting");
                std::process::exit(0);
            }),
            on_macro_hotkey: Arc::new(move |name| {
                pending_for_cb.lock().push(name);
                if let Some(ctx) = repaint_for_cb.lock().as_ref() {
                    ctx.request_repaint();
                }
            }),
        });
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (
                &mut hotkeys,
                &stop,
                pending_for_cb,
                repaint_for_cb,
                HotkeyCallbacks::default(),
            );
        }

        let highlighter = SharedHighlighter::new();
        highlighter.set_enabled(settings.highlight_active_action);
        let settings_ui = SettingsUi::from_settings(settings);
        let action_log = SharedActionLog::new();
        action_log.set_log_images(settings_ui.settings().save_meta_images);
        let mut add_action_picker = AddActionPicker::default();
        add_action_picker.load_from_settings(settings_ui.settings());

        match Database::load_default() {
            #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
            Ok(mut db) => {
                let mut catalog = db.program_catalog().unwrap_or_default();
                apply_main_monitor_resolution(&mut catalog);
                let mut macros: Vec<_> = db.macros.values().cloned().collect();
                macros.sort_by(|a, b| a.name.cmp(&b.name));
                #[cfg(target_arch = "wasm32")]
                {
                    let _ =
                        wasm_demo_seed::ensure_demo_if_empty(&mut macros, &mut catalog, &mut db);
                }
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
                    pending_import: wasm_io::new_pending_import(),
                    #[cfg(not(target_arch = "wasm32"))]
                    backup_task: None,
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
                    pending_import: wasm_io::new_pending_import(),
                    #[cfg(not(target_arch = "wasm32"))]
                    backup_task: None,
                }
            }
        }
    }

    /// Browser entry: in-memory DB, no tray / global hotkeys / FS init.
    #[cfg(target_arch = "wasm32")]
    pub fn load_web(cc: &eframe::CreationContext<'_>) -> Self {
        let app = Self::load();
        SettingsUi::install_fonts(&cc.egui_ctx);
        SettingsUi::apply_appearance(&cc.egui_ctx, app.settings_ui.settings());
        app.bind_hotkey_repaint(cc.egui_ctx.clone());
        app
    }
}

impl eframe::App for SqyreApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.take_pending_db_import();
        ui_overlays::handle_close_to_tray(self, ui.ctx());
        ui_overlays::show_floating_windows(self, ui.ctx());
        ui_overlays::sync_frame_state(self, ui.ctx());
        ui_overlays::handle_shortcuts(self, ui);

        ui_macro_list::show(self, ui);

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui_toolbar::brand_header(self, ui);
            ui_toolbar::main_toolbar(self, ui);
            if self.macros.is_empty() {
                #[cfg(target_arch = "wasm32")]
                ui.label("No macros loaded. Use Import to open a db.yaml.");
                #[cfg(not(target_arch = "wasm32"))]
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
    ///
    /// On wasm there are no transparent OS overlay windows — an opaque clear keeps a
    /// panic/blank frame from looking like the page body color bleeding through.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        #[cfg(target_arch = "wasm32")]
        {
            egui::Rgba::from(egui::Color32::from_rgb(0x1a, 0x1a, 0x1a)).to_array()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            [0.0, 0.0, 0.0, 0.0]
        }
    }
}

impl Drop for SqyreApp {
    fn drop(&mut self) {
        self.hotkeys.stop();
    }
}
