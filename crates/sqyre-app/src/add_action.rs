//! Add Action picker dialog.
//!
//! Hover a tile ~1s for a **view** tip of that type’s defaults; right-click opens
//! **edit** (persisted in user settings). Left-click inserts a clone into the tree.

use crate::action_tooltip::{
    apply_picker_result, paint_action_edit_header, paint_edit_fields, show_action_view_tip,
};
use crate::hotkey_record::HotkeyRecordUi;
use crate::icon_cache::IconCache;
use crate::key_record::KeyRecordUi;
use crate::paint_ctx::{CatalogPaint, EditFieldsCtx, RecordBridges, VarTheme};
use crate::pickers::{self, ActivePicker};
use crate::preview_tooltip::PreviewTooltipCache;
use crate::tree_chrome;
use crate::widgets::SaveCancel;
use eframe::egui::{self, Color32, CornerRadius, Key, Sense, Vec2};
use sqyre_domain::{
    action_templates, action_type_label, blank_action, Action, ActionId, ActionTemplate,
    ACTION_PICKER_CATEGORIES,
};
use sqyre_hotkeys::{MacroHotkeyBridge, ScreenClickBridge};
use sqyre_persist::ProgramCatalog;
use sqyre_persist::UserSettings;
use sqyre_serialize::{action_from_map, action_to_map};
use sqyre_ui_model::{action_icon_glyph, action_pastel_color};
use sqyre_validate::validate_action;
use std::collections::{HashMap, HashSet};
use web_time::{Duration, Instant};

/// How long a tile must be hovered before the defaults **view** tip opens.
const DEFAULTS_HOVER_DELAY: Duration = Duration::from_secs(1);

/// Modal state for the categorized blank-action picker.
#[derive(Debug, Default)]
pub struct AddActionPicker {
    pub open: bool,
    /// Overrides for blank templates keyed by action type.
    defaults: HashMap<String, Action>,
    /// View / edit tip for a type’s default prototype.
    tip: Option<DefaultsTip>,
    /// Tile currently under the pointer, waiting for [`DEFAULTS_HOVER_DELAY`].
    hover_pending: Option<HoverPending>,
}

#[derive(Debug, Clone)]
struct HoverPending {
    action_type: String,
    anchor: egui::Pos2,
    since: Instant,
}

#[derive(Debug, Clone)]
struct DefaultsEdit {
    action_type: String,
    draft: Action,
    error: Option<String>,
    anchor: egui::Pos2,
    picker: ActivePicker,
}

#[derive(Debug, Clone)]
enum DefaultsTip {
    View {
        action_type: String,
        action: Box<Action>,
    },
    Edit(Box<DefaultsEdit>),
}

impl DefaultsTip {
    fn is_editing(&self) -> bool {
        matches!(self, Self::Edit { .. })
    }
}

impl AddActionPicker {
    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn load_from_settings(&mut self, settings: &UserSettings) {
        self.defaults.clear();
        for (ty, map) in &settings.action_defaults {
            if let Ok(action) = action_from_map(map) {
                self.defaults.insert(ty.clone(), action);
            }
        }
    }

    pub fn store_into_settings(&self, settings: &mut UserSettings) {
        settings.action_defaults.clear();
        for (ty, action) in &self.defaults {
            if let Ok(map) = action_to_map(action) {
                settings.action_defaults.insert(ty.clone(), map);
            }
        }
    }

    /// Fresh action for insert: cloned default (new UID) or built-in blank.
    pub fn create_action(&self, action_type: &str) -> Option<Action> {
        if let Some(proto) = self.defaults.get(action_type) {
            let mut a = proto.clone();
            reassign_uids(&mut a);
            return Some(a);
        }
        blank_action(action_type)
    }

    fn prototype_for(&self, action_type: &str) -> Option<Action> {
        if let Some(proto) = self.defaults.get(action_type) {
            return Some(proto.clone());
        }
        blank_action(action_type)
    }

    fn open_view(&mut self, action_type: String) {
        let Some(action) = self.prototype_for(&action_type) else {
            return;
        };
        self.tip = Some(DefaultsTip::View {
            action_type,
            action: Box::new(action),
        });
        self.hover_pending = None;
    }

    fn open_edit(&mut self, action_type: String, anchor: egui::Pos2) {
        let Some(draft) = self.prototype_for(&action_type) else {
            return;
        };
        self.tip = Some(DefaultsTip::Edit(Box::new(DefaultsEdit {
            action_type,
            draft,
            error: None,
            anchor,
            picker: ActivePicker::None,
        })));
        self.hover_pending = None;
    }

