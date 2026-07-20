//! Image-search utilities: RGB load, PureCV matcher façade, find-pixel, OCR helpers.

mod find_pixel;
mod image_util;
mod ocr_boxes;
#[cfg(not(target_arch = "wasm32"))]
mod ocr_engine;
mod ocr_preprocess;
mod search_cache;

pub use find_pixel::find_pixel;
pub use image_util::{
    gray_to_rgb, load_rgb_image, mask_as_u8, resize_mask, resize_nearest, rgb_to_grayscale,
    rgba_to_rgb_buf,
};
pub use ocr_boxes::{find_target_in_boxes, parse_tsv_word_boxes, text_from_ocr_boxes, OcrWordBox};
#[cfg(not(target_arch = "wasm32"))]
pub use ocr_engine::{recognize_image, LeptessOcr, OcrRecognition};
pub use ocr_preprocess::{
    preprocess_for_ocr, preprocess_for_ocr_with_steps, OcrPreprocessOptions, OcrPreprocessStep,
};
pub use search_cache::{
    clear_search_cache, get_cached_blurred_template, get_cached_image_mask,
    invalidate_search_masks_under, invalidate_search_templates_under,
    reset_search_cache_for_testing, with_search_cache_test_lock,
};
