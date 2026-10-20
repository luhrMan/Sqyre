//! Brand "S" spinner geometry: the Sqyre shield's S — itself a fat
//! double-ring "8" — drawn from its real upper/lower stroke outlines,
//! gated by a sweep window that travels the figure-8 like the icons8
//! "loading infinity" gap. Each pixel's traversal position is its nearest
//! point on a fitted vertical lemniscate, so the gap is always a
//! spatially contiguous wedge sliding around the S — even through the
//! middle, where the two crescents' strokes cross.
//!
//! The `d` strings are copied verbatim from
//! `crates/sqyre-app/assets/icons/sqyre-stepper-{up,down}.svg` (which
//! sqyre-app rasterizes via resvg); this crate draws the same curves
//! dependency-free. The affine below composes each SVG's nested group/path
//! transforms and re-centers the result on the shared viewBox
//! `40.5 20.25 46.75 84` so both halves compose the S at the origin.

use std::sync::OnceLock;

/// Glyph-space size of the S viewBox shared by both halves (points).
pub(crate) const GLYPH_W: f32 = 46.75;
pub(crate) const GLYPH_H: f32 = 84.0;

/// Spinner sweep rate — full traversals of the S per second. The X11 busy
/// tick advances `busy_phase` by `SWEEP_PER_SEC * tick` every 50 ms so egui
/// callers using `time * SWEEP_PER_SEC` stay in step.
pub(crate) const SWEEP_PER_SEC: f32 = 0.6;

/// Fraction of the S's own path drawn at once — one gap of `1−TRACE_DRAWN`
/// sweeps the shape (icons8 "loading infinity": ~24% empty, ~76% inked).
pub(crate) const TRACE_DRAWN: f32 = 0.76;

/// Fraction of a square button face the S glyph occupies — shared by the
/// native raster spinner and egui busy chrome so both draw it the same size.
pub(crate) const FACE_FILL: f32 = 0.80;

/// The traversal field: for each glyph-box pixel, the arc-length fraction
/// `u ∈ [0,1)` of the nearest point on a vertical Gerono lemniscate fitted
/// to the glyph. Mapping by *position* — not by which S half drew the ink —
/// keeps every `u` window a spatially contiguous wedge: the gap sweeps the
/// 8 smoothly, even through the middle where the two crescents' tips cross.
struct UField {
    w: u32,
    h: u32,
    /// Row-major traversal fraction.
    u: Vec<f32>,
}

/// The glyph-box traversal field, computed once.
fn u_field() -> &'static UField {
    static FIELD: OnceLock<UField> = OnceLock::new();
    FIELD.get_or_init(build_u_field)
}

/// Vertical Gerono lemniscate fitted to the glyph, arc-length-parametrized:
/// top → upper-right → center → lower-left → bottom → lower-right → center
/// → upper-left → top. `ax` reaches the lobes' stroke centerline (~±15 of
/// the ±23.4 edge) so nearest-point lookups order ink sensibly.
fn lemniscate() -> &'static [(f32, f32, f32)] {
    static PTS: OnceLock<Vec<(f32, f32, f32)>> = OnceLock::new();
    PTS.get_or_init(|| {
        const N: usize = 256;
        let (ax, ay) = (GLYPH_W * 0.66, GLYPH_H * 0.5);
        let mut out = Vec::with_capacity(N);
        let mut len = 0.0f32;
        let mut prev = (0.0f32, -ay);
        for k in 0..N {
            let t = k as f32 * std::f32::consts::TAU / N as f32;
            let p = (t.sin() * t.cos() * ax, -t.cos() * ay);
            len += (p.0 - prev.0).hypot(p.1 - prev.1);
            out.push((p.0, p.1, len));
            prev = p;
        }
        let total = len + (out[0].0 - prev.0).hypot(out[0].1 - prev.1);
        for p in &mut out {
            p.2 /= total;
        }
        out
    })
}

fn build_u_field() -> UField {
    let (w, h) = (half_mask(true).w as usize, half_mask(true).h as usize);
    let s = h as f32 / GLYPH_H;
    let pts = lemniscate();
    let mut u = vec![0.0f32; w * h];
    for y in 0..h {
        let gy = (y as f32 + 0.5) / s - GLYPH_H * 0.5;
        for x in 0..w {
            let gx = (x as f32 + 0.5) / s - GLYPH_W * 0.5;
            let mut best = (f32::MAX, 0.0f32);
            for &p in pts {
                let d2 = (p.0 - gx).powi(2) + (p.1 - gy).powi(2);
                if d2 < best.0 {
                    best = (d2, p.2);
                }
            }
            u[y * w + x] = best.1;
        }
    }
    UField {
        w: w as u32,
        h: h as u32,
        u,
    }
}

