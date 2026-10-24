use super::*;

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
        ) -> std::result::Result<(i32, i32), crate::backends::PortError> {
            Err(crate::backends::PortError::invalid("point"))
        }
        fn resolve_search_area(
            &self,
            r: &CoordinateRef,
            _macro_: &Macro,
        ) -> std::result::Result<(i32, i32, i32, i32), crate::backends::PortError> {
            assert!(r.is_collection(), "expected collection ref, got {r:?}");
            assert_eq!(r.as_str(), "Demo~bag@1,2-1,2");
            Ok((50, 60, 54, 64))
        }
    }

    let mut backend = RecordingBackend::default();
    let mut capturer = RecordingCapturer {
        next: Some(img),
        bounds: full_desktop(),
        ..Default::default()
    };
    let resolver = CollectionOnly;
    let logger = SharedActionLog::new();
    let find_id = ActionId::new();
    let mut macro_ = quiet_macro(vec![Action {
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

    run_search_logged(&mut macro_, &mut backend, &mut capturer, &resolver, &logger);

    assert_eq!(
        macro_.variables.get("foundX").map(|v| v.as_display()),
        Some("51".into())
    );
    assert_eq!(
        macro_.variables.get("foundY").map(|v| v.as_display()),
        Some("61".into())
    );
    capturer.assert_logged("rect:50,60,4,4");
    assert!(
        capturer.log.iter().all(|e| !e.starts_with("rgb_fresh:")),
        "one-shot should crop the cache: {:?}",
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
        bounds: full_desktop(),
        ..Default::default()
    };
    let resolver = SEARCH_FIXED_AREA;
    let logger = SharedActionLog::new();
    let find_id = ActionId::new();
    let mut macro_ = quiet_macro(vec![Action {
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

    run_search_logged(&mut macro_, &mut backend, &mut capturer, &resolver, &logger);

    assert_eq!(
        macro_.variables.get("foundX").map(|v| v.as_display()),
        Some("103".into()) // 100 origin + 3
    );
    assert_eq!(
        macro_.variables.get("foundY").map(|v| v.as_display()),
        Some("205".into()) // 200 origin + 5
    );
    let entries = logger.entries_for(find_id);
    let lines = lines_for(&entries);
    assert!(
        lines.iter().any(|l| l.contains("found matching pixel")),
        "{lines:?}"
    );
    assert!(capturer.has_prefix("rect:"));

    // Structured find: hit at local (3,5) + origin (100,200) → screen (103,205).
    let finds: Vec<_> = entries.iter().flat_map(|e| e.finds().to_vec()).collect();
    assert_eq!(finds.len(), 1, "expected one structured find: {finds:?}");
    let f = &finds[0];
    assert_eq!(f.screen, (95, 197, 112, 214));
    let sqyre_ports::FindContext::Image { rect, .. } = f.context.as_ref().expect("capture context")
    else {
        unreachable!()
    };
    assert_eq!(*rect, (0, 0, 10, 10), "marker box clamps to image bounds");
    assert!(
        entries.iter().any(|e| e.is_image()),
        "expected capture image entry"
    );
}

#[test]
fn find_pixel_not_found_logs() {
    let img = RgbaImage::from_pixel(4, 4, Rgba([0, 0, 255, 255]));
    let mut backend = RecordingBackend::default();
    let mut capturer = RecordingCapturer {
        next: Some(img),
        bounds: full_desktop(),
        ..Default::default()
    };
    let resolver = SEARCH_FIXED_AREA;
    let logger = SharedActionLog::new();
    let find_id = ActionId::new();
    let mut macro_ = quiet_macro(vec![Action {
        id: find_id,
        kind: ActionKind::FindPixel {
            name: "red".into(),
            search_area: CoordinateRef("Prog~Box".into()),
            target_color: "#ff0000".into(),
            color_tolerance: 0,
            detection: Default::default(),
        },
    }]);
    run_search_logged(&mut macro_, &mut backend, &mut capturer, &resolver, &logger);
    let lines = lines_for(&logger.entries_for(find_id));
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
    let mut capturer = capturer_next(img);
    let resolver = SEARCH_FIXED_AREA;
    let mut macro_ = quiet_macro(vec![find_pixel_action(
        ActionId::new(),
        "#ff0000",
        detection_branch(Some(21), None, Default::default(), Default::default()),
    )]);

    run_search(&mut macro_, &mut backend, &mut capturer, &resolver);
    assert_slept(&backend, 21);
}

#[test]
fn find_pixel_no_find_runs_branch_when_flag_set() {
    let mut backend = RecordingBackend::default();
    let mut capturer = capturer_next(RgbaImage::from_pixel(4, 4, Rgba([0, 0, 255, 255])));
    let resolver = SEARCH_FIXED_AREA;
    let mut macro_ = Macro::new("t", 0, vec![]);
    macro_.keyboard_delay = 0;
    macro_.mouse_delay = 0;
    macro_.variables.set("foundX", ScalarValue::Int(1));
    macro_.variables.set("foundY", ScalarValue::Int(2));
    macro_.root = root_loop(vec![find_pixel_action(
        ActionId::new(),
        "#ff0000",
        detection_branch(
            None,
            Some(15),
            Default::default(),
            coords_xy("foundX", "foundY"),
        ),
    )]);

    run_search(&mut macro_, &mut backend, &mut capturer, &resolver);
    assert_slept(&backend, 15);
    assert!(macro_.variables.get("foundX").is_none());
    assert!(macro_.variables.get("foundY").is_none());
}

#[test]
fn find_pixel_skips_branch_when_missing() {
    let mut backend = RecordingBackend::default();
    let mut capturer = capturer_next(RgbaImage::from_pixel(4, 4, Rgba([0, 0, 255, 255])));
    let resolver = SEARCH_FIXED_AREA;
    let mut macro_ = quiet_macro(vec![find_pixel_action(
        ActionId::new(),
        "#ff0000",
        detection_branch(Some(21), None, Default::default(), Default::default()),
    )]);

    run_search(&mut macro_, &mut backend, &mut capturer, &resolver);
    assert_did_not_sleep(&backend, 21);
}

#[test]
fn find_pixel_wait_until_found_retries_then_succeeds() {
    let miss = solid_rgba(8, 8, [0, 0, 0]);
    let mut hit = solid_rgba(8, 8, [0, 0, 0]);
    hit.put_pixel(2, 3, Rgba([255, 0, 0, 255]));

    let mut backend = RecordingBackend::default();
    let mut capturer = capturer_queue(vec![miss, hit]);
    let resolver = SEARCH_FIXED_AREA;
    let logger = SharedActionLog::new();
    let find_id = ActionId::new();
    let mut macro_ = quiet_macro(vec![find_pixel_action(
        find_id,
        "#ff0000",
        detection_branch(
            None,
            None,
            wait_until_found(5, 1),
            coords_xy("foundX", "foundY"),
        ),
    )]);

    run_search_logged(&mut macro_, &mut backend, &mut capturer, &resolver, &logger);

    assert_eq!(
        capturer.log.len(),
        2,
        "expected miss then hit: {:?}",
        capturer.log
    );
    assert!(
        capturer.log[0].starts_with("rect:"),
        "first capture should crop the cache: {:?}",
        capturer.log
    );
    assert!(
        capturer.log[1].starts_with("rgb_fresh:"),
        "wait retry should request a fresh frame: {:?}",
        capturer.log
    );
    assert_eq!(
        macro_.variables.get("foundX").map(|v| v.as_display()),
        Some("102".into())
    );
    let lines = lines_for(&logger.entries_for(find_id));
    assert!(
        lines.iter().any(|l| l.starts_with("timing: total ")),
        "{lines:?}"
    );
}

#[test]
fn find_pixel_wait_until_found_stops_when_flag_set() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let miss = solid_rgba(8, 8, [0, 0, 0]);
    let stop = Arc::new(AtomicBool::new(false));
    let stop_for_backend = Arc::clone(&stop);

    let mut backend = StopAfterSleeps::new(stop_for_backend, 1);
    let mut capturer = RecordingCapturer {
        next: Some(miss),
        bounds: full_desktop(),
        ..Default::default()
    };
    let resolver = SEARCH_FIXED_AREA;
    let find_id = ActionId::new();
    let mut macro_ = quiet_macro(vec![Action {
        id: find_id,
        kind: ActionKind::FindPixel {
            name: "red".into(),
            search_area: CoordinateRef("area".into()),
            target_color: "#ff0000".into(),
            color_tolerance: 0,
            detection: sqyre_domain::DetectionBranch {
                wait: wait_until_found(30, 50),
                ..Default::default()
            },
        },
    }]);

    execute_macro_with(
        &mut macro_,
        ExecDeps::new(&mut backend)
            .capturer(&mut capturer)
            .resolver(&resolver)
            .stop_flag(&stop),
    )
    .unwrap();

    assert!(stop.load(Ordering::SeqCst));
    assert!(backend.sleeps >= 1);
}

#[test]
fn find_pixel_repeat_while_found_runs_then_stops_on_miss() {
    let mut hit = solid_rgba(8, 8, [0, 0, 0]);
    hit.put_pixel(1, 1, Rgba([0, 255, 0, 255]));
    let miss = solid_rgba(8, 8, [0, 0, 0]);

    let mut backend = RecordingBackend::default();
    let mut capturer = capturer_queue(vec![hit, miss]);
    let resolver = SEARCH_FIXED_AREA;
    let mut macro_ = quiet_macro(vec![find_pixel_action(
        ActionId::new(),
        "#00ff00",
        detection_branch(Some(7), None, while_found(1, 5), Default::default()),
    )]);

    run_search(&mut macro_, &mut backend, &mut capturer, &resolver);

    assert_eq!(
        count_sleeps(&backend, 7),
        1,
        "expected one child run then stop on miss: {:?}",
        backend.log
    );
    assert_eq!(capturer.log.len(), 2, "{:?}", capturer.log);
}

#[test]
fn find_pixel_wait_while_found_waits_then_runs_no_find() {
    let mut hit = solid_rgba(8, 8, [0, 0, 0]);
    hit.put_pixel(1, 1, Rgba([0, 255, 0, 255]));
    let miss = solid_rgba(8, 8, [0, 0, 0]);

    let mut backend = RecordingBackend::default();
    let mut capturer = capturer_queue(vec![hit, miss]);
    let resolver = SEARCH_FIXED_AREA;
    let mut macro_ = quiet_macro(vec![find_pixel_action(
        ActionId::new(),
        "#00ff00",
        detection_branch(None, Some(11), wait_while_found(5, 1), Default::default()),
    )]);

    run_search(&mut macro_, &mut backend, &mut capturer, &resolver);

    assert_eq!(
        capturer.log.len(),
        2,
        "expected hit then miss: {:?}",
        capturer.log
    );
    assert_eq!(
        count_sleeps(&backend, 11),
        1,
        "expected no-find branch once after wait: {:?}",
        backend.log
    );
}

#[test]
fn find_pixel_repeat_until_found_runs_no_find_then_hit() {
    let miss = solid_rgba(8, 8, [0, 0, 0]);
    let mut hit = solid_rgba(8, 8, [0, 0, 0]);
    hit.put_pixel(2, 2, Rgba([0, 255, 0, 255]));

    let mut backend = RecordingBackend::default();
    let mut capturer = capturer_queue(vec![miss, hit]);
    let resolver = SEARCH_FIXED_AREA;
    let mut macro_ = quiet_macro(vec![find_pixel_action(
        ActionId::new(),
        "#00ff00",
        detection_branch(Some(9), Some(9), until_found(1, 5), Default::default()),
    )]);

    run_search(&mut macro_, &mut backend, &mut capturer, &resolver);

    assert_eq!(
        capturer.log.len(),
        2,
        "expected miss then hit: {:?}",
        capturer.log
    );
    assert_eq!(
        count_sleeps(&backend, 9),
        2,
        "expected no-find then hit branch: {:?}",
        backend.log
    );
}

#[test]
fn find_pixel_repeat_until_found_skips_no_find_without_flag() {
    let miss = solid_rgba(8, 8, [0, 0, 0]);
    let mut hit = solid_rgba(8, 8, [0, 0, 0]);
    hit.put_pixel(2, 2, Rgba([0, 255, 0, 255]));

    let mut backend = RecordingBackend::default();
    let mut capturer = capturer_queue(vec![miss, hit]);
    let resolver = SEARCH_FIXED_AREA;
    let mut macro_ = quiet_macro(vec![find_pixel_action(
        ActionId::new(),
        "#00ff00",
        detection_branch(Some(9), None, until_found(1, 5), Default::default()),
    )]);

    run_search(&mut macro_, &mut backend, &mut capturer, &resolver);

    assert_eq!(capturer.log.len(), 2, "{:?}", capturer.log);
    assert_eq!(
        count_sleeps(&backend, 9),
        1,
        "expected only hit branch: {:?}",
        backend.log
    );
}

#[test]
fn find_pixel_runs_children_per_cluster() {
    let mut img = solid_rgba(40, 20, [0, 0, 0]);
    // Two far-apart green blobs (beyond default close-match distance of 10).
    img.put_pixel(2, 2, Rgba([0, 255, 0, 255]));
    img.put_pixel(3, 2, Rgba([0, 255, 0, 255]));
    img.put_pixel(30, 10, Rgba([0, 255, 0, 255]));
    img.put_pixel(31, 10, Rgba([0, 255, 0, 255]));

    let mut backend = RecordingBackend::default();
    let mut capturer = capturer_next(img);
    let resolver = SEARCH_FIXED_AREA;
    let mut macro_ = quiet_macro(vec![find_pixel_action(
        ActionId::new(),
        "#00ff00",
        detection_branch(Some(6), None, Default::default(), Default::default()),
    )]);

    run_search(&mut macro_, &mut backend, &mut capturer, &resolver);
    assert_eq!(
        count_sleeps(&backend, 6),
        2,
        "expected one child run per clustered blob: {:?}",
        backend.log
    );
}
