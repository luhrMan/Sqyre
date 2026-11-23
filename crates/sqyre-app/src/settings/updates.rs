//! Updates settings section.

use super::common::*;
use super::*;

impl SettingsUi {
    pub(super) fn draw_updates(
        &mut self,
        ui: &mut egui::Ui,
        update: &mut crate::update::UpdateManager,
        q: &str,
        section_hit: bool,
    ) {
        use crate::update::{UpdateState, SQYRE_VERSION};

        let flatpak = sqyre_update::is_flatpak_install();

        if setting_visible(q, section_hit, SETTING_UPDATE_VERSION) {
            ui.label(format!("Current version: {SQYRE_VERSION}"));
        }

        if flatpak {
            if setting_visible(q, section_hit, SETTING_UPDATE_ACTIONS) {
                ui.label(
                    "Flatpak install: updates come from flatpak update or your software center \
                     (in-app self-replace is not supported).",
                );
            }
            return;
        }

        if setting_visible(q, section_hit, SETTING_AUTO_UPDATE)
            && dirty_checkbox(
                ui,
                &mut self.settings.data.auto_update_check,
                "Check for updates on startup",
                ht::SET_UPDATE_CHECK,
            )
        {
            self.mark_dirty();
        }

        if setting_visible(q, section_hit, SETTING_UPDATE_INTERVAL) {
            ui.horizontal(|ui| {
                ui.label("Check for updates every:");
                let mut v = self.settings.data.update_check_interval_hours;
                if ui
                    .add(
                        crate::widgets::num_entry(&mut v)
                            .range(
                                sqyre_persist::MIN_UPDATE_CHECK_INTERVAL_HOURS
                                    ..=sqyre_persist::MAX_UPDATE_CHECK_INTERVAL_HOURS,
                            )
                            .speed(1)
                            .suffix(" hours"),
                    )
                    .on_hover_text(ht::SET_UPDATE_INTERVAL)
                    .changed()
                {
                    self.settings.data.update_check_interval_hours = v;
                    self.mark_dirty();
                }
            });
            ui.label(
                egui::RichText::new("0 hours checks once per launch.")
                    .small()
                    .weak(),
            );
        }

        if setting_visible(q, section_hit, SETTING_UPDATE_PRERELEASE)
            && dirty_checkbox(
                ui,
                &mut self.settings.data.update_include_prerelease,
                "Include pre-release versions",
                ht::SET_UPDATE_PRERELEASE,
            )
        {
            self.mark_dirty();
        }

        if setting_visible(q, section_hit, SETTING_UPDATE_ACTIONS) {
            ui.horizontal(|ui| {
                let busy = update.is_busy();
                if ui
                    .add_enabled(
                        !busy,
                        crate::widgets::IconButton::new(
                            egui_phosphor::regular::ARROW_CLOCKWISE,
                            ht::SET_UPDATE_NOW,
                            None,
                        ),
                    )
                    .clicked()
                {
                    update.start_check(self.settings.data.update_include_prerelease);
                    ui.ctx().request_repaint();
                }
                match &update.state {
                    UpdateState::Available { .. }
                        if ui
                            .add_enabled(
                                !busy,
                                crate::widgets::IconButton::new(
                                    egui_phosphor::regular::DOWNLOAD_SIMPLE,
                                    "Download & install",
                                    None,
                                ),
                            )
                            .clicked() =>
                    {
                        update.start_download();
                    }
                    UpdateState::Ready { .. }
                        if crate::widgets::icon_button(
                            ui,
                            egui_phosphor::regular::ARROW_CLOCKWISE,
                            "Restart to finish",
                        )
                        .clicked() =>
                    {
                        self.restart_requested = true;
                    }
                    _ => {}
                }
            });

            let status = match &update.state {
                UpdateState::Idle => "Update status: idle".to_string(),
                UpdateState::Unavailable { reason } => reason.clone(),
                UpdateState::Checking => "Checking for updates…".to_string(),
                UpdateState::UpToDate => "You are up to date.".to_string(),
                UpdateState::Available { version, .. } => {
                    format!("Update available: v{version}")
                }
                UpdateState::Downloading { version } => {
                    format!("Downloading v{version}…")
                }
                UpdateState::Ready { version } => {
                    format!("v{version} installed. Restart to finish.")
                }
                UpdateState::Failed { message } => format!("Update check failed: {message}"),
            };
            let busy = matches!(
                update.state,
                UpdateState::Checking | UpdateState::Downloading { .. }
            );
            if busy {
                crate::widgets::loading_line(ui, egui::RichText::new(status).weak().small());
            } else {
                ui.label(egui::RichText::new(status).weak().small());
            }
        }
    }
}
