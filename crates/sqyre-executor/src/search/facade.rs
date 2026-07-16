//! `TemplateMatcher` over `sqyre-match` (injectable in tests).

use crate::backends::TemplateMatcher;
use sqyre_match::{
    blur_image_owned, find_template_matches, search_blur_kernel, ImageBuf, Point,
    DEFAULT_CLOSE_MATCHES_DISTANCE,
};
use sqyre_vision::mask_as_u8;

#[derive(Debug, Default)]
pub struct MatchFacade {
    pub close_matches_distance: i32,
}

impl MatchFacade {
    pub fn new() -> Self {
        Self {
            close_matches_distance: DEFAULT_CLOSE_MATCHES_DISTANCE,
        }
    }
}

impl TemplateMatcher for MatchFacade {
    fn find_matches(
        &self,
        search: &ImageBuf,
        template: &ImageBuf,
        mask: Option<&ImageBuf>,
        threshold: f32,
        blur: i32,
    ) -> std::result::Result<Vec<Point>, sqyre_match::MatchError> {
        let mask_bytes = mask.map(|m| {
            if m.channels == 1 {
                m.data.clone()
            } else {
                mask_as_u8(m)
            }
        });
        let kernel = search_blur_kernel(blur);
        let search_blurred = blur_image_owned(search.clone(), kernel)?;
        find_template_matches(
            &search_blurred,
            template,
            mask_bytes.as_deref(),
            threshold,
            blur,
            self.close_matches_distance(),
        )
    }

    fn close_matches_distance(&self) -> i32 {
        if self.close_matches_distance > 0 {
            self.close_matches_distance
        } else {
            DEFAULT_CLOSE_MATCHES_DISTANCE
        }
    }
}
