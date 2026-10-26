use super::*;

#[test]
fn image_search_expands_target_tags_at_runtime() {
    sqyre_vision::with_search_cache_test_lock(|| {
        sqyre_vision::reset_search_cache_for_testing();
        let dir = tempfile::tempdir().unwrap();
        let sword_path = dir.path().join("sword.png");
        let potion_path = dir.path().join("potion.png");
        let sword = patterned_rgba(10, 10, 40);
        let potion = patterned_rgba(10, 10, 200);
        sword.save(&sword_path).unwrap();
        potion.save(&potion_path).unwrap();

        let mut search = RgbaImage::from_pixel(50, 50, Rgba([20, 20, 20, 255]));
        stamp_rgba(&mut search, &sword, 12, 14);

        let icons = MapIcons {
            paths: HashMap::from([
                ("Game~Sword".into(), vec![sword_path]),
                ("Game~Potion".into(), vec![potion_path]),
            ]),
            masks: HashMap::new(),
            meta: HashMap::from([
                (
                    "Game~Sword".into(),
                    ItemMeta {
                        name: "Sword".into(),
                        stack_max: 1,
                        cols: 1,
                        rows: 1,
                        tags: vec!["weapon".into()],
                    },
                ),
                (
                    "Game~Potion".into(),
                    ItemMeta {
                        name: "Potion".into(),
                        stack_max: 20,
                        cols: 1,
                        rows: 1,
                        tags: vec!["heal".into()],
                    },
                ),
            ]),
        };
        let mut backend = RecordingBackend::default();
        let mut capturer = RecordingCapturer {
            next: Some(search),
            bounds: full_desktop(),
            ..Default::default()
        };
        let resolver = SEARCH_FIXED_AREA;
        let close_matches = 8;
        let logger = SharedActionLog::new();
        let search_id = ActionId::new();
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.keyboard_delay = 0;
        macro_.mouse_delay = 0;
        macro_.root = root_loop(vec![Action {
            id: search_id,
            kind: ActionKind::ImageSearch {
                name: "by-tag".into(),
                targets: vec![],
                target_tags: vec!["+weapon".into()],
                search_area: CoordinateRef("Prog~Box".into()),
                tolerance: 0.7,
                blur: 0,
                match_method: Default::default(),
                sort_by: Default::default(),
                sort_then: Default::default(),
                tag_priority: Vec::new(),
                detection: sqyre_domain::DetectionBranch {
                    subactions: vec![sqyre_domain::test_action(ActionKind::Wait {
                        time: ScalarValue::Int(3),
                    })],
                    ..Default::default()
                },
            },
        }]);

        execute_macro_with(
            &mut macro_,
            ExecDeps::new(&mut backend)
                .capturer(&mut capturer)
                .resolver(&resolver)
                .icons(&icons)
                .close_matches_distance(close_matches)
                .logger(&logger),
        )
        .unwrap();

        assert!(
            backend.has("sleep:3"),
            "expected tag-expanded Sword match: {:?}",
            backend.log
        );
        assert_eq!(
            macro_.variables.get("ItemName").map(|v| v.as_display()),
            Some("Sword".into())
        );

        // Structured find: one hit near the stamped template at (12,14),
        // screen box = local box + origin (100,200).
        let entries = logger.entries_for(search_id);
        let (steps, finds) = entries
            .iter()
            .find_map(|e| match e {
                crate::ActionLogEntry::ItemPipeline { steps, finds, .. } => Some((steps, finds)),
                _ => None,
            })
            .expect("pipeline logged");
        assert_eq!(finds.len(), 1, "expected one structured find: {finds:?}");
        let f = &finds[0];
        assert_eq!(f.label, "Find #1");
        let (sl, st, sr, sb) = f.screen;
        assert!(
            (sl - 112).abs() <= 1 && (st - 214).abs() <= 1,
            "screen box near stamp+origin: {f:?}"
        );
        assert_eq!(sr - sl, 10);
        assert_eq!(sb - st, 10);
        let sqyre_ports::FindContext::Step { step, rect } =
            f.context.as_ref().expect("overlay context")
        else {
            unreachable!()
        };
        assert_eq!(
            steps[*step].label, "Where found (all matches)",
            "find context should point at the overlay step"
        );
        assert_eq!(
            *rect,
            (sl - 100, st - 200, sr - 100, sb - 200),
            "context rect mirrors the screen box in capture-local space"
        );
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
fn image_search_no_find_runs_branch() {
    sqyre_vision::with_search_cache_test_lock(|| {
        sqyre_vision::reset_search_cache_for_testing();
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
                    tags: Vec::new(),
                },
            )]),
        };
        let mut backend = RecordingBackend::default();
        let mut capturer = RecordingCapturer {
            next: Some(img),
            bounds: full_desktop(),
            ..Default::default()
        };
        let resolver = SEARCH_FIXED_AREA;
        let logger = SharedActionLog::new();
        let search_id = ActionId::new();
        let mut macro_ = quiet_macro(vec![Action {
            id: search_id,
            kind: ActionKind::ImageSearch {
                name: "find".into(),
                targets: vec!["Prog~Item".into()],
                target_tags: Vec::new(),
                search_area: CoordinateRef("Prog~Box".into()),
                tolerance: 0.99,
                blur: 0,
                match_method: Default::default(),
                sort_by: Default::default(),
                sort_then: Default::default(),
                tag_priority: Vec::new(),
                detection: sqyre_domain::DetectionBranch {
                    else_actions: vec![sqyre_domain::test_action(ActionKind::Wait {
                        time: ScalarValue::Int(13),
                    })],
                    ..Default::default()
                },
            },
        }]);

        execute_macro_with(
            &mut macro_,
            ExecDeps::new(&mut backend)
                .capturer(&mut capturer)
                .resolver(&resolver)
                .icons(&icons)
                .logger(&logger),
        )
        .unwrap();

        assert!(backend.has("sleep:13"));
        let lines = lines_for(&logger.entries_for(search_id));
        assert!(
            lines.iter().any(|l| l.contains("Image Search: searching")),
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
    });
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
            kind: ActionKind::LoopJump {
                mode: LoopJumpMode::Break,
            },
        },
    ];
    let branch = sqyre_domain::DetectionBranch {
        subactions,
        ..Default::default()
    };
    let targets = ["a".into(), "b".into()];
    let area = CoordinateRef::default();
    let ctx = DetectionCtx::new(ActionId::new(), "test", &area, &targets, &branch);
    run_matches(&mut exec, &ctx, &results, &mut macro_).unwrap();
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
    // blur=0 means no blur (kernel 0); pattern must still be found unblurred.
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
        sqyre_domain::MatchMethod::CcoeffNormed,
    )
    .unwrap();
    assert!(
        hits.iter()
            .any(|p| (p.x - 15).abs() <= 2 && (p.y - 18).abs() <= 2),
        "expected peak near (15,18), got {hits:?}"
    );
}

