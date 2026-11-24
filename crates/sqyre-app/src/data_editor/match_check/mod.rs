//! MatchCheck: live template-match heatmap over a search-area preview.
//!
//! Legend / marker colors use app theme viz tokens (`match_pass_fg`,
//! `match_fail_fg`, `match_within_fg`) — not status `error_fg` / `warn_fg` /
//! `ok_fg`, which are reserved for chrome and banners.

#[cfg(feature = "native-runtime")]
#[expect(
    clippy::too_many_arguments,
    reason = "pixel-check helpers share LTRB, match settings, and cache payloads"
)]
mod inner;

#[cfg(feature = "native-runtime")]
pub(crate) use inner::*;

#[cfg(all(test, feature = "native-runtime"))]
mod tests {
    use super::inner::{
        arm_live_cycle, closeness_percent, collect_tolerance_matches, coords_displayable,
        display_xy_to_match, fingerprint, heatmap_hover_allowed, hover_at_image, live_refresh_due,
        match_xy_to_display, no_match_box_caption, scores_to_heatmap_rgba, HeatmapStampMap,
        MatchCheckSettings, MatchCheckTool, MatchSummary, LIVE_INTERVAL,
    };
    use sqyre_domain::{MatchMethod, PlaneFlip};
    use sqyre_match::MatchMap;
    use std::sync::mpsc;
    use web_time::{Duration, Instant};

    fn map(w: usize, h: usize, scores: &[f32]) -> MatchMap {
        MatchMap {
            width: w,
            height: h,
            scores: scores.to_vec(),
        }
    }

    fn summary(min: f32, max: f32) -> MatchSummary {
        MatchSummary {
            score_min: min,
            score_max: max,
            ..Default::default()
        }
    }

    #[test]
    fn heatmap_normed_higher_is_better() {
        // Wide image so peak kernels at x=0 and x=10 do not overlap.
        let mut scores = vec![0.0f32; 11];
        scores[0] = 0.5;
        scores[10] = 1.0;
        let map = map(11, 1, &scores);
        let (rgba, summary) = scores_to_heatmap_rgba(
            &map,
            11,
            1,
            1,
            1,
            MatchMethod::CcoeffNormed,
            0.9,
            HeatmapStampMap::display_space(11, 1),
        );
        assert_eq!(rgba.len(), 11 * 4);
        // 0.5 is below viz margin — origin stays unpainted; peak at 10 paints.
        assert_eq!(rgba[3], 0, "weak origin should not paint");
        assert!(rgba[10 * 4 + 3] > 0, "strong peak should paint");
        assert!(summary.best_score > 0.99);
        assert_eq!(summary.best_x, 10);
    }

    #[test]
    fn heatmap_sqdiff_lower_is_better() {
        let mut scores = vec![1.0f32; 11];
        scores[0] = 0.1;
        scores[10] = 0.9;
        let map = map(11, 1, &scores);
        let (rgba, summary) = scores_to_heatmap_rgba(
            &map,
            11,
            1,
            1,
            1,
            MatchMethod::SqdiffNormed,
            0.5,
            HeatmapStampMap::display_space(11, 1),
        );
        assert!(summary.best_score < 0.2);
        assert_eq!(summary.best_x, 0);
        assert!(rgba[3] > 0, "better match peak should paint");
        assert_eq!(
            rgba[10 * 4 + 3],
            0,
            "worse score outside tolerance should not paint"
        );
    }

    #[test]
    fn match_xy_maps_through_area_and_display_scale() {
        // Region peak (4,2) with area origin (10,20) on 100x100 src → 50x50 display.
        let (dx, dy) = match_xy_to_display(4, 2, 1.0, 10.0, 20.0, 100, 100, 50, 50);
        assert_eq!((dx, dy), (7, 11));
        let back = display_xy_to_match(dx, dy, 1.0, 10.0, 20.0, 100, 100, 50, 50).unwrap();
        assert_eq!(back, (4, 2));
    }

    #[test]
    fn match_xy_remaps_half_scale_peaks() {
        let (dx, dy) = match_xy_to_display(4, 2, 0.5, 0.0, 0.0, 40, 20, 40, 20);
        assert_eq!((dx, dy), (8, 4));
    }

