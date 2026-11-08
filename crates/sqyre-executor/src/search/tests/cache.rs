use super::*;

#[test]
fn image_search_caches_blurred_templates() {
    sqyre_vision::with_search_cache_test_lock(|| {
        sqyre_vision::reset_search_cache_for_testing();
        let dir = tempfile::tempdir().unwrap();
        let tmpl_path = dir.path().join("tmpl.png");
        let tmpl = RgbaImage::from_pixel(8, 8, Rgba([255, 0, 0, 255]));
        tmpl.save(&tmpl_path).unwrap();

        let icons = icons_for(vec![("Prog~Item", vec![tmpl_path.clone()], None)]);
        let search = RgbaImage::from_pixel(16, 16, Rgba([0, 255, 0, 255]));
        let mut backend = RecordingBackend::default();
        let mut capturer = capturer_next(search);
        let resolver = SEARCH_FIXED_AREA;
        let close_matches = 17;
        let search_id = ActionId::new();
        let mut macro_ = fast_macro_of(vec![Action {
            id: search_id,
            kind: ActionKind::ImageSearch(Box::new(sqyre_domain::ImageSearchData {
                name: "find".into(),
                targets: vec!["Prog~Item".into()],
                search_area: CoordinateRef("Prog~Box".into()),
                tolerance: 0.99,
                blur: 5,
                ..Default::default()
            })),
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

        let kernel = search_blur_kernel(5);
        let first = get_cached_blurred_template(&tmpl_path, kernel).unwrap();
        let second = get_cached_blurred_template(&tmpl_path, kernel).unwrap();
        assert!(std::sync::Arc::ptr_eq(&first, &second));
    });
}

#[test]
fn nested_image_search_after_click_captures_fresh_screen() {
    sqyre_vision::with_search_cache_test_lock(|| {
        sqyre_vision::reset_search_cache_for_testing();
        let dir = tempfile::tempdir().unwrap();
        let parent_tmpl_path = dir.path().join("parent.png");
        let nested_tmpl_path = dir.path().join("nested.png");
        let parent_tmpl = patterned_rgba(10, 10, 11);
        let nested_tmpl = patterned_rgba(8, 8, 33);
        parent_tmpl.save(&parent_tmpl_path).unwrap();
        nested_tmpl.save(&nested_tmpl_path).unwrap();

        let mut parent_only = RgbaImage::from_pixel(50, 50, Rgba([30, 30, 30, 255]));
        stamp_rgba(&mut parent_only, &parent_tmpl, 15, 18);
        let mut nested_visible = parent_only.clone();
        stamp_rgba(&mut nested_visible, &nested_tmpl, 25, 28);

        let icons = icons_for(vec![
            ("Prog~Parent", vec![parent_tmpl_path], None),
            ("Prog~Nested", vec![nested_tmpl_path], None),
        ]);
        let mut backend = RecordingBackend::default();
        let mut capturer = capturer_queue(vec![parent_only, nested_visible]);
        let resolver = SEARCH_FIXED_AREA;
        let close_matches = 8;
        let mut macro_ = fast_macro_of(vec![Action::new(ActionKind::ImageSearch(Box::new(
            sqyre_domain::ImageSearchData {
                name: "parent".into(),
                targets: vec!["Prog~Parent".into()],
                search_area: CoordinateRef("Prog~Box".into()),
                tolerance: 0.7,
                detection: sqyre_domain::DetectionBranch {
                    subactions: vec![
                        Action::new(ActionKind::Click {
                            button: MouseButton::Left,
                            state: PressState::Tap,
                        }),
                        Action::new(ActionKind::ImageSearch(Box::new(
                            sqyre_domain::ImageSearchData {
                                name: "nested".into(),
                                targets: vec!["Prog~Nested".into()],
                                search_area: CoordinateRef("Prog~Box".into()),
                                tolerance: 0.7,
                                detection: sqyre_domain::DetectionBranch {
                                    coords: coords_xy("nestedX", "nestedY"),
                                    subactions: vec![wait_child(7)],
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        ))),
                    ],
                    ..Default::default()
                },
                ..Default::default()
            },
        )))]);

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
        assert_eq!(
            fresh_captures, 2,
            "parent and post-click nested search should each capture fresh: {:?}",
            capturer.log
        );
        assert!(
            backend.has("sleep:7"),
            "nested search should find on the updated screen: {:?}",
            backend.log
        );
        assert!(macro_.variables.get("nestedX").is_some());
    });
}

#[test]
fn nested_image_search_without_input_reuses_cache() {
    sqyre_vision::with_search_cache_test_lock(|| {
        sqyre_vision::reset_search_cache_for_testing();
        let dir = tempfile::tempdir().unwrap();
        let parent_tmpl_path = dir.path().join("parent.png");
        let nested_tmpl_path = dir.path().join("nested.png");
        let parent_tmpl = patterned_rgba(10, 10, 11);
        let nested_tmpl = patterned_rgba(8, 8, 33);
        parent_tmpl.save(&parent_tmpl_path).unwrap();
        nested_tmpl.save(&nested_tmpl_path).unwrap();

        // Both templates visible on one frame — nested must reuse the cache crop.
        let mut both = RgbaImage::from_pixel(50, 50, Rgba([30, 30, 30, 255]));
        stamp_rgba(&mut both, &parent_tmpl, 15, 18);
        stamp_rgba(&mut both, &nested_tmpl, 25, 28);

        let icons = icons_for(vec![
            ("Prog~Parent", vec![parent_tmpl_path], None),
            ("Prog~Nested", vec![nested_tmpl_path], None),
        ]);
        let mut backend = RecordingBackend::default();
        // Same frame for every capture — nested must crop cache, not require a
        // second queued buffer (portal would reuse the last PipeWire crop).
        let mut capturer = capturer_next(both);
        let resolver = SEARCH_FIXED_AREA;
        let close_matches = 8;
        let mut macro_ = fast_macro_of(vec![Action::new(ActionKind::ImageSearch(Box::new(
            sqyre_domain::ImageSearchData {
                name: "parent".into(),
                targets: vec!["Prog~Parent".into()],
                search_area: CoordinateRef("Prog~Box".into()),
                tolerance: 0.7,
                detection: sqyre_domain::DetectionBranch {
                    subactions: vec![Action::new(ActionKind::ImageSearch(Box::new(
                        sqyre_domain::ImageSearchData {
                            name: "nested".into(),
                            targets: vec!["Prog~Nested".into()],
                            search_area: CoordinateRef("Prog~Box".into()),
                            tolerance: 0.7,
                            detection: sqyre_domain::DetectionBranch {
                                coords: coords_xy("nestedX", "nestedY"),
                                subactions: vec![wait_child(7)],
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    )))],
                    ..Default::default()
                },
                ..Default::default()
            },
        )))]);

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
        assert_eq!(
            fresh_captures, 1,
            "nested without input should crop cache, not wait fresh: {:?}",
            capturer.log
        );
        assert!(
            capturer.has_prefix("rect:"),
            "nested should use non-fresh crop: {:?}",
            capturer.log
        );
        assert!(
            backend.has("sleep:7"),
            "nested search should find on the shared cache frame: {:?}",
            backend.log
        );
        assert!(macro_.variables.get("nestedX").is_some());
    });
}

#[test]
fn run_matches_clears_coords_on_miss() {
    let mut backend = RecordingBackend::default();
    let mut exec = crate::run::Executor::new(&mut backend);
    let mut macro_ = Macro::new("t", 0, vec![]);
    macro_.variables.set("foundX", ScalarValue::Int(9));
    macro_.variables.set("foundY", ScalarValue::Int(8));
    let coords = coords_xy("foundX", "foundY");
    let branch = sqyre_domain::DetectionBranch {
        coords,
        ..Default::default()
    };
    let area = CoordinateRef::default();
    let ctx = DetectionCtx::new(ActionId::new(), "test", &area, &[], &branch);
    run_matches(&mut exec, &ctx, &[], &mut macro_).unwrap();
    assert!(macro_.variables.get("foundX").is_none());
    assert!(macro_.variables.get("foundY").is_none());
}
