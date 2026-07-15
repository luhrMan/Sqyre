//! Image search / find-pixel orchestration (Go `executor_search.go`).

use crate::backends::{DesktopRect, ItemMeta, TemplateMatcher};
use crate::error::{ExecError, FlowSignal, Result};
use crate::highlight::{highlight_clear, highlight_fill};
use crate::run::{execute_action, Executor};
use sqyre_domain::{Action, ActionKind, Macro, ScalarValue, REPEAT_WHILE_FOUND};
use sqyre_match::{
    blur_image, find_template_matches, search_blur_kernel, ImageBuf, Point,
    DEFAULT_CLOSE_MATCHES_DISTANCE,
};
use sqyre_vision::{find_pixel, load_rgb_image, mask_as_u8, resize_mask, rgba_to_rgb_buf};
use std::time::{Duration, Instant};

pub(crate) fn execute_image_search(
    exec: &mut Executor<'_>,
    action: &Action,
    macro_: &mut Macro,
) -> Result<()> {
    let ActionKind::ImageSearch {
        targets,
        search_area,
        tolerance,
        blur,
        wait,
        coords,
        run_branch_on_no_find,
        subactions,
        ..
    } = &action.kind
    else {
        return Err(ExecError::Message("not image search".into()));
    };

    highlight_fill(exec.highlighter, &macro_.name, action.id, 0.0);
    let action_id = action.id;
    let macro_name = macro_.name.clone();
    let result = (|| {
        let mut results =
            capture_and_match(exec, action_id, targets, search_area, *tolerance, *blur, macro_);

        if wait.wait_until_found_active() && results.is_empty() {
            exec.log(
                action_id,
                format!(
                    "Image Search: waiting up to {}s until found",
                    wait.wait_til_found_seconds
                ),
            );
            let _ = retry_while_not_found(exec, wait, 100, |exec| {
                results = capture_and_match(
                    exec,
                    action_id,
                    targets,
                    search_area,
                    *tolerance,
                    *blur,
                    macro_,
                );
                Ok(!results.is_empty())
            })?;
        }

        if wait.effective_repeat_mode() == REPEAT_WHILE_FOUND {
            let deadline =
                Instant::now() + Duration::from_secs(wait.wait_til_found_seconds.max(0) as u64);
            while Instant::now() < deadline {
                exec.check_stopped()?;
                if results.is_empty() {
                    break;
                }
                run_matches(
                    exec,
                    action_id,
                    targets,
                    &results,
                    coords,
                    *run_branch_on_no_find,
                    subactions,
                    macro_,
                )?;
                results = capture_and_match(
                    exec,
                    action_id,
                    targets,
                    search_area,
                    *tolerance,
                    *blur,
                    macro_,
                );
                let interval = wait.effective_interval_ms(100).max(1);
                exec.automation.milli_sleep(interval);
            }
            return Ok(());
        }

        run_matches(
            exec,
            action_id,
            targets,
            &results,
            coords,
            *run_branch_on_no_find,
            subactions,
            macro_,
        )
    })();
    highlight_clear(exec.highlighter, &macro_name, action_id);
    result
}

struct NamedPoint {
    point: Point,
    origin: DesktopRect,
    meta: Option<ItemMeta>,
    tmpl_w: i32,
    tmpl_h: i32,
    name: String,
}

