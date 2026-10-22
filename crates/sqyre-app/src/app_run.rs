//! Macro run/stop and recording visibility for SqyreApp.

use crate::SqyreApp;
use eframe::egui;

/// Android consent-gate state for [`PendingProjectionRun`].
///
/// MediaProjection consent cannot be persisted or reused (single-use tokens since
/// Android 14's `MEDIA_PROJECTION_PREVENTS_REUSING_CONSENT`), so a run that reads
/// the screen parks here while a modal explains and fires the system dialog.
#[cfg(all(target_os = "android", feature = "native-runtime"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectionGate {
    /// The modal is explaining; the system dialog has not been fired yet.
    Ask,
    /// The consent dialog was fired — waiting for Running (grant) or Stopped.
    Waiting,
    /// The dialog was dismissed without consent; offer a retry.
    Denied,
}

/// A macro that reads the screen, queued while MediaProjection consent is
/// requested. `ui_overlays::show_projection_run_gate` paints the modal and calls
/// `start_macro_by_name` once the projection reports Running.
#[cfg(all(target_os = "android", feature = "native-runtime"))]
#[derive(Debug)]
pub(crate) struct PendingProjectionRun {
    pub macro_name: String,
    pub state: ProjectionGate,
}

impl SqyreApp {
    #[cfg(not(target_arch = "wasm32"))]
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
        // Don't take while a chooser is open — overlay clicks share this queue
        // and used to be discarded here. Names from the press that opened it can
        // still be arriving; drop those so the menu does not reopen after a pick.
        if let Some(open) = &self.hotkey_chooser {
            let macros = &self.workspace.macros;
            self.pending_hotkey_macros.lock().retain(|name| {
                crate::hotkey_chooser::chord_of(name, macros).as_ref() != Some(&open.chord)
            });
            return;
        }
        let pending: Vec<String> = self
            .run_session
            .macro_hotkeys
            .between_fires(|| std::mem::take(&mut *self.pending_hotkey_macros.lock()));
        if pending.is_empty() {
            return;
        }

