//! Window top bar (brand / Data Editor / Settings), footer (delay / hotkey / builders), and action chrome.

use crate::macro_meta::collect_all_macro_tags;
use crate::theme;
use crate::SqyreApp;
use eframe::egui::{self, Color32};
use sqyre_hotkeys::{format_hotkey, HotkeyTrigger};
use sqyre_ui_model::action_pastel_color;
use std::sync::atomic::Ordering;

/// Compact toolbar control: icon glyph + hover label.
pub fn toolbar_icon(ui: &mut egui::Ui, glyph: &str, tip: &str, enabled: bool) -> egui::Response {
    toolbar_icon_colored(ui, glyph, tip, enabled, None)
}

/// Compact toolbar control with an optional fixed glyph color.
fn toolbar_icon_colored(
    ui: &mut egui::Ui,
    glyph: &str,
    tip: &str,
    enabled: bool,
    color: Option<Color32>,
) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| {
        crate::widgets::icon_button_colored(ui, glyph, tip, color)
    })
    .inner
}

/// Full-width window header above the macro list and central panel:
/// Sqyre command palette and run status on the left; Data Editor / Settings / list toggle on the right.
/// Must be shown before the side panel so it spans the whole window width.
pub fn top_bar(app: &mut SqyreApp, ui: &mut egui::Ui) {
    egui::Panel::top("app_top_bar")
        .frame(
            egui::Frame::side_top_panel(ui.style())
                .inner_margin(egui::Margin::symmetric(8, theme::SPACE_4 as i8)),
        )
        .show(ui, |ui| brand_header(app, ui));
}

fn brand_header(app: &mut SqyreApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = theme::SPACE_4;

        // Brand / command palette (primary affordance), top-left of the window.
        let tex = app.icon_cache.sqyre_fallback(ui.ctx());
        let size = egui::vec2(28.0, 28.0);
        let image = egui::Image::new((tex.id(), size))
            .fit_to_exact_size(size)
            .maintain_aspect_ratio(true);
        let button = egui::Button::image_and_text(image, egui::RichText::new("Sqyre").heading())
            .frame_when_inactive(false);
        if ui
            .add(button)
            .on_hover_text("Command palette (Ctrl+K)")
            .clicked()
        {
            app.command_palette.open_palette();
        }

        #[cfg(not(target_arch = "wasm32"))]
        show_update_banner(app, ui);

        // Right-to-left: first added is rightmost.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (list_glyph, list_tip) = if app.macro_list_open {
                ("▷", "Hide macro list")
            } else {
                ("☰", "Show macro list")
            };
            if toolbar_icon(ui, list_glyph, list_tip, true).clicked() {
                app.macro_list_open = !app.macro_list_open;
            }
            crate::widgets::section_separator(ui);
            let permissions_missing = app.settings_ui.permissions_missing();
            let settings_tip = if permissions_missing {
                "Settings: a permission is missing"
            } else {
                "Settings"
            };
            let settings_btn = toolbar_icon(ui, "⚙", settings_tip, true);
            if permissions_missing {
                crate::widgets::warn_badge(ui, settings_btn.rect);
            }
            if settings_btn.clicked() {
                app.settings_ui.request_open(ui.ctx());
            }
            if toolbar_icon(ui, "📁", "Data Editor", true).clicked() {
                app.data_editor.request_open(ui.ctx());
            }
            #[cfg(target_arch = "wasm32")]
            {
                if toolbar_icon(ui, "⬆", "Export db.yaml", true).clicked() {
                    app.export_db_yaml();
                }
                if toolbar_icon(ui, "⬇", "Import db.yaml", true).clicked() {
                    app.request_db_import();
                }
            }

            // Leftover width between the brand and the right-side buttons.
            let status = app.run_session.state.status.lock().clone();
            if !status.is_empty() {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    // Hover shows the elided text on desktop; touch screens have no
                    // hover, so a tap opens the full message.
                    let resp = ui.add(
                        egui::Label::new(&status)
                            .truncate()
                            .selectable(false)
                            .sense(egui::Sense::click()),
                    );
                    let width = crate::widgets::dialog_constrain_rect(ui.ctx())
                        .width()
                        .min(420.0);
                    egui::Popup::from_toggle_button_response(&resp)
                        .width(width)
                        .show(|ui| {
                            ui.set_max_width(width);
                            ui.add(egui::Label::new(&status).wrap());
                        });
                });
            }
        });
    });
}

