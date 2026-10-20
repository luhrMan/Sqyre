//! Brand "S" spinner geometry: the Sqyre shield's S drawn from its real
//! upper/lower stroke outlines, animated as an alternating tip-to-tip
//! load. Each crescent slides a unit-length loaded window along its own
//! path: it fills inward from the S's outer tip toward the middle tip,
//! then drains the same way (the loaded tail slides inward and off the
//! tip) — while the other crescent runs the opposite half of the cycle,
//! so one finishing loading hands off to the other.
//!
//! The `d` strings are copied verbatim from
//! `crates/sqyre-app/assets/icons/sqyre-stepper-{up,down}.svg` (which
//! sqyre-app rasterizes via resvg); this crate draws the same curves
//! dependency-free. The affine below composes each SVG's nested group/path
//! transforms and re-centers the result on the shared viewBox
//! `40.5 20.25 46.75 84` so both halves compose the S at the origin.

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::OnceLock;

/// Glyph-space size of the S viewBox shared by both halves (points).
pub(crate) const GLYPH_W: f32 = 46.75;
pub(crate) const GLYPH_H: f32 = 84.0;

/// Spinner rate — full load/drain alternations per second. The X11 busy
/// tick advances `busy_phase` by `SWEEP_PER_SEC * tick` every 50 ms so egui
/// callers using `time * SWEEP_PER_SEC` stay in step.
pub(crate) const SWEEP_PER_SEC: f32 = 0.6;

/// Fraction of a square button face the S glyph occupies — shared by the
/// native raster spinner and egui busy chrome so both draw it the same size.
pub(crate) const FACE_FILL: f32 = 0.80;

/// The load field: for each glyph-box pixel, each half's normalized path
/// distance `v ∈ [0,1]` along that stroke from its *outer* tip (the S's
/// top tip for the upper crescent, bottom tip for the lower), so `v ≈ 1`
/// is the crescent's free tip in the middle of the S. Non-ink pixels stay
/// `f32::INFINITY` and are never drawn.
struct FillField {
    w: u32,
    h: u32,
    up: Vec<f32>,
    down: Vec<f32>,
}

/// The glyph-box load field, computed once.
fn fill_field() -> &'static FillField {
    static FIELD: OnceLock<FillField> = OnceLock::new();
    FIELD.get_or_init(build_fill_field)
}

fn build_fill_field() -> FillField {
    let (up, down) = (half_mask(true), half_mask(false));
    FillField {
        w: up.w,
        h: up.h,
        up: stroke_distances(up, true),
        down: stroke_distances(down, false),
    }
}

/// Normalized path distance along one crescent's ink from its outer tip:
/// Dijkstra over 8-connected ink pixels (`alpha > 128`), seeded at the
/// stroke's extreme ink row (topmost for the upper crescent — the S's top
/// tip — bottommost for the lower). Because distance follows the mask, the
/// wavefront sweeps the crescent's curve exactly, reaching its free middle
/// tip last.
fn stroke_distances(mask: &Mask, top: bool) -> Vec<f32> {
    let (w, h) = (mask.w as usize, mask.h as usize);
    let ink = |i: usize| mask.alpha[i] > 128;
    let rows: Box<dyn Iterator<Item = usize>> = if top {
        Box::new(0..h)
    } else {
        Box::new((0..h).rev())
    };
    let mut seed = None;
    for y in rows {
        let xs: Vec<usize> = (0..w).filter(|&x| ink(y * w + x)).collect();
        if !xs.is_empty() {
            seed = Some(y * w + xs[xs.len() / 2]);
            break;
        }
    }
    let seed = seed.expect("S half mask is empty");
    // Dijkstra with a fixed-point min-heap (f32 isn't Ord).
    const Q: f32 = 4096.0;
    let mut dist = vec![f32::INFINITY; w * h];
    dist[seed] = 0.0;
    let mut heap = BinaryHeap::new();
    heap.push(Reverse((0u64, seed)));
    while let Some(Reverse((qi, i))) = heap.pop() {
        let d = qi as f32 / Q;
        if d > dist[i] {
            continue;
        }
        let (x, y) = (i % w, i / w);
        for ny in y.saturating_sub(1)..=(y + 1).min(h - 1) {
            for nx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                let ni = ny * w + nx;
                if ni == i || !ink(ni) {
                    continue;
                }
                let nd = d + (nx as f32 - x as f32).hypot(ny as f32 - y as f32);
                if nd < dist[ni] {
                    dist[ni] = nd;
                    heap.push(Reverse(((nd * Q) as u64, ni)));
                }
            }
        }
    }
    let vmax = dist
        .iter()
        .copied()
        .filter(|d| d.is_finite())
        .fold(0.0f32, f32::max);
    debug_assert!(vmax > 0.0, "S half has no reachable ink");
    for d in &mut dist {
        *d /= vmax;
    }
    dist
}

