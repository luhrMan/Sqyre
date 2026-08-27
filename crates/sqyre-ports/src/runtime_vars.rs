//! Live runtime-variable snapshot sink.

use parking_lot::Mutex;
use std::sync::Arc;

/// Optional live runtime-variable publisher (UI “Live variables”).
pub trait RuntimeVarSink: Send + Sync {
    fn publish(&self, pairs: &[(String, String)]);
}

/// Shared snapshot of runtime variables for the UI.
#[derive(Clone, Default)]
pub struct SharedRuntimeVars {
    inner: Arc<Mutex<Vec<(String, String)>>>,
}

impl SharedRuntimeVars {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&self) {
        self.inner.lock().clear();
    }

    pub fn snapshot(&self) -> Vec<(String, String)> {
        self.inner.lock().clone()
    }
}

impl RuntimeVarSink for SharedRuntimeVars {
    fn publish(&self, pairs: &[(String, String)]) {
        *self.inner.lock() = pairs.to_vec();
    }
}
