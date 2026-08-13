//! Cached egui textures for program-catalog item PNGs and OS process icons.

use crate::assets;
use crate::demo_icons;
use crate::window_types::ProcessIcon;
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions, Vec2};
use sqyre_domain::PROGRAM_DELIMITER;
use sqyre_persist::ProgramCatalog;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const FALLBACK_KEY: &str = "__sqyre_fallback__";
/// Raster size for the brand fallback texture (displayed smaller in UI).
const FALLBACK_PX: u32 = 128;
/// Display size for OS process icons in lists / forms.
pub const PROCESS_ICON_SIDE: f32 = 16.0;

#[derive(Default)]
pub struct IconCache {
    textures: HashMap<PathBuf, TextureHandle>,
    /// Remember targets that failed so we do not spam disk/read errors.
    missing: HashMap<String, ()>,
    fallback: Option<TextureHandle>,
    /// OS process icons keyed by process path.
    process: HashMap<String, TextureHandle>,
    process_missing: HashMap<String, ()>,
}

impl IconCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// First variant PNG for `program~item`, loaded into a retained texture.
    ///
    /// Falls back to in-memory [`demo_icons`] placeholders when no file exists
    /// (WASM demo seed).
    pub fn for_target(
        &mut self,
        ctx: &egui::Context,
        catalog: &ProgramCatalog,
        target: &str,
    ) -> Option<TextureHandle> {
        if self.missing.contains_key(target) {
            return None;
        }
        let path = demo_icons::merged_variant_paths(catalog, target)
            .into_iter()
            .next();
        let Some(path) = path else {
            self.missing.insert(target.to_string(), ());
            return None;
        };
        match self.get_or_load(ctx, &path) {
            Some(t) => Some(t),
            None => {
                self.missing.insert(target.to_string(), ());
                None
            }
        }
    }

    /// Item icon, or the embedded Sqyre brand SVG (rasterized) when no variant exists.
    pub fn for_target_or_fallback(
        &mut self,
        ctx: &egui::Context,
        catalog: &ProgramCatalog,
        target: &str,
    ) -> TextureHandle {
        self.for_target(ctx, catalog, target)
            .unwrap_or_else(|| self.sqyre_fallback(ctx))
    }

    pub fn sqyre_fallback(&mut self, ctx: &egui::Context) -> TextureHandle {
        if let Some(t) = &self.fallback {
            return t.clone();
        }
        let (rgba, w, h) =
            assets::app_icon_rgba(FALLBACK_PX).expect("embedded Sqyre SVG must rasterize");
        let color = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
        let tex = ctx.load_texture(FALLBACK_KEY, color, TextureOptions::LINEAR);
        self.fallback = Some(tex.clone());
        tex
    }

    /// Load an arbitrary image path into a retained texture.
    /// Also resolves in-memory [`demo_icons`] when the path is not on disk.
    pub fn for_path(&mut self, ctx: &egui::Context, path: &Path) -> Option<TextureHandle> {
        if path.is_file() || demo_icons::contains(path) {
            return self.get_or_load(ctx, path);
        }
        None
    }

    /// OS process icon for a bound executable path (and optional window title).
    ///
    /// Uses a previously seeded texture when the window picker already supplied
    /// [`ProcessIcon`] bytes; otherwise asks [`sqyre_capture::process_icon`].
    pub fn for_process(
        &mut self,
        ctx: &egui::Context,
        process_path: &str,
        window_title: &str,
    ) -> Option<TextureHandle> {
        let key = process_cache_key(process_path);
        if key.is_empty() {
            return None;
        }
        if let Some(t) = self.process.get(&key) {
            return Some(t.clone());
        }
        if self.process_missing.contains_key(&key) {
            return None;
        }
        #[cfg(feature = "native-runtime")]
        {
            let Some(icon) =
                sqyre_capture::process_icon(process_path, window_title).map(|i| ProcessIcon {
                    width: i.width,
                    height: i.height,
                    rgba: i.rgba,
                })
            else {
                self.process_missing.insert(key, ());
                return None;
            };
            Some(self.insert_process_icon(ctx, &key, &icon))
        }
        #[cfg(not(feature = "native-runtime"))]
        {
            self.process_missing.insert(key, ());
            None
        }
    }

    /// OS process icon for a catalog program (via its bound `process_path`).
    pub fn for_program(
        &mut self,
        ctx: &egui::Context,
        catalog: &ProgramCatalog,
        program: &str,
    ) -> Option<TextureHandle> {
        let prog = catalog.get(program.trim())?;
        if prog.process_path.trim().is_empty() {
            return None;
        }
        self.for_process(ctx, &prog.process_path, &prog.window_title)
    }

    /// Seed / refresh the process-icon cache from a listed window's icon bytes.
    pub fn seed_process_icon(
        &mut self,
        ctx: &egui::Context,
        process_path: &str,
        icon: &ProcessIcon,
    ) -> Option<TextureHandle> {
        let key = process_cache_key(process_path);
        if key.is_empty() {
            return None;
        }
        self.process_missing.remove(&key);
        Some(self.insert_process_icon(ctx, &key, icon))
    }

    /// Cached process icon only (no OS fetch).
    pub fn cached_process(&self, process_path: &str) -> Option<TextureHandle> {
        let key = process_cache_key(process_path);
        if key.is_empty() {
            return None;
        }
        self.process.get(&key).cloned()
    }

    /// Drop a cached texture so the next load re-reads from disk / demo store.
    pub fn invalidate_path(&mut self, path: &Path) {
        self.textures.remove(path);
    }

    /// Forget a sticky miss for `program~item` so listings recheck disk.
    ///
    /// Call this after adding, overwriting, or deleting icon variants. Path-only
    /// invalidation is not enough: [`for_target`] remembers empty results and
    /// would keep showing the fallback until restart.
    pub fn invalidate_target(&mut self, target: &str) {
        self.missing.remove(target);
    }

    fn insert_process_icon(
        &mut self,
        ctx: &egui::Context,
        key: &str,
        icon: &ProcessIcon,
    ) -> TextureHandle {
        let size = [icon.width as usize, icon.height as usize];
        let color = ColorImage::from_rgba_unmultiplied(size, &icon.rgba);
        let name = format!("process_icon:{key}");
        let tex = ctx.load_texture(name, color, TextureOptions::LINEAR);
        self.process.insert(key.to_string(), tex.clone());
        tex
    }

    fn get_or_load(&mut self, ctx: &egui::Context, path: &Path) -> Option<TextureHandle> {
        if let Some(t) = self.textures.get(path) {
            return Some(t.clone());
        }
        let tex = load_texture(ctx, path)?;
        self.textures.insert(path.to_path_buf(), tex.clone());
        Some(tex)
    }
}

