/// Interleaved row-major image (`channels` = 1 or 3), matching PureCV `Matrix<u8>` layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageBuf {
    pub width: usize,
    pub height: usize,
    pub channels: usize,
    pub data: Vec<u8>,
}

impl ImageBuf {
    pub fn new(width: usize, height: usize, channels: usize, fill: u8) -> Self {
        assert!(channels == 1 || channels == 3);
        Self {
            width,
            height,
            channels,
            data: vec![fill; width * height * channels],
        }
    }

    pub fn from_raw(width: usize, height: usize, channels: usize, data: Vec<u8>) -> Self {
        assert_eq!(data.len(), width * height * channels);
        Self {
            width,
            height,
            channels,
            data,
        }
    }

    #[inline]
    pub fn pixel_offset(&self, x: usize, y: usize) -> usize {
        (y * self.width + x) * self.channels
    }

    /// Stamp `src` into this image at top-left `(x, y)`. Clips if needed.
    pub fn stamp(&mut self, src: &ImageBuf, x: usize, y: usize) {
        assert_eq!(self.channels, src.channels);
        for sy in 0..src.height {
            let dy = y + sy;
            if dy >= self.height {
                break;
            }
            for sx in 0..src.width {
                let dx = x + sx;
                if dx >= self.width {
                    break;
                }
                let si = src.pixel_offset(sx, sy);
                let di = self.pixel_offset(dx, dy);
                self.data[di..di + self.channels]
                    .copy_from_slice(&src.data[si..si + self.channels]);
            }
        }
    }

    /// Copy a sub-rect. Clamps to this image; `None` if the result would be empty.
    pub fn crop(&self, x: usize, y: usize, width: usize, height: usize) -> Option<Self> {
        if width == 0 || height == 0 || x >= self.width || y >= self.height {
            return None;
        }
        let width = width.min(self.width - x);
        let height = height.min(self.height - y);
        let mut data = vec![0u8; width * height * self.channels];
        for row in 0..height {
            let src = self.pixel_offset(x, y + row);
            let dst = row * width * self.channels;
            let n = width * self.channels;
            data[dst..dst + n].copy_from_slice(&self.data[src..src + n]);
        }
        Some(Self {
            width,
            height,
            channels: self.channels,
            data,
        })
    }
}

/// Top-left match coordinate in the result / search image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamp_clips_at_edges() {
        let mut dst = ImageBuf::new(4, 4, 3, 0);
        let src = ImageBuf::new(3, 3, 3, 200);
        dst.stamp(&src, 2, 2);
        // Only bottom-right 2×2 of stamp lands in dst.
        assert_eq!(dst.data[dst.pixel_offset(2, 2)], 200);
        assert_eq!(dst.data[dst.pixel_offset(3, 3)], 200);
        assert_eq!(dst.data[dst.pixel_offset(0, 0)], 0);
        assert_eq!(dst.data[dst.pixel_offset(1, 1)], 0);
    }

    #[test]
    fn crop_copies_subrect() {
        let mut img = ImageBuf::new(4, 3, 3, 0);
        let i9 = img.pixel_offset(1, 1);
        let i8 = img.pixel_offset(2, 1) + 1;
        img.data[i9] = 9;
        img.data[i8] = 8;
        let crop = img.crop(1, 1, 2, 1).expect("crop");
        assert_eq!(crop.width, 2);
        assert_eq!(crop.height, 1);
        assert_eq!(crop.data[0], 9);
        assert_eq!(crop.data[4], 8);
        assert!(img.crop(4, 0, 1, 1).is_none());
        assert!(img.crop(0, 0, 0, 1).is_none());
    }

    #[test]
    fn from_raw_roundtrip_length() {
        let data = vec![1u8, 2, 3, 4, 5, 6];
        let img = ImageBuf::from_raw(2, 1, 3, data.clone());
        assert_eq!(img.width, 2);
        assert_eq!(img.height, 1);
        assert_eq!(img.data, data);
    }
}
