use super::common::{run_matches, set_coord_outputs, sort_hits, DetectionCtx, DetectionHit};
use crate::backends::{DesktopRect, IconStore, ItemMeta};
use crate::run::{execute_macro_with, ExecDeps};
use crate::test_support::{
    fast_macro_of, FixedCollection, FixedResolver, RecordingBackend, RecordingCapturer,
    StopAfterSleeps, SEARCH_FIXED_AREA,
};
use crate::{lines_for, SharedActionLog};
use ::image::{Rgba, RgbaImage};
use sqyre_domain::{
    root_loop, Action, ActionId, ActionKind, CoordinateOutputs, CoordinateRef, LoopJumpMode, Macro,
    MatchGrouping, MatchOrder, MouseButton, PressState, RepeatMode, ScalarValue,
    WaitTilFoundConfig,
};
use sqyre_match::{search_blur_kernel, ImageBuf, DEFAULT_CLOSE_MATCHES_DISTANCE};
use sqyre_vision::get_cached_blurred_template;
use sqyre_vision::{ocr_text_matches, OcrMatch};
use std::collections::HashMap;
use std::path::PathBuf;

mod cache;
mod collection;
mod hits;
mod image;
mod ocr;
mod pixel;

fn full_desktop() -> DesktopRect {
    DesktopRect {
        x: 0,
        y: 0,
        w: 2000,
        h: 2000,
    }
}

fn run_search(
    macro_: &mut Macro,
    backend: &mut RecordingBackend,
    capturer: &mut RecordingCapturer,
    resolver: &dyn crate::backends::CoordinateResolver,
) {
    execute_macro_with(
        macro_,
        ExecDeps::new(backend).capturer(capturer).resolver(resolver),
    )
    .unwrap();
}

fn run_search_logged(
    macro_: &mut Macro,
    backend: &mut RecordingBackend,
    capturer: &mut RecordingCapturer,
    resolver: &dyn crate::backends::CoordinateResolver,
    logger: &SharedActionLog,
) {
    execute_macro_with(
        macro_,
        ExecDeps::new(backend)
            .capturer(capturer)
            .resolver(resolver)
            .logger(logger),
    )
    .unwrap();
}

fn run_search_ocr(
    macro_: &mut Macro,
    backend: &mut RecordingBackend,
    capturer: &mut RecordingCapturer,
    resolver: &dyn crate::backends::CoordinateResolver,
    ocr: &dyn crate::backends::OcrEngine,
) {
    execute_macro_with(
        macro_,
        ExecDeps::new(backend)
            .capturer(capturer)
            .resolver(resolver)
            .ocr(ocr),
    )
    .unwrap();
}

fn capturer_next(img: RgbaImage) -> RecordingCapturer {
    RecordingCapturer {
        next: Some(img),
        bounds: full_desktop(),
        ..Default::default()
    }
}

fn capturer_frames(queue: Vec<RgbaImage>, next: Option<RgbaImage>) -> RecordingCapturer {
    RecordingCapturer {
        queue,
        next,
        bounds: full_desktop(),
        ..Default::default()
    }
}

fn capturer_queue(queue: Vec<RgbaImage>) -> RecordingCapturer {
    RecordingCapturer {
        queue,
        bounds: full_desktop(),
        ..Default::default()
    }
}

fn wait_child(ms: i64) -> Action {
    sqyre_domain::test_action(ActionKind::Wait {
        time: ScalarValue::Int(ms),
    })
}

fn coords_xy(x: &str, y: &str) -> CoordinateOutputs {
    CoordinateOutputs {
        output_x_variable: x.into(),
        output_y_variable: y.into(),
    }
}

fn detection_branch(
    then_ms: Option<i64>,
    else_ms: Option<i64>,
    wait: WaitTilFoundConfig,
    coords: CoordinateOutputs,
) -> sqyre_domain::DetectionBranch {
    sqyre_domain::DetectionBranch {
        wait,
        coords,
        subactions: then_ms.map(wait_child).into_iter().collect(),
        else_actions: else_ms.map(wait_child).into_iter().collect(),
        ..Default::default()
    }
}

fn wait_until_found(seconds: impl Into<f64>, interval_ms: i32) -> WaitTilFoundConfig {
    WaitTilFoundConfig {
        repeat_mode: RepeatMode::WaitUntilFound,
        wait_til_found_seconds: seconds.into(),
        wait_til_found_interval_ms: interval_ms,
        max_iterations: 0,
    }
}

fn wait_while_found(seconds: impl Into<f64>, interval_ms: i32) -> WaitTilFoundConfig {
    WaitTilFoundConfig {
        repeat_mode: RepeatMode::WaitWhileFound,
        wait_til_found_seconds: seconds.into(),
        wait_til_found_interval_ms: interval_ms,
        max_iterations: 0,
    }
}

fn while_found(interval_ms: i32, max_iterations: i32) -> WaitTilFoundConfig {
    WaitTilFoundConfig {
        repeat_mode: RepeatMode::RepeatWhileFound,
        wait_til_found_seconds: 0.0,
        wait_til_found_interval_ms: interval_ms,
        max_iterations,
    }
}

fn until_found(interval_ms: i32, max_iterations: i32) -> WaitTilFoundConfig {
    WaitTilFoundConfig {
        repeat_mode: RepeatMode::RepeatUntilFound,
        wait_til_found_seconds: 0.0,
        wait_til_found_interval_ms: interval_ms,
        max_iterations,
    }
}

