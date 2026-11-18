//! Sound settings section.

use super::common::*;
use super::*;

impl SettingsUi {
    pub(super) fn draw_sound(&mut self, ui: &mut egui::Ui, q: &str, section_hit: bool) {
        if setting_visible(q, section_hit, SETTING_FINISH_SOUND)
            && dirty_checkbox(
                ui,
                &mut self.settings.sound.play_finish_sound,
                "Play a sound when a macro finishes",
                ht::SET_FINISH_CUE,
            )
        {
            self.mark_dirty();
        }

        if setting_visible(q, section_hit, SETTING_UI_SOUNDS) {
            let mut changed = dirty_checkbox(
                ui,
                &mut self.settings.sound.play_add_sound,
                "Play a sound when adding",
                ht::SET_ADD_DEL_CUES,
            );
            changed |= dirty_checkbox(
                ui,
                &mut self.settings.sound.play_delete_sound,
                "Play a sound when deleting",
                ht::SET_ADD_DEL_CUES,
            );
            if changed {
                self.mark_dirty();
            }
        }

        if setting_visible(q, section_hit, SETTING_SOUND_VOLUME) {
            ui.horizontal(|ui| {
                ui.label("Sound volume:");
                let mut pct = (self.settings.sound.sound_volume * 100.0).round() as i32;
                if ui
                    .add(egui::Slider::new(&mut pct, 0..=100).suffix("%"))
                    .on_hover_text(ht::SET_CUE_VOLUME)
                    .changed()
                {
                    self.settings.sound.sound_volume = pct as f32 / 100.0;
                    self.mark_dirty();
                }
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        if setting_visible(q, section_hit, SETTING_SOUND_FILES) {
            self.sound_file_row(
                ui,
                "Finish sound:",
                PickPurpose::FinishSound,
                ht::SET_FINISH_FILE,
                |s| &mut s.sound.finish_sound_file,
            );
            self.sound_file_row(
                ui,
                "Add sound:",
                PickPurpose::AddSound,
                ht::SET_ADD_FILE,
                |s| &mut s.sound.add_sound_file,
            );
            self.sound_file_row(
                ui,
                "Delete sound:",
                PickPurpose::DeleteSound,
                ht::SET_DELETE_FILE,
                |s| &mut s.sound.delete_sound_file,
            );
        }
    }

    /// Cue-sound file row: current file name, a picker, and a built-in reset.
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn sound_file_row(
        &mut self,
        ui: &mut egui::Ui,
        label: &str,
        purpose: PickPurpose,
        help: &str,
        field: impl Fn(&mut UserSettings) -> &mut String,
    ) {
        ui.horizontal(|ui| {
            ui.label(label).on_hover_text(help);
            let path = field(&mut self.settings);
            let name = if path.trim().is_empty() {
                "built-in".to_string()
            } else {
                std::path::Path::new(path.trim())
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.trim().to_string())
            };
            ui.label(egui::RichText::new(name).weak().small());
            if crate::file_dialogs::supported(purpose)
                && settings_choose_btn(ui, "Choose…").clicked()
            {
                crate::file_dialogs::request(purpose, "Choose cue sound", None);
            }
            if !field(&mut self.settings).trim().is_empty()
                && crate::widgets::icon_button(
                    ui,
                    egui_phosphor::regular::HOUSE,
                    "Use built-in sound",
                )
                .clicked()
            {
                field(&mut self.settings).clear();
                self.mark_dirty();
            }
        });
    }
}
