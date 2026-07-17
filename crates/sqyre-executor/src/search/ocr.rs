//! OCR action: capture → preprocess → recognize → write vars → run children on match.

use super::common::{
    clear_coord_outputs, maybe_repeat_while_found, maybe_wait_until_found, run_detection_outcome,
    set_coord_outputs,
};
use crate::action_log::draw_rect_rgb;
use crate::error::{ExecError, Result};
use crate::run::Executor;
use sqyre_domain::{Action, ActionKind, Macro, ScalarValue};
use sqyre_match::ImageBuf;
use std::time::Instant;

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
        blur,
        min_threshold,
        resize,
        grayscale,
        threshold_otsu,
        threshold_invert,
        detection,
        ..
    } = &action.kind
    else {
        return Err(ExecError::Message("not ocr".into()));
    };
    let sqyre_domain::DetectionBranch {
        wait,
        coords,
        run_branch_on_no_find,
        subactions,
        ..
    } = detection;

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
    }
    maybe_wait_until_found(exec, wait, matched, 500, |exec| {
        let (s, m) = ocr_shot_and_match(exec, action_id, &ocr_params, macro_);
        shot = s;
        matched = m;
        Ok(matched)
    })?;

    if maybe_repeat_while_found(exec, wait, 200, |exec, refresh| {
        if refresh {
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
        run_detection_outcome(exec, matched, *run_branch_on_no_find, subactions, macro_)
    })? {
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
    run_detection_outcome(exec, matched, *run_branch_on_no_find, subactions, macro_).map(|_| ())
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

#[allow(clippy::too_many_arguments)]
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
    let Some(resolver) = exec.deps.resolver else {
        exec.log(action_id, "OCR: missing CoordinateResolver");
        return None;
    };
    if exec.deps.capturer.is_none() {
        exec.log(action_id, "OCR: missing ScreenCapturer");
        return None;
    }
    let Some(ocr) = exec.deps.ocr else {
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

    let capture_started = Instant::now();
    let (img, origin) = match exec
        .deps
        .capturer
        .as_mut()
        .unwrap()
        .capture_search_area_rgb(lx, ty, rx, by)
    {
        Ok(v) => v,
        Err(e) => {
            exec.log(action_id, format!("OCR: capture: {e}"));
            return None;
        }
    };
    exec.log_timing(action_id, "capture", capture_started.elapsed());
    let search_center_x = origin.x + origin.w / 2;
    let search_center_y = origin.y + origin.h / 2;
    let rgb = img.into_image_buf();
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
    let preprocess_started = Instant::now();
    let (processed, scale, steps) =
        match sqyre_vision::preprocess_for_ocr_with_steps(&rgb, opts, collect) {
            Ok(v) => v,
            Err(e) => {
                exec.log(action_id, format!("OCR: preprocess: {e}"));
                return None;
            }
        };
    exec.log_timing(action_id, "preprocess", preprocess_started.elapsed());
    for step in &steps {
        exec.log_image(action_id, &step.label, &step.image);
    }
    let recognize_started = Instant::now();
    let recognized = match ocr.recognize(&processed) {
        Ok(v) => v,
        Err(e) => {
            exec.log(action_id, format!("OCR: {e}"));
            return None;
        }
    };
    exec.log_timing(action_id, "recognize", recognize_started.elapsed());

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
