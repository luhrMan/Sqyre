//! General settings section.

use super::common::*;
use super::*;

impl SettingsUi {
    pub(super) fn draw_general(&mut self, ui: &mut egui::Ui, q: &str, section_hit: bool) {
        if setting_visible(q, section_hit, SETTING_LOG_META) {
            ui.horizontal(|ui| {
                if crate::action_tooltip::help::tip(
                    ui.checkbox(
                        &mut self.settings.runtime.save_meta_images,
                        "Log Meta Images",
                    ),
                    crate::action_tooltip::help::SETTING_LOG_META,
                )
                .changed()
                {
                    self.mark_dirty();
                }
                ui.label("Keep at most:");
                let mut v = self.settings.runtime.action_log_max_entries;
                if ui
                    .add(
                        crate::widgets::num_entry(&mut v)
                            .range(
                                sqyre_persist::MIN_ACTION_LOG_MAX_ENTRIES
                                    ..=sqyre_persist::MAX_ACTION_LOG_MAX_ENTRIES,
                            )
                            .speed(100)
                            .suffix(" entries"),
                    )
                    .on_hover_text(ht::SET_LOG_CAP)
                    .changed()
                {
                    self.settings.runtime.action_log_max_entries = v;
                    self.mark_dirty();
                }
            });
        }

        #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
        if setting_visible(q, section_hit, SETTING_WORKER_THREADS) {
            ui.horizontal(|ui| {
                ui.label("CPU threads for image work:");
                let max = sqyre_persist::available_worker_threads();
                let mut v = self.settings.runtime.worker_threads;
                if ui
                    .add(
                        crate::widgets::num_entry(&mut v)
                            .range(sqyre_persist::MIN_WORKER_THREADS..=max)
                            .speed(1),
                    )
                    .on_hover_text(format!(
                        "How many CPU threads Sqyre may use for image search, find pixel, and capture conversion (1–{max} on this machine). Default is {} (16 or fewer). Restart Sqyre after changing.",
                        sqyre_persist::default_worker_threads()
                    ))
                    .changed()
                {
                    self.settings.runtime.worker_threads = v;
                    self.mark_dirty();
                }
                ui.label(format!("of {max}"));
            });
            ui.label(
                egui::RichText::new("Restart Sqyre for a new thread count to take effect.")
                    .small()
                    .weak(),
            );
        }

        if setting_visible(q, section_hit, SETTING_LAST_MACRO)
            && dirty_checkbox(
                ui,
                &mut self.settings.runtime.restore_last_macro,
                "Re-select the last used macro on launch",
                ht::SET_LAST_MACRO,
            )
        {
            self.mark_dirty();
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            if setting_visible(q, section_hit, SETTING_CLOSE_TO_TRAY)
                && dirty_checkbox(
                    ui,
                    &mut self.settings.data.close_to_tray,
                    "Closing the window minimizes to the tray",
                    ht::SET_CLOSE_TO_TRAY,
                )
            {
                self.mark_dirty();
            }

            if setting_visible(q, section_hit, SETTING_START_MINIMIZED)
                && dirty_checkbox(
                    ui,
                    &mut self.settings.data.start_minimized,
                    "Start in the tray",
                    ht::SET_START_MINIMIZED,
                )
            {
                self.mark_dirty();
            }

            #[cfg(target_os = "linux")]
            if setting_visible(q, section_hit, SETTING_AUTOSTART)
                && dirty_checkbox(
                    ui,
                    &mut self.settings.data.launch_on_login,
                    "Launch on login",
                    ht::SET_AUTOSTART,
                )
            {
                self.mark_dirty();
            }

            #[cfg(all(target_os = "linux", feature = "native-runtime"))]
            if setting_visible(q, section_hit, SETTING_SUSPEND_CAPTURE)
                && dirty_checkbox(
                    ui,
                    &mut self.settings.runtime.suspend_screen_capture_when_idle,
                    "Sleep screen capture when idle",
                    ht::SET_SUSPEND_CAPTURE,
                )
            {
                self.mark_dirty();
            }
        }

        #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
        {
            let show_diag = setting_visible(q, section_hit, SETTING_DIAGNOSTICS);
            let show_mem = setting_visible(q, section_hit, SETTING_MEMORY_LOG);
            if show_diag || show_mem {
                ui.label(egui::RichText::new("Diagnostics").strong());
            }
            if show_diag
                && dirty_checkbox(
                    ui,
                    &mut self.settings.runtime.diagnostic_logging,
                    "Write diagnostic log (diag.log)",
                    ht::SET_DIAG_LOG,
                )
            {
                self.mark_dirty();
            }
            if show_mem {
                ui.horizontal(|ui| {
                    if dirty_checkbox(
                        ui,
                        &mut self.settings.runtime.memory_logging,
                        "Sample memory (mem.log)",
                        ht::SET_MEM_LOG,
                    ) {
                        self.mark_dirty();
                    }
                    ui.label("every");
                    let mut v = self.settings.runtime.memory_logging_interval_ms;
                    if ui
                        .add(
                            crate::widgets::num_entry(&mut v)
                                .range(
                                    sqyre_persist::MIN_MEMORY_SAMPLE_INTERVAL_MS
                                        ..=sqyre_persist::MAX_MEMORY_SAMPLE_INTERVAL_MS,
                                )
                                .speed(500)
                                .suffix(" ms"),
                        )
                        .on_hover_text(ht::SET_MEM_INTERVAL)
                        .changed()
                    {
                        self.settings.runtime.memory_logging_interval_ms = v;
                        self.mark_dirty();
                    }
                });
            }
        }
    }
}
