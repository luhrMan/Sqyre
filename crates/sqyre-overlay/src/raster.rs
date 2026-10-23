//! CPU rasterizer for native overlay buttons (colors, radius, Phosphor icon).

use crate::brand_s;
use egui_phosphor::Variant;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use fontdue::layout::{CoordinateSystem, Layout, LayoutSettings, TextStyle};
use fontdue::Font;
use std::sync::OnceLock;

#[cfg(any(target_os = "linux", target_os = "windows"))]
const TIP_PAD_X: f32 = 11.0;
#[cfg(any(target_os = "linux", target_os = "windows"))]
const TIP_PAD_Y: f32 = 7.0;
#[cfg(any(target_os = "linux", target_os = "windows"))]
const TIP_MAX_TEXT_W: f32 = 280.0;
/// Logical tip font size; rasterized at [`TIP_SSAA`]× then downsampled.
#[cfg(any(target_os = "linux", target_os = "windows"))]
const TIP_FONT_PX: f32 = 13.0;
#[cfg(any(target_os = "linux", target_os = "windows"))]
const TIP_SSAA: u32 = 2;
/// Shared with X11 tip window shape.
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub const TIP_CORNER_PX: f32 = 7.0;
#[cfg(any(target_os = "linux", target_os = "windows"))]
const TIP_BORDER_W: f32 = 1.0;
/// Panel fill `#1c1914` — shared with tip window backing pixel.
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub const TIP_BG_RGB: [u8; 3] = [0x1c, 0x19, 0x14];
/// Soft gold border.
#[cfg(any(target_os = "linux", target_os = "windows"))]
const TIP_BORDER: [u8; 4] = [0xdc, 0x9d, 0x2e, 0xa0];
/// Cream text `#f5e6c0`.
#[cfg(any(target_os = "linux", target_os = "windows"))]
const TIP_FG: [u8; 4] = [0xf5, 0xe6, 0xc0, 0xff];
#[cfg(target_os = "linux")]
const MENU_ROW_H: f32 = 28.0;
#[cfg(target_os = "linux")]
const MENU_PAD_X: f32 = 12.0;
#[cfg(target_os = "linux")]
const MENU_PAD_Y: f32 = 8.0;
#[cfg(target_os = "linux")]
const MENU_TITLE_H: f32 = 22.0;
#[cfg(target_os = "linux")]
const MENU_MAX_W: f32 = 360.0;
#[cfg(target_os = "linux")]
const MENU_MIN_W: f32 = 180.0;

#[cfg(target_os = "linux")]
const UI_FONT_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/liberation-sans-fonts/LiberationSans-Regular.ttf",
    "/usr/local/share/fonts/liberation-sans-fonts/LiberationSans-Regular.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/TTF/DejaVuSans.ttf",
];
#[cfg(target_os = "windows")]
const UI_FONT_CANDIDATES: &[&str] = &[
    r"C:\Windows\Fonts\segoeui.ttf",
    r"C:\Windows\Fonts\arial.ttf",
];

/// Mutable RGBA8 destination: `rgba` is row-major with length `w * h * 4`.
///
/// Bundles the `(buffer, width, height)` triple every drawing helper needs so
/// they take a canvas plus their own geometry instead of re-threading all three.
struct Canvas<'a> {
    rgba: &'a mut [u8],
    w: u32,
    h: u32,
}

impl Canvas<'_> {
    /// Byte offset of pixel `(x, y)`, or `None` when outside the buffer.
    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return None;
        }
        Some(((y as u32 * self.w + x as u32) * 4) as usize)
    }
}

/// One selectable chooser row: `(name, x, y, w, h)` in window coordinates.
/// An empty `name` marks the Cancel row.
#[cfg(target_os = "linux")]
pub type ChooserRow = (String, i32, i32, u32, u32);

/// Style + content needed to paint one button face.
///
/// `bg[3] == 0` means no fill: only border / icon / spinner ink are written.
/// [`rasterize`] (24-bit face) treats any `bg[3] > 0` as opaque and uses alpha
/// as an XFixes ShapeBounding mask; [`rasterize_argb`] (32-bit face) honors
/// real translucency.
#[derive(Debug, Clone)]
pub struct ButtonPaint {
    pub w: u32,
    pub h: u32,
    pub bg: [u8; 4],
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

#[cfg(any(target_os = "linux", target_os = "windows"))]
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
#[cfg(any(target_os = "linux", target_os = "windows"))]
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
    let mut canvas = Canvas {
        rgba: &mut hi,
        w: hi_w,
        h: hi_h,
    };
    fill_rounded_panel(&mut canvas, corner, border_w);

