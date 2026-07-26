//! Process-wide capturer singleton and Arc [`ScreenCapturer`] wrapper.
//!
//! Platform modules provide `OsCapturer` with `*_ref` methods; this macro adds
//! `shared_capturer`, `SharedRunCapturer`, and a shared [`ScreenCapturer`] forwarder
//! used by both `OsCapturer` and `SharedRunCapturer`.

/// Define `shared_capturer`, `SharedRunCapturer`, and `ScreenCapturer` for `OsCapturer`.
///
/// `$capturer` must implement `open() -> Result<Self, E: ToString>` and the
/// `capture_rect_ref` / `capture_rect_rgb_ref` / `virtual_bounds_ref` /
/// `monitor_sizes_ref` / `monitor_rects_ref` methods used below.
#[macro_export]
macro_rules! define_shared_run_capturer {
    () => {
        /// Process-wide capturer for UI offload (cloned via [`Arc`]; access serialized by inner Mutex).
        static SHARED_UI_CAPTURER: ::std::sync::OnceLock<
            Result<::std::sync::Arc<OsCapturer>, String>,
        > = ::std::sync::OnceLock::new();

        /// Shared capturer for UI-thread offload (preview tooltips, AutoPic, etc.).
        pub fn shared_capturer() -> Result<::std::sync::Arc<OsCapturer>, String> {
            match SHARED_UI_CAPTURER.get_or_init(|| {
                OsCapturer::open()
                    .map(::std::sync::Arc::new)
                    .map_err(|e| e.to_string())
            }) {
                Ok(c) => Ok(::std::sync::Arc::clone(c)),
                Err(e) => Err(e.clone()),
            }
        }

        /// [`ScreenCapturer`] over a shared [`Arc`] capturer (macro run thread).
        pub struct SharedRunCapturer(pub ::std::sync::Arc<OsCapturer>);

        $crate::__impl_screen_capturer_forward!(OsCapturer);
        $crate::__impl_screen_capturer_forward!(SharedRunCapturer, 0);
    };
}

/// Private [`ScreenCapturer`] forwarder shared by `OsCapturer` (accessed as `self`) and
/// `SharedRunCapturer` (accessed as `self.0`, passed as the optional tuple-field index),
/// so [`define_shared_run_capturer`] only has to write the capture logic once instead of
/// two nearly identical trait impls.
#[macro_export]
#[doc(hidden)]
macro_rules! __impl_screen_capturer_forward {
    ($ty:ty $(, $field:tt)?) => {
        impl ::sqyre_ports::ScreenCapturer for $ty {
            fn capture_monitor(
                &mut self,
                display_index: i32,
            ) -> Result<::image::RgbaImage, ::sqyre_ports::CaptureError> {
                if display_index != 0 {
                    return Err(::sqyre_ports::CaptureError::UnsupportedDisplay(
                        display_index,
                    ));
                }
                let vb = self $(.$field)? .virtual_bounds_ref()?;
                self $(.$field)? .capture_rect_ref(vb)
            }

            fn capture_rect(
                &mut self,
                rect: ::sqyre_ports::DesktopRect,
            ) -> Result<::image::RgbaImage, ::sqyre_ports::CaptureError> {
                self $(.$field)? .capture_rect_ref(rect)
            }

            fn capture_rect_rgb(
                &mut self,
                rect: ::sqyre_ports::DesktopRect,
            ) -> Result<::sqyre_ports::RgbCapture, ::sqyre_ports::CaptureError> {
                self $(.$field)? .capture_rect_rgb_ref(rect)
            }

            fn virtual_bounds(
                &mut self,
            ) -> Result<::sqyre_ports::DesktopRect, ::sqyre_ports::CaptureError> {
                self $(.$field)? .virtual_bounds_ref()
            }

            fn monitor_sizes(&mut self) -> Result<Vec<(i32, i32)>, ::sqyre_ports::CaptureError> {
                self $(.$field)? .monitor_sizes_ref()
            }
        }
    };
}
