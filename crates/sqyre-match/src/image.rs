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
}

/// Top-left match coordinate in the result / search image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}
