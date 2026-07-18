use super::common::set_coord_outputs;
use super::image::{run_matches, sort_points, NamedPoint};
use super::ocr::ocr_target_matched;
use crate::backends::{DesktopRect, IconStore, ItemMeta, RecordingBackend, RecordingCapturer};
use crate::run::{execute_macro_with, ExecDeps};
use crate::SharedActionLog;
use image::{Rgba, RgbaImage};
use sqyre_domain::{
    root_loop, Action, ActionId, ActionKind, CoordinateOutputs, CoordinateRef, Macro, MatchOrder,
    ScalarValue, WaitTilFoundConfig,
};
use sqyre_match::{search_blur_kernel, ImageBuf, Point, DEFAULT_CLOSE_MATCHES_DISTANCE};
use sqyre_vision::get_cached_blurred_template;
use std::collections::HashMap;
use std::path::PathBuf;

struct FixedArea;

impl crate::backends::CoordinateResolver for FixedArea {
    fn resolve_point(
        &self,
        _r: &CoordinateRef,
        _macro_: &Macro,
    ) -> std::result::Result<(i32, i32), String> {
        Ok((0, 0))
    }
    fn resolve_search_area(
        &self,
        _r: &CoordinateRef,
        _macro_: &Macro,
    ) -> std::result::Result<(i32, i32, i32, i32), String> {
        Ok((100, 200, 110, 210))
    }
}

struct MapIcons {
    paths: HashMap<String, Vec<PathBuf>>,
    masks: HashMap<String, PathBuf>,
    meta: HashMap<String, ItemMeta>,
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
}

fn named(name: &str, x: i32, y: i32, ox: i32, oy: i32) -> NamedPoint {
    NamedPoint {
        point: Point { x, y },
        origin: DesktopRect {
            x: ox,
            y: oy,
            w: 10,
            h: 10,
        },
        meta: None,
        tmpl_w: 1,
        tmpl_h: 1,
        name: name.into(),
    }
}

#[test]
fn sort_points_uses_row_band_then_x() {
    let mut pts = vec![
        named("b", 20, 10, 0, 0),
        named("a", 5, 12, 0, 0), // same band (abs dy <= 5), lower x → first
        named("c", 1, 30, 0, 0), // next row
    ];
    sort_points(&mut pts, &MatchOrder::default());
    assert_eq!(
        pts.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
        vec!["a", "b", "c"]
    );
}

#[test]
fn sort_points_respects_match_order() {
    let mut pts = vec![
        named("a", 5, 10, 0, 0),
        named("b", 20, 12, 0, 0),
        named("c", 1, 30, 0, 0),
    ];
    sort_points(
        &mut pts,
        &MatchOrder {
            grouping: "row".into(),
            horizontal: "right_to_left".into(),
            vertical: "top_to_bottom".into(),
        },
    );
    assert_eq!(
        pts.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
        vec!["b", "a", "c"]
    );

    let mut pts = vec![
        named("a", 10, 5, 0, 0),
        named("b", 12, 20, 0, 0),
        named("c", 30, 1, 0, 0),
    ];
    sort_points(
        &mut pts,
        &MatchOrder {
            grouping: "column".into(),
            horizontal: "left_to_right".into(),
            vertical: "top_to_bottom".into(),
        },
    );
    assert_eq!(
        pts.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
        vec!["a", "b", "c"]
    );
}

#[test]
fn ocr_empty_target_always_matches() {
    assert!(ocr_target_matched("", ""));
    assert!(ocr_target_matched("", "anything"));
    assert!(ocr_target_matched("Hi", "say Hi there"));
    assert!(!ocr_target_matched("Hi", "hello"));
}

#[test]
fn image_search_caches_blurred_templates() {
    sqyre_vision::with_search_cache_test_lock(|| {
        sqyre_vision::reset_search_cache_for_testing();
        let dir = tempfile::tempdir().unwrap();
        let tmpl_path = dir.path().join("tmpl.png");
        let tmpl = RgbaImage::from_pixel(8, 8, Rgba([255, 0, 0, 255]));
        tmpl.save(&tmpl_path).unwrap();

        let icons = MapIcons {
            paths: HashMap::from([("Prog~Item".into(), vec![tmpl_path.clone()])]),
            masks: HashMap::new(),
            meta: HashMap::new(),
        };
        let search = RgbaImage::from_pixel(16, 16, Rgba([0, 255, 0, 255]));
        let mut backend = RecordingBackend::default();
        let mut capturer = RecordingCapturer {
            next: Some(search),
            bounds: DesktopRect {
                x: 0,
                y: 0,
                w: 2000,
                h: 2000,
            },
            ..Default::default()
        };
        let resolver = FixedArea;
        let close_matches = 17;
        let search_id = ActionId::new();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.keyboard_delay = 0;
        macro_.mouse_delay = 0;
        macro_.root = root_loop(vec![Action {
            id: search_id,
            kind: ActionKind::ImageSearch {
                name: "find".into(),
                targets: vec!["Prog~Item".into()],
                search_area: CoordinateRef("Prog~Box".into()),
                tolerance: 0.99,
                blur: 5,
                detection: Default::default(),
            },
        }]);

        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: Some(&mut capturer),
                close_matches_distance: close_matches,
                resolver: Some(&resolver),
                icons: Some(&icons),
                macros: None,
                continue_waiter: None,
                window_focuser: None,
                ocr: None,
                stop_flag: None,
                logger: None,
                highlighter: None,
                runtime_vars: None,
                variables_dir: None,
            },
        )
        .unwrap();

        let kernel = search_blur_kernel(5);
        let first = get_cached_blurred_template(&tmpl_path, kernel).unwrap();
        let second = get_cached_blurred_template(&tmpl_path, kernel).unwrap();
        assert!(std::sync::Arc::ptr_eq(&first, &second));
    });
}

#[test]
fn set_coord_outputs_writes_variables() {
    let mut macro_ = Macro::new("t", 0, vec![]);
    let coords = CoordinateOutputs {
        output_x_variable: "fx".into(),
        output_y_variable: "fy".into(),
    };
    set_coord_outputs(&mut macro_, &coords, 11, 22);
    assert_eq!(
        macro_.variables.get("fx").map(|v| v.as_display()),
        Some("11".into())
    );
    assert_eq!(
        macro_.variables.get("fy").map(|v| v.as_display()),
        Some("22".into())
    );
}

