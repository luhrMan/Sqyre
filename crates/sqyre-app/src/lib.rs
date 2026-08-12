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
#[cfg(feature = "native-runtime")]
mod collection_capture;
mod data_editor;
mod data_editor_preview;
mod demo_icons;
#[cfg(feature = "native-runtime")]
mod diag;
pub mod docs_fixture;
mod file_dialogs;
mod hotkey_record;
mod icon_cache;
mod icon_variants;
mod image_view;
mod key_record;
mod log;
mod macro_meta;
#[cfg(feature = "native-runtime")]
mod macro_overlay;
mod macro_record;
mod overlay_icons;
mod paint_ctx;
mod pickers;
#[cfg(feature = "native-runtime")]
mod pixel_color;
#[cfg(feature = "native-runtime")]
#[path = "preview_tooltip.rs"]
mod preview_tooltip;
#[cfg(not(feature = "native-runtime"))]
#[path = "preview_tooltip_stub.rs"]
mod preview_tooltip;
mod recorded_action;
#[cfg(feature = "native-runtime")]
mod recording_overlay;
mod run_session;
mod settings;
mod single_instance;
#[cfg(not(target_arch = "wasm32"))]
mod sound;
mod status_banner;
pub mod theme;
mod tray;
mod tree_chrome;
mod tree_clipboard;
mod tree_dnd;
mod tree_history;
mod tree_state;
mod ui_macro_list;
mod ui_macro_tree;
mod ui_overlays;
mod ui_toolbar;
#[cfg(not(target_arch = "wasm32"))]
mod update;
mod var_pills;
mod variables_panel;
#[cfg(any(test, target_arch = "wasm32"))]
mod wasm_demo_seed;
mod wasm_io;
mod widgets;
#[cfg(target_os = "windows")]
mod win_focused_keys;
mod window_types;
mod workspace;

pub use settings::SettingsUi;

use add_action::AddActionPicker;
use app_backends::RunState;
use catalog::{apply_main_monitor_resolution, ensure_general_program_seeded, prepare_catalog};
use data_editor::DataEditor;
use eframe::egui;
use hotkey_record::HotkeyRecordUi;
use icon_cache::IconCache;
use key_record::KeyRecordUi;
use macro_meta::MacroMetaUi;
use macro_record::MacroRecordUi;
use parking_lot::Mutex;
use preview_tooltip::PreviewTooltipCache;
use run_session::RunSession;
use sqyre_domain::Macro;
use sqyre_hotkeys::{
    default_hotkeys, HotkeyCallbacks, HotkeyService, MacroRecordBridge, ScreenClickBridge,
};
use sqyre_persist::{Database, ProgramCatalog, UserSettings};
use sqyre_ui_model::{SharedActionLog, SharedHighlighter, SharedRuntimeVars};
use std::sync::Arc;
use tree_state::TreeState;
use wasm_io::PendingImport;
use workspace::Workspace;

/// Launch the desktop shell (single-instance lock, tray, fonts).
#[cfg(not(target_arch = "wasm32"))]
pub fn run() -> eframe::Result<()> {
    let _ = sqyre_persist::initialize_directories();
    #[cfg(feature = "native-runtime")]
    diag::install(sqyre_persist::sqyre_dir());
    sqyre_update::cleanup_stale_update();
    #[cfg(all(
        not(target_arch = "wasm32"),
        feature = "native-runtime",
        target_os = "linux"
    ))]
    install_x11_secondary_error_hook();

    let instance_lock = match single_instance::try_acquire() {
        Ok(Some(lock)) => Some(lock),
        Ok(None) => {
            crate::log::warn("Sqyre is already running");
            std::process::exit(0);
        }
        Err(e) => {
            crate::log::warn(format!("failed to acquire instance lock: {e}"));
            std::process::exit(1);
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([100.0, 100.0])
            .with_title("Sqyre")
            .with_icon(assets::app_icon()),
        // wgpu's DX12 HWND swapchain has no per-pixel alpha; glow + DWM blur-behind
        // is required for deferred overlay button transparency (egui#3632).
        #[cfg(target_os = "windows")]
        renderer: eframe::Renderer::Glow,
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
#[cfg(all(
    not(target_arch = "wasm32"),
    feature = "native-runtime",
    target_os = "linux"
))]
fn install_x11_secondary_error_hook() {
    winit::platform::x11::register_xlib_error_hook(Box::new(|display, _event| {
        sqyre_capture::owns_secondary_x_display(display)
    }));
}