    /// Apply a key captured by [`KeyRecordUi`] onto a Key-action defaults draft.
    pub fn apply_recorded_key(&mut self, recorded: String) {
        let Some(DefaultsTip::Edit(edit)) = self.tip.as_mut() else {
            return;
        };
        crate::recorded_action::apply_recorded_key(&mut edit.draft.kind, recorded);
    }

    /// Apply a chord onto a Pause continue-key defaults draft. Returns true when applied.
    pub fn apply_recorded_chord(&mut self, recorded: Vec<String>) -> bool {
        let Some(DefaultsTip::Edit(edit)) = self.tip.as_mut() else {
            return false;
        };
        crate::recorded_action::apply_recorded_chord(&mut edit.draft.kind, recorded)
    }

    /// Apply a hex color from the Find Pixel screen dropper onto a defaults draft.
    pub fn apply_recorded_color(&mut self, recorded: String) {
        let Some(DefaultsTip::Edit(edit)) = self.tip.as_mut() else {
            return;
        };
        crate::recorded_action::apply_recorded_color(&mut edit.draft.kind, recorded);
    }

    /// Draw the picker when open. Returns a freshly constructed blank [`Action`] when
    /// the user picks a tile (caller inserts it into the tree).
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        previews: &mut PreviewTooltipCache,
        macros: &[(String, Vec<String>)],
        known_vars: &HashSet<String>,
        key_record: &mut KeyRecordUi,
        hotkey_record: &mut HotkeyRecordUi,
        macro_hotkeys: &MacroHotkeyBridge,
        screen_click: &ScreenClickBridge,
        mut on_defaults_saved: impl FnMut(&AddActionPicker),
    ) -> Option<Action> {
        if !self.open {
            return None;
        }

        let mut open = self.open;
        let mut picked: Option<Action> = None;
        let mut hover_type: Option<(String, egui::Pos2)> = None;
        let mut edit_request: Option<(String, egui::Pos2)> = None;
        let editing = self.tip.as_ref().is_some_and(|t| t.is_editing());

        egui::Window::new("Add Action")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([900.0, 420.0])
            .min_width(640.0)
            .min_height(280.0)
            .show(ctx, |ui| {
                let is_dark = ui.visuals().dark_mode;
                ui.label(
                    "Pick an action type — hover ~1s to preview defaults, right-click to edit",
                );
                ui.add_space(6.0);

                let templates = action_templates();
                let list_h = pickers::popup_scroll_max_height(ui, 0.0);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(list_h)
                    .show(ui, |ui| {
                        ui.columns(ACTION_PICKER_CATEGORIES.len(), |cols| {
                            for (col_i, category) in ACTION_PICKER_CATEGORIES.iter().enumerate() {
                                let col = &mut cols[col_i];
                                col.strong(*category);
                                col.add_space(4.0);
                                for tmpl in templates.iter().filter(|t| t.category == *category) {
                                    let sample = self
                                        .prototype_for(tmpl.action_type)
                                        .unwrap_or_else(|| tmpl.create());
                                    let resp = picker_tile(col, tmpl, &sample, is_dark);
                                    if resp.secondary_clicked() {
                                        edit_request = Some((
                                            tmpl.action_type.to_string(),
                                            resp.rect.right_top(),
                                        ));
                                    } else if resp.clicked() {
                                        picked = self.create_action(tmpl.action_type);
                                    } else if resp.hovered() {
                                        hover_type = Some((
                                            tmpl.action_type.to_string(),
                                            resp.rect.right_top(),
                                        ));
                                    }
                                }
                            }
                        });
                    });
            });

        if let Some((ty, anchor)) = edit_request {
            self.open_edit(ty, anchor);
        } else if editing {
            // Keep edit tip open; ignore hover-driven view changes.
            self.hover_pending = None;
        } else if let Some((ty, anchor)) = hover_type {
            let viewing_same = matches!(
                &self.tip,
                Some(DefaultsTip::View { action_type, .. }) if action_type == &ty
            );
            if viewing_same {
                self.hover_pending = None;
            } else {
                let restart = match &self.hover_pending {
                    Some(p) => p.action_type != ty,
                    None => true,
                };
                if restart {
                    self.hover_pending = Some(HoverPending {
                        action_type: ty,
                        anchor,
                        since: Instant::now(),
                    });
                } else if let Some(p) = &mut self.hover_pending {
                    p.anchor = anchor;
                }
            }
        } else {
            // Left the tile: dismiss view tip (edit stays until Cancel).
            self.hover_pending = None;
            if matches!(&self.tip, Some(DefaultsTip::View { .. })) {
                self.tip = None;
            }
        }

        if !editing {
            if let Some(pending) = &self.hover_pending {
                let elapsed = pending.since.elapsed();
                if elapsed >= DEFAULTS_HOVER_DELAY {
                    let ty = pending.action_type.clone();
                    self.open_view(ty);
                } else {
                    ctx.request_repaint_after(DEFAULTS_HOVER_DELAY.saturating_sub(elapsed));
                }
            }
        }

        let is_dark = ctx.global_style().visuals.dark_mode;
        let defaults_saved = match &self.tip {
            Some(DefaultsTip::View { .. }) => {
                self.show_defaults_view(ctx, catalog, icons, previews, known_vars, is_dark);
                false
            }
            Some(DefaultsTip::Edit { .. }) => self.show_defaults_edit(
                ctx,
                catalog,
                icons,
                previews,
                macros,
                known_vars,
                key_record,
                hotkey_record,
                macro_hotkeys,
                screen_click,
            ),
            None => false,
        };
        if defaults_saved {
            on_defaults_saved(self);
        }

        if picked.is_some() {
            open = false;
            self.tip = None;
            self.hover_pending = None;
        }
        self.open = open;
        if !self.open {
            self.tip = None;
            self.hover_pending = None;
        }
        picked
    }

    fn show_defaults_view(
        &self,
        ctx: &egui::Context,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        previews: &mut PreviewTooltipCache,
        known_vars: &HashSet<String>,
        is_dark: bool,
    ) {
        let Some(DefaultsTip::View {
            action_type,
            action,
        }) = &self.tip
        else {
            return;
        };
        show_action_view_tip(
            ctx,
            egui::Id::new(("action_default_view", action_type.as_str())),
            action,
            &mut CatalogPaint {
                catalog,
                icons,
                previews,
            },
            VarTheme {
                known_vars,
                is_dark,
            },
        );
    }

    /// Returns true when defaults were saved this frame.
    #[allow(clippy::too_many_arguments)]
    fn show_defaults_edit(
        &mut self,
        ctx: &egui::Context,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        previews: &mut PreviewTooltipCache,
        macros: &[(String, Vec<String>)],
        known_vars: &HashSet<String>,
        key_record: &mut KeyRecordUi,
        hotkey_record: &mut HotkeyRecordUi,
        macro_hotkeys: &MacroHotkeyBridge,
        screen_click: &ScreenClickBridge,
    ) -> bool {
        if !matches!(&self.tip, Some(DefaultsTip::Edit { .. })) {
            return false;
        }

        // Escape: close nested pickers first, then the edit tip.
        // Skip while key / chord / screen recording.
        if !key_record.is_open()
            && !hotkey_record.is_open()
            && !screen_click.is_armed()
            && ctx.input(|i| i.key_pressed(Key::Escape))
        {
            if let Some(DefaultsTip::Edit(edit)) = self.tip.as_mut() {
                match &mut edit.picker {
                    p @ ActivePicker::Items { .. }
                    | p @ ActivePicker::Coord { .. }
                    | p @ ActivePicker::Macro { .. }
                    | p @ ActivePicker::Window { .. } => {
                        *p = ActivePicker::None;
                        return false;
                    }
                    ActivePicker::None => {}
                }
            }
            self.tip = None;
            return false;
        }

        let (type_key, anchor) = match &self.tip {
            Some(DefaultsTip::Edit(edit)) => (edit.action_type.clone(), edit.anchor),
            _ => return false,
        };
        let label = action_type_label(&type_key);
        let is_dark = ctx.global_style().visuals.dark_mode;
        let pastel = tree_chrome::rgba_pub(action_pastel_color(&type_key, is_dark));

        if let Some(DefaultsTip::Edit(edit)) = self.tip.as_mut() {
            let result = pickers::show_active_picker(
                ctx,
                &mut edit.picker,
                &mut CatalogPaint {
                    catalog,
                    icons,
                    previews,
                },
                macros,
            );
            apply_picker_result(&mut edit.draft, result);
        }

        let mut save = false;
        let mut cancel = false;
        let mut open = true;
        let err_msg = match &self.tip {
            Some(DefaultsTip::Edit(edit)) => edit.error.clone(),
            _ => None,
        };

        egui::Window::new(format!("Default: {label}"))
            .id(egui::Id::new(("action_default_edit", type_key.as_str())))
            .open(&mut open)
            .title_bar(true)
            .collapsible(false)
            .resizable(true)
            .constrain(true)
            .default_pos(anchor + Vec2::new(8.0, 0.0))
            .default_size([340.0, 360.0])
            .min_size([220.0, 120.0])
            .show(ctx, |ui| {
                match paint_action_edit_header(
                    ui,
                    label,
                    pastel,
                    Some("New actions of this type start with these values"),
                    err_msg.as_deref(),
                ) {
                    SaveCancel::Cancel => cancel = true,
                    SaveCancel::Save => save = true,
                    SaveCancel::None => {}
                }
                let list_h = pickers::popup_scroll_max_height(ui, 0.0);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(list_h)
                    .show(ui, |ui| {
                        if let Some(DefaultsTip::Edit(edit)) = self.tip.as_mut() {
                            let mut fields = EditFieldsCtx {
                                paint: CatalogPaint {
                                    catalog,
                                    icons,
                                    previews,
                                },
                                bridges: RecordBridges {
                                    key_record,
                                    hotkey_record,
                                    macro_hotkeys,
                                    screen_click,
                                },
                                theme: VarTheme {
                                    known_vars,
                                    is_dark,
                                },
                                macros,
                                active_macro: None,
                            };
                            paint_edit_fields(ui, &mut edit.draft, &mut edit.picker, &mut fields);
                        }
                    });
            });

        if !open || cancel {
            self.tip = None;
            return false;
        }

        if save {
            let (ty, draft) = match &self.tip {
                Some(DefaultsTip::Edit(edit)) => (edit.action_type.clone(), edit.draft.clone()),
                _ => return false,
            };
            if let Err(e) = validate_action(&draft, None) {
                if let Some(DefaultsTip::Edit(edit)) = self.tip.as_mut() {
                    edit.error = Some(e.to_string());
                }
                return false;
            }
            self.defaults.insert(ty, draft);
            self.tip = None;
            return true;
        }
        false
    }
}

