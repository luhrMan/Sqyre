//! Latest MediaProjection frame, published by the Kotlin shell and read by capture.
//!
//! The shell offers every `ImageReader` frame; the store only copies one while a
//! capture has asked within [`DEMAND_WINDOW`], so an idle projection does not keep
//! a full-screen copy alive or burn a memcpy per display refresh.

use crate::AndroidError;
use parking_lot::{Condvar, Mutex, MutexGuard};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Frames keep flowing this long after the last capture request.
pub const DEMAND_WINDOW: Duration = Duration::from_secs(2);

/// `ImageReader` plane geometry for one `RGBA_8888` frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameLayout {
    pub width: i32,
    pub height: i32,
    pub row_stride: i32,
    pub pixel_stride: i32,
}

/// One tightly packed RGBA frame in display pixels (origin top-left).
#[derive(Debug, Clone)]
pub struct Frame {
    width: u32,
    height: u32,
    rgba: Arc<[u8]>,
}

impl Frame {
    /// Pack a strided `RGBA_8888` plane into tight rows.
    pub fn pack(layout: FrameLayout, src: &[u8]) -> Result<Self, AndroidError> {
        let width = positive(layout.width).ok_or(AndroidError::BadFrame("width"))?;
        let height = positive(layout.height).ok_or(AndroidError::BadFrame("height"))?;
        if layout.pixel_stride != 4 {
            return Err(AndroidError::BadFrame("pixel stride is not 4 (RGBA_8888)"));
        }
        let row_bytes = width * 4;
        let row_stride = positive(layout.row_stride)
            .filter(|&s| s >= row_bytes)
            .ok_or(AndroidError::BadFrame("row stride"))?;
        if src.len() < row_stride * (height - 1) + row_bytes {
            return Err(AndroidError::BadFrame("buffer shorter than frame"));
        }
        let mut rgba = Vec::with_capacity(row_bytes * height);
        for row in src.chunks(row_stride).take(height) {
            rgba.extend_from_slice(&row[..row_bytes]);
        }
        Ok(Self {
            width: width as u32,
            height: height as u32,
            rgba: rgba.into(),
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Tightly packed RGBA bytes, `width * height * 4` long.
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }
}

fn positive(v: i32) -> Option<usize> {
    usize::try_from(v).ok().filter(|&v| v > 0)
}

/// Whether the shell's MediaProjection session is delivering frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Projection {
    /// No consent yet (or re-armed for the next run after a stop).
    NotStarted,
    Running,
    /// User revoked, consent denied, or the shell tore the session down.
    Stopped,
}

#[derive(Debug)]
struct State {
    frame: Option<Frame>,
    generation: u64,
    demand_at: Option<Instant>,
    projection: Projection,
}

impl State {
    fn demand_active(&self, now: Instant) -> bool {
        self.demand_at
            .is_some_and(|t| now.saturating_duration_since(t) < DEMAND_WINDOW)
    }

    fn projection_error(&self) -> Option<AndroidError> {
        match self.projection {
            Projection::Running => None,
            Projection::NotStarted => Some(AndroidError::NotStarted),
            Projection::Stopped => Some(AndroidError::ProjectionStopped),
        }
    }
}

/// Process-wide latest frame plus projection state.
#[derive(Debug)]
pub struct FrameStore {
    state: Mutex<State>,
    ready: Condvar,
}

impl Default for FrameStore {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameStore {
    pub const fn new() -> Self {
        Self {
            state: Mutex::new(State {
                frame: None,
                generation: 0,
                demand_at: None,
                projection: Projection::NotStarted,
            }),
            ready: Condvar::new(),
        }
    }

    /// True while a capture asked recently enough that the shell should hand frames over.
    pub fn wants_frames(&self) -> bool {
        self.state.lock().demand_active(Instant::now())
    }

    /// Store a frame from the shell and wake waiting captures.
    pub fn publish(&self, layout: FrameLayout, src: &[u8]) -> Result<(), AndroidError> {
        let frame = Frame::pack(layout, src)?;
        let mut s = self.state.lock();
        s.projection = Projection::Running;
        s.frame = Some(frame);
        s.generation += 1;
        drop(s);
        self.ready.notify_all();
        Ok(())
    }

    /// The shell lost or refused the projection; drop the cached frame.
    pub fn mark_stopped(&self) {
        let mut s = self.state.lock();
        s.projection = Projection::Stopped;
        s.frame = None;
        s.generation += 1;
        drop(s);
        self.ready.notify_all();
    }

    /// Allow the next capture to ask for consent again after a stop.
    pub fn rearm(&self) {
        let mut s = self.state.lock();
        if s.projection == Projection::Stopped {
            s.projection = Projection::NotStarted;
        }
    }

    pub fn projection(&self) -> Projection {
        self.state.lock().projection
    }

    /// Latest frame. After an idle gap the cached frame is stale, so this waits up to
    /// `timeout` for the next one.
    pub fn latest(&self, timeout: Duration) -> Result<Frame, AndroidError> {
        let now = Instant::now();
        let mut s = self.state.lock();
        if !s.demand_active(now) {
            s.frame = None;
        }
        s.demand_at = Some(now);
        if let Some(e) = s.projection_error() {
            return Err(e);
        }
        if let Some(f) = &s.frame {
            return Ok(f.clone());
        }
        self.wait_until(&mut s, now + timeout, |s| s.frame.is_some());
        s.frame
            .clone()
            .ok_or_else(|| s.projection_error().unwrap_or(AndroidError::NoFrame))
    }

