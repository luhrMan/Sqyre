//! Optional match-scale percent: shrink search + template before correlating,
//! then remap peaks back to full-resolution coordinates.

use crate::image_util::resize_nearest;
use sqyre_domain::{clamp_match_scale_percent, DEFAULT_MATCH_SCALE_PERCENT};
use sqyre_match::{ImageBuf, Point};

/// Result of applying [`scale_for_match`] to a search/template pair.
pub struct ScaledMatchInputs {
    pub search: ImageBuf,
    pub template: ImageBuf,
    pub mask: Option<Vec<u8>>,
    /// Factor applied to widths/heights (`1.0` = unchanged). Peak coords divide by this.
    pub scale: f32,
}

/// Scale search, template, and optional mask by `match_scale_percent` (10..=100).
///
/// `100` (or values that clamp to it) returns clones with `scale = 1.0` and skips
/// resize work when dimensions would be unchanged.
pub fn scale_for_match(
    search: &ImageBuf,
    template: &ImageBuf,
    mask: Option<&[u8]>,
    match_scale_percent: i32,
) -> ScaledMatchInputs {
    let percent = clamp_match_scale_percent(match_scale_percent);
    if percent >= DEFAULT_MATCH_SCALE_PERCENT {
        return ScaledMatchInputs {
            search: search.clone(),
            template: template.clone(),
            mask: mask.map(|m| m.to_vec()),
            scale: 1.0,
        };
    }
    let scale = percent as f32 / 100.0;
    let sw = ((search.width as f32) * scale).round().max(1.0) as usize;
    let sh = ((search.height as f32) * scale).round().max(1.0) as usize;
    let tw = ((template.width as f32) * scale).round().max(1.0) as usize;
    let th = ((template.height as f32) * scale).round().max(1.0) as usize;
    // Actual scale from search width (used for peak remap); height uses the same %.
    let scale_x = sw as f32 / search.width.max(1) as f32;
    let scaled_mask = mask.map(|m| {
        let mask_img = ImageBuf::from_raw(template.width, template.height, 1, m.to_vec());
        resize_nearest(&mask_img, tw, th).data
    });
    ScaledMatchInputs {
        search: resize_nearest(search, sw, sh),
        template: resize_nearest(template, tw, th),
        mask: scaled_mask,
        scale: scale_x,
    }
}

/// Map a peak from match-space back to the pre-scale search image.
pub fn remap_point_to_source(p: Point, scale: f32) -> Point {
    if (scale - 1.0).abs() < f32::EPSILON || scale <= 0.0 {
        return p;
    }
    Point {
        x: ((p.x as f32) / scale).round() as i32,
        y: ((p.y as f32) / scale).round() as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_percent_keeps_dims() {
        let search = ImageBuf::new(40, 20, 3, 1);
        let tmpl = ImageBuf::new(8, 4, 3, 2);
        let out = scale_for_match(&search, &tmpl, None, 100);
        assert_eq!(out.search.width, 40);
        assert_eq!(out.template.width, 8);
        assert!((out.scale - 1.0).abs() < f32::EPSILON);
        let p = remap_point_to_source(Point { x: 10, y: 5 }, out.scale);
        assert_eq!(p, Point { x: 10, y: 5 });
    }

    #[test]
    fn half_percent_halves_and_remaps() {
        let search = ImageBuf::new(40, 20, 3, 1);
        let tmpl = ImageBuf::new(8, 4, 3, 2);
        let mask = vec![255u8; 8 * 4];
        let out = scale_for_match(&search, &tmpl, Some(&mask), 50);
        assert_eq!(out.search.width, 20);
        assert_eq!(out.search.height, 10);
        assert_eq!(out.template.width, 4);
        assert_eq!(out.template.height, 2);
        assert_eq!(out.mask.as_ref().map(|m| m.len()), Some(8));
        assert!((out.scale - 0.5).abs() < f32::EPSILON);
        let p = remap_point_to_source(Point { x: 4, y: 2 }, out.scale);
        assert_eq!(p, Point { x: 8, y: 4 });
    }
}
