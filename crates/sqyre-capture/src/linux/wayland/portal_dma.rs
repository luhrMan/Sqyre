//! dma-buf / memfd mapping and PipeWire frame blit into the RGBA cache.

use crate::error::CaptureError;
use pipewire as pw;
use pw::spa::buffer::DataType;
use pw::spa::param::video::VideoFormat;

/// linux/dma-buf.h `DMA_BUF_IOCTL_SYNC` (`_IOW('b', 0, struct dma_buf_sync)`).
const DMA_BUF_IOCTL_SYNC: libc::c_ulong = 0x4008_6200;
const DMA_BUF_SYNC_READ: u64 = 1 << 0;
const DMA_BUF_SYNC_END: u64 = 1 << 2;

#[repr(C)]
struct DmaBufSync {
    flags: u64,
}

/// CPU-map coherency for GNOME ScreenCast DMA-BUF / memfd. Without START/END
/// the mapping often stays on the first GPU write for the whole wait-until-found.
struct SpaBufSync {
    fd: i32,
    active: bool,
}

impl SpaBufSync {
    fn begin(ty: DataType, fd: i32) -> Self {
        let active = fd >= 0 && matches!(ty, DataType::DmaBuf | DataType::MemFd);
        if active {
            dma_buf_sync(fd, DMA_BUF_SYNC_READ);
        }
        Self { fd, active }
    }
}

impl Drop for SpaBufSync {
    fn drop(&mut self) {
        if self.active {
            dma_buf_sync(self.fd, DMA_BUF_SYNC_READ | DMA_BUF_SYNC_END);
        }
    }
}

fn dma_buf_sync(fd: i32, flags: u64) {
    let mut sync = DmaBufSync { flags };
    // SAFETY: `fd` is the live spa_data fd; `sync` is `struct dma_buf_sync`.
    let _ = unsafe { libc::ioctl(fd, DMA_BUF_IOCTL_SYNC, &mut sync) };
}

struct MappedFd {
    ptr: *mut libc::c_void,
    len: usize,
}

impl MappedFd {
    fn as_slice(&self) -> &[u8] {
        // SAFETY: `ptr`/`len` come from a successful `mmap` of `len` bytes.
        unsafe { std::slice::from_raw_parts(self.ptr.cast(), self.len) }
    }
}

impl Drop for MappedFd {
    fn drop(&mut self) {
        if !self.ptr.is_null() && self.len > 0 {
            // SAFETY: mapping created by `mmap_spa_fd` and not yet unmapped.
            unsafe {
                libc::munmap(self.ptr, self.len);
            }
        }
    }
}

fn mmap_spa_fd(data: &pw::spa::buffer::Data) -> Option<MappedFd> {
    let raw = data.as_raw();
    let len = raw.maxsize as usize;
    let fd = data.fd();
    if len == 0 || fd < 0 {
        return None;
    }
    // SAFETY: `fd` is PipeWire's buffer fd; `mapoffset`/`maxsize` are spa_data.
    let ptr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            len,
            libc::PROT_READ,
            libc::MAP_SHARED,
            fd,
            raw.mapoffset as libc::off_t,
        )
    };
    if ptr == libc::MAP_FAILED {
        return None;
    }
    Some(MappedFd { ptr, len })
}

fn chunk_byte_range(
    mapped_len: usize,
    offset: usize,
    size: usize,
) -> Option<std::ops::Range<usize>> {
    let end = offset.checked_add(size)?;
    (end <= mapped_len && size > 0).then_some(offset..end)
}

pub(super) fn with_spa_chunk_bytes<T>(
    data: &mut pw::spa::buffer::Data,
    offset: usize,
    size: usize,
    f: impl FnOnce(&[u8]) -> T,
) -> Option<T> {
    let ty = data.type_();
    let fd = data.fd();
    let _sync = SpaBufSync::begin(ty, fd);
    if let Some(mapped) = data.data() {
        let range = chunk_byte_range(mapped.len(), offset, size)?;
        return Some(f(&mapped[range]));
    }
    let map = mmap_spa_fd(data)?;
    let range = chunk_byte_range(map.as_slice().len(), offset, size)?;
    Some(f(&map.as_slice()[range]))
}