/// Sync macros/catalog into `db` and write `db.yaml`. The single database-save
/// implementation shared by [`SqyreApp::persist_database`] and
/// [`SqyreApp::persist_database_for_editor`] so the two call sites cannot drift.
fn sync_and_save_database(
    db: &mut Database,
    macros: &[Macro],
    catalog: &ProgramCatalog,
) -> Result<(), String> {
    db.set_programs_from_catalog(catalog);
    db.replace_macros(macros.iter().cloned());
    db.save_default().map_err(|e| e.to_string())
}

pub struct SqyreApp {
    pub(crate) workspace: Workspace,
    pub(crate) run_session: RunSession,
    pub(crate) tree: TreeState,
    hotkeys: Box<dyn HotkeyService>,
    screen_click: ScreenClickBridge,
    macro_record_bridge: MacroRecordBridge,
    /// Macro names requested by the hotkey thread (drained each frame).
    pending_hotkey_macros: Arc<Mutex<Vec<String>>>,
    /// egui context for waking the UI when a hotkey queues a macro while idle/unfocused.
    hotkey_repaint: Arc<Mutex<Option<egui::Context>>>,
    hotkey_record: HotkeyRecordUi,
    key_record: KeyRecordUi,
    macro_record: MacroRecordUi,
    icon_cache: IconCache,
    preview_tooltips: PreviewTooltipCache,
    add_action_picker: AddActionPicker,
    data_editor: DataEditor,
    settings_ui: SettingsUi,
    variables_panel: variables_panel::VariablesPanelUi,
    /// Window was hidden because a point/search-area recording is armed.
    hidden_for_recording: bool,
    /// Outline windows for live search-area selection rect.
    #[cfg(feature = "native-runtime")]
    recording_overlay: crate::recording_overlay::RecordingOverlay,
    /// Always-on-top floating buttons that start macros.
    #[cfg(feature = "native-runtime")]
    macro_overlay: crate::macro_overlay::MacroOverlay,
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
    /// Background Find Pixel color sample (native only).
    #[cfg(not(target_arch = "wasm32"))]
    pixel_sample_pending: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
    /// Background update check / download (native only).
    #[cfg(not(target_arch = "wasm32"))]
    update: update::UpdateManager,
}

