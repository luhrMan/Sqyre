//! Per-action Logs window: text detail, shared pipeline images, and clickable
//! image-search item cards with per-item processing / find steps.

use crate::image_view;
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use sqyre_domain::ActionId;
use sqyre_ports::{lines_for, ActionLogEntry, LogImage, SharedActionLog};
use std::collections::HashMap;

/// Texture key for a log image (entry index + optional step within an item pipeline).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum TexKey {
    Entry(usize),
    Step { entry: usize, step: usize },
    Thumb(usize),
}

/// Retained egui textures + selected item for the open logs window.
#[derive(Default)]
pub struct LogsImageCache {
    action: Option<ActionId>,
    textures: HashMap<TexKey, (usize, TextureHandle)>,
    /// Entry index of the selected [`ActionLogEntry::ItemPipeline`], if any.
    pub selected_item: Option<usize>,
}

impl LogsImageCache {
    pub fn clear(&mut self) {
        self.action = None;
        self.textures.clear();
        self.selected_item = None;
    }

    fn ensure_action(&mut self, action_id: ActionId) {
        if self.action != Some(action_id) {
            self.action = Some(action_id);
            self.textures.clear();
            self.selected_item = None;
        }
    }

    fn texture(
        &mut self,
        ctx: &egui::Context,
        action_id: ActionId,
        key: TexKey,
        image: &LogImage,
    ) -> Option<TextureHandle> {
        self.ensure_action(action_id);
        let ptr = std::sync::Arc::as_ptr(&image.pixels) as usize;
        let size = [image.width as usize, image.height as usize];
        if image.pixels.len() != size[0] * size[1] * 4 {
            return None;
        }
        if let Some((old_ptr, tex)) = self.textures.get_mut(&key) {
            if *old_ptr != ptr {
                let color = ColorImage::from_rgba_unmultiplied(size, &image.pixels);
                tex.set(color, TextureOptions::NEAREST);
                *old_ptr = ptr;
            }
            return Some(tex.clone());
        }
        let color = ColorImage::from_rgba_unmultiplied(size, &image.pixels);
        let name = format!(
            "action-log-{}-{:?}-{}",
            action_id.as_str(),
            key,
            image.label
        );
        let tex = ctx.load_texture(name, color, TextureOptions::NEAREST);
        self.textures.insert(key, (ptr, tex.clone()));
        Some(tex)
    }
}

/// Draw the floating Logs window for `action_id`. Returns `true` when the window should close.
pub fn show_logs_window(
    ctx: &egui::Context,
    action_id: ActionId,
    title: &str,
    action_log: &SharedActionLog,
    image_cache: &mut LogsImageCache,
    pending_scale: Option<&crate::widgets::ViewportScaleEvent>,
) -> bool {
    image_cache.ensure_action(action_id);
    let entries = action_log.entries_for(action_id);
    let text_count = entries.iter().filter(|e| e.as_text().is_some()).count();
    let image_count = entries.iter().filter(|e| e.is_image()).count();
    let item_count = entries.iter().filter(|e| e.is_item_pipeline()).count();

    // Drop selection if the entry disappeared (e.g. log rotated).
    if let Some(sel) = image_cache.selected_item {
        if !matches!(entries.get(sel), Some(ActionLogEntry::ItemPipeline { .. })) {
            image_cache.selected_item = None;
        }
    }

    let mut open = true;
    let mut close_clicked = false;
    let id = egui::Id::new(("sqyre_logs", action_id.as_str()));
    crate::widgets::fit_dialog_window(
        egui::Window::new(title)
            .open(&mut open)
            .default_size([640.0, 560.0])
            .min_width(420.0)
            .min_height(280.0),
        ctx,
        id,
        pending_scale,
    )
    .show(ctx, |ui| {
        ui.horizontal(|ui| {
            if ui.button("Copy text").clicked() {
                ui.ctx().copy_text(lines_for(&entries).join("\n"));
            }
            if ui.button("Clear logs").clicked() {
                action_log.clear();
                image_cache.clear();
            }
            ui.label(format!(
                "{text_count} line(s) · {image_count} image(s) · {item_count} item(s)"
            ));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    close_clicked = true;
                }
            });
        });
        ui.separator();

        let list_h = crate::pickers::popup_scroll_max_height(ui, 0.0);
        let list_w = crate::widgets::visible_width(ui);
        crate::pickers::dialog_scroll(list_w, list_h).show(ui, |ui| {
            ui.set_max_width(list_w);
            if entries.is_empty() {
                ui.label("No logs yet — run the macro.");
                return;
            }

                // Detail view for a selected image-search item.
                if let Some(sel) = image_cache.selected_item {
                    if let Some(ActionLogEntry::ItemPipeline {
                        title,
                        summary,
                        steps,
                        details,
                        ..
                    }) = entries.get(sel)
                    {
                        show_item_detail(
                            ui,
                            action_id,
                            image_cache,
                            sel,
                            title,
                            summary,
                            details,
                            steps,
                        );
                        return;
                    }
                }

                let avail_w = ui.available_width().max(120.0);
                let mut pending_items: Vec<(usize, &ActionLogEntry)> = Vec::new();

                for (i, entry) in entries.iter().enumerate() {
                    match entry {
                        ActionLogEntry::Text(line) => {
                            ui.monospace(line);
                        }
                        ActionLogEntry::Image(img) => {
                            show_labeled_image(
                                ui,
                                action_id,
                                image_cache,
                                TexKey::Entry(i),
                                img,
                                avail_w,
                            );
                        }
                        ActionLogEntry::ItemPipeline { .. } => {
                            pending_items.push((i, entry));
                        }
                    }
                }
                flush_item_gallery(ui, action_id, image_cache, &mut pending_items);
            });
    });

    if !open || close_clicked {
        image_cache.clear();
        true
    } else {
        false
    }
}