    let ox = pad_x - ink_min_x;
    let oy = pad_y - ink_min_y;
    for g in glyphs {
        let (metrics, bitmap) = font.rasterize_config(g.key);
        if metrics.width == 0 || metrics.height == 0 || bitmap.is_empty() {
            continue;
        }
        blit_coverage(
            &mut canvas,
            Coverage {
                bitmap: &bitmap,
                bw: metrics.width,
                bh: metrics.height,
                ox: (ox + g.x).round() as i32,
                oy: (oy + g.y).round() as i32,
            },
            TIP_FG,
        );
    }

    if TIP_SSAA <= 1 {
        return (hi_w, hi_h, hi);
    }
    downsample_box(&hi, hi_w, hi_h, TIP_SSAA)
}

/// Opaque hotkey-conflict menu: `(w, h, rgba, row_hits)` in physical pixels.
/// `hover` / `pressed` are optional row indices (including Cancel).
#[cfg(target_os = "linux")]
pub fn rasterize_chooser(
    title: &str,
    names: &[String],
    hover: Option<usize>,
    pressed: Option<usize>,
) -> (u32, u32, Vec<u8>, Vec<ChooserRow>) {
    if names.is_empty() {
        return (0, 0, Vec::new(), Vec::new());
    }
    let font = ui_font();
    let title = title.trim();
    let mut max_text = title.len().max(12) as f32 * 7.5;
    for n in names {
        max_text = max_text.max(n.len() as f32 * 7.5);
    }
    max_text = max_text.max(6.0 * 7.5); // "Cancel"
    let content_w = max_text.clamp(MENU_MIN_W - MENU_PAD_X * 2.0, MENU_MAX_W - MENU_PAD_X * 2.0);
    let w = (content_w + MENU_PAD_X * 2.0)
        .ceil()
        .clamp(MENU_MIN_W, MENU_MAX_W) as u32;
    let title_h = if title.is_empty() { 0.0 } else { MENU_TITLE_H };
    // title + macro rows + gap + Cancel
    let h = (MENU_PAD_Y * 2.0 + title_h + names.len() as f32 * MENU_ROW_H + 6.0 + MENU_ROW_H + 4.0)
        .ceil() as u32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let mut canvas = Canvas {
        rgba: &mut rgba,
        w,
        h,
    };
    fill_rounded_panel(&mut canvas, TIP_CORNER_PX, TIP_BORDER_W);

    let mut y = MENU_PAD_Y;
    if !title.is_empty() {
        draw_line_text(
            &mut canvas,
            font,
            title,
            (MENU_PAD_X, y + 2.0, TIP_FONT_PX * 0.9),
            [0xb8, 0xa8, 0x88, 0xff],
        );
        y += title_h;
    }

    let mut hits = Vec::with_capacity(names.len() + 1);
    let row_x = 6i32;
    let row_w = w.saturating_sub(12);
    for (i, name) in names.iter().enumerate() {
        let row_y = y.round() as i32;
        let row_h = MENU_ROW_H.round() as u32;
        let state = row_chip_state(i, hover, pressed);
        fill_row_chip(&mut canvas, (row_x, row_y, row_w, row_h), false, state);
        hits.push((name.clone(), row_x, row_y, row_w, row_h));
        let fg = match state {
            ChipState::Pressed => [0xff, 0xf0, 0xc8, 0xff],
            ChipState::Hover => [0xff, 0xf6, 0xd8, 0xff],
            ChipState::Idle => TIP_FG,
        };
        draw_line_text(
            &mut canvas,
            font,
            name,
            (MENU_PAD_X, y + 6.0, TIP_FONT_PX),
            fg,
        );
        y += MENU_ROW_H;
    }

    y += 6.0;
    let cancel_i = names.len();
    let row_y = y.round() as i32;
    let row_h = MENU_ROW_H.round() as u32;
    let state = row_chip_state(cancel_i, hover, pressed);
    fill_row_chip(&mut canvas, (row_x, row_y, row_w, row_h), true, state);
    hits.push((String::new(), row_x, row_y, row_w, row_h));
    let fg = match state {
        ChipState::Pressed => [0xe8, 0xd8, 0xb8, 0xff],
        ChipState::Hover => [0xd8, 0xc8, 0xa8, 0xff],
        ChipState::Idle => [0xc8, 0xb8, 0x98, 0xff],
    };
    draw_line_text(
        &mut canvas,
        font,
        "Cancel",
        (MENU_PAD_X, y + 6.0, TIP_FONT_PX),
        fg,
    );

    (w, h, rgba, hits)
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg(target_os = "linux")]
enum ChipState {
    Idle,
    Hover,
    Pressed,
}

#[cfg(target_os = "linux")]
fn row_chip_state(i: usize, hover: Option<usize>, pressed: Option<usize>) -> ChipState {
    if pressed == Some(i) {
        ChipState::Pressed
    } else if hover == Some(i) {
        ChipState::Hover
    } else {
        ChipState::Idle
    }
}

