//! Macros settings section (playback, recording, hotkeys, defaults, image search).

use super::common::*;
use super::*;

impl SettingsUi {
    pub(super) fn draw_macros(
        &mut self,
        ui: &mut egui::Ui,
        q: &str,
        section_hit: bool,
        macro_hotkeys: &sqyre_hotkeys::MacroHotkeyBridge,
    ) {
        let mut first = true;
        self.draw_macro_playback(ui, q, section_hit, &mut first);
        self.draw_macro_recording(ui, q, section_hit, &mut first);
        self.draw_macro_hotkeys(ui, q, section_hit, &mut first, macro_hotkeys);
        self.draw_macro_image_search(ui, q, section_hit, &mut first);
        self.draw_macro_action_defaults(ui, q, section_hit, &mut first);
    }

    pub(super) fn draw_macro_playback(
        &mut self,
        ui: &mut egui::Ui,
        q: &str,
        section_hit: bool,
        first: &mut bool,
    ) {
        let hit = section_hit || query_matches(q, SUBSECTION_PLAYBACK);
        let show_highlight = setting_visible(q, hit, SETTING_HIGHLIGHT_ACTION);
        let show_click_fx = setting_visible(q, hit, SETTING_CLICK_FX);
        let show_release = setting_visible(q, hit, SETTING_RELEASE_HELD);
        let show_overlay_stop = setting_visible(q, hit, SETTING_OVERLAY_PRESS_STOPS);
        let show_overlay_detect = setting_visible(q, hit, SETTING_OVERLAY_IN_DETECTION);
        let show_overlay_over = setting_visible(q, hit, SETTING_OVERLAY_OVER_SQYRE);
        let show_cursor = setting_visible(q, hit, SETTING_RESTORE_CURSOR);
        let show_while = setting_visible(q, hit, SETTING_WHILE_BUDGET);
        let show_depth = setting_visible(q, hit, SETTING_RUN_MACRO_DEPTH);
        let show_flash_ms = setting_visible(q, hit, SETTING_CLICK_FLASH_MS);
        if !(show_highlight
            || show_click_fx
            || show_release
            || show_cursor
            || show_overlay_stop
            || show_overlay_detect
            || show_overlay_over
            || show_while
            || show_depth
            || show_flash_ms)
        {
            return;
        }
        subsection_heading(ui, first, "Playback");

        if show_highlight
            && dirty_checkbox(
                ui,
                &mut self.settings.runtime.highlight_active_action,
                "Highlight the currently executing action",
                ht::SET_RUN_ROW_HL,
            )
        {
            self.mark_dirty();
        }

        if show_click_fx
            && dirty_checkbox(
                ui,
                &mut self.settings.runtime.click_effect,
                "Flash a marker where clicks land",
                ht::SET_CLICK_MARKER,
            )
        {
            self.mark_dirty();
        }

        if show_release
            && dirty_checkbox(
                ui,
                &mut self.settings.runtime.release_held_inputs_on_end,
                "Release held keys and buttons when a macro ends",
                ht::SET_RELEASE_KEYS,
            )
        {
            self.mark_dirty();
        }

        #[cfg(any(feature = "native-runtime", target_os = "windows"))]
        if show_cursor
            && dirty_checkbox(
                ui,
                &mut self.settings.runtime.restore_cursor_on_end,
                "Restore the mouse position when a macro ends",
                ht::SET_RESTORE_CURSOR,
            )
        {
            self.mark_dirty();
        }

        if show_overlay_stop
            && dirty_checkbox(
                ui,
                &mut self.settings.overlay.overlay_press_stops_run,
                "Overlay buttons stop the running macro",
                ht::SET_OVL_STOP,
            )
        {
            self.mark_dirty();
        }

        if show_overlay_detect
            && dirty_checkbox(
                ui,
                &mut self.settings.overlay.overlay_buttons_in_detection,
                "Overlay buttons appear in detection",
                ht::SET_CAPTURE_BUTTONS,
            )
        {
            self.mark_dirty();
        }

        if show_overlay_over
            && dirty_checkbox(
                ui,
                &mut self.settings.overlay.overlay_buttons_over_sqyre,
                "Show overlay buttons over Sqyre",
                ht::SET_BUTTONS_FRONT,
            )
        {
            self.mark_dirty();
        }

        if show_while || show_depth || show_flash_ms {
            advanced_header(
                ui,
                "settings_macros_playback_advanced",
                q,
                &[
                    SETTING_WHILE_BUDGET,
                    SETTING_RUN_MACRO_DEPTH,
                    SETTING_CLICK_FLASH_MS,
                ],
                |ui| {
                    if show_flash_ms {
                        ui.horizontal(|ui| {
                            ui.label("Click marker stays visible for:");
                            let mut v = self.settings.runtime.click_flash_ms;
                            if ui
                                .add(
                                    crate::widgets::num_entry(&mut v)
                                        .range(
                                            sqyre_persist::MIN_CLICK_FLASH_MS
                                                ..=sqyre_persist::MAX_CLICK_FLASH_MS,
                                        )
                                        .speed(50)
                                        .suffix(" ms"),
                                )
                                .on_hover_text(ht::SET_CLICK_FLASH_MS)
                                .changed()
                            {
                                self.settings.runtime.click_flash_ms = v;
                                self.mark_dirty();
                            }
                        });
                    }

                    if show_while {
                        ui.horizontal(|ui| {
                            ui.label("Stop endless loops after:");
                            let mut v = self.settings.runtime.while_max_iterations;
                            if ui
                                .add(
                                    crate::widgets::num_entry(&mut v)
                                        .range(
                                            sqyre_persist::MIN_WHILE_MAX_ITERATIONS
                                                ..=sqyre_persist::MAX_WHILE_MAX_ITERATIONS,
                                        )
                                        .speed(1000)
                                        .suffix(" iterations"),
                                )
                                .on_hover_text(ht::SET_WHILE_BUDGET)
                                .changed()
                            {
                                self.settings.runtime.while_max_iterations = v;
                                self.mark_dirty();
                            }
                        });
                    }

                    if show_depth {
                        ui.horizontal(|ui| {
                            ui.label("Limit nested Run Macro calls:");
                            let mut v = self.settings.runtime.run_macro_max_depth;
                            if ui
                                .add(
                                    crate::widgets::num_entry(&mut v)
                                        .range(
                                            sqyre_persist::MIN_RUN_MACRO_MAX_DEPTH
                                                ..=sqyre_persist::MAX_RUN_MACRO_MAX_DEPTH,
                                        )
                                        .speed(1),
                                )
                                .on_hover_text(ht::SET_RUN_DEPTH)
                                .changed()
                            {
                                self.settings.runtime.run_macro_max_depth = v;
                                self.mark_dirty();
                            }
                        });
                    }
                },
            );
        }
    }