#[test]
fn find_pixel_uses_collection_cell_search_area() {
    let mut img = RgbaImage::new(4, 4);
    for p in img.pixels_mut() {
        *p = Rgba([0, 0, 0, 255]);
    }
    img.put_pixel(1, 1, Rgba([0, 255, 0, 255]));

    struct CollectionOnly;
    impl crate::backends::CoordinateResolver for CollectionOnly {
        fn resolve_point(
            &self,
            _r: &CoordinateRef,
            _macro_: &Macro,
        ) -> std::result::Result<(i32, i32), String> {
            Err("point".into())
        }
        fn resolve_search_area(
            &self,
            r: &CoordinateRef,
            _macro_: &Macro,
        ) -> std::result::Result<(i32, i32, i32, i32), String> {
            assert!(r.is_collection(), "expected collection ref, got {r:?}");
            assert_eq!(r.as_str(), "Demo~bag@1,2-1,2");
            Ok((50, 60, 54, 64))
        }
    }

    let mut backend = RecordingBackend::default();
    let mut capturer = RecordingCapturer {
        next: Some(img),
        bounds: DesktopRect {
            x: 0,
            y: 0,
            w: 2000,
            h: 2000,
        },
        ..Default::default()
    };
    let resolver = CollectionOnly;
    let logger = SharedActionLog::new();
    let find_id = ActionId::new();
    let mut macro_ = Macro::new("t", 0, vec![]);
    macro_.keyboard_delay = 0;
    macro_.mouse_delay = 0;
    macro_.root = root_loop(vec![Action {
        id: find_id,
        kind: ActionKind::FindPixel {
            name: "green".into(),
            search_area: CoordinateRef::collection("Demo", "bag", 1, 2, 1, 2),
            target_color: "#00ff00".into(),
            color_tolerance: 0,
            detection: sqyre_domain::DetectionBranch {
                coords: CoordinateOutputs {
                    output_x_variable: "foundX".into(),
                    output_y_variable: "foundY".into(),
                },
                ..Default::default()
            },
        },
    }]);

    execute_macro_with(
        &mut macro_,
        ExecDeps {
            automation: &mut backend,
            capturer: Some(&mut capturer),
            close_matches_distance: 0,
            resolver: Some(&resolver),
            icons: None,
            macros: None,
            continue_waiter: None,
            window_focuser: None,
            ocr: None,
            stop_flag: None,
            logger: Some(&logger),
            highlighter: None,
            runtime_vars: None,
            variables_dir: None,
        },
    )
    .unwrap();

    assert_eq!(
        macro_.variables.get("foundX").map(|v| v.as_display()),
        Some("51".into())
    );
    assert_eq!(
        macro_.variables.get("foundY").map(|v| v.as_display()),
        Some("61".into())
    );
    assert!(
        capturer.log.iter().any(|e| e == "rect:50,60,4,4"),
        "{:?}",
        capturer.log
    );
}

#[test]
fn find_pixel_sets_coords_and_logs() {
    let mut img = RgbaImage::new(10, 10);
    for p in img.pixels_mut() {
        *p = Rgba([0, 0, 0, 255]);
    }
    img.put_pixel(3, 5, Rgba([255, 0, 0, 255]));

    let mut backend = RecordingBackend::default();
    let mut capturer = RecordingCapturer {
        next: Some(img),
        bounds: DesktopRect {
            x: 0,
            y: 0,
            w: 2000,
            h: 2000,
        },
        ..Default::default()
    };
    let resolver = FixedArea;
    let logger = SharedActionLog::new();
    let find_id = ActionId::new();
    let mut macro_ = Macro::new("t", 0, vec![]);
    macro_.keyboard_delay = 0;
    macro_.mouse_delay = 0;
    macro_.root = root_loop(vec![Action {
        id: find_id,
        kind: ActionKind::FindPixel {
            name: "red".into(),
            search_area: CoordinateRef("Prog~Box".into()),
            target_color: "#ff0000".into(),
            color_tolerance: 0,
            detection: sqyre_domain::DetectionBranch {
                coords: CoordinateOutputs {
                    output_x_variable: "foundX".into(),
                    output_y_variable: "foundY".into(),
                },
                ..Default::default()
            },
        },
    }]);

    execute_macro_with(
        &mut macro_,
        ExecDeps {
            automation: &mut backend,
            capturer: Some(&mut capturer),
            close_matches_distance: 0,
            resolver: Some(&resolver),
            icons: None,
            macros: None,
            continue_waiter: None,
            window_focuser: None,
            ocr: None,
            stop_flag: None,
            logger: Some(&logger),
            highlighter: None,
            runtime_vars: None,
            variables_dir: None,
        },
    )
    .unwrap();

    assert_eq!(
        macro_.variables.get("foundX").map(|v| v.as_display()),
        Some("103".into()) // 100 origin + 3
    );
    assert_eq!(
        macro_.variables.get("foundY").map(|v| v.as_display()),
        Some("205".into()) // 200 origin + 5
    );
    let lines = logger.lines_for(find_id);
    assert!(
        lines.iter().any(|l| l.contains("found matching pixel")),
        "{lines:?}"
    );
    assert!(capturer.log.iter().any(|e| e.starts_with("rect:")));
}

#[test]
fn find_pixel_not_found_logs() {
    let img = RgbaImage::from_pixel(4, 4, Rgba([0, 0, 255, 255]));
    let mut backend = RecordingBackend::default();
    let mut capturer = RecordingCapturer {
        next: Some(img),
        bounds: DesktopRect {
            x: 0,
            y: 0,
            w: 2000,
            h: 2000,
        },
        ..Default::default()
    };
    let resolver = FixedArea;
    let logger = SharedActionLog::new();
    let find_id = ActionId::new();
    let mut macro_ = Macro::new("t", 0, vec![]);
    macro_.keyboard_delay = 0;
    macro_.mouse_delay = 0;
    macro_.root = root_loop(vec![Action {
        id: find_id,
        kind: ActionKind::FindPixel {
            name: "red".into(),
            search_area: CoordinateRef("Prog~Box".into()),
            target_color: "#ff0000".into(),
            color_tolerance: 0,
            detection: Default::default(),
        },
    }]);
    execute_macro_with(
        &mut macro_,
        ExecDeps {
            automation: &mut backend,
            capturer: Some(&mut capturer),
            close_matches_distance: 0,
            resolver: Some(&resolver),
            icons: None,
            macros: None,
            continue_waiter: None,
            window_focuser: None,
            ocr: None,
            stop_flag: None,
            logger: Some(&logger),
            highlighter: None,
            runtime_vars: None,
            variables_dir: None,
        },
    )
    .unwrap();
    let lines = logger.lines_for(find_id);
    assert!(
        lines.iter().any(|l| l.contains("pixel not found")),
        "{lines:?}"
    );
}

#[test]
fn find_pixel_runs_branch_when_found() {
    let mut img = RgbaImage::new(10, 10);
    for p in img.pixels_mut() {
        *p = Rgba([0, 0, 0, 255]);
    }
    img.put_pixel(3, 5, Rgba([255, 0, 0, 255]));

    let mut backend = RecordingBackend::default();
    let mut capturer = RecordingCapturer {
        next: Some(img),
        bounds: DesktopRect {
            x: 0,
            y: 0,
            w: 2000,
            h: 2000,
        },
        ..Default::default()
    };
    let resolver = FixedArea;
    let mut macro_ = Macro::new("t", 0, vec![]);
    macro_.keyboard_delay = 0;
    macro_.mouse_delay = 0;
    macro_.root = root_loop(vec![Action {
        id: ActionId::new(),
        kind: ActionKind::FindPixel {
            name: "red".into(),
            search_area: CoordinateRef("Prog~Box".into()),
            target_color: "#ff0000".into(),
            color_tolerance: 0,
            detection: sqyre_domain::DetectionBranch {
                subactions: vec![sqyre_domain::test_action(ActionKind::Wait {
                    time: ScalarValue::Int(21),
                })],
                ..Default::default()
            },
        },
    }]);

    execute_macro_with(
        &mut macro_,
        ExecDeps {
            automation: &mut backend,
            capturer: Some(&mut capturer),
            close_matches_distance: 0,
            resolver: Some(&resolver),
            icons: None,
            macros: None,
            continue_waiter: None,
            window_focuser: None,
            ocr: None,
            stop_flag: None,
            logger: None,
            highlighter: None,
            runtime_vars: None,
            variables_dir: None,
        },
    )
    .unwrap();

    assert!(
        backend.log.iter().any(|e| e == "sleep:21"),
        "expected child wait on FindPixel match: {:?}",
        backend.log
    );
}