fn capture_and_match(
    exec: &mut Executor<'_>,
    action_id: sqyre_domain::ActionId,
    targets: &[String],
    search_area: &sqyre_domain::CoordinateRef,
    tolerance: f64,
    blur: i32,
    macro_: &Macro,
) -> Vec<NamedPoint> {
    let Some(resolver) = exec.resolver else {
        exec.log(action_id, "Image Search: missing CoordinateResolver");
        return Vec::new();
    };
    if exec.capturer.is_none() {
        exec.log(action_id, "Image Search: missing ScreenCapturer");
        return Vec::new();
    }
    let Some(icons) = exec.icons else {
        exec.log(action_id, "Image Search: missing IconStore");
        return Vec::new();
    };

    let (lx, ty, rx, by) = match resolver.resolve_search_area(search_area, macro_) {
        Ok(v) => v,
        Err(e) => {
            exec.log(
                action_id,
                format!(
                    "Image Search: resolve search area {}: {e}",
                    search_area.display_label()
                ),
            );
            return Vec::new();
        }
    };
    let w = (rx - lx).max(0);
    let h = (by - ty).max(0);
    exec.log(
        action_id,
        format!(
            "Image Searching | {targets:?} in X1:{lx} Y1:{ty} X2:{rx} Y2:{by}, width:{w} height:{h}"
        ),
    );

    let (img, origin) = match exec
        .capturer
        .as_mut()
        .unwrap()
        .capture_search_area(lx, ty, rx, by)
    {
        Ok(v) => v,
        Err(e) => {
            exec.log(action_id, format!("Image Search: capture: {e}"));
            return Vec::new();
        }
    };
    let search = rgba_to_rgb_buf(&img);
    let kernel = search_blur_kernel(blur);
    let search_blurred = match blur_image(&search, kernel) {
        Ok(b) => b,
        Err(e) => {
            exec.log(action_id, format!("Image Search: blur: {e}"));
            return Vec::new();
        }
    };

    let threshold = tolerance as f32;
    let mut out = Vec::new();
    let match_started = Instant::now();
    for target in targets {
        let paths = icons.variant_paths(target);
        if paths.is_empty() {
            exec.log(
                action_id,
                format!("Image Search: no icon variants for {target}"),
            );
            continue;
        }
        let meta = icons.item_meta(target);
        let mask_path = icons.mask_path(target);
        for path in paths {
            let template = match load_rgb_image(&path) {
                Ok(t) => t,
                Err(e) => {
                    exec.log(action_id, format!("Image Search: load {path:?}: {e}"));
                    continue;
                }
            };
            exec.log(
                action_id,
                format!(
                    "Image Search: matching {target} ({}x{}) against {}x{}",
                    template.width, template.height, search_blurred.width, search_blurred.height
                ),
            );
            let mask_bytes = mask_path
                .as_ref()
                .and_then(|p| load_rgb_image(p).ok())
                .map(|m| {
                    let resized = resize_mask(&m, template.width, template.height);
                    mask_as_u8(&resized)
                });
            let t0 = Instant::now();
            let matches = match find_template_matches(
                &search_blurred,
                &template,
                mask_bytes.as_deref(),
                threshold,
                blur,
                DEFAULT_CLOSE_MATCHES_DISTANCE,
            ) {
                Ok(m) => m,
                Err(e) => {
                    exec.log(action_id, format!("Image Search match: {e}"));
                    continue;
                }
            };
            exec.log(
                action_id,
                format!(
                    "Image Search: {target} → {} match(es) in {:.0}ms",
                    matches.len(),
                    t0.elapsed().as_secs_f64() * 1000.0
                ),
            );
            let half_w = (template.width / 2) as i32;
            let half_h = (template.height / 2) as i32;
            for mut p in matches {
                p.x += half_w;
                p.y += half_h;
                out.push(NamedPoint {
                    name: target.clone(),
                    point: p,
                    origin,
                    meta: meta.clone(),
                    tmpl_w: template.width as i32,
                    tmpl_h: template.height as i32,
                });
            }
        }
    }
    exec.log(
        action_id,
        format!(
            "Image Search: capture+match done in {:.0}ms ({} raw hit(s))",
            match_started.elapsed().as_secs_f64() * 1000.0,
            out.len()
        ),
    );
    sort_points(&mut out);
    let _ = exec.matcher; // optional override reserved for tests injecting TemplateMatcher
    out
}

