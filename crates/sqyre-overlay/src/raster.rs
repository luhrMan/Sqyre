//! CPU rasterizer for native X11 overlay buttons (colors, radius, Phosphor icon).

use egui_phosphor::Variant;
use fontdue::layout::{CoordinateSystem, Layout, LayoutSettings, TextStyle};
use fontdue::Font;
use std::sync::OnceLock;

const TIP_PAD_X: f32 = 11.0;
const TIP_PAD_Y: f32 = 7.0;
const TIP_MAX_TEXT_W: f32 = 280.0;
/// Logical tip font size; rasterized at [`TIP_SSAA`]× then downsampled.
const TIP_FONT_PX: f32 = 13.0;
const TIP_SSAA: u32 = 2;
/// Shared with X11 tip window shape.
pub const TIP_CORNER_PX: f32 = 7.0;
const TIP_BORDER_W: f32 = 1.0;
/// Panel fill `#1c1914` — shared with tip window backing pixel.
pub const TIP_BG_RGB: [u8; 3] = [0x1c, 0x19, 0x14];
/// Soft gold border.
const TIP_BORDER: [u8; 4] = [0xdc, 0x9d, 0x2e, 0xa0];
/// Cream text `#f5e6c0`.
const TIP_FG: [u8; 4] = [0xf5, 0xe6, 0xc0, 0xff];

const UI_FONT_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/liberation-sans-fonts/LiberationSans-Regular.ttf",
    "/usr/local/share/fonts/liberation-sans-fonts/LiberationSans-Regular.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/TTF/DejaVuSans.ttf",
];

/// Style + content needed to paint one button face.
#[derive(Debug, Clone)]
pub struct ButtonPaint {
    pub w: u32,
    pub h: u32,
    pub bg: [u8; 3],
    pub border: [u8; 4],
    pub border_width: f32,
    pub corner_radius: f32,
    pub icon_glyph: char,
    pub icon: [u8; 4],
    pub icon_hover: [u8; 4],
    pub hovered: bool,
    pub busy: bool,
    pub busy_phase: f32,
}

fn phosphor_font() -> &'static Font {
    static FONT: OnceLock<Font> = OnceLock::new();
    FONT.get_or_init(|| {
        Font::from_bytes(
            Variant::Regular.font_bytes(),
            fontdue::FontSettings::default(),
        )
        .expect("Phosphor Regular font")
    })
}

fn ui_font() -> &'static Font {
    static FONT: OnceLock<Font> = OnceLock::new();
    FONT.get_or_init(|| {
        for path in UI_FONT_CANDIDATES {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(font) = Font::from_bytes(bytes, fontdue::FontSettings::default()) {
                    return font;
                }
            }
        }
        // Bundled proportional fallback (prefer system Liberation/DejaVu above).
        Font::from_bytes(
            epaint_default_fonts::UBUNTU_LIGHT,
            fontdue::FontSettings::default(),
        )
        .expect("Ubuntu Light font")
    })
}