#[test]
fn find_template_matches_sqdiff_normed_peak() {
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
    let hits = sqyre_match::find_template_matches_preblurred(
        &search,
        &tmpl,
        None,
        0.1,
        DEFAULT_CLOSE_MATCHES_DISTANCE,
        sqyre_domain::MatchMethod::SqdiffNormed,
    )
    .unwrap();
    assert!(
        hits.iter()
            .any(|p| (p.x - 15).abs() <= 1 && (p.y - 18).abs() <= 1),
        "expected SQDIFF_NORMED peak near (15,18), got {hits:?}"
    );
}

#[test]
fn image_search_wait_until_found_retries_then_succeeds() {
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
            bounds: full_desktop(),
            ..Default::default()
        };
        let resolver = SEARCH_FIXED_AREA;
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
                target_tags: Vec::new(),
                search_area: CoordinateRef("Prog~Box".into()),
                tolerance: 0.7,
                blur: 0,
                match_method: Default::default(),
                sort_by: Default::default(),
                sort_then: Default::default(),
                tag_priority: Vec::new(),
                detection: sqyre_domain::DetectionBranch {
                    wait: wait_until_found(5, 1),
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
            ExecDeps::new(&mut backend)
                .capturer(&mut capturer)
                .resolver(&resolver)
                .icons(&icons)
                .close_matches_distance(close_matches),
        )
        .unwrap();

        assert!(
            capturer.log.len() >= 2,
            "expected miss then hit capture: {:?}",
            capturer.log
        );
        assert!(
            capturer
                .log
                .iter()
                .filter(|e| e.starts_with("rgb_fresh:"))
                .count()
                >= 2,
            "image search should fresh-capture on first attempt and on wait retry: {:?}",
            capturer.log
        );
        assert!(
            backend.has("sleep:3"),
            "expected child on find: {:?}",
            backend.log
        );
        assert!(macro_.variables.get("foundX").is_some());
    });
}