fn sort_points(pts: &mut [NamedPoint]) {
    pts.sort_by(|a, b| {
        let ay = a.point.y + a.origin.y;
        let by = b.point.y + b.origin.y;
        let ax = a.point.x + a.origin.x;
        let bx = b.point.x + b.origin.x;
        if (ay - by).abs() <= 5 {
            ax.cmp(&bx).then(a.name.cmp(&b.name))
        } else {
            ay.cmp(&by).then(ax.cmp(&bx))
        }
    });
}

fn run_matches(
    exec: &mut Executor<'_>,
    action_id: sqyre_domain::ActionId,
    targets: &[String],
    results: &[NamedPoint],
    coords: &sqyre_domain::CoordinateOutputs,
    run_branch_on_no_find: bool,
    subactions: &[Action],
    macro_: &mut Macro,
) -> Result<()> {
    let mut found_names: Vec<&str> = results.iter().map(|np| np.name.as_str()).collect();
    found_names.sort_unstable();
    found_names.dedup();
    let not_found: Vec<&str> = targets
        .iter()
        .map(|t| t.as_str())
        .filter(|t| !found_names.iter().any(|f| f == t))
        .collect();
    exec.log(
        action_id,
        format!(
            "Total # found: {} (found: {:?}; not found: {:?})",
            results.len(),
            found_names,
            not_found
        ),
    );

    let mut first: Option<(i32, i32)> = None;
    let total = results.len();
    for (count, np) in results.iter().enumerate() {
        if total > 0 {
            highlight_fill(
                exec.highlighter,
                &macro_.name,
                action_id,
                count as f64 / total as f64,
            );
        }
        let x = np.point.x + np.origin.x;
        let y = np.point.y + np.origin.y;
        if first.is_none() {
            first = Some((x, y));
        }
        set_coord_outputs(macro_, coords, x, y);
        if let Some(meta) = &np.meta {
            macro_
                .variables
                .set("StackMax", ScalarValue::Int(meta.stack_max as i64));
            macro_
                .variables
                .set("Cols", ScalarValue::Int(meta.cols as i64));
            macro_
                .variables
                .set("Rows", ScalarValue::Int(meta.rows as i64));
            macro_
                .variables
                .set("ItemName", ScalarValue::String(meta.name.clone()));
            macro_
                .variables
                .set("ImagePixelWidth", ScalarValue::Int(np.tmpl_w as i64));
            macro_
                .variables
                .set("ImagePixelHeight", ScalarValue::Int(np.tmpl_h as i64));
        }
        match run_children_flow(exec, subactions, macro_) {
            Err(ExecError::Flow(FlowSignal::Break)) => break,
            Err(ExecError::Flow(FlowSignal::Continue)) => continue,
            Err(e) => return Err(e),
            Ok(()) => {}
        }
    }
    if results.is_empty() && run_branch_on_no_find {
        run_children_flow(exec, subactions, macro_)?;
    }
    if let Some((x, y)) = first {
        set_coord_outputs(macro_, coords, x, y);
    }
    Ok(())
}

fn run_children_flow(exec: &mut Executor<'_>, children: &[Action], macro_: &mut Macro) -> Result<()> {
    for child in children {
        execute_action(exec, child, macro_)?;
    }
    Ok(())
}

fn set_coord_outputs(macro_: &mut Macro, coords: &sqyre_domain::CoordinateOutputs, x: i32, y: i32) {
    if !coords.output_x_variable.is_empty() {
        macro_
            .variables
            .set(&coords.output_x_variable, ScalarValue::Int(x as i64));
    }
    if !coords.output_y_variable.is_empty() {
        macro_
            .variables
            .set(&coords.output_y_variable, ScalarValue::Int(y as i64));
    }
}

