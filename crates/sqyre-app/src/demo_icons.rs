//! In-memory placeholder icons for WASM demo items / collections.
//!
//! Native builds leave this store empty unless tests register icons. On WASM,
//! [`crate::wasm_demo_seed`] fills it so the editor can show tiles without a filesystem.

// Registration helpers are only called from the wasm seed / unit tests.
#![cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]

use image::{Rgba, RgbaImage};
use sqyre_persist::images_path;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const ITEM_SIZE: u32 = 48;
const COLLECTION_W: u32 = 160;
const COLLECTION_H: u32 = 96;

#[derive(Clone)]
pub struct DemoRgba {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

fn store() -> &'static Mutex<HashMap<PathBuf, DemoRgba>> {
    static STORE: OnceLock<Mutex<HashMap<PathBuf, DemoRgba>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn item_index() -> &'static Mutex<HashMap<String, Vec<PathBuf>>> {
    static INDEX: OnceLock<Mutex<HashMap<String, Vec<PathBuf>>>> = OnceLock::new();
    INDEX.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Serializes tests that mutate the global demo-icon store.
fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Run `f` while holding the demo-icon store lock (tests that seed / clear).
pub(crate) fn with_exclusive<R>(f: impl FnOnce() -> R) -> R {
    let _guard = test_lock()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    f()
}

/// Drop all registered placeholders (tests / re-seed).
pub fn clear() {
    store().lock().expect("demo icon store").clear();
    item_index().lock().expect("demo item index").clear();
}

pub fn item_variant_path(program: &str, item: &str, variant: &str) -> PathBuf {
    let dir = images_path().join("icons").join(program);
    if variant.is_empty() {
        dir.join(format!("{item}.png"))
    } else {
        dir.join(format!("{item}~{variant}.png"))
    }
}

pub fn item_icon_path(program: &str, item: &str) -> PathBuf {
    item_variant_path(program, item, "")
}

pub fn collection_icon_path(program: &str, collection: &str) -> PathBuf {
    images_path()
        .join("Collections")
        .join(program)
        .join(format!("{collection}.png"))
}

fn push_item_path(target: String, path: PathBuf) {
    let mut index = item_index().lock().expect("demo item index");
    let entry = index.entry(target).or_default();
    if !entry.contains(&path) {
        entry.push(path);
    }
}

/// Register the primary (`{item}.png`) procedural icon.
pub fn register_item(program: &str, item: &str, accent: [u8; 3], pattern: usize) {
    register_item_variant(program, item, "", accent, pattern);
}

/// Register an additional `{item}~{variant}.png` placeholder.
pub fn register_item_variant(
    program: &str,
    item: &str,
    variant: &str,
    accent: [u8; 3],
    pattern: usize,
) {
    let path = item_variant_path(program, item, variant);
    let target = format!("{program}~{item}");
    let label = if variant.is_empty() { item } else { variant };
    let img = paint_item_tile(accent, pattern, label);
    push_item_path(target, path.clone());
    store()
        .lock()
        .expect("demo icon store")
        .insert(path, img);
}

/// Register a procedural collection board image.
pub fn register_collection(program: &str, collection: &str, accent: [u8; 3], pattern: usize) {
    let path = collection_icon_path(program, collection);
    let img = paint_collection_board(accent, pattern);
    store()
        .lock()
        .expect("demo icon store")
        .insert(path, img);
}

/// All demo variant paths for `program~item` (primary first).
pub fn variant_paths_for_target(target: &str) -> Vec<PathBuf> {
    item_index()
        .lock()
        .expect("demo item index")
        .get(target)
        .cloned()
        .unwrap_or_default()
}

/// Path used for `program~item` when a demo icon was registered (primary / first).
pub fn path_for_item_target(target: &str) -> Option<PathBuf> {
    variant_paths_for_target(target).into_iter().next()
}

/// Disk paths plus any in-memory demo variants for `target`.
pub fn merged_variant_paths(catalog: &sqyre_persist::ProgramCatalog, target: &str) -> Vec<PathBuf> {
    let mut paths = catalog.variant_paths(target);
    for p in variant_paths_for_target(target) {
        if !paths.contains(&p) {
            paths.push(p);
        }
    }
    paths.sort();
    paths
}

pub fn get(path: &Path) -> Option<DemoRgba> {
    store().lock().expect("demo icon store").get(path).cloned()
}

pub fn contains(path: &Path) -> bool {
    store().lock().expect("demo icon store").contains_key(path)
}

fn paint_item_tile(accent: [u8; 3], pattern: usize, label: &str) -> DemoRgba {
    let mut img = RgbaImage::from_pixel(ITEM_SIZE, ITEM_SIZE, Rgba([28, 30, 34, 255]));
    let margin = 4u32;
    let [ar, ag, ab] = accent;
    let fill = Rgba([ar, ag, ab, 255]);
    let dark = Rgba([
        ar.saturating_mul(2) / 3,
        ag.saturating_mul(2) / 3,
        ab.saturating_mul(2) / 3,
        255,
    ]);
    let light = Rgba([
        ar.saturating_add(40).min(255),
        ag.saturating_add(40).min(255),
        ab.saturating_add(40).min(255),
        255,
    ]);
    let border = Rgba([240, 240, 245, 220]);

    for y in margin..ITEM_SIZE - margin {
        for x in margin..ITEM_SIZE - margin {
            let lx = x - margin;
            let ly = y - margin;
            let inner = ITEM_SIZE - 2 * margin;
            let pix = match pattern % 4 {
                0 => {
                    // Diagonal stripes
                    if (lx + ly) / 4 % 2 == 0 {
                        fill
                    } else {
                        dark
                    }
                }
                1 => {
                    // Checker
                    if (lx / 6 + ly / 6) % 2 == 0 {
                        fill
                    } else {
                        dark
                    }
                }
                2 => {
                    // Center diamond
                    let cx = inner as i32 / 2;
                    let cy = inner as i32 / 2;
                    let d = (lx as i32 - cx).abs() + (ly as i32 - cy).abs();
                    if d < inner as i32 / 3 {
                        light
                    } else if d < inner as i32 / 2 {
                        fill
                    } else {
                        dark
                    }
                }
                _ => {
                    // Dots
                    let on_dot = (lx % 8 < 3) && (ly % 8 < 3);
                    if on_dot {
                        light
                    } else {
                        fill
                    }
                }
            };
            img.put_pixel(x, y, pix);
        }
    }

    // Border ring
    for i in margin..ITEM_SIZE - margin {
        img.put_pixel(i, margin, border);
        img.put_pixel(i, ITEM_SIZE - margin - 1, border);
        img.put_pixel(margin, i, border);
        img.put_pixel(ITEM_SIZE - margin - 1, i, border);
    }

    // Tiny initial glyph (first alphanumeric char)
    if let Some(ch) = label
        .chars()
        .find(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
    {
        blit_glyph(&mut img, 18, 17, ch, Rgba([255, 255, 255, 240]));
    }

    DemoRgba {
        rgba: img.into_raw(),
        width: ITEM_SIZE,
        height: ITEM_SIZE,
    }
}

fn paint_collection_board(accent: [u8; 3], pattern: usize) -> DemoRgba {
    let mut img = RgbaImage::from_pixel(COLLECTION_W, COLLECTION_H, Rgba([22, 24, 28, 255]));
    let [ar, ag, ab] = accent;
    let cols = 6u32;
    let rows = 4u32;
    let pad = 6u32;
    let gap = 3u32;
    let cell_w = (COLLECTION_W - pad * 2 - gap * (cols - 1)) / cols;
    let cell_h = (COLLECTION_H - pad * 2 - gap * (rows - 1)) / rows;

    for row in 0..rows {
        for col in 0..cols {
            let x0 = pad + col * (cell_w + gap);
            let y0 = pad + row * (cell_h + gap);
            let shade = ((row + col + pattern as u32) % 3) as u8;
            let cell = Rgba([
                ar.saturating_sub(shade * 25),
                ag.saturating_sub(shade * 20),
                ab.saturating_sub(shade * 15),
                255,
            ]);
            for y in y0..y0 + cell_h {
                for x in x0..x0 + cell_w {
                    let edge = x == x0 || y == y0 || x + 1 == x0 + cell_w || y + 1 == y0 + cell_h;
                    img.put_pixel(
                        x,
                        y,
                        if edge {
                            Rgba([255, 255, 255, 180])
                        } else {
                            cell
                        },
                    );
                }
            }
        }
    }

    DemoRgba {
        rgba: img.into_raw(),
        width: COLLECTION_W,
        height: COLLECTION_H,
    }
}

/// Ultra-minimal 5×7 glyphs for A–Z / 0–9 (MSB = left).
fn glyph_bits(ch: char) -> Option<[u8; 7]> {
    Some(match ch {
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10001, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10101, 0b11011, 0b10001],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111],
        '3' => [0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        _ => return None,
    })
}

fn blit_glyph(img: &mut RgbaImage, ox: u32, oy: u32, ch: char, color: Rgba<u8>) {
    let Some(rows) = glyph_bits(ch) else {
        return;
    };
    for (row, bits) in rows.iter().enumerate() {
        for col in 0..5u32 {
            if bits & (1 << (4 - col)) != 0 {
                let x = ox + col * 2;
                let y = oy + row as u32 * 2;
                for dy in 0..2u32 {
                    for dx in 0..2u32 {
                        if x + dx < img.width() && y + dy < img.height() {
                            img.put_pixel(x + dx, y + dy, color);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_lookup_item() {
        with_exclusive(|| {
            clear();
            register_item("Demo", "Sword", [200, 100, 40], 0);
            let path = path_for_item_target("Demo~Sword").expect("path");
            assert_eq!(path, item_icon_path("Demo", "Sword"));
            let img = get(&path).expect("rgba");
            assert_eq!(img.width, ITEM_SIZE);
            assert_eq!(img.rgba.len(), (ITEM_SIZE * ITEM_SIZE * 4) as usize);
            clear();
        });
    }

    #[test]
    fn register_item_variants() {
        with_exclusive(|| {
            clear();
            register_item("Demo", "Potion", [180, 40, 40], 0);
            register_item_variant("Demo", "Potion", "glow", [220, 80, 40], 1);
            register_item_variant("Demo", "Potion", "empty", [80, 80, 90], 2);
            let paths = variant_paths_for_target("Demo~Potion");
            assert_eq!(paths.len(), 3);
            assert!(paths[0].ends_with("Potion.png"));
            assert!(paths
                .iter()
                .any(|p| p.to_string_lossy().contains("Potion~glow.png")));
            assert!(contains(&item_variant_path("Demo", "Potion", "glow")));
            clear();
        });
    }

    #[test]
    fn register_collection_by_path() {
        with_exclusive(|| {
            clear();
            register_collection("Demo", "Bag", [40, 120, 200], 1);
            let path = collection_icon_path("Demo", "Bag");
            assert!(contains(&path));
            let img = get(&path).expect("rgba");
            assert_eq!((img.width, img.height), (COLLECTION_W, COLLECTION_H));
            clear();
        });
    }
}