fn find_pixel_action(
    id: ActionId,
    color: &str,
    detection: sqyre_domain::DetectionBranch,
) -> Action {
    Action {
        id,
        kind: ActionKind::FindPixel(Box::new(sqyre_domain::FindPixelData {
            name: "red".into(),
            search_area: CoordinateRef("Prog~Box".into()),
            target_color: color.into(),
            detection,
            ..Default::default()
        })),
    }
}

fn ocr_action(
    id: ActionId,
    target: &str,
    output_variable: &str,
    detection: sqyre_domain::DetectionBranch,
) -> Action {
    Action {
        id,
        kind: ActionKind::Ocr(Box::new(sqyre_domain::OcrData {
            name: "read".into(),
            target: target.into(),
            search_area: CoordinateRef("prog~box".into()),
            output_variable: output_variable.into(),
            blur: 1,
            resize: 1.0,
            grayscale: true,
            detection,
            ..Default::default()
        })),
    }
}

fn assert_slept(backend: &RecordingBackend, ms: i64) {
    let key = format!("sleep:{ms}");
    assert!(backend.has(&key), "expected {key} in {:?}", backend.log);
}

fn assert_did_not_sleep(backend: &RecordingBackend, ms: i64) {
    let key = format!("sleep:{ms}");
    assert!(
        !backend.has(&key),
        "did not expect {key} in {:?}",
        backend.log
    );
}

fn count_sleeps(backend: &RecordingBackend, ms: i64) -> usize {
    let key = format!("sleep:{ms}");
    backend.count(&key)
}

struct MapIcons {
    paths: HashMap<String, Vec<PathBuf>>,
    masks: HashMap<String, PathBuf>,
    meta: HashMap<String, ItemMeta>,
}

fn item_meta(name: &str, stack_max: i32, cols: i32, rows: i32, tags: &[&str]) -> ItemMeta {
    ItemMeta {
        name: name.into(),
        stack_max,
        cols,
        rows,
        tags: tags.iter().map(ToString::to_string).collect(),
    }
}

/// `MapIcons` with empty masks keyed by catalog key → (paths, optional meta).
fn icons_for(items: Vec<(&str, Vec<PathBuf>, Option<ItemMeta>)>) -> MapIcons {
    MapIcons {
        paths: items
            .iter()
            .map(|(key, paths, _)| ((*key).to_string(), paths.clone()))
            .collect(),
        masks: HashMap::new(),
        meta: items
            .into_iter()
            .filter_map(|(key, _, meta)| meta.map(|m| (key.to_string(), m)))
            .collect(),
    }
}

impl IconStore for MapIcons {
    fn variant_paths(&self, target: &str) -> Vec<PathBuf> {
        self.paths.get(target).cloned().unwrap_or_default()
    }
    fn mask_path(&self, target: &str) -> Option<PathBuf> {
        self.masks.get(target).cloned()
    }
    fn item_meta(&self, target: &str) -> Option<ItemMeta> {
        self.meta.get(target).cloned()
    }
    fn catalog_item_refs(&self) -> Vec<(String, ItemMeta)> {
        let mut keys: Vec<_> = self.meta.keys().cloned().collect();
        keys.sort();
        keys.into_iter()
            .filter_map(|k| self.meta.get(&k).cloned().map(|m| (k, m)))
            .collect()
    }
}

fn named(name: &str, x: i32, y: i32, ox: i32, oy: i32) -> DetectionHit {
    DetectionHit::plain(x + ox, y + oy, name)
}

fn solid_rgba(w: u32, h: u32, rgb: [u8; 3]) -> RgbaImage {
    let mut img = RgbaImage::new(w, h);
    for p in img.pixels_mut() {
        *p = Rgba([rgb[0], rgb[1], rgb[2], 255]);
    }
    img
}

fn stamp_rgba(dst: &mut RgbaImage, src: &RgbaImage, x: u32, y: u32) {
    for (sx, sy, p) in src.enumerate_pixels() {
        let dx = x + sx;
        let dy = y + sy;
        if dx < dst.width() && dy < dst.height() {
            dst.put_pixel(dx, dy, *p);
        }
    }
}

/// Non-uniform template so CCOEFF_NORMED has variance to score (survives blur=5).
fn patterned_rgba(w: u32, h: u32, seed: u8) -> RgbaImage {
    let mut img = RgbaImage::new(w, h);
    for (x, y, p) in img.enumerate_pixels_mut() {
        *p = Rgba([
            (x * 17 + y * 9 + seed as u32) as u8,
            (x * 3 + y * 29 + 40) as u8,
            (255u32.wrapping_sub(x * 11).wrapping_add(seed as u32)) as u8,
            255,
        ]);
    }
    img
}

/// 3-channel patterned template matching [`patterned_rgba`]'s seed-0 fill.
fn patterned_imagebuf(w: usize, h: usize) -> ImageBuf {
    let mut img = ImageBuf::new(w, h, 3, 40);
    for y in 0..h {
        for x in 0..w {
            let o = img.pixel_offset(x, y);
            img.data[o] = (x * 17 + y * 9) as u8;
            img.data[o + 1] = (x * 3 + y * 29) as u8;
            img.data[o + 2] = (255 - x * 11) as u8;
        }
    }
    img
}

fn bag_2x2_resolver() -> FixedResolver {
    FixedResolver {
        point: (0, 0),
        area: (0, 0, 40, 40),
        grid: None,
        collections: Some(HashMap::from([(
            "bag".into(),
            FixedCollection {
                rows: 2,
                cols: 2,
                bounds: (0, 0, 40, 40),
            },
        )])),
        atlas_members: None,
    }
}
