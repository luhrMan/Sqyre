//! Data Editor modal prompts: confirm, variant name, overlay icon picker, window picker.
//!
//! Draw methods return the user's decision; [`super::DataEditor`] applies it, so this
//! module never imports the editor and can be exercised with a bare `egui::Context`.

use crate::data_editor_preview::variant_display_label;
use crate::icon_cache::IconCache;
use crate::pickers::ActivePicker;
use crate::widgets::ViewportScaleEvent;
use eframe::egui;
use sqyre_persist::CatalogView;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PendingConfirm {
    Delete {
        label: String,
    },
    Overwrite {
        kind: &'static str,
        name: String,
    },
    DeleteVariant {
        variant: String,
    },
    OverwriteVariant {
        variant: String,
        source: PathBuf,
    },
    /// Remove the mask PNG for the selected mask entity (shape settings stay).
    DeleteMaskImage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VariantPrompt {
    /// Ask for a name before adding a non-first variant.
    Name { source: PathBuf },
}

/// Selection the confirm body describes (program icon, mask entity name).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ConfirmSubject<'a> {
    pub program: Option<&'a str>,
    pub entity: Option<&'a str>,
}

/// Modal prompt state owned by the Data Editor.
#[derive(Debug, Default)]
pub(crate) struct EditorPrompts {
    pub confirm: Option<PendingConfirm>,
    pub variant_prompt: Option<VariantPrompt>,
    pub variant_name_draft: String,
    /// Overlay button id whose icon picker popup is open.
    pub overlay_icon_picker_for: Option<String>,
    /// Filter text for the overlay icon picker.
    pub overlay_icon_search: String,
    /// Running-window / coordinate / items picker for the active form.
    pub window_picker: ActivePicker,
}

impl EditorPrompts {
    /// Any prompt that owns keyboard input (Enter / Delete) over the editor body.
    pub(crate) fn any_open(&self) -> bool {
        self.confirm.is_some()
            || self.variant_prompt.is_some()
            || self.window_picker.is_open()
            || self.overlay_icon_picker_for.is_some()
    }

    /// Close prompts tied to the current entity selection.
    pub(crate) fn clear_entity_prompts(&mut self) {
        self.variant_prompt = None;
        self.overlay_icon_picker_for = None;
    }

    pub(crate) fn ask_variant_name(&mut self, source: PathBuf) {
        self.variant_name_draft.clear();
        self.variant_prompt = Some(VariantPrompt::Name { source });
    }

    pub(crate) fn open_overlay_icon_picker(&mut self, button_id: String) {
        self.overlay_icon_search.clear();
        self.overlay_icon_picker_for = Some(button_id);
    }

    /// Draw the pending confirm; returns it once the user confirms.
    pub(crate) fn draw_confirm(
        &mut self,
        ctx: &egui::Context,
        catalog: CatalogView<'_>,
        icons: &mut IconCache,
        subject: ConfirmSubject<'_>,
        pending_scale: Option<&ViewportScaleEvent>,
    ) -> Option<PendingConfirm> {
        let confirm = self.confirm.clone()?;
        let (title, confirm_label) = match &confirm {
            PendingConfirm::Delete { .. } | PendingConfirm::DeleteVariant { .. } => {
                ("Delete", "Delete")
            }
            PendingConfirm::DeleteMaskImage => ("Remove Image", "Remove"),
            PendingConfirm::Overwrite { .. } | PendingConfirm::OverwriteVariant { .. } => {
                ("Overwrite", "Overwrite")
            }
        };
        let mut outcome = crate::widgets::ConfirmCancel::None;
        let open = crate::widgets::confirm_window(ctx, title, pending_scale, |ui| {
            match &confirm {
                PendingConfirm::Delete { label } => {
                    ui.horizontal(|ui| {
                        if let Some(prog) = subject.program {
                            crate::icon_cache::paint_program_icon(ui, catalog, icons, prog);
                        }
                        ui.label(format!("Delete {label}? This cannot be undone."));
                    });
                }
                PendingConfirm::Overwrite { kind, name } => {
                    ui.label(format!(
                        "{kind} “{name}” already exists. Overwrite / rename onto it?"
                    ));
                }
                PendingConfirm::DeleteVariant { variant } => {
                    ui.label(format!(
                        "Delete icon variant “{}”? This cannot be undone.",
                        variant_display_label(variant)
                    ));
                }
                PendingConfirm::OverwriteVariant { variant, .. } => {
                    ui.label(format!(
                        "Variant “{}” already exists. Overwrite it?",
                        variant_display_label(variant)
                    ));
                }
                PendingConfirm::DeleteMaskImage => {
                    let name = subject.entity.unwrap_or_default();
                    ui.label(format!(
                        "Remove the mask image for “{name}”? Searches fall back to the shape settings. This cannot be undone."
                    ));
                }
            }
            outcome = crate::widgets::confirm_cancel_row(ui, confirm_label);
        });
        match outcome {
            crate::widgets::ConfirmCancel::Confirm => {
                self.confirm = None;
                Some(confirm)
            }
            crate::widgets::ConfirmCancel::Cancel => {
                self.confirm = None;
                None
            }
            crate::widgets::ConfirmCancel::None => {
                if !open {
                    self.confirm = None;
                }
                None
            }
        }
    }

