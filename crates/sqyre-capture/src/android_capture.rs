//! Android: MediaProjection frames from the Kotlin shell, plus app launch as focus.
//!
//! The display is the whole virtual desktop: one monitor at (0, 0) in physical pixels,
//! the same space accessibility gestures use.

use crate::{
    crop_packed_rgba, crop_packed_rgba_to_rgb, CaptureError, NotReady, ProcessIcon, WindowInfo,
    PROCESS_ICON_TARGET_PX,
};
use image::RgbaImage;
use sqyre_android::{bridge, frames, AndroidError, Frame, LaunchableApp};
use sqyre_ports::{AutomationError, DesktopRect, RgbCapture};
use std::time::Duration;

/// How long one capture waits for the projection to deliver a frame.
const FRAME_WAIT: Duration = Duration::from_millis(500);

/// Stateless handle; frames live in [`sqyre_android::frames`].
///
/// `open` always succeeds: the shared slot caches its first result, and consent is
/// requested lazily by the first capture instead.
pub struct OsCapturer;

crate::define_shared_run_capturer!();

impl OsCapturer {
    pub fn open() -> Result<Self, CaptureError> {
        Ok(Self)
    }

    pub fn capture_rect_ref(&self, rect: DesktopRect) -> Result<RgbaImage, CaptureError> {
        to_rgba(&latest()?, rect)
    }

    pub fn capture_rect_fresh_ref(&self, rect: DesktopRect) -> Result<RgbaImage, CaptureError> {
        to_rgba(&fresh()?, rect)
    }

    pub fn capture_rect_rgb_ref(&self, rect: DesktopRect) -> Result<RgbCapture, CaptureError> {
        to_rgb(&latest()?, rect)
    }

    pub fn capture_rect_rgb_fresh_ref(
        &self,
        rect: DesktopRect,
    ) -> Result<RgbCapture, CaptureError> {
        to_rgb(&fresh()?, rect)
    }

    /// Frames are already deduplicated by the store, so every capture is "quiet".
    pub fn capture_rect_rgb_quiet_ref(
        &self,
        rect: DesktopRect,
    ) -> Result<(RgbCapture, bool), CaptureError> {
        self.capture_rect_rgb_ref(rect).map(|rgb| (rgb, true))
    }

    /// Drop the cached frame and stop copying until the next capture.
    pub fn release_cpu_frame_cache(&self) {
        frames().release();
    }

    pub fn cpu_frame_cache_bytes(&self) -> usize {
        frames().cached_bytes()
    }

    pub fn virtual_bounds_ref(&self) -> Result<DesktopRect, CaptureError> {
        let (w, h) = bridge::display_size().map_err(projection_error)?;
        Ok(DesktopRect { x: 0, y: 0, w, h })
    }

    pub fn monitor_rects_ref(&self) -> Result<Vec<DesktopRect>, CaptureError> {
        Ok(vec![self.virtual_bounds_ref()?])
    }

    pub fn monitor_sizes_ref(&self) -> Result<Vec<(i32, i32)>, CaptureError> {
        Ok(self
            .monitor_rects_ref()?
            .into_iter()
            .map(|r| (r.w, r.h))
            .collect())
    }
}

fn latest() -> Result<Frame, CaptureError> {
    frames()
        .latest(FRAME_WAIT)
        .map_err(start_projection_on_error)
}

fn fresh() -> Result<Frame, CaptureError> {
    frames()
        .fresh(FRAME_WAIT)
        .map_err(start_projection_on_error)
}

/// Not-started projections ask the shell for consent; the caller retries on `NotReady`.
fn start_projection_on_error(e: AndroidError) -> CaptureError {
    match e {
        AndroidError::NotStarted => match bridge::request_projection() {
            Ok(()) => NotReady::AwaitingProjection.into(),
            Err(e) => projection_error(e),
        },
        AndroidError::NoFrame => NotReady::AwaitingFirstFrame.into(),
        other => projection_error(other),
    }
}

fn projection_error(e: AndroidError) -> CaptureError {
    CaptureError::Projection(e.to_string())
}

