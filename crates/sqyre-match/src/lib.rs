//! Template matching via OpenCV-compatible path:
//! `TM_CCOEFF_NORMED` (method 5), optional CV_8U binary mask, peak scan + spatial dedup.
//!
//! Blur uses PureCV (`gaussian_blur`, σ=0 → OpenCV ksize formula, `BORDER_REFLECT_101`).

mod blur;
mod image;
mod peaks;
mod template;

pub use blur::{blur_image, blur_image_owned, search_blur_kernel};
pub use image::{ImageBuf, Point};
pub use peaks::{cluster_points, find_peaks, DEFAULT_CLOSE_MATCHES_DISTANCE};
pub use template::{
    match_ccoeff_normed, match_ccoeff_normed_with_integrals, prepare_search_integrals, MatchError,
    MatchMap, SearchIntegrals,
};

/// Full path used by image search when the search image is already blurred and the
/// template is not: blur template with `blur`, run CCOEFF_NORMED, extract peaks.
pub fn find_template_matches(
    search_blurred: &ImageBuf,
    template: &ImageBuf,
    mask: Option<&[u8]>,
    threshold: f32,
    blur: i32,
    close_matches_distance: i32,
) -> Result<Vec<Point>, MatchError> {
    let kernel = search_blur_kernel(blur);
    let template_blurred = blur_image_owned(template.clone(), kernel)?;
    find_template_matches_preblurred(
        search_blurred,
        &template_blurred,
        mask,
        threshold,
        close_matches_distance,
    )
}

/// Match when both search and template are already blurred (cached-template path).
pub fn find_template_matches_preblurred(
    search_blurred: &ImageBuf,
    template_blurred: &ImageBuf,
    mask: Option<&[u8]>,
    threshold: f32,
    close_matches_distance: i32,
) -> Result<Vec<Point>, MatchError> {
    find_template_matches_preblurred_with_integrals(
        search_blurred,
        template_blurred,
        mask,
        threshold,
        close_matches_distance,
        None,
    )
}

/// Like [`find_template_matches_preblurred`], reusing shared search integrals.
pub fn find_template_matches_preblurred_with_integrals(
    search_blurred: &ImageBuf,
    template_blurred: &ImageBuf,
    mask: Option<&[u8]>,
    threshold: f32,
    close_matches_distance: i32,
    integrals: Option<&SearchIntegrals>,
) -> Result<Vec<Point>, MatchError> {
    let map =
        match_ccoeff_normed_with_integrals(search_blurred, template_blurred, mask, integrals)?;
    Ok(find_peaks(&map, threshold, close_matches_distance))
}