const CARD_MIN: f32 = 112.0;
const CARD_MAX: f32 = 180.0;
const CARD_GAP: f32 = 8.0;
/// Frame padding so the thumb stays inside the card width.
const CARD_THUMB_INSET: f32 = 28.0;

/// Even-row gallery: more columns as the window widens; cards stretch between
/// [`CARD_MIN`] and [`CARD_MAX`]. Uses [`crate::widgets::visible_width`] so the
/// row count cannot ratchet the Logs window toward `max_size`.
fn item_gallery_metrics(count: usize, avail_w: f32) -> (f32, usize) {
    if count == 0 {
        return (CARD_MAX, 1);
    }
    let avail = avail_w.max(CARD_MIN);
    let fit = |cell: f32| {
        crate::pickers::grid_column_count_for_width(avail, cell, CARD_GAP)
            .min(count)
            .max(1)
    };
    let cols = fit(CARD_MIN);
    let stretched = (avail - CARD_GAP * (cols as f32 - 1.0)) / cols as f32;
    if stretched > CARD_MAX {
        (CARD_MAX, fit(CARD_MAX))
    } else {
        (stretched.max(CARD_MIN), cols)
    }
}

fn flush_item_gallery(
    ui: &mut egui::Ui,
    action_id: ActionId,
    image_cache: &mut LogsImageCache,
    pending: &mut Vec<(usize, &ActionLogEntry)>,
) {
    if pending.is_empty() {
        return;
    }
    ui.add_space(10.0);
    crate::widgets::title_with_count(
        ui,
        egui::RichText::new("Items — click an image to inspect processing & finds")
            .strong()
            .small(),
        pending.len(),
    );
    ui.add_space(4.0);

    let avail = crate::widgets::visible_content_width(ui);
    let (card_w, cols) = item_gallery_metrics(pending.len(), avail);
    ui.scope(|ui| {
        ui.set_max_width(avail);
        let old_spacing = ui.spacing().item_spacing;
        ui.spacing_mut().item_spacing = egui::vec2(CARD_GAP, CARD_GAP);

        let mut i = 0;
        while i < pending.len() {
            ui.horizontal(|ui| {
                ui.set_max_width(avail);
                ui.spacing_mut().item_spacing.x = CARD_GAP;
                let end = (i + cols).min(pending.len());
                for &(idx, entry) in &pending[i..end] {
                    let ActionLogEntry::ItemPipeline {
                        title,
                        summary,
                        thumbnail,
                        ..
                    } = entry
                    else {
                        continue;
                    };
                    show_item_card(
                        ui,
                        action_id,
                        image_cache,
                        idx,
                        title,
                        summary,
                        thumbnail,
                        card_w,
                    );
                }
            });
            i += cols;
        }

        ui.spacing_mut().item_spacing = old_spacing;
    });
    pending.clear();
    ui.add_space(8.0);
}

#[allow(clippy::too_many_arguments)] // log card/detail: cache, entry identity, captions, and image payloads
fn show_item_card(
    ui: &mut egui::Ui,
    action_id: ActionId,
    image_cache: &mut LogsImageCache,
    entry_index: usize,
    title: &str,
    summary: &str,
    thumbnail: &LogImage,
    card_w: f32,
) {
    let selected = image_cache.selected_item == Some(entry_index);
    let frame = egui::Frame::group(ui.style()).stroke(if selected {
        egui::Stroke::new(2.0, egui::Color32::from_rgb(70, 140, 220))
    } else {
        ui.visuals().widgets.noninteractive.bg_stroke
    });
    let thumb_edge = (card_w - CARD_THUMB_INSET).max(64.0);
    ui.vertical(|ui| {
        ui.set_width(card_w);
        frame.show(ui, |ui| {
            ui.set_max_width(card_w);
            ui.add(egui::Label::new(egui::RichText::new(title).strong().small()).truncate())
                .on_hover_text(title);
            ui.add(egui::Label::new(egui::RichText::new(summary).weak().small()).truncate())
                .on_hover_text(summary);
            if let Some(tex) =
                image_cache.texture(ui.ctx(), action_id, TexKey::Thumb(entry_index), thumbnail)
            {
                let [tw, th] = tex.size();
                let size = fit_thumb(tw as f32, th as f32, thumb_edge);
                let resp = ui.add(
                    egui::Image::new((tex.id(), size))
                        .sense(egui::Sense::click())
                        .corner_radius(4.0),
                );
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    image_cache.selected_item = Some(entry_index);
                }
                resp.on_hover_text("Click to view processing steps and where it was found");
            }
            if ui
                .small_button(if selected { "Open ▸" } else { "Inspect ▸" })
                .clicked()
            {
                image_cache.selected_item = Some(entry_index);
            }
        });
    });
}

