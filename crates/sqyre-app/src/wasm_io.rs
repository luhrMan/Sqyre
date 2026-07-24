//! WASM YAML import/export for the browser editor.

use crate::SqyreApp;
use parking_lot::Mutex;
use std::sync::Arc;

/// Shared slot filled by async file dialogs (import).
pub(crate) type PendingImport = Arc<Mutex<Option<Result<Vec<u8>, String>>>>;

pub(crate) fn new_pending_import() -> PendingImport {
    Arc::new(Mutex::new(None))
}

impl SqyreApp {
    /// Apply a completed import (called from the UI frame loop).
    pub(crate) fn take_pending_db_import(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            let Some(result) = self.pending_import.lock().take() else {
                return;
            };
            match result {
                Ok(bytes) => match sqyre_persist::Database::from_yaml_bytes(&bytes) {
                    Ok(db) => {
                        let mut catalog = db.program_catalog().unwrap_or_default();
                        crate::catalog::apply_main_monitor_resolution(&mut catalog);
                        let mut macros: Vec<_> = db.macros.values().cloned().collect();
                        macros.sort_by(|a, b| a.name.cmp(&b.name));
                        self.db = db;
                        self.catalog = catalog;
                        self.macros = macros;
                        self.selected_macro = 0;
                        self.clear_selected_actions();
                        self.load_error = None;
                        self.save_error = None;
                        *self.run.status.lock() = "Imported db.yaml.".into();
                        self.refresh_macro_hotkey_bindings();
                    }
                    Err(e) => {
                        self.load_error = Some(e.to_string());
                        *self.run.status.lock() = format!("Import failed: {e}");
                    }
                },
                Err(e) => {
                    *self.run.status.lock() = format!("Import failed: {e}");
                }
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn request_db_import(&self) {
        let pending = Arc::clone(&self.pending_import);
        let repaint = self.hotkey_repaint.lock().clone();
        wasm_bindgen_futures::spawn_local(async move {
            let file = rfd::AsyncFileDialog::new()
                .add_filter("YAML", &["yaml", "yml"])
                .pick_file()
                .await;
            let result = match file {
                Some(f) => Ok(f.read().await),
                None => Err("import cancelled".into()),
            };
            *pending.lock() = Some(result);
            if let Some(ctx) = repaint.as_ref() {
                ctx.request_repaint();
            }
        });
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn export_db_yaml(&mut self) {
        match self.db.to_yaml_bytes() {
            Ok(bytes) => {
                let pending = bytes;
                wasm_bindgen_futures::spawn_local(async move {
                    if let Some(file) = rfd::AsyncFileDialog::new()
                        .add_filter("YAML", &["yaml", "yml"])
                        .set_file_name("db.yaml")
                        .save_file()
                        .await
                    {
                        let _ = file.write(&pending).await;
                    }
                });
                *self.run.status.lock() = "Export started…".into();
            }
            Err(e) => {
                self.save_error = Some(e.to_string());
                *self.run.status.lock() = format!("Export failed: {e}");
            }
        }
    }
}
