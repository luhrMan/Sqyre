//! Domain model for macros and the 21 action kinds.
//!
//! Action `type` strings and field names follow the serialize codecs (newer
//! tip including `while`, `navigateselect`, and `navigatekey`). UIDs are
//! runtime-only unless injected for undo/clipboard snapshots.

mod action;
mod blank;
mod display;
mod expr;
mod labels;
mod macro_model;
mod rename;
mod scalar;
mod set_value;
mod variables;

pub use action::*;
pub use blank::{
    action_picker_category, action_templates, blank_action, ActionTemplate, ACTION_PICKER_CATEGORIES,
};
pub use display::*;
pub use expr::{evaluate_expression, numeric_to_scalar};
pub use labels::{action_type_description, action_type_label};
pub use macro_model::*;
pub use rename::*;
pub use scalar::*;
pub use set_value::{
    looks_like_arithmetic, resolve_scalar_int, resolve_set_variable_value, resolve_variables_in_text,
};
pub use variables::*;
