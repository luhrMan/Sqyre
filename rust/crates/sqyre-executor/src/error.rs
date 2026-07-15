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

#[derive(Debug, Error)]
pub enum ExecError {
    #[error(transparent)]
    Flow(#[from] FlowSignal),
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, ExecError>;