#[test]
fn image_search_wait_until_found_stops_when_flag_set() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    sqyre_vision::with_search_cache_test_lock(|| {
        sqyre_vision::reset_search_cache_for_testing();
        let dir = tempfile::tempdir().unwrap();
        let tmpl_path = dir.path().join("tmpl.png");
        patterned_rgba(10, 10, 11).save(&tmpl_path).unwrap();
        let miss = RgbaImage::from_pixel(50, 50, Rgba([0, 255, 0, 255]));

        let icons = MapIcons {
            paths: HashMap::from([("Prog~Item".into(), vec![tmpl_path])]),
            masks: HashMap::new(),
            meta: HashMap::new(),
        };
        let stop = Arc::new(AtomicBool::new(false));
        let stop_for_backend = Arc::clone(&stop);

        let mut backend = StopAfterSleeps::new(stop_for_backend, 1);
        let mut capturer = RecordingCapturer {
            next: Some(miss),
            bounds: full_desktop(),
            ..Default::default()
        };
        let resolver = SEARCH_FIXED_AREA;
        let mut macro_ = quiet_macro(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::ImageSearch {
                name: "find".into(),
                targets: vec!["Prog~Item".into()],
                target_tags: Vec::new(),
                search_area: CoordinateRef("Prog~Box".into()),
                tolerance: 0.7,
                blur: 0,
                match_method: Default::default(),
                sort_by: Default::default(),
                sort_then: Default::default(),
                tag_priority: Vec::new(),
                detection: sqyre_domain::DetectionBranch {
                    wait: wait_until_found(30, 50),
                    subactions: vec![sqyre_domain::test_action(ActionKind::Wait {
                        time: ScalarValue::Int(3),
                    })],
                    ..Default::default()
                },
            },
        }]);

        execute_macro_with(
            &mut macro_,
            ExecDeps::new(&mut backend)
                .capturer(&mut capturer)
                .resolver(&resolver)
                .icons(&icons)
                .stop_flag(stop.as_ref()),
        )
        .unwrap();

        assert!(stop.load(Ordering::SeqCst));
        assert!(backend.sleeps >= 1);
        assert!(
            !backend.inner.has("sleep:3"),
            "stop should abort before running find children: {:?}",
            backend.inner.log
        );
    });
}

