//! Integer status codes returned by `SqyreBridge` Kotlin methods.
//!
//! Values must match the constants on `com.sqyre.app.SqyreBridge`.

use crate::AndroidError;

pub const OK: i32 = 0;
pub const ACCESSIBILITY_OFF: i32 = 1;
pub const REJECTED: i32 = 2;
pub const NO_FOCUSED_TEXT: i32 = 3;
pub const APP_NOT_FOUND: i32 = 4;
pub const TITLE_MISMATCH: i32 = 5;

/// Map a Kotlin status code to `Ok` or the matching [`AndroidError`].
pub fn check(code: i32) -> Result<(), AndroidError> {
    match code {
        OK => Ok(()),
        ACCESSIBILITY_OFF => Err(AndroidError::AccessibilityOff),
        REJECTED => Err(AndroidError::Rejected),
        NO_FOCUSED_TEXT => Err(AndroidError::NoFocusedText),
        APP_NOT_FOUND => Err(AndroidError::AppNotFound),
        TITLE_MISMATCH => Err(AndroidError::TitleMismatch),
        other => Err(AndroidError::UnknownStatus(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_every_known_code() {
        assert!(check(OK).is_ok());
        assert!(matches!(
            check(ACCESSIBILITY_OFF),
            Err(AndroidError::AccessibilityOff)
        ));
        assert!(matches!(check(REJECTED), Err(AndroidError::Rejected)));
        assert!(matches!(
            check(NO_FOCUSED_TEXT),
            Err(AndroidError::NoFocusedText)
        ));
        assert!(matches!(
            check(APP_NOT_FOUND),
            Err(AndroidError::AppNotFound)
        ));
        assert!(matches!(
            check(TITLE_MISMATCH),
            Err(AndroidError::TitleMismatch)
        ));
        assert!(matches!(check(42), Err(AndroidError::UnknownStatus(42))));
    }
}
