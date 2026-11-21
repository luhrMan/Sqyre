//! Data Editor confirm and variant-name dialogs.

use super::{DataEditor, DataEditorCtx};
use crate::data_editor_preview::variant_display_label;
use crate::icon_cache::IconCache;
use crate::preview_tooltip::PreviewTooltipCache;
use eframe::egui;
use sqyre_persist::{CatalogView, ProgramCatalog, UserSettings};
use std::path::PathBuf;

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub(crate) enum VariantPrompt {
    /// Ask for a name before adding a non-first variant.
    Name { source: PathBuf },
}

impl DataEditor {
    pub(super) fn draw_confirm(
        &mut self,
        env: &mut DataEditorCtx<'_>,
        previews: &mut PreviewTooltipCache,
    ) {
        let Some(confirm) = self.confirm.clone() else {
            return;
        };
        let (title, confirm_label) = match &confirm {
            PendingConfirm::Delete { .. } | PendingConfirm::DeleteVariant { .. } => {
                ("Delete", "Delete")
            }
            PendingConfirm::DeleteMaskImage => ("Remove Image", "Remove"),
            PendingConfirm::Overwrite { .. } | PendingConfirm::OverwriteVariant { .. } => {
                ("Overwrite", "Overwrite")
            }
        };
        let ctx = env.ctx;
        let open = crate::widgets::confirm_window(ctx, title, env.pending_scale, |ui| {
            match &confirm {
                PendingConfirm::Delete { label } => {
                    ui.horizontal(|ui| {
                        if let Some(prog) = self.selected_program.as_deref() {
                            crate::icon_cache::paint_program_icon(
                                ui,
                                CatalogView::new(env.catalog),
                                env.icons,
                                prog,
                            );
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
                    let name = self.selected_entity.as_deref().unwrap_or_default();
                    ui.label(format!(
                        "Remove the mask image for “{name}”? Searches fall back to the shape settings. This cannot be undone."
                    ));
                }
            }
            match crate::widgets::confirm_cancel_row(ui, confirm_label) {
                crate::widgets::ConfirmCancel::Cancel => {
                    self.confirm = None;
                }
                crate::widgets::ConfirmCancel::Confirm => match confirm {
                    PendingConfirm::Delete { .. } => {
                        self.confirm = None;
                        self.on_delete(env, previews);
                    }
                    PendingConfirm::Overwrite { .. } => {
                        self.confirm = None;
                        self.apply_update(env, true, previews);
                    }
                    PendingConfirm::DeleteVariant { variant } => {
                        self.confirm = None;
                        self.delete_icon_variant(
                            CatalogView::new(env.catalog),
                            env.icons,
                            env.settings,
                            &variant,
                        );
                    }
                    PendingConfirm::OverwriteVariant { variant, source } => {
                        self.confirm = None;
                        self.overwrite_icon_variant(
                            CatalogView::new(env.catalog),
                            env.icons,
                            &variant,
                            &source,
                        );
                    }
                    PendingConfirm::DeleteMaskImage => {
                        self.confirm = None;
                        self.remove_mask_image(CatalogView::new(env.catalog), env.icons);
                    }
                },
                crate::widgets::ConfirmCancel::None => {}
            }
        });
        if !open {
            self.confirm = None;
        }
    }

    pub(super) fn draw_variant_name_prompt(
        &mut self,
        ctx: &egui::Context,
        catalog: &ProgramCatalog,
        icons: &mut IconCache,
        settings: &UserSettings,
        pending_scale: Option<&crate::widgets::ViewportScaleEvent>,
    ) {
        let Some(VariantPrompt::Name { source }) = self.variant_prompt.clone() else {
            return;
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
        if !open || cancel {
            self.variant_prompt = None;
            self.variant_name_draft.clear();
            return;
        }
        if submit {
            let name = self.variant_name_draft.trim().to_string();
            self.variant_prompt = None;
            self.variant_name_draft.clear();
            self.add_icon_variant(CatalogView::from(catalog), icons, settings, &name, &source);
        }
    }
}
