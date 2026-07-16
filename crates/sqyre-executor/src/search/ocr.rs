//! OCR action: capture → preprocess → recognize → write vars → run children on match.

use super::common::{clear_coord_outputs, retry_while_not_found, run_detection_children, set_coord_outputs};
use crate::action_log::draw_rect_rgb;
use crate::error::{ExecError, Result};
use crate::run::Executor;
use sqyre_domain::{Action, ActionKind, Macro, ScalarValue};
use sqyre_match::ImageBuf;
use sqyre_vision::rgba_to_rgb_buf;

/// OCR branch action: capture → preprocess → recognize → write vars → run children on match
/// (or on miss when `run_branch_on_no_find`). Capture/OCR errors are logged and treated as miss.
pub(crate) fn execute_ocr(
    exec: &mut Executor<'_>,
    action: &Action,
    macro_: &mut Macro,
) -> Result<()> {
    let ActionKind::Ocr {
        target,
        search_area,
        output_variable,
        coords,
        wait,
        run_branch_on_no_find,
        blur,
        min_threshold,
        resize,
        grayscale,
        threshold_otsu,
        threshold_invert,
        subactions,
        ..
    } = &action.kind
    else {
        return Err(ExecError::Message("not ocr".into()));
    };

    let action_id = action.id;
    let ocr_params = OcrRunParams {
        search_area,
        target,
        blur: *blur,
        min_threshold: *min_threshold,
        resize: *resize,
        grayscale: *grayscale,
        threshold_otsu: *threshold_otsu,
        threshold_invert: *threshold_invert,
    };

    let (mut shot, mut matched) = ocr_shot_and_match(exec, action_id, &ocr_params, macro_);

    if wait.wait_until_found_active() && !matched {
        exec.log(
            action_id,
            format!(
                "OCR: waiting up to {}s until text contains {target:?}",
                wait.wait_til_found_seconds
            ),
        );
        let _ = retry_while_not_found(exec, wait, 500, |exec| {
            let (s, m) = ocr_shot_and_match(exec, action_id, &ocr_params, macro_);
            shot = s;
            matched = m;
            Ok(matched)
        })?;
    }

    if wait.is_repeat_while_found() {
        let max_iter = wait.effective_max_iterations();
        let interval = wait.effective_interval_ms(200).max(1);
        for i in 0..max_iter {
            exec.check_stopped()?;
            if i > 0 {
                exec.interruptible_sleep(interval)?;
                let (s, m) = ocr_shot_and_match(exec, action_id, &ocr_params, macro_);
                shot = s;
                matched = m;
            }
            apply_ocr_outputs(
                exec,
                action_id,
                macro_,
                output_variable,
                coords,
                &shot,
                matched,
            );
            if matched {
                run_detection_children(exec, subactions, macro_)?;
            } else {
                if *run_branch_on_no_find {
                    run_detection_children(exec, subactions, macro_)?;
                }
                return Ok(());
            }
        }
        return Ok(());
    }

    apply_ocr_outputs(
        exec,
        action_id,
        macro_,
        output_variable,
        coords,
        &shot,
        matched,
    );
    if matched || *run_branch_on_no_find {
        return run_detection_children(exec, subactions, macro_);
    }
    Ok(())
}

struct OcrRunParams<'a> {
    search_area: &'a sqyre_domain::CoordinateRef,
    target: &'a str,
    blur: i32,
    min_threshold: i32,
    resize: f64,
    grayscale: bool,
    threshold_otsu: bool,
    threshold_invert: bool,
}

fn ocr_shot_and_match(
    exec: &mut Executor<'_>,
    action_id: sqyre_domain::ActionId,
    params: &OcrRunParams<'_>,
    macro_: &Macro,
) -> (Option<OcrShot>, bool) {
    let shot = run_ocr_once(
        exec,
        action_id,
        params.search_area,
        params.target,
        params.blur,
        params.min_threshold,
        params.resize,
        params.grayscale,
        params.threshold_otsu,
        params.threshold_invert,
        macro_,
    );
    let matched = shot
        .as_ref()
        .is_some_and(|s| ocr_target_matched(params.target, &s.text));
    (shot, matched)
}

fn apply_ocr_outputs(
    exec: &mut Executor<'_>,
    action_id: sqyre_domain::ActionId,
    macro_: &mut Macro,
    output_variable: &str,
    coords: &sqyre_domain::CoordinateOutputs,
    shot: &Option<OcrShot>,
    matched: bool,
) {
    if !matched {
        macro_.variables.delete(output_variable);
        clear_coord_outputs(macro_, coords);
        return;
    }
    let Some(result) = shot else {
        return;
    };
    if !output_variable.is_empty() {
        macro_
            .variables
            .set(output_variable, ScalarValue::String(result.text.clone()));
    }
    set_coord_outputs(macro_, coords, result.x, result.y);
    exec.log(
        action_id,
        format!(
            "OCR: {} chars → {:?} at ({}, {})",
            result.text.len(),
            output_variable,
            result.x,
            result.y
        ),
    );
}

