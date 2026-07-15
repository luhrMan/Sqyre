//! Per-action Logs window: text detail, shared pipeline images, and clickable
//! image-search item cards with per-item processing / find steps.

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use sqyre_domain::ActionId;
use sqyre_executor::{ActionLogEntry, LogImage, SharedActionLog};
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
    textures: HashMap<TexKey, TextureHandle>,
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
        if let Some(t) = self.textures.get(&key) {
            return Some(t.clone());
        }
        let size = [image.width as usize, image.height as usize];
        if image.pixels.len() != size[0] * size[1] * 4 {
            return None;
        }
        let color = ColorImage::from_rgba_unmultiplied(size, &image.pixels);
        let name = format!(
            "action-log-{}-{:?}-{}",
            action_id.as_str(),
            key,
            image.label
        );
        let tex = ctx.load_texture(name, color, TextureOptions::NEAREST);
        self.textures.insert(key, tex.clone());
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
) -> bool {
    image_cache.ensure_action(action_id);
    let entries = action_log.entries_for(action_id);
    let copy_text = action_log.lines_for(action_id).join("\n");
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
    egui::Window::new(title)
        .open(&mut open)
        .default_size([640.0, 560.0])
        .min_width(420.0)
        .min_height(280.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Copy text").clicked() {
                    ui.ctx().copy_text(copy_text.clone());
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

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
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
    ui.label(
        egui::RichText::new("Items — click an image to inspect processing & finds")
            .strong()
            .size(13.0),
    );
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        for &(i, entry) in pending.iter() {
            let ActionLogEntry::ItemPipeline {
                title,
                summary,
                thumbnail,
                ..
            } = entry
            else {
                continue;
            };
            show_item_card(ui, action_id, image_cache, i, title, summary, thumbnail);
        }
    });
    pending.clear();
    ui.add_space(8.0);
}

fn show_item_card(
    ui: &mut egui::Ui,
    action_id: ActionId,
    image_cache: &mut LogsImageCache,
    entry_index: usize,
    title: &str,
    summary: &str,
    thumbnail: &LogImage,
) {
    const CARD_W: f32 = 140.0;
    let selected = image_cache.selected_item == Some(entry_index);
    let frame = egui::Frame::group(ui.style()).stroke(if selected {
        egui::Stroke::new(2.0, egui::Color32::from_rgb(70, 140, 220))
    } else {
        ui.visuals().widgets.noninteractive.bg_stroke
    });
    // Nested vertical: horizontal_wrapped uses LTR layout, which would otherwise
    // lay out title/image/button side-by-side and produce a diagonal staircase.
    ui.vertical(|ui| {
        ui.set_width(CARD_W);
        frame.show(ui, |ui| {
            ui.set_max_width(CARD_W);
            ui.label(egui::RichText::new(title).strong().size(12.0));
            ui.label(egui::RichText::new(summary).weak().size(11.0));
            if let Some(tex) =
                image_cache.texture(ui.ctx(), action_id, TexKey::Thumb(entry_index), thumbnail)
            {
                let [tw, th] = tex.size();
                let size = fit_thumb(tw as f32, th as f32, 112.0);
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
        ui.label(egui::RichText::new(title).strong().size(15.0));
        ui.label(egui::RichText::new(summary).weak());
    });
    ui.separator();

    if !details.is_empty() {
        ui.label(egui::RichText::new("Details").strong().size(13.0));
        for line in details {
            ui.monospace(line);
        }
        ui.add_space(8.0);
    }

    ui.label(
        egui::RichText::new("Processing & find steps (chronological)")
            .strong()
            .size(13.0),
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
        ui.label(egui::RichText::new(&image.label).strong().size(12.5));
        ui.label(
            egui::RichText::new(format!("{}×{}", image.width, image.height))
                .weak()
                .size(11.0),
        );
        if let Some(tex) = image_cache.texture(ui.ctx(), action_id, key, image) {
            let [tw, th] = tex.size();
            let size = fit_width(tw as f32, th as f32, avail_w - 16.0);
            ui.add(egui::Image::new((tex.id(), size)));
        } else {
            ui.colored_label(
                egui::Color32::from_rgb(200, 80, 80),
                "(image unavailable)",
            );
        }
    });
}

fn fit_width(w: f32, h: f32, max_w: f32) -> egui::Vec2 {
    if w <= 0.0 || h <= 0.0 {
        return egui::vec2(64.0, 64.0);
    }
    let scale = (max_w / w).min(1.0).max(0.05);
    let scale = if w < 96.0 { (96.0 / w).min(4.0) } else { scale };
    egui::vec2(w * scale, h * scale)
}

fn fit_thumb(w: f32, h: f32, max_edge: f32) -> egui::Vec2 {
    if w <= 0.0 || h <= 0.0 {
        return egui::vec2(64.0, 64.0);
    }
    let scale = (max_edge / w.max(h)).min(4.0).max(0.05);
    egui::vec2(w * scale, h * scale)
}
