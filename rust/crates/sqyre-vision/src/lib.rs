//! Image-search utilities: RGB load, PureCV matcher façade, find-pixel.

mod find_pixel;
mod image_util;
mod matcher;

pub use find_pixel::find_pixel;
pub use image_util::{load_rgb_image, rgba_to_rgb_buf, mask_as_u8, resize_mask};
pub use matcher::MatchEngine;