#[cfg(target_os = "linux")]
fn fill_row_chip(
    canvas: &mut Canvas<'_>,
    (x, y, rw, rh): (i32, i32, u32, u32),
    muted: bool,
    state: ChipState,
) {
    let (bg, border) = match (muted, state) {
        (false, ChipState::Idle) => ([0x32, 0x2c, 0x24, 0xff], [0x9a, 0x78, 0x38, 0xff]),
        (false, ChipState::Hover) => ([0x4a, 0x3e, 0x2a, 0xff], [0xef, 0xc0, 0x4a, 0xff]),
        (false, ChipState::Pressed) => ([0x24, 0x1e, 0x16, 0xff], [0xdc, 0x9d, 0x2e, 0xff]),
        (true, ChipState::Idle) => ([0x2a, 0x26, 0x20, 0xff], [0x6a, 0x5c, 0x44, 0xff]),
        (true, ChipState::Hover) => ([0x3a, 0x34, 0x2c, 0xff], [0x9a, 0x88, 0x68, 0xff]),
        (true, ChipState::Pressed) => ([0x1e, 0x1a, 0x16, 0xff], [0x8a, 0x78, 0x58, 0xff]),
    };
    let radius = 5.0_f32;
    let border_w = if state == ChipState::Idle { 1.2 } else { 2.0 };
    for py in y..(y + rh as i32) {
        for px in x..(x + rw as i32) {
            let Some(i) = canvas.index(px, py) else {
                continue;
            };
            let lx = px - x;
            let ly = py - y;
            let d = sd_rounded_rect(
                lx as f32 + 0.5,
                ly as f32 + 0.5,
                rw as f32,
                rh as f32,
                radius,
            );
            if d > 0.0 {
                continue;
            }
            canvas.rgba[i] = bg[0];
            canvas.rgba[i + 1] = bg[1];
            canvas.rgba[i + 2] = bg[2];
            canvas.rgba[i + 3] = 255;
            if d >= -border_w {
                canvas.rgba[i] = border[0];
                canvas.rgba[i + 1] = border[1];
                canvas.rgba[i + 2] = border[2];
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn draw_line_text(
    canvas: &mut Canvas<'_>,
    font: &Font,
    text: &str,
    (x, y, px): (f32, f32, f32),
    color: [u8; 4],
) {
    let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
    layout.reset(&LayoutSettings {
        x: 0.0,
        y: 0.0,
        max_width: Some((canvas.w as f32 - x - MENU_PAD_X).max(8.0)),
        ..LayoutSettings::default()
    });
    layout.append(&[font], &TextStyle::new(text, px, 0));
    let glyphs = layout.glyphs();
    if glyphs.is_empty() {
        return;
    }
    let mut ink_min_y = f32::INFINITY;
    for g in glyphs {
        ink_min_y = ink_min_y.min(g.y);
    }
    let oy = y - ink_min_y;
    for g in glyphs {
        let (metrics, bitmap) = font.rasterize_config(g.key);
        if metrics.width == 0 || metrics.height == 0 || bitmap.is_empty() {
            continue;
        }
        blit_coverage(
            canvas,
            Coverage {
                bitmap: &bitmap,
                bw: metrics.width,
                bh: metrics.height,
                ox: (x + g.x).round() as i32,
                oy: (oy + g.y).round() as i32,
            },
            color,
        );
    }
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn fill_rounded_panel(canvas: &mut Canvas<'_>, corner: f32, border_w: f32) {
    let (w, h) = (canvas.w, canvas.h);
    let r = corner.min((w.min(h) as f32) * 0.5);
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let d = sd_rounded_rect(x as f32 + 0.5, y as f32 + 0.5, w as f32, h as f32, r);
            let Some(i) = canvas.index(x, y) else {
                continue;
            };
            // Always fill the window rect with panel RGB so shape misses are not black flecks.
            canvas.rgba[i] = TIP_BG_RGB[0];
            canvas.rgba[i + 1] = TIP_BG_RGB[1];
            canvas.rgba[i + 2] = TIP_BG_RGB[2];
            canvas.rgba[i + 3] = 255;
            if d <= 0.0 && border_w > 0.0 && d >= -border_w {
                let edge = ((border_w + d) / border_w).clamp(0.0, 1.0);
                let a = (TIP_BORDER[3] as f32 / 255.0) * edge;
                blend(&mut canvas.rgba[i..i + 4], TIP_BORDER, a);
            }
        }
    }
}

/// Glyph coverage bitmap (`bw` × `bh`, 8-bit alpha) blitted at `(ox, oy)`.
#[cfg(any(target_os = "linux", target_os = "windows"))]
struct Coverage<'a> {
    bitmap: &'a [u8],
    bw: usize,
    bh: usize,
    ox: i32,
    oy: i32,
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn blit_coverage(canvas: &mut Canvas<'_>, cov: Coverage<'_>, color: [u8; 4]) {
    let a_scale = color[3] as f32 / 255.0;
    for gy in 0..cov.bh {
        for gx in 0..cov.bw {
            let cover = cov.bitmap[gy * cov.bw + gx] as f32 / 255.0;
            if cover < 0.01 {
                continue;
            }
            let Some(i) = canvas.index(cov.ox + gx as i32, cov.oy + gy as i32) else {
                continue;
            };
            blend(&mut canvas.rgba[i..i + 4], color, cover * a_scale);
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
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

/// Alpha above this is included in the XFixes shape (binary punch-out, not soft glass).
#[cfg(target_os = "linux")]
pub const SHAPE_ALPHA_MIN: u8 = 96;

/// How glyph / spinner ink lands on the painted face.
#[derive(Clone, Copy, PartialEq, Eq)]
enum InkMode {
    /// Opaque face: source-over RGB, alpha forced 255.
    #[cfg(target_os = "linux")]
    Fill,
    /// Punch-out mask: keep RGB, accumulate alpha for the XFixes shape.
    #[cfg(target_os = "linux")]
    Mask,
    /// Premultiplied ARGB face: source-over including alpha.
    Over,
}

/// Paint an RGBA8 buffer (row-major, length `w*h*4`) for a 24-bit face.
///
/// With an opaque `bg`, every in-shape pixel is filled (alpha 255). With
/// `bg[3] == 0`, only chrome ink is written and alpha marks the punch-out mask.
#[cfg(target_os = "linux")]
pub fn rasterize(paint: &ButtonPaint) -> Vec<u8> {
    let w = paint.w.max(1) as i32;
    let h = paint.h.max(1) as i32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let fill_bg = paint.bg[3] > 0;

    let r = paint.corner_radius.clamp(0.0, (w.min(h) as f32) * 0.5);
    let bw = paint.border_width.max(0.0);

    for y in 0..h {
        for x in 0..w {
            let d = sd_rounded_rect(x as f32 + 0.5, y as f32 + 0.5, w as f32, h as f32, r);
            let i = ((y * w + x) * 4) as usize;
            if d > 0.0 {
                continue;
            }
            if fill_bg {
                rgba[i] = paint.bg[0];
                rgba[i + 1] = paint.bg[1];
                rgba[i + 2] = paint.bg[2];
                rgba[i + 3] = 255;
                if bw > 0.0 && paint.border[3] > 0 && d >= -bw {
                    let a = (paint.border[3] as f32 / 255.0).clamp(0.0, 1.0);
                    blend(&mut rgba[i..i + 4], paint.border, a);
                }
            } else if bw > 0.0 && paint.border[3] > 0 && d >= -bw {
                // Border ring only — solid ink for opaque-window + Shape punch-out.
                let a = paint.border[3];
                rgba[i] = paint.border[0];
                rgba[i + 1] = paint.border[1];
                rgba[i + 2] = paint.border[2];
                rgba[i + 3] = a;
            }
        }
    }

    let mode = if fill_bg {
        InkMode::Fill
    } else {
        InkMode::Mask
    };
    stamp_face_ink(&mut rgba, w as u32, h as u32, paint, mode);
    rgba
}

/// Paint a premultiplied RGBA8 buffer (row-major, length `w*h*4`) for a
/// 32-bit ARGB face.
///
/// Unlike [`rasterize`], which treats `bg[3]` as a fill/no-fill switch for
/// opaque windows, this honors translucency: fill and border are composited
/// by rounded-rect coverage (anti-aliased edge) and ink lands source-over.
/// Corners outside the edge stay alpha 0 — the painted alpha *is* the shape,
/// so an ARGB face needs no XFixes Bounding (and cannot overflow past it).
pub fn rasterize_argb(paint: &ButtonPaint) -> Vec<u8> {
    let w = paint.w.max(1) as i32;
    let h = paint.h.max(1) as i32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];

    let r = paint.corner_radius.clamp(0.0, (w.min(h) as f32) * 0.5);
    let bw = paint.border_width.max(0.0);
    let bg_a = paint.bg[3] as f32 / 255.0;
    let border_a = paint.border[3] as f32 / 255.0;
    let banded = bw > 0.0 && border_a > 0.0;

    for y in 0..h {
        for x in 0..w {
            let d = sd_rounded_rect(x as f32 + 0.5, y as f32 + 0.5, w as f32, h as f32, r);
            let fill_cov = (0.5 - d).clamp(0.0, 1.0);
            if fill_cov <= 0.0 {
                continue;
            }
            let band_cov = if banded {
                (fill_cov - (0.5 - (d + bw)).clamp(0.0, 1.0)).max(0.0)
            } else {
                0.0
            };
            let a_fill = bg_a * fill_cov;
            let a_band = border_a * band_cov;
            let inv = 1.0 - a_band;
            let i = ((y * w + x) * 4) as usize;
            rgba[i] =
                (paint.border[0] as f32 * a_band + paint.bg[0] as f32 * a_fill * inv).round() as u8;
            rgba[i + 1] =
                (paint.border[1] as f32 * a_band + paint.bg[1] as f32 * a_fill * inv).round() as u8;
            rgba[i + 2] =
                (paint.border[2] as f32 * a_band + paint.bg[2] as f32 * a_fill * inv).round() as u8;
            rgba[i + 3] = ((a_band + a_fill * inv) * 255.0).round() as u8;
        }
    }

    stamp_face_ink(&mut rgba, w as u32, h as u32, paint, InkMode::Over);
    rgba
}

/// Icon glyph or busy spinner composited onto the painted face.
fn stamp_face_ink(rgba: &mut [u8], w: u32, h: u32, paint: &ButtonPaint, mode: InkMode) {
    // Icon: the busy spinner replaces it outright (no dimmed ghost behind).
    let icon_c = if paint.hovered {
        paint.icon_hover
    } else {
        paint.icon
    };
    let mut canvas = Canvas { rgba, w, h };
    if !paint.busy && paint.icon_glyph != '\0' && icon_c[3] > 0 {
        blit_glyph(
            &mut canvas,
            paint.icon_glyph,
            (w.min(h) as f32 * 0.55).max(8.0),
            icon_c,
            mode,
        );
    }

    if paint.busy {
        draw_spinner(&mut canvas, paint.busy_phase, paint.icon_hover, mode);
    }
}

/// Scanline rectangles for pixels with alpha ≥ [`SHAPE_ALPHA_MIN`].
#[cfg(target_os = "linux")]
pub fn shape_rects_from_alpha(rgba: &[u8], w: u32, h: u32) -> Vec<(i16, i16, u16, u16)> {
    if w == 0 || h == 0 || rgba.len() < (w * h * 4) as usize {
        return Vec::new();
    }
    let mut out = Vec::new();
    for y in 0..h {
        let mut x = 0u32;
        while x < w {
            while x < w && rgba[((y * w + x) * 4 + 3) as usize] < SHAPE_ALPHA_MIN {
                x += 1;
            }
            if x >= w {
                break;
            }
            let start = x;
            while x < w && rgba[((y * w + x) * 4 + 3) as usize] >= SHAPE_ALPHA_MIN {
                x += 1;
            }
            let width = (x - start) as u16;
            if width > 0 {
                out.push((start as i16, y as i16, width, 1));
            }
        }
    }
    out
}

fn sd_rounded_rect(px: f32, py: f32, w: f32, h: f32, r: f32) -> f32 {
    let half = [w * 0.5, h * 0.5];
    let p = [px - half[0], py - half[1]];
    let q = [p[0].abs() - (half[0] - r), p[1].abs() - (half[1] - r)];
    let outside = [q[0].max(0.0), q[1].max(0.0)];
    let len = (outside[0] * outside[0] + outside[1] * outside[1]).sqrt();
    len + q[0].min(0.0).max(q[1].min(0.0)) - r
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn blend(dst: &mut [u8], src: [u8; 4], a: f32) {
    let a = a.clamp(0.0, 1.0);
    for i in 0..3 {
        dst[i] = ((src[i] as f32) * a + (dst[i] as f32) * (1.0 - a)).round() as u8;
    }
    dst[3] = 255;
}

/// Write ink for punch-out windows: keep RGB, accumulate alpha for the shape mask.
#[cfg(target_os = "linux")]
fn stamp_ink(dst: &mut [u8], src: [u8; 4], a: f32) {
    let a = a.clamp(0.0, 1.0);
    if a < 0.01 {
        return;
    }
    if dst[3] == 0 {
        dst[0] = src[0];
        dst[1] = src[1];
        dst[2] = src[2];
        dst[3] = (a * 255.0).round().clamp(0.0, 255.0) as u8;
        return;
    }
    for i in 0..3 {
        dst[i] = ((src[i] as f32) * a + (dst[i] as f32) * (1.0 - a)).round() as u8;
    }
    dst[3] = dst[3].max((a * 255.0).round().clamp(0.0, 255.0) as u8);
}

/// Source-over ink onto a premultiplied ARGB face (`a` already includes src alpha).
fn stamp_over(dst: &mut [u8], src: [u8; 4], a: f32) {
    let sa = a.clamp(0.0, 1.0);
    if sa < 0.01 {
        return;
    }
    let inv = 1.0 - sa;
    for i in 0..3 {
        dst[i] = ((src[i] as f32) * sa + (dst[i] as f32) * inv)
            .round()
            .clamp(0.0, 255.0) as u8;
    }
    dst[3] = (sa * 255.0 + (dst[3] as f32) * inv)
        .round()
        .clamp(0.0, 255.0) as u8;
}

fn stamp_mode(dst: &mut [u8], src: [u8; 4], a: f32, mode: InkMode) {
    match mode {
        #[cfg(target_os = "linux")]
        InkMode::Fill => blend(dst, src, a),
        #[cfg(target_os = "linux")]
        InkMode::Mask => stamp_ink(dst, src, a),
        InkMode::Over => stamp_over(dst, src, a),
    }
}

fn blit_glyph(canvas: &mut Canvas<'_>, ch: char, px: f32, color: [u8; 4], mode: InkMode) {
    let font = phosphor_font();
    let (metrics, bitmap) = font.rasterize(ch, px);
    if metrics.width == 0 || metrics.height == 0 || bitmap.is_empty() {
        return;
    }
    let ox = (canvas.w as i32 - metrics.width as i32) / 2;
    let oy = (canvas.h as i32 - metrics.height as i32) / 2;
    let a_scale = color[3] as f32 / 255.0;
    for gy in 0..metrics.height {
        for gx in 0..metrics.width {
            let cover = bitmap[gy * metrics.width + gx] as f32 / 255.0;
            if cover < 0.01 {
                continue;
            }
            let Some(i) = canvas.index(ox + gx as i32, oy + gy as i32) else {
                continue;
            };
            let a = cover * a_scale;
            stamp_mode(&mut canvas.rgba[i..i + 4], color, a, mode);
        }
    }
}

thread_local! {
    /// Coverage scratch for [`draw_spinner`] — the host thread repaints every
    /// busy button on each 50 ms tick; reuse the `w*h` buffer across frames.
    static SPIN_COV: std::cell::RefCell<Vec<f32>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// S glyph fit on a `w`×`h` face: (scale, cx, cy). Shared by the animated
/// spinner and the busy shape union so both trace identical ink.
fn s_fit(w: u32, h: u32) -> (f32, f32, f32) {
    let scale = (w as f32 / brand_s::GLYPH_W).min(h as f32 / brand_s::GLYPH_H) * brand_s::FACE_FILL;
    (scale, w as f32 * 0.5, h as f32 * 0.5)
}

/// Both S strokes' coverage into `cov` (zeroed first, `w*h`), with
/// `weight(face_px_center, face_py_center)` multiplying each pixel — the
/// sweep gate dims coverage by traversal position.
fn s_coverage(cov: &mut Vec<f32>, w: u32, h: u32, weight: impl Fn(f32, f32) -> f32) {
    let (scale, cx, cy) = s_fit(w, h);
    cov.clear();
    cov.resize((w * h) as usize, 0.0);
    for half in brand_s::halves() {
        brand_s::rasterize_weighted(
            cov.as_mut_slice(),
            w,
            h,
            half,
            |x, y| (cx + x * scale, cy + y * scale),
            &weight,
        );
    }
}

/// Brand spinner: the two S half-strokes drawn as themselves, gated by a
/// sweep window — one gap travels down the S's own figure-8 (upper lobe
/// then lower lobe) and wraps invisibly through the open tips.
fn draw_spinner(canvas: &mut Canvas<'_>, phase: f32, color: [u8; 4], mode: InkMode) {
    let (scale, cx, cy) = s_fit(canvas.w, canvas.h);
    SPIN_COV.with(|cov| {
        let mut cov = cov.borrow_mut();
        s_coverage(&mut cov, canvas.w, canvas.h, |px, py| {
            brand_s::trace_weight((px - cx) / scale, (py - cy) / scale, phase)
        });
        stamp_spinner(canvas, cov.as_slice(), color, mode);
    });
}

fn stamp_spinner(canvas: &mut Canvas<'_>, cov: &[f32], color: [u8; 4], mode: InkMode) {
    let a_scale = color[3] as f32 / 255.0;
    for (px, &c) in cov.iter().enumerate() {
        let a = c.min(1.0) * a_scale;
        if a < 0.01 {
            continue;
        }
        let i = px * 4;
        stamp_mode(&mut canvas.rgba[i..i + 4], color, a, mode);
    }
}

/// Union alpha of the busy face across the spinner's whole orbit (border +
/// S strokes at every position). `ShapeBounding` is applied once per busy
/// flip — per-frame shape updates regress on XWayland — so the busy frames
/// must all paint inside a bounding computed from this union, or strokes
/// clip wherever they leave the first frame's ink.
#[cfg(target_os = "linux")]
pub fn busy_shape_alpha(paint: &ButtonPaint) -> Vec<u8> {
    // The union over a full sweep is the S *fully drawn*: every traversal
    // fraction is inked at some phase, so one full-coverage pass replaces
    // sampling animation frames. Chrome is phase-invariant — rasterize it
    // once without the icon the busy state suppresses.
    let mut rgba = rasterize(&ButtonPaint {
        busy: false,
        icon_glyph: '\u{0}',
        ..paint.clone()
    });
    let (w, h) = (paint.w.max(1), paint.h.max(1));
    SPIN_COV.with(|cov| {
        let mut cov = cov.borrow_mut();
        s_coverage(&mut cov, w, h, |_, _| 1.0);
        for (o, &c) in rgba.chunks_exact_mut(4).zip(cov.iter()) {
            o[3] = o[3].max((c.min(1.0) * 255.0).round() as u8);
        }
    });
    rgba
}

#[cfg(all(test, target_os = "linux"))]
mod tip_tests {
    use super::*;

    #[test]
    fn tip_is_compact_and_has_ink() {
        let (w, h, rgba) = rasterize_tip("Remove all Gems");
        assert!((70..=200).contains(&w), "w={w}");
        assert!((18..=36).contains(&h), "h={h}");
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

    #[test]
    fn chooser_draws_row_chips_and_cancel_hit() {
        let names = vec!["Macro A".into(), "Macro B".into()];
        let (w, h, rgba, hits) = rasterize_chooser("Hotkey: f1", &names, None, None);
        assert!(w >= 180 && h > 80, "w={w} h={h}");
        assert_eq!(hits.len(), 3); // 2 macros + Cancel
        assert_eq!(hits[0].0, "Macro A");
        assert_eq!(hits[1].0, "Macro B");
        assert!(hits[2].0.is_empty(), "cancel sentinel");
        assert_eq!(rgba.len(), (w * h * 4) as usize);
        let bright = rgba
            .chunks(4)
            .filter(|p| p[0] > 40 || p[1] > 40 || p[2] > 40)
            .count();
        assert!(
            bright > 100,
            "expected row chip / text ink, bright={bright}"
        );
        let (w2, _, rgba2, _) = rasterize_chooser("Hotkey: f1", &names, Some(0), None);
        assert_eq!(w, w2);
        let hover_gold = rgba2
            .chunks(4)
            .filter(|p| p[0] > 0xE0 && p[1] > 0xA0 && p[2] < 0x80)
            .count();
        assert!(
            hover_gold > 20,
            "expected brighter hover border pixels, hover_gold={hover_gold}"
        );
    }

    #[test]
    fn busy_s_halves_orbit_with_phase() {
        let paint = ButtonPaint {
            w: 48,
            h: 48,
            bg: [0, 0, 0, 0],
            border: [0, 0, 0, 0],
            border_width: 0.0,
            corner_radius: 0.0,
            icon_glyph: '\u{0}',
            icon: [0, 0, 0, 0],
            icon_hover: [0xdc, 0x9d, 0x2e, 0xff],
            hovered: false,
            busy: true,
            busy_phase: 0.0,
        };
        let split = rasterize(&paint);
        let crossed = rasterize(&ButtonPaint {
            busy_phase: 0.5,
            ..paint.clone()
        });
        let ink = |rgba: &[u8]| {
            rgba.chunks_exact(4)
                .filter(|p| p[3] >= SHAPE_ALPHA_MIN)
                .count()
        };
        assert!(
            ink(&split) > 50 && ink(&crossed) > 50,
            "S strokes must draw"
        );
        assert_ne!(split, crossed, "spinner ink should move with phase");
    }

    #[test]
    fn busy_replaces_icon_and_shape_covers_orbit() {
        let paint = ButtonPaint {
            w: 40,
            h: 40,
            bg: [0, 0, 0, 0],
            border: [0xdc, 0x9d, 0x2e, 0xff],
            border_width: 1.5,
            corner_radius: 8.0,
            icon_glyph: egui_phosphor::regular::PLAY.chars().next().unwrap(),
            icon: [0xf5, 0xe6, 0xc0, 0xff],
            icon_hover: [0xdc, 0x9d, 0x2e, 0xff],
            hovered: false,
            busy: true,
            busy_phase: 0.0,
        };
        let busy = rasterize(&paint);
        let idle = rasterize(&ButtonPaint {
            busy: false,
            ..paint.clone()
        });
        // Cream icon pixels must be gone while busy (gold spinner ink instead).
        let cream = |rgba: &[u8]| {
            rgba.chunks_exact(4)
                .filter(|p| p[0] > 0xd0 && p[1] > 0xb0 && p[2] > 0x80 && p[3] >= SHAPE_ALPHA_MIN)
                .count()
        };
        assert!(cream(&idle) > 20, "idle should paint the icon");
        assert_eq!(cream(&busy), 0, "icon must not ghost under the spinner");

        // ShapeBounding is applied once per busy flip; it must cover the ink
        // of every animation frame or strokes clip outside the first frame.
        let union = busy_shape_alpha(&paint);
        for k in 0..16 {
            let frame = rasterize(&ButtonPaint {
                busy_phase: k as f32 / 16.0,
                ..paint.clone()
            });
            for (u, f) in union.chunks_exact(4).zip(frame.chunks_exact(4)) {
                if f[3] >= SHAPE_ALPHA_MIN {
                    assert!(
                        u[3] >= SHAPE_ALPHA_MIN,
                        "frame {k} ink clipped by the busy bounding union"
                    );
                }
            }
        }
    }

    fn argb_paint(
        bg: [u8; 4],
        border: [u8; 4],
        border_width: f32,
        corner_radius: f32,
    ) -> ButtonPaint {
        ButtonPaint {
            w: 40,
            h: 40,
            bg,
            border,
            border_width,
            corner_radius,
            icon_glyph: '\u{0}',
            icon: [0, 0, 0, 0],
            icon_hover: [0, 0, 0, 0],
            hovered: false,
            busy: false,
            busy_phase: 0.0,
        }
    }

    /// Pixel index helper: row `y`, column `x`, RGBA stride.
    fn px(y: u32, x: u32) -> usize {
        ((y * 40 + x) * 4) as usize
    }

    #[test]
    fn argb_bg_alpha_composites_and_corners_stay_clear() {
        let paint = argb_paint([0x80, 0x40, 0x20, 128], [0, 0, 0, 0], 0.0, 8.0);
        let rgba = rasterize_argb(&paint);
        let center = px(20, 20);
        // Premultiplied at ~50%: alpha carries through, RGB is scaled.
        assert_eq!(rgba[center + 3], 128);
        assert_eq!(rgba[center], 64);
        // Fully outside the rounded edge stays transparent.
        assert_eq!(rgba[3], 0);
        // The AA fringe produces partial alpha between 0 and the fill alpha.
        let fringe = rgba
            .chunks_exact(4)
            .filter(|p| p[3] > 0 && p[3] < 128)
            .count();
        assert!(fringe > 0, "rounded edge should anti-alias");
    }

    #[test]
    fn argb_border_ring_paints_over_translucent_bg() {
        let paint = argb_paint([0x10, 0x10, 0x10, 128], [0xdc, 0x9d, 0x2e, 255], 3.0, 6.0);
        let rgba = rasterize_argb(&paint);
        // Deep inside the border band: opaque gold.
        let band = px(1, 20);
        assert_eq!(rgba[band + 3], 255);
        assert_eq!(rgba[band], 0xdc);
        // Interior keeps the translucent bg.
        let center = px(20, 20);
        assert_eq!(rgba[center + 3], 128);
    }

    #[test]
    fn argb_transparent_bg_leaves_interior_clear() {
        let paint = argb_paint([0, 0, 0, 0], [0xdc, 0x9d, 0x2e, 255], 2.0, 8.0);
        let rgba = rasterize_argb(&paint);
        let center = px(20, 20);
        assert_eq!(rgba[center + 3], 0, "center must stay clear");
        let band = px(0, 20);
        assert!(rgba[band + 3] > 0, "border ring must paint");
    }

    #[test]
    fn transparent_bg_leaves_interior_clear_and_shapes_chrome() {
        let paint = ButtonPaint {
            w: 32,
            h: 32,
            bg: [0, 0, 0, 0],
            border: [0xdc, 0x9d, 0x2e, 0xff],
            border_width: 2.0,
            corner_radius: 8.0,
            icon_glyph: '\u{0}',
            icon: [0, 0, 0, 0],
            icon_hover: [0, 0, 0, 0],
            hovered: false,
            busy: false,
            busy_phase: 0.0,
        };
        let rgba = rasterize(&paint);
        let cx = ((16 * 32 + 16) * 4) as usize;
        assert_eq!(rgba[cx + 3], 0, "center must stay clear");
        let rects = shape_rects_from_alpha(&rgba, 32, 32);
        assert!(!rects.is_empty(), "border ring must produce a shape");
        let ink = rgba
            .chunks_exact(4)
            .filter(|p| p[3] >= SHAPE_ALPHA_MIN)
            .count();
        assert!(ink > 20 && ink < 32 * 32 / 2, "ink={ink}");
    }
}