fn retry_while_not_found(
    exec: &mut Executor<'_>,
    wait: &sqyre_domain::WaitTilFoundConfig,
    default_interval_ms: i32,
    mut retry: impl FnMut(&mut Executor<'_>) -> Result<bool>,
) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(wait.wait_til_found_seconds.max(0) as u64);
    let mut interval = wait.effective_interval_ms(default_interval_ms).max(1);
    let max_interval = (interval * 5).min(2000).max(interval);
    while Instant::now() < deadline {
        exec.check_stopped()?;
        exec.automation.milli_sleep(interval);
        if retry(exec)? {
            return Ok(());
        }
        if interval < max_interval {
            interval = (interval * 2).min(max_interval);
        }
    }
    Ok(())
}

pub(crate) fn execute_find_pixel(
    exec: &mut Executor<'_>,
    action: &Action,
    macro_: &mut Macro,
) -> Result<()> {
    let ActionKind::FindPixel {
        search_area,
        target_color,
        color_tolerance,
        wait,
        coords,
        ..
    } = &action.kind
    else {
        return Err(ExecError::Message("not find pixel".into()));
    };

    let mut found = try_find_pixel(exec, search_area, target_color, *color_tolerance, macro_);
    if wait.wait_until_found_active() && found.is_none() {
        let _ = retry_while_not_found(exec, wait, 100, |exec| {
            found = try_find_pixel(exec, search_area, target_color, *color_tolerance, macro_);
            Ok(found.is_some())
        })?;
    }
    if let Some((x, y)) = found {
        exec.log(
            action.id,
            format!("FindPixel: found matching pixel at screen ({x}, {y})"),
        );
        set_coord_outputs(macro_, coords, x, y);
    } else {
        exec.log(action.id, "FindPixel: pixel not found");
    }
    Ok(())
}

fn try_find_pixel(
    exec: &mut Executor<'_>,
    search_area: &sqyre_domain::CoordinateRef,
    target_color: &str,
    color_tolerance: i32,
    macro_: &Macro,
) -> Option<(i32, i32)> {
    let resolver = exec.resolver?;
    let capturer = exec.capturer.as_mut()?;
    let (lx, ty, rx, by) = resolver.resolve_search_area(search_area, macro_).ok()?;
    let (img, origin) = capturer.capture_search_area(lx, ty, rx, by).ok()?;
    let buf = rgba_to_rgb_buf(&img);
    let local = find_pixel(&buf, target_color, color_tolerance)?;
    Some((local.x + origin.x, local.y + origin.y))
}

/// `TemplateMatcher` over `sqyre-match` (injectable in tests).
#[derive(Debug, Default)]
pub struct MatchFacade {
    pub close_matches_distance: i32,
}

impl MatchFacade {
    pub fn new() -> Self {
        Self {
            close_matches_distance: DEFAULT_CLOSE_MATCHES_DISTANCE,
        }
    }
}

