//! Bounded file reads + image decode for files whose bytes are not fully
//! trusted (imported icons/masks, desktop-file `Icon=` paths, pasted images).
//!
//! `image::open` / `load_from_memory` happily decompress a 100 KB PNG into
//! gigabytes of pixels ("decompression bomb"), so every untrusted decode must
//! go through [`decode_image_bounded`] — and the read itself through
//! [`read_file_capped`] so a huge file is not slurped first.

use image::{DynamicImage, ImageError};
use std::io::{self, Read};
use std::path::Path;

/// Largest decoded dimension accepted for any loaded image. Legitimate
/// search templates are screen-crop sized; 8K-wide rows already fit.
pub const MAX_IMAGE_DECODE_DIM: u32 = 16_384;

/// Total decoded allocation cap — 256 MiB ≈ one 8K RGBA frame.
pub const MAX_IMAGE_DECODE_ALLOC: u64 = 256 * 1024 * 1024;

/// Largest image file read from disk before decode.
pub const MAX_IMAGE_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// `image` crate limits applied to every untrusted decode.
pub fn image_decode_limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DECODE_DIM);
    limits.max_image_height = Some(MAX_IMAGE_DECODE_DIM);
    limits.max_alloc = Some(MAX_IMAGE_DECODE_ALLOC);
    limits
}

/// Decode `bytes` under [`image_decode_limits`]; format is sniffed like
/// [`image::load_from_memory`].
pub fn decode_image_bounded(bytes: &[u8]) -> Result<DynamicImage, ImageError> {
    let mut reader = image::ImageReader::new(io::Cursor::new(bytes)).with_guessed_format()?;
    reader.limits(image_decode_limits());
    reader.decode()
}

/// Read at most `max_bytes` from `path`; `Err` when the file is larger.
/// A metadata pre-check plus a bounded read keeps this correct even when the
/// file grows between the stat and the read.
pub fn read_file_capped(path: &Path, max_bytes: u64) -> io::Result<Vec<u8>> {
    if std::fs::metadata(path)
        .map(|m| m.len() > max_bytes)
        .unwrap_or(false)
    {
        return Err(too_large(path, max_bytes));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max_bytes {
        return Err(too_large(path, max_bytes));
    }
    Ok(bytes)
}

fn too_large(path: &Path, max_bytes: u64) -> io::Error {
    io::Error::new(
        io::ErrorKind::FileTooLarge,
        format!("{} exceeds {max_bytes} byte cap", path.display()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_file_capped_rejects_oversized() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.bin");
        std::fs::write(&path, vec![0u8; 1024]).unwrap();
        assert_eq!(read_file_capped(&path, 2048).unwrap().len(), 1024);
        let err = read_file_capped(&path, 100).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::FileTooLarge);
    }

    #[test]
    fn decode_image_bounded_accepts_normal_png() {
        let mut buf = Vec::new();
        image::RgbImage::from_pixel(4, 4, image::Rgb([1, 2, 3]))
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();
        let img = decode_image_bounded(&buf).unwrap();
        assert_eq!((img.width(), img.height()), (4, 4));
    }
}
