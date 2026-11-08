use super::*;

fn bag_search(
    targets: Vec<String>,
    area: CoordinateRef,
    wait_ms: i64,
) -> sqyre_domain::ImageSearchData {
    sqyre_domain::ImageSearchData {
        name: "bag".into(),
        targets,
        search_area: area,
        tolerance: 0.7,
        detection: sqyre_domain::DetectionBranch {
            coords: coords_xy("foundX", "foundY"),
            subactions: vec![wait_child(wait_ms)],
            ..Default::default()
        },
        ..Default::default()
    }
}

#[test]
fn image_search_collection_matches_inside_cell_not_across() {
    sqyre_vision::with_search_cache_test_lock(|| {
        sqyre_vision::reset_search_cache_for_testing();
        let dir = tempfile::tempdir().unwrap();
        let tmpl_path = dir.path().join("Item.png");
        let tmpl = patterned_rgba(12, 12, 21);
        tmpl.save(&tmpl_path).unwrap();

        let icons = icons_for(vec![(
            "Prog~Item",
            vec![tmpl_path],
            Some(item_meta("Item", 1, 1, 1, &[])),
        )]);
        let resolver = bag_2x2_resolver();
        let search_area = CoordinateRef::collection("Demo", "bag", 1, 1, 2, 2);

        let mut in_cell = RgbaImage::from_pixel(40, 40, Rgba([20, 20, 20, 255]));
        stamp_rgba(&mut in_cell, &tmpl, 4, 4);
        let mut backend = RecordingBackend::default();
        let mut capturer = capturer_next(in_cell);
        let logger = SharedActionLog::new();
        let search_id = ActionId::new();
        let mut macro_ = fast_macro_of(vec![Action {
            id: search_id,
            kind: ActionKind::ImageSearch(Box::new(sqyre_domain::ImageSearchData {
                ..bag_search(vec!["Prog~Item".into()], search_area.clone(), 4)
            })),
        }]);
        execute_macro_with(
            &mut macro_,
            ExecDeps::new(&mut backend)
                .capturer(&mut capturer)
                .resolver(&resolver)
                .icons(&icons)
                .logger(&logger)
                .close_matches_distance(8),
        )
        .unwrap();
        assert!(
            backend.has("sleep:4"),
            "in-cell 1x1 item should match: {:?}",
            backend.log
        );
        let Some(ScalarValue::Int(fx)) = macro_.variables.get("foundX").cloned() else {
            panic!(
                "expected foundX int, got {:?}",
                macro_.variables.get("foundX")
            );
        };
        let Some(ScalarValue::Int(fy)) = macro_.variables.get("foundY").cloned() else {
            panic!(
                "expected foundY int, got {:?}",
                macro_.variables.get("foundY")
            );
        };
        assert!(
            (0..20).contains(&fx) && (0..20).contains(&fy),
            "expected hit in cell 1,1, got ({fx},{fy})"
        );
        let lines = lines_for(&logger.entries_for(search_id));
        assert!(
            lines
                .iter()
                .any(|l| l.contains("footprint 1x1 → 4 placement(s)")),
            "{lines:?}"
        );

        sqyre_vision::reset_search_cache_for_testing();
        let mut straddle = RgbaImage::from_pixel(40, 40, Rgba([20, 20, 20, 255]));
        stamp_rgba(&mut straddle, &tmpl, 14, 4);
        let mut backend = RecordingBackend::default();
        let mut capturer = capturer_next(straddle);
        let mut macro_ = fast_macro_of(vec![Action::new(ActionKind::ImageSearch(Box::new(
            sqyre_domain::ImageSearchData {
                ..bag_search(vec!["Prog~Item".into()], search_area, 4)
            },
        )))]);
        execute_macro_with(
            &mut macro_,
            ExecDeps::new(&mut backend)
                .capturer(&mut capturer)
                .resolver(&resolver)
                .icons(&icons)
                .close_matches_distance(8),
        )
        .unwrap();
        assert!(
            macro_.variables.get("foundX").is_none(),
            "straddling template should not match a 1x1 cell search"
        );
        assert_did_not_sleep(&backend, 4);
    });
}