/// Traversal fraction `u ∈ [0,1)` of glyph-space point `(x,y)` — nearest
/// sample of the fitted figure-8 path (nearest cell; the field is smooth
/// inside each lobe, and the soft gap edge hides cell granularity).
pub(crate) fn trace_u(x: f32, y: f32) -> f32 {
    let f = u_field();
    let s = f.h as f32 / GLYPH_H;
    let px = ((x + GLYPH_W * 0.5) * s - 0.5)
        .round()
        .clamp(0.0, f.w as f32 - 1.0) as usize;
    let py = ((y + GLYPH_H * 0.5) * s - 0.5)
        .round()
        .clamp(0.0, f.h as f32 - 1.0) as usize;
    f.u[py * f.w as usize + px]
}

/// Draw weight ∈ [0,1] at traversal fraction `u`: the path is inked except
/// inside one traveling gap of `1 − TRACE_DRAWN`, with soft edges. The S's
/// open tips absorb the wrap — when the gap straddles `u = 0/1` the shape
/// is fully drawn except where the S is naturally open.
pub(crate) fn trace_alpha(u: f32, phase: f32) -> f32 {
    let d = (u - phase).rem_euclid(1.0);
    const EDGE: f32 = 0.03;
    (d / EDGE).min((TRACE_DRAWN - d) / EDGE).clamp(0.0, 1.0)
}

/// `d` attribute of `sqyre-stepper-up.svg`'s path (upper S stroke).
const UP_D: &str = "M 102.02019,71.89654 c -15.427314,7.32611 -57.60343,31.408898 9.14816,76.201828 0.59091,-10.03752 -8.85901,-16.004 -15.974005,-21.81528 -17.498204,-14.28046 -16.561768,-36.127718 6.825845,-45.456685 0.28607,0.04859 13.91221,5.906437 17.30925,13.969738 1.67665,3.979784 1.68291,7.943257 1.07648,8.789197 -1.25182,1.74619 -5.02331,3.11818 -4.99094,4.18969 2.34813,2.82188 11.44641,9.72898 12.58745,9.92071 -0.50903,-6.24026 0.74484,-12.49029 0.20173,-18.716016 C 126.46242,85.03254 114.09785,76.96368 102.02008,71.89654";

/// `d` attribute of `sqyre-stepper-down.svg`'s path (lower S stroke).
const DOWN_D: &str = "M 102.02009,179.20174 c 15.427314,-7.32611 57.60343,-31.408898 -9.14816,-76.201828 -0.59091,10.03752 8.85901,16.004 15.974005,21.81528 17.498204,14.28046 16.561768,36.127718 -6.825845,45.456685 -0.28607,-0.04859 -13.91221,-5.906437 -17.30925,-13.969738 -1.67665,-3.979784 -1.68291,-7.943257 -1.07648,-8.789197 1.25182,-1.74619 5.02331,-3.11818 4.99094,-4.18969 -2.34813,-2.82188 -11.44641,-9.72898 -12.58745,-9.92071 0.50903,6.24026 -0.74484,12.49029 -0.20173,18.716016 C 77.57786,166.06574 89.94243,174.1346 102.0202,179.20174";

/// Upper then lower S-stroke outlines: flattened closed polygons in glyph
/// space, centered on the origin, y down.
pub(crate) fn halves() -> &'static [Vec<(f32, f32)>; 2] {
    static HALVES: OnceLock<[Vec<(f32, f32)>; 2]> = OnceLock::new();
    HALVES.get_or_init(|| [flatten(UP_D), flatten(DOWN_D)])
}

/// 8-bit coverage of one S half, rasterized once for egui textures.
pub(crate) struct Mask {
    pub w: u32,
    pub h: u32,
    /// Row-major alpha; RGB is implied white so a `Color32` tint colors it.
    pub alpha: Vec<u8>,
}

/// Raster height of the cached half masks.
const MASK_H: u32 = 128;

