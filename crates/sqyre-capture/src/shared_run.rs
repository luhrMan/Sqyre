//! Process-wide capturer singleton and Arc [`ScreenCapturer`] wrapper.
//!
//! Platform modules provide `OsCapturer` with `*_ref` methods; this macro adds
//! `shared_capturer`, `SharedRunCapturer`, and the `&mut self` trait forwards.

/// Define `shared_capturer`, `SharedRunCapturer`, and `ScreenCapturer` for `OsCapturer`.
///
/// `$capturer` must implement `open() -> Result<Self, E: ToString>` and the
/// `capture_rect_ref` / `capture_rect_rgb_ref` / `virtual_bounds_ref` /
/// `monitor_sizes_ref` methods used below.
#[macro_export]
macro_rules! define_shared_run_capturer {
    () => {
        /// Process-wide capturer for UI offload (cloned via [`Arc`]; access serialized by inner Mutex).
        static SHARED_UI_CAPTURER: ::std::sync::OnceLock<
            Result<::std::sync::Arc<OsCapturer>, String>,
        > = ::std::sync::OnceLock::new();

        /// Shared capturer for UI-thread offload (preview tooltips, AutoPic, etc.).
        pub fn shared_capturer() -> Result<::std::sync::Arc<OsCapturer>, String> {
            match SHARED_UI_CAPTURER
                .get_or_init(|| OsCapturer::open().map(::std::sync::Arc::new).map_err(|e| e.to_string()))
            {
                Ok(c) => Ok(::std::sync::Arc::clone(c)),
                Err(e) => Err(e.clone()),
            }
        }

        impl ::sqyre_executor::ScreenCapturer for OsCapturer {
            fn capture_monitor(
                &mut self,
                display_index: i32,
            ) -> Result<::image::RgbaImage, String> {
                if display_index != 0 {
                    return Err($crate::error::CaptureError::UnsupportedDisplay(display_index).into());
                }
                let vb = self.virtual_bounds_ref()?;
                self.capture_rect_ref(vb)
            }

            fn capture_rect(
                &mut self,
                rect: ::sqyre_executor::DesktopRect,
            ) -> Result<::image::RgbaImage, String> {
                self.capture_rect_ref(rect)
            }

            fn capture_rect_rgb(
                &mut self,
                rect: ::sqyre_executor::DesktopRect,
            ) -> Result<::sqyre_executor::RgbCapture, String> {
                self.capture_rect_rgb_ref(rect)
            }

            fn virtual_bounds(&mut self) -> Result<::sqyre_executor::DesktopRect, String> {
                self.virtual_bounds_ref()
            }

            fn monitor_sizes(&mut self) -> Result<Vec<(i32, i32)>, String> {
                self.monitor_sizes_ref()
            }
        }

        /// [`ScreenCapturer`] over a shared [`Arc`] capturer (macro run thread).
        pub struct SharedRunCapturer(pub ::std::sync::Arc<OsCapturer>);

        impl ::sqyre_executor::ScreenCapturer for SharedRunCapturer {
            fn capture_monitor(
                &mut self,
                display_index: i32,
            ) -> Result<::image::RgbaImage, String> {
                if display_index != 0 {
                    return Err($crate::error::CaptureError::UnsupportedDisplay(display_index).into());
                }
                let vb = self.0.virtual_bounds_ref()?;
                self.0.capture_rect_ref(vb)
            }

            fn capture_rect(
                &mut self,
                rect: ::sqyre_executor::DesktopRect,
            ) -> Result<::image::RgbaImage, String> {
                self.0.capture_rect_ref(rect)
            }

            fn capture_rect_rgb(
                &mut self,
                rect: ::sqyre_executor::DesktopRect,
            ) -> Result<::sqyre_executor::RgbCapture, String> {
                self.0.capture_rect_rgb_ref(rect)
            }

            fn virtual_bounds(&mut self) -> Result<::sqyre_executor::DesktopRect, String> {
                self.0.virtual_bounds_ref()
            }

            fn monitor_sizes(&mut self) -> Result<Vec<(i32, i32)>, String> {
                self.0.monitor_sizes_ref()
            }
        }
    };
}