#[test]
fn image_search_collection_uses_item_grid_footprint() {
    sqyre_vision::with_search_cache_test_lock(|| {
        sqyre_vision::reset_search_cache_for_testing();
        let dir = tempfile::tempdir().unwrap();
        let tmpl_path = dir.path().join("Item.png");
        // Larger than one 20×20 cell; fits the 2×2 (40×40) footprint.
        let tmpl = patterned_rgba(28, 28, 33);
        tmpl.save(&tmpl_path).unwrap();

        let icons = icons_for(vec![(
            "Prog~Item",
            vec![tmpl_path],
            Some(item_meta("Item", 1, 2, 2, &[])),
        )]);
        let mut screen = RgbaImage::from_pixel(40, 40, Rgba([20, 20, 20, 255]));
        stamp_rgba(&mut screen, &tmpl, 6, 6);
        let mut backend = RecordingBackend::default();
        let mut capturer = capturer_next(screen);
        let resolver = bag_2x2_resolver();
        let logger = SharedActionLog::new();
        let search_id = ActionId::new();
        let mut macro_ = fast_macro_of(vec![Action {
            id: search_id,
            kind: ActionKind::ImageSearch(Box::new(sqyre_domain::ImageSearchData {
                ..bag_search(
                    vec!["Prog~Item".into()],
                    CoordinateRef::collection("Demo", "bag", 1, 1, 2, 2),
                    6,
                )
            })),
        }]);
        execute_macro_with(
            &mut macro_,
            ExecDeps::new(&mut backend)
                .capturer(&mut capturer)
                .resolver(&resolver)
                .icons(&icons)
                .logger(&logger)
                .close_matches_distance(8),
        )
        .unwrap();
        assert!(
            backend.has("sleep:6"),
            "2x2 item should match its 2x2 footprint: {:?}",
            backend.log
        );
        let lines = lines_for(&logger.entries_for(search_id));
        assert!(
            lines
                .iter()
                .any(|l| l.contains("footprint 2x2 → 1 placement(s)")),
            "{lines:?}"
        );
        let Some(ScalarValue::Int(fx)) = macro_.variables.get("foundX").cloned() else {
            panic!(
                "expected foundX int, got {:?}",
                macro_.variables.get("foundX")
            );
        };
        let Some(ScalarValue::Int(fy)) = macro_.variables.get("foundY").cloned() else {
            panic!(
                "expected foundY int, got {:?}",
                macro_.variables.get("foundY")
            );
        };
        assert!(
            (0..40).contains(&fx) && (0..40).contains(&fy),
            "expected hit in 2x2 footprint, got ({fx},{fy})"
        );
    });
}

#[test]
fn image_search_collection_prunes_occupied_cells() {
    sqyre_vision::with_search_cache_test_lock(|| {
        sqyre_vision::reset_search_cache_for_testing();
        let dir = tempfile::tempdir().unwrap();
        let path_a = dir.path().join("ItemA.png");
        let path_b = dir.path().join("ItemB.png");
        // Identical templates: without pruning both would claim the same cell.
        let tmpl = patterned_rgba(12, 12, 44);
        tmpl.save(&path_a).unwrap();
        tmpl.save(&path_b).unwrap();

        let icons = icons_for(vec![
            (
                "Prog~ItemA",
                vec![path_a],
                Some(item_meta("ItemA", 1, 1, 1, &[])),
            ),
            (
                "Prog~ItemB",
                vec![path_b],
                Some(item_meta("ItemB", 1, 1, 1, &[])),
            ),
        ]);
        let mut screen = RgbaImage::from_pixel(40, 40, Rgba([20, 20, 20, 255]));
        stamp_rgba(&mut screen, &tmpl, 4, 4); // cell 1,1
        let mut backend = RecordingBackend::default();
        let mut capturer = capturer_next(screen);
        let resolver = bag_2x2_resolver();
        let logger = SharedActionLog::new();
        let search_id = ActionId::new();
        let mut macro_ = fast_macro_of(vec![Action {
            id: search_id,
            kind: ActionKind::ImageSearch(Box::new(sqyre_domain::ImageSearchData {
                ..bag_search(
                    vec!["Prog~ItemA".into(), "Prog~ItemB".into()],
                    CoordinateRef::collection("Demo", "bag", 1, 1, 2, 2),
                    3,
                )
            })),
        }]);
        execute_macro_with(
            &mut macro_,
            ExecDeps::new(&mut backend)
                .capturer(&mut capturer)
                .resolver(&resolver)
                .icons(&icons)
                .logger(&logger)
                .close_matches_distance(8),
        )
        .unwrap();
        // Only one occupant: first target claims the cell; second is pruned.
        assert_eq!(
            count_sleeps(&backend, 3),
            1,
            "expected a single hit after cell claim: {:?}",
            backend.log
        );
        let lines = lines_for(&logger.entries_for(search_id));
        assert!(
            lines.iter().any(|l| l.contains("Prog~ItemB")
                && l.contains("skipped")
                && l.contains("occupied")),
            "expected occupied-cell prune log for ItemB: {lines:?}"
        );
    });
}