/// Alpha coverage of one S half over the whole glyph box.
pub(crate) fn half_mask(up: bool) -> &'static Mask {
    static MASKS: [OnceLock<Mask>; 2] = [OnceLock::new(), OnceLock::new()];
    let idx = usize::from(!up); // halves()[0] is the upper stroke
    MASKS[idx].get_or_init(|| {
        let scale = MASK_H as f32 / GLYPH_H;
        let w = (GLYPH_W * scale).round().max(1.0) as u32;
        let mut cov = vec![0.0f32; (w * MASK_H) as usize];
        rasterize_into(&mut cov, w, MASK_H, &halves()[idx], |x, y| {
            ((x + GLYPH_W * 0.5) * scale, (y + GLYPH_H * 0.5) * scale)
        });
        Mask {
            w,
            h: MASK_H,
            alpha: cov
                .iter()
                .map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8)
                .collect(),
        }
    })
}

/// Accumulate `poly`'s coverage into `dst` (row-major `bw`×`bh`, NOT cleared
/// — caller zeroes once so both halves union into one buffer). `map` takes
/// glyph-space points to buffer pixel coordinates.
///
/// Scanline fill sampled at four sub-rows per pixel row with fractional span
/// ends: enough antialiasing for ~1px edges at button sizes.
pub(crate) fn rasterize_into(
    dst: &mut [f32],
    bw: u32,
    bh: u32,
    poly: &[(f32, f32)],
    map: impl Fn(f32, f32) -> (f32, f32),
) {
    rasterize_weighted(dst, bw, bh, poly, map, |_, _| 1.0);
}

/// [`rasterize_into`] with a per-pixel `weight(px_center_x, y)` multiplier —
/// the spinner's sweep gate dims coverage by traversal position.
pub(crate) fn rasterize_weighted(
    dst: &mut [f32],
    bw: u32,
    bh: u32,
    poly: &[(f32, f32)],
    map: impl Fn(f32, f32) -> (f32, f32),
    weight: impl Fn(f32, f32) -> f32,
) {
    if poly.len() < 3 {
        return;
    }
    let pts: Vec<(f32, f32)> = poly.iter().map(|&(x, y)| map(x, y)).collect();
    let y_top = pts
        .iter()
        .map(|p| p.1)
        .fold(f32::INFINITY, f32::min)
        .floor()
        .max(0.0) as i32;
    let y_bot = pts
        .iter()
        .map(|p| p.1)
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .min(bh as f32) as i32;
    let mut xs = Vec::with_capacity(8);
    for py in y_top..y_bot {
        for frac in [0.125f32, 0.375, 0.625, 0.875] {
            let y = py as f32 + frac;
            xs.clear();
            for i in 0..pts.len() {
                let (ax, ay) = pts[i];
                let (bx, by) = pts[(i + 1) % pts.len()];
                if (ay <= y) == (by <= y) {
                    continue;
                }
                xs.push(ax + (y - ay) * (bx - ax) / (by - ay));
            }
            xs.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            for span in xs.chunks_exact(2) {
                let (l, r) = (span[0], span[1]);
                let x0 = (l.floor() as i32).max(0);
                let x1 = (r.ceil() as i32).min(bw as i32);
                for px in x0..x1 {
                    let cover = (r.min(px as f32 + 1.0) - l.max(px as f32)).clamp(0.0, 1.0)
                        * 0.25
                        * weight(px as f32 + 0.5, y);
                    dst[(py as u32 * bw + px as u32) as usize] += cover;
                }
            }
        }
    }
}

/// Composite the two SVG transforms (path `matrix`, then group `matrix`) and
/// re-center into glyph space so the assembled S is origin-centered.
fn to_glyph(x: f32, y: f32) -> (f32, f32) {
    let (x, y) = (0.75853797 * x - 0.35524606, 0.75853797 * y - 23.405601);
    let (x, y) = (1.0973354 * x - 20.52961, 1.0188785 * y - 10.923955);
    (x - 40.5 - GLYPH_W * 0.5, y - 20.25 - GLYPH_H * 0.5)
}