#[test]
fn find_pixel_no_find_runs_branch_when_flag_set() {
    let img = RgbaImage::from_pixel(4, 4, Rgba([0, 0, 255, 255]));
    let mut backend = RecordingBackend::default();
    let mut capturer = RecordingCapturer {
        next: Some(img),
        bounds: DesktopRect {
            x: 0,
            y: 0,
            w: 2000,
            h: 2000,
        },
        ..Default::default()
    };
    let resolver = FixedArea;
    let mut macro_ = Macro::new("t", 0, vec![]);
    macro_.keyboard_delay = 0;
    macro_.mouse_delay = 0;
    macro_.variables.set("foundX", ScalarValue::Int(1));
    macro_.variables.set("foundY", ScalarValue::Int(2));
    macro_.root = root_loop(vec![Action {
        id: ActionId::new(),
        kind: ActionKind::FindPixel {
            name: "red".into(),
            search_area: CoordinateRef("Prog~Box".into()),
            target_color: "#ff0000".into(),
            color_tolerance: 0,
            detection: sqyre_domain::DetectionBranch {
                coords: CoordinateOutputs {
                    output_x_variable: "foundX".into(),
                    output_y_variable: "foundY".into(),
                },
                run_branch_on_no_find: true,
                subactions: vec![sqyre_domain::test_action(ActionKind::Wait {
                    time: ScalarValue::Int(15),
                })],
                ..Default::default()
            },
        },
    }]);

    execute_macro_with(
        &mut macro_,
        ExecDeps {
            automation: &mut backend,
            capturer: Some(&mut capturer),
            close_matches_distance: 0,
            resolver: Some(&resolver),
            icons: None,
            macros: None,
            continue_waiter: None,
            window_focuser: None,
            ocr: None,
            stop_flag: None,
            logger: None,
            highlighter: None,
            runtime_vars: None,
            variables_dir: None,
        },
    )
    .unwrap();

    assert!(
        backend.log.iter().any(|e| e == "sleep:15"),
        "expected child wait on FindPixel no-find: {:?}",
        backend.log
    );
    assert!(macro_.variables.get("foundX").is_none());
    assert!(macro_.variables.get("foundY").is_none());
}

#[test]
fn find_pixel_skips_branch_when_missing() {
    let img = RgbaImage::from_pixel(4, 4, Rgba([0, 0, 255, 255]));
    let mut backend = RecordingBackend::default();
    let mut capturer = RecordingCapturer {
        next: Some(img),
        bounds: DesktopRect {
            x: 0,
            y: 0,
            w: 2000,
            h: 2000,
        },
        ..Default::default()
    };
    let resolver = FixedArea;
    let mut macro_ = Macro::new("t", 0, vec![]);
    macro_.keyboard_delay = 0;
    macro_.mouse_delay = 0;
    macro_.root = root_loop(vec![Action {
        id: ActionId::new(),
        kind: ActionKind::FindPixel {
            name: "red".into(),
            search_area: CoordinateRef("Prog~Box".into()),
            target_color: "#ff0000".into(),
            color_tolerance: 0,
            detection: sqyre_domain::DetectionBranch {
                subactions: vec![sqyre_domain::test_action(ActionKind::Wait {
                    time: ScalarValue::Int(21),
                })],
                ..Default::default()
            },
        },
    }]);

    execute_macro_with(
        &mut macro_,
        ExecDeps {
            automation: &mut backend,
            capturer: Some(&mut capturer),
            close_matches_distance: 0,
            resolver: Some(&resolver),
            icons: None,
            macros: None,
            continue_waiter: None,
            window_focuser: None,
            ocr: None,
            stop_flag: None,
            logger: None,
            highlighter: None,
            runtime_vars: None,
            variables_dir: None,
        },
    )
    .unwrap();

    assert!(
        !backend.log.iter().any(|e| e == "sleep:21"),
        "child should not run on FindPixel miss: {:?}",
        backend.log
    );
}

