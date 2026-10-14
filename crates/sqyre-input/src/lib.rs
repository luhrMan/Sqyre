//! OS `AutomationBackend`: rustautogui + arboard on desktop, accessibility gestures on Android.

#[cfg(target_os = "android")]
mod android;
#[cfg(not(target_os = "android"))]
mod desktop;

#[cfg(target_os = "android")]
pub use android::{prepare_for_automation, release_held_inputs, OsAutomation};
#[cfg(not(target_os = "android"))]
pub use desktop::{prepare_for_automation, release_held_inputs, OsAutomation};