    /// A frame published after this call, or the latest one if none arrives within `timeout`.
    pub fn fresh(&self, timeout: Duration) -> Result<Frame, AndroidError> {
        let now = Instant::now();
        let mut s = self.state.lock();
        s.demand_at = Some(now);
        if let Some(e) = s.projection_error() {
            return Err(e);
        }
        let generation = s.generation;
        self.wait_until(&mut s, now + timeout, |s| s.generation > generation);
        s.frame
            .clone()
            .ok_or_else(|| s.projection_error().unwrap_or(AndroidError::NoFrame))
    }

    /// Drop the cached frame and stop copying until the next capture asks.
    pub fn release(&self) {
        let mut s = self.state.lock();
        s.frame = None;
        s.demand_at = None;
    }

    /// Bytes held by the cached frame (0 when released).
    pub fn cached_bytes(&self) -> usize {
        self.state.lock().frame.as_ref().map_or(0, |f| f.rgba.len())
    }

    fn wait_until(
        &self,
        s: &mut MutexGuard<'_, State>,
        deadline: Instant,
        done: impl Fn(&State) -> bool,
    ) {
        while !done(s) && s.projection == Projection::Running {
            let now = Instant::now();
            if now >= deadline {
                return;
            }
            self.ready.wait_for(s, deadline - now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: i32 = 3;
    const H: i32 = 2;

    /// 3×2 RGBA frame with 4 bytes of row padding; pixel (x, y) = [x, y, 7, 255].
    fn padded_plane() -> (FrameLayout, Vec<u8>) {
        let row_stride = W * 4 + 4;
        let mut src = Vec::new();
        for y in 0..H {
            for x in 0..W {
                src.extend_from_slice(&[x as u8, y as u8, 7, 255]);
            }
            src.extend_from_slice(&[0xAA; 4]);
        }
        let layout = FrameLayout {
            width: W,
            height: H,
            row_stride,
            pixel_stride: 4,
        };
        (layout, src)
    }

    #[test]
    fn pack_drops_row_padding() {
        let (layout, src) = padded_plane();
        let f = Frame::pack(layout, &src).expect("pack");
        assert_eq!((f.width(), f.height()), (3, 2));
        assert_eq!(f.rgba().len(), 3 * 2 * 4);
        assert_eq!(&f.rgba()[..4], &[0, 0, 7, 255]);
        assert_eq!(&f.rgba()[(3 + 2) * 4..(3 + 2) * 4 + 4], &[2, 1, 7, 255]);
        assert!(!f.rgba().contains(&0xAA));
    }

    #[test]
    fn pack_accepts_last_row_without_padding() {
        let (layout, mut src) = padded_plane();
        src.truncate(src.len() - 4);
        assert!(Frame::pack(layout, &src).is_ok());
    }

    #[test]
    fn pack_rejects_bad_layouts() {
        let (layout, src) = padded_plane();
        for bad in [
            FrameLayout { width: 0, ..layout },
            FrameLayout {
                height: -1,
                ..layout
            },
            FrameLayout {
                pixel_stride: 3,
                ..layout
            },
            FrameLayout {
                row_stride: W * 4 - 1,
                ..layout
            },
        ] {
            assert!(
                matches!(Frame::pack(bad, &src), Err(AndroidError::BadFrame(_))),
                "{bad:?}"
            );
        }
        assert!(matches!(
            Frame::pack(layout, &src[..src.len() - 5]),
            Err(AndroidError::BadFrame(_))
        ));
    }

    #[test]
    fn latest_reports_projection_state_without_waiting() {
        let store = FrameStore::new();
        assert!(matches!(
            store.latest(Duration::from_secs(5)),
            Err(AndroidError::NotStarted)
        ));
        store.mark_stopped();
        assert!(matches!(
            store.latest(Duration::from_secs(5)),
            Err(AndroidError::ProjectionStopped)
        ));
        store.rearm();
        assert_eq!(store.projection(), Projection::NotStarted);
    }

    #[test]
    fn publish_only_matters_while_demanded() {
        let store = FrameStore::new();
        assert!(!store.wants_frames());
        let (layout, src) = padded_plane();
        store.publish(layout, &src).expect("publish");
        assert_eq!(store.projection(), Projection::Running);
        // First request after idle discards the stale frame and waits for a new one.
        assert!(matches!(
            store.latest(Duration::from_millis(1)),
            Err(AndroidError::NoFrame)
        ));
        assert!(store.wants_frames());
        store.publish(layout, &src).expect("publish");
        assert_eq!(store.latest(Duration::ZERO).expect("frame").width(), 3);
        assert_eq!(store.cached_bytes(), 3 * 2 * 4);
        store.release();
        assert_eq!(store.cached_bytes(), 0);
        assert!(!store.wants_frames());
    }

    #[test]
    fn fresh_waits_for_a_newer_frame() {
        let store = Arc::new(FrameStore::new());
        let (layout, src) = padded_plane();
        store.publish(layout, &src).expect("publish");
        let publisher = {
            let store = Arc::clone(&store);
            let src = src.clone();
            std::thread::spawn(move || {
                while !store.wants_frames() {
                    std::thread::yield_now();
                }
                store.publish(layout, &src).expect("publish");
            })
        };
        let frame = store.fresh(Duration::from_secs(5)).expect("fresh frame");
        assert_eq!(frame.height(), 2);
        publisher.join().expect("publisher");
    }

    #[test]
    fn stop_wakes_waiting_capture() {
        let store = Arc::new(FrameStore::new());
        let (layout, src) = padded_plane();
        store.publish(layout, &src).expect("publish");
        let stopper = {
            let store = Arc::clone(&store);
            std::thread::spawn(move || {
                while !store.wants_frames() {
                    std::thread::yield_now();
                }
                store.mark_stopped();
            })
        };
        assert!(matches!(
            store.fresh(Duration::from_secs(5)),
            Err(AndroidError::ProjectionStopped)
        ));
        stopper.join().expect("stopper");
    }
}
