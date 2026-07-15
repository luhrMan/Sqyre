//! Template matching aligned with Sqyre's Go/OpenCV path:
//! `TM_CCOEFF_NORMED` (method 5), optional CV_8U binary mask, peak scan + spatial dedup.
//!
//! Blur uses PureCV (`gaussian_blur`, σ=0 → OpenCV ksize formula, `BORDER_REFLECT_101`).

mod blur;
mod image;
mod peaks;
mod template;

pub use blur::{blur_image, search_blur_kernel};
pub use image::{ImageBuf, Point};
pub use peaks::{find_peaks, DEFAULT_CLOSE_MATCHES_DISTANCE};
pub use template::{match_ccoeff_normed, MatchError, MatchMap};

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
    let template_blurred = blur_image(template, kernel)?;
    let map = match_ccoeff_normed(search_blurred, &template_blurred, mask)?;
    Ok(find_peaks(&map, threshold, close_matches_distance))
}