    /// Draw the variant-name prompt; returns `(name, source)` on submit.
    pub(crate) fn draw_variant_name_prompt(
        &mut self,
        ctx: &egui::Context,
        pending_scale: Option<&ViewportScaleEvent>,
    ) -> Option<(String, PathBuf)> {
        let Some(VariantPrompt::Name { source }) = self.variant_prompt.clone() else {
            return None;
        };
        let mut submit = false;
        let mut cancel = false;
        let open = crate::widgets::confirm_window(ctx, "Add Icon Variant", pending_scale, |ui| {
            ui.label("Variant name");
            let name_w = crate::widgets::name_edit_width(ui, &self.variant_name_draft, "");
            ui.add(egui::TextEdit::singleline(&mut self.variant_name_draft).desired_width(name_w));
            match crate::widgets::confirm_cancel_row(ui, "Add") {
                crate::widgets::ConfirmCancel::Cancel => cancel = true,
                crate::widgets::ConfirmCancel::Confirm => submit = true,
                crate::widgets::ConfirmCancel::None => {}
            }
        });
        if !open || cancel || submit {
            let name = self.variant_name_draft.trim().to_string();
            self.variant_prompt = None;
            self.variant_name_draft.clear();
            return submit.then_some((name, source));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn any_open_tracks_each_prompt() {
        let mut p = EditorPrompts::default();
        assert!(!p.any_open());
        p.confirm = Some(PendingConfirm::DeleteMaskImage);
        assert!(p.any_open());
        p.confirm = None;
        p.ask_variant_name(PathBuf::from("a.png"));
        assert!(p.any_open());
        p.clear_entity_prompts();
        assert!(!p.any_open());
        p.open_overlay_icon_picker("btn".into());
        assert!(p.any_open());
        p.clear_entity_prompts();
        assert!(!p.any_open());
    }

    #[test]
    fn ask_variant_name_resets_draft() {
        let mut p = EditorPrompts {
            variant_name_draft: "stale".into(),
            ..Default::default()
        };
        p.ask_variant_name(PathBuf::from("b.png"));
        assert!(p.variant_name_draft.is_empty());
        assert_eq!(
            p.variant_prompt,
            Some(VariantPrompt::Name {
                source: PathBuf::from("b.png")
            })
        );
    }

    #[test]
    fn open_overlay_icon_picker_resets_search() {
        let mut p = EditorPrompts {
            overlay_icon_search: "old".into(),
            ..Default::default()
        };
        p.open_overlay_icon_picker("btn".into());
        assert!(p.overlay_icon_search.is_empty());
        assert_eq!(p.overlay_icon_picker_for.as_deref(), Some("btn"));
    }

    #[test]
    fn draw_confirm_paints_without_data_editor() {
        let cat = sqyre_persist::ProgramCatalog::default();
        let mut icons = IconCache::default();
        let mut prompts = EditorPrompts {
            confirm: Some(PendingConfirm::Delete {
                label: "program “Game”".into(),
            }),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        // First frames are egui's invisible sizing pass.
        for frame in 0..4 {
            let out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(frame as f64 * 0.05),
                    predicted_dt: 0.05,
                    ..Default::default()
                },
                |ui| {
                    let confirmed = prompts.draw_confirm(
                        ui.ctx(),
                        CatalogView::from(&cat),
                        &mut icons,
                        ConfirmSubject::default(),
                        None,
                    );
                    assert!(confirmed.is_none(), "no click yet");
                },
            );
            out.drop_without_applying_deltas();
        }
        assert!(
            prompts.confirm.is_some(),
            "confirm stays open until the user chooses"
        );
        assert!(prompts.any_open(), "confirm owns keyboard while open");
        let rect = ctx.memory(|m| m.area_rect(egui::Id::new(("sqyre_confirm", "Delete"))));
        assert!(
            rect.is_some_and(|r| r.width() > 1.0 && r.height() > 1.0),
            "confirm window area should exist after settling"
        );
    }

    #[test]
    fn draw_confirm_escape_dismisses() {
        let cat = sqyre_persist::ProgramCatalog::default();
        let mut icons = IconCache::default();
        let mut prompts = EditorPrompts {
            confirm: Some(PendingConfirm::Overwrite {
                kind: "Item",
                name: "Potion".into(),
            }),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        // egui's first frames are an invisible sizing pass.
        for frame in 0..3 {
            let out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(frame as f64 * 0.05),
                    predicted_dt: 0.05,
                    ..Default::default()
                },
                |ui| {
                    let _ = prompts.draw_confirm(
                        ui.ctx(),
                        CatalogView::from(&cat),
                        &mut icons,
                        ConfirmSubject::default(),
                        None,
                    );
                },
            );
            out.drop_without_applying_deltas();
        }
        assert!(prompts.confirm.is_some(), "still open after sizing");
        let mut esc = egui::RawInput {
            screen_rect: Some(screen),
            time: Some(0.2),
            predicted_dt: 0.05,
            ..Default::default()
        };
        esc.events.push(egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        let out = ctx.run_ui(esc, |ui| {
            let _ = prompts.draw_confirm(
                ui.ctx(),
                CatalogView::from(&cat),
                &mut icons,
                ConfirmSubject::default(),
                None,
            );
        });
        out.drop_without_applying_deltas();
        assert!(
            prompts.confirm.is_none(),
            "Esc should clear the pending confirm via confirm_cancel_row"
        );
        assert!(!prompts.any_open());
    }
}