/// Selected macro's name entry with tag chips below it.
fn paint_name_and_tags(app: &mut SqyreApp, ui: &mut egui::Ui, idx: usize, meta_enabled: bool) {
    app.workspace
        .macro_meta
        .sync_selection(idx, &app.workspace.macros[idx]);
    let other_names: Vec<String> = app
        .workspace
        .macros
        .iter()
        .map(|m| m.name.clone())
        .collect();
    let all_tags = collect_all_macro_tags(&app.workspace.macros);
    let (meta, persist_tags) = ui
        .vertical(|ui| {
            let m = &mut app.workspace.macros[idx];
            let meta = ui
                .horizontal(|ui| {
                    app.workspace
                        .macro_meta
                        .paint_name_row(ui, m, &other_names, meta_enabled)
                })
                .inner;
            let persist_tags =
                app.workspace
                    .macro_meta
                    .paint_tags_row(ui, m, &all_tags, meta_enabled);
            (meta, persist_tags)
        })
        .inner;
    if persist_tags {
        app.persist_macro_at(idx);
    }
    if let Some(new_name) = meta.rename_to {
        app.rename_selected_macro(new_name);
    }
}

/// Halo behind the Run / Stop button, painted into a slot reserved before the button.
/// Pulses only when `animate`; otherwise it holds full strength so idle frames need no
/// repaint.
#[cfg(not(target_arch = "wasm32"))]
fn paint_run_glow(
    ui: &egui::Ui,
    slot: egui::layers::ShapeIdx,
    rect: egui::Rect,
    color: egui::Color32,
    animate: bool,
) {
    let pulse = crate::widgets::controls::glow_pulse_if(ui, animate);
    let rounding = ui.visuals().widgets.inactive.corner_radius;
    let halo = crate::widgets::controls::glow_halo_shapes(rect, rounding, color, pulse);
    ui.painter().set(slot, egui::Shape::Vec(halo));
}

/// Run button that becomes a red-glowing Stop button while a macro runs, followed by a separator.
#[cfg(not(target_arch = "wasm32"))]
fn paint_run_stop(app: &mut SqyreApp, ui: &mut egui::Ui, running: bool) {
    let glow = (running || app.settings_ui.settings().run_button_glow)
        .then(|| ui.painter().add(egui::Shape::Noop));
    let (icon, tip, color) = if running {
        (
            "⏹",
            format!(
                "Stop (Esc). {} exits Sqyre (failsafe).",
                sqyre_hotkeys::FAILSAFE_LABEL
            ),
            theme::MACRO_STOP,
        )
    } else {
        ("▶", "Run".to_owned(), theme::MACRO_START)
    };
    let button = toolbar_icon_colored(ui, icon, &tip, true, Some(color));
    #[cfg(feature = "native-runtime")]
    if running {
        let (pressed, released, pos, focused) = ui.input(|i| {
            (
                i.pointer.any_pressed(),
                i.pointer.any_released(),
                i.pointer.interact_pos(),
                i.focused,
            )
        });
        if pressed || released {
            let layer = pos.and_then(|p| ui.ctx().layer_id_at(p));
            sqyre_capture::event_log(
                "SQYRE_STOPBTN",
                &[
                    ("phase", if pressed { "press" } else { "release" }),
                    ("pos", &format!("{pos:?}").replace(' ', "")),
                    ("rect", &format!("{:?}", button.rect).replace(' ', "")),
                    ("layer", &format!("{layer:?}").replace(' ', "")),
                    (
                        "btn_layer",
                        &format!("{:?}", ui.layer_id()).replace(' ', ""),
                    ),
                    ("hovered", if button.hovered() { "yes" } else { "no" }),
                    ("clicked", if button.clicked() { "yes" } else { "no" }),
                    ("focused", if focused { "yes" } else { "no" }),
                ],
            );
        }
    }
    if let Some(slot) = glow {
        paint_run_glow(ui, slot, button.rect, color, running || button.hovered());
    }
    if button.clicked() {
        if running {
            app.request_stop();
        } else {
            app.start_macro(ui.ctx());
        }
    }
    crate::widgets::section_separator(ui);
}

