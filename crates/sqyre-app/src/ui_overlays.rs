//! Floating windows, overlays, and per-frame shell bookkeeping outside the main panels.

use crate::add_action::AddActionPicker;
use crate::catalog::apply_main_monitor_resolution;
use crate::data_editor::DataEditor;
use crate::icon_cache::IconCache;
#[cfg(feature = "native-runtime")]
use crate::pixel_color;
use crate::preview_tooltip::PreviewTooltipCache;
use crate::variables_panel;
use crate::SqyreApp;
use eframe::egui;
use sqyre_domain::ActionId;
use std::sync::atomic::Ordering;

/// Close → hide to tray when available; Quit from tray allows real exit.
pub fn handle_close_to_tray(app: &mut SqyreApp, ctx: &egui::Context) {
    if app.tray.is_active()
        && !app.tray.quit_requested()
        && ctx.input(|i| i.viewport().close_requested())
    {
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }
}

/// Always-on-top macro buttons (settings-backed); hidden while recording is armed.
#[cfg(feature = "native-runtime")]
pub fn sync_macro_overlay(app: &mut SqyreApp, ctx: &egui::Context) {
    if app.screen_click.is_armed() || app.macro_record_bridge.is_armed() {
        return;
    }
    let buttons = app.settings_ui.settings().overlay_buttons.clone();
    let preview = app.data_editor.overlay_edit_preview();
    let running_macro = if app.run_session.state.running.load(Ordering::SeqCst)
        && !app.workspace.macros.is_empty()
    {
        let idx = app
            .workspace
            .selected_macro
            .min(app.workspace.macros.len() - 1);
        Some(app.workspace.macros[idx].name.as_str())
    } else {
        None
    };
    app.macro_overlay.sync(
        ctx,
        &buttons,
        preview.as_ref(),
        &app.workspace.catalog,
        &app.pending_hotkey_macros,
        running_macro,
    );
}

#[cfg(all(target_os = "linux", feature = "native-runtime"))]
pub fn show_wayland_permissions(app: &mut SqyreApp, ctx: &egui::Context) {
    let mut settings = app.settings_ui.settings().clone();
    let mut changed: Option<sqyre_persist::UserSettings> = None;
    app.wayland_permissions_ui
        .show(ctx, &mut settings, &mut |s| {
            changed = Some(s.clone());
        });
    if let Some(s) = changed {
        *app.settings_ui.settings_mut() = s.clone();
        sqyre_capture::apply_wayland_permission_settings(
            s.wayland_screen_capture,
            s.wayland_input_control,
            s.wayland_global_shortcuts,
            s.wayland_window_management,
        );
        let _ = app.settings_ui.save_settings();
    }
}

fn action_display_name(app: &SqyreApp, action_id: ActionId) -> String {
    if app.workspace.macros.is_empty() {
        return action_id.as_str();
    }
    let idx = app
        .workspace
        .selected_macro
        .min(app.workspace.macros.len() - 1);
    let root = &app.workspace.macros[idx].root;
    let action = if action_id.is_root() {
        Some(root)
    } else {
        root.find_by_id(action_id)
    };
    action
        .map(|a| a.display_name())
        .unwrap_or_else(|| action_id.as_str())
}

pub fn show_logs_window(app: &mut SqyreApp, ctx: &egui::Context) {
    let Some(action_id) = app.run_session.logs_window else {
        return;
    };
    let title = format!("Logs — {}", action_display_name(app, action_id));
    if crate::action_logs_ui::show_logs_window(
        ctx,
        action_id,
        &title,
        &app.run_session.action_log,
        &mut app.run_session.logs_image_cache,
    ) {
        app.run_session.logs_window = None;
    }
}

