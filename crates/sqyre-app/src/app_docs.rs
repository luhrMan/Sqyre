//! Docs / screenshot helpers for in-memory demo app state.

use crate::action_logs_ui::LogsWindowState;
use crate::add_action::AddActionPicker;
use crate::app_backends::RunState;
use crate::data_editor::{DataEditor, EditorTab};
use crate::hotkey_record::HotkeyRecordUi;
use crate::icon_cache::IconCache;
use crate::key_record::KeyRecordUi;
use crate::macro_meta::MacroMetaUi;
use crate::macro_record::MacroRecordUi;
use crate::preview_tooltip::PreviewTooltipCache;
#[cfg(feature = "native-runtime")]
use crate::recording_overlay::RecordingOverlay;
use crate::run_session::RunSession;
use crate::settings::SettingsUi;
#[cfg(not(target_arch = "wasm32"))]
use crate::tray;
use crate::tree_state::TreeState;
use crate::variables_panel;
use crate::workspace::Workspace;
use crate::{docs_fixture, SqyreApp};
use eframe::egui;
use parking_lot::Mutex;
use sqyre_domain::{blank_action, Action, ActionKind, Macro};
use sqyre_hotkeys::{HotkeyService, MacroRecordBridge, NullHotkeys, ScreenClickBridge};
#[cfg(feature = "native-runtime")]
use sqyre_overlay::MacroOverlay;
use sqyre_persist::UserSettings;
use sqyre_ports::{SharedActionLog, SharedHighlighter, SharedRuntimeVars};
use std::sync::Arc;

const DOCS_EDITOR_TABS: &[(&str, EditorTab)] = &[
    ("programs", EditorTab::Programs),
    ("items", EditorTab::Items),
    ("masks", EditorTab::Masks),
    ("screencap", EditorTab::ScreenCap),
    ("pixelcheck", EditorTab::PixelCheck),
    ("points", EditorTab::Points),
    ("searchareas", EditorTab::SearchAreas),
    ("collections", EditorTab::Collections),
    ("atlases", EditorTab::Atlases),
    ("overlay", EditorTab::Overlay),
];

fn find_action(macro_: &Macro, type_key: &str) -> Option<Action> {
    let mut found = None;
    for child in macro_.root.children() {
        child.walk(&mut |action| {
            if found.is_none() && action.type_key() == type_key {
                found = Some(action.clone());
            }
        });
        if found.is_some() {
            break;
        }
    }
    found
}

fn walk_actions_mut(action: &mut Action, visit: &mut impl FnMut(&mut Action)) {
    visit(action);
    if let Some(children) = action.children_mut() {
        for child in children {
            walk_actions_mut(child, visit);
        }
    }
    if let Some(children) = action.else_children_mut() {
        for child in children {
            walk_actions_mut(child, visit);
        }
    }
}

