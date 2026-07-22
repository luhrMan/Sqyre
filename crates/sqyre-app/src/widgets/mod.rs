//! Shared egui widgets used across panels.

pub mod dialogs;
pub mod fields;
pub mod tags;

pub use dialogs::{
    confirm_cancel_row, save_cancel_row, save_cancel_row_ltr, ConfirmCancel, SaveCancel,
};
pub use fields::{
    combo_str, drag_field, drag_field_enabled, searchable_combo, searchable_combo_width,
    text_field, text_field_width, W_MULTILINE, W_TEXT, W_VAR,
};
pub use tags::{tag_chip_editor, TagChipOptions};
