//! Process-wide capturer singleton and Arc [`ScreenCapturer`] wrapper.
//!
//! Platform modules provide `OsCapturer` with `*_ref` methods; this macro adds
//! `shared_capturer`, `SharedRunCapturer`, and a shared [`ScreenCapturer`] forwarder
//! used by both `OsCapturer` and `SharedRunCapturer`.

/// Define `shared_capturer`, `SharedRunCapturer`, and `ScreenCapturer` for `OsCapturer`.
///
/// `$capturer` must implement `open() -> Result<Self, ::sqyre_ports::CaptureError>` and the
/// `capture_rect_ref` / `capture_rect_rgb_ref` / `capture_rect_rgb_fresh_ref` /
/// `virtual_bounds_ref` / `monitor_sizes_ref` / `monitor_rects_ref` methods used below.
#[macro_export]
macro_rules! define_shared_run_capturer {
    () => {
        static SHARED_UI_CAPTURER: ::parking_lot::Mutex<
            Option<
                Result<::std::sync::Arc<OsCapturer>, ::sqyre_ports::CaptureError>,
            >,
        > = ::parking_lot::Mutex::new(None);
        static SHARED_UI_CAPTURER_CV: ::parking_lot::Condvar = ::parking_lot::Condvar::new();
        static SHARED_UI_OPENING: ::std::sync::atomic::AtomicBool =
            ::std::sync::atomic::AtomicBool::new(false);
        static SHARED_UI_GENERATION: ::std::sync::atomic::AtomicU64 =
            ::std::sync::atomic::AtomicU64::new(0);
        ::std::thread_local! {
            static SHARED_UI_OPEN_GEN: ::std::cell::Cell<u64> = const { ::std::cell::Cell::new(0) };
        }

        /// Shared capturer for UI-thread offload (preview tooltips, AutoPic, etc.).
        ///
        /// Blocks while `OsCapturer::open()` runs. On Wayland that can wait on the
        /// portal ScreenCast picker — call [`shared_capturer_if_ready`] from the UI
        /// thread instead, and open from a background task.
        pub fn shared_capturer(
        ) -> Result<::std::sync::Arc<OsCapturer>, ::sqyre_ports::CaptureError> {
            use ::std::sync::atomic::Ordering;
            loop {
                let mut slot = SHARED_UI_CAPTURER.lock();
                if let Some(r) = slot.as_ref() {
                    return match r {
                        Ok(c) => Ok(::std::sync::Arc::clone(c)),
                        Err(e) => Err(e.clone()),
                    };
                }
                if SHARED_UI_OPENING
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    let gen = SHARED_UI_GENERATION.load(Ordering::SeqCst);
                    SHARED_UI_OPEN_GEN.with(|g| g.set(gen));
                    drop(slot);
                    let result = OsCapturer::open().map(::std::sync::Arc::new);
                    let mut slot = SHARED_UI_CAPTURER.lock();
                    if SHARED_UI_GENERATION.load(Ordering::SeqCst) == gen {
                        *slot = Some(result.clone());
                    }
                    SHARED_UI_OPENING.store(false, Ordering::SeqCst);
                    SHARED_UI_CAPTURER_CV.notify_all();
                    return result;
                }
                SHARED_UI_CAPTURER_CV.wait(&mut slot);
            }
        }

        /// Peek at [`shared_capturer`] without starting or waiting for `open()`.
        ///
        /// `None` while another thread is still inside [`shared_capturer`]; `Some`
        /// after the first open attempt has finished (ok or error).
        pub fn shared_capturer_if_ready() -> Option<
            Result<::std::sync::Arc<OsCapturer>, ::sqyre_ports::CaptureError>,
        > {
            SHARED_UI_CAPTURER.lock().as_ref().map(|r| match r {
                Ok(c) => Ok(::std::sync::Arc::clone(c)),
                Err(e) => Err(e.clone()),
            })
        }

        /// True while a thread is inside [`OsCapturer::open`] for the shared slot.
        pub fn shared_capturer_is_opening() -> bool {
            SHARED_UI_OPENING.load(::std::sync::atomic::Ordering::SeqCst)
        }

        /// Drop the process-wide capturer so the next [`shared_capturer`] opens a new session.
        pub fn reset_shared_capturer() {
            use ::std::sync::atomic::Ordering;
            SHARED_UI_GENERATION.fetch_add(1, Ordering::SeqCst);
            *SHARED_UI_CAPTURER.lock() = None;
            SHARED_UI_CAPTURER_CV.notify_all();
        }

        /// True when this thread's in-flight [`shared_capturer`] open was cancelled by
        /// [`reset_shared_capturer`].
        pub fn shared_capturer_open_superseded() -> bool {
            SHARED_UI_OPEN_GEN.with(|g| {
                g.get()
                    != SHARED_UI_GENERATION.load(::std::sync::atomic::Ordering::SeqCst)
            })
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

            fn capture_rect_rgb_fresh(
                &mut self,
                rect: ::sqyre_ports::DesktopRect,
            ) -> Result<::sqyre_ports::RgbCapture, ::sqyre_ports::CaptureError> {
                self $(.$field)? .capture_rect_rgb_fresh_ref(rect)
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
