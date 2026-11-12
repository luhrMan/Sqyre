//! Drain native overlay-button context-menu actions into the Data Editor.

use crate::data_editor::EditorTab;
use crate::SqyreApp;
use eframe::egui;
use sqyre_overlay::OverlayButtonContextAction;

impl SqyreApp {
    /// Apply Open settings / Disable picks from overlay hosts.
    #[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
    pub(crate) fn drain_overlay_context_actions(&mut self, ctx: &egui::Context) {
        let actions = self.macro_overlay.drain_context_actions();
        if actions.is_empty() {
            return;
        }
        for action in actions {
            self.apply_overlay_context_action(ctx, action);
        }
    }

    #[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
    fn apply_overlay_context_action(
        &mut self,
        ctx: &egui::Context,
        action: OverlayButtonContextAction,
    ) {
        match action {
            OverlayButtonContextAction::OpenSettings { button_id } => {
                let Some(program) = self
                    .settings_ui
                    .settings()
                    .overlay_buttons
                    .iter()
                    .find(|b| b.id == button_id)
                    .map(|b| b.program.clone())
                else {
                    return;
                };
                self.focus_main_window(ctx);
                self.data_editor.open_entity(
                    ctx,
                    EditorTab::Overlay,
                    &program,
                    &button_id,
                    &self.workspace.catalog,
                    self.settings_ui.settings(),
                );
            }
            OverlayButtonContextAction::Disable { button_id } => {
                let settings = self.settings_ui.settings_mut();
                if self
                    .data_editor
                    .set_overlay_button_enabled(settings, &button_id, false)
                {
                    let _ = self.data_editor.persist_overlay_settings(settings);
                }
            }
        }
    }

    /// Bring the main window forward (tray-hidden or backgrounded).
    #[cfg(all(feature = "native-runtime", feature = "overlay-buttons"))]
    fn focus_main_window(&mut self, ctx: &egui::Context) {
        #[cfg(target_os = "android")]
        {
            let _ = ctx;
            sqyre_android::bridge::show_shell();
        }
        #[cfg(not(target_os = "android"))]
        {
            #[cfg(not(target_arch = "wasm32"))]
            self.tray.show(ctx);
            #[cfg(target_arch = "wasm32")]
            {
                let _ = ctx;
            }
        }
    }
}
