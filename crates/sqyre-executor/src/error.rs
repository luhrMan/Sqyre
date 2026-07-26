use sqyre_match::MatchError;
use sqyre_ports::{AutomationError, CaptureError, PortError};
use thiserror::Error;

/// Control-flow signals consumed by loop / foreach / while / imagesearch.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FlowSignal {
    #[error("break")]
    Break,
    #[error("continue")]
    Continue,
    #[error("stopped")]
    Stopped,
}

/// Image Search per-variant failure: template load (icon cache / decode) or
/// template matching. Keeps [`MatchError`] intact instead of erasing it to a
/// `String` at the point it's produced.
#[derive(Debug, Error)]
pub enum SearchError {
    #[error("{0}")]
    Template(String),
    #[error(transparent)]
    Match(#[from] MatchError),
}

#[derive(Debug, Error)]
pub enum ExecError {
    #[error(transparent)]
    Flow(#[from] FlowSignal),
    #[error(transparent)]
    Capture(#[from] CaptureError),
    #[error(transparent)]
    Automation(#[from] AutomationError),
    #[error(transparent)]
    Search(#[from] SearchError),
    #[error(transparent)]
    Port(#[from] PortError),
    /// Domain / macro-configuration failure with no typed port error behind it.
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, ExecError>;
