//! Cached egui textures for program-catalog item PNGs.

use crate::assets::APP_ICON_PNG;
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use sqyre_persist::ProgramCatalog;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const FALLBACK_KEY: &str = "__sqyre_fallback__";

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
    pub fn for_target(
        &mut self,
        ctx: &egui::Context,
        catalog: &ProgramCatalog,
        target: &str,
    ) -> Option<TextureHandle> {
        if self.missing.contains_key(target) {
            return None;
        }
        let path = catalog.variant_paths(target).into_iter().next()?;
        match self.get_or_load(ctx, &path) {
            Some(t) => Some(t),
            None => {
                self.missing.insert(target.to_string(), ());
                None
            }
        }
    }

    /// Item icon, or the embedded Sqyre brand PNG when no variant exists.
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
        let tex = load_png_bytes(ctx, FALLBACK_KEY, APP_ICON_PNG)
            .expect("embedded Sqyre PNG must decode");
        self.fallback = Some(tex.clone());
        tex
    }

    /// Load an arbitrary image path into a retained texture.
    pub fn for_path(&mut self, ctx: &egui::Context, path: &Path) -> Option<TextureHandle> {
        if !path.is_file() {
            return None;
        }
        self.get_or_load(ctx, path)
    }

    /// Drop a cached texture so the next load re-reads from disk.
    pub fn invalidate_path(&mut self, path: &Path) {
        self.textures.remove(path);
    }

    fn get_or_load(&mut self, ctx: &egui::Context, path: &Path) -> Option<TextureHandle> {
        if let Some(t) = self.textures.get(path) {
            return Some(t.clone());
        }
        let tex = load_png_file(ctx, path)?;
        self.textures.insert(path.to_path_buf(), tex.clone());
        Some(tex)
    }
}

fn load_png_file(ctx: &egui::Context, path: &Path) -> Option<TextureHandle> {
    let bytes = std::fs::read(path).ok()?;
    load_png_bytes(ctx, &path.to_string_lossy(), &bytes)
}

fn load_png_bytes(ctx: &egui::Context, name: &str, bytes: &[u8]) -> Option<TextureHandle> {
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color = ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    Some(ctx.load_texture(name.to_owned(), color, TextureOptions::LINEAR))
}