#[test]
fn image_search_wait_until_found_runs_final_search_on_timeout() {
    sqyre_vision::with_search_cache_test_lock(|| {
        sqyre_vision::reset_search_cache_for_testing();
        let dir = tempfile::tempdir().unwrap();
        let tmpl_path = dir.path().join("tmpl.png");
        let tmpl = patterned_rgba(10, 10, 33);
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
            queue: vec![miss.clone(), miss.clone(), miss.clone(), miss, hit],
            next: None,
            bounds: full_desktop(),
            ..Default::default()
        };
        let resolver = SEARCH_FIXED_AREA;
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
                target_tags: Vec::new(),
                search_area: CoordinateRef("Prog~Box".into()),
                tolerance: 0.7,
                blur: 0,
                match_method: Default::default(),
                sort_by: Default::default(),
                sort_then: Default::default(),
                tag_priority: Vec::new(),
                detection: sqyre_domain::DetectionBranch {
                    wait: wait_until_found(0.15, 30),
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
            ExecDeps::new(&mut backend)
                .capturer(&mut capturer)
                .resolver(&resolver)
                .icons(&icons)
                .close_matches_distance(close_matches),
        )
        .unwrap();

        let fresh_captures = capturer
            .log
            .iter()
            .filter(|e| e.starts_with("rgb_fresh:"))
            .count();
        assert!(
            fresh_captures >= 2,
            "expected wait retries plus a final timeout search: {:?}",
            capturer.log
        );
        assert!(
            backend.has("sleep:3"),
            "final search should find the template after wait timeout: {:?}",
            backend.log
        );
        assert!(macro_.variables.get("foundX").is_some());
    });
}

#[test]
fn image_search_repeat_while_found_then_stops() {
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
            bounds: full_desktop(),
            ..Default::default()
        };
        let resolver = SEARCH_FIXED_AREA;
        let close_matches = 8;
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.keyboard_delay = 0;
        macro_.mouse_delay = 0;
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::ImageSearch {
                name: "loop".into(),
                targets: vec!["Prog~Item".into()],
                target_tags: Vec::new(),
                search_area: CoordinateRef("Prog~Box".into()),
                tolerance: 0.7,
                blur: 0,
                match_method: Default::default(),
                sort_by: Default::default(),
                sort_then: Default::default(),
                tag_priority: Vec::new(),
                detection: sqyre_domain::DetectionBranch {
                    wait: while_found(1, 5),
                    subactions: vec![sqyre_domain::test_action(ActionKind::Wait {
                        time: ScalarValue::Int(9),
                    })],
                    ..Default::default()
                },
            },
        }]);

        execute_macro_with(
            &mut macro_,
            ExecDeps::new(&mut backend)
                .capturer(&mut capturer)
                .resolver(&resolver)
                .icons(&icons)
                .close_matches_distance(close_matches),
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
        let default_path = dir.path().join("Item.png");
        let alt_path = dir.path().join("Item~Alt.png");
        let first = patterned_rgba(10, 10, 1);
        let second = patterned_rgba(10, 10, 90);
        first.save(&default_path).unwrap();
        second.save(&alt_path).unwrap();

        // Screen contains only the named Alt variant.
        let mut search = RgbaImage::from_pixel(50, 50, Rgba([40, 40, 40, 255]));
        stamp_rgba(&mut search, &second, 15, 18);

        let icons = MapIcons {
            paths: HashMap::from([("Prog~Item".into(), vec![default_path, alt_path])]),
            masks: HashMap::new(),
            meta: HashMap::from([(
                "Prog~Item".into(),
                ItemMeta {
                    name: "Item".into(),
                    stack_max: 3,
                    cols: 1,
                    rows: 1,
                    tags: Vec::new(),
                },
            )]),
        };
        let mut backend = RecordingBackend::default();
        let mut capturer = RecordingCapturer {
            next: Some(search),
            bounds: full_desktop(),
            ..Default::default()
        };
        let resolver = SEARCH_FIXED_AREA;
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
                target_tags: Vec::new(),
                search_area: CoordinateRef("Prog~Box".into()),
                tolerance: 0.7,
                blur: 0,
                match_method: Default::default(),
                sort_by: Default::default(),
                sort_then: Default::default(),
                tag_priority: Vec::new(),
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
            ExecDeps::new(&mut backend)
                .capturer(&mut capturer)
                .resolver(&resolver)
                .icons(&icons)
                .logger(&logger)
                .close_matches_distance(close_matches),
        )
        .unwrap();

        assert!(
            backend.has("sleep:2"),
            "expected match via Alt variant: {:?}",
            backend.log
        );
        assert_eq!(
            macro_.variables.get("StackMax").map(|v| v.as_display()),
            Some("3".into())
        );
        assert_eq!(
            macro_.variables.get("VariantName").map(|v| v.as_display()),
            Some("Alt".into())
        );
        let lines = lines_for(&logger.entries_for(search_id));
        assert!(
            lines.iter().any(|l| l.contains("Prog~Item~Alt")),
            "expected named variant in logs: {lines:?}"
        );
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
            bounds: full_desktop(),
            ..Default::default()
        };
        let resolver = SEARCH_FIXED_AREA;
        let close_matches = 8;
        let mut macro_ = Macro::new("t", 0, vec![]);
        macro_.keyboard_delay = 0;
        macro_.mouse_delay = 0;
        macro_.root = root_loop(vec![Action {
            id: ActionId::new(),
            kind: ActionKind::ImageSearch {
                name: "masked".into(),
                targets: vec!["Prog~Item".into()],
                target_tags: Vec::new(),
                search_area: CoordinateRef("Prog~Box".into()),
                tolerance: 0.7,
                blur: 0,
                match_method: Default::default(),
                sort_by: Default::default(),
                sort_then: Default::default(),
                tag_priority: Vec::new(),
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
            ExecDeps::new(&mut backend)
                .capturer(&mut capturer)
                .resolver(&resolver)
                .icons(&icons)
                .close_matches_distance(close_matches),
        )
        .unwrap();

        assert!(
            backend.has("sleep:5"),
            "match with mask path should succeed: {:?}",
            backend.log
        );
        assert!(macro_.variables.get("foundX").is_some());
    });
}