/// Cell index of glyph-space point `(x,y)` in a `w`×`h` glyph-box field.
fn field_cell(w: u32, h: u32, x: f32, y: f32) -> usize {
    let s = h as f32 / GLYPH_H;
    let px = ((x + GLYPH_W * 0.5) * s - 0.5)
        .round()
        .clamp(0.0, w as f32 - 1.0) as usize;
    let py = ((y + GLYPH_H * 0.5) * s - 0.5)
        .round()
        .clamp(0.0, h as f32 - 1.0) as usize;
    py * w as usize + px
}

/// Draw weight of path distance `v` under the loaded window `(z−1, z)`
/// sliding over `v ∈ [0,2)` with a soft edge: `z < 1` draws the `v < z`
/// prefix — the stroke filling in from its outer tip — and `z > 1` draws
/// the `v > z−1` suffix — the loaded tail shrinking toward the inner tip,
/// which unloads last.
fn stroke_weight(v: f32, z: f32) -> f32 {
    const EDGE: f32 = 0.05;
    ((v - (z - 1.0)) / EDGE).min((z - v) / EDGE).clamp(0.0, 1.0)
}

/// Draw weight ∈ [0,1] at glyph-space `(x,y)` for sweep `phase`: the upper
/// crescent fills then drains (window position `z = 2·phase`) while the
/// lower runs the same cycle offset by half a period.
pub(crate) fn trace_weight(x: f32, y: f32, phase: f32) -> f32 {
    let f = fill_field();
    let i = field_cell(f.w, f.h, x, y);
    let z_up = (phase * 2.0).rem_euclid(2.0);
    let z_dn = (z_up + 1.0).rem_euclid(2.0);
    stroke_weight(f.up[i], z_up).max(stroke_weight(f.down[i], z_dn))
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

    /// Glyph coords of a stroke's outer tip (`v == 0`, the distance seed)
    /// or inner tip (max `v`) from a half's load field.
    fn tip_px(f: &FillField, half: &[f32], inner: bool) -> (f32, f32) {
        let idx = if inner {
            half.iter()
                .enumerate()
                .filter(|(_, v)| v.is_finite())
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(i, _)| i)
        } else {
            half.iter().position(|&v| v == 0.0)
        }
        .expect("stroke tip");
        let (x, y) = (idx % f.w as usize, idx / f.w as usize);
        let s = f.h as f32 / GLYPH_H;
        (
            (x as f32 + 0.5) / s - GLYPH_W * 0.5,
            (y as f32 + 0.5) / s - GLYPH_H * 0.5,
        )
    }

    #[test]
    fn loads_inward_from_the_outer_tips() {
        // "Tips inward": quarter-cycle — the upper crescent is filling
        // (outer tip loaded, middle tip still waiting) while the lower
        // drains (outer tip unloaded, middle tip still inked). The
        // three-quarter phase swaps the roles.
        let f = fill_field();
        let (uo, ui) = (tip_px(f, &f.up, false), tip_px(f, &f.up, true));
        let (dlo, dli) = (tip_px(f, &f.down, false), tip_px(f, &f.down, true));
        assert!(trace_weight(uo.0, uo.1, 0.25) > 0.5, "upper outer tip");
        assert!(
            trace_weight(ui.0, ui.1, 0.25) < 0.5,
            "upper inner tip early"
        );
        assert!(
            trace_weight(dlo.0, dlo.1, 0.25) < 0.5,
            "lower outer tip late"
        );
        assert!(
            trace_weight(dli.0, dli.1, 0.25) > 0.5,
            "lower inner tip early"
        );
        assert!(trace_weight(uo.0, uo.1, 0.75) < 0.5, "upper outer tip late");
        assert!(trace_weight(ui.0, ui.1, 0.75) > 0.5, "upper inner tip late");
        assert!(trace_weight(dlo.0, dlo.1, 0.75) > 0.5, "lower outer tip");
        assert!(trace_weight(dli.0, dli.1, 0.75) < 0.5, "lower inner tip");
    }

    #[test]
    fn each_handoff_leaves_one_stroke_full_and_the_other_empty() {
        // At the half-cycle boundary one crescent is fully loaded and the
        // other fully drained — the load hands off cleanly, both ways.
        let (up, down) = (half_mask(true), half_mask(false));
        let (w, h) = (up.w as usize, up.h as usize);
        let ms = up.h as f32 / GLYPH_H;
        for (phase, full_up) in [(0.5f32, true), (0.0f32, false)] {
            let (mut up_drawn, mut up_n, mut dn_drawn, mut dn_n) = (0usize, 0, 0, 0);
            for py in 0..h {
                let gy = (py as f32 + 0.5) / ms - GLYPH_H * 0.5;
                for px in 0..w {
                    let i = py * w + px;
                    let drawn =
                        trace_weight((px as f32 + 0.5) / ms - GLYPH_W * 0.5, gy, phase) > 0.5;
                    match (up.alpha[i] > 128, down.alpha[i] > 128) {
                        (true, false) => {
                            up_n += 1;
                            up_drawn += drawn as usize;
                        }
                        (false, true) => {
                            dn_n += 1;
                            dn_drawn += drawn as usize;
                        }
                        _ => {}
                    }
                }
            }
            let (full, empty) = if full_up {
                (up_drawn as f32 / up_n as f32, dn_drawn as f32 / dn_n as f32)
            } else {
                (dn_drawn as f32 / dn_n as f32, up_drawn as f32 / up_n as f32)
            };
            assert!(
                full > 0.9 && empty < 0.1,
                "phase {phase}: {full:.2} loaded / {empty:.2} drained"
            );
        }
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
    fn load_alternates_smoothly_and_stays_half_drawn() {
        // Fill/drain regression guard: at every phase roughly half the S's
        // ink is loaded (one stroke fills while the other drains), and
        // consecutive frames share almost all of it — a scrambled field
        // would flicker instead of sliding the load fronts.
        let (up, down) = (half_mask(true), half_mask(false));
        let (w, h) = (up.w as usize, up.h as usize);
        let ms = up.h as f32 / GLYPH_H;
        let mut drawn = vec![false; w * h];
        let mut prev = vec![false; w * h];
        let ink_px = (0..w * h)
            .filter(|&i| up.alpha[i].max(down.alpha[i]) > 128)
            .count();
        let mut min_overlap = f32::MAX;
        for k in 0..64 {
            let phase = k as f32 / 64.0;
            let mut drawn_px = 0usize;
            for py in 0..h {
                let gy = (py as f32 + 0.5) / ms - GLYPH_H * 0.5;
                for px in 0..w {
                    let gx = (px as f32 + 0.5) / ms - GLYPH_W * 0.5;
                    let i = py * w + px;
                    let ink = up.alpha[i].max(down.alpha[i]) > 128;
                    drawn[i] = ink && trace_weight(gx, gy, phase) > 0.5;
                    if drawn[i] {
                        drawn_px += 1;
                    }
                }
            }
            let drawn_frac = drawn_px as f32 / ink_px as f32;
            assert!(
                (0.30..=0.70).contains(&drawn_frac),
                "phase {phase:.3}: {drawn_frac:.2} of ink loaded, expected ~0.5"
            );
            if k > 0 {
                let same = drawn
                    .iter()
                    .zip(prev.iter())
                    .filter(|(a, b)| **a && **b)
                    .count();
                min_overlap = min_overlap.min(same as f32 / drawn_px.max(1) as f32);
            }
            std::mem::swap(&mut drawn, &mut prev);
        }
        assert!(
            min_overlap >= 0.80,
            "load jumps between frames: min overlap {min_overlap:.2}"
        );
    }
}