#[test]
fn image_search_collection_respects_manual_search_order() {
    sqyre_vision::with_search_cache_test_lock(|| {
        sqyre_vision::reset_search_cache_for_testing();
        let dir = tempfile::tempdir().unwrap();
        let path_a = dir.path().join("ItemA.png");
        let path_b = dir.path().join("ItemB.png");
        let tmpl = patterned_rgba(12, 12, 44);
        tmpl.save(&path_a).unwrap();
        tmpl.save(&path_b).unwrap();

        let icons = icons_for(vec![
            (
                "Prog~ItemA",
                vec![path_a],
                Some(item_meta("ItemA", 1, 1, 1, &[])),
            ),
            (
                "Prog~ItemB",
                vec![path_b],
                Some(item_meta("ItemB", 1, 1, 1, &[])),
            ),
        ]);
        let mut screen = RgbaImage::from_pixel(40, 40, Rgba([20, 20, 20, 255]));
        stamp_rgba(&mut screen, &tmpl, 4, 4);
        let mut backend = RecordingBackend::default();
        let mut capturer = capturer_next(screen);
        let resolver = bag_2x2_resolver();
        let logger = SharedActionLog::new();
        let search_id = ActionId::new();
        let mut macro_ = fast_macro_of(vec![Action {
            id: search_id,
            kind: ActionKind::ImageSearch(Box::new(sqyre_domain::ImageSearchData {
                sort_by: sqyre_domain::ItemSortBy::Manual,
                // Manual order: B before A (NameAsc would be A then B).
                ..bag_search(
                    vec!["Prog~ItemB".into(), "Prog~ItemA".into()],
                    CoordinateRef::collection("Demo", "bag", 1, 1, 2, 2),
                    3,
                )
            })),
        }]);
        execute_macro_with(
            &mut macro_,
            ExecDeps::new(&mut backend)
                .capturer(&mut capturer)
                .resolver(&resolver)
                .icons(&icons)
                .logger(&logger)
                .close_matches_distance(8),
        )
        .unwrap();
        assert_eq!(count_sleeps(&backend, 3), 1, "{:?}", backend.log);
        let lines = lines_for(&logger.entries_for(search_id));
        assert!(
            lines.iter().any(|l| l.contains("Prog~ItemA")
                && l.contains("skipped")
                && l.contains("occupied")),
            "Manual order should let ItemB claim first: {lines:?}"
        );
    });
}

#[test]
fn image_search_collection_variant_early_exit_is_per_placement() {
    sqyre_vision::with_search_cache_test_lock(|| {
        sqyre_vision::reset_search_cache_for_testing();
        let dir = tempfile::tempdir().unwrap();
        let default_path = dir.path().join("Item.png");
        let alt_path = dir.path().join("Item~Alt.png");
        let default_tmpl = patterned_rgba(12, 12, 11);
        let alt_tmpl = patterned_rgba(12, 12, 77);
        default_tmpl.save(&default_path).unwrap();
        alt_tmpl.save(&alt_path).unwrap();

        let icons = icons_for(vec![(
            "Prog~Item",
            vec![default_path.clone(), alt_path.clone()],
            Some(item_meta("Item", 1, 1, 1, &[])),
        )]);
        // Cell 1,1 = Default; cell 2,2 = Alt. Early-exit must not be global:
        // finding Default in one cell must still allow Alt in another.
        let mut screen = RgbaImage::from_pixel(40, 40, Rgba([20, 20, 20, 255]));
        stamp_rgba(&mut screen, &default_tmpl, 4, 4);
        stamp_rgba(&mut screen, &alt_tmpl, 24, 24);
        let mut backend = RecordingBackend::default();
        let mut capturer = capturer_next(screen);
        let resolver = bag_2x2_resolver();
        let logger = SharedActionLog::new();
        let search_id = ActionId::new();
        let mut macro_ = fast_macro_of(vec![Action {
            id: search_id,
            kind: ActionKind::ImageSearch(Box::new(sqyre_domain::ImageSearchData {
                ..bag_search(
                    vec!["Prog~Item".into()],
                    CoordinateRef::collection("Demo", "bag", 1, 1, 2, 2),
                    5,
                )
            })),
        }]);
        execute_macro_with(
            &mut macro_,
            ExecDeps::new(&mut backend)
                .capturer(&mut capturer)
                .resolver(&resolver)
                .icons(&icons)
                .logger(&logger)
                .close_matches_distance(8),
        )
        .unwrap();
        assert_eq!(
            count_sleeps(&backend, 5),
            2,
            "expected Default and Alt hits in separate cells: {:?}",
            backend.log
        );
        let lines = lines_for(&logger.entries_for(search_id));
        assert!(
            lines
                .iter()
                .any(|l| l.contains("Prog~Item~Alt") && l.contains("match")),
            "Alt must still be searched on other cells: {lines:?}"
        );
    });
}
