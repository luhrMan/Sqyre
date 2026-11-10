//! Window top bar (brand / Data Editor / Settings), footer (delay / hotkey / builders), and action chrome.

use crate::help_text as ht;
use crate::macro_meta::collect_macro_tag_completions;
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
    ui.add_enabled(enabled, crate::widgets::IconButton::new(glyph, tip, color))
}

/// Full-width window header above the macro list and central panel:
/// Sqyre brand with Data Editor / Settings stacked under it (tag-row plane),
/// name/tags in the mid gap, macro-list toggle on that same plane at the right.
/// Must be shown before the side panel so it spans the whole window width.
pub fn top_bar(app: &mut SqyreApp, ui: &mut egui::Ui) {
    egui::Panel::top("app_top_bar")
        .frame(egui::Frame::side_top_panel(ui.style()))
        .show(ui, |ui| {
            crate::widgets::show_overflow(ui, "app_top_bar", 1.0, |ui| {
                brand_header(app, ui);
            });
        });
}

fn brand_header(app: &mut SqyreApp, ui: &mut egui::Ui) {
    let running = app.run_session.state.running.load(Ordering::SeqCst);
    // Top-align brand with the name row. Data Editor / Settings / list toggle
    // float onto the tag-chip plane afterward so they do not stack a third row
    // under a tall brand column.
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
        let brand_rect = paint_brand_button(app, ui);

        let gap = ui.spacing().item_spacing.x;
        let list_side = crate::widgets::icon_btn_side(ui);
        let list_reserve = list_side + gap;

        // Top-right update banner (native only).
        #[cfg(not(target_arch = "wasm32"))]
        let mid_right = {
            let right = ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                show_update_banner(app, ui);
            });
            let chrome_left = if right.response.rect.width() > 0.0 {
                right.response.rect.left() - gap
            } else {
                ui.max_rect().right()
            };
            chrome_left.min(ui.max_rect().right() - list_reserve)
        };
        #[cfg(target_arch = "wasm32")]
        let mid_right = ui.max_rect().right() - list_reserve;

        let stack_id = ui.make_persistent_id("name_tag_stack_h");
        let estimate = ui.spacing().interact_size.y * 2.0
            + ui.spacing().item_spacing.y
            + crate::theme::SPACE_4;
        let stack_h = ui
            .ctx()
            .data(|d| d.get_temp::<f32>(stack_id))
            .unwrap_or(estimate);
        let top = ui.max_rect().top();
        // Mid paints into last frame's stack height; final allocate comes after
        // chrome Y is known so a lone under-brand row still fits.
        let mid_rect = egui::Rect::from_min_max(
            egui::pos2(brand_rect.right() + gap, top),
            egui::pos2(mid_right, top + brand_rect.height().max(stack_h)),
        );

        let chrome_h = list_side + ui.spacing().item_spacing.y;
        let mut painted_h = brand_rect.height();
        if mid_rect.width() > 40.0 {
            let mut mid = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(mid_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Min)),
            );
            mid.set_clip_rect(mid_rect);
            if let Some(idx) = app.workspace.selected_macro_index() {
                paint_name_and_tags(app, &mut mid, idx, !running);
                painted_h = mid.min_rect().height().max(1.0);
                if (painted_h - stack_h).abs() > 0.5 {
                    ui.ctx().data_mut(|d| d.insert_temp(stack_id, painted_h));
                    ui.ctx().request_repaint();
                }
            }
        }

        // Two-band name/tag stack → sit chrome on the lower band; otherwise
        // park it under the brand (adds at most one compact row).
        let tag_plane_y = if painted_h > brand_rect.height() + ui.spacing().item_spacing.y {
            top + painted_h - chrome_h
        } else {
            brand_rect.bottom()
        };
        let header_h = brand_rect
            .height()
            .max(painted_h)
            .max((tag_plane_y - top) + chrome_h);
        if !app.workspace.macros.is_empty() || header_h > brand_rect.height() {
            ui.allocate_space(egui::vec2(0.0, header_h));
        }

        // Data Editor / Settings under the brand, on the tag plane.
        let chrome_w = brand_rect
            .width()
            .max(list_side * 2.0 + ui.spacing().item_spacing.x);
        let chrome_rect = egui::Rect::from_min_size(
            egui::pos2(brand_rect.left(), tag_plane_y),
            egui::vec2(chrome_w, chrome_h),
        );
        let mut chrome_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(chrome_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        paint_left_chrome(app, &mut chrome_ui);

        // List toggle on the same plane at the right edge.
        let list_rect = egui::Rect::from_min_size(
            egui::pos2(ui.max_rect().right() - list_side, tag_plane_y),
            egui::Vec2::splat(list_side),
        );
        let mut list_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(list_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        paint_list_toggle(app, &mut list_ui);
    });
}

