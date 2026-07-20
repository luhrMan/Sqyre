//! Shared egui widgets used across panels.

pub mod dialogs;
pub mod tags;

pub use dialogs::{
    confirm_cancel_row, save_cancel_row, save_cancel_row_ltr, ConfirmCancel, SaveCancel,
};
pub use tags::{tag_chip_editor, TagChipOptions};
