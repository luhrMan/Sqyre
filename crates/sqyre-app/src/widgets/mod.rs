//! Shared egui widgets used across panels.

mod action_icons;
pub mod context_menu;
pub(crate) mod controls;
pub mod dialogs;
pub mod empty_state;
pub mod fields;
pub mod headers;
pub mod match_settings;
pub mod scroll;
pub mod sections;
pub mod tags;

pub(crate) use action_icons::{action_glyph_font, action_icon_side, VectorIcon};
pub use context_menu::{
    menu_item, menu_item_danger, rect_context_menu, rect_danger_menu, response_context_menu,
    response_danger_menu, row_context_menu, row_danger_menu,
};
pub use controls::{
    dirty_action_button, glow_halo_shapes, glow_pulse, icon_button, icon_button_colored,
    icon_toggle, mouse_button_picker, press_state_toggle, record_icon_button, ICON_BTN_SIDE,
    PHOSPHOR_FILL_FAMILY,
};
pub use dialogs::{
    confirm_cancel_row, confirm_choice_row, confirm_window, consume_escape, dialog_constrain_rect,
    dismiss_row, fill_resize_body, fit_dialog_popup, fit_dialog_window,
    floating_scrollbar_overlay_width, save_cancel_row, sync_viewport_window_scale,
    visible_content_width, visible_height, visible_width, ConfirmCancel, ConfirmChoice,
    ConfirmKind, SaveCancel, ViewportScaleEvent, FLOATER_MIN_COMPACT, FLOATER_MIN_EDITOR,
    FLOATER_MIN_MODAL, FLOATER_MIN_PALETTE, FLOATER_MIN_PANEL, FLOATER_MIN_PICKER,
    FOOTER_RESERVE_SAVE_CANCEL,
};
pub use empty_state::{empty_state, list_vacancy, list_vacancy_copy, EmptyStateAction};
pub use fields::{
    combo_condition_operator, combo_enum, combo_str, combo_str_labeled, drag_field,
    drag_field_enabled, fill_row, searchable_combo, searchable_combo_width, searchable_combo_with,
    text_field, text_field_width, W_TEXT, W_VAR,
};
pub use headers::{heading_with_count, heading_with_count_and, title_with_count};
pub use match_settings::configure_match_blur_drag;
pub use scroll::{
    dialog_scroll, enable_dense_row_extend, scroll_both, scroll_vertical, SCROLL_SOURCE_NO_DRAG,
};
pub use sections::{section_separator, split_view, tinted_section, SplitView};
pub use tags::{tag_chip_editor, TagChipOptions};