#[doc(hidden)]
impl SqyreApp {
    pub fn for_docs() -> Self {
        // Keep docs/screenshot/kittest harnesses at 1.0 PPP: AccessKit pointer clicks
        // and golden PNGs assume unscaled coordinates.
        let settings = UserSettings {
            ui_scale: 1.0,
            // Headless kittest/docs must not touch Pulse, GitHub, or portal ScreenCast.
            play_ui_sounds: false,
            play_finish_sound: false,
            auto_update_check: false,
            // Continuous repaint keeps `Harness::run` from settling; goldens stay deterministic.
            run_button_glow: false,
            ..UserSettings::default()
        };
        SettingsUi::apply_action_colors(&settings);

        let hotkeys: Box<dyn HotkeyService> = Box::new(NullHotkeys::default());
        #[cfg(any(feature = "native-runtime", target_os = "windows"))]
        let continue_wait =
            sqyre_hotkeys::ContinueWaitBridge::new(sqyre_hotkeys::ContinueSource::Unavailable);
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
        action_log.set_log_verbose(settings_ui.settings().save_meta_images);
        action_log.set_max_entries(settings_ui.settings().action_log_max_entries.max(1) as usize);
        let mut add_action_picker = AddActionPicker::default();
        add_action_picker.load_from_settings(settings_ui.settings());

        let catalog = docs_fixture::demo_catalog();
        let macros = docs_fixture::demo_macros();
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
                hotkey_tag_filters: settings_ui.settings().hotkey_tag_filters.clone(),
            },
            run_session: RunSession {
                state: run,
                #[cfg(any(feature = "native-runtime", target_os = "windows"))]
                continue_wait,
                macro_hotkeys,
                action_log,
                runtime_vars: SharedRuntimeVars::new(),
                highlighter,
                click_fx: sqyre_ports::SharedClickFx::new(),
                logs_window: None,
                logs_state: LogsWindowState::default(),
            },
            tree,
            hotkeys,
            screen_click,
            macro_record_bridge: MacroRecordBridge::new(),
            pending_hotkey_macros,
            hotkey_repaint,
            hotkey_record: HotkeyRecordUi::default(),
            key_record: KeyRecordUi::default(),
            macro_record: MacroRecordUi::default(),
            icon_cache: IconCache::new(),
            preview_tooltips: PreviewTooltipCache::new(),
            add_action_picker,
            command_palette: crate::command_palette::CommandPaletteUi::default(),
            data_editor: DataEditor::default(),
            settings_ui,
            variables_panel: variables_panel::VariablesPanelUi::default(),
            macro_prompt_builder: crate::macro_prompt_builder::MacroPromptBuilderUi::default(),
            macro_yaml_builder: crate::macro_yaml_builder::MacroYamlBuilderUi::default(),
            #[cfg(not(target_arch = "wasm32"))]
            hidden_for_recording: false,
            #[cfg(feature = "native-runtime")]
            recording_overlay: RecordingOverlay::new(),
            #[cfg(feature = "native-runtime")]
            click_fx_overlay: crate::click_fx::ClickFxOverlay::new(),
            #[cfg(all(target_os = "android", feature = "native-runtime"))]
            screen_pick: Default::default(),
            #[cfg(all(target_os = "android", feature = "native-runtime"))]
            pending_projection_run: None,
            #[cfg(feature = "native-runtime")]
            macro_overlay: MacroOverlay::new(),
            #[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
            overlay_visibility: crate::overlay_visibility::OverlayVisibilityPoller::new(),
            #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
            hotkey_focus_tags: crate::hotkey_focus_tags::HotkeyFocusTagPoller::new(),
            hotkey_chooser: None,
            start_when_idle: None,
            // Match product default so main-window goldens include the Macros sidebar.
            macro_list_open: true,
            macro_list_filter: String::new(),
            #[cfg(not(target_arch = "wasm32"))]
            tray: tray::SystemTray::default(),
            #[cfg(not(target_arch = "wasm32"))]
            instance_lock: None,
            pending_delete_macro: None,
            last_viewport_content: None,
            pending_viewport_scale: None,
            pending_import: crate::wasm_io::new_pending_import(),
            #[cfg(not(target_arch = "wasm32"))]
            tasks: crate::BackgroundTasks::default(),
            #[cfg(not(target_arch = "wasm32"))]
            update: crate::update::UpdateManager::default(),
            // Screenshots never run the probe; mark it already finished.
            #[cfg(all(
                not(target_arch = "wasm32"),
                feature = "native-runtime",
                target_os = "linux"
            ))]
            portal_probe: crate::PortalProbe::finished(),
            #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
            hotkeys_deferred: None,
            #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
            permissions_intro: None,
        };
        if let Some(m) = app.workspace.macros.first() {
            app.workspace.macro_meta.sync_selection(0, m);
        }
        app
    }

    pub fn open_add_action_picker(&mut self) {
        self.add_action_picker.open();
    }

    pub fn open_command_palette_for_docs(&mut self) {
        self.command_palette.open_palette();
    }

    /// Whether the settings window is showing (docs / interaction harnesses).
    pub fn docs_settings_open(&self) -> bool {
        self.settings_ui.open
    }

    /// Whether the data editor window is showing (docs / interaction harnesses).
    pub fn docs_data_editor_open(&self) -> bool {
        self.data_editor.open
    }

    /// Program catalog (docs / interaction harnesses).
    pub fn docs_catalog(&self) -> &sqyre_persist::ProgramCatalog {
        &self.workspace.catalog
    }

    /// Whether the command palette is showing (docs / interaction harnesses).
    pub fn docs_command_palette_open(&self) -> bool {
        self.command_palette.is_open()
    }
    pub fn open_data_editor(&mut self) {
        // Coordinates → Points shows the post-ScreenCap tab strip with filled form data.
        self.data_editor
            .open_points_for_docs("Demo Program", "center", &self.workspace.catalog);
    }

    /// Open the data editor on the Programs tab (docs / interaction harnesses).
    /// Unlike Points/SearchAreas, New here does not arm screen-click recording.
    pub fn open_data_editor_programs_for_docs(&mut self) {
        self.data_editor
            .open_programs_for_docs(&self.workspace.catalog);
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

    /// Mutable settings for docs / interaction harnesses.
    pub fn docs_settings_mut(&mut self) -> &mut UserSettings {
        self.settings_ui.settings_mut()
    }

    /// Open the settings window (integration / screenshot harnesses).
    pub fn open_settings_for_docs(&mut self) {
        // Docs harness paints without a live ctx here; focus re-arms on next open cycle.
        self.settings_ui.open = true;
    }

    /// Open Settings → Appearance (docs screenshots of the sidebar layout).
    pub fn open_settings_appearance_for_docs(&mut self) {
        self.settings_ui.open_appearance_for_docs();
    }

    pub fn settings_section_labels_for_docs() -> Vec<&'static str> {
        SettingsUi::section_labels_for_docs()
    }

    pub fn open_settings_section_for_docs(&mut self, label: &str) {
        assert!(
            self.settings_ui.open_section_for_docs(label),
            "unknown settings section: {label}"
        );
    }

    pub fn data_editor_tab_names_for_docs() -> Vec<&'static str> {
        DOCS_EDITOR_TABS.iter().map(|(name, _)| *name).collect()
    }

    pub fn open_data_editor_tab_for_docs(&mut self, name: &str) {
        let tab = DOCS_EDITOR_TABS
            .iter()
            .find(|(tab_name, _)| *tab_name == name)
            .map(|(_, tab)| *tab)
            .unwrap_or_else(|| panic!("unknown data editor tab: {name}"));
        self.data_editor.open_tab_for_docs(
            tab,
            &self.workspace.catalog,
            self.settings_ui.settings(),
        );
    }

    pub fn open_variables_panel_for_docs(&mut self, tab: &str) {
        let builtins = match tab {
            "runtime" => false,
            "builtins" => true,
            _ => panic!("unknown variables panel tab: {tab}"),
        };
        self.variables_panel.open_for_docs(builtins);
    }

    pub fn open_action_editor_for_docs(&mut self, type_key: &str) {
        let macro_idx = self.workspace.selected_macro;
        let mut action = find_action(&self.workspace.macros[macro_idx], type_key);
        if action.is_none() {
            let blank = blank_action(type_key)
                .unwrap_or_else(|| panic!("unknown action type key: {type_key}"));
            let macro_ = &mut self.workspace.macros[macro_idx];
            if matches!(type_key, "loopjump" | "navigateselect") {
                let loop_idx = macro_
                    .root
                    .children()
                    .iter()
                    .position(|child| child.type_key() == "loop");
                if let Some(loop_idx) = loop_idx {
                    macro_.root.children_mut().unwrap()[loop_idx]
                        .children_mut()
                        .expect("loop action has children")
                        .push(blank.clone());
                } else {
                    let mut parent = blank_action("loop").expect("loop action");
                    parent
                        .children_mut()
                        .expect("loop action has children")
                        .push(blank.clone());
                    macro_
                        .root
                        .children_mut()
                        .expect("macro root has children")
                        .push(parent);
                }
            } else {
                macro_
                    .root
                    .children_mut()
                    .expect("macro root has children")
                    .push(blank.clone());
            }
            action = Some(blank);
        }
        self.tree.tooltip.open_edit(
            action.as_ref().expect("action opened"),
            egui::pos2(40.0, 80.0),
        );
    }

    pub fn apply_stress_fixture_for_docs(&mut self) {
        let macro_idx = self.workspace.selected_macro;
        {
            let macro_ = &mut self.workspace.macros[macro_idx];
            macro_.name = "Daily inventory restock and screenshot backup routine".into();
            macro_.tags = [
                "inventory",
                "restock",
                "screenshots",
                "backup",
                "nightly",
                "maintenance",
                "operations",
                "long-running-maintenance",
            ]
            .map(str::to_owned)
            .to_vec();
            if let Some(variable) = macro_.variable_decls.first_mut() {
                variable.description = "Stores the last verified inventory quantity and the operator who reconciled it before the nightly screenshot archive is written, so the maintenance run can resume safely after an interruption.".into();
            }
            walk_actions_mut(&mut macro_.root, &mut |action| match &mut action.kind {
                ActionKind::ImageSearch { name, .. } => {
                    *name = "Find the confirm purchase button in the shop dialog".into();
                }
                ActionKind::Type { text, .. } => {
                    *text = "Please confirm the purchase after checking the selected inventory count. Save a screenshot of the updated shop dialog, verify the backup completed, and leave a clear note for the next maintenance run.".into();
                }
                _ => {}
            });
        }
        let extra_names = [
            "Monthly stock reconciliation",
            "Capture a screenshot of the returns dashboard",
            "Archive completed purchase records",
            "Recheck low inventory notifications",
            "Prepare the next shift handoff",
            "Nightly backup for the item catalog",
        ];
        let extra_tags = [
            "inventory",
            "screenshots",
            "archive",
            "operations",
            "maintenance",
            "backup",
        ];
        for index in 0..30 {
            let mut extra = Macro::new(
                format!(
                    "{} {:02}",
                    extra_names[index % extra_names.len()],
                    index + 1
                ),
                0,
                Vec::new(),
            );
            extra.tags = vec![extra_tags[index % extra_tags.len()].into()];
            self.workspace.macros.push(extra);
        }
        self.workspace
            .macro_meta
            .sync_selection(macro_idx, &self.workspace.macros[macro_idx]);
    }

    pub fn open_logs_window_for_docs(&mut self) {
        if let Some(action_id) = self.demo_first_action_id() {
            self.run_session.logs_window = Some(action_id);
            self.select_action(action_id);
        }
    }

    pub fn open_yaml_builder_for_docs(&mut self) {
        let selected = self.workspace.macros.get(self.workspace.selected_macro);
        self.macro_yaml_builder.open_builder(selected);
    }

    pub fn open_ai_builder_for_docs(&mut self) {
        self.macro_prompt_builder.open_builder();
    }

    pub fn open_delay_dialog_for_docs(&mut self) {
        self.workspace.macro_meta.open_delay_for_docs();
    }

    /// Show the macro list panel (docs / interaction harnesses).
    pub fn open_macro_list_for_docs(&mut self) {
        self.macro_list_open = true;
    }

    pub fn set_macro_list_open_for_docs(&mut self, open: bool) {
        self.macro_list_open = open;
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

    /// Root-loop child count of the selected macro (docs / interaction harnesses).
    pub fn docs_selected_root_child_count(&self) -> usize {
        self.workspace
            .macros
            .get(self.workspace.selected_macro)
            .map(|m| m.root.children().len())
            .unwrap_or(0)
    }
}
