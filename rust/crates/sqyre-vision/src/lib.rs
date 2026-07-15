//! Image-search utilities: RGB load, PureCV matcher façade, find-pixel, OCR helpers.

mod find_pixel;
mod image_util;
mod matcher;
mod ocr_boxes;
mod ocr_engine;
mod ocr_preprocess;

pub use find_pixel::find_pixel;
pub use image_util::{load_rgb_image, mask_as_u8, resize_mask, rgba_to_rgb_buf};
pub use matcher::MatchEngine;
pub use ocr_boxes::{
    find_target_in_boxes, parse_tsv_word_boxes, text_from_ocr_boxes, OcrWordBox,
};
pub use ocr_engine::{recognize_image, LeptessOcr, OcrRecognition};
pub use ocr_preprocess::{
    preprocess_for_ocr, preprocess_for_ocr_with_steps, OcrPreprocessOptions, OcrPreprocessStep,
};