#[test]
fn image_search_variant_early_exit_skips_later_variants_after_hit() {
    sqyre_vision::with_search_cache_test_lock(|| {
        sqyre_vision::reset_search_cache_for_testing();
        let dir = tempfile::tempdir().unwrap();
        let default_path = dir.path().join("Item.png");
        let alt_path = dir.path().join("Item~Alt.png");
        let default_tmpl = patterned_rgba(10, 10, 11);
        let alt_tmpl = patterned_rgba(10, 10, 90);
        default_tmpl.save(&default_path).unwrap();
        alt_tmpl.save(&alt_path).unwrap();

        let mut search = RgbaImage::from_pixel(50, 50, Rgba([40, 40, 40, 255]));
        stamp_rgba(&mut search, &default_tmpl, 15, 18);

        let icons = MapIcons {
            paths: HashMap::from([("Prog~Item".into(), vec![default_path, alt_path])]),
            masks: HashMap::new(),
            meta: HashMap::from([(
                "Prog~Item".into(),
                ItemMeta {
                    name: "Item".into(),
                    stack_max: 1,
                    cols: 1,
                    rows: 1,
                    tags: Vec::new(),
                },
            )]),
        };
        let mut backend = RecordingBackend::default();
        let mut capturer = RecordingCapturer {
            next: Some(search),
            bounds: full_desktop(),
            ..Default::default()
        };
        let logger = SharedActionLog::new();
        let search_id = ActionId::new();
        let mut macro_ = quiet_macro(vec![Action {
            id: search_id,
            kind: ActionKind::ImageSearch {
                name: "multi".into(),
                targets: vec!["Prog~Item".into()],
                target_tags: Vec::new(),
                search_area: CoordinateRef("Prog~Box".into()),
                tolerance: 0.7,
                blur: 0,
                match_method: Default::default(),
                sort_by: Default::default(),
                sort_then: Default::default(),
                tag_priority: Vec::new(),
                detection: sqyre_domain::DetectionBranch {
                    coords: CoordinateOutputs {
                        output_x_variable: "foundX".into(),
                        output_y_variable: "foundY".into(),
                    },
                    subactions: vec![wait_child(2)],
                    ..Default::default()
                },
            },
        }]);
        execute_macro_with(
            &mut macro_,
            ExecDeps::new(&mut backend)
                .capturer(&mut capturer)
                .resolver(&SEARCH_FIXED_AREA)
                .icons(&icons)
                .logger(&logger)
                .close_matches_distance(8),
        )
        .unwrap();
        assert!(
            backend.has("sleep:2"),
            "Default variant should match: {:?}",
            backend.log
        );
        let lines = lines_for(&logger.entries_for(search_id));
        assert!(
            lines
                .iter()
                .any(|l| l.contains("Prog~Item") && l.contains("match")),
            "{lines:?}"
        );
        assert!(
            lines.iter().all(|l| !l.contains("Prog~Item~Alt")),
            "Alt must not run after Default hits: {lines:?}"
        );
    });
}