#[test]
fn image_search_no_find_runs_branch() {
    let img = RgbaImage::from_pixel(8, 8, Rgba([10, 20, 30, 255]));
    let dir = tempfile::tempdir().unwrap();
    let tmpl_path = dir.path().join("tmpl.png");
    // Distinct template that will not match the solid search image well at high threshold.
    let mut tmpl = RgbaImage::new(4, 4);
    for (i, p) in tmpl.pixels_mut().enumerate() {
        *p = Rgba([(i as u8).wrapping_mul(37), 200, 50, 255]);
    }
    tmpl.save(&tmpl_path).unwrap();

    let icons = MapIcons {
        paths: HashMap::from([("Prog~Item".into(), vec![tmpl_path])]),
        masks: HashMap::new(),
        meta: HashMap::from([(
            "Prog~Item".into(),
            ItemMeta {
                name: "Item".into(),
                stack_max: 99,
                cols: 2,
                rows: 2,
            },
        )]),
    };
    let mut backend = RecordingBackend::default();
    let mut capturer = RecordingCapturer {
        next: Some(img),
        bounds: DesktopRect {
            x: 0,
            y: 0,
            w: 2000,
            h: 2000,
        },
        ..Default::default()
    };
    let resolver = FixedArea;
    let logger = SharedActionLog::new();
    let search_id = ActionId::new();
    let mut macro_ = Macro::new("t", 0, vec![]);
    macro_.keyboard_delay = 0;
    macro_.mouse_delay = 0;
    macro_.root = root_loop(vec![Action {
        id: search_id,
        kind: ActionKind::ImageSearch {
            name: "find".into(),
            targets: vec!["Prog~Item".into()],
            search_area: CoordinateRef("Prog~Box".into()),
            tolerance: 0.99,
            blur: 0,
            detection: sqyre_domain::DetectionBranch {
                run_branch_on_no_find: true,
                subactions: vec![sqyre_domain::test_action(ActionKind::Wait {
                    time: ScalarValue::Int(13),
                })],
                ..Default::default()
            },
        },
    }]);

    execute_macro_with(
        &mut macro_,
        ExecDeps {
            automation: &mut backend,
            capturer: Some(&mut capturer),
            close_matches_distance: 0,
            resolver: Some(&resolver),
            icons: Some(&icons),
            macros: None,
            continue_waiter: None,
            window_focuser: None,
            ocr: None,
            stop_flag: None,
            logger: Some(&logger),
            highlighter: None,
            runtime_vars: None,
            variables_dir: None,
        },
    )
    .unwrap();

    assert!(backend.log.iter().any(|e| e == "sleep:13"));
    let lines = logger.lines_for(search_id);
    assert!(
        lines.iter().any(|l| l.contains("Image Searching")),
        "expected search-area log before match: {lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.contains("matching")),
        "expected per-target match log: {lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.contains("Total # found: 0")),
        "{lines:?}"
    );
    let entries = logger.entries_for(search_id);
    let image_labels: Vec<_> = entries
        .iter()
        .filter_map(|e| match e {
            crate::ActionLogEntry::Image(img) => Some(img.label.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        image_labels.iter().any(|l| l.contains("Capture")),
        "expected capture image in logs: {image_labels:?}"
    );
    let item_titles: Vec<_> = entries
        .iter()
        .filter_map(|e| match e {
            crate::ActionLogEntry::ItemPipeline { title, .. } => Some(title.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        item_titles
            .iter()
            .any(|t| t.contains("Prog~Item") || t.contains("Item")),
        "expected item pipeline card in logs: {item_titles:?}"
    );
}

#[test]
fn image_search_break_stops_match_loop() {
    // Two synthetic matches via run_matches directly.
    let mut backend = RecordingBackend::default();
    let mut exec = crate::run::Executor::new(&mut backend);
    let mut macro_ = Macro::new("t", 0, vec![]);
    macro_.keyboard_delay = 0;
    macro_.mouse_delay = 0;
    let results = vec![named("a", 1, 1, 0, 0), named("b", 2, 2, 0, 0)];
    let subactions = vec![
        Action {
            id: ActionId::new(),
            kind: ActionKind::Wait {
                time: ScalarValue::Int(4),
            },
        },
        Action {
            id: ActionId::new(),
            kind: ActionKind::Break,
        },
    ];
    run_matches(
        &mut exec,
        ActionId::new(),
        &["a".into(), "b".into()],
        &results,
        &CoordinateOutputs::defaults(),
        false,
        &subactions,
        &mut macro_,
    )
    .unwrap();
    assert_eq!(
        backend
            .log
            .iter()
            .filter(|e| e.as_str() == "sleep:4")
            .count(),
        1,
        "break should stop after first match: {:?}",
        backend.log
    );
}

#[test]
fn find_template_matches_exact_peak() {
    // blur=0 still maps to kernel 5; pattern must survive that.
    let mut tmpl = ImageBuf::new(10, 10, 3, 40);
    for y in 0..10 {
        for x in 0..10 {
            let o = tmpl.pixel_offset(x, y);
            tmpl.data[o] = (x * 17 + y * 9) as u8;
            tmpl.data[o + 1] = (x * 3 + y * 29) as u8;
            tmpl.data[o + 2] = (255 - x * 11) as u8;
        }
    }
    let mut search = ImageBuf::new(50, 50, 3, 30);
    search.stamp(&tmpl, 15, 18);
    let kernel = search_blur_kernel(0);
    let search_blurred = sqyre_match::blur_image_owned(search, kernel).unwrap();
    let hits = sqyre_match::find_template_matches(
        &search_blurred,
        &tmpl,
        None,
        0.7,
        0,
        DEFAULT_CLOSE_MATCHES_DISTANCE,
    )
    .unwrap();
    assert!(
        hits.iter()
            .any(|p| (p.x - 15).abs() <= 2 && (p.y - 18).abs() <= 2),
        "expected peak near (15,18), got {hits:?}"
    );
}

#[test]
fn ocr_writes_text_and_target_coords() {
    use crate::backends::{FixedOcrEngine, OcrResult};
    use sqyre_vision::OcrWordBox;

    let img = RgbaImage::from_pixel(20, 10, Rgba([255, 255, 255, 255]));
    let mut backend = RecordingBackend::default();
    let mut capturer = RecordingCapturer {
        next: Some(img),
        bounds: DesktopRect {
            x: 0,
            y: 0,
            w: 2000,
            h: 2000,
        },
        ..Default::default()
    };
    let resolver = FixedArea;
    let ocr = FixedOcrEngine {
        result: OcrResult {
            text: "Hello Submit Button".into(),
            words: vec![
                OcrWordBox {
                    word: "Hello".into(),
                    left: 0,
                    top: 0,
                    right: 40,
                    bottom: 20,
                },
                OcrWordBox {
                    word: "Submit".into(),
                    left: 50,
                    top: 0,
                    right: 110,
                    bottom: 20,
                },
                OcrWordBox {
                    word: "Button".into(),
                    left: 120,
                    top: 0,
                    right: 180,
                    bottom: 20,
                },
            ],
        },
        ..Default::default()
    };
    let log = SharedActionLog::new();
    let ocr_id = ActionId::new();

    let mut macro_ = Macro::new("t", 0, vec![]);
    macro_.keyboard_delay = 0;
    macro_.mouse_delay = 0;
    macro_.root = root_loop(vec![Action {
        id: ocr_id,
        kind: ActionKind::Ocr {
            name: "read".into(),
            target: "Submit".into(),
            search_area: CoordinateRef("prog~box".into()),
            output_variable: "ocrText".into(),
            blur: 1,
            min_threshold: 0,
            resize: 1.0,
            grayscale: true,
            threshold_otsu: false,
            threshold_invert: false,
            detection: sqyre_domain::DetectionBranch {
                coords: CoordinateOutputs {
                    output_x_variable: "foundX".into(),
                    output_y_variable: "foundY".into(),
                },
                ..Default::default()
            },
        },
    }]);

    execute_macro_with(
        &mut macro_,
        ExecDeps {
            automation: &mut backend,
            capturer: Some(&mut capturer),
            close_matches_distance: 0,
            resolver: Some(&resolver),
            icons: None,
            macros: None,
            continue_waiter: None,
            window_focuser: None,
            ocr: Some(&ocr),
            stop_flag: None,
            logger: Some(&log),
            highlighter: None,
            runtime_vars: None,
            variables_dir: None,
        },
    )
    .unwrap();

    assert_eq!(
        macro_.variables.get("ocrText").map(|v| v.as_display()),
        Some("Hello Submit Button".into())
    );
    // FixedArea resolves to (100,200)-(110,210); box center (80,10) + origin
    assert_eq!(
        macro_.variables.get("foundX").map(|v| v.as_display()),
        Some("180".into())
    );
    assert_eq!(
        macro_.variables.get("foundY").map(|v| v.as_display()),
        Some("210".into())
    );
    let entries = log.entries_for(ocr_id);
    let image_labels: Vec<_> = entries
        .iter()
        .filter_map(|e| match e {
            crate::ActionLogEntry::Image(img) => Some(img.label.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        image_labels.iter().any(|l| l.contains("Capture")),
        "expected capture image: {image_labels:?}"
    );
    assert!(
        image_labels
            .iter()
            .any(|l| l.contains("Ready for OCR") || l.contains("Grayscale")),
        "expected preprocess step images: {image_labels:?}"
    );
    assert!(
        image_labels.iter().any(|l| l.contains("word boxes")),
        "expected OCR word-box overlay: {image_labels:?}"
    );
    let lines = log.lines_for(ocr_id);
    assert!(
        lines.iter().any(|l| l.contains("OCR full text")),
        "expected full OCR text log: {lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("word[") && l.contains("Hello")),
        "expected per-word OCR detail: {lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("word[") && l.contains("Submit")),
        "expected Submit word in OCR detail: {lines:?}"
    );
}

#[test]
fn ocr_runs_branch_when_target_found() {
    use crate::backends::{FixedOcrEngine, OcrResult};
    use sqyre_vision::OcrWordBox;

    let img = RgbaImage::from_pixel(20, 10, Rgba([255, 255, 255, 255]));
    let mut backend = RecordingBackend::default();
    let mut capturer = RecordingCapturer {
        next: Some(img),
        bounds: DesktopRect {
            x: 0,
            y: 0,
            w: 2000,
            h: 2000,
        },
        ..Default::default()
    };
    let resolver = FixedArea;
    let ocr = FixedOcrEngine {
        result: OcrResult {
            text: "Hello Submit Button".into(),
            words: vec![OcrWordBox {
                word: "Submit".into(),
                left: 50,
                top: 0,
                right: 110,
                bottom: 20,
            }],
        },
        ..Default::default()
    };

    let mut macro_ = Macro::new("t", 0, vec![]);
    macro_.keyboard_delay = 0;
    macro_.mouse_delay = 0;
    macro_.root = root_loop(vec![Action {
        id: ActionId::new(),
        kind: ActionKind::Ocr {
            name: "read".into(),
            target: "Submit".into(),
            search_area: CoordinateRef("prog~box".into()),
            output_variable: "ocrText".into(),
            blur: 1,
            min_threshold: 0,
            resize: 1.0,
            grayscale: true,
            threshold_otsu: false,
            threshold_invert: false,
            detection: sqyre_domain::DetectionBranch {
                subactions: vec![sqyre_domain::test_action(ActionKind::Wait {
                    time: ScalarValue::Int(19),
                })],
                ..Default::default()
            },
        },
    }]);

    execute_macro_with(
        &mut macro_,
        ExecDeps {
            automation: &mut backend,
            capturer: Some(&mut capturer),
            close_matches_distance: 0,
            resolver: Some(&resolver),
            icons: None,
            macros: None,
            continue_waiter: None,
            window_focuser: None,
            ocr: Some(&ocr),
            stop_flag: None,
            logger: None,
            highlighter: None,
            runtime_vars: None,
            variables_dir: None,
        },
    )
    .unwrap();

    assert!(
        backend.log.iter().any(|e| e == "sleep:19"),
        "expected child wait on OCR match: {:?}",
        backend.log
    );
    assert_eq!(
        macro_.variables.get("ocrText").map(|v| v.as_display()),
        Some("Hello Submit Button".into())
    );
}

#[test]
fn ocr_no_find_runs_branch_when_flag_set() {
    use crate::backends::{FixedOcrEngine, OcrResult};

    let img = RgbaImage::from_pixel(20, 10, Rgba([255, 255, 255, 255]));
    let mut backend = RecordingBackend::default();
    let mut capturer = RecordingCapturer {
        next: Some(img),
        bounds: DesktopRect {
            x: 0,
            y: 0,
            w: 2000,
            h: 2000,
        },
        ..Default::default()
    };
    let resolver = FixedArea;
    let ocr = FixedOcrEngine {
        result: OcrResult {
            text: "Hello World".into(),
            words: vec![],
        },
        ..Default::default()
    };

    let mut macro_ = Macro::new("t", 0, vec![]);
    macro_.keyboard_delay = 0;
    macro_.mouse_delay = 0;
    macro_.variables.set("foundX", ScalarValue::Int(1));
    macro_.variables.set("foundY", ScalarValue::Int(2));
    macro_
        .variables
        .set("ocrText", ScalarValue::String("stale".into()));
    macro_.root = root_loop(vec![Action {
        id: ActionId::new(),
        kind: ActionKind::Ocr {
            name: "read".into(),
            target: "will-not-match-zzz".into(),
            search_area: CoordinateRef("prog~box".into()),
            output_variable: "ocrText".into(),
            blur: 1,
            min_threshold: 0,
            resize: 1.0,
            grayscale: true,
            threshold_otsu: false,
            threshold_invert: false,
            detection: sqyre_domain::DetectionBranch {
                coords: CoordinateOutputs {
                    output_x_variable: "foundX".into(),
                    output_y_variable: "foundY".into(),
                },
                run_branch_on_no_find: true,
                subactions: vec![sqyre_domain::test_action(ActionKind::Wait {
                    time: ScalarValue::Int(17),
                })],
                ..Default::default()
            },
        },
    }]);

    execute_macro_with(
        &mut macro_,
        ExecDeps {
            automation: &mut backend,
            capturer: Some(&mut capturer),
            close_matches_distance: 0,
            resolver: Some(&resolver),
            icons: None,
            macros: None,
            continue_waiter: None,
            window_focuser: None,
            ocr: Some(&ocr),
            stop_flag: None,
            logger: None,
            highlighter: None,
            runtime_vars: None,
            variables_dir: None,
        },
    )
    .unwrap();

    assert!(
        backend.log.iter().any(|e| e == "sleep:17"),
        "expected child wait on OCR no-find: {:?}",
        backend.log
    );
    assert!(macro_.variables.get("ocrText").is_none());
    assert!(macro_.variables.get("foundX").is_none());
    assert!(macro_.variables.get("foundY").is_none());
}

#[test]
fn ocr_skips_branch_when_target_missing() {
    use crate::backends::{FixedOcrEngine, OcrResult};

    let img = RgbaImage::from_pixel(20, 10, Rgba([255, 255, 255, 255]));
    let mut backend = RecordingBackend::default();
    let mut capturer = RecordingCapturer {
        next: Some(img),
        bounds: DesktopRect {
            x: 0,
            y: 0,
            w: 2000,
            h: 2000,
        },
        ..Default::default()
    };
    let resolver = FixedArea;
    let ocr = FixedOcrEngine {
        result: OcrResult {
            text: "Hello World".into(),
            words: vec![],
        },
        ..Default::default()
    };

    let mut macro_ = Macro::new("t", 0, vec![]);
    macro_.keyboard_delay = 0;
    macro_.mouse_delay = 0;
    macro_.root = root_loop(vec![Action {
        id: ActionId::new(),
        kind: ActionKind::Ocr {
            name: "read".into(),
            target: "Submit".into(),
            search_area: CoordinateRef("prog~box".into()),
            output_variable: "ocrText".into(),
            blur: 1,
            min_threshold: 0,
            resize: 1.0,
            grayscale: true,
            threshold_otsu: false,
            threshold_invert: false,
            detection: sqyre_domain::DetectionBranch {
                subactions: vec![sqyre_domain::test_action(ActionKind::Wait {
                    time: ScalarValue::Int(19),
                })],
                ..Default::default()
            },
        },
    }]);

    execute_macro_with(
        &mut macro_,
        ExecDeps {
            automation: &mut backend,
            capturer: Some(&mut capturer),
            close_matches_distance: 0,
            resolver: Some(&resolver),
            icons: None,
            macros: None,
            continue_waiter: None,
            window_focuser: None,
            ocr: Some(&ocr),
            stop_flag: None,
            logger: None,
            highlighter: None,
            runtime_vars: None,
            variables_dir: None,
        },
    )
    .unwrap();

    assert!(
        !backend.log.iter().any(|e| e == "sleep:19"),
        "child should not run on OCR miss: {:?}",
        backend.log
    );
    assert!(macro_.variables.get("ocrText").is_none());
}

fn solid_rgba(w: u32, h: u32, rgb: [u8; 3]) -> RgbaImage {
    let mut img = RgbaImage::new(w, h);
    for p in img.pixels_mut() {
        *p = Rgba([rgb[0], rgb[1], rgb[2], 255]);
    }
    img
}

#[test]
fn find_pixel_wait_until_found_retries_then_succeeds() {
    use sqyre_domain::RepeatMode;
    let miss = solid_rgba(8, 8, [0, 0, 0]);
    let mut hit = solid_rgba(8, 8, [0, 0, 0]);
    hit.put_pixel(2, 3, Rgba([255, 0, 0, 255]));

    let mut backend = RecordingBackend::default();
    let mut capturer = RecordingCapturer {
        queue: vec![miss, hit],
        bounds: DesktopRect {
            x: 0,
            y: 0,
            w: 2000,
            h: 2000,
        },
        ..Default::default()
    };
    let resolver = FixedArea;
    let logger = SharedActionLog::new();
    let find_id = ActionId::new();
    let mut macro_ = Macro::new("t", 0, vec![]);
    macro_.keyboard_delay = 0;
    macro_.mouse_delay = 0;
    macro_.root = root_loop(vec![Action {
        id: find_id,
        kind: ActionKind::FindPixel {
            name: "red".into(),
            search_area: CoordinateRef("area".into()),
            target_color: "#ff0000".into(),
            color_tolerance: 0,
            detection: sqyre_domain::DetectionBranch {
                wait: WaitTilFoundConfig {
                    repeat_mode: RepeatMode::WaitUntilFound,
                    wait_til_found_seconds: 5,
                    wait_til_found_interval_ms: 1,
                    max_iterations: 0,
                },
                coords: CoordinateOutputs {
                    output_x_variable: "foundX".into(),
                    output_y_variable: "foundY".into(),
                },
                ..Default::default()
            },
        },
    }]);

    execute_macro_with(
        &mut macro_,
        ExecDeps {
            automation: &mut backend,
            capturer: Some(&mut capturer),
            close_matches_distance: 0,
            resolver: Some(&resolver),
            icons: None,
            macros: None,
            continue_waiter: None,
            window_focuser: None,
            ocr: None,
            stop_flag: None,
            logger: Some(&logger),
            highlighter: None,
            runtime_vars: None,
            variables_dir: None,
        },
    )
    .unwrap();

    assert_eq!(
        capturer.log.len(),
        2,
        "expected miss then hit: {:?}",
        capturer.log
    );
    assert_eq!(
        macro_.variables.get("foundX").map(|v| v.as_display()),
        Some("102".into())
    );
    assert!(
        logger
            .lines_for(find_id)
            .iter()
            .any(|l| l.starts_with("timing: total ")),
        "{:?}",
        logger.lines_for(find_id)
    );
}

#[test]
fn find_pixel_wait_until_found_stops_when_flag_set() {
    use sqyre_domain::RepeatMode;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let miss = solid_rgba(8, 8, [0, 0, 0]);
    let stop = Arc::new(AtomicBool::new(false));
    let stop_for_backend = Arc::clone(&stop);

    /// Sleeps briefly then trips the stop flag so wait-until-found aborts.
    struct StopOnSleep {
        inner: RecordingBackend,
        stop: Arc<AtomicBool>,
        sleeps: usize,
    }
    impl crate::backends::AutomationBackend for StopOnSleep {
        fn milli_sleep(&mut self, ms: i32) {
            self.inner.milli_sleep(ms);
            self.sleeps += 1;
            if self.sleeps >= 1 {
                self.stop.store(true, Ordering::SeqCst);
            }
        }
        fn move_to(&mut self, x: i32, y: i32, opts: crate::backends::MoveOptions) {
            self.inner.move_to(x, y, opts);
        }
        fn click(&mut self, button: &str, down: bool) -> Result<(), String> {
            self.inner.click(button, down)
        }
        fn scroll(&mut self, up: bool) -> Result<(), String> {
            self.inner.scroll(up)
        }
        fn key_down(&mut self, key: &str) -> Result<(), String> {
            self.inner.key_down(key)
        }
        fn key_up(&mut self, key: &str) -> Result<(), String> {
            self.inner.key_up(key)
        }
        fn type_char(&mut self, ch: char) {
            self.inner.type_char(ch);
        }
        fn write_clipboard(&mut self, s: &str) -> Result<(), String> {
            self.inner.write_clipboard(s)
        }
    }

    let mut backend = StopOnSleep {
        inner: RecordingBackend::default(),
        stop: stop_for_backend,
        sleeps: 0,
    };
    let mut capturer = RecordingCapturer {
        next: Some(miss),
        bounds: DesktopRect {
            x: 0,
            y: 0,
            w: 2000,
            h: 2000,
        },
        ..Default::default()
    };
    let resolver = FixedArea;
    let find_id = ActionId::new();
    let mut macro_ = Macro::new("t", 0, vec![]);
    macro_.keyboard_delay = 0;
    macro_.mouse_delay = 0;
    macro_.root = root_loop(vec![Action {
        id: find_id,
        kind: ActionKind::FindPixel {
            name: "red".into(),
            search_area: CoordinateRef("area".into()),
            target_color: "#ff0000".into(),
            color_tolerance: 0,
            detection: sqyre_domain::DetectionBranch {
                wait: WaitTilFoundConfig {
                    repeat_mode: RepeatMode::WaitUntilFound,
                    wait_til_found_seconds: 30,
                    wait_til_found_interval_ms: 50,
                    max_iterations: 0,
                },
                ..Default::default()
            },
        },
    }]);

    execute_macro_with(
        &mut macro_,
        ExecDeps {
            automation: &mut backend,
            capturer: Some(&mut capturer),
            close_matches_distance: 0,
            resolver: Some(&resolver),
            icons: None,
            macros: None,
            continue_waiter: None,
            window_focuser: None,
            ocr: None,
            stop_flag: Some(&stop),
            logger: None,
            highlighter: None,
            runtime_vars: None,
            variables_dir: None,
        },
    )
    .unwrap();

    assert!(stop.load(Ordering::SeqCst));
    assert!(backend.sleeps >= 1);
}

#[test]
fn find_pixel_repeat_while_found_runs_then_stops_on_miss() {
    use sqyre_domain::RepeatMode;
    let mut hit = solid_rgba(8, 8, [0, 0, 0]);
    hit.put_pixel(1, 1, Rgba([0, 255, 0, 255]));
    let miss = solid_rgba(8, 8, [0, 0, 0]);

    let mut backend = RecordingBackend::default();
    let mut capturer = RecordingCapturer {
        queue: vec![hit, miss],
        bounds: DesktopRect {
            x: 0,
            y: 0,
            w: 2000,
            h: 2000,
        },
        ..Default::default()
    };
    let resolver = FixedArea;
    let find_id = ActionId::new();
    let child_id = ActionId::new();
    let mut macro_ = Macro::new("t", 0, vec![]);
    macro_.keyboard_delay = 0;
    macro_.mouse_delay = 0;
    macro_.root = root_loop(vec![Action {
        id: find_id,
        kind: ActionKind::FindPixel {
            name: "green".into(),
            search_area: CoordinateRef("area".into()),
            target_color: "#00ff00".into(),
            color_tolerance: 0,
            detection: sqyre_domain::DetectionBranch {
                wait: WaitTilFoundConfig {
                    repeat_mode: RepeatMode::WhileFound,
                    wait_til_found_seconds: 0,
                    wait_til_found_interval_ms: 1,
                    max_iterations: 5,
                },
                subactions: vec![Action {
                    id: child_id,
                    kind: ActionKind::Wait {
                        time: ScalarValue::Int(7),
                    },
                }],
                ..Default::default()
            },
        },
    }]);

    execute_macro_with(
        &mut macro_,
        ExecDeps {
            automation: &mut backend,
            capturer: Some(&mut capturer),
            close_matches_distance: 0,
            resolver: Some(&resolver),
            icons: None,
            macros: None,
            continue_waiter: None,
            window_focuser: None,
            ocr: None,
            stop_flag: None,
            logger: None,
            highlighter: None,
            runtime_vars: None,
            variables_dir: None,
        },
    )
    .unwrap();

    let child_waits = backend
        .log
        .iter()
        .filter(|e| e.as_str() == "sleep:7")
        .count();
    assert_eq!(
        child_waits, 1,
        "expected one child run then stop on miss: {:?}",
        backend.log
    );
    assert_eq!(capturer.log.len(), 2, "{:?}", capturer.log);
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

#[test]
fn image_search_wait_until_found_retries_then_succeeds() {
    use sqyre_domain::RepeatMode;
    sqyre_vision::with_search_cache_test_lock(|| {
        sqyre_vision::reset_search_cache_for_testing();
        let dir = tempfile::tempdir().unwrap();
        let tmpl_path = dir.path().join("tmpl.png");
        let tmpl = patterned_rgba(10, 10, 11);
        tmpl.save(&tmpl_path).unwrap();

        let miss = RgbaImage::from_pixel(50, 50, Rgba([0, 255, 0, 255]));
        let mut hit = RgbaImage::from_pixel(50, 50, Rgba([30, 30, 30, 255]));
        stamp_rgba(&mut hit, &tmpl, 15, 18);

        let icons = MapIcons {
            paths: HashMap::from([("Prog~Item".into(), vec![tmpl_path])]),
            masks: HashMap::new(),
            meta: HashMap::new(),
        };
        let mut backend = RecordingBackend::default();
        let mut capturer = RecordingCapturer {
            queue: vec![miss],
            next: Some(hit),
            bounds: DesktopRect {
                x: 0,
                y: 0,
                w: 2000,
                h: 2000,
            },
            ..Default::default()
        };
        let resolver = FixedArea;
        let close_matches = 8;
        let search_id = ActionId::new();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.keyboard_delay = 0;
        macro_.mouse_delay = 0;
        macro_.root = root_loop(vec![Action {
            id: search_id,
            kind: ActionKind::ImageSearch {
                name: "find".into(),
                targets: vec!["Prog~Item".into()],
                search_area: CoordinateRef("Prog~Box".into()),
                tolerance: 0.7,
                blur: 0,
                detection: sqyre_domain::DetectionBranch {
                    wait: WaitTilFoundConfig {
                        repeat_mode: RepeatMode::WaitUntilFound,
                        wait_til_found_seconds: 5,
                        wait_til_found_interval_ms: 1,
                        max_iterations: 0,
                    },
                    coords: CoordinateOutputs {
                        output_x_variable: "foundX".into(),
                        output_y_variable: "foundY".into(),
                    },
                    subactions: vec![sqyre_domain::test_action(ActionKind::Wait {
                        time: ScalarValue::Int(3),
                    })],
                    ..Default::default()
                },
            },
        }]);

        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: Some(&mut capturer),
                close_matches_distance: close_matches,
                resolver: Some(&resolver),
                icons: Some(&icons),
                macros: None,
                continue_waiter: None,
                window_focuser: None,
                ocr: None,
                stop_flag: None,
                logger: None,
                highlighter: None,
                runtime_vars: None,
                variables_dir: None,
            },
        )
        .unwrap();

        assert!(
            capturer.log.len() >= 2,
            "expected miss then hit capture: {:?}",
            capturer.log
        );
        assert!(
            backend.log.iter().any(|e| e == "sleep:3"),
            "expected child on find: {:?}",
            backend.log
        );
        assert!(macro_.variables.get("foundX").is_some());
    });
}

#[test]
fn image_search_repeat_while_found_then_stops() {
    use sqyre_domain::RepeatMode;
    sqyre_vision::with_search_cache_test_lock(|| {
        sqyre_vision::reset_search_cache_for_testing();
        let dir = tempfile::tempdir().unwrap();
        let tmpl_path = dir.path().join("tmpl.png");
        let tmpl = patterned_rgba(10, 10, 22);
        tmpl.save(&tmpl_path).unwrap();

        let mut hit = RgbaImage::from_pixel(50, 50, Rgba([10, 10, 10, 255]));
        stamp_rgba(&mut hit, &tmpl, 12, 14);
        let miss = RgbaImage::from_pixel(50, 50, Rgba([10, 10, 10, 255]));

        let icons = MapIcons {
            paths: HashMap::from([("Prog~Item".into(), vec![tmpl_path])]),
            masks: HashMap::new(),
            meta: HashMap::new(),
        };
        let mut backend = RecordingBackend::default();
        let mut capturer = RecordingCapturer {
            queue: vec![hit],
            next: Some(miss),
            bounds: DesktopRect {
                x: 0,
                y: 0,
                w: 2000,
                h: 2000,
            },
            ..Default::default()
        };
        let resolver = FixedArea;
        let close_matches = 8;
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.keyboard_delay = 0;
        macro_.mouse_delay = 0;
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::ImageSearch {
                name: "loop".into(),
                targets: vec!["Prog~Item".into()],
                search_area: CoordinateRef("Prog~Box".into()),
                tolerance: 0.7,
                blur: 0,
                detection: sqyre_domain::DetectionBranch {
                    wait: WaitTilFoundConfig {
                        repeat_mode: RepeatMode::WhileFound,
                        wait_til_found_seconds: 0,
                        wait_til_found_interval_ms: 1,
                        max_iterations: 5,
                    },
                    subactions: vec![sqyre_domain::test_action(ActionKind::Wait {
                        time: ScalarValue::Int(9),
                    })],
                    ..Default::default()
                },
            },
        }]);

        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: Some(&mut capturer),
                close_matches_distance: close_matches,
                resolver: Some(&resolver),
                icons: Some(&icons),
                macros: None,
                continue_waiter: None,
                window_focuser: None,
                ocr: None,
                stop_flag: None,
                logger: None,
                highlighter: None,
                runtime_vars: None,
                variables_dir: None,
            },
        )
        .unwrap();

        let child_waits = backend
            .log
            .iter()
            .filter(|e| e.as_str() == "sleep:9")
            .count();
        assert_eq!(
            child_waits, 1,
            "expected one child run then stop: {:?}",
            backend.log
        );
    });
}

#[test]
fn image_search_multi_variant_matches_either_template() {
    sqyre_vision::with_search_cache_test_lock(|| {
        sqyre_vision::reset_search_cache_for_testing();
        let dir = tempfile::tempdir().unwrap();
        let v1 = dir.path().join("v1.png");
        let v2 = dir.path().join("v2.png");
        let first = patterned_rgba(10, 10, 1);
        let second = patterned_rgba(10, 10, 90);
        first.save(&v1).unwrap();
        second.save(&v2).unwrap();

        // Screen contains only the second variant.
        let mut search = RgbaImage::from_pixel(50, 50, Rgba([40, 40, 40, 255]));
        stamp_rgba(&mut search, &second, 15, 18);

        let icons = MapIcons {
            paths: HashMap::from([("Prog~Item".into(), vec![v1, v2])]),
            masks: HashMap::new(),
            meta: HashMap::from([(
                "Prog~Item".into(),
                ItemMeta {
                    name: "Item".into(),
                    stack_max: 3,
                    cols: 1,
                    rows: 1,
                },
            )]),
        };
        let mut backend = RecordingBackend::default();
        let mut capturer = RecordingCapturer {
            next: Some(search),
            bounds: DesktopRect {
                x: 0,
                y: 0,
                w: 2000,
                h: 2000,
            },
            ..Default::default()
        };
        let resolver = FixedArea;
        let close_matches = 8;
        let logger = SharedActionLog::new();
        let search_id = ActionId::new();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.keyboard_delay = 0;
        macro_.mouse_delay = 0;
        macro_.root = root_loop(vec![Action {
            id: search_id,
            kind: ActionKind::ImageSearch {
                name: "multi".into(),
                targets: vec!["Prog~Item".into()],
                search_area: CoordinateRef("Prog~Box".into()),
                tolerance: 0.7,
                blur: 0,
                detection: sqyre_domain::DetectionBranch {
                    coords: CoordinateOutputs {
                        output_x_variable: "foundX".into(),
                        output_y_variable: "foundY".into(),
                    },
                    subactions: vec![sqyre_domain::test_action(ActionKind::Wait {
                        time: ScalarValue::Int(2),
                    })],
                    ..Default::default()
                },
            },
        }]);

        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: Some(&mut capturer),
                close_matches_distance: close_matches,
                resolver: Some(&resolver),
                icons: Some(&icons),
                macros: None,
                continue_waiter: None,
                window_focuser: None,
                ocr: None,
                stop_flag: None,
                logger: Some(&logger),
                highlighter: None,
                runtime_vars: None,
                variables_dir: None,
            },
        )
        .unwrap();

        assert!(
            backend.log.iter().any(|e| e == "sleep:2"),
            "expected match via second variant: {:?}",
            backend.log
        );
        assert_eq!(
            macro_.variables.get("StackMax").map(|v| v.as_display()),
            Some("3".into())
        );
        let lines = logger.lines_for(search_id);
        assert!(
            lines.iter().any(|l| l.contains("Total # found:")),
            "{lines:?}"
        );
    });
}

