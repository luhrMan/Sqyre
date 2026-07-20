//! Cached egui textures for program-catalog item PNGs.

use crate::assets;
use crate::demo_icons;
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use sqyre_persist::ProgramCatalog;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const FALLBACK_KEY: &str = "__sqyre_fallback__";
/// Raster size for the brand fallback texture (displayed smaller in UI).
const FALLBACK_PX: u32 = 128;

#[derive(Default)]
pub struct IconCache {
    textures: HashMap<PathBuf, TextureHandle>,
    /// Remember targets that failed so we do not spam disk/read errors.
    missing: HashMap<String, ()>,
    fallback: Option<TextureHandle>,
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

    /// Drop a cached texture so the next load re-reads from disk / demo store.
    pub fn invalidate_path(&mut self, path: &Path) {
        self.textures.remove(path);
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

fn load_texture(ctx: &egui::Context, path: &Path) -> Option<TextureHandle> {
    if let Ok(bytes) = std::fs::read(path) {
        return load_png_bytes(ctx, &path.to_string_lossy(), &bytes);
    }
    let demo = demo_icons::get(path)?;
    let color = ColorImage::from_rgba_unmultiplied(
        [demo.width as usize, demo.height as usize],
        &demo.rgba,
    );
    Some(ctx.load_texture(path.to_string_lossy(), color, TextureOptions::LINEAR))
}

fn load_png_bytes(ctx: &egui::Context, name: &str, bytes: &[u8]) -> Option<TextureHandle> {
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color = ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    Some(ctx.load_texture(name.to_owned(), color, TextureOptions::LINEAR))
}