#[test]
fn image_search_variant_exit_early_disabled_tries_all_variants() {
    sqyre_vision::with_search_cache_test_lock(|| {
        sqyre_vision::reset_search_cache_for_testing();
        let dir = tempfile::tempdir().unwrap();
        let default_path = dir.path().join("Item.png");
        let alt_path = dir.path().join("Item~Alt.png");
        let default_tmpl = patterned_rgba(10, 10, 11);
        let alt_tmpl = patterned_rgba(10, 10, 90);
        default_tmpl.save(&default_path).unwrap();
        alt_tmpl.save(&alt_path).unwrap();

        let mut search = RgbaImage::from_pixel(50, 50, Rgba([40, 40, 40, 255]));
        stamp_rgba(&mut search, &default_tmpl, 15, 18);

        let icons = MapIcons {
            paths: HashMap::from([("Prog~Item".into(), vec![default_path, alt_path])]),
            masks: HashMap::new(),
            meta: HashMap::from([(
                "Prog~Item".into(),
                ItemMeta {
                    name: "Item".into(),
                    stack_max: 1,
                    cols: 1,
                    rows: 1,
                    tags: Vec::new(),
                },
            )]),
        };
        let mut backend = RecordingBackend::default();
        let mut capturer = RecordingCapturer {
            next: Some(search),
            bounds: full_desktop(),
            ..Default::default()
        };
        let logger = SharedActionLog::new();
        let search_id = ActionId::new();
        let mut macro_ = quiet_macro(vec![Action {
            id: search_id,
            kind: ActionKind::ImageSearch {
                name: "multi".into(),
                targets: vec!["Prog~Item".into()],
                target_tags: Vec::new(),
                search_area: CoordinateRef("Prog~Box".into()),
                tolerance: 0.7,
                blur: 0,
                match_method: Default::default(),
                sort_by: Default::default(),
                sort_then: Default::default(),
                tag_priority: Vec::new(),
                detection: sqyre_domain::DetectionBranch {
                    coords: CoordinateOutputs {
                        output_x_variable: "foundX".into(),
                        output_y_variable: "foundY".into(),
                    },
                    subactions: vec![wait_child(2)],
                    ..Default::default()
                },
            },
        }]);
        execute_macro_with(
            &mut macro_,
            ExecDeps::new(&mut backend)
                .capturer(&mut capturer)
                .resolver(&SEARCH_FIXED_AREA)
                .icons(&icons)
                .logger(&logger)
                .close_matches_distance(8)
                .variant_exit_early(false),
        )
        .unwrap();
        assert!(
            backend.has("sleep:2"),
            "Default variant should match: {:?}",
            backend.log
        );
        let lines = lines_for(&logger.entries_for(search_id));
        assert!(
            lines
                .iter()
                .any(|l| l.contains("Prog~Item") && l.contains("match")),
            "{lines:?}"
        );
        assert!(
            lines
                .iter()
                .any(|l| l.contains("Prog~Item~Alt") && l.contains("matching")),
            "Alt must still be tried when exit-early is off: {lines:?}"
        );
    });
}
