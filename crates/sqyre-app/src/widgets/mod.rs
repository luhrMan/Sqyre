//! Shared egui widgets used across panels.

pub mod dialogs;
pub mod fields;
pub mod match_settings;
pub mod tags;

pub use dialogs::{
    confirm_cancel_row, confirm_window, dialog_constrain_rect, fit_dialog_popup, fit_dialog_window,
    poll_confirm_keys, save_cancel_row, save_cancel_row_ltr, ConfirmCancel, DIALOG_EDGE_MARGIN_FRAC,
    SaveCancel,
};
pub use fields::{
    combo_condition_operator, combo_enum, combo_str, combo_str_labeled, drag_field,
    drag_field_enabled, searchable_combo, searchable_combo_width, searchable_combo_with,
    text_field, text_field_width, W_MULTILINE, W_TEXT, W_VAR,
};
pub use match_settings::configure_match_blur_drag;
pub use tags::{tag_chip_editor, TagChipOptions};