/// Data editor, settings, variables, add-action picker, logs.
pub fn show_floating_windows(app: &mut SqyreApp, ctx: &egui::Context) {
    show_logs_window(app, ctx);
    app.data_editor.show(
        ctx,
        &mut app.workspace.db,
        &mut app.workspace.macros,
        app.workspace.selected_macro,
        &mut app.workspace.catalog,
        &mut app.icon_cache,
        &mut app.preview_tooltips,
        &app.screen_click,
        app.settings_ui.settings_mut(),
    );
    #[cfg(not(target_arch = "wasm32"))]
    {
        app.settings_ui.show(
            ctx,
            &mut app.workspace.db,
            &mut app.workspace.macros,
            &mut app.workspace.catalog,
            &mut app.update,
        );
        if app.settings_ui.restart_requested {
            app.settings_ui.restart_requested = false;
            crate::update::restart_app(&mut app.instance_lock);
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        app.settings_ui.show(
            ctx,
            &mut app.workspace.db,
            &mut app.workspace.macros,
            &mut app.workspace.catalog,
        );
    }
    if !app.workspace.macros.is_empty() {
        let idx = app
            .workspace
            .selected_macro
            .min(app.workspace.macros.len() - 1);
        let running = app.run_session.state.running.load(Ordering::SeqCst);
        if app.variables_panel.show(
            ctx,
            &mut app.workspace.macros[idx],
            !running,
            &app.run_session.runtime_vars,
            running,
        ) {
            app.persist_macro_at(idx);
        }
    }
    if let Some(action) = {
        let catalog = &app.workspace.catalog;
        let icons = &mut app.icon_cache;
        let previews = &mut app.preview_tooltips;
        let macros: Vec<(String, Vec<String>)> = app
            .workspace
            .macros
            .iter()
            .map(|m| (m.name.clone(), m.tags.clone()))
            .collect();
        let known_vars = if app.workspace.macros.is_empty() {
            sqyre_domain::KnownVariableNames::default()
        } else {
            let idx = app
                .workspace
                .selected_macro
                .min(app.workspace.macros.len() - 1);
            app.tree
                .known_vars_cached(&app.workspace.macros[idx])
                .clone()
        };
        let mut defaults_to_persist = false;
        let picked = app.add_action_picker.show(
            ctx,
            catalog,
            icons,
            previews,
            &macros,
            &known_vars,
            &mut app.key_record,
            &mut app.hotkey_record,
            &app.run_session.macro_hotkeys,
            &app.screen_click,
            |_| {
                defaults_to_persist = true;
            },
        );
        if defaults_to_persist {
            app.add_action_picker
                .store_into_settings(app.settings_ui.settings_mut());
            if let Err(e) = app.settings_ui.save_settings() {
                crate::log::warn(format!("save action defaults: {e}"));
            }
        }
        picked
    } {
        let anchor = ctx
            .pointer_interact_pos()
            .unwrap_or_else(|| ctx.content_rect().center());
        app.insert_blank_action(action, anchor);
    }
}

/// Settings reload, highlighter / log prefs, color sample, recording + macro overlays,
/// hotkey/key record UI, and repaint pacing.
pub fn sync_frame_state(app: &mut SqyreApp, ctx: &egui::Context) {
    #[cfg(not(target_arch = "wasm32"))]
    poll_scheduled_backup(app, ctx);
    #[cfg(not(target_arch = "wasm32"))]
    poll_update(app, ctx);

    // Keep highlighter enable flag in sync with the preference.
    let highlight_on = app.settings_ui.settings().highlight_active_action;
    if app.run_session.highlighter.is_enabled() != highlight_on {
        app.run_session.highlighter.set_enabled(highlight_on);
    }
    app.run_session
        .action_log
        .set_log_images(app.settings_ui.settings().save_meta_images);
    if app.settings_ui.reload_requested {
        app.settings_ui.reload_requested = false;
        apply_main_monitor_resolution(&mut app.workspace.catalog);
        app.workspace.selected_macro = 0;
        app.clear_selected_actions();
        app.tree.histories.clear();
        app.tree.tooltip.cancel();
        app.add_action_picker = AddActionPicker::default();
        app.add_action_picker
            .load_from_settings(app.settings_ui.settings());
        let editor_open = app.data_editor.open;
        app.data_editor = DataEditor::default();
        app.data_editor.open = editor_open;
        let vars_open = app.variables_panel.open;
        app.variables_panel = variables_panel::VariablesPanelUi::default();
        app.variables_panel.open = vars_open;
        app.pending_delete_macro = None;
        app.icon_cache = IconCache::new();
        app.preview_tooltips = PreviewTooltipCache::new();
        app.refresh_macro_hotkey_bindings();
    }

    // Sample color before restoring visibility so the app isn't under the cursor.
    #[cfg(all(not(target_arch = "wasm32"), feature = "native-runtime"))]
    {
        use std::sync::mpsc::TryRecvError;

        if let Some(rx) = app.pixel_sample_pending.as_ref() {
            match rx.try_recv() {
                Ok(Ok(hex)) => {
                    app.pixel_sample_pending = None;
                    app.tree.tooltip.apply_recorded_color(hex.clone());
                    app.add_action_picker.apply_recorded_color(hex);
                }
                Ok(Err(e)) => {
                    app.pixel_sample_pending = None;
                    crate::log::warn(format!("sample pixel color: {e}"));
                }
                Err(TryRecvError::Empty) => ctx.request_repaint(),
                Err(TryRecvError::Disconnected) => {
                    app.pixel_sample_pending = None;
                    crate::log::warn("sample pixel color: capture failed");
                }
            }
        }
        if app.pixel_sample_pending.is_none() {
            if let Some((x, y)) = app.screen_click.take_color_point() {
                match pixel_color::spawn_sample_pixel_hex(x, y) {
                    Ok(rx) => {
                        app.pixel_sample_pending = Some(rx);
                        ctx.request_repaint();
                    }
                    Err(e) => crate::log::warn(format!("sample pixel color: {e}")),
                }
            }
        }
    }
    #[cfg(all(target_arch = "wasm32", feature = "native-runtime"))]
    if let Some((x, y)) = app.screen_click.take_color_point() {
        match pixel_color::sample_pixel_hex(x, y) {
            Ok(hex) => {
                app.tree.tooltip.apply_recorded_color(hex.clone());
                app.add_action_picker.apply_recorded_color(hex);
            }
            Err(e) => crate::log::warn(format!("sample pixel color: {e}")),
        }
    }
    app.update_recording_visibility(ctx);
    #[cfg(feature = "native-runtime")]
    sync_macro_overlay(app, ctx);
    #[cfg(all(target_os = "linux", feature = "native-runtime"))]
    show_wayland_permissions(app, ctx);
    // Windows Raw Input suppresses WH_KEYBOARD_LL while we are focused; mirror
    // egui keys into the hotkey bridges so Record / Esc / chords still work.
    #[cfg(target_os = "windows")]
    crate::win_focused_keys::feed_focused_keyboard(app, ctx);
    app.drain_pending_hotkey_macros(ctx);

    if let Some(chord) = app.hotkey_record.show(ctx, &app.run_session.macro_hotkeys) {
        if !app.tree.tooltip.apply_recorded_chord(chord.clone())
            && !app.add_action_picker.apply_recorded_chord(chord.clone())
        {
            app.apply_hotkey_to_selected(chord, None);
        }
    }
    if let Some(key) = app.key_record.show(ctx, &app.run_session.macro_hotkeys) {
        app.tree.tooltip.apply_recorded_key(key.clone());
        app.add_action_picker.apply_recorded_key(key);
    }
    if let Some(copied) = {
        let macros: Vec<(String, Vec<String>)> = app
            .workspace
            .macros
            .iter()
            .map(|m| (m.name.clone(), m.tags.clone()))
            .collect();
        let result = app.macro_record.show(crate::macro_record::MacroRecordShow {
            ctx,
            macro_hotkeys: &app.run_session.macro_hotkeys,
            bridge: &app.macro_record_bridge,
            catalog: &mut app.workspace.catalog,
            icons: &mut app.icon_cache,
            previews: &mut app.preview_tooltips,
            key_record: &mut app.key_record,
            hotkey_record: &mut app.hotkey_record,
            screen_click: &app.screen_click,
            macros: &macros,
        });
        if result.catalog_changed {
            if let Err(e) = app.persist_database() {
                crate::log::warn(format!("persist after macro-record points: {e}"));
            }
        }
        result.copy
    } {
        app.set_action_clipboard(ctx, copied.maps, &copied.yaml);
    }

    let running = app.run_session.state.running.load(Ordering::SeqCst);
    if running
        || app.hotkey_record.is_open()
        || app.key_record.is_open()
        || app.macro_record.is_open()
        || app.screen_click.is_armed()
        || app.macro_record_bridge.is_armed()
    {
        ctx.request_repaint();
    } else if app
        .settings_ui
        .settings()
        .overlay_buttons
        .iter()
        .any(|b| b.enabled)
    {
        // Overlay focus-gating polls on its own schedule; avoid per-frame
        // transparent window clears (flicker) while still draining click queue promptly.
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    } else {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // While an update worker is running, poll_update already scheduled a short wake.
            if !app.update.is_busy()
                && (app.settings_ui.settings().backup_enabled
                    || app.settings_ui.settings().auto_update_check)
            {
                // Coarse wake so automatic backups / update polls can fire while idle.
                ctx.request_repaint_after(std::time::Duration::from_secs(60));
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn poll_scheduled_backup(app: &mut SqyreApp, ctx: &egui::Context) {
    use std::sync::mpsc;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Poll in-flight task first.
    if let Some(rx) = app.backup_task.take() {
        match rx.try_recv() {
            Ok(Ok(path)) => {
                app.settings_ui.note_backup_success(&path);
            }
            Ok(Err(e)) => {
                crate::log::warn(format!("automatic backup failed: {e}"));
            }
            Err(mpsc::TryRecvError::Empty) => {
                app.backup_task = Some(rx);
                ctx.request_repaint_after(std::time::Duration::from_millis(250));
                return;
            }
            Err(mpsc::TryRecvError::Disconnected) => {}
        }
    }

    let settings = app.settings_ui.settings();
    if !settings.backup_enabled || app.backup_task.is_some() {
        return;
    }
    let interval_secs = (settings.backup_interval_hours.max(1) as u64).saturating_mul(3600);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let due = settings.last_backup_unix <= 0
        || now.saturating_sub(settings.last_backup_unix) as u64 >= interval_secs;
    if !due {
        return;
    }

    let keep = settings.backup_max_keep.max(1) as usize;
    let (tx, rx) = mpsc::channel();
    app.backup_task = Some(rx);
    thread::spawn(move || {
        let result = (|| {
            let path = sqyre_persist::create_backup().map_err(|e| e.to_string())?;
            sqyre_persist::prune_backups(keep).map_err(|e| e.to_string())?;
            Ok(path)
        })();
        let _ = tx.send(result);
    });
    ctx.request_repaint_after(std::time::Duration::from_millis(250));
}

#[cfg(not(target_arch = "wasm32"))]
fn poll_update(app: &mut SqyreApp, ctx: &egui::Context) {
    if app.update.poll() {
        match &app.update.state {
            crate::update::UpdateState::UpToDate
            | crate::update::UpdateState::Available { .. }
            | crate::update::UpdateState::Failed { .. } => {
                crate::update::note_check_time(app.settings_ui.settings_mut());
                app.settings_ui.persist();
            }
            crate::update::UpdateState::Ready { version } => {
                app.settings_ui.set_update_status_ok(format!(
                    "Update {version} installed. Restart to finish."
                ));
            }
            _ => {}
        }
        ctx.request_repaint();
    } else if app.update.is_busy() {
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
}

/// Ctrl+C / Ctrl+X / Ctrl+V / Ctrl+Z / Ctrl+Y / Ctrl+A — skip while editing an action
/// or when a text field has keyboard focus (so Ctrl+A still selects-all in editors).
///
/// Uses [`egui::Context::text_edit_focused`] rather than `egui_wants_keyboard_input`:
/// the latter is true whenever *any* widget (including the TreeView) has focus, which
/// would block tree clipboard shortcuts while selection is yellow.
///
/// Copy/cut/paste must listen for [`egui::Event::{Copy,Cut,Paste}`]: egui-winit
/// converts those chords into clipboard events and never emits `Key` presses for
/// C/X/V (unlike Ctrl+A / Ctrl+Z / Ctrl+Y).
///
/// Mutating shortcuts match the action toolbar: disabled while a macro is running.
pub fn handle_shortcuts(app: &mut SqyreApp, ui: &mut egui::Ui) {
    let running = app.run_session.state.running.load(Ordering::SeqCst);
    if !app.tree.tooltip.is_editing()
        && !app.hotkey_record.is_open()
        && !app.key_record.is_open()
        && !app.macro_record.is_open()
        && !ui.ctx().text_edit_focused()
    {
        let (copy, cut, paste, undo, redo, add_action) = ui.ctx().input(|i| {
            let mut copy = false;
            let mut cut = false;
            let mut paste = false;
            for ev in &i.events {
                match ev {
                    egui::Event::Copy => copy = true,
                    egui::Event::Cut => cut = true,
                    egui::Event::Paste(_) => paste = true,
                    _ => {}
                }
            }
            let mod_key = i.modifiers.command;
            let undo = mod_key && !i.modifiers.shift && i.key_pressed(egui::Key::Z);
            let redo = mod_key
                && (i.key_pressed(egui::Key::Y)
                    || (i.modifiers.shift && i.key_pressed(egui::Key::Z)));
            let add_action = mod_key && i.key_pressed(egui::Key::A);
            (copy, cut, paste, undo, redo, add_action)
        });
        if running {
            return;
        }
        if cut {
            app.cut_selection(ui.ctx());
        } else if copy {
            app.copy_selection(ui.ctx());
        } else if paste {
            app.paste_clipboard();
        } else if undo {
            app.undo_tree();
        } else if redo {
            app.redo_tree();
        } else if add_action && !app.workspace.macros.is_empty() {
            app.add_action_picker.open();
        }
    }
}
