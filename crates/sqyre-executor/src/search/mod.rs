//! Image search / find-pixel orchestration.

mod common;
mod image;
mod ocr;
mod pixel;
#[cfg(test)]
mod tests;

pub(crate) use image::execute_image_search;
pub(crate) use ocr::execute_ocr;
pub(crate) use pixel::execute_find_pixel;