/// Parse the `M/L/C/Z` subset of SVG path syntax used by the S strokes and
/// flatten cubics, returning a closed polygon in glyph space.
fn flatten(d: &str) -> Vec<(f32, f32)> {
    const CUBIC_STEPS: usize = 16;
    let b = d.as_bytes();
    let mut i = 0usize;
    let mut cmd = b' ';
    let mut cur = (0.0f32, 0.0f32);
    let mut out = Vec::with_capacity(160);
    while i < b.len() {
        match b[i] {
            b' ' | b',' | b'\t' | b'\n' | b'\r' => {
                i += 1;
                continue;
            }
            c if c.is_ascii_alphabetic() => {
                cmd = c;
                i += 1;
                continue;
            }
            _ => {}
        }
        match cmd {
            b'M' | b'm' | b'L' | b'l' => {
                let (x, y) = (num(b, &mut i), num(b, &mut i));
                cur = if cmd.is_ascii_lowercase() {
                    (cur.0 + x, cur.1 + y)
                } else {
                    (x, y)
                };
                out.push(to_glyph(cur.0, cur.1));
                if cmd == b'M' {
                    cmd = b'L';
                } else if cmd == b'm' {
                    cmd = b'l';
                }
            }
            b'C' | b'c' => {
                let (mut x1, mut y1) = (num(b, &mut i), num(b, &mut i));
                let (mut x2, mut y2) = (num(b, &mut i), num(b, &mut i));
                let (mut x, mut y) = (num(b, &mut i), num(b, &mut i));
                if cmd == b'c' {
                    x1 += cur.0;
                    y1 += cur.1;
                    x2 += cur.0;
                    y2 += cur.1;
                    x += cur.0;
                    y += cur.1;
                }
                let (p0, p1, p2, p3) = (cur, (x1, y1), (x2, y2), (x, y));
                for s in 1..=CUBIC_STEPS {
                    let t = s as f32 / CUBIC_STEPS as f32;
                    let mt = 1.0 - t;
                    let (x, y) = (
                        mt * mt * mt * p0.0
                            + 3.0 * mt * mt * t * p1.0
                            + 3.0 * mt * t * t * p2.0
                            + t * t * t * p3.0,
                        mt * mt * mt * p0.1
                            + 3.0 * mt * mt * t * p1.1
                            + 3.0 * mt * t * t * p2.1
                            + t * t * t * p3.1,
                    );
                    out.push(to_glyph(x, y));
                }
                cur = p3;
            }
            b'Z' | b'z' => break,
            _ => panic!(
                "unsupported S-path command {:?} (embedded stepper SVGs may only use M/L/C/Z)",
                cmd as char
            ),
        }
    }
    out
}

