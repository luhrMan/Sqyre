use super::types::ActivePicker;
use crate::window_types::{self, WindowInfo};
use eframe::egui;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;

pub(crate) fn fetch_open_windows() -> Result<Vec<WindowInfo>, String> {
    sqyre_capture::list_open_windows()
        .map(|rows| {
            rows.into_iter()
                .map(window_types::window_info_from_capture)
                .collect()
        })
        .map_err(|e| e.to_string())
}

pub fn refresh_window_picker(picker: &mut ActivePicker) {
    let ActivePicker::Window {
        load_error,
        pending,
        ..
    } = picker
    else {
        return;
    };
    if pending.is_some() {
        return;
    }
    *load_error = None;
    let (tx, rx) = mpsc::channel();
    #[cfg(feature = "native-runtime")]
    thread::spawn(move || {
        let _ = tx.send(fetch_open_windows());
    });
    #[cfg(not(feature = "native-runtime"))]
    {
        let _ = tx.send(Ok(Vec::new()));
    }
    *pending = Some(rx);
}

/// Apply a finished background window-list fetch; request repaint while still loading.
pub fn poll_window_picker_load(picker: &mut ActivePicker, ctx: &egui::Context) {
    let ActivePicker::Window {
        windows,
        load_error,
        pending,
        ..
    } = picker
    else {
        return;
    };
    let Some(rx) = pending.as_ref() else {
        return;
    };
    match rx.try_recv() {
        Ok(Ok(list)) => {
            *windows = list;
            *load_error = None;
            *pending = None;
        }
        Ok(Err(e)) => {
            windows.clear();
            *load_error = Some(e);
            *pending = None;
        }
        Err(TryRecvError::Empty) => {
            ctx.request_repaint();
        }
        Err(TryRecvError::Disconnected) => {
            windows.clear();
            *load_error = Some("window list fetch failed".into());
            *pending = None;
        }
    }
}

/// Open a Focus Window picker and kick off a background window-list fetch.
pub fn open_window_picker(process_path: &str, window_title: &str) -> ActivePicker {
    let mut picker = ActivePicker::Window {
        search: String::new(),
        process_path: process_path.to_string(),
        window_title: window_title.to_string(),
        windows: Vec::new(),
        load_error: None,
        scroll_to_selection: true,
        pending: None,
    };
    refresh_window_picker(&mut picker);
    picker
}