fn reassign_uids(action: &mut Action) {
    action.id = ActionId::new();
    if let Some(kids) = action.children_mut() {
        for kid in kids.iter_mut() {
            reassign_uids(kid);
        }
    }
    if let Some(kids) = action.else_children_mut() {
        for kid in kids.iter_mut() {
            reassign_uids(kid);
        }
    }
}

fn picker_tile(
    ui: &mut egui::Ui,
    tmpl: &ActionTemplate,
    sample: &Action,
    is_dark: bool,
) -> egui::Response {
    let glyph = action_icon_glyph(sample);
    let pastel = action_pastel_color(tmpl.action_type, is_dark);
    let fill = Color32::from_rgba_unmultiplied(pastel[0], pastel[1], pastel[2], pastel[3]);

    let desired = Vec2::new(ui.available_width().max(120.0), 36.0);
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click());

    let visuals = ui.style().interact(&response);
    let bg = if response.hovered() {
        fill.gamma_multiply(1.15)
    } else {
        fill
    };
    ui.painter().rect_filled(rect, CornerRadius::same(8), bg);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(8),
        egui::Stroke::new(1.0, visuals.bg_stroke.color),
        egui::StrokeKind::Inside,
    );

    let text = format!("{glyph}  {}", tmpl.label);
    let fg = crate::theme::contrast_fg(fill);
    let galley = ui
        .painter()
        .layout_no_wrap(text, egui::FontId::proportional(14.0), fg);
    let text_pos = egui::pos2(rect.left() + 10.0, rect.center().y - galley.size().y * 0.5);
    ui.painter().galley(text_pos, galley, Color32::PLACEHOLDER);

    // No egui `on_hover_text` — the delayed action view tip is the hover UI.
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqyre_domain::action_templates;

    #[test]
    fn every_template_has_a_description_or_label() {
        for t in action_templates() {
            assert!(!t.label.is_empty());
            let a = t.create();
            assert_eq!(a.type_key(), t.action_type);
        }
    }

    #[test]
    fn create_action_uses_override_defaults() {
        let mut picker = AddActionPicker::default();
        let mut wait = blank_action("wait").unwrap();
        if let sqyre_domain::ActionKind::Wait { time } = &mut wait.kind {
            *time = sqyre_domain::ScalarValue::Int(777);
        }
        picker.defaults.insert("wait".into(), wait);
        let created = picker.create_action("wait").unwrap();
        match created.kind {
            sqyre_domain::ActionKind::Wait { time } => {
                assert_eq!(time, sqyre_domain::ScalarValue::Int(777));
            }
            other => panic!("unexpected {other:?}"),
        }
        // Fresh UID each create.
        let again = picker.create_action("wait").unwrap();
        assert_ne!(created.id, again.id);
    }
}