/// Draw a square process icon (non-interactive).
pub fn paint_process_icon(ui: &mut egui::Ui, tex: &TextureHandle, side: f32) {
    let side = side.max(1.0);
    ui.add(
        egui::Image::new((tex.id(), Vec2::splat(side)))
            .fit_to_exact_size(Vec2::splat(side))
            .maintain_aspect_ratio(true)
            .sense(egui::Sense::hover()),
    );
}

/// How to render a catalog program name next to its OS icon.
#[derive(Debug, Clone, Copy)]
pub enum ProgramLabelStyle {
    /// Plain selectable list row.
    Selectable { selected: bool },
    /// Strong 16px header with leading child count in parentheses.
    /// `selected: None` → non-interactive label.
    Header {
        selected: Option<bool>,
        child_count: usize,
    },
}

/// Paint optional process icon + program name as one selectable (or label) widget.
///
/// When `compact` is true and the program has a process icon, header styles omit the
/// name and show only `({child_count})` (full name on hover).
pub fn paint_program_label(
    ui: &mut egui::Ui,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    program: &str,
    style: ProgramLabelStyle,
    compact: bool,
) -> egui::Response {
    let tex = icons.for_program(ui.ctx(), catalog, program);
    let hide_name = compact && tex.is_some();

    let (selected, text, interactive) = match style {
        ProgramLabelStyle::Selectable { selected } => {
            // Flat program rows have no child count; always keep the name.
            (selected, program.to_string(), true)
        }
        ProgramLabelStyle::Header {
            selected: Some(selected),
            child_count,
        } => {
            let text = if hide_name {
                format!("({child_count})")
            } else {
                format!("({child_count}) {program}")
            };
            (selected, text, true)
        }
        ProgramLabelStyle::Header {
            selected: None,
            child_count,
        } => {
            let text = if hide_name {
                format!("({child_count})")
            } else {
                format!("({child_count}) {program}")
            };
            (false, text, false)
        }
    };

    let rich = match style {
        ProgramLabelStyle::Selectable { .. } => egui::RichText::new(text),
        ProgramLabelStyle::Header { .. } => egui::RichText::new(text).strong(),
    };

    let icon = tex.as_ref().map(|tex| {
        egui::Image::new((tex.id(), Vec2::splat(PROCESS_ICON_SIDE)))
            .fit_to_exact_size(Vec2::splat(PROCESS_ICON_SIDE))
            .maintain_aspect_ratio(true)
    });

    let response = match (icon, interactive) {
        (Some(icon), true) => ui.selectable_label(selected, (icon, rich)),
        (None, true) => ui.selectable_label(selected, rich),
        (Some(icon), false) => {
            ui.horizontal(|ui| {
                ui.add(icon);
                ui.label(rich);
            })
            .response
        }
        (None, false) => ui.label(rich),
    };

    if hide_name {
        response.on_hover_text(program)
    } else {
        response
    }
}

