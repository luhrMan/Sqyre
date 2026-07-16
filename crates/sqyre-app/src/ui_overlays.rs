//! Floating windows, overlays, and per-frame shell bookkeeping outside the main panels.

use crate::add_action::AddActionPicker;
use crate::catalog::apply_main_monitor_resolution;
use crate::data_editor::DataEditor;
use crate::icon_cache::IconCache;
use crate::pixel_color;
use crate::preview_tooltip::PreviewTooltipCache;
use crate::single_instance;
use crate::variables_panel;
use crate::SqyreApp;
use eframe::egui;
use sqyre_domain::collect_known_variable_names;
use sqyre_domain::ActionId;
use std::collections::HashSet;
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
/// While the Data Editor Overlay tab is editing a button, that button is previewed
/// live (even if overlays are globally disabled / focus-gated).
pub fn sync_macro_overlay(app: &mut SqyreApp, ctx: &egui::Context) {
    let enabled = app.settings_ui.settings().overlay_enabled;
    let buttons = app.settings_ui.settings().overlay_buttons.clone();
    let preview = app.data_editor.overlay_edit_preview();
    let hide = app.screen_click.is_armed();
    let running_macro = if app.run.running.load(Ordering::SeqCst) && !app.macros.is_empty() {
        let idx = app.selected_macro.min(app.macros.len() - 1);
        Some(app.macros[idx].name.as_str())
    } else {
        None
    };
    app.macro_overlay.sync(
        ctx,
        enabled,
        &buttons,
        preview.as_ref(),
        &app.catalog,
        &app.pending_hotkey_macros,
        hide,
        running_macro,
    );
}

fn action_display_name(app: &SqyreApp, action_id: ActionId) -> String {
    if app.macros.is_empty() {
        return action_id.as_str();
    }
    let idx = app.selected_macro.min(app.macros.len() - 1);
    let root = &app.macros[idx].root;
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
    let Some(action_id) = app.logs_window else {
        return;
    };
    let title = format!("Logs — {}", action_display_name(app, action_id));
    if crate::action_logs_ui::show_logs_window(
        ctx,
        action_id,
        &title,
        &app.action_log,
        &mut app.logs_image_cache,
    ) {
        app.logs_window = None;
    }
}

