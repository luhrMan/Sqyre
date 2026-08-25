//! OCR action: capture → preprocess → recognize → write vars → run children per hit.

use super::common::{
    apply_detection_hits, capture_search_buf, run_detection_shell, sort_hits, DetectionCtx,
    DetectionHit,
};
use crate::error::{ExecError, Result};
use crate::log_draw::draw_rect_rgb;
use crate::run::Executor;
use sqyre_domain::{action_type_label, Action, ActionKind, Macro, ScalarValue};
use std::time::Instant;

/// OCR branch action: capture → preprocess → recognize → write vars → run children per hit
/// (or else children on miss). Capture/OCR errors are logged and treated as miss.
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
    let action_id = action.id;
    let label = action_type_label(action.type_key());
    let wait = &detection.wait;
    let ocr_params = OcrRunParams {
        target,
        blur: *blur,
        min_threshold: *min_threshold,
        resize: *resize,
        grayscale: *grayscale,
        threshold_otsu: *threshold_otsu,
        threshold_invert: *threshold_invert,
    };
    let targets = if target.is_empty() {
        Vec::new()
    } else {
        vec![target.clone()]
    };
    let ctx = DetectionCtx::new(action_id, label, search_area, &targets, detection);

    let attempt0 = ocr_attempt(exec, &ctx, &ocr_params, macro_, false);
    if wait.wait_until_found_active() && attempt0.hits.is_empty() {
        exec.log(
            action_id,
            format!(
                "{label}: waiting up to {}s until text contains {target:?}",
                wait.wait_til_found_seconds
            ),
        );
    } else if wait.wait_while_found_active() && !attempt0.hits.is_empty() {
        exec.log(
            action_id,
            format!(
                "{label}: waiting up to {}s while text contains {target:?}",
                wait.wait_til_found_seconds
            ),
        );
    }

    let mut initial = Some(attempt0);
    run_detection_shell(
        exec,
        macro_,
        &ctx,
        |exec, macro_, fresh| {
            if let Some(first) = initial.take() {
                return Ok(first);
            }
            Ok(ocr_attempt(exec, &ctx, &ocr_params, macro_, fresh))
        },
        |attempt| !attempt.hits.is_empty(),
        |exec, macro_, attempt, pass| {
            apply_ocr_text_var(macro_, output_variable, attempt);
            apply_detection_hits(exec, &ctx, &attempt.hits, macro_, pass)
        },
    )
}

struct OcrRunParams<'a> {
    target: &'a str,
    blur: i32,
    min_threshold: i32,
    resize: f64,
    grayscale: bool,
    threshold_otsu: bool,
    threshold_invert: bool,
}

#[derive(Clone)]
struct OcrAttempt {
    text: Option<String>,
    hits: Vec<DetectionHit>,
}

fn apply_ocr_text_var(macro_: &mut Macro, output_variable: &str, attempt: &OcrAttempt) {
    if attempt.hits.is_empty() {
        macro_.variables.delete(output_variable);
        return;
    }
    if let Some(text) = &attempt.text {
        if !output_variable.is_empty() {
            macro_
                .variables
                .set(output_variable, ScalarValue::String(text.clone()));
        }
    }
}

fn ocr_attempt(
    exec: &mut Executor<'_>,
    ctx: &DetectionCtx<'_>,
    params: &OcrRunParams<'_>,
    macro_: &Macro,
    fresh: bool,
) -> OcrAttempt {
    match run_ocr_once(exec, ctx, params, macro_, fresh) {
        Some(a) => a,
        None => OcrAttempt {
            text: None,
            hits: Vec::new(),
        },
    }
}

