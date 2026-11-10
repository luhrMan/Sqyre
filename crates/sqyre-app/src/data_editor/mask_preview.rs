//! Live mask scrim overlay for the Masks data-editor items grid.
//!
//! Transparent pixels = kept for matching; [`crate::theme::preview_scrim`] =
//! excluded. Built from the current (unsaved) form state so geometry edits
//! update the preview immediately.

use eframe::egui::{self, Color32, ColorImage, TextureHandle};
use std::path::{Path, PathBuf};

/// Which program items the Masks form preview grid shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum MaskItemsFilter {
    #[default]
    AllItems,
    UsingThisMask,
}

impl MaskItemsFilter {
    pub(crate) const ALL: [Self; 2] = [Self::AllItems, Self::UsingThisMask];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::AllItems => "All items",
            Self::UsingThisMask => "Using this mask",
        }
    }
}

/// Shape geometry used when no mask PNG is on disk.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MaskShapeParams {
    pub circle: bool,
    pub inverse: bool,
    /// Horizontal center as percent of width (0–100).
    pub center_x: f32,
    /// Vertical center as percent of height (0–100).
    pub center_y: f32,
    /// Rectangle width percent (ignored for circle).
    pub base: f32,
    /// Rectangle height percent (ignored for circle).
    pub height: f32,
    /// Circle radius percent of the shorter side (ignored for rectangle).
    pub radius: f32,
}

#[derive(Debug, Clone, PartialEq)]
enum MaskScrimFingerprint {
    Shape {
        size: u32,
        params: MaskShapeParams,
    },
    Png {
        size: u32,
        path: PathBuf,
        mtime_secs: Option<u64>,
    },
}

/// Cached egui texture for the items-grid scrim.
pub(crate) struct MaskScrimCache {
    fingerprint: MaskScrimFingerprint,
    tex: TextureHandle,
}

/// Parse a mask percent field; `${var}` / empty / garbage → `default`.
pub(crate) fn mask_percent(s: &str, default: f32) -> f32 {
    let t = s.trim();
    if t.is_empty() || t.contains('$') || t.contains('{') {
        return default;
    }
    t.parse::<f32>().unwrap_or(default).clamp(0.0, 100.0)
}

/// Whether pixel `(x, y)` in a `w`×`h` image is kept by `params`.
pub(crate) fn shape_keeps_pixel(
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    p: &MaskShapeParams,
) -> bool {
    let w = w.max(1) as f32;
    let h = h.max(1) as f32;
    let cx = p.center_x / 100.0 * w;
    let cy = p.center_y / 100.0 * h;
    let inside = if p.circle {
        let r = p.radius / 100.0 * w.min(h);
        let dx = (x as f32 + 0.5) - cx;
        let dy = (y as f32 + 0.5) - cy;
        dx * dx + dy * dy <= r * r
    } else {
        let half_w = p.base / 100.0 * w * 0.5;
        let half_h = p.height / 100.0 * h * 0.5;
        let px = x as f32 + 0.5;
        let py = y as f32 + 0.5;
        (px - cx).abs() <= half_w && (py - cy).abs() <= half_h
    };
    if p.inverse {
        inside
    } else {
        !inside
    }
}

/// RGBA bytes: transparent kept, `scrim` where excluded.
///
/// Output is unmultiplied so [`ColorImage::from_rgba_unmultiplied`] keeps the
/// intended alpha (egui `Color32` channels are premultiplied).
pub(crate) fn build_shape_scrim_rgba(
    w: usize,
    h: usize,
    p: &MaskShapeParams,
    scrim: Color32,
) -> Vec<u8> {
    let mut out = vec![0u8; w * h * 4];
    let [sr, sg, sb, sa] = scrim.to_srgba_unmultiplied();
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            if !shape_keeps_pixel(x, y, w, h, p) {
                out[i] = sr;
                out[i + 1] = sg;
                out[i + 2] = sb;
                out[i + 3] = sa;
            }
        }
    }
    out
}

/// RGBA bytes from a mask PNG (any channel > 0 = kept). `src` is raw RGBA.
pub(crate) fn build_png_scrim_rgba(
    src_rgba: &[u8],
    src_w: usize,
    src_h: usize,
    tw: usize,
    th: usize,
    scrim: Color32,
) -> Vec<u8> {
    let mut out = vec![0u8; tw * th * 4];
    if src_w == 0 || src_h == 0 || tw == 0 || th == 0 {
        return out;
    }
    let [sr, sg, sb, sa] = scrim.to_srgba_unmultiplied();
    for y in 0..th {
        let sy = y * src_h / th;
        for x in 0..tw {
            let sx = x * src_w / tw;
            let s = (sy * src_w + sx) * 4;
            let kept = (src_rgba[s] | src_rgba[s + 1] | src_rgba[s + 2]) > 0;
            if !kept {
                let i = (y * tw + x) * 4;
                out[i] = sr;
                out[i + 1] = sg;
                out[i + 2] = sb;
                out[i + 3] = sa;
            }
        }
    }
    out
}

