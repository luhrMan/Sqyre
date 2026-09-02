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
mod command_palette;
mod data_editor;
mod data_editor_preview;
mod demo_icons;
#[cfg(feature = "native-runtime")]
mod diag;
pub mod docs_fixture;
mod egui_keys;
mod file_dialogs;
mod hotkey_record;
#[cfg(not(target_arch = "wasm32"))]
mod hotkey_wake;
mod icon_cache;
mod icon_variants;
mod image_view;
mod key_record;
#[cfg(all(target_os = "linux", feature = "native-runtime"))]
mod linux_focused_keys;
mod log;
mod macro_meta;
#[cfg(feature = "native-runtime")]
mod macro_record;
/// Phosphor overlay icon catalog + paint helpers (lives in `sqyre-overlay`).
#[allow(unused_imports)] // re-export surface for `crate::overlay_icons::…`
mod overlay_icons {
    pub use sqyre_overlay::{
        catalog, glyph_font_id, register_phosphor_family, resolve, show_icon_picker_grid,
        style_preview_button, OverlayIcon, OverlayPaintStyle, DEFAULT_ICON_ID,
    };
}
mod paint_ctx;
#[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
mod permissions_panel;
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
use command_palette::CommandPaletteUi;
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
use sqyre_ports::{SharedActionLog, SharedHighlighter, SharedRuntimeVars};
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

    let mut options = eframe::NativeOptions {
        viewport: {
            let builder = egui::ViewportBuilder::default()
                .with_inner_size([960.0, 640.0])
                .with_min_inner_size([100.0, 100.0])
                .with_title("Sqyre")
                .with_app_id(assets::APP_ID)
                .with_icon(assets::app_icon());
            // wgpu deferred overlay viewports inherit transparency from the root GL/VK config.
            #[cfg(target_os = "linux")]
            let builder = builder.with_transparent(true);
            builder
        },
        // wgpu's DX12 HWND swapchain has no per-pixel alpha; glow + DWM blur-behind
        // is required for deferred overlay button transparency (egui#3632).
        // Linux Wayland: glow cannot create transparent deferred viewports; wgpu does.
        #[cfg(target_os = "windows")]
        renderer: eframe::Renderer::Glow,
        #[cfg(target_os = "linux")]
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    // Native Wayland: winit's set_visible / set_outer_position are no-ops, so tray-hide
    // cannot unmap the root window (the old 1×1 off-screen hack left an Alt-Tab skeleton)
    // and X11 window-type / SKIP_TASKBAR hints never apply to overlay viewports.
    // Prefer XWayland whenever DISPLAY is available (normal GNOME/Plasma/Cosmic sessions).
    #[cfg(target_os = "linux")]
    if std::env::var_os("DISPLAY").is_some() {
        options.event_loop_builder = Some(Box::new(|builder| {
            use winit::platform::x11::EventLoopBuilderExtX11 as _;
            builder.with_x11();
        }));
        #[cfg(feature = "native-runtime")]
        sqyre_capture::note("ui: X11/XWayland event loop (tray hide + overlay Alt-Tab)");
    }
    let result = eframe::run_native(
        assets::APP_ID,
        options,
        Box::new(move |cc| {
            let mut app = SqyreApp::load();
            app.instance_lock = instance_lock;
            SettingsUi::install_fonts(&cc.egui_ctx);
            SettingsUi::apply_appearance(&cc.egui_ctx, app.settings_ui.settings());
            app.bind_hotkey_repaint(cc.egui_ctx.clone());
            app.tray = tray::SystemTray::install(cc.egui_ctx.clone(), cc.winit_window().cloned());
            Ok(Box::new(app))
        }),
    );
    #[cfg(feature = "native-runtime")]
    sqyre_capture::mark_site("app:run_native:returned");
    result
}

/// Handle `--version` / `--help` before starting the GUI.
///
/// Returns `true` when the process should exit without opening a window.
#[cfg(not(target_arch = "wasm32"))]
pub fn handle_cli_args() -> bool {
    let args: Vec<String> = std::env::args().skip(1).collect();
    handle_cli_args_from(&args)
}

