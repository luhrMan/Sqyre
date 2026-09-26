//! Shared egui widgets used across panels.

pub(crate) mod controls;
pub mod dialogs;
pub mod empty_state;
pub mod fields;
pub mod headers;
pub mod match_settings;
pub mod tags;

pub use controls::{
    dirty_action_button, icon_button, icon_button_bare_colored, icon_button_colored,
    mouse_button_picker, press_state_toggle, record_icon_button, ICON_BTN_SIDE,
};
pub use dialogs::{
    confirm_cancel_row, confirm_choice_row, confirm_window, dialog_constrain_rect, dismiss_row,
    fill_resize_body, fit_dialog_popup, fit_dialog_window, floating_scrollbar_overlay_width,
    save_cancel_row, sync_viewport_window_scale, visible_content_width, visible_height,
    visible_size, visible_width, ConfirmCancel, ConfirmChoice, ConfirmKind, SaveCancel,
    ViewportScaleEvent,
};
pub use empty_state::{empty_state, list_vacancy, EmptyStateAction};
pub use fields::{
    combo_condition_operator, combo_enum, combo_str, combo_str_labeled, drag_field,
    drag_field_enabled, searchable_combo, searchable_combo_width, searchable_combo_with,
    text_field, text_field_width, W_TEXT, W_VAR,
};
pub use headers::{heading_with_count, title_with_count};
pub use match_settings::configure_match_blur_drag;
pub use tags::{tag_chip_editor, TagChipOptions};
