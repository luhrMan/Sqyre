//! Docs / screenshot helpers for in-memory demo app state.

use crate::action_logs_ui::LogsImageCache;
use crate::add_action::AddActionPicker;
use crate::app_backends::RunState;
use crate::data_editor::DataEditor;
use crate::hotkey_record::HotkeyRecordUi;
use crate::icon_cache::IconCache;
use crate::key_record::KeyRecordUi;
use crate::macro_meta::MacroMetaUi;
use crate::macro_overlay::MacroOverlay;
use crate::preview_tooltip::PreviewTooltipCache;
use crate::recording_overlay::RecordingOverlay;
use crate::run_session::RunSession;
use crate::settings::SettingsUi;
use crate::tree_state::TreeState;
use crate::variables_panel;
use crate::workspace::Workspace;
use crate::{docs_fixture, tray, SqyreApp};
use eframe::egui;
use parking_lot::Mutex;
use sqyre_executor::{SharedActionLog, SharedHighlighter, SharedRuntimeVars};
use sqyre_hotkeys::{HotkeyService, NullHotkeys, ScreenClickBridge};
use sqyre_persist::UserSettings;
use std::sync::Arc;

impl SqyreApp {
    pub fn for_docs() -> Self {
        // Keep docs/screenshot/kittest harnesses at 1.0 PPP. Product default scale
        // may be higher for desktop readability, but AccessKit pointer clicks and
        // golden PNGs assume unscaled coordinates.
        let settings = UserSettings {
            ui_scale: 1.0,
            ..UserSettings::default()
        };
        SettingsUi::apply_action_colors(&settings);

        let hotkeys: Box<dyn HotkeyService> = Box::new(NullHotkeys::default());
        let continue_wait = sqyre_hotkeys::ContinueWaitBridge::new(false);
        let screen_click = ScreenClickBridge::new();
        let macro_hotkeys = sqyre_hotkeys::MacroHotkeyBridge::new();
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

        let tree = TreeState {
            exec_fully_expanded: true,
            ..TreeState::default()
        };

        let mut app = Self {
            workspace: Workspace {
                db,
                macros,
                catalog,
                load_error: None,
                platform_warning: None,
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
                logs_image_cache: LogsImageCache::default(),
            },
            tree,
            hotkeys,
            screen_click,
            pending_hotkey_macros,
            hotkey_repaint,
            hotkey_record: HotkeyRecordUi::default(),
            key_record: KeyRecordUi::default(),
            icon_cache: IconCache::new(),
            preview_tooltips: PreviewTooltipCache::new(),
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
            pending_import: crate::wasm_io::new_pending_import(),
            #[cfg(not(target_arch = "wasm32"))]
            backup_task: None,
            #[cfg(not(target_arch = "wasm32"))]
            pixel_sample_pending: None,
            #[cfg(not(target_arch = "wasm32"))]
            update: crate::update::UpdateManager::default(),
        };
        if let Some(m) = app.workspace.macros.first() {
            app.workspace.macro_meta.sync_selection(0, m);
        }
        app
    }

    pub fn open_add_action_picker(&mut self) {
        self.add_action_picker.open();
    }

    pub fn open_data_editor(&mut self) {
        self.data_editor.open = true;
        self.data_editor
            .select_program_for_docs("Demo Program", &self.workspace.catalog);
    }

    pub fn select_action(&mut self, id: sqyre_domain::ActionId) {
        self.select_one_action(id);
    }

    /// First top-level action under the demo macro root (skips the root loop).
    pub fn demo_first_action_id(&self) -> Option<sqyre_domain::ActionId> {
        self.workspace
            .macros
            .first()?
            .root
            .children()
            .first()
            .map(|a| a.id)
    }

    /// First image-search action in the selected macro tree.
    pub fn demo_image_search_id(&self) -> Option<sqyre_domain::ActionId> {
        let m = self.workspace.macros.get(self.workspace.selected_macro)?;
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
        self.tree.exec_fully_expanded = true;
    }

    /// Settings used for docs appearance (fonts / scale).
    pub fn docs_settings(&self) -> &UserSettings {
        self.settings_ui.settings()
    }

    /// Open the settings window (integration / screenshot harnesses).
    pub fn open_settings_for_docs(&mut self) {
        self.settings_ui.open = true;
    }

    /// Show the macro list panel (docs / interaction harnesses).
    pub fn open_macro_list_for_docs(&mut self) {
        self.macro_list_open = true;
    }

    /// Number of macros currently loaded (docs / interaction harnesses).
    pub fn docs_macro_count(&self) -> usize {
        self.workspace.macros.len()
    }

    /// Selected macro name (docs / interaction harnesses).
    pub fn docs_selected_macro_name(&self) -> Option<&str> {
        self.workspace
            .macros
            .get(self.workspace.selected_macro)
            .map(|m| m.name.as_str())
    }
}