        let (groups, direct) =
            crate::hotkey_chooser::partition_pending(&pending, &self.workspace.macros);
        for name in direct {
            #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
            sqyre_capture::event_log(
                "SQYRE_OVERLAY",
                &[("fire", "direct"), ("name", name.as_str())],
            );
            self.start_macro_by_name(&name, ctx);
        }
        for (chord_key, trigger, names) in groups {
            let chord = (chord_key, trigger);
            if names.len() <= 1 {
                if let Some(name) = names.into_iter().next() {
                    #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
                    sqyre_capture::event_log(
                        "SQYRE_HOTKEY",
                        &[("fire", "start"), ("name", name.as_str())],
                    );
                    self.start_macro_by_name(&name, ctx);
                }
                continue;
            }
            #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
            sqyre_capture::event_log(
                "SQYRE_HOTKEY",
                &[
                    ("fire", "choose"),
                    ("chord", chord.0.as_str()),
                    ("count", &names.len().to_string()),
                ],
            );
            self.open_hotkey_chooser(names, chord, ctx);
            // One chooser at a time; remaining groups wait for a later fire.
            break;
        }
    }

    pub(crate) fn request_stop(&mut self) {
        #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
        sqyre_capture::mark_site("run:stop:requested");
        self.run_session.state.stop.request_stop();
        *self.run_session.state.status.lock() = "Stop requested…".into();
    }

    /// Hide the main window while a screen-click recording is armed.
    pub(crate) fn update_recording_visibility(&mut self, ctx: &egui::Context) {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = ctx;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Android has one window; hiding it leaves a black screen with nothing to pick on.
            let should_hide = !cfg!(target_os = "android")
                && self.settings_ui.settings().hide_app_during_recording
                && (self.screen_click.is_armed() || self.macro_record_bridge.is_armed());
            if should_hide && !self.hidden_for_recording {
                self.hidden_for_recording = true;
                #[cfg(feature = "native-runtime")]
                sqyre_capture::mark_site("recording:hide_main");
                // Same unmap path as tray Hide — ViewportCommand alone is not enough
                // to remap afterward on GNOME/XWayland (recording looks "frozen").
                self.tray.set_root_visible(ctx, false);
                #[cfg(all(feature = "native-runtime", target_os = "linux"))]
                if sqyre_capture::LinuxSessionInfo::detect().has_wayland {
                    sqyre_capture::event_log("SQYRE_HUD", &[("hide", "wayland-unmap")]);
                    sqyre_capture::nudge_portal_capture_after_ui_hide();
                }
            } else if !should_hide && self.hidden_for_recording {
                self.hidden_for_recording = false;
                #[cfg(feature = "native-runtime")]
                sqyre_capture::mark_site("recording:show_main");
                self.tray.set_root_visible(ctx, true);
                #[cfg(all(feature = "native-runtime", target_os = "linux"))]
                sqyre_capture::event_log("SQYRE_HUD", &[("show", "remap")]);
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
                self.hidden_for_recording,
            );
            #[cfg(target_os = "android")]
            self.screen_pick.sync(ctx, &self.screen_click);
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
    use crate::app_backends::{
        os_automation, trim_process_heap, BridgeContinueWait, StopWatchAutomation,
    };
    use crate::catalog::{CatalogIcons, CatalogResolver, SnapshotMacros};
    use sqyre_capture::{shared_capturer, OsWindowFocuser, SharedRunCapturer};
    use sqyre_domain::Macro;
    use sqyre_executor::{execute_macro_with, ExecDeps, OcrEngine};
    use sqyre_persist::variables_path;
    use sqyre_vision::shared_ocrs;
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread;

    /// Clears `running` and wakes the UI even if the worker panics mid-macro.
    /// Without this, overlay buttons stay busy and `start_macro_by_name` no-ops forever.
    struct RunningClearOnDrop {
        running: Arc<AtomicBool>,
        running_macro: Arc<parking_lot::Mutex<Option<String>>>,
        ctx: egui::Context,
    }

    impl Drop for RunningClearOnDrop {
        fn drop(&mut self) {
            self.running.store(false, Ordering::SeqCst);
            *self.running_macro.lock() = None;
            self.ctx.request_repaint();
        }
    }

    /// Mirror a run status line into the projection notification so it stays
    /// visible in the shade while Sqyre is backgrounded.
    #[cfg(target_os = "android")]
    fn mirror_run_status(msg: &str) {
        if let Err(e) = sqyre_android::bridge::set_run_status(msg) {
            crate::log::warn(format_args!("run status notification: {e}"));
        }
    }

    impl SqyreApp {
        pub(crate) fn start_macro_by_name(&mut self, name: &str, ctx: &egui::Context) {
            if self.run_session.state.running.load(Ordering::SeqCst) {
                sqyre_capture::mark_site("overlay:start:skip-running");
                sqyre_capture::note(&format!(
                    "overlay: start skipped already-running name={name}"
                ));
                sqyre_capture::event_log(
                    "SQYRE_OVERLAY",
                    &[
                        ("start", "skip"),
                        ("reason", "already-running"),
                        ("name", name),
                    ],
                );
                *self.run_session.state.status.lock() =
                    format!("Already running — stop first to run \"{name}\".");
                ctx.request_repaint();
                return;
            }
            let Some(idx) = self
                .workspace
                .macros
                .iter()
                .position(|m| m.name.eq_ignore_ascii_case(name))
            else {
                sqyre_capture::mark_site("overlay:start:skip-unknown");
                sqyre_capture::note(&format!("overlay: start skipped unknown-macro name={name}"));
                sqyre_capture::event_log(
                    "SQYRE_OVERLAY",
                    &[
                        ("start", "skip"),
                        ("reason", "unknown-macro"),
                        ("name", name),
                    ],
                );
                *self.run_session.state.status.lock() = format!("No macro named \"{name}\".");
                ctx.request_repaint();
                return;
            };
            if let Err(e) = sqyre_validate::validate_macro(&self.workspace.macros[idx]) {
                *self.run_session.state.status.lock() = format!("Cannot run {name}: {e}");
                sqyre_capture::note(&format!(
                    "overlay: start skipped invalid-macro name={name} err={e}"
                ));
                ctx.request_repaint();
                return;
            }
            // Screen-reading runs park behind the consent gate instead of firing
            // the system dialog mid-run once the first capture asks.
            #[cfg(target_os = "android")]
            if sqyre_android::frames().projection() != sqyre_android::Projection::Running
                && self.workspace.macros[idx].uses_screen_capture(&self.workspace.macros)
            {
                self.pending_projection_run = Some(PendingProjectionRun {
                    macro_name: self.workspace.macros[idx].name.clone(),
                    state: ProjectionGate::Ask,
                });
                sqyre_capture::event_log("SQYRE_OVERLAY", &[("start", "gate"), ("name", name)]);
                ctx.request_repaint();
                return;
            }
            // Show the running macro's tree so highlight overlays have matching rows.
            self.workspace.selected_macro = idx;
            self.run_session.logs_image_cache.clear();
            self.tree.last_exec_follow = None;
            self.launch_macro(ctx, idx);
        }

        fn launch_macro(&self, ctx: &egui::Context, idx: usize) {
            let settings = self.settings_ui.settings();
            MacroLaunch {
                macro_: self.workspace.macros[idx].clone(),
                macros: self.workspace.macros.clone(),
                catalog: self.workspace.catalog.clone(),
                ctx: ctx.clone(),
                running: Arc::clone(&self.run_session.state.running),
                status: Arc::clone(&self.run_session.state.status),
                stop: self.run_session.state.stop.clone(),
                action_log: self.run_session.action_log.clone(),
                runtime_vars: self.run_session.runtime_vars.clone(),
                highlighter: self.run_session.highlighter.clone(),
                continue_wait: self.run_session.continue_wait.clone(),
                macro_hotkeys: self.run_session.macro_hotkeys.clone(),
                running_macro: self.macro_overlay.running_macro_slot(),
                close_matches: settings.image_search_close_matches_distance,
                variant_exit_early: settings.image_search_variant_exit_early,
                release_held_inputs: settings.release_held_inputs_on_end,
                while_max_iterations: settings.while_max_iterations,
                run_macro_max_depth: settings.run_macro_max_depth.max(1) as usize,
                play_finish_sound: settings.play_finish_sound,
                sound_volume: settings.sound_volume,
            }
            .start();
        }
    }

    struct MacroLaunch {
        macro_: Macro,
        macros: Vec<Macro>,
        catalog: sqyre_persist::ProgramCatalog,
        ctx: egui::Context,
        running: Arc<AtomicBool>,
        status: Arc<parking_lot::Mutex<String>>,
        stop: sqyre_hotkeys::StopFlag,
        action_log: sqyre_ports::SharedActionLog,
        runtime_vars: sqyre_ports::SharedRuntimeVars,
        highlighter: sqyre_ports::SharedHighlighter,
        continue_wait: sqyre_hotkeys::ContinueWaitBridge,
        macro_hotkeys: sqyre_hotkeys::MacroHotkeyBridge,
        running_macro: Arc<parking_lot::Mutex<Option<String>>>,
        close_matches: i32,
        variant_exit_early: bool,
        release_held_inputs: bool,
        while_max_iterations: i32,
        run_macro_max_depth: usize,
        play_finish_sound: bool,
        sound_volume: f32,
    }

    impl MacroLaunch {
        fn start(self) {
            let Self {
                mut macro_,
                macros,
                catalog,
                ctx,
                running,
                status,
                stop,
                action_log,
                runtime_vars,
                highlighter,
                continue_wait,
                macro_hotkeys,
                running_macro,
                close_matches,
                variant_exit_early,
                release_held_inputs,
                while_max_iterations,
                run_macro_max_depth,
                play_finish_sound,
                sound_volume,
            } = self;
            stop.clear();
            action_log.clear();
            runtime_vars.clear();
            highlighter.clear_all();
            let macro_lookup = {
                let map: BTreeMap<String, Arc<Macro>> = macros
                    .iter()
                    .map(|m| (m.name.clone(), Arc::new(m.clone())))
                    .collect();
                SnapshotMacros(Arc::new(map))
            };
            running.store(true, Ordering::SeqCst);
            let running_msg = format!("Running {}…", macro_.name);
            *status.lock() = running_msg.clone();
            #[cfg(target_os = "android")]
            mirror_run_status(&running_msg);
            *running_macro.lock() = Some(macro_.name.clone());
            #[cfg(target_os = "android")]
            {
                *OVERLAY_STARTED.lock() = Some(macro_.name.clone());
            }
            let name = macro_.name.clone();
            sqyre_capture::mark_site(&format!("overlay:start:ok:{name}"));
            sqyre_capture::note(&format!("overlay: start ok name={name}"));
            sqyre_capture::event_log("SQYRE_OVERLAY", &[("start", "ok"), ("name", name.as_str())]);

            // Must run on the UI thread: winit's SetCapture/ReleaseCapture are
            // thread-affine. Doing this only on the worker never clears Start-click capture.
            sqyre_input::prepare_for_automation();
            // A screen recording the user stopped may be requested again by this run.
            #[cfg(target_os = "android")]
            sqyre_android::frames().rearm();

            let continue_wait = BridgeContinueWait {
                continue_wait,
                macro_hotkeys,
            };
            let stop_flag = stop;
            let running_macro_slot = running_macro;
            thread::spawn(move || {
                let _clear_running = RunningClearOnDrop {
                    running: Arc::clone(&running),
                    running_macro: running_macro_slot,
                    ctx: ctx.clone(),
                };
                sqyre_capture::mark_site("run:worker:begin");
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let mut automation = os_automation().map_err(|e| format!("automation: {e}"))?;
                    let capturer_arc = shared_capturer().map_err(|e| format!("capture: {e}"))?;
                    let mut capturer = SharedRunCapturer(capturer_arc);
                    let resolver = CatalogResolver(&catalog);
                    let icons = CatalogIcons(&catalog);
                    let focuser = OsWindowFocuser;
                    let ocr_engine = shared_ocrs()
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
                    sqyre_capture::mark_site("run:worker:exec");
                    execute_macro_with(
                        &mut macro_,
                        ExecDeps {
                            automation: &mut watched,
                            capturer: Some(&mut capturer),
                            close_matches_distance: close_matches,
                            variant_exit_early,
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
                }));
                sqyre_capture::mark_site("run:worker:exec-done");

                crate::mem_diag::sample("pre_clear");
                sqyre_vision::clear_search_cache();
                sqyre_capture::release_capture_frame_cache();
                trim_process_heap();
                crate::mem_diag::sample_with_action_log("post_run", &action_log);

                let msg = match result {
                    Ok(Ok(())) if stop_flag.is_stopped() => "Stopped.".into(),
                    Ok(Ok(())) => {
                        if play_finish_sound {
                            crate::sound::play_finish_sound(sound_volume);
                        }
                        "Finished.".into()
                    }
                    Ok(Err(e)) => {
                        format!("Error: {}", crate::status_banner::user_error_detail(&e))
                    }
                    Err(_) => {
                        sqyre_capture::note("overlay: run worker panicked; cleared running");
                        sqyre_capture::event_log(
                            "SQYRE_OVERLAY",
                            &[("run", "panic"), ("running", "cleared")],
                        );
                        "Crashed (see crash.log).".into()
                    }
                };
                *status.lock() = msg.clone();
                #[cfg(target_os = "android")]
                mirror_run_status(&msg);
                let done = status.lock().clone().replace(' ', "_");
                sqyre_capture::note(&format!("overlay: run done status={done}"));
            });
        }
    }

    /// Last macro an overlay button started. Applied to the tree selection on the next frame.
    #[cfg(target_os = "android")]
    static OVERLAY_STARTED: parking_lot::Mutex<Option<String>> = parking_lot::Mutex::new(None);

    /// Screen-reading overlay run waiting for the consent modal (the activity was in the background).
    #[cfg(target_os = "android")]
    static OVERLAY_PROJECTION: parking_lot::Mutex<Option<String>> = parking_lot::Mutex::new(None);

    #[cfg(target_os = "android")]
    struct OverlayRunKit {
        ctx: egui::Context,
        macros: Vec<Macro>,
        catalog: sqyre_persist::ProgramCatalog,
        running: Arc<AtomicBool>,
        status: Arc<parking_lot::Mutex<String>>,
        stop: sqyre_hotkeys::StopFlag,
        action_log: sqyre_ports::SharedActionLog,
        runtime_vars: sqyre_ports::SharedRuntimeVars,
        highlighter: sqyre_ports::SharedHighlighter,
        continue_wait: sqyre_hotkeys::ContinueWaitBridge,
        macro_hotkeys: sqyre_hotkeys::MacroHotkeyBridge,
        running_macro: Arc<parking_lot::Mutex<Option<String>>>,
        close_matches: i32,
        variant_exit_early: bool,
        release_held_inputs: bool,
        while_max_iterations: i32,
        run_macro_max_depth: usize,
        play_finish_sound: bool,
        sound_volume: f32,
    }

    #[cfg(target_os = "android")]
    static OVERLAY_KIT: parking_lot::Mutex<Option<Arc<OverlayRunKit>>> =
        parking_lot::Mutex::new(None);

    /// Remember the macros and settings an overlay tap can run after Sqyre leaves the screen.
    #[cfg(target_os = "android")]
    pub(crate) fn refresh_android_overlay_kit(app: &SqyreApp, ctx: &egui::Context) {
        let settings = app.settings_ui.settings();
        let kit = OverlayRunKit {
            ctx: ctx.clone(),
            macros: app.workspace.macros.clone(),
            catalog: app.workspace.catalog.clone(),
            running: Arc::clone(&app.run_session.state.running),
            status: Arc::clone(&app.run_session.state.status),
            stop: app.run_session.state.stop.clone(),
            action_log: app.run_session.action_log.clone(),
            runtime_vars: app.run_session.runtime_vars.clone(),
            highlighter: app.run_session.highlighter.clone(),
            continue_wait: app.run_session.continue_wait.clone(),
            macro_hotkeys: app.run_session.macro_hotkeys.clone(),
            running_macro: app.macro_overlay.running_macro_slot(),
            close_matches: settings.image_search_close_matches_distance,
            variant_exit_early: settings.image_search_variant_exit_early,
            release_held_inputs: settings.release_held_inputs_on_end,
            while_max_iterations: settings.while_max_iterations,
            run_macro_max_depth: settings.run_macro_max_depth.max(1) as usize,
            play_finish_sound: settings.play_finish_sound,
            sound_volume: settings.sound_volume,
        };
        *OVERLAY_KIT.lock() = Some(Arc::new(kit));
    }

    /// Install the tap handler once. The kit is filled by [`refresh_android_overlay_kit`].
    #[cfg(target_os = "android")]
    pub(crate) fn install_android_overlay_run() {
        sqyre_android::set_overlay_run_handler(Arc::new(|name| launch_overlay_macro(name)));
    }

    #[cfg(target_os = "android")]
    pub(crate) fn take_android_overlay_started() -> Option<String> {
        OVERLAY_STARTED.lock().take()
    }

    #[cfg(target_os = "android")]
    pub(crate) fn take_android_projection_request() -> Option<String> {
        OVERLAY_PROJECTION.lock().take()
    }

    #[cfg(target_os = "android")]
    fn launch_overlay_macro(name: &str) {
        let kit = OVERLAY_KIT.lock().clone();
        let Some(kit) = kit else {
            sqyre_capture::note("overlay: start skipped no-kit");
            return;
        };
        if kit.running.load(Ordering::SeqCst) {
            *kit.status.lock() = format!("Already running — stop first to run \"{name}\".");
            kit.ctx.request_repaint();
            return;
        }
        let Some(idx) = kit
            .macros
            .iter()
            .position(|m| m.name.eq_ignore_ascii_case(name))
        else {
            *kit.status.lock() = format!("No macro named \"{name}\".");
            kit.ctx.request_repaint();
            return;
        };
        if let Err(e) = sqyre_validate::validate_macro(&kit.macros[idx]) {
            *kit.status.lock() = format!("Cannot run {name}: {e}");
            kit.ctx.request_repaint();
            return;
        }
        if sqyre_android::frames().projection() != sqyre_android::Projection::Running
            && kit.macros[idx].uses_screen_capture(&kit.macros)
        {
            *OVERLAY_PROJECTION.lock() = Some(kit.macros[idx].name.clone());
            if !sqyre_android::frames().shell_visible() {
                if let Err(e) = sqyre_android::bridge::show_shell() {
                    sqyre_capture::note(&format!("overlay: show shell for projection gate: {e}"));
                }
            }
            kit.ctx.request_repaint();
            sqyre_capture::event_log("SQYRE_OVERLAY", &[("start", "gate"), ("name", name)]);
            return;
        }
        MacroLaunch {
            macro_: kit.macros[idx].clone(),
            macros: kit.macros.clone(),
            catalog: kit.catalog.clone(),
            ctx: kit.ctx.clone(),
            running: Arc::clone(&kit.running),
            status: Arc::clone(&kit.status),
            stop: kit.stop.clone(),
            action_log: kit.action_log.clone(),
            runtime_vars: kit.runtime_vars.clone(),
            highlighter: kit.highlighter.clone(),
            continue_wait: kit.continue_wait.clone(),
            macro_hotkeys: kit.macro_hotkeys.clone(),
            running_macro: Arc::clone(&kit.running_macro),
            close_matches: kit.close_matches,
            variant_exit_early: kit.variant_exit_early,
            release_held_inputs: kit.release_held_inputs,
            while_max_iterations: kit.while_max_iterations,
            run_macro_max_depth: kit.run_macro_max_depth,
            play_finish_sound: kit.play_finish_sound,
            sound_volume: kit.sound_volume,
        }
        .start();
    }
}

#[cfg(all(target_os = "android", feature = "native-runtime"))]
pub(crate) use native_run::{
    install_android_overlay_run, refresh_android_overlay_kit, take_android_overlay_started,
    take_android_projection_request,
};