#[test]
fn image_search_uses_mask_path_when_present() {
    sqyre_vision::with_search_cache_test_lock(|| {
        sqyre_vision::reset_search_cache_for_testing();
        let dir = tempfile::tempdir().unwrap();
        let tmpl_path = dir.path().join("tmpl.png");
        let mask_path = dir.path().join("mask.png");

        let tmpl = patterned_rgba(10, 10, 7);
        tmpl.save(&tmpl_path).unwrap();

        // Full-white mask (all pixels active) — exercises mask load/cache path.
        let mask = RgbaImage::from_pixel(10, 10, Rgba([255, 255, 255, 255]));
        mask.save(&mask_path).unwrap();

        let mut search = RgbaImage::from_pixel(50, 50, Rgba([20, 20, 20, 255]));
        stamp_rgba(&mut search, &tmpl, 15, 18);

        let icons = MapIcons {
            paths: HashMap::from([("Prog~Item".into(), vec![tmpl_path])]),
            masks: HashMap::from([("Prog~Item".into(), mask_path)]),
            meta: HashMap::new(),
        };
        let mut backend = RecordingBackend::default();
        let mut capturer = RecordingCapturer {
            next: Some(search),
            bounds: DesktopRect {
                x: 0,
                y: 0,
                w: 2000,
                h: 2000,
            },
            ..Default::default()
        };
        let resolver = FixedArea;
        let close_matches = 8;
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.keyboard_delay = 0;
        macro_.mouse_delay = 0;
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::ImageSearch {
                name: "masked".into(),
                targets: vec!["Prog~Item".into()],
                search_area: CoordinateRef("Prog~Box".into()),
                tolerance: 0.7,
                blur: 0,
                detection: sqyre_domain::DetectionBranch {
                    coords: CoordinateOutputs {
                        output_x_variable: "foundX".into(),
                        output_y_variable: "foundY".into(),
                    },
                    subactions: vec![sqyre_domain::test_action(ActionKind::Wait {
                        time: ScalarValue::Int(5),
                    })],
                    ..Default::default()
                },
            },
        }]);

        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: Some(&mut capturer),
                close_matches_distance: close_matches,
                resolver: Some(&resolver),
                icons: Some(&icons),
                macros: None,
                continue_waiter: None,
                window_focuser: None,
                ocr: None,
                stop_flag: None,
                logger: None,
                highlighter: None,
                runtime_vars: None,
                variables_dir: None,
            },
        )
        .unwrap();

        assert!(
            backend.log.iter().any(|e| e == "sleep:5"),
            "match with mask path should succeed: {:?}",
            backend.log
        );
        assert!(macro_.variables.get("foundX").is_some());
    });
}

