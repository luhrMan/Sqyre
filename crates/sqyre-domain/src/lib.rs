//! Domain model for macros and the [`ACTION_KIND_COUNT`] action kinds.
//!
//! Action `type` strings and field names are the YAML wire vocabulary
//! (`while`, `navigateselect`, `searcharea`, …). UIDs are runtime-only unless
//! injected for undo/clipboard snapshots.

mod action;
mod bindings;
mod blank;
mod color;
mod expr;
mod macro_model;
mod rename;
mod scalar;
mod set_value;
mod taxonomy;
mod variables;

pub use action::WIRE_TYPE_KEYS;
pub use action::*;
pub use bindings::{BindingRole, VariableBinding};
pub use blank::{action_templates, blank_action, test_action, ActionTemplate};
pub use color::{
    format_hex_color, normalize_hex_rgb, parse_hex_color, ACTION_COLOR_CATEGORIES,
    ACTION_COLOR_KEY_CONTROL_FLOW, ACTION_COLOR_KEY_DEFAULT, ACTION_COLOR_KEY_DETECTION,
    ACTION_COLOR_KEY_MISCELLANEOUS, ACTION_COLOR_KEY_MOUSE_KEYBOARD, ACTION_COLOR_KEY_VARIABLES,
    ACTION_COLOR_KEY_WAIT,
};
pub use expr::{evaluate_expression, numeric_to_scalar};
pub use macro_model::*;
pub use rename::*;
pub use scalar::*;
pub use set_value::{
    expand_variable_refs, looks_like_arithmetic, resolve_scalar_int, resolve_set_variable_value,
    resolve_variables_in_text,
};
pub use taxonomy::{
    action_color_category, action_color_key, action_delay_class, action_icon,
    action_picker_category, action_type_description, action_type_label, action_type_table,
    ActionTypeMeta, DelayClass, ACTION_KIND_COUNT, ACTION_PICKER_CATEGORIES,
};
pub use variables::*;