    pub(super) fn draw_macro_recording(
        &mut self,
        ui: &mut egui::Ui,
        q: &str,
        section_hit: bool,
        first: &mut bool,
    ) {
        let hit = section_hit || query_matches(q, SUBSECTION_RECORDING);
        let show_hide = setting_visible(q, hit, SETTING_HIDE_WHILE_RECORDING);
        let show_thresholds = setting_visible(q, hit, SETTING_RECORD_THRESHOLDS);
        if !(show_hide || show_thresholds) {
            return;
        }
        subsection_heading(ui, first, "Recording");
        if show_hide
            && dirty_checkbox(
                ui,
                &mut self.settings.ui.hide_app_during_recording,
                "Hide Sqyre while recording points and search areas",
                ht::SET_HIDE_ON_RECORD,
            )
        {
            self.mark_dirty();
        }

        if show_thresholds {
            advanced_header(
                ui,
                "settings_macros_recording_advanced",
                q,
                &[SETTING_RECORD_THRESHOLDS],
                |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Write a Wait after a pause of:");
                        let mut v = self.settings.runtime.record_wait_threshold_ms;
                        if ui
                            .add(
                                crate::widgets::num_entry(&mut v)
                                    .range(
                                        sqyre_persist::MIN_RECORD_WAIT_THRESHOLD_MS
                                            ..=sqyre_persist::MAX_RECORD_WAIT_THRESHOLD_MS,
                                    )
                                    .speed(5)
                                    .suffix(" ms"),
                            )
                            .on_hover_text(ht::SET_REC_WAIT)
                            .changed()
                        {
                            self.settings.runtime.record_wait_threshold_ms = v;
                            self.mark_dirty();
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Drop mouse moves shorter than:");
                        let mut v = self.settings.runtime.record_move_min_distance;
                        if ui
                            .add(
                                crate::widgets::num_entry(&mut v)
                                    .range(
                                        sqyre_persist::MIN_RECORD_MOVE_MIN_DISTANCE
                                            ..=sqyre_persist::MAX_RECORD_MOVE_MIN_DISTANCE,
                                    )
                                    .speed(1)
                                    .suffix(" px"),
                            )
                            .on_hover_text(ht::SET_REC_MOVE)
                            .changed()
                        {
                            self.settings.runtime.record_move_min_distance = v;
                            self.mark_dirty();
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Clicks held longer than this become down/up:");
                        let mut v = self.settings.runtime.record_tap_max_ms;
                        if ui
                            .add(
                                crate::widgets::num_entry(&mut v)
                                    .range(
                                        sqyre_persist::MIN_RECORD_TAP_MAX_MS
                                            ..=sqyre_persist::MAX_RECORD_TAP_MAX_MS,
                                    )
                                    .speed(10)
                                    .suffix(" ms"),
                            )
                            .on_hover_text(ht::SET_REC_TAP)
                            .changed()
                        {
                            self.settings.runtime.record_tap_max_ms = v;
                            self.mark_dirty();
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Snap recorded points to saved points within:");
                        let mut v = self.settings.runtime.record_snap_point_px;
                        if ui
                            .add(
                                crate::widgets::num_entry(&mut v)
                                    .range(
                                        sqyre_persist::MIN_RECORD_SNAP_POINT_PX
                                            ..=sqyre_persist::MAX_RECORD_SNAP_POINT_PX,
                                    )
                                    .speed(1)
                                    .suffix(" px"),
                            )
                            .on_hover_text(ht::SET_REC_SNAP)
                            .changed()
                        {
                            self.settings.runtime.record_snap_point_px = v;
                            self.mark_dirty();
                        }
                    });
                },
            );
        }
    }

