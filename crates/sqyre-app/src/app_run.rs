//! Macro run/stop and recording visibility for SqyreApp.

use crate::SqyreApp;
use eframe::egui;

impl SqyreApp {
    pub(crate) fn start_macro(&mut self, ctx: &egui::Context) {
        if self.macros.is_empty() {
            return;
        }
        let idx = self.selected_macro.min(self.macros.len() - 1);
        let name = self.macros[idx].name.clone();
        self.start_macro_by_name(&name, ctx);
    }

    pub(crate) fn drain_pending_hotkey_macros(&mut self, ctx: &egui::Context) {
        let pending: Vec<String> = std::mem::take(&mut *self.pending_hotkey_macros.lock());
        for name in pending {
            self.start_macro_by_name(&name, ctx);
        }
    }

    pub(crate) fn request_stop(&mut self) {
        self.run.stop.request_stop();
        *self.run.status.lock() = "Stop requested…".into();
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
                && self.screen_click.is_armed();
            if should_hide && !self.hidden_for_recording {
                self.hidden_for_recording = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            } else if !should_hide && self.hidden_for_recording {
                self.hidden_for_recording = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }
    }

    /// Live selection outline + coords HUD while screen-click recording is armed.
    pub(crate) fn sync_recording_overlay(&mut self, ctx: &egui::Context) {
        #[cfg(not(target_arch = "wasm32"))]
        self.recording_overlay.sync(ctx, &self.screen_click);
        #[cfg(target_arch = "wasm32")]
        let _ = ctx;
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn start_macro_by_name(&mut self, _name: &str, _ctx: &egui::Context) {
        *self.run.status.lock() = "Run is not available in the browser editor.".into();
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native_run {
    use super::*;
    use crate::app_backends::{trim_process_heap, AppOcr, BridgeContinueWait, StopWatchAutomation};
    use crate::catalog::{CatalogIcons, CatalogResolver, SnapshotMacros};
    use sqyre_capture::{shared_capturer, OsWindowFocuser, SharedRunCapturer};
    use sqyre_domain::Macro;
    use sqyre_executor::{execute_macro_with, ExecDeps, OcrEngine};
    use sqyre_input::OsAutomation;
    use sqyre_persist::variables_path;
    use sqyre_vision::LeptessOcr;
    use std::collections::BTreeMap;
    use std::sync::atomic::Ordering;
    use std::sync::Arc;
    use std::thread;

    impl SqyreApp {
        pub(crate) fn start_macro_by_name(&mut self, name: &str, ctx: &egui::Context) {
            if self.run.running.load(Ordering::SeqCst) {
                return;
            }
            let Some(idx) = self.macros.iter().position(|m| m.name == name) else {
                return;
            };
            // Show the running macro's tree so highlight overlays have matching rows.
            self.selected_macro = idx;
            let mut macro_ = self.macros[idx].clone();
            let catalog = self.catalog.clone();
            let stop_flag = self.run.stop.clone();
            stop_flag.clear();
            let running = Arc::clone(&self.run.running);
            let status = Arc::clone(&self.run.status);
            self.action_log.clear();
            self.runtime_vars.clear();
            self.logs_image_cache.clear();
            self.highlighter.clear_all();
            self.last_exec_follow = None;
            let action_log = self.action_log.clone();
            let runtime_vars = self.runtime_vars.clone();
            let highlighter = self.highlighter.clone();
            let continue_wait = BridgeContinueWait {
                continue_wait: self.continue_wait.clone(),
                macro_hotkeys: self.macro_hotkeys.clone(),
            };
            let close_matches = self
                .settings_ui
                .settings()
                .image_search_close_matches_distance;
            let play_finish_sound = self.settings_ui.settings().play_finish_sound;
            let macro_lookup = {
                let map: BTreeMap<String, Arc<Macro>> = self
                    .macros
                    .iter()
                    .map(|m| (m.name.clone(), Arc::new(m.clone())))
                    .collect();
                SnapshotMacros(Arc::new(map))
            };
            let ctx = ctx.clone();
            running.store(true, Ordering::SeqCst);
            *status.lock() = format!("Running {}…", macro_.name);

            thread::spawn(move || {
                let result = (|| -> Result<(), String> {
                    let mut automation =
                        OsAutomation::new().map_err(|e| format!("automation: {e}"))?;
                    let capturer_arc = shared_capturer().map_err(|e| format!("capture: {e}"))?;
                    let mut capturer = SharedRunCapturer(capturer_arc);
                    let resolver = CatalogResolver(&catalog);
                    let icons = CatalogIcons(&catalog);
                    let focuser = OsWindowFocuser;
                    let ocr_engine = LeptessOcr::from_env_or_system()
                        .map_err(|e| {
                            eprintln!("sqyre: {e}");
                            e
                        })
                        .ok()
                        .map(AppOcr);
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
                            resolver: Some(&resolver),
                            icons: Some(&icons),
                            macros: Some(&macro_lookup),
                            continue_waiter: Some(&continue_wait),
                            window_focuser: Some(&focuser),
                            ocr: ocr_engine.as_ref().map(|e| e as &dyn OcrEngine),
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
                            crate::sound::play_finish_sound();
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