impl SqyreApp {
    fn load() -> Self {
        let settings = UserSettings::load_default().unwrap_or_else(|e| {
            crate::log::warn(format!("failed to load settings: {e}"));
            UserSettings::default()
        });
        settings.apply_sqyre_dir_override();
        SettingsUi::apply_action_colors(&settings);

        let (mut hotkeys, continue_wait, screen_click, macro_record_bridge, macro_hotkeys) =
            default_hotkeys();
        let run = RunState::default();
        let stop = run.stop.clone();
        let pending_hotkey_macros = Arc::new(Mutex::new(Vec::new()));
        let pending_for_cb = Arc::clone(&pending_hotkey_macros);
        let hotkey_repaint = Arc::new(Mutex::new(None::<egui::Context>));
        let repaint_for_cb = Arc::clone(&hotkey_repaint);

        #[cfg(not(target_arch = "wasm32"))]
        if let Err(e) = hotkeys.start(HotkeyCallbacks {
            on_escape_stop: Arc::new(move || stop.request_stop()),
            on_failsafe: Arc::new(|| {
                crate::log::warn(format!(
                    "failsafe {} — exiting",
                    sqyre_hotkeys::FAILSAFE_LABEL
                ));
                sqyre_input::release_held_inputs();
                std::process::exit(0);
            }),
            on_macro_hotkey: Arc::new(move |name| {
                pending_for_cb.lock().push(name);
                if let Some(ctx) = repaint_for_cb.lock().as_ref() {
                    ctx.request_repaint();
                }
            }),
        }) {
            crate::log::warn(format!("failed to start global hotkeys: {e}"));
        }
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

        let (db, macros, catalog, load_error) = match Database::load_default_with_warnings() {
            #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
            Ok((mut db, load_warnings)) => {
                let mut catalog = Arc::unwrap_or_clone(db.program_catalog().unwrap_or_default());
                let mut macros: Vec<_> = db.macros.values().cloned().collect();
                macros.sort_by(|a, b| a.name.cmp(&b.name));
                #[cfg(target_arch = "wasm32")]
                {
                    apply_main_monitor_resolution(&mut catalog);
                    let _ =
                        wasm_demo_seed::ensure_demo_if_empty(&mut macros, &mut catalog, &mut db);
                }
                let _ = prepare_catalog(&mut catalog, &mut db);
                let load_error = if load_warnings.is_empty() {
                    None
                } else {
                    Some(load_warnings.join("\n"))
                };
                (db, macros, catalog, load_error)
            }
            Err(e) => {
                let mut catalog = ProgramCatalog::default();
                apply_main_monitor_resolution(&mut catalog);
                let _ = ensure_general_program_seeded(&mut catalog);
                (
                    Database::default(),
                    Vec::new(),
                    catalog,
                    Some(e.to_string()),
                )
            }
        };

        let platform_warning = {
            #[cfg(feature = "native-runtime")]
            {
                #[cfg(all(
                    not(target_arch = "wasm32"),
                    any(target_os = "linux", target_os = "windows")
                ))]
                {
                    // Ensure OCR data exists (downloads eng.traineddata when missing).
                    let ocr_warning = match sqyre_vision::shared_leptess() {
                        Ok(_) => None,
                        Err(e) => {
                            let warning = format!("OCR unavailable: {e}");
                            crate::log::warn(&warning);
                            Some(warning)
                        }
                    };
                    #[cfg(target_os = "linux")]
                    {
                        sqyre_capture::linux_session_capture_warning()
                            .or_else(|| match sqyre_capture::shared_capturer() {
                                Ok(_) => None,
                                Err(e) => Some(format!("Screen capture unavailable: {e}")),
                            })
                            .or(ocr_warning)
                    }
                    #[cfg(target_os = "windows")]
                    {
                        ocr_warning
                    }
                }
                #[cfg(not(any(
                    all(target_os = "linux", not(target_arch = "wasm32")),
                    all(target_os = "windows", not(target_arch = "wasm32"))
                )))]
                {
                    None
                }
            }
            #[cfg(not(feature = "native-runtime"))]
            {
                None
            }
        };

        let mut app = Self {
            workspace: Workspace {
                db,
                macros,
                catalog,
                load_error,
                platform_warning,
                save_error: None,
                selected_macro: 0,
                macro_meta: MacroMetaUi::default(),
                hotkey_tag_filter: None,
            },
            run_session: RunSession {
                state: run,
                continue_wait,
                macro_hotkeys,
                action_log,
                runtime_vars: SharedRuntimeVars::new(),
                highlighter,
                logs_window: None,
                logs_image_cache: action_logs_ui::LogsImageCache::default(),
            },
            tree: TreeState::default(),
            hotkeys,
            screen_click,
            macro_record_bridge,
            pending_hotkey_macros,
            hotkey_repaint,
            hotkey_record: HotkeyRecordUi::default(),
            key_record: KeyRecordUi::default(),
            macro_record: MacroRecordUi::default(),
            icon_cache: IconCache::new(),
            preview_tooltips: PreviewTooltipCache::new(),
            add_action_picker,
            data_editor: DataEditor::default(),
            settings_ui,
            variables_panel: variables_panel::VariablesPanelUi::default(),
            hidden_for_recording: false,
            #[cfg(feature = "native-runtime")]
            recording_overlay: recording_overlay::RecordingOverlay::new(),
            #[cfg(feature = "native-runtime")]
            macro_overlay: macro_overlay::MacroOverlay::new(),
            macro_list_open: true,
            macro_list_filter: String::new(),
            tray: tray::SystemTray::default(),
            instance_lock: None,
            pending_delete_macro: None,
            pending_import: wasm_io::new_pending_import(),
            #[cfg(not(target_arch = "wasm32"))]
            backup_task: None,
            #[cfg(not(target_arch = "wasm32"))]
            pixel_sample_pending: None,
            #[cfg(not(target_arch = "wasm32"))]
            update: update::UpdateManager::default(),
        };
        app.refresh_macro_hotkey_bindings();
        #[cfg(not(target_arch = "wasm32"))]
        app.maybe_start_update_check();
        app
    }

    /// Sync working macros + catalog into `db` and write `db.yaml`.
    pub(crate) fn persist_database(&mut self) -> Result<(), String> {
        match sync_and_save_database(
            &mut self.workspace.db,
            &self.workspace.macros,
            &self.workspace.catalog,
        ) {
            Ok(()) => {
                self.workspace.save_error = None;
                Ok(())
            }
            Err(msg) => {
                self.workspace.save_error = Some(msg.clone());
                Err(msg)
            }
        }
    }

    /// Data Editor's persist path: the same save implementation as [`Self::persist_database`],
    /// plus catalog-generation continuity the editor's `ListCache` invalidation relies on.
    ///
    /// Takes explicit `db`/`macros`/`catalog` rather than `&mut self` because
    /// `DataEditor::show` (and its `on_new`/`on_update`/`on_delete` helpers) run with those
    /// fields already disjointly borrowed out of `SqyreApp`, so `&mut SqyreApp` isn't
    /// available at the call site.
    pub(crate) fn persist_database_for_editor(
        db: &mut Database,
        macros: &[Macro],
        catalog: &mut ProgramCatalog,
    ) -> Result<(), String> {
        let previous_generation = catalog.generation();
        sync_and_save_database(db, macros, catalog)?;
        *catalog = Arc::unwrap_or_clone(db.program_catalog().map_err(|e| e.to_string())?);
        // YAML reload resets generation to 0; keep ListCache invalidation working.
        catalog.continue_generation_after_reload(previous_generation);
        Ok(())
    }

    /// Start a background update check when the preference is on.
    #[cfg(not(target_arch = "wasm32"))]
    fn maybe_start_update_check(&mut self) {
        if self.settings_ui.settings().auto_update_check {
            self.update.start_check();
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
        // Poll background tasks before floating windows so Settings sees fresh update state.
        ui_overlays::sync_frame_state(self, ui.ctx());
        ui_overlays::show_floating_windows(self, ui.ctx());
        ui_overlays::handle_shortcuts(self, ui);

        ui_macro_list::show(self, ui);

        egui::CentralPanel::default().show(ui, |ui| {
            ui_toolbar::brand_header(self, ui);
            ui_toolbar::main_toolbar(self, ui);
            if self.workspace.macros.is_empty() {
                ui.horizontal(|ui| {
                    ui.label("Please");
                    if ui.button("create a new macro").clicked() {
                        self.create_macro();
                    }
                    ui.label("or");
                    #[cfg(not(target_arch = "wasm32"))]
                    if ui.button("import a backup").clicked() {
                        self.settings_ui.request_restore_backup();
                    }
                    #[cfg(target_arch = "wasm32")]
                    if ui.button("import a backup").clicked() {
                        self.request_db_import();
                    }
                });
                return;
            }
            if !ui_toolbar::show_meta_and_hotkey(self, ui) {
                return;
            }
            let force_openness = ui_toolbar::action_toolbar(self, ui);
            ui_macro_tree::show(self, ui, force_openness);
        });
        // After tips/panels paint so tooltip preview outlines apply this frame.
        self.sync_recording_overlay(ui.ctx());
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
        #[cfg(not(target_arch = "wasm32"))]
        sqyre_input::release_held_inputs();
        self.hotkeys.stop();
    }
}
