//! Macro executor with injected backends (Go `AutomationBackend` + capture/match).

mod action_log;
mod backends;
mod error;
mod expr;
mod highlight;
mod misc;
mod run;
mod search;

pub use action_log::{ActionLogger, SharedActionLog, MAX_LINES_PER_ACTION};
pub use backends::*;
pub use error::{ExecError, FlowSignal};
pub use highlight::{
    ActionHighlighter, HighlightEvent, HighlightKind, HighlightSnapshot, SharedHighlighter,
};
pub use run::{execute_action, execute_macro, execute_macro_with, ExecDeps, Executor};
pub use search::MatchFacade;
