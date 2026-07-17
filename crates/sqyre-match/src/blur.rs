use crate::image::ImageBuf;
use crate::template::MatchError;

/// Normalizes a blur amount to a positive odd Gaussian kernel size.
/// Blur kernel size for image search (odd; minimum 5).
pub fn search_blur_kernel(blur: i32) -> i32 {
    let mut blur = blur;
    if blur <= 0 {
        blur = 5;
    }
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
/// without an extra clone.
pub fn blur_image_owned(img: ImageBuf, blur: i32) -> Result<ImageBuf, MatchError> {
    let k = search_blur_kernel(blur);
    if k as usize > img.width || k as usize > img.height {
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
    fn search_blur_kernel_defaults_and_odd() {
        assert_eq!(search_blur_kernel(0), 5);
        assert_eq!(search_blur_kernel(-3), 5);
        assert_eq!(search_blur_kernel(4), 5);
        assert_eq!(search_blur_kernel(5), 5);
        assert_eq!(search_blur_kernel(6), 7);
    }
}