/// Validated crop window inside a frame.
struct Window {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

fn window(frame: &Frame, rect: DesktopRect) -> Result<Window, CaptureError> {
    if rect.is_empty() {
        return Err(CaptureError::EmptyRect);
    }
    let (x, y, w, h) = (
        i64::from(rect.x),
        i64::from(rect.y),
        i64::from(rect.w),
        i64::from(rect.h),
    );
    if x < 0 || y < 0 || x + w > i64::from(frame.width()) || y + h > i64::from(frame.height()) {
        return Err(CaptureError::OutsideVirtualDesktop);
    }
    Ok(Window {
        x: x as usize,
        y: y as usize,
        w: w as usize,
        h: h as usize,
    })
}

fn to_rgba(frame: &Frame, rect: DesktopRect) -> Result<RgbaImage, CaptureError> {
    let win = window(frame, rect)?;
    let data = crop_packed_rgba(
        frame.rgba(),
        frame.width() as usize,
        win.x,
        win.y,
        win.w,
        win.h,
    );
    RgbaImage::from_raw(win.w as u32, win.h as u32, data).ok_or(CaptureError::GetImage {
        x: rect.x,
        y: rect.y,
        w: rect.w,
        h: rect.h,
    })
}

fn to_rgb(frame: &Frame, rect: DesktopRect) -> Result<RgbCapture, CaptureError> {
    let win = window(frame, rect)?;
    let data = crop_packed_rgba_to_rgb(
        frame.rgba(),
        frame.width() as usize,
        win.x,
        win.y,
        win.w,
        win.h,
    );
    Ok(RgbCapture {
        width: win.w as u32,
        height: win.h as u32,
        data,
    })
}

/// Focus Window picker rows: one per launchable app (package as path, label as title).
pub fn list_open_windows() -> Result<Vec<WindowInfo>, CaptureError> {
    let apps =
        bridge::launchable_apps().map_err(|e| CaptureError::Message(format!("app list: {e}")))?;
    Ok(apps
        .iter()
        .map(|app| WindowInfo {
            icon: process_icon(&app.package),
            ..window_info(app)
        })
        .collect())
}

/// Launcher icon for the app package bound as `process_path`.
pub fn process_icon(process_path: &str) -> Option<ProcessIcon> {
    let package = process_path.trim();
    if package.is_empty() {
        return None;
    }
    match bridge::app_icon(package, PROCESS_ICON_TARGET_PX) {
        Ok(icon) => icon.map(|i| ProcessIcon {
            width: i.side,
            height: i.side,
            rgba: i.rgba,
        }),
        Err(e) => {
            crate::note(&format!("app icon {package}: {e}"));
            None
        }
    }
}

/// App that came to the front last (tracked by the accessibility service).
pub fn get_active_window() -> Result<Option<WindowInfo>, CaptureError> {
    let app = bridge::foreground_app()
        .map_err(|e| CaptureError::Message(format!("foreground app: {e}")))?;
    Ok(app.as_ref().map(window_info))
}

fn window_info(app: &LaunchableApp) -> WindowInfo {
    WindowInfo {
        title: app.label.clone(),
        process_name: app.short_name().to_string(),
        process_path: app.package.clone(),
        icon: None,
    }
}

/// Focus Window on Android: launch (or bring forward) the app whose package is the
/// bound process path. A non-empty title must equal the app label.
#[derive(Debug, Default, Clone, Copy)]
pub struct OsWindowFocuser;

impl sqyre_ports::WindowFocuser for OsWindowFocuser {
    fn focus(&self, process_path: &str, window_title: &str) -> Result<(), AutomationError> {
        let package = process_path.trim();
        if package.is_empty() {
            return Err(AutomationError::InvalidArg(
                "focus window: app package is empty".into(),
            ));
        }
        bridge::launch(package, window_title.trim()).map_err(|e| match e {
            AndroidError::AppNotFound | AndroidError::TitleMismatch => {
                AutomationError::WindowNotFound {
                    process_path: package.to_string(),
                    title: window_title.to_string(),
                }
            }
            other => AutomationError::Backend(other.to_string()),
        })
    }
}
