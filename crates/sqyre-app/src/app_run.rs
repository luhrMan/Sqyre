//! Macro run/stop and recording visibility for SqyreApp.

use crate::SqyreApp;
use eframe::egui;

impl SqyreApp {
    pub(crate) fn start_macro(&mut self, ctx: &egui::Context) {
        if self.workspace.macros.is_empty() {
            return;
        }
        let idx = self
            .workspace
            .selected_macro
            .min(self.workspace.macros.len() - 1);
        let name = self.workspace.macros[idx].name.clone();
        self.start_macro_by_name(&name, ctx);
    }

    pub(crate) fn drain_pending_hotkey_macros(&mut self, ctx: &egui::Context) {
        let pending: Vec<String> = std::mem::take(&mut *self.pending_hotkey_macros.lock());
        for name in pending {
            self.start_macro_by_name(&name, ctx);
        }
    }

    pub(crate) fn request_stop(&mut self) {
        self.run_session.state.stop.request_stop();
        *self.run_session.state.status.lock() = "Stop requested…".into();
    }

    /// Hide the main window while a screen-click recording is armed.
    pub(crate) fn update_recording_visibility(&mut self, ctx: &egui::Context) {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = ctx;
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let should_hide = self.settings_ui.settings().hide_app_during_recording
                && (self.screen_click.is_armed() || self.macro_record_bridge.is_armed());
            if should_hide && !self.hidden_for_recording {
                self.hidden_for_recording = true;
                #[cfg(feature = "native-runtime")]
                sqyre_capture::mark_site("recording:hide_main");
                // GNOME Wayland often ignores Visible(false); the wgpu surface then
                // clears transparent → opaque black. Minimize instead.
                #[cfg(all(feature = "native-runtime", target_os = "linux"))]
                if sqyre_capture::LinuxSessionInfo::detect().has_wayland {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                } else {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                }
                #[cfg(not(all(feature = "native-runtime", target_os = "linux")))]
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            } else if !should_hide && self.hidden_for_recording {
                self.hidden_for_recording = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }
    }

    /// Live selection outline + coords HUD while recording, or while a tooltip
    /// preview requests the desktop outline for a point / search area.
    pub(crate) fn sync_recording_overlay(&mut self, ctx: &egui::Context) {
        #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
        {
            let preview_outline = self.preview_tooltips.take_desktop_outline();
            self.recording_overlay.sync_with_macro_record(
                ctx,
                &self.screen_click,
                Some(&self.macro_record_bridge),
                preview_outline,
            );
        }
        #[cfg(any(target_arch = "wasm32", not(feature = "native-runtime")))]
        let _ = ctx;
    }

