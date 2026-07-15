//! Domain model for macros and the 21 action kinds.
//!
//! Action `type` strings and field names follow Go serialize codecs (newer
//! tip including `while` and `navigateselect`). UIDs are runtime-only unless
//! injected for undo/clipboard snapshots.

mod action;
mod labels;
mod macro_model;
mod scalar;
mod variables;

pub use action::*;
pub use labels::{action_type_description, action_type_label};
pub use macro_model::*;
pub use scalar::*;
pub use variables::*;
