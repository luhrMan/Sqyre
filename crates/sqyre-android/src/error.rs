//! Typed errors for the Android shell bridge.

use thiserror::Error;

/// Failure talking to the Kotlin shell or reading a projection frame.
#[derive(Debug, Error)]
pub enum AndroidError {
    #[error("Android bridge is not initialized (SqyreBridge.nativeInit has not run)")]
    NotInitialized,
    #[error(
        "the Sqyre accessibility service is off; turn it on in Android Settings > Accessibility"
    )]
    AccessibilityOff,
    #[error("Android rejected the gesture or action")]
    Rejected,
    #[error("no focused text field to type into")]
    NoFocusedText,
    #[error("app is not installed or has no launcher activity")]
    AppNotFound,
    #[error("app label does not match the window title")]
    TitleMismatch,
    #[error("Android bridge returned unknown status {0}")]
    UnknownStatus(i32),
    #[error("screen recording has not started")]
    NotStarted,
    #[error("screen recording has not delivered a frame yet")]
    NoFrame,
    #[error("screen recording stopped")]
    ProjectionStopped,
    #[error("bad screen-recording frame: {0}")]
    BadFrame(&'static str),
    #[cfg(target_os = "android")]
    #[error("JNI: {0}")]
    Jni(#[from] jni::errors::Error),
}