    #[test]
    fn heatmap_peak_kernel_stays_compact() {
        let map = map(1, 1, &[1.0]);
        let (rgba, _) = scores_to_heatmap_rgba(
            &map,
            20,
            20,
            8,
            8,
            MatchMethod::CcoeffNormed,
            0.9,
            HeatmapStampMap::display_space(20, 20),
        );
        let painted = rgba.chunks(4).filter(|p| p[3] > 0).count();
        // Soft disk radius 4 → at most (2*4+1)^2 = 81 cells, far below an 8×8 footprint.
        assert!(painted > 0 && painted <= 81, "painted={painted}");
    }

    #[test]
    fn heatmap_fades_weak_scores() {
        let map = map(1, 1, &[0.1]);
        let (rgba, _) = scores_to_heatmap_rgba(
            &map,
            4,
            4,
            2,
            2,
            MatchMethod::CcoeffNormed,
            0.95,
            HeatmapStampMap::display_space(4, 4),
        );
        // Far below tolerance — should not paint any heat.
        let alphas: Vec<u8> = rgba.chunks(4).map(|p| p[3]).collect();
        assert!(
            alphas.iter().all(|&a| a == 0),
            "weak match should not paint: {alphas:?}"
        );
    }

    #[test]
    fn heatmap_suppresses_clustered_weak_scores() {
        // Tight score range that used to normalize to full-span yellow streaks.
        let map = map(4, 1, &[0.38, 0.39, 0.40, 0.41]);
        let (rgba, _) = scores_to_heatmap_rgba(
            &map,
            8,
            2,
            2,
            1,
            MatchMethod::CcoeffNormed,
            0.95,
            HeatmapStampMap::display_space(8, 2),
        );
        let painted = rgba.chunks(4).filter(|p| p[3] > 0).count();
        assert_eq!(painted, 0, "clustered weak scores should not paint");
    }

    #[test]
    fn coords_displayable_rejects_invalid_bounds() {
        assert!(!coords_displayable(None, Some(0), Some(10), Some(10)));
        assert!(!coords_displayable(Some(0), Some(0), Some(0), Some(10)));
        assert!(!coords_displayable(Some(10), Some(0), Some(0), Some(10)));
        assert!(coords_displayable(Some(0), Some(0), Some(10), Some(10)));
    }

    #[test]
    fn hover_out_of_bounds() {
        let map = map(2, 2, &[0.5, 0.6, 0.7, 0.8]);
        let summary = summary(0.5, 0.8);
        assert!(hover_at_image(
            &map,
            &summary,
            -1,
            0,
            1,
            1,
            MatchMethod::CcoeffNormed,
            0.5,
            HeatmapStampMap::display_space(2, 2),
            2,
            2,
        )
        .is_none());
        let h = hover_at_image(
            &map,
            &summary,
            0,
            0,
            1,
            1,
            MatchMethod::CcoeffNormed,
            0.4,
            HeatmapStampMap::display_space(2, 2),
            2,
            2,
        )
        .unwrap();
        assert!((h.closeness_pct - 50.0).abs() < 0.1);
        assert!(h.passes);
    }

    #[test]
    fn closeness_normed_is_absolute_percent() {
        let summary = MatchSummary::default();
        assert!(
            (closeness_percent(0.873, MatchMethod::CcoeffNormed, &summary) - 87.3).abs() < 0.01
        );
    }

    #[test]
    fn hover_pooled_across_template_footprint() {
        let map = map(2, 1, &[0.5, 0.95]);
        let summary = summary(0.5, 0.95);
        // Pixel inside the high-score template block should inherit 0.95.
        let h = hover_at_image(
            &map,
            &summary,
            1,
            0,
            2,
            1,
            MatchMethod::CcoeffNormed,
            0.9,
            HeatmapStampMap::display_space(2, 1),
            2,
            1,
        )
        .unwrap();
        assert!((h.closeness_pct - 95.0).abs() < 0.1);
    }

    #[test]
    fn hover_includes_heatmap_near_miss() {
        // 0.85 is below tolerance 0.95 but within HEATMAP_VIZ_MARGIN (0.12).
        let map = map(2, 1, &[0.85, 0.95]);
        let summary = summary(0.85, 0.95);
        let h = hover_at_image(
            &map,
            &summary,
            0,
            0,
            1,
            1,
            MatchMethod::CcoeffNormed,
            0.95,
            HeatmapStampMap::display_space(2, 1),
            2,
            1,
        )
        .expect("near-miss should be hoverable");
        assert!(!h.passes);
        assert!((h.closeness_pct - 85.0).abs() < 0.1);
    }