/// Read one SVG number at `*i` (skipping separators). The grammar is
/// `[+-]?digits[.digits][eE[+-]digits]`; a sign may directly follow the prior
/// number (`1-2`), so digits and dots are consumed one at a time.
fn num(b: &[u8], i: &mut usize) -> f32 {
    while *i < b.len() && matches!(b[*i], b' ' | b',' | b'\t' | b'\n' | b'\r') {
        *i += 1;
    }
    let start = *i;
    if *i < b.len() && matches!(b[*i], b'-' | b'+') {
        *i += 1;
    }
    while *i < b.len() && (b[*i].is_ascii_digit() || b[*i] == b'.') {
        *i += 1;
    }
    if *i < b.len() && matches!(b[*i], b'e' | b'E') {
        *i += 1;
        if *i < b.len() && matches!(b[*i], b'-' | b'+') {
            *i += 1;
        }
        while *i < b.len() && b[*i].is_ascii_digit() {
            *i += 1;
        }
    }
    let tok = std::str::from_utf8(&b[start..*i]).expect("S-path number is ASCII");
    tok.parse()
        .unwrap_or_else(|_| panic!("bad S-path number {tok:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn centroid(pts: &[(f32, f32)]) -> (f32, f32) {
        let n = pts.len() as f32;
        (
            pts.iter().map(|p| p.0).sum::<f32>() / n,
            pts.iter().map(|p| p.1).sum::<f32>() / n,
        )
    }

    #[test]
    fn halves_fill_top_and_bottom_of_the_glyph() {
        let [up, down] = halves();
        assert!(up.len() > 100 && down.len() > 100);
        let (ux, uy) = centroid(up);
        let (dx, dy) = centroid(down);
        assert!(uy < -8.0, "upper half centroid y={uy}");
        assert!(dy > 8.0, "lower half centroid y={dy}");
        // An S's lobes lean to opposite sides; the halves are mirror images.
        assert!((ux + dx).abs() < 2.0 && (uy + dy).abs() < 2.0);
        for p in up.iter().chain(down.iter()) {
            assert!(
                p.0.abs() <= GLYPH_W * 0.5 + 0.5 && p.1.abs() <= GLYPH_H * 0.5 + 0.5,
                "point {p:?} escaped the glyph box"
            );
        }
    }

    #[test]
    fn trace_u_follows_the_figure8() {
        // u = arc-length fraction along the fitted vertical lemniscate:
        // top of the glyph ≈ 0/1, bottom ≈ ½; the upper-right descent is
        // early, the upper-left ascent late.
        let top = trace_u(0.0, -GLYPH_H * 0.45);
        assert!(!(0.06..=0.94).contains(&top), "top → u={top}");
        let bottom = trace_u(0.0, GLYPH_H * 0.45);
        assert!((bottom - 0.5).abs() < 0.08, "bottom → u={bottom}");
        let right_up = trace_u(GLYPH_W * 0.3, -GLYPH_H * 0.25);
        let left_up = trace_u(-GLYPH_W * 0.3, -GLYPH_H * 0.25);
        assert!(right_up < 0.3, "upper-right → u={right_up}");
        assert!(left_up > 0.7, "upper-left → u={left_up}");
    }

    #[test]
    fn trace_alpha_leaves_one_traveling_gap() {
        // The gap is a single `1−TRACE_DRAWN` window; edges are soft but the
        // middle is fully open/closed.
        let closed = |u: f32, p: f32| trace_alpha(u, p) < 0.05;
        // At phase 0, the gap sits at u ∈ (0.76, 1).
        assert!(closed(0.9, 0.0) && !closed(0.5, 0.0) && !closed(0.1, 0.0));
        // At phase 0.3 the gap has moved to u ∈ (0.06, 0.3).
        assert!(closed(0.15, 0.3) && !closed(0.9, 0.3));
    }

    #[test]
    fn embedded_s_paths_use_only_supported_commands() {
        // `flatten` implements the M/L/C/Z subset the stepper SVGs ship; an
        // asset refresh that introduces other commands must fail here, not at
        // runtime inside a OnceLock::get_or_init.
        for (name, d) in [("up", UP_D), ("down", DOWN_D)] {
            for c in d.bytes().filter(u8::is_ascii_alphabetic) {
                assert!(
                    matches!(c, b'M' | b'L' | b'C' | b'Z' | b'm' | b'l' | b'c' | b'z'),
                    "{name} path uses unsupported command {:?}",
                    c as char
                );
            }
        }
    }

    #[test]
    fn trace_gap_sweeps_smoothly_and_stays_a_quarter_open() {
        // The "incorrect quarters" regression guard: at every phase the open
        // window is ~1−TRACE_DRAWN of the ink and consecutive frames share
        // almost all of it — a scrambled u-field would flicker quarters in
        // and out instead of sliding one window around the S. (The open set
        // legitimately splits into 2+ regions while the gap straddles the
        // lemniscate's self-intersection, so this checks temporal smoothness
        // rather than connectivity.)
        let (up, down) = (half_mask(true), half_mask(false));
        let (w, h) = (up.w as usize, up.h as usize);
        let ms = up.h as f32 / GLYPH_H;
        let mut open = vec![false; w * h];
        let mut prev = vec![false; w * h];
        let ink_px = (0..w * h)
            .filter(|&i| up.alpha[i].max(down.alpha[i]) > 128)
            .count();
        let mut min_overlap = f32::MAX;
        for k in 0..64 {
            let phase = k as f32 / 64.0;
            let mut open_px = 0usize;
            for py in 0..h {
                let gy = (py as f32 + 0.5) / ms - GLYPH_H * 0.5;
                for px in 0..w {
                    let gx = (px as f32 + 0.5) / ms - GLYPH_W * 0.5;
                    let i = py * w + px;
                    let ink = up.alpha[i].max(down.alpha[i]) > 128;
                    open[i] = ink && trace_alpha(trace_u(gx, gy), phase) < 0.05;
                    if open[i] {
                        open_px += 1;
                    }
                }
            }
            let open_frac = open_px as f32 / ink_px as f32;
            assert!(
                (0.15..=0.35).contains(&open_frac),
                "phase {phase:.3}: gap is {open_frac:.2} of ink, not ~{:.2}",
                1.0 - TRACE_DRAWN
            );
            if k > 0 {
                let same = open
                    .iter()
                    .zip(prev.iter())
                    .filter(|(a, b)| **a && **b)
                    .count();
                min_overlap = min_overlap.min(same as f32 / open_px.max(1) as f32);
            }
            std::mem::swap(&mut open, &mut prev);
        }
        assert!(
            min_overlap >= 0.75,
            "gap jumps between frames: min overlap {min_overlap:.2}"
        );
    }
}