/// Delay + hotkey toggles.
fn paint_delay_hotkey(app: &mut SqyreApp, ui: &mut egui::Ui, idx: usize, running: bool) {
    app.workspace
        .macro_meta
        .paint_delay_button(ui, &app.workspace.macros[idx], !running);
    let hk_label = {
        let m = &app.workspace.macros[idx];
        if m.hotkey.is_empty() {
            sqyre_domain::EMPTY_NOT_SET.to_string()
        } else {
            format_hotkey(&m.hotkey)
        }
    };
    let hotkey_open = crate::widgets::wrap_unit(ui, |ui| {
        let open = app.workspace.macro_meta.paint_hotkey_toggle(ui);
        ui.weak(hk_label);
        open
    })
    .inner;
    if hotkey_open {
        crate::widgets::wrap_unit(ui, |ui| {
            theme::section_frame(ui.style())
                .inner_margin(egui::Margin::symmetric(theme::SPACE_4 as i8, 1))
                .show(ui, |ui| {
                    ui.horizontal(|ui| paint_hotkey_controls(app, ui, idx, running));
                });
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn show_update_banner(app: &mut SqyreApp, ui: &mut egui::Ui) {
    use crate::update::UpdateState;

    if !app.update.show_banner() {
        if let UpdateState::Ready { version } = &app.update.state {
            let version = version.clone();
            ui.horizontal(|ui| {
                ui.colored_label(
                    theme::ok_fg(),
                    format!("v{version} installed — restart to finish"),
                );
                if ui.small_button("Restart").clicked() {
                    crate::update::restart_app(&mut app.instance_lock);
                }
            });
        }
        return;
    }
    let version = app.update.available_version().unwrap_or("?").to_string();
    ui.horizontal(|ui| {
        ui.colored_label(theme::ok_fg(), format!("Update available: v{version}"));
        if ui.small_button("Download & install").clicked() {
            app.update.start_download();
        }
        if ui.small_button("Dismiss").clicked() {
            app.update.dismiss_banner();
        }
    });
}

/// Browser editor note at the top of the central panel.
/// Chrome controls (list toggle, Data Editor, Settings) live in [`top_bar`];
/// run/stop sits in [`action_toolbar`].
#[cfg(target_arch = "wasm32")]
pub fn wasm_editor_note(ui: &mut egui::Ui) {
    ui.small(
        "Browser editor: import/export db.yaml. Run, capture, and global hotkeys are desktop-only.",
    );
    crate::widgets::section_separator(ui);
}

/// Name/tags editor, delay popup, and validation status for the selected macro.
/// Returns `false` if macros became empty (caller should stop drawing the editor).
pub fn show_meta_and_hotkey(app: &mut SqyreApp, ui: &mut egui::Ui) -> bool {
    let running = app.run_session.state.running.load(Ordering::SeqCst);
    let idx = app
        .workspace
        .selected_macro
        .min(app.workspace.macros.len() - 1);
    app.workspace.selected_macro = idx;
    let meta_enabled = !running;
    paint_name_and_tags(app, ui, idx, meta_enabled);
    let idx = app.workspace.selected_macro;
    {
        let pending = app.pending_viewport_scale;
        let m = &mut app.workspace.macros[idx];
        let delay_out =
            app.workspace
                .macro_meta
                .paint_delay_popup(ui, m, meta_enabled, pending.as_ref());
        if delay_out.persist {
            app.persist_macro_at(idx);
        }
    }
    if let Err(e) = sqyre_validate::validate_macro(&app.workspace.macros[idx]) {
        // Truncate to the pane — keep the status on one line and avoid raising
        // CentralPanel min_size when the message is long.
        let msg = format!("Validation: {e}");
        ui.add(
            egui::Label::new(egui::RichText::new(&msg).color(crate::theme::error_fg())).truncate(),
        )
        .on_hover_text(&msg);
    }
    // Selection / length may have changed after rename.
    let idx = app
        .workspace
        .selected_macro
        .min(app.workspace.macros.len().saturating_sub(1));
    app.workspace.selected_macro = idx;
    if app.workspace.macros.is_empty() {
        return false;
    }
    true
}

fn paint_hotkey_controls(app: &mut SqyreApp, ui: &mut egui::Ui, idx: usize, running: bool) {
    if crate::widgets::record_icon_button(ui, "Record a global hotkey chord", !running).clicked() {
        app.hotkey_record.open(&app.run_session.macro_hotkeys);
    }
    if toolbar_icon(
        ui,
        egui_phosphor::regular::ERASER,
        crate::action_tooltip::help::META_HOTKEY_CLEAR,
        !running && !app.workspace.macros[idx].hotkey.is_empty(),
    )
    .clicked()
    {
        app.apply_hotkey_to_selected(Vec::new(), None);
    }

    let mut on_press =
        HotkeyTrigger::parse(&app.workspace.macros[idx].hotkey_trigger) == HotkeyTrigger::Press;
    let tip = if on_press {
        crate::action_tooltip::help::META_HOTKEY_PRESS
    } else {
        crate::action_tooltip::help::META_HOTKEY_RELEASE
    };
    let toggled = ui
        .add_enabled_ui(!running, |ui| {
            crate::widgets::icon_toggle(
                ui,
                &mut on_press,
                tip,
                egui_phosphor::regular::HAND_TAP,
                egui_phosphor::fill::HAND_TAP,
            )
        })
        .inner
        .changed();
    if toggled {
        let trigger = if on_press {
            HotkeyTrigger::Press
        } else {
            HotkeyTrigger::Release
        };
        let chord = app.workspace.macros[idx].hotkey.clone();
        app.apply_hotkey_to_selected(chord, Some(trigger));
    }
}

/// Single icon that pops up the AI and YAML Macro Builder entries.
fn paint_builder_menu(app: &mut SqyreApp, ui: &mut egui::Ui) {
    let resp = toolbar_icon(
        ui,
        egui_phosphor::regular::MAGIC_WAND,
        "Macro builders",
        true,
    );
    egui::Popup::menu(&resp).show(|ui| {
        let ai_label = format!("{} AI Macro Builder", egui_phosphor::regular::SPARKLE);
        if crate::widgets::menu_item(ui, &ai_label, true) {
            app.macro_prompt_builder.open_builder();
        }
        if crate::widgets::menu_item(ui, "{}  YAML Macro Builder", true) {
            if app.run_session.state.running.load(Ordering::SeqCst) {
                *app.run_session.state.status.lock() =
                    "Cannot open YAML Macro Builder while a macro is running.".into();
            } else {
                let selected = app.workspace.macros.get(app.workspace.selected_macro);
                app.macro_yaml_builder.open_builder(selected);
            }
        }
    });
}

/// Thin full-width footer: delay, hotkey, and macro builder buttons for the selected macro.
/// Must be shown before the side panel so it spans the whole window width.
pub fn footer_bar(app: &mut SqyreApp, ui: &mut egui::Ui) {
    if app.workspace.macros.is_empty() {
        return;
    }
    egui::Panel::bottom("app_footer_bar")
        .frame(egui::Frame::side_top_panel(ui.style()).inner_margin(egui::Margin::symmetric(8, 1)))
        .show(ui, |ui| {
            let running = app.run_session.state.running.load(Ordering::SeqCst);
            let idx = app
                .workspace
                .selected_macro
                .min(app.workspace.macros.len() - 1);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = theme::SPACE_4;
                // Right-to-left: builder menu rightmost, delay/hotkey fill the rest from the left.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    paint_builder_menu(app, ui);
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        paint_delay_hotkey(app, ui, idx, running);
                    });
                });
            });
        });
}