/// Parse CLI flags (tests pass argv without `argv[0]`).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn handle_cli_args_from(args: &[String]) -> bool {
    let Some(arg) = args.first().map(String::as_str) else {
        return false;
    };
    match arg {
        "--version" | "-V" => {
            println!("{}", crate::update::SQYRE_VERSION);
            true
        }
        "--help" | "-h" => {
            println!("Sqyre — desktop macro automation");
            println!("Usage: sqyre [--version | --help]");
            true
        }
        _ => false,
    }
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
    command_palette: CommandPaletteUi,
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
    macro_overlay: sqyre_overlay::MacroOverlay,
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
    /// Background portal ScreenCast probe (Linux Wayland — must not block startup).
    #[cfg(all(
        not(target_arch = "wasm32"),
        feature = "native-runtime",
        target_os = "linux"
    ))]
    capture_probe_pending: Option<std::sync::mpsc::Receiver<Result<(), String>>>,
    /// True after the deferred portal probe has been started (or skipped).
    #[cfg(all(
        not(target_arch = "wasm32"),
        feature = "native-runtime",
        target_os = "linux"
    ))]
    capture_probe_finished: bool,
    /// Do not start portal ScreenCast before this instant (lets the first frames stay responsive).
    #[cfg(all(
        not(target_arch = "wasm32"),
        feature = "native-runtime",
        target_os = "linux"
    ))]
    capture_probe_not_before: Option<std::time::Instant>,
    /// Wayland: start evdev after the ScreenCast picker so device fds do not steal clicks.
    #[cfg(all(
        not(target_arch = "wasm32"),
        feature = "native-runtime",
        target_os = "linux"
    ))]
    hotkeys_deferred: Option<HotkeyCallbacks>,
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
        #[cfg(all(feature = "native-runtime", target_os = "linux"))]
        screen_click.set_absolute_pos(sqyre_capture::portal_cursor_position);
        let run = RunState::default();
        let stop = run.stop.clone();
        let pending_hotkey_macros = Arc::new(Mutex::new(Vec::new()));
        let pending_for_cb = Arc::clone(&pending_hotkey_macros);
        let hotkey_repaint = Arc::new(Mutex::new(None::<egui::Context>));
        let repaint_for_cb = Arc::clone(&hotkey_repaint);

        #[cfg(not(target_arch = "wasm32"))]
        let hotkey_callbacks = HotkeyCallbacks {
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
                crate::hotkey_wake::queue_macro_hotkey(&pending_for_cb, &repaint_for_cb, name);
            }),
        };
        #[cfg(all(
            not(target_arch = "wasm32"),
            feature = "native-runtime",
            target_os = "linux"
        ))]
        let hotkeys_deferred = if sqyre_capture::shared_capturer_open_may_block() {
            Some(hotkey_callbacks)
        } else {
            if let Err(e) = hotkeys.start(hotkey_callbacks) {
                crate::log::warn(format!("failed to start global hotkeys: {e}"));
            }
            None
        };
        #[cfg(all(
            not(target_arch = "wasm32"),
            not(all(feature = "native-runtime", target_os = "linux"))
        ))]
        if let Err(e) = hotkeys.start(hotkey_callbacks) {
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
                        sqyre_capture::linux_session_capture_warning().or(ocr_warning)
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
            command_palette: CommandPaletteUi::default(),
            data_editor: DataEditor::default(),
            settings_ui,
            variables_panel: variables_panel::VariablesPanelUi::default(),
            hidden_for_recording: false,
            #[cfg(feature = "native-runtime")]
            recording_overlay: recording_overlay::RecordingOverlay::new(),
            #[cfg(feature = "native-runtime")]
            macro_overlay: sqyre_overlay::MacroOverlay::new(),
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
            #[cfg(all(
                not(target_arch = "wasm32"),
                feature = "native-runtime",
                target_os = "linux"
            ))]
            capture_probe_pending: None,
            #[cfg(all(
                not(target_arch = "wasm32"),
                feature = "native-runtime",
                target_os = "linux"
            ))]
            capture_probe_finished: false,
            #[cfg(all(
                not(target_arch = "wasm32"),
                feature = "native-runtime",
                target_os = "linux"
            ))]
            capture_probe_not_before: None,
            #[cfg(all(
                not(target_arch = "wasm32"),
                feature = "native-runtime",
                target_os = "linux"
            ))]
            hotkeys_deferred,
            #[cfg(not(target_arch = "wasm32"))]
            update: update::UpdateManager::default(),
        };
        app.refresh_macro_hotkey_bindings();
        #[cfg(not(target_arch = "wasm32"))]
        app.maybe_start_update_check();
        app
    }

    #[cfg(all(
        not(target_arch = "wasm32"),
        feature = "native-runtime",
        target_os = "linux"
    ))]
    pub(crate) fn start_deferred_hotkeys(&mut self) {
        let Some(callbacks) = self.hotkeys_deferred.take() else {
            return;
        };
        if let Err(e) = self.hotkeys.start(callbacks) {
            crate::log::warn(format!("failed to start global hotkeys: {e}"));
        }
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
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        #[cfg(not(target_arch = "wasm32"))]
        self.tray.poll_commands(ctx, frame);
        // Unmap as soon as the WM asks to close so portal/tray/wgpu teardown
        // is not user-visible (Drop alone was finishing in <1s while the window
        // stayed up for ~2s afterward).
        #[cfg(all(feature = "native-runtime", not(target_arch = "wasm32")))]
        if ctx.input(|i| i.viewport().close_requested()) {
            sqyre_capture::set_process_exiting();
            sqyre_capture::mark_site("app:close_requested");
            if let Some(win) = frame.winit_window().or(self.tray.root_window()) {
                win.set_visible(false);
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.take_pending_db_import();
        // Poll background tasks before floating windows so Settings sees fresh update state.
        ui_overlays::sync_frame_state(self, ui.ctx());
        #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
        if self.hidden_for_recording
            && (self.screen_click.is_armed() || self.macro_record_bridge.is_armed())
        {
            // Poll grab/snapshot while the main window is unmapped. If the
            // completing click lands this frame, fall through so visibility is
            // restored and Data Editor can take the recorded point/area —
            // otherwise the wake poller stops and the frozen cover stays up.
            self.sync_recording_overlay(ui.ctx());
            if self.screen_click.is_armed() || self.macro_record_bridge.is_armed() {
                return;
            }
            self.update_recording_visibility(ui.ctx());
        }
        #[cfg(not(target_arch = "wasm32"))]
        if self.tray.application_hidden() {
            return;
        }
        ui_overlays::show_floating_windows(self, ui.ctx());
        ui_overlays::handle_shortcuts(self, ui);
        ui_overlays::show_command_palette(self, ui.ctx());

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
        #[cfg(all(feature = "native-runtime", not(target_arch = "wasm32")))]
        {
            sqyre_capture::set_process_exiting();
            sqyre_capture::mark_site("app:drop:start");
            // Belt-and-suspenders if close_requested was missed (e.g. tray Quit).
            if let Some(win) = self.tray.root_window() {
                win.set_visible(false);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        sqyre_input::release_held_inputs();
        #[cfg(all(feature = "native-runtime", not(target_arch = "wasm32")))]
        let t_hk = web_time::Instant::now();
        self.hotkeys.stop();
        #[cfg(all(feature = "native-runtime", not(target_arch = "wasm32")))]
        {
            sqyre_capture::cap_log(
                "APP",
                "drop",
                &format!("hotkeys_ms={}", t_hk.elapsed().as_millis()),
            );
            sqyre_capture::mark_site("app:drop:after_hotkeys");
        }

        // Tear down known-slow fields before automatic drop order so we get
        // timed sites if quit hangs (tray dbus, X11 outline under XWayland games,
        // portal kick/PipeWire join).
        #[cfg(all(feature = "native-runtime", not(target_arch = "wasm32")))]
        {
            let t = web_time::Instant::now();
            self.tray = tray::SystemTray::default();
            sqyre_capture::cap_log(
                "APP",
                "drop",
                &format!("tray_ms={}", t.elapsed().as_millis()),
            );
            sqyre_capture::mark_site("app:drop:after_tray");

            let t = web_time::Instant::now();
            self.macro_overlay = sqyre_overlay::MacroOverlay::new();
            sqyre_capture::cap_log(
                "APP",
                "drop",
                &format!("macro_overlay_ms={}", t.elapsed().as_millis()),
            );
            sqyre_capture::mark_site("app:drop:after_macro_overlay");

            let t = web_time::Instant::now();
            self.recording_overlay = recording_overlay::RecordingOverlay::new();
            sqyre_capture::cap_log(
                "APP",
                "drop",
                &format!("recording_overlay_ms={}", t.elapsed().as_millis()),
            );
            sqyre_capture::mark_site("app:drop:after_recording_overlay");

            let t = web_time::Instant::now();
            self.preview_tooltips = preview_tooltip::PreviewTooltipCache::new();
            sqyre_capture::cap_log(
                "APP",
                "drop",
                &format!("preview_ms={}", t.elapsed().as_millis()),
            );
            sqyre_capture::mark_site("app:drop:after_preview");

            let t = web_time::Instant::now();
            sqyre_capture::reset_shared_capturer();
            sqyre_capture::cap_log(
                "APP",
                "drop",
                &format!("capturer_ms={}", t.elapsed().as_millis()),
            );
            sqyre_capture::mark_site("app:drop:done");
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod cli_tests {
    use super::handle_cli_args_from;

    #[test]
    fn version_and_help_are_consumed() {
        assert!(handle_cli_args_from(&["--version".into()]));
        assert!(handle_cli_args_from(&["-V".into()]));
        assert!(handle_cli_args_from(&["--help".into()]));
        assert!(handle_cli_args_from(&["-h".into()]));
        assert!(!handle_cli_args_from(&[]));
        assert!(!handle_cli_args_from(&["--unknown".into()]));
    }
}