    #[cfg(not(feature = "native-runtime"))]
    pub(crate) fn start_macro_by_name(&mut self, _name: &str, _ctx: &egui::Context) {
        *self.run_session.state.status.lock() =
            "Run is not available in the browser editor.".into();
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
mod native_run {
    use super::*;
    use crate::app_backends::{trim_process_heap, BridgeContinueWait, StopWatchAutomation};
    use crate::catalog::{CatalogIcons, CatalogResolver, SnapshotMacros};
    use sqyre_capture::{shared_capturer, OsWindowFocuser, SharedRunCapturer};
    use sqyre_domain::Macro;
    use sqyre_executor::{execute_macro_with, ExecDeps, OcrEngine};
    use sqyre_input::OsAutomation;
    use sqyre_persist::variables_path;
    use sqyre_vision::shared_leptess;
    use std::collections::BTreeMap;
    use std::sync::atomic::Ordering;
    use std::sync::Arc;
    use std::thread;

    impl SqyreApp {
        pub(crate) fn start_macro_by_name(&mut self, name: &str, ctx: &egui::Context) {
            if self.run_session.state.running.load(Ordering::SeqCst) {
                return;
            }
            let Some(idx) = self.workspace.macros.iter().position(|m| m.name == name) else {
                return;
            };
            if let Err(e) = sqyre_validate::validate_macro(&self.workspace.macros[idx]) {
                *self.run_session.state.status.lock() = format!("Cannot run {name}: {e}");
                return;
            }
            // Show the running macro's tree so highlight overlays have matching rows.
            self.workspace.selected_macro = idx;
            let mut macro_ = self.workspace.macros[idx].clone();
            let catalog = self.workspace.catalog.clone();
            let stop_flag = self.run_session.state.stop.clone();
            stop_flag.clear();
            let running = Arc::clone(&self.run_session.state.running);
            let status = Arc::clone(&self.run_session.state.status);
            self.run_session.action_log.clear();
            self.run_session.runtime_vars.clear();
            self.run_session.logs_image_cache.clear();
            self.run_session.highlighter.clear_all();
            self.tree.last_exec_follow = None;
            let action_log = self.run_session.action_log.clone();
            let runtime_vars = self.run_session.runtime_vars.clone();
            let highlighter = self.run_session.highlighter.clone();
            let continue_wait = BridgeContinueWait {
                continue_wait: self.run_session.continue_wait.clone(),
                macro_hotkeys: self.run_session.macro_hotkeys.clone(),
            };
            let close_matches = self
                .settings_ui
                .settings()
                .image_search_close_matches_distance;
            let release_held_inputs = self.settings_ui.settings().release_held_inputs_on_end;
            let while_max_iterations = self.settings_ui.settings().while_max_iterations;
            let run_macro_max_depth =
                self.settings_ui.settings().run_macro_max_depth.max(1) as usize;
            let play_finish_sound = self.settings_ui.settings().play_finish_sound;
            let sound_volume = self.settings_ui.settings().sound_volume;
            let macro_lookup = {
                let map: BTreeMap<String, Arc<Macro>> = self
                    .workspace
                    .macros
                    .iter()
                    .map(|m| (m.name.clone(), Arc::new(m.clone())))
                    .collect();
                SnapshotMacros(Arc::new(map))
            };
            let ctx = ctx.clone();
            running.store(true, Ordering::SeqCst);
            *status.lock() = format!("Running {}…", macro_.name);

            // Must run on the UI thread: winit's SetCapture/ReleaseCapture are
            // thread-affine. Doing this only on the worker never clears Start-click capture.
            sqyre_input::prepare_for_automation();

            thread::spawn(move || {
                let result = (|| -> Result<(), String> {
                    let mut automation =
                        OsAutomation::new().map_err(|e| format!("automation: {e}"))?;
                    let capturer_arc = shared_capturer().map_err(|e| format!("capture: {e}"))?;
                    let mut capturer = SharedRunCapturer(capturer_arc);
                    let resolver = CatalogResolver(&catalog);
                    let icons = CatalogIcons(&catalog);
                    let focuser = OsWindowFocuser;
                    let ocr_engine = shared_leptess()
                        .map_err(|e| {
                            crate::log::warn(format_args!("{e}"));
                            e
                        })
                        .ok();
                    let stop_raw = stop_flag.raw();
                    let mut watched = StopWatchAutomation {
                        inner: &mut automation,
                        stop: &stop_flag,
                    };
                    let vars_dir = variables_path();
                    execute_macro_with(
                        &mut macro_,
                        ExecDeps {
                            automation: &mut watched,
                            capturer: Some(&mut capturer),
                            close_matches_distance: close_matches,
                            release_held_inputs,
                            while_max_iterations,
                            run_macro_max_depth,
                            resolver: Some(&resolver),
                            icons: Some(&icons),
                            macros: Some(&macro_lookup),
                            continue_waiter: Some(&continue_wait),
                            window_focuser: Some(&focuser),
                            ocr: ocr_engine.as_ref().map(|e| e.as_ref() as &dyn OcrEngine),
                            stop_flag: Some(stop_raw.as_ref()),
                            logger: Some(&action_log),
                            highlighter: Some(&highlighter),
                            runtime_vars: Some(&runtime_vars),
                            variables_dir: Some(vars_dir.as_path()),
                        },
                    )
                    .map_err(|e| e.to_string())
                })();

                sqyre_vision::clear_search_cache();
                trim_process_heap();

                let msg = match result {
                    Ok(()) if stop_flag.is_stopped() => "Stopped.".into(),
                    Ok(()) => {
                        if play_finish_sound {
                            crate::sound::play_finish_sound(sound_volume);
                        }
                        "Finished.".into()
                    }
                    Err(e) => format!("Error: {e}"),
                };
                *status.lock() = msg;
                running.store(false, Ordering::SeqCst);
                ctx.request_repaint();
            });
        }
    }
}