/// Opaque tip panel: `(width, height, rgba)`. Empty text → empty buffer / 0 size.
pub fn rasterize_tip(text: &str) -> (u32, u32, Vec<u8>) {
    let text = text.trim();
    if text.is_empty() {
        return (0, 0, Vec::new());
    }
    let font = ui_font();
    let ssaa = TIP_SSAA.max(1) as f32;
    let font_px = TIP_FONT_PX * ssaa;
    let pad_x = TIP_PAD_X * ssaa;
    let pad_y = TIP_PAD_Y * ssaa;
    let max_text_w = TIP_MAX_TEXT_W * ssaa;
    let corner = TIP_CORNER_PX * ssaa;
    let border_w = TIP_BORDER_W * ssaa;

    // fontdue::layout places each glyph on a shared baseline (manual ymin math
    // was making characters bob independently).
    let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
    layout.reset(&LayoutSettings {
        x: 0.0,
        y: 0.0,
        max_width: Some(max_text_w),
        ..LayoutSettings::default()
    });
    layout.append(&[font], &TextStyle::new(text, font_px, 0));
    let glyphs = layout.glyphs();
    if glyphs.is_empty() {
        return (0, 0, Vec::new());
    }

    let mut ink_min_x = f32::INFINITY;
    let mut ink_min_y = f32::INFINITY;
    let mut ink_max_x = f32::NEG_INFINITY;
    let mut ink_max_y = f32::NEG_INFINITY;
    for g in glyphs {
        ink_min_x = ink_min_x.min(g.x);
        ink_min_y = ink_min_y.min(g.y);
        ink_max_x = ink_max_x.max(g.x + g.width as f32);
        ink_max_y = ink_max_y.max(g.y + g.height as f32);
    }
    if !ink_min_x.is_finite() {
        return (0, 0, Vec::new());
    }

    let text_w = (ink_max_x - ink_min_x).max(1.0);
    let text_h = (ink_max_y - ink_min_y).max(font_px * 0.5);
    let hi_w = (text_w + pad_x * 2.0).ceil().max(1.0) as u32;
    let hi_h = (text_h + pad_y * 2.0).ceil().max(1.0) as u32;
    let mut hi = vec![0u8; (hi_w * hi_h * 4) as usize];
    fill_rounded_panel(&mut hi, hi_w, hi_h, corner, border_w);

    let ox = pad_x - ink_min_x;
    let oy = pad_y - ink_min_y;
    for g in glyphs {
        let (metrics, bitmap) = font.rasterize_config(g.key);
        if metrics.width == 0 || metrics.height == 0 || bitmap.is_empty() {
            continue;
        }
        blit_coverage(
            &mut hi,
            hi_w,
            hi_h,
            &bitmap,
            metrics.width,
            metrics.height,
            (ox + g.x).round() as i32,
            (oy + g.y).round() as i32,
            TIP_FG,
        );
    }

    if TIP_SSAA <= 1 {
        return (hi_w, hi_h, hi);
    }
    downsample_box(&hi, hi_w, hi_h, TIP_SSAA)
}

fn fill_rounded_panel(rgba: &mut [u8], w: u32, h: u32, corner: f32, border_w: f32) {
    let r = corner.min((w.min(h) as f32) * 0.5);
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let d = sd_rounded_rect(x as f32 + 0.5, y as f32 + 0.5, w as f32, h as f32, r);
            let i = ((y as u32 * w + x as u32) * 4) as usize;
            // Always fill the window rect with panel RGB so shape misses are not black flecks.
            rgba[i] = TIP_BG_RGB[0];
            rgba[i + 1] = TIP_BG_RGB[1];
            rgba[i + 2] = TIP_BG_RGB[2];
            rgba[i + 3] = 255;
            if d <= 0.0 && border_w > 0.0 && d >= -border_w {
                let edge = ((border_w + d) / border_w).clamp(0.0, 1.0);
                let a = (TIP_BORDER[3] as f32 / 255.0) * edge;
                blend(&mut rgba[i..i + 4], TIP_BORDER, a);
            }
        }
    }
}

fn blit_coverage(
    rgba: &mut [u8],
    w: u32,
    h: u32,
    bitmap: &[u8],
    bw: usize,
    bh: usize,
    ox: i32,
    oy: i32,
    color: [u8; 4],
) {
    let a_scale = color[3] as f32 / 255.0;
    for gy in 0..bh {
        for gx in 0..bw {
            let cover = bitmap[gy * bw + gx] as f32 / 255.0;
            if cover < 0.01 {
                continue;
            }
            let x = ox + gx as i32;
            let y = oy + gy as i32;
            if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
                continue;
            }
            let i = ((y as u32 * w + x as u32) * 4) as usize;
            blend(&mut rgba[i..i + 4], color, cover * a_scale);
        }
    }
}

