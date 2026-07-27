use crate::image::ImageBuf;
use crate::template::MatchError;

/// Normalizes a blur amount to a positive odd Gaussian kernel size, or `0` when
/// `blur <= 0` — the caller should skip blurring entirely in that case.
pub fn search_blur_kernel(blur: i32) -> i32 {
    if blur <= 0 {
        return 0;
    }
    let mut blur = blur;
    if blur % 2 == 0 {
        blur += 1;
    }
    blur
}

/// Gaussian blur via PureCV with OpenCV-compatible σ=0 and `BORDER_REFLECT_101`.
///
/// Clones `img.data` into a PureCV matrix. Prefer [`blur_image_owned`] when the
/// caller can give up the buffer.
pub fn blur_image(img: &ImageBuf, blur: i32) -> Result<ImageBuf, MatchError> {
    blur_image_owned(img.clone(), blur)
}

/// Like [`blur_image`] but takes ownership so the pixel buffer moves into PureCV
/// without an extra clone. No-op (identity) when `blur <= 0`.
pub fn blur_image_owned(img: ImageBuf, blur: i32) -> Result<ImageBuf, MatchError> {
    let k = search_blur_kernel(blur);
    if k <= 0 || k as usize > img.width || k as usize > img.height {
        return Ok(img);
    }

    use purecv::core::{BorderTypes, Matrix, Size};
    use purecv::imgproc::gaussian_blur;

    let mat = Matrix::from_vec(img.height, img.width, img.channels, img.data);
    let out = gaussian_blur(&mat, Size::new(k, k), 0.0, 0.0, BorderTypes::Reflect101)
        .map_err(|e| MatchError::Blur(e.to_string()))?;
    Ok(ImageBuf {
        width: out.cols,
        height: out.rows,
        channels: out.channels,
        data: out.data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_blur_kernel_no_blur_and_odd() {
        assert_eq!(search_blur_kernel(0), 0);
        assert_eq!(search_blur_kernel(-3), 0);
        assert_eq!(search_blur_kernel(4), 5);
        assert_eq!(search_blur_kernel(5), 5);
        assert_eq!(search_blur_kernel(6), 7);
    }

    #[test]
    fn blur_image_owned_is_identity_when_blur_not_positive() {
        let img = ImageBuf::new(4, 4, 3, 42);
        let out = blur_image_owned(img.clone(), 0).unwrap();
        assert_eq!(out.data, img.data);
    }
}