/// Data editor, settings, variables, add-action picker, logs.
pub fn show_floating_windows(app: &mut SqyreApp, ctx: &egui::Context) {
    show_logs_window(app, ctx);
    app.data_editor.show(
        ctx,
        &mut app.db,
        &mut app.macros,
        app.selected_macro,
        &mut app.catalog,
        &mut app.icon_cache,
        &mut app.preview_tooltips,
        &app.screen_click,
        app.settings_ui.settings_mut(),
    );
    app.settings_ui.show(
        ctx,
        &mut app.db,
        &mut app.macros,
        &mut app.catalog,
    );
    if !app.macros.is_empty() {
        let idx = app.selected_macro.min(app.macros.len() - 1);
        let running = app.run.running.load(Ordering::SeqCst);
        if app
            .variables_panel
            .show(
                ctx,
                &mut app.macros[idx],
                !running,
                &app.runtime_vars,
                running,
            )
        {
            app.persist_macro_at(idx);
        }
    }
    if let Some(action) = {
        let catalog = &app.catalog;
        let icons = &mut app.icon_cache;
        let previews = &mut app.preview_tooltips;
        let macros: Vec<(String, Vec<String>)> = app
            .macros
            .iter()
            .map(|m| (m.name.clone(), m.tags.clone()))
            .collect();
        let known_vars = if app.macros.is_empty() {
            HashSet::new()
        } else {
            let idx = app.selected_macro.min(app.macros.len() - 1);
            collect_known_variable_names(&app.macros[idx])
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
            &app.macro_hotkeys,
            &app.screen_click,
            |_| {
                defaults_to_persist = true;
            },
        );
        if defaults_to_persist {
            app.add_action_picker
                .store_into_settings(app.settings_ui.settings_mut());
            if let Err(e) = app.settings_ui.save_settings() {
                eprintln!("sqyre: save action defaults: {e}");
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
    // Keep highlighter enable flag in sync with the preference.
    let highlight_on = app.settings_ui.settings().highlight_active_action;
    if app.highlighter.is_enabled() != highlight_on {
        app.highlighter.set_enabled(highlight_on);
    }
    app.action_log
        .set_log_images(app.settings_ui.settings().save_meta_images);
    if app.settings_ui.reload_requested {
        app.settings_ui.reload_requested = false;
        apply_main_monitor_resolution(&mut app.catalog);
        match single_instance::reacquire(app.instance_lock.take()) {
            Ok(lock) => app.instance_lock = lock,
            Err(e) => eprintln!("sqyre: re-acquire instance lock: {e}"),
        }
        if app.instance_lock.is_none() {
            eprintln!(
                "sqyre: warning: could not lock {} (another instance may be using this data dir)",
                sqyre_persist::sqyre_dir().join("sqyre.lock").display()
            );
        }
        app.selected_macro = 0;
        app.selected_action = None;
        app.tree_histories.clear();
        app.tooltip.cancel();
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
    if let Some((x, y)) = app.screen_click.take_color_point() {
        match pixel_color::sample_pixel_hex(x, y) {
            Ok(hex) => {
                app.tooltip.apply_recorded_color(hex.clone());
                app.add_action_picker.apply_recorded_color(hex);
            }
            Err(e) => eprintln!("sqyre: sample pixel color: {e}"),
        }
    }
    app.update_recording_visibility(ctx);
    app.sync_recording_overlay(ctx);
    sync_macro_overlay(app, ctx);
    app.drain_pending_hotkey_macros(ctx);

    if let Some(chord) = app.hotkey_record.show(ctx, &app.macro_hotkeys) {
        if !app.tooltip.apply_recorded_chord(chord.clone())
            && !app.add_action_picker.apply_recorded_chord(chord.clone())
        {
            app.apply_hotkey_to_selected(chord, None);
        }
    }
    if let Some(key) = app.key_record.show(ctx, &app.macro_hotkeys) {
        app.tooltip.apply_recorded_key(key.clone());
        app.add_action_picker.apply_recorded_key(key);
    }

    let running = app.run.running.load(Ordering::SeqCst);
    if running
        || app.hotkey_record.is_open()
        || app.key_record.is_open()
        || app.screen_click.is_armed()
    {
        ctx.request_repaint();
    } else if app.settings_ui.settings().overlay_enabled {
        // Overlay focus-gating polls on its own schedule; avoid per-frame
        // transparent window clears (flicker) while still draining click queue promptly.
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
}

/// Ctrl+C / Ctrl+X / Ctrl+V / Ctrl+Z / Ctrl+Y / Ctrl+A — skip while editing an action
/// or when a text field has keyboard focus (so Ctrl+A still selects-all in editors).
pub fn handle_shortcuts(app: &mut SqyreApp, ui: &mut egui::Ui) {
    let running = app.run.running.load(Ordering::SeqCst);
    if !app.tooltip.is_editing()
        && !app.hotkey_record.is_open()
        && !app.key_record.is_open()
        && !ui.ctx().egui_wants_keyboard_input()
    {
        let (copy, cut, paste, undo, redo, add_action) = ui.ctx().input(|i| {
            let mod_key = i.modifiers.command;
            let copy = mod_key && i.key_pressed(egui::Key::C);
            let cut = mod_key && i.key_pressed(egui::Key::X);
            let paste = mod_key && i.key_pressed(egui::Key::V);
            let undo = mod_key && !i.modifiers.shift && i.key_pressed(egui::Key::Z);
            let redo = mod_key
                && (i.key_pressed(egui::Key::Y)
                    || (i.modifiers.shift && i.key_pressed(egui::Key::Z)));
            let add_action = mod_key && i.key_pressed(egui::Key::A);
            (copy, cut, paste, undo, redo, add_action)
        });
        if cut {
            app.cut_selection();
        } else if copy {
            app.copy_selection();
        } else if paste {
            app.paste_clipboard();
        } else if undo {
            app.undo_tree();
        } else if redo {
            app.redo_tree();
        } else if add_action && !running && !app.macros.is_empty() {
            app.add_action_picker.open();
        }
    }
}