fn downsample_box(src: &[u8], sw: u32, sh: u32, factor: u32) -> (u32, u32, Vec<u8>) {
    let f = factor.max(1);
    let dw = (sw / f).max(1);
    let dh = (sh / f).max(1);
    let mut out = vec![0u8; (dw * dh * 4) as usize];
    let inv = 1.0 / (f * f) as f32;
    for dy in 0..dh {
        for dx in 0..dw {
            let mut acc = [0.0_f32; 4];
            for oy in 0..f {
                for ox in 0..f {
                    let sx = dx * f + ox;
                    let sy = dy * f + oy;
                    if sx >= sw || sy >= sh {
                        continue;
                    }
                    let i = ((sy * sw + sx) * 4) as usize;
                    acc[0] += src[i] as f32;
                    acc[1] += src[i + 1] as f32;
                    acc[2] += src[i + 2] as f32;
                    acc[3] += src[i + 3] as f32;
                }
            }
            let o = ((dy * dw + dx) * 4) as usize;
            out[o] = (acc[0] * inv).round() as u8;
            out[o + 1] = (acc[1] * inv).round() as u8;
            out[o + 2] = (acc[2] * inv).round() as u8;
            out[o + 3] = (acc[3] * inv).round() as u8;
        }
    }
    (dw, dh, out)
}

/// Paint an opaque RGBA8 buffer (row-major, length `w*h*4`).
pub fn rasterize(paint: &ButtonPaint) -> Vec<u8> {
    let w = paint.w.max(1) as i32;
    let h = paint.h.max(1) as i32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];

    let r = paint.corner_radius.clamp(0.0, (w.min(h) as f32) * 0.5);
    let bw = paint.border_width.max(0.0);

    for y in 0..h {
        for x in 0..w {
            let d = sd_rounded_rect(x as f32 + 0.5, y as f32 + 0.5, w as f32, h as f32, r);
            let i = ((y * w + x) * 4) as usize;
            if d <= 0.0 {
                rgba[i] = paint.bg[0];
                rgba[i + 1] = paint.bg[1];
                rgba[i + 2] = paint.bg[2];
                rgba[i + 3] = 255;
                // Inner border band.
                if bw > 0.0 && paint.border[3] > 0 && d >= -bw {
                    let a = (paint.border[3] as f32 / 255.0).clamp(0.0, 1.0);
                    blend(&mut rgba[i..i + 4], paint.border, a);
                }
            }
        }
    }

    // Icon (dimmed when busy).
    let icon_c = if paint.busy {
        let c = if paint.hovered {
            paint.icon_hover
        } else {
            paint.icon
        };
        [
            c[0],
            c[1],
            c[2],
            ((c[3] as u16 * 90) / 255) as u8,
        ]
    } else if paint.hovered {
        paint.icon_hover
    } else {
        paint.icon
    };
    if paint.icon_glyph != '\0' && icon_c[3] > 0 {
        blit_glyph(
            &mut rgba,
            w as u32,
            h as u32,
            paint.icon_glyph,
            (w.min(h) as f32 * 0.55).max(8.0),
            icon_c,
        );
    }

    if paint.busy {
        draw_spinner(&mut rgba, w as u32, h as u32, paint.busy_phase, paint.icon_hover);
    }

    rgba
}

fn sd_rounded_rect(px: f32, py: f32, w: f32, h: f32, r: f32) -> f32 {
    let half = [w * 0.5, h * 0.5];
    let p = [px - half[0], py - half[1]];
    let q = [p[0].abs() - (half[0] - r), p[1].abs() - (half[1] - r)];
    let outside = [q[0].max(0.0), q[1].max(0.0)];
    let len = (outside[0] * outside[0] + outside[1] * outside[1]).sqrt();
    len + q[0].min(0.0).max(q[1].min(0.0)) - r
}

fn blend(dst: &mut [u8], src: [u8; 4], a: f32) {
    let a = a.clamp(0.0, 1.0);
    for i in 0..3 {
        dst[i] = ((src[i] as f32) * a + (dst[i] as f32) * (1.0 - a)).round() as u8;
    }
    dst[3] = 255;
}

fn blit_glyph(rgba: &mut [u8], w: u32, h: u32, ch: char, px: f32, color: [u8; 4]) {
    let font = phosphor_font();
    let (metrics, bitmap) = font.rasterize(ch, px);
    if metrics.width == 0 || metrics.height == 0 || bitmap.is_empty() {
        return;
    }
    let ox = (w as i32 - metrics.width as i32) / 2;
    let oy = (h as i32 - metrics.height as i32) / 2;
    let a_scale = color[3] as f32 / 255.0;
    for gy in 0..metrics.height {
        for gx in 0..metrics.width {
            let cover = bitmap[gy * metrics.width + gx] as f32 / 255.0;
            if cover < 0.01 {
                continue;
            }
            let x = ox + gx as i32;
            let y = oy + gy as i32;
            if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
                continue;
            }
            let i = ((y as u32 * w + x as u32) * 4) as usize;
            blend(&mut rgba[i..i + 4], color, cover * a_scale);
        }
    }
}