/// Paint a program's process icon when bound; returns whether an icon was drawn.
pub fn paint_program_icon(
    ui: &mut egui::Ui,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    program: &str,
) -> bool {
    let Some(tex) = icons.for_program(ui.ctx(), catalog, program) else {
        return false;
    };
    paint_process_icon(ui, &tex, PROCESS_ICON_SIDE);
    true
}

/// Leading OS icon for a catalog program name or `program~…` ref string.
pub fn paint_leading_program_icon(
    ui: &mut egui::Ui,
    catalog: &ProgramCatalog,
    icons: &mut IconCache,
    text: &str,
) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }
    let prog = text
        .split_once(PROGRAM_DELIMITER)
        .map(|(p, _)| p)
        .unwrap_or(text);
    if catalog.get(prog).is_none() {
        return false;
    }
    paint_program_icon(ui, catalog, icons, prog)
}

/// Leading OS icon for a Focus Window process path (+ optional title hint).
pub fn paint_leading_process_icon(
    ui: &mut egui::Ui,
    icons: &mut IconCache,
    process_path: &str,
    window_title: &str,
) -> bool {
    let Some(tex) = icons.for_process(ui.ctx(), process_path, window_title) else {
        return false;
    };
    paint_process_icon(ui, &tex, PROCESS_ICON_SIDE);
    true
}

fn process_cache_key(process_path: &str) -> String {
    let path = process_path.trim();
    if path.is_empty() {
        return String::new();
    }
    #[cfg(windows)]
    {
        path.to_lowercase()
    }
    #[cfg(not(windows))]
    {
        path.to_string()
    }
}

fn load_texture(ctx: &egui::Context, path: &Path) -> Option<TextureHandle> {
    if let Ok(bytes) = std::fs::read(path) {
        return load_png_bytes(ctx, &path.to_string_lossy(), &bytes);
    }
    let demo = demo_icons::get(path)?;
    let color =
        ColorImage::from_rgba_unmultiplied([demo.width as usize, demo.height as usize], &demo.rgba);
    Some(ctx.load_texture(path.to_string_lossy(), color, TextureOptions::LINEAR))
}

fn load_png_bytes(ctx: &egui::Context, name: &str, bytes: &[u8]) -> Option<TextureHandle> {
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color = ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    Some(ctx.load_texture(name.to_owned(), color, TextureOptions::LINEAR))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalidate_target_clears_sticky_miss() {
        let mut cache = IconCache::new();
        cache.missing.insert("Prog~Item".into(), ());
        cache.invalidate_target("Prog~Item");
        assert!(!cache.missing.contains_key("Prog~Item"));
    }

    #[test]
    fn invalidate_path_does_not_clear_sticky_miss() {
        let mut cache = IconCache::new();
        let path = PathBuf::from("/tmp/Prog/Item~Original.png");
        cache.missing.insert("Prog~Item".into(), ());
        cache.invalidate_path(&path);
        assert!(
            cache.missing.contains_key("Prog~Item"),
            "path invalidation alone must not imply a target recheck"
        );
    }
}
