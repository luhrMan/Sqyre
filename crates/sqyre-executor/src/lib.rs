//! Macro executor with injected backends (automation + capture/match).

mod actions;
mod backends;
mod error;
mod log_draw;
mod navigate;
mod path_confine;
mod run;
mod search;
#[cfg(test)]
pub(crate) mod test_support;

pub use backends::*;
pub use error::{ExecError, FlowSignal, SearchError};
pub use log_draw::{crop_match_preview, draw_rect_rgb};
pub use run::{execute_action, execute_macro, execute_macro_with, ExecDeps, Executor};
pub use sqyre_ui_model::{
    clear_highlights, highlight_clear, highlight_cursor, highlight_fill, lines_for,
    ActionHighlighter, ActionLogEntry, ActionLogger, HighlightEvent, HighlightKind,
    HighlightSnapshot, LogImage, RuntimeVarSink, SharedActionLog, SharedHighlighter,
    SharedRuntimeVars, MAX_ENTRIES_PER_ACTION,
};