    pub(super) fn draw_macro_hotkeys(
        &mut self,
        ui: &mut egui::Ui,
        q: &str,
        section_hit: bool,
        first: &mut bool,
        macro_hotkeys: &sqyre_hotkeys::MacroHotkeyBridge,
    ) {
        let hit = section_hit || query_matches(q, SUBSECTION_HOTKEYS);
        let show_tags = setting_visible(q, hit, SETTING_HOTKEY_TAGS_FOCUSED);

        let show_panic = setting_visible(q, hit, SETTING_PANIC_KEY);
        let show_retrigger = setting_visible(q, hit, SETTING_RETRIGGER);
        let show_trigger = setting_visible(q, hit, SETTING_DEFAULT_TRIGGER);
        let _ = macro_hotkeys;

        if !(show_tags || show_retrigger || show_trigger || show_panic) {
            return;
        }
        subsection_heading(ui, first, "Hotkeys");
        if show_tags
            && dirty_checkbox(
                ui,
                &mut self.settings.runtime.hotkey_tags_while_focused,
                "Select hotkey tags while a Program is focused",
                ht::SET_HOTKEY_TAGS,
            )
        {
            self.mark_dirty();
        }

        if show_panic {
            ui.horizontal(|ui| {
                ui.label("Panic/stop key:").on_hover_text(ht::SET_PANIC_KEY);
                let label = if self.settings.runtime.panic_key.trim().is_empty() {
                    "(disabled)".to_string()
                } else {
                    self.settings.runtime.panic_key.trim().to_string()
                };
                ui.label(egui::RichText::new(label).monospace());
                if crate::widgets::record_icon_button(ui, ht::SET_PANIC_KEY, true).clicked() {
                    self.panic_record.open(macro_hotkeys);
                }
                if crate::widgets::icon_button(
                    ui,
                    egui_phosphor::regular::ARROW_COUNTER_CLOCKWISE,
                    "Reset panic key to Esc",
                )
                .clicked()
                {
                    self.settings.runtime.panic_key = "esc".to_string();
                    self.mark_dirty();
                }
                if crate::widgets::clear_icon_button(ui, "Disable panic/stop key").clicked() {
                    self.settings.runtime.panic_key.clear();
                    self.mark_dirty();
                }
            });
        }

        if show_retrigger {
            ui.horizontal(|ui| {
                ui.label("Hotkey pressed while its macro is running:")
                    .on_hover_text(ht::SET_RETRIGGER);
                let mut policy = self.settings.runtime.hotkey_while_running;
                egui::ComboBox::from_id_salt("settings_hotkey_while_running")
                    .selected_text(policy.label())
                    .show_ui(ui, |ui| {
                        for p in HotkeyWhileRunning::ALL {
                            ui.selectable_value(&mut policy, p, p.label());
                        }
                    });
                if policy != self.settings.runtime.hotkey_while_running {
                    self.settings.runtime.hotkey_while_running = policy;
                    self.mark_dirty();
                }
            });
        }

        if show_trigger {
            ui.horizontal(|ui| {
                ui.label("New macro hotkeys fire on:")
                    .on_hover_text(ht::SET_DEFAULT_TRIGGER);
                let mut trigger = sqyre_hotkeys::HotkeyTrigger::parse(
                    &self.settings.runtime.default_hotkey_trigger,
                );
                egui::ComboBox::from_id_salt("settings_default_hotkey_trigger")
                    .selected_text(trigger.as_str())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut trigger,
                            sqyre_hotkeys::HotkeyTrigger::Press,
                            "press",
                        );
                        ui.selectable_value(
                            &mut trigger,
                            sqyre_hotkeys::HotkeyTrigger::Release,
                            "release",
                        );
                    });
                if trigger.as_str() != self.settings.runtime.default_hotkey_trigger {
                    self.settings.runtime.default_hotkey_trigger = trigger.as_str().to_string();
                    self.mark_dirty();
                }
            });
        }
    }

    /// Defaults stamped onto new macros; action defaults are edited per type
    /// in the Add Action picker.
    pub(super) fn draw_macro_action_defaults(
        &mut self,
        ui: &mut egui::Ui,
        q: &str,
        section_hit: bool,
        first: &mut bool,
    ) {
        let hit = section_hit || query_matches(q, SUBSECTION_ACTION_DEFAULTS);
        let show_delays = setting_visible(q, hit, SETTING_DEFAULT_DELAYS);
        let show_log_edge = setting_visible(q, hit, SETTING_LOG_IMAGE_EDGE);
        let show_saved = setting_visible(q, hit, SETTING_ACTION_DEFAULTS_RESET);
        if !(show_delays || show_log_edge || show_saved) {
            return;
        }
        subsection_heading(ui, first, "New macros & actions");

        if show_delays {
            ui.horizontal(|ui| {
                ui.label("New macro delays — key:")
                    .on_hover_text(ht::SET_DEFAULT_DELAYS);
                if dirty_i32(
                    ui,
                    &mut self.settings.runtime.default_keyboard_delay_ms,
                    ht::SET_DEFAULT_DELAYS,
                    |d| d.range(0..=60_000).speed(5).suffix(" ms"),
                ) {
                    self.mark_dirty();
                }
                ui.label("mouse:");
                if dirty_i32(
                    ui,
                    &mut self.settings.runtime.default_mouse_delay_ms,
                    ht::SET_DEFAULT_DELAYS,
                    |d| d.range(0..=60_000).speed(5).suffix(" ms"),
                ) {
                    self.mark_dirty();
                }
            });
        }

        if show_log_edge {
            ui.horizontal(|ui| {
                ui.label("Logged search images capped at:");
                let mut v = self.settings.runtime.log_image_max_edge;
                if ui
                    .add(
                        crate::widgets::num_entry(&mut v)
                            .range(
                                sqyre_persist::MIN_LOG_IMAGE_MAX_EDGE
                                    ..=sqyre_persist::MAX_LOG_IMAGE_MAX_EDGE,
                            )
                            .speed(32)
                            .suffix(" px"),
                    )
                    .on_hover_text(ht::SET_LOG_IMAGE_EDGE)
                    .changed()
                {
                    self.settings.runtime.log_image_max_edge = v;
                    self.mark_dirty();
                }
            });
        }

        if show_saved {
            let saved = self.settings.runtime.action_defaults.len();
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        saved > 0,
                        crate::widgets::IconButton::new(
                            egui_phosphor::regular::ARROW_COUNTER_CLOCKWISE,
                            ht::SET_RESET_ACTION_DEFAULTS,
                            None,
                        ),
                    )
                    .clicked()
                {
                    self.settings.runtime.action_defaults.clear();
                    self.mark_dirty();
                }
                ui.label(egui::RichText::new(format!("{saved} saved")).weak().small());
            });
        }
    }

    pub(super) fn draw_macro_image_search(
        &mut self,
        ui: &mut egui::Ui,
        q: &str,
        section_hit: bool,
        first: &mut bool,
    ) {
        let hit = section_hit || query_matches(q, SUBSECTION_IMAGE_SEARCH);
        let show_variant = setting_visible(q, hit, SETTING_VARIANT_EXIT_EARLY);
        let show_distance = setting_visible(q, hit, SETTING_IMAGE_SEARCH_DISTANCE);
        if !(show_variant || show_distance) {
            return;
        }
        subsection_heading(ui, first, "Image Search");

        if show_variant
            && dirty_checkbox(
                ui,
                &mut self.settings.runtime.image_search_variant_exit_early,
                "Stop at the first matching icon variant",
                ht::SET_VARIANT_STOP,
            )
        {
            self.mark_dirty();
        }

        if show_distance {
            advanced_header(
                ui,
                "settings_macros_image_search_advanced",
                q,
                &[SETTING_IMAGE_SEARCH_DISTANCE],
                |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Ignore nearby duplicate image matches:");
                        if dirty_i32(
                            ui,
                            &mut self.settings.runtime.image_search_close_matches_distance,
                            ht::SET_DEDUP_PX,
                            |d| d.range(0..=100).speed(1).suffix(" px"),
                        ) {
                            self.mark_dirty();
                        }
                    });
                },
            );
        }
    }
}
