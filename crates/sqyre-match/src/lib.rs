//! Template matching via OpenCV-compatible path:
//! all six `TM_*` methods, optional CV_8U binary mask, peak scan + spatial dedup.
//!
//! Blur uses PureCV (`gaussian_blur`, σ=0 → OpenCV ksize formula, `BORDER_REFLECT_101`).
//! `blur <= 0` means no blur — [`search_blur_kernel`] returns `0` and blurring is skipped.

mod blur;
mod corr_simd;
mod image;
mod peaks;
mod template;

pub use blur::{blur_image, blur_image_owned, search_blur_kernel};
pub use corr_simd::{map_rgb_to_gray_u8, threshold_gray_in_place};
pub use image::{ImageBuf, Point};
pub use peaks::{
    cluster_points, find_peaks, find_peaks_for_method, DEFAULT_CLOSE_MATCHES_DISTANCE,
};
pub use template::{
    match_template, match_template_with_prepared, prepare_search, prepare_template, MatchError,
    MatchMap, PreparedTemplate, SearchPrep,
};

pub use sqyre_domain::MatchMethod;

/// Full path used by image search when the search image is already blurred and the
/// template is not: blur template with `blur`, run matching, extract peaks.
pub fn find_template_matches(
    search_blurred: &ImageBuf,
    template: &ImageBuf,
    mask: Option<&[u8]>,
    threshold: f32,
    blur: i32,
    close_matches_distance: i32,
    method: MatchMethod,
) -> Result<Vec<Point>, MatchError> {
    let kernel = search_blur_kernel(blur);
    if kernel <= 0 {
        return find_template_matches_preblurred(
            search_blurred,
            template,
            mask,
            threshold,
            close_matches_distance,
            method,
        );
    }
    let template_blurred = blur_image_owned(template.clone(), kernel)?;
    find_template_matches_preblurred(
        search_blurred,
        &template_blurred,
        mask,
        threshold,
        close_matches_distance,
        method,
    )
}

/// Match when both search and template are already blurred (cached-template path).
pub fn find_template_matches_preblurred(
    search_blurred: &ImageBuf,
    template_blurred: &ImageBuf,
    mask: Option<&[u8]>,
    threshold: f32,
    close_matches_distance: i32,
    method: MatchMethod,
) -> Result<Vec<Point>, MatchError> {
    let prepared = prepare_template(template_blurred, mask, method)?;
    find_template_matches_preblurred_with_prepared(
        search_blurred,
        template_blurred,
        &prepared,
        threshold,
        close_matches_distance,
        method,
        None,
    )
}

/// Like [`find_template_matches_preblurred`], reusing a [`PreparedTemplate`] (skips
/// re-packing the template) and optionally a [`SearchPrep`] (skips re-deriving the
/// search frame) — both shared across template variants matched against one capture.
pub fn find_template_matches_preblurred_with_prepared(
    search_blurred: &ImageBuf,
    template_blurred: &ImageBuf,
    prepared: &PreparedTemplate,
    threshold: f32,
    close_matches_distance: i32,
    method: MatchMethod,
    search_prep: Option<&SearchPrep>,
) -> Result<Vec<Point>, MatchError> {
    let map =
        match_template_with_prepared(search_blurred, template_blurred, prepared, search_prep)?;
    Ok(find_peaks_for_method(
        &map,
        threshold,
        close_matches_distance,
        method,
    ))
}
