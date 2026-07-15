//! Domain model for macros and the 22 action kinds.
//!
//! Action `type` strings and field names follow Go serialize codecs (newer
//! tip including `while`, `navigateselect`, and `navigatekey`). UIDs are
//! runtime-only unless injected for undo/clipboard snapshots.

mod action;
mod display;
mod labels;
mod macro_model;
mod rename;
mod scalar;
mod variables;

pub use action::*;
pub use display::*;
pub use labels::{action_type_description, action_type_label};
pub use macro_model::*;
pub use rename::*;
pub use scalar::*;
pub use variables::*;