/// Data Editor / Settings (and wasm import/export) under the brand icon.
fn paint_left_chrome(app: &mut SqyreApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if toolbar_icon(ui, "📁", "Data Editor", true).clicked() {
            app.data_editor.request_open(ui.ctx());
        }
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
        #[cfg(target_arch = "wasm32")]
        {
            if toolbar_icon(ui, "⬆", &format!("Export {}", sqyre_persist::DB_FILE), true).clicked()
            {
                app.export_db_yaml();
            }
            if toolbar_icon(ui, "⬇", &format!("Import {}", sqyre_persist::DB_FILE), true).clicked()
            {
                app.request_db_import();
            }
        }
    });
}

fn paint_list_toggle(app: &mut SqyreApp, ui: &mut egui::Ui) {
    let (list_glyph, list_tip) = if app.macro_list_open {
        ("▷", "Hide macro list")
    } else {
        ("☰", "Show macro list")
    };
    if toolbar_icon(ui, list_glyph, list_tip, true).clicked() {
        app.macro_list_open = !app.macro_list_open;
    }
}

/// Brand / command palette hit target at the top-left of the window.
/// While a macro runs the shield keeps its outline and the S becomes the
/// animated trace spinner shared with overlay buttons and loading slots.
fn paint_brand_button(app: &mut SqyreApp, ui: &mut egui::Ui) -> egui::Rect {
    let running = app.run_session.state.running.load(Ordering::SeqCst);
    #[cfg(feature = "native-runtime")]
    let waking = sqyre_capture::pipewire_stream_waking();
    #[cfg(not(feature = "native-runtime"))]
    let waking = false;
    let busy = running || waking;
    let brand_tip = if waking {
        ht::CAPTURE_WAKE
    } else {
        ht::TB_PALETTE
    };
    let size = egui::vec2(28.0, 28.0);
    // Pad the hit rect so the hover frame doesn't hug the icon edge.
    let (hit_rect, icon_resp) = ui.allocate_exact_size(
        size + egui::Vec2::splat(theme::SPACE_4 * 2.0),
        egui::Sense::click(),
    );
    let icon_rect = egui::Rect::from_center_size(hit_rect.center(), size);
    let icon_resp = icon_resp
        .on_hover_text(brand_tip)
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    // Same hover/press frame a `frame_when_inactive(false)` button draws.
    let state = icon_resp.widget_state();
    if !matches!(
        state,
        egui::widget_style::WidgetState::Inactive | egui::widget_style::WidgetState::Noninteractive
    ) {
        let visuals = ui.visuals().widgets.state(state);
        ui.painter().rect(
            hit_rect.expand(visuals.expansion),
            visuals.corner_radius,
            visuals.weak_bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
    }
    icon_resp.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            ui.is_enabled(),
            if waking {
                ht::CAPTURE_WAKE
            } else {
                sqyre_ports::APP_NAME
            },
        )
    });
    if busy {
        // The shield stays; the S inside it becomes the trace spinner —
        // drawn into a hollow cut by `sqyre_shell` at the S's exact
        // viewBox region (`40.5 20.25 46.75 84` of the 128×128 shield).
        let shell = app.icon_cache.sqyre_shell(ui.ctx());
        ui.painter().image(
            shell.id(),
            icon_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        let s_rect = egui::Rect::from_min_max(
            icon_rect.min
                + egui::vec2(
                    icon_rect.width() * (40.5 / 128.0),
                    icon_rect.height() * (20.25 / 128.0),
                ),
            icon_rect.min
                + egui::vec2(
                    icon_rect.width() * (87.25 / 128.0),
                    icon_rect.height() * (104.25 / 128.0),
                ),
        );
        sqyre_overlay::icons::paint_trace_spinner(ui, s_rect, sqyre_ui_theme::accent());
        // Pace like the run-glow pulse — a bare request_repaint can be
        // dropped on an idle window (see hotkey_wake), freezing the icon.
        ui.ctx()
            .request_repaint_after(sqyre_overlay::icons::SPINNER_TICK);
    } else {
        let tex = app.icon_cache.sqyre_fallback(ui.ctx());
        ui.painter().image(
            tex.id(),
            icon_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
    if icon_resp.clicked() {
        app.command_palette.open_palette();
    }
    hit_rect
}

/// Selected macro's name entry, tag field to its right, created tags below.
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
    let all_tags = collect_macro_tag_completions(
        &app.workspace.macros,
        app.workspace
            .catalog
            .program_names()
            .map(|name| name.as_str()),
    );
    let program_icons = app
        .icon_cache
        .program_name_icons(ui.ctx(), &app.workspace.catalog);
    let m = &mut app.workspace.macros[idx];
    let laid = app.workspace.macro_meta.paint_name_and_tags(
        ui,
        m,
        &other_names,
        &all_tags,
        meta_enabled,
        Some(&program_icons),
    );
    if laid.mutations.persist {
        app.persist_macro_at(idx);
    }
    if let Some(new_name) = laid.mutations.rename_to {
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
/// Chrome controls (Data Editor / Settings under the brand, list toggle right)
/// live in [`top_bar`]; run/stop sits in [`action_toolbar`].
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
    app.workspace.selected_macro = app.workspace.selected_macro_index().unwrap_or(0);
    let meta_enabled = !running;
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
        crate::widgets::error_icon(ui, &format!("Validation: {e}"));
    }
    // Selection / length may have changed after rename.
    app.workspace.selected_macro = app.workspace.selected_macro_index().unwrap_or(0);
    !app.workspace.macros.is_empty()
}

fn paint_hotkey_controls(app: &mut SqyreApp, ui: &mut egui::Ui, idx: usize, running: bool) {
    if crate::widgets::record_icon_button(ui, "Record a global hotkey chord", !running).clicked() {
        app.hotkey_record.open(&app.run_session.macro_hotkeys);
    }
    if toolbar_icon_colored(
        ui,
        egui_phosphor::regular::ERASER,
        crate::action_tooltip::help::META_HOTKEY_CLEAR,
        !running && !app.workspace.macros[idx].hotkey.is_empty(),
        Some(theme::MACRO_STOP),
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
        .add_enabled(
            !running,
            crate::widgets::IconToggle::new(
                &mut on_press,
                tip,
                egui_phosphor::regular::HAND_TAP,
                egui_phosphor::fill::HAND_TAP,
            ),
        )
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

/// Floating delay / hotkey / builder buttons pinned to the central panel's
/// bottom-right corner. Replaces the old reserved footer strip.
pub fn footer_floats(app: &mut SqyreApp, ui: &mut egui::Ui) {
    if app.workspace.macros.is_empty() {
        return;
    }
    let running = app.run_session.state.running.load(Ordering::SeqCst);
    let idx = app.workspace.selected_macro_index().unwrap_or(0);
    crate::widgets::float_bottom_right(ui, ui.clip_rect(), |ui| {
        // Right-to-left: builder menu rightmost; then hotkey controls, chord,
        // key toggle, delay — the same left-to-right order the footer strip had.
        paint_builder_menu(app, ui);
        if app.workspace.macro_meta.hotkey_open {
            theme::section_frame(ui.style())
                .inner_margin(egui::Margin::symmetric(theme::SPACE_4 as i8, 1))
                .show(ui, |ui| {
                    ui.horizontal(|ui| paint_hotkey_controls(app, ui, idx, running));
                });
        }
        let hk_label = {
            let m = &app.workspace.macros[idx];
            if m.hotkey.is_empty() {
                sqyre_domain::EMPTY_NOT_SET.to_string()
            } else {
                format_hotkey(&m.hotkey)
            }
        };
        ui.weak(hk_label);
        app.workspace.macro_meta.paint_hotkey_toggle(ui);
        app.workspace
            .macro_meta
            .paint_delay_button(ui, &app.workspace.macros[idx], !running);
        // Leftmost: running status as a hoverable icon (was the header label).
        let status = app.run_session.state.status.lock().clone();
        if !status.is_empty() {
            crate::widgets::controls::status_icon(
                ui,
                egui_phosphor::regular::INFO,
                sqyre_ui_theme::accent(),
                &status,
            );
        }
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
    force_openness
}

#[cfg(test)]
mod tests {
    /// The tag field sits in the header gap. On a narrow window that gap cannot
    /// hold both entries, and the name used to collapse to a sliver.
    #[test]
    fn narrow_header_shows_the_macro_name() {
        let mut app = crate::SqyreApp::for_docs();
        app.workspace.macros[0].name =
            "Inventory restock and backup routine for the evening".into();
        let ctx = eframe::egui::Context::default();
        // Chrome icons moved off this row, so the name/tag gap is wider than
        // before — keep the window tight enough that the tag field still wraps.
        let screen = eframe::egui::Rect::from_min_size(
            eframe::egui::Pos2::ZERO,
            eframe::egui::vec2(320.0, 640.0),
        );
        let mut visible = 0.0;
        let mut name_y = 0.0;
        let mut tag_y = None;
        for frame in 0..3 {
            let out = ctx.run_ui(
                eframe::egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(frame as f64 * 0.1),
                    ..Default::default()
                },
                |ui| {
                    super::top_bar(&mut app, ui);
                },
            );
            for clipped in &out.shapes {
                if let eframe::egui::Shape::Text(text) = &clipped.shape {
                    let label = text.galley.text();
                    if label.contains("Inventory") {
                        let galley =
                            eframe::egui::Rect::from_min_size(text.pos, text.galley.size());
                        visible = galley.intersect(clipped.clip_rect).width();
                        name_y = text.pos.y;
                    } else if label.contains("Add tag") {
                        tag_y = Some(text.pos.y);
                    }
                }
            }
            out.drop_without_applying_deltas();
        }
        assert!(
            visible > 80.0,
            "macro name text is clipped to {visible}px in the header"
        );
        let tag_y = tag_y.expect("tag field");
        assert!(
            tag_y > name_y + 8.0,
            "tag field did not wrap under the name (name_y {name_y}, tag_y {tag_y})"
        );
    }
}