fn path_mtime_secs(path: &Path) -> Option<u64> {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

fn load_mask_png_rgba(path: &Path) -> Option<(usize, usize, Vec<u8>)> {
    let bytes = sqyre_ports::read_file_capped(path, sqyre_ports::MAX_IMAGE_FILE_BYTES).ok()?;
    let img = sqyre_ports::decode_image_bounded(&bytes).ok()?.into_rgba8();
    let w = img.width() as usize;
    let h = img.height() as usize;
    Some((w, h, img.into_raw()))
}

/// Ensure `cache` holds a scrim texture for the current mask form inputs.
pub(crate) fn ensure_mask_scrim(
    ctx: &egui::Context,
    cache: &mut Option<MaskScrimCache>,
    size: u32,
    mask_png: Option<&Path>,
    shape: MaskShapeParams,
) -> Option<TextureHandle> {
    let size = size.max(8);
    let fingerprint = if let Some(path) = mask_png.filter(|p| p.is_file()) {
        MaskScrimFingerprint::Png {
            size,
            path: path.to_path_buf(),
            mtime_secs: path_mtime_secs(path),
        }
    } else {
        MaskScrimFingerprint::Shape {
            size,
            params: shape,
        }
    };
    if let Some(c) = cache.as_ref() {
        if c.fingerprint == fingerprint {
            return Some(c.tex.clone());
        }
    }
    let scrim = crate::theme::preview_scrim();
    let rgba = match &fingerprint {
        MaskScrimFingerprint::Shape { size, params } => {
            let s = *size as usize;
            build_shape_scrim_rgba(s, s, params, scrim)
        }
        MaskScrimFingerprint::Png { size, path, .. } => {
            let (sw, sh, raw) = load_mask_png_rgba(path)?;
            let s = *size as usize;
            build_png_scrim_rgba(&raw, sw, sh, s, s, scrim)
        }
    };
    let s = size as usize;
    let color = ColorImage::from_rgba_unmultiplied([s, s], &rgba);
    let tex = ctx.load_texture("mask_scrim_overlay", color, egui::TextureOptions::NEAREST);
    *cache = Some(MaskScrimCache {
        fingerprint,
        tex: tex.clone(),
    });
    Some(tex)
}

/// Paint a cached scrim stretched over `icon_rect`.
pub(crate) fn paint_mask_scrim_over(
    painter: &egui::Painter,
    icon_rect: egui::Rect,
    tex: &TextureHandle,
) {
    if !icon_rect.is_positive() {
        return;
    }
    painter.image(
        tex.id(),
        icon_rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect_params(inverse: bool) -> MaskShapeParams {
        MaskShapeParams {
            circle: false,
            inverse,
            center_x: 50.0,
            center_y: 50.0,
            base: 50.0,
            height: 50.0,
            radius: 0.0,
        }
    }

    fn circle_params(inverse: bool) -> MaskShapeParams {
        MaskShapeParams {
            circle: true,
            inverse,
            center_x: 50.0,
            center_y: 50.0,
            base: 0.0,
            height: 0.0,
            radius: 25.0,
        }
    }

    #[test]
    fn mask_percent_defaults_on_var_or_garbage() {
        assert_eq!(mask_percent("50", 10.0), 50.0);
        assert_eq!(mask_percent("${x}", 50.0), 50.0);
        assert_eq!(mask_percent("", 40.0), 40.0);
        assert_eq!(mask_percent("nope", 33.0), 33.0);
        assert_eq!(mask_percent("150", 0.0), 100.0);
    }

    #[test]
    fn inverse_rect_keeps_center_excludes_corner() {
        let p = rect_params(true);
        assert!(shape_keeps_pixel(16, 16, 32, 32, &p));
        assert!(!shape_keeps_pixel(0, 0, 32, 32, &p));
        let scrim = Color32::from_rgba_unmultiplied(16, 16, 16, 170);
        let rgba = build_shape_scrim_rgba(32, 32, &p, scrim);
        // Center kept → transparent.
        let c = (16 * 32 + 16) * 4;
        assert_eq!(&rgba[c..c + 4], &[0, 0, 0, 0]);
        // Corner excluded → scrim (unmultiplied).
        assert_eq!(&rgba[0..4], &scrim.to_srgba_unmultiplied());
    }

    #[test]
    fn non_inverse_rect_scrims_center() {
        let p = rect_params(false);
        assert!(!shape_keeps_pixel(16, 16, 32, 32, &p));
        assert!(shape_keeps_pixel(0, 0, 32, 32, &p));
    }

    #[test]
    fn inverse_circle_keeps_center() {
        let p = circle_params(true);
        assert!(shape_keeps_pixel(16, 16, 32, 32, &p));
        assert!(!shape_keeps_pixel(0, 0, 32, 32, &p));
    }

    #[test]
    fn png_scrim_keeps_white_excludes_black() {
        // 2×2: white, black / black, white
        let src = [
            255, 255, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255,
        ];
        let scrim = Color32::from_rgba_unmultiplied(16, 16, 16, 170);
        let expected = scrim.to_srgba_unmultiplied();
        let out = build_png_scrim_rgba(&src, 2, 2, 2, 2, scrim);
        assert_eq!(&out[0..4], &[0, 0, 0, 0]);
        assert_eq!(&out[4..8], &expected);
        assert_eq!(&out[8..12], &expected);
        assert_eq!(&out[12..16], &[0, 0, 0, 0]);
    }
}