#[allow(clippy::too_many_arguments)] // log card/detail: cache, entry identity, captions, and image payloads
fn show_item_detail(
    ui: &mut egui::Ui,
    action_id: ActionId,
    image_cache: &mut LogsImageCache,
    entry_index: usize,
    title: &str,
    summary: &str,
    details: &[String],
    steps: &[LogImage],
) {
    ui.horizontal(|ui| {
        if ui.button("← Back to items").clicked() {
            image_cache.selected_item = None;
        }
        ui.label(egui::RichText::new(title).strong().heading());
        ui.label(egui::RichText::new(summary).weak());
    });
    ui.separator();

    if !details.is_empty() {
        ui.label(egui::RichText::new("Details").strong().small());
        for line in details {
            ui.monospace(line);
        }
        ui.add_space(8.0);
    }

    ui.label(
        egui::RichText::new("Processing & find steps (chronological)")
            .strong()
            .small(),
    );
    ui.add_space(4.0);
    let avail_w = ui.available_width().max(120.0);
    for (si, step) in steps.iter().enumerate() {
        show_labeled_image(
            ui,
            action_id,
            image_cache,
            TexKey::Step {
                entry: entry_index,
                step: si,
            },
            step,
            avail_w,
        );
    }
}

fn show_labeled_image(
    ui: &mut egui::Ui,
    action_id: ActionId,
    image_cache: &mut LogsImageCache,
    key: TexKey,
    image: &LogImage,
    avail_w: f32,
) {
    ui.add_space(8.0);
    ui.group(|ui| {
        ui.label(egui::RichText::new(&image.label).strong().small());
        ui.label(
            egui::RichText::new(format!("{}×{}", image.width, image.height))
                .weak()
                .small(),
        );
        if let Some(tex) = image_cache.texture(ui.ctx(), action_id, key, image) {
            let [tw, th] = tex.size();
            let size = fit_width(tw as f32, th as f32, avail_w - 16.0);
            ui.add(egui::Image::new((tex.id(), size)));
        } else {
            ui.colored_label(egui::Color32::from_rgb(200, 80, 80), "(image unavailable)");
        }
    });
}

fn fit_width(w: f32, h: f32, max_w: f32) -> egui::Vec2 {
    if w <= 0.0 || h <= 0.0 {
        return egui::vec2(64.0, 64.0);
    }
    let scale = (max_w / w).clamp(0.05, 1.0);
    let scale = if w < 96.0 { (96.0 / w).min(4.0) } else { scale };
    egui::vec2(w * scale, h * scale)
}

fn fit_thumb(w: f32, h: f32, max_edge: f32) -> egui::Vec2 {
    if w <= 0.0 || h <= 0.0 {
        return egui::vec2(64.0, 64.0);
    }
    let scale = image_view::fit_scale(w, h, max_edge, max_edge).clamp(0.05, 4.0);
    egui::vec2(w * scale, h * scale)
}

#[cfg(test)]
mod tests {
    use super::{item_gallery_metrics, CARD_GAP, CARD_MAX, CARD_MIN};

    #[test]
    fn few_items_stay_at_max_card_width() {
        let (w, cols) = item_gallery_metrics(2, 640.0);
        assert!((w - CARD_MAX).abs() < 0.01);
        assert_eq!(cols, 2);
    }

    #[test]
    fn many_items_fill_row_and_wrap() {
        let avail = 640.0;
        let (w, cols) = item_gallery_metrics(20, avail);
        let max_cols = crate::pickers::grid_column_count_for_width(avail, CARD_MIN, CARD_GAP);
        assert_eq!(cols, max_cols);
        let expected = (avail - CARD_GAP * (cols as f32 - 1.0)) / cols as f32;
        assert!((w - expected.clamp(CARD_MIN, CARD_MAX)).abs() < 0.01);
    }

    #[test]
    fn wider_window_adds_columns() {
        let (_, narrow) = item_gallery_metrics(20, 420.0);
        let (_, wide) = item_gallery_metrics(20, 900.0);
        assert!(wide > narrow);
    }
}