struct OcrShot {
    text: String,
    x: i32,
    y: i32,
}

fn run_ocr_once(
    exec: &mut Executor<'_>,
    action_id: sqyre_domain::ActionId,
    search_area: &sqyre_domain::CoordinateRef,
    target: &str,
    blur: i32,
    min_threshold: i32,
    resize: f64,
    grayscale: bool,
    threshold_otsu: bool,
    threshold_invert: bool,
    macro_: &Macro,
) -> Option<OcrShot> {
    let Some(resolver) = exec.resolver else {
        exec.log(action_id, "OCR: missing CoordinateResolver");
        return None;
    };
    if exec.capturer.is_none() {
        exec.log(action_id, "OCR: missing ScreenCapturer");
        return None;
    }
    let Some(ocr) = exec.ocr else {
        exec.log(action_id, "OCR: missing OcrEngine");
        return None;
    };

    let (lx, ty, rx, by) = match resolver.resolve_search_area(search_area, macro_) {
        Ok(v) => v,
        Err(e) => {
            exec.log(
                action_id,
                format!(
                    "OCR: resolve search area {}: {e}",
                    search_area.display_label()
                ),
            );
            return None;
        }
    };
    exec.log(
        action_id,
        format!(
            "{target} OCR search | {} in X1:{lx} Y1:{ty} X2:{rx} Y2:{by}",
            search_area.display_label()
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
            exec.log(action_id, format!("OCR: capture: {e}"));
            return None;
        }
    };
    let search_center_x = origin.x + origin.w / 2;
    let search_center_y = origin.y + origin.h / 2;
    let rgb = rgba_to_rgb_buf(&img);
    exec.log_image(action_id, "Capture (raw)", &rgb);
    let opts = sqyre_vision::OcrPreprocessOptions::from_action_fields(
        grayscale,
        blur,
        min_threshold,
        resize,
        threshold_otsu,
        threshold_invert,
    );
    let collect = exec.log_images_enabled();
    let (processed, scale, steps) =
        match sqyre_vision::preprocess_for_ocr_with_steps(&rgb, opts, collect) {
            Ok(v) => v,
            Err(e) => {
                exec.log(action_id, format!("OCR: preprocess: {e}"));
                return None;
            }
        };
    for step in &steps {
        exec.log_image(action_id, &step.label, &step.image);
    }
    let recognized = match ocr.recognize(&processed) {
        Ok(v) => v,
        Err(e) => {
            exec.log(action_id, format!("OCR: {e}"));
            return None;
        }
    };

    exec.log(
        action_id,
        format!(
            "OCR full text ({} chars): {}",
            recognized.text.len(),
            recognized.text
        ),
    );
    exec.log(
        action_id,
        format!("OCR words found: {}", recognized.words.len()),
    );
    for (i, w) in recognized.words.iter().enumerate() {
        let ww = (w.right - w.left).max(0);
        let wh = (w.bottom - w.top).max(0);
        exec.log(
            action_id,
            format!(
                "  word[{i}] {:?} box=({},{})-({},{}) size={ww}×{wh}",
                w.word, w.left, w.top, w.right, w.bottom
            ),
        );
    }

    // Overlay word boxes on an RGB copy of the OCR input for the logs UI.
    if collect && !recognized.words.is_empty() {
        let mut overlay = if processed.channels == 1 {
            gray_to_rgb(&processed)
        } else {
            processed.clone()
        };
        for w in &recognized.words {
            draw_rect_rgb(
                &mut overlay,
                w.left,
                w.top,
                w.right.saturating_sub(1),
                w.bottom.saturating_sub(1),
                [40, 220, 80],
            );
        }
        exec.log_image(action_id, "OCR word boxes", &overlay);
    }

    let mut out_x = search_center_x;
    let mut out_y = search_center_y;
    let resize_scale = if scale > 0.0 { scale } else { 1.0 };
    if let Some((bx, by)) = sqyre_vision::find_target_in_boxes(&recognized.words, target) {
        out_x = origin.x + (bx as f64 / resize_scale) as i32;
        out_y = origin.y + (by as f64 / resize_scale) as i32;
        exec.log(
            action_id,
            format!(
                "OCR target {target:?} matched at image ({bx}, {by}) → screen ({out_x}, {out_y}) (scale={resize_scale:.3})"
            ),
        );
    } else {
        exec.log(
            action_id,
            format!("OCR target {target:?} not found among word boxes"),
        );
    }

    Some(OcrShot {
        text: recognized.text,
        x: out_x,
        y: out_y,
    })
}

fn gray_to_rgb(img: &ImageBuf) -> ImageBuf {
    sqyre_vision::gray_to_rgb(img)
}

/// Substring match: empty target always matches
/// (including empty OCR text), so wait-until-found never arms on blank target.
pub(super) fn ocr_target_matched(target: &str, found_text: &str) -> bool {
    if target.is_empty() {
        true
    } else {
        found_text.contains(target)
    }
}
