//! Live runtime-variable snapshot sink (Go `runtime_vars`).

use std::sync::{Arc, Mutex};

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
        if let Ok(mut g) = self.inner.lock() {
            g.clear();
        }
    }

    pub fn snapshot(&self) -> Vec<(String, String)> {
        self.inner.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

impl RuntimeVarSink for SharedRuntimeVars {
    fn publish(&self, pairs: &[(String, String)]) {
        if let Ok(mut g) = self.inner.lock() {
            *g = pairs.to_vec();
        }
    }
}
