//! Macro executor with injected backends (Go `AutomationBackend` + capture/match).

mod backends;
mod error;
mod run;
mod search;

pub use backends::*;
pub use error::{ExecError, FlowSignal};
pub use run::{execute_action, execute_macro, execute_macro_with, ExecDeps, Executor};
pub use search::MatchFacade;