    #[test]
    fn hover_skips_far_below_heatmap_margin() {
        let map = map(2, 1, &[0.5, 0.95]);
        let summary = summary(0.5, 0.95);
        assert!(hover_at_image(
            &map,
            &summary,
            0,
            0,
            1,
            1,
            MatchMethod::CcoeffNormed,
            0.95,
            HeatmapStampMap::display_space(2, 1),
            2,
            1,
        )
        .is_none());
    }

    #[test]
    fn collect_tolerance_matches_filters_weak_peaks() {
        let map = map(4, 1, &[0.38, 0.39, 0.96, 0.41]);
        let matches = collect_tolerance_matches(&map, 0.95, 0, MatchMethod::CcoeffNormed);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].x, 2);
        assert!(matches[0].score >= 0.95);
    }

    #[test]
    fn live_refresh_due_respects_pause_pending_and_interval() {
        let now = Instant::now();
        let mut settings = MatchCheckSettings::default();
        assert!(live_refresh_due(&settings, false, now));
        assert!(!live_refresh_due(&settings, true, now));
        settings.paused = true;
        assert!(!live_refresh_due(&settings, false, now));
        settings.paused = false;
        settings.last_live_at = Some(now);
        assert!(!live_refresh_due(&settings, false, now));
        assert!(live_refresh_due(
            &settings,
            false,
            now + LIVE_INTERVAL + Duration::from_millis(1)
        ));
    }

    #[test]
    fn no_match_box_caption_includes_percent_and_status() {
        assert_eq!(no_match_box_caption(87.3), "87.3% · no match");
        assert_eq!(no_match_box_caption(0.0), "0.0% · no match");
    }

    #[test]
    fn heatmap_hover_skips_while_pointer_or_pan_drag_active() {
        assert!(heatmap_hover_allowed(false, false));
        assert!(!heatmap_hover_allowed(true, false));
        assert!(!heatmap_hover_allowed(false, true));
        assert!(!heatmap_hover_allowed(true, true));
    }

    #[test]
    fn arm_live_cycle_is_one_shot_until_flags_clear() {
        let now = Instant::now();
        let mut settings = MatchCheckSettings::default();
        assert!(live_refresh_due(&settings, false, now));
        arm_live_cycle(&mut settings, now);
        assert!(settings.capture_once);
        assert!(settings.awaiting_panel);
        assert!(!settings.rematch_once);
        // Rate-limit + in-flight flags must stop re-arming every frame.
        assert!(!live_refresh_due(&settings, false, now));
        settings.capture_once = false;
        assert!(!live_refresh_due(&settings, false, now));
        settings.awaiting_panel = false;
        assert!(!live_refresh_due(&settings, false, now));
        assert!(live_refresh_due(
            &settings,
            false,
            now + LIVE_INTERVAL + Duration::from_millis(1)
        ));
    }

    #[test]
    fn fingerprint_includes_plane_flip() {
        let none = fingerprint(
            "Prog",
            "Item",
            "",
            0,
            0,
            10,
            10,
            0,
            100,
            MatchMethod::CcoeffNormed,
            PlaneFlip::None,
            0.95,
            0,
            8,
        );
        let horiz = fingerprint(
            "Prog",
            "Item",
            "",
            0,
            0,
            10,
            10,
            0,
            100,
            MatchMethod::CcoeffNormed,
            PlaneFlip::Horizontal,
            0.95,
            0,
            8,
        );
        assert_ne!(none, horiz);
        assert!(horiz.contains("Horizontal"));
    }

    #[test]
    fn invalidate_clears_pending_job_and_cache() {
        let (_tx, rx) = mpsc::channel();
        let mut tool = MatchCheckTool {
            pending: Some(rx),
            ..Default::default()
        };
        tool.invalidate();
        assert!(tool.pending.is_none());
        assert!(tool.cache.is_none());
    }

    #[test]
    fn stop_compute_pauses_and_clears_live_flags() {
        let (_tx, rx) = mpsc::channel();
        let mut tool = MatchCheckTool {
            pending: Some(rx),
            ..Default::default()
        };
        tool.settings.last_inputs = "inputs".into();
        tool.settings.capture_once = true;
        tool.settings.awaiting_panel = true;
        tool.settings.rematch_once = true;
        tool.stop_compute();
        assert!(tool.pending.is_none());
        assert!(tool.cache.is_none());
        assert!(tool.settings.last_inputs.is_empty());
        assert!(tool.settings.paused);
        assert!(!tool.settings.capture_once);
        assert!(!tool.settings.awaiting_panel);
        assert!(!tool.settings.rematch_once);
    }
}