#[allow(clippy::too_many_arguments)] // src frame + dest rect in one blit
pub(super) fn copy_pw_frame_into_rect(
    src: &[u8],
    size: usize,
    src_stride: usize,
    src_w: u32,
    src_h: u32,
    format: VideoFormat,
    dst: &mut [u8],
    dst_stride: usize,
    dst_x: usize,
    dst_y: usize,
    dst_w: u32,
    dst_h: u32,
) -> Result<(), CaptureError> {
    if src_w == dst_w && src_h == dst_h {
        return copy_pw_frame_to_rgba_at(
            src, size, src_stride, src_w, src_h, format, dst, dst_stride, dst_x, dst_y,
        );
    }
    let mut tmp = vec![0u8; src_w as usize * src_h as usize * 4];
    copy_pw_frame_to_rgba_at(
        src,
        size,
        src_stride,
        src_w,
        src_h,
        format,
        &mut tmp,
        src_w as usize * 4,
        0,
        0,
    )?;
    let sw = src_w as usize;
    let sh = src_h as usize;
    let dw = dst_w as usize;
    let dh = dst_h as usize;
    for y in 0..dh {
        let sy = y * sh / dh;
        for x in 0..dw {
            let sx = x * sw / dw;
            let src_off = (sy * sw + sx) * 4;
            let dst_off = (dst_y + y) * dst_stride + (dst_x + x) * 4;
            if src_off + 4 > tmp.len() || dst_off + 4 > dst.len() {
                return Err(CaptureError::Message(
                    "portal capture: RGBA buffer too small".into(),
                ));
            }
            dst[dst_off..dst_off + 4].copy_from_slice(&tmp[src_off..src_off + 4]);
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)] // row copy needs src/dst geometry in one pass
fn copy_pw_frame_to_rgba_at(
    src: &[u8],
    size: usize,
    src_stride: usize,
    width: u32,
    height: u32,
    format: VideoFormat,
    dst: &mut [u8],
    dst_stride: usize,
    dst_x: usize,
    dst_y: usize,
) -> Result<(), CaptureError> {
    let w = width as usize;
    let h = height as usize;
    for y in 0..h {
        let src_off = y * src_stride;
        if src_off >= size.min(src.len()) {
            break;
        }
        let row_len = src_stride.min(src.len().saturating_sub(src_off));
        let row = &src[src_off..src_off + row_len];
        let dst_row_off = (dst_y + y) * dst_stride + dst_x * 4;
        if dst_row_off + w * 4 > dst.len() {
            return Err(CaptureError::Message(
                "portal capture: RGBA buffer too small".into(),
            ));
        }
        let dst_row = &mut dst[dst_row_off..dst_row_off + w * 4];
        swizzle_row_to_rgba(row, w, format, dst_row)?;
    }
    Ok(())
}

fn swizzle_row_to_rgba(
    row: &[u8],
    width: usize,
    format: VideoFormat,
    dst: &mut [u8],
) -> Result<(), CaptureError> {
    let bpp = match format {
        VideoFormat::RGB | VideoFormat::BGR => 3,
        VideoFormat::RGBA | VideoFormat::BGRA | VideoFormat::RGBx | VideoFormat::BGRx => 4,
        other => {
            return Err(CaptureError::Message(format!(
                "portal capture: unsupported PipeWire format {other:?}"
            )));
        }
    };
    for x in 0..width {
        let src_off = x * bpp;
        let dst_off = x * 4;
        if src_off + bpp > row.len() || dst_off + 4 > dst.len() {
            break;
        }
        match format {
            VideoFormat::RGBA => {
                dst[dst_off..dst_off + 4].copy_from_slice(&row[src_off..src_off + 4])
            }
            VideoFormat::BGRA => {
                dst[dst_off] = row[src_off + 2];
                dst[dst_off + 1] = row[src_off + 1];
                dst[dst_off + 2] = row[src_off];
                dst[dst_off + 3] = row[src_off + 3];
            }
            VideoFormat::RGBx => {
                dst[dst_off..dst_off + 3].copy_from_slice(&row[src_off..src_off + 3]);
                dst[dst_off + 3] = 255;
            }
            VideoFormat::BGRx => {
                dst[dst_off] = row[src_off + 2];
                dst[dst_off + 1] = row[src_off + 1];
                dst[dst_off + 2] = row[src_off];
                dst[dst_off + 3] = 255;
            }
            VideoFormat::RGB => {
                dst[dst_off..dst_off + 3].copy_from_slice(&row[src_off..src_off + 3]);
                dst[dst_off + 3] = 255;
            }
            VideoFormat::BGR => {
                dst[dst_off] = row[src_off + 2];
                dst[dst_off + 1] = row[src_off + 1];
                dst[dst_off + 2] = row[src_off];
                dst[dst_off + 3] = 255;
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bgrx_row_to_rgba() {
        let row = [0u8, 1, 2, 0, 10, 11, 12, 0];
        let mut out = [0u8; 8];
        swizzle_row_to_rgba(&row, 2, VideoFormat::BGRx, &mut out).unwrap();
        assert_eq!(out, [2, 1, 0, 255, 12, 11, 10, 255]);
    }

    #[test]
    fn chunk_byte_range_skips_prefix_offset() {
        assert_eq!(chunk_byte_range(12, 4, 4), Some(4..8));
        assert_eq!(chunk_byte_range(8, 0, 8), Some(0..8));
        assert_eq!(chunk_byte_range(8, 6, 4), None);
        assert_eq!(chunk_byte_range(8, 0, 0), None);
    }

    #[test]
    fn composite_frame_at_offset() {
        let row = [0u8, 1, 2, 255, 10, 11, 12, 255];
        let mut dst = vec![0u8; 16];
        copy_pw_frame_to_rgba_at(
            &row,
            row.len(),
            8,
            2,
            1,
            VideoFormat::RGBA,
            &mut dst,
            8,
            1,
            0,
        )
        .unwrap();
        assert_eq!(&dst[4..8], &[0, 1, 2, 255]);
        assert_eq!(&dst[0..4], &[0, 0, 0, 0]);
    }

    #[test]
    fn scale_frame_into_smaller_rect() {
        let mut src = vec![0u8; 8];
        src[0..4].copy_from_slice(&[10, 20, 30, 255]);
        src[4..8].copy_from_slice(&[40, 50, 60, 255]);
        let mut dst = vec![0u8; 4];
        copy_pw_frame_into_rect(
            &src,
            src.len(),
            8,
            2,
            1,
            VideoFormat::RGBA,
            &mut dst,
            4,
            0,
            0,
            1,
            1,
        )
        .unwrap();
        assert_eq!(&dst, &[10, 20, 30, 255]);
    }
}