fn run_ocr_once(
    exec: &mut Executor<'_>,
    ctx: &DetectionCtx<'_>,
    params: &OcrRunParams<'_>,
    macro_: &Macro,
    fresh: bool,
) -> Option<OcrAttempt> {
    let action_id = ctx.action_id;
    let label = ctx.label;
    let Some(ocr) = exec.deps.ocr else {
        exec.log(action_id, format!("{label}: missing OcrEngine"));
        return None;
    };

    let (rgb, origin) = capture_search_buf(exec, ctx, macro_, fresh, |exec, lx, ty, rx, by| {
        exec.log(
            action_id,
            format!(
                "{label}: searching {:?} in {} X1:{lx} Y1:{ty} X2:{rx} Y2:{by}",
                params.target,
                ctx.search_area.display_label()
            ),
        );
    })?;
    let search_center_x = origin.x + origin.w / 2;
    let search_center_y = origin.y + origin.h / 2;
    exec.log_image(action_id, "Capture (raw)", &rgb);
    let opts = sqyre_vision::OcrPreprocessOptions::from_action_fields(
        params.grayscale,
        params.blur,
        params.min_threshold,
        params.resize,
        params.threshold_otsu,
        params.threshold_invert,
    );
    let collect = exec.log_images_enabled();
    let preprocess_started = Instant::now();
    let (processed, scale, steps) =
        match sqyre_vision::preprocess_for_ocr_with_steps(&rgb, opts, collect) {
            Ok(v) => v,
            Err(e) => {
                exec.log(action_id, format!("{label}: preprocess: {e}"));
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
            exec.log(action_id, format!("{label}: {e}"));
            return None;
        }
    };
    exec.log_timing(action_id, "recognize", recognize_started.elapsed());

    exec.log(
        action_id,
        format!(
            "{label} full text ({} chars): {}",
            recognized.text.len(),
            recognized.text
        ),
    );
    exec.log(
        action_id,
        format!("{label} words found: {}", recognized.words.len()),
    );
    if collect {
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
    }

    if collect && !recognized.words.is_empty() {
        let mut overlay = if processed.channels == 1 {
            sqyre_vision::gray_to_rgb(&processed)
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
        exec.log_image(action_id, format!("{label} word boxes"), &overlay);
    }

    let resize_scale = if scale > 0.0 { scale } else { 1.0 };
    let mut hits = if params.target.is_empty() {
        // Empty target always matches once at search-area center.
        vec![DetectionHit::plain(search_center_x, search_center_y, "")]
    } else {
        let occurrences = sqyre_vision::find_target_occurrences(&recognized.words, params.target);
        if occurrences.is_empty() {
            exec.log(
                action_id,
                format!(
                    "{label} target {:?} not found among word boxes",
                    params.target
                ),
            );
            // Text-contains can still succeed when boxes miss; treat as miss for coords/hits
            // unless full text contains the target — then one synthetic center hit.
            if ocr_target_matched(params.target, &recognized.text) {
                exec.log(
                    action_id,
                    format!(
                        "{label} target {:?} in full text but no word-box occurrence; using search center",
                        params.target
                    ),
                );
                vec![DetectionHit::plain(
                    search_center_x,
                    search_center_y,
                    params.target,
                )]
            } else {
                Vec::new()
            }
        } else {
            exec.log(
                action_id,
                format!(
                    "{label} target {:?} matched {} occurrence(s)",
                    params.target,
                    occurrences.len()
                ),
            );
            if collect {
                for (bx, by) in &occurrences {
                    let sx = origin.x + (*bx as f64 / resize_scale) as i32;
                    let sy = origin.y + (*by as f64 / resize_scale) as i32;
                    exec.log(
                        action_id,
                        format!(
                            "{label} target {:?} matched at image ({bx}, {by}) → screen ({sx}, {sy}) (scale={resize_scale:.3})",
                            params.target
                        ),
                    );
                }
            }
            occurrences
                .into_iter()
                .map(|(bx, by)| {
                    DetectionHit::plain(
                        origin.x + (bx as f64 / resize_scale) as i32,
                        origin.y + (by as f64 / resize_scale) as i32,
                        params.target,
                    )
                })
                .collect()
        }
    };
    sort_hits(&mut hits, &ctx.branch.order);

    let attempt = OcrAttempt {
        text: Some(recognized.text),
        hits,
    };
    Some(attempt)
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
