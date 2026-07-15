use sqyre_match::{
    blur_image, find_template_matches, search_blur_kernel, ImageBuf, MatchError, Point,
    DEFAULT_CLOSE_MATCHES_DISTANCE,
};

/// Default `TemplateMatcher`-compatible engine over `sqyre-match`.
#[derive(Debug, Default, Clone)]
pub struct MatchEngine {
    pub close_matches_distance: i32,
}

impl MatchEngine {
    pub fn new() -> Self {
        Self {
            close_matches_distance: DEFAULT_CLOSE_MATCHES_DISTANCE,
        }
    }

    pub fn find_matches(
        &self,
        search: &ImageBuf,
        template: &ImageBuf,
        mask: Option<&[u8]>,
        threshold: f32,
        blur: i32,
    ) -> Result<Vec<Point>, MatchError> {
        // Mirror Go: blur search once outside; here blur both via find_template_matches
        // when search is pre-blurred callers pass blur=0 with already-blurred search —
        // for the facade we blur search here then run with templatePreBlurred=false path.
        let kernel = search_blur_kernel(blur);
        let search_blurred = blur_image(search, kernel)?;
        find_template_matches(
            &search_blurred,
            template,
            mask,
            threshold,
            blur,
            self.close_matches_distance,
        )
    }

    /// Match against an already-blurred search image (Go `templatePreBlurred` path).
    pub fn find_matches_preblurred_search(
        &self,
        search_blurred: &ImageBuf,
        template: &ImageBuf,
        mask: Option<&[u8]>,
        threshold: f32,
        blur: i32,
    ) -> Result<Vec<Point>, MatchError> {
        find_template_matches(
            search_blurred,
            template,
            mask,
            threshold,
            blur,
            self.close_matches_distance,
        )
    }
}