impl TemplateMatcher for MatchFacade {
    fn find_matches(
        &self,
        search: &ImageBuf,
        template: &ImageBuf,
        mask: Option<&ImageBuf>,
        threshold: f32,
        blur: i32,
    ) -> std::result::Result<Vec<Point>, sqyre_match::MatchError> {
        let mask_bytes = mask.map(|m| {
            if m.channels == 1 {
                m.data.clone()
            } else {
                mask_as_u8(m)
            }
        });
        let kernel = search_blur_kernel(blur);
        let search_blurred = blur_image(search, kernel)?;
        find_template_matches(
            &search_blurred,
            template,
            mask_bytes.as_deref(),
            threshold,
            blur,
            if self.close_matches_distance > 0 {
                self.close_matches_distance
            } else {
                DEFAULT_CLOSE_MATCHES_DISTANCE
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::{
        DesktopRect, IconStore, ItemMeta, RecordingBackend, RecordingCapturer, TemplateMatcher,
    };
    use crate::run::{execute_macro_with, ExecDeps};
    use crate::SharedActionLog;
    use image::{Rgba, RgbaImage};
    use sqyre_domain::{
        root_loop, Action, ActionId, ActionKind, CoordinateOutputs, CoordinateRef, Macro,
        ScalarValue, WaitTilFoundConfig,
    };
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
        paths: HashMap<String, PathBuf>,
        meta: HashMap<String, ItemMeta>,
    }

    impl IconStore for MapIcons {
        fn variant_paths(&self, target: &str) -> Vec<PathBuf> {
            self.paths.get(target).cloned().into_iter().collect()
        }
        fn mask_path(&self, _target: &str) -> Option<PathBuf> {
            None
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
        sort_points(&mut pts);
        assert_eq!(
            pts.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            vec!["a", "b", "c"]
        );
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
                wait: WaitTilFoundConfig::default(),
                coords: CoordinateOutputs {
                    output_x_variable: "foundX".into(),
                    output_y_variable: "foundY".into(),
                },
                run_branch_on_no_find: false,
                order: Default::default(),
                subactions: vec![],
            },
        }]);

        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: Some(&mut capturer),
                matcher: None,
                resolver: Some(&resolver),
                icons: None,
                macros: None,
                continue_waiter: None,
                window_focuser: None,
                stop_flag: None,
                logger: Some(&logger),
                highlighter: None,
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
        assert!(capturer.log.iter().any(|e| e == "rect:50,60,4,4"), "{:?}", capturer.log);
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
                wait: WaitTilFoundConfig::default(),
                coords: CoordinateOutputs {
                    output_x_variable: "foundX".into(),
                    output_y_variable: "foundY".into(),
                },
                run_branch_on_no_find: false,
                order: Default::default(),
                subactions: vec![],
            },
        }]);

        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: Some(&mut capturer),
                matcher: None,
                resolver: Some(&resolver),
                icons: None,
                macros: None,
                continue_waiter: None,
                window_focuser: None,
                stop_flag: None,
                logger: Some(&logger),
                highlighter: None,
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
                wait: WaitTilFoundConfig::default(),
                coords: CoordinateOutputs::defaults(),
                run_branch_on_no_find: false,
                order: Default::default(),
                subactions: vec![],
            },
        }]);
        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: Some(&mut capturer),
                matcher: None,
                resolver: Some(&resolver),
                icons: None,
                macros: None,
                continue_waiter: None,
                window_focuser: None,
                stop_flag: None,
                logger: Some(&logger),
                highlighter: None,
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
            paths: HashMap::from([("Prog~Item".into(), tmpl_path)]),
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
                row_split: 0,
                col_split: 0,
                tolerance: 0.99,
                blur: 0,
                wait: WaitTilFoundConfig::default(),
                coords: CoordinateOutputs::defaults(),
                run_branch_on_no_find: true,
                order: Default::default(),
                subactions: vec![Action {
                    id: ActionId::new(),
                    kind: ActionKind::Wait {
                        time: ScalarValue::Int(13),
                    },
                }],
            },
        }]);

        execute_macro_with(
            &mut macro_,
            ExecDeps {
                automation: &mut backend,
                capturer: Some(&mut capturer),
                matcher: None,
                resolver: Some(&resolver),
                icons: Some(&icons),
                macros: None,
                continue_waiter: None,
                window_focuser: None,
                stop_flag: None,
                logger: Some(&logger),
                highlighter: None,
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
            lines
                .iter()
                .any(|l| l.contains("Total # found: 0")),
            "{lines:?}"
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
        let results = vec![
            named("a", 1, 1, 0, 0),
            named("b", 2, 2, 0, 0),
        ];
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
    fn match_facade_finds_exact_template() {
        // blur=0 still maps to kernel 5 (Go searchBlurKernel); pattern must survive that.
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
        let facade = MatchFacade::new();
        let hits = facade
            .find_matches(&search, &tmpl, None, 0.7, 0)
            .unwrap();
        assert!(
            hits.iter()
                .any(|p| (p.x - 15).abs() <= 2 && (p.y - 18).abs() <= 2),
            "expected peak near (15,18), got {hits:?}"
        );
    }
}
