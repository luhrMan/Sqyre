//! Docs / screenshot helpers for in-memory demo app state.

use crate::action_logs_ui::LogsImageCache;
use crate::action_tooltip::TooltipState;
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
use crate::settings::SettingsUi;
use crate::ui_macro_tree::TreeDragMode;
use crate::variables_panel;
use crate::{docs_fixture, tray, SqyreApp};
use eframe::egui;
use parking_lot::Mutex;
use sqyre_domain::ActionId;
use sqyre_executor::{SharedActionLog, SharedHighlighter, SharedRuntimeVars};
use sqyre_hotkeys::{
    ContinueWaitBridge, HotkeyService, MacroHotkeyBridge, NullHotkeys, ScreenClickBridge,
};
use sqyre_persist::UserSettings;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

impl SqyreApp {
    pub fn for_docs() -> Self {
        let mut settings = UserSettings::default();
        // Keep docs/screenshot/kittest harnesses at 1.0 PPP. Product default scale
        // may be higher for desktop readability, but AccessKit pointer clicks and
        // golden PNGs assume unscaled coordinates.
        settings.ui_scale = 1.0;
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
            pending_import: crate::wasm_io::new_pending_import(),
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
        self.macros.len()
    }

    /// Selected macro name (docs / interaction harnesses).
    pub fn docs_selected_macro_name(&self) -> Option<&str> {
        self.macros
            .get(self.selected_macro)
            .map(|m| m.name.as_str())
    }
}