fn draw_spinner(rgba: &mut [u8], w: u32, h: u32, phase: f32, color: [u8; 4]) {
    let cx = w as f32 * 0.5;
    let cy = h as f32 * 0.5;
    let r = (w.min(h) as f32) * 0.32;
    let stroke = (r * 0.22).clamp(1.5, 3.0);
    const TICKS: i32 = 8;
    for i in 0..TICKS {
        let a = phase + (i as f32) * (std::f32::consts::TAU / TICKS as f32);
        let fade = 0.25 + 0.75 * (1.0 - (i as f32) / TICKS as f32);
        let c = [
            color[0],
            color[1],
            color[2],
            ((color[3] as f32) * fade).round().clamp(0.0, 255.0) as u8,
        ];
        let (dx, dy) = (a.cos(), a.sin());
        let x0 = cx + dx * (r * 0.45);
        let y0 = cy + dy * (r * 0.45);
        let x1 = cx + dx * r;
        let y1 = cy + dy * r;
        draw_line(rgba, w, h, x0, y0, x1, y1, stroke, c);
    }
}

fn draw_line(
    rgba: &mut [u8],
    w: u32,
    h: u32,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    stroke: f32,
    color: [u8; 4],
) {
    let steps = ((x1 - x0).hypot(y1 - y0) * 2.0).ceil().max(1.0) as i32;
    let half = stroke * 0.5;
    let a_scale = color[3] as f32 / 255.0;
    for s in 0..=steps {
        let t = s as f32 / steps as f32;
        let px = x0 + (x1 - x0) * t;
        let py = y0 + (y1 - y0) * t;
        let r0 = half.ceil() as i32;
        for oy in -r0..=r0 {
            for ox in -r0..=r0 {
                if (ox * ox + oy * oy) as f32 > half * half {
                    continue;
                }
                let x = (px as i32) + ox;
                let y = (py as i32) + oy;
                if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
                    continue;
                }
                let i = ((y as u32 * w + x as u32) * 4) as usize;
                blend(&mut rgba[i..i + 4], color, a_scale);
            }
        }
    }
}

#[cfg(test)]
mod tip_tests {
    use super::*;

    #[test]
    fn tip_is_compact_and_has_ink() {
        let (w, h, rgba) = rasterize_tip("Remove all Gems");
        assert!(w >= 70 && w <= 200, "w={w}");
        assert!(h >= 18 && h <= 36, "h={h}");
        assert_eq!(rgba.len(), (w * h * 4) as usize);

        let is_ink = |i: usize| {
            rgba[i] > TIP_BG_RGB[0].saturating_add(20)
                || rgba[i + 1] > TIP_BG_RGB[1].saturating_add(20)
                || rgba[i + 2] > TIP_BG_RGB[2].saturating_add(20)
        };
        let mut first = None;
        let mut last = None;
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                if is_ink(i) {
                    if first.is_none() {
                        first = Some(y);
                    }
                    last = Some(y);
                }
            }
        }
        let first = first.expect("tip text ink");
        let last = last.expect("tip text ink");
        let top_pad = first;
        let bot_pad = h - 1 - last;
        assert!(
            (top_pad as i32 - bot_pad as i32).abs() <= 3,
            "vertical padding imbalance top={top_pad} bot={bot_pad} h={h}"
        );
    }

    #[test]
    fn tip_uses_proportional_system_font_when_available() {
        let liberation = std::path::Path::new(
            "/usr/share/fonts/liberation-sans-fonts/LiberationSans-Regular.ttf",
        )
        .exists()
            || std::path::Path::new(
                "/usr/local/share/fonts/liberation-sans-fonts/LiberationSans-Regular.ttf",
            )
            .exists();
        if !liberation {
            return;
        }
        // Monospace "Remove all Gems" is much wider; proportional should be compact.
        let (w, _, _) = rasterize_tip("Remove all Gems");
        assert!(w < 160, "expected proportional tip width, got {w}");
    }
}
