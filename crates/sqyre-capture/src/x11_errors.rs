//! Catch Xlib errors on Sqyre capture connections (avoid fatal abort on BadMatch).

use crate::error::CaptureError;
use std::cell::Cell;
use x11::xlib::{Display, XErrorEvent, XSetErrorHandler, XSync};

thread_local! {
    static LAST_X11_ERROR: Cell<Option<X11Error>> = const { Cell::new(None) };
}

#[derive(Clone, Copy, Debug)]
struct X11Error {
    error_code: u8,
    request_code: u8,
    minor_code: u8,
    serial: u64,
}

extern "C" fn capture_error_handler(_display: *mut Display, event: *mut XErrorEvent) -> i32 {
    // SAFETY: Xlib invokes the handler with a valid error event pointer.
    unsafe {
        if !event.is_null() {
            let e = &*event;
            LAST_X11_ERROR.with(|cell| {
                cell.set(Some(X11Error {
                    error_code: e.error_code,
                    request_code: e.request_code,
                    minor_code: e.minor_code,
                    serial: e.serial,
                }));
            });
        }
    }
    1
}

/// Run `f` with a temporary X error handler; map a recorded X error to [`CaptureError`].
pub fn with_capture_error_handler<F, T>(display: *mut Display, f: F) -> Result<T, CaptureError>
where
    F: FnOnce() -> Result<T, CaptureError>,
{
    LAST_X11_ERROR.with(|cell| cell.set(None));
    // SAFETY: previous handler is restored after `f` and `XSync`.
    unsafe {
        let previous = XSetErrorHandler(Some(capture_error_handler));
        let result = f();
        XSync(display, 0);
        XSetErrorHandler(previous);
        if let Some(err) = LAST_X11_ERROR.with(|cell| cell.take()) {
            return Err(x11_error_to_capture(err));
        }
        result
    }
}

fn x11_error_to_capture(err: X11Error) -> CaptureError {
    let name = match err.error_code {
        8 => "BadMatch",
        9 => "BadDrawable",
        10 => "BadAccess",
        11 => "BadAlloc",
        12 => "BadColor",
        13 => "BadGC",
        14 => "BadIDChoice",
        15 => "BadName",
        16 => "BadLength",
        17 => "BadImplementation",
        _ => "XError",
    };
    CaptureError::Message(format!(
        "X11 {name} (request {} minor {} serial {})",
        err.request_code, err.minor_code, err.serial
    ))
}