#[test]
fn ocr_wait_until_found_retries_then_succeeds() {
    use crate::backends::{OcrResult, QueuedOcrEngine};
    use sqyre_domain::RepeatMode;

    let img = RgbaImage::from_pixel(20, 10, Rgba([255, 255, 255, 255]));
    let mut backend = RecordingBackend::default();
    let mut capturer = RecordingCapturer {
        next: Some(img),
        bounds: DesktopRect {
            x: 0,
            y: 0,
            w: 2000,
            h: 2000,
        },
        ..Default::default()
    };
    let resolver = FixedArea;
    let ocr = QueuedOcrEngine {
        queue: std::sync::Mutex::new(vec![
            OcrResult {
                text: "noise".into(),
                words: vec![],
            },
            OcrResult {
                text: "Submit now".into(),
                words: vec![],
            },
        ]),
        ..Default::default()
    };

    let mut macro_ = Macro::new("t", 0, vec![]);
    macro_.keyboard_delay = 0;
    macro_.mouse_delay = 0;
    macro_.root = root_loop(vec![Action {
        id: ActionId::new(),
        kind: ActionKind::Ocr {
            name: "read".into(),
            target: "Submit".into(),
            search_area: CoordinateRef("prog~box".into()),
            output_variable: "ocrText".into(),
            blur: 1,
            min_threshold: 0,
            resize: 1.0,
            grayscale: true,
            threshold_otsu: false,
            threshold_invert: false,
            detection: sqyre_domain::DetectionBranch {
                wait: WaitTilFoundConfig {
                    repeat_mode: RepeatMode::WaitUntilFound,
                    wait_til_found_seconds: 5,
                    wait_til_found_interval_ms: 1,
                    max_iterations: 0,
                },
                coords: CoordinateOutputs {
                    output_x_variable: "foundX".into(),
                    output_y_variable: "foundY".into(),
                },
                subactions: vec![sqyre_domain::test_action(ActionKind::Wait {
                    time: ScalarValue::Int(11),
                })],
                ..Default::default()
            },
        },
    }]);

    execute_macro_with(
        &mut macro_,
        ExecDeps {
            automation: &mut backend,
            capturer: Some(&mut capturer),
            close_matches_distance: 0,
            resolver: Some(&resolver),
            icons: None,
            macros: None,
            continue_waiter: None,
            window_focuser: None,
            ocr: Some(&ocr),
            stop_flag: None,
            logger: None,
            highlighter: None,
            runtime_vars: None,
            variables_dir: None,
        },
    )
    .unwrap();

    assert!(
        ocr.log.lock().unwrap().len() >= 2,
        "expected OCR retries: {:?}",
        ocr.log.lock().unwrap()
    );
    assert_eq!(
        macro_.variables.get("ocrText").map(|v| v.as_display()),
        Some("Submit now".into())
    );
    assert!(
        backend.log.iter().any(|e| e == "sleep:11"),
        "{:?}",
        backend.log
    );
}

