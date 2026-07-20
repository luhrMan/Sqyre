//! Macro executor with injected backends (automation + capture/match).

mod action_log;
mod actions;
mod backends;
mod error;
mod highlight;
mod navigate;
mod run;
mod runtime_vars;
mod search;
#[cfg(test)]
pub(crate) mod test_support;

pub use action_log::{
    crop_match_preview, draw_rect_rgb, ActionLogEntry, ActionLogger, LogImage, SharedActionLog,
    MAX_ENTRIES_PER_ACTION,
};
pub use backends::*;
pub use error::{ExecError, FlowSignal};
pub use highlight::{
    ActionHighlighter, HighlightEvent, HighlightKind, HighlightSnapshot, SharedHighlighter,
};
pub use run::{execute_action, execute_macro, execute_macro_with, ExecDeps, Executor};
pub use runtime_vars::{RuntimeVarSink, SharedRuntimeVars};