/// Action chrome (run/stop, add/vars/clipboard/history/expand-or-collapse).
/// `any_collapsed` comes from [`crate::ui_macro_tree::any_branch_collapsed`].
/// Returns expand/collapse force.
pub fn action_toolbar(
    app: &mut SqyreApp,
    ui: &mut egui::Ui,
    any_collapsed: Option<bool>,
) -> Option<bool> {
    let running = app.run_session.state.running.load(Ordering::SeqCst);
    let mut force_openness: Option<bool> = None;
    ui.horizontal_wrapped(|ui| {
        // Tight gap between toolbar icon buttons (scale SPACE_4).
        ui.spacing_mut().item_spacing.x = theme::SPACE_4;
        #[cfg(not(target_arch = "wasm32"))]
        paint_run_stop(app, ui, running);
        let can_copy = app.can_copy_selection();
        let can_paste = app.can_paste_clipboard();
        let can_undo = app.can_undo();
        let can_redo = app.can_redo();
        if toolbar_icon_colored(
            ui,
            "+",
            "Add Action (Ctrl+A)",
            !running,
            Some(theme::MACRO_START),
        )
        .clicked()
        {
            app.add_action_picker.open();
        }
        if crate::widgets::record_icon_button(ui, "Record actions (Esc to finish)", !running)
            .clicked()
            && app.macro_record.open(
                &app.run_session.macro_hotkeys,
                &app.macro_record_bridge,
                &mut app.workspace.catalog,
            )
        {
            if let Err(e) = app.persist_database() {
                app.report_persist_failure("Save after record prep", &e);
            }
        }
        // Light-theme variables pastel reads better as a glyph on dark chrome.
        let vars_color = theme::rgba(action_pastel_color("setvariable", false));
        if toolbar_icon_colored(ui, "x", "Variables", true, Some(vars_color)).clicked() {
            app.variables_panel.open = true;
        }
        crate::widgets::section_separator(ui);
        if toolbar_icon(ui, "📄", "Copy (Ctrl+C)", can_copy && !running).clicked() {
            app.copy_selection(ui.ctx());
        }
        if toolbar_icon(ui, "✂", "Cut (Ctrl+X)", can_copy && !running).clicked() {
            app.cut_selection(ui.ctx());
        }
        if toolbar_icon(ui, "📋", "Paste (Ctrl+V)", can_paste && !running).clicked() {
            app.paste_clipboard();
        }
        if toolbar_icon(ui, "↺", "Undo (Ctrl+Z)", can_undo && !running).clicked() {
            app.undo_tree();
        }
        if toolbar_icon(ui, "↻", "Redo (Ctrl+Y)", can_redo && !running).clicked() {
            app.redo_tree();
        }
        let expand = any_collapsed.unwrap_or(false);
        let (glyph, tip) = if expand {
            (egui_phosphor::regular::TREE_VIEW, "Expand all branches")
        } else {
            (
                egui_phosphor::regular::SQUARE_SPLIT_VERTICAL,
                "Collapse all branches",
            )
        };
        if toolbar_icon(ui, glyph, tip, any_collapsed.is_some()).clicked() {
            force_openness = Some(expand);
        }
    });
    ui.add_space(theme::SPACE_4);
    force_openness
}