#[test]
fn ocr_repeat_while_found_then_stops_on_miss() {
    use crate::backends::{OcrResult, QueuedOcrEngine};
    use sqyre_domain::RepeatMode;

    let img = RgbaImage::from_pixel(20, 10, Rgba([255, 255, 255, 255]));
    let mut backend = RecordingBackend::default();
    let mut capturer = RecordingCapturer {
        next: Some(img),
        bounds: DesktopRect {
            x: 0,
            y: 0,
            w: 2000,
            h: 2000,
        },
        ..Default::default()
    };
    let resolver = FixedArea;
    let ocr = QueuedOcrEngine {
        queue: std::sync::Mutex::new(vec![
            OcrResult {
                text: "Keep going".into(),
                words: vec![],
            },
            OcrResult {
                text: "gone".into(),
                words: vec![],
            },
        ]),
        ..Default::default()
    };

    let mut macro_ = Macro::new("t", 0, vec![]);
    macro_.keyboard_delay = 0;
    macro_.mouse_delay = 0;
    macro_.root = root_loop(vec![Action {
        id: ActionId::new(),
        kind: ActionKind::Ocr {
            name: "loop".into(),
            target: "Keep".into(),
            search_area: CoordinateRef("prog~box".into()),
            output_variable: "ocrText".into(),
            blur: 1,
            min_threshold: 0,
            resize: 1.0,
            grayscale: true,
            threshold_otsu: false,
            threshold_invert: false,
            detection: sqyre_domain::DetectionBranch {
                wait: WaitTilFoundConfig {
                    repeat_mode: RepeatMode::WhileFound,
                    wait_til_found_seconds: 0,
                    wait_til_found_interval_ms: 1,
                    max_iterations: 5,
                },
                subactions: vec![sqyre_domain::test_action(ActionKind::Wait {
                    time: ScalarValue::Int(8),
                })],
                ..Default::default()
            },
        },
    }]);

    execute_macro_with(
        &mut macro_,
        ExecDeps {
            automation: &mut backend,
            capturer: Some(&mut capturer),
            close_matches_distance: 0,
            resolver: Some(&resolver),
            icons: None,
            macros: None,
            continue_waiter: None,
            window_focuser: None,
            ocr: Some(&ocr),
            stop_flag: None,
            logger: None,
            highlighter: None,
            runtime_vars: None,
            variables_dir: None,
        },
    )
    .unwrap();

    let child_waits = backend
        .log
        .iter()
        .filter(|e| e.as_str() == "sleep:8")
        .count();
    assert_eq!(
        child_waits, 1,
        "expected one OCR while-found iteration: {:?}",
        backend.log
    );
}
