//! Shared egui widgets used across panels.

mod action_icons;
pub mod context_menu;
pub(crate) mod controls;
pub mod dialogs;
pub mod elide;
pub mod empty_state;
pub mod fields;
pub mod headers;
pub mod match_settings;
pub mod num_entry;
pub mod num_or_var;
pub mod overflow;
mod pair;
mod row_ids;
pub mod scroll;
pub mod sections;
mod spinner;
pub mod tags;
pub mod wrap;

pub(crate) use action_icons::{action_glyph_font, action_icon_side, VectorIcon};
pub use context_menu::{
    context_menu_popup, menu_item, menu_item_danger, parent_popup, rect_context_menu,
    rect_danger_menu, register_nested_popup, response_context_menu, response_danger_menu,
    row_context_menu, row_danger_menu,
};
pub use controls::{
    add_button, cancel_icon_button, clear_icon_button, confirm_icon_button, dirty_action_button,
    error_badge, error_icon, icon_btn_side, icon_button, icon_toggle, mouse_button_picker,
    press_state_toggle, record_icon_button, remove_icon_button, warn_badge, warn_icon, IconButton,
    IconToggle, PHOSPHOR_FILL_FAMILY,
};
pub use dialogs::{
    clamp_row_right_to_clip, confirm_cancel_row, confirm_choice_row, confirm_window,
    consume_escape, dialog_constrain_rect, dismiss_row, fill_resize_body, fit_dialog_popup,
    floating_scrollbar_overlay_width, save_row, sync_viewport_window_scale, visible_content_width,
    visible_height, visible_width, ConfirmCancel, ConfirmChoice, ConfirmKind, SaveCancel,
    ViewportScaleEvent, FLOATER_MIN_COMPACT, FLOATER_MIN_EDITOR, FLOATER_MIN_MODAL,
    FLOATER_MIN_PALETTE, FLOATER_MIN_PANEL, FLOATER_MIN_PICKER, FOOTER_RESERVE_ACTION,
};
pub use empty_state::{empty_state, list_vacancy, list_vacancy_copy, EmptyCta, EmptyStateAction};
pub use fields::{
    combo_condition_operator, combo_enum, combo_str, combo_str_labeled, drag_field,
    drag_field_enabled, fill_row, name_edit_width, num_text_edit, num_text_field_opts,
    reset_focus_search, search_field, searchable_combo, text_field, text_field_opts, NumFieldOpts,
    SearchFieldOpts, SearchTrailing, SearchableComboOpts, TextFieldOpts, W_TEXT, W_VAR,
};
pub use headers::{count_suffix, count_text, heading_with_count, title_with_count};
pub use match_settings::configure_match_blur_drag;
pub use num_entry::{num_entry, NumEntry};
pub(crate) use num_or_var::{
    apply_num_or_var_choice, non_literal, num_or_var_hover, num_or_var_menu, take_var_focus,
    variable_entry,
};
pub use num_or_var::{num_or_var_field, NumOrVarOpts};
pub use overflow::{always_bar, text_width, OverflowItem};
pub(crate) use pair::pair_slots;
pub use pair::{pair_icon, pair_labeled, pair_leading, PAIR_SLOT_GAP};
pub use row_ids::{remove_row_id, stable_row_ids};
pub use scroll::{
    enable_dense_row_extend, scroll_vertical, show_dialog_scroll, show_dialog_scroll_mod,
    show_overflow, show_scroll_edge, show_scroll_edge_fit, SCROLL_SOURCE_NO_DRAG,
};
pub use sections::{
    advanced_section, collapsible_header, collapsible_section, float_bottom_right,
    frame_outer_rect, section_separator, split_view, tinted_section, SplitView,
};
pub(crate) use spinner::{brand_spinner, loading_line};
pub use tags::{tag_chip_editor, tag_chip_list, tag_draft_editor, TagChipOptions};
pub use wrap::{wrap_unit, wrapped_row};
