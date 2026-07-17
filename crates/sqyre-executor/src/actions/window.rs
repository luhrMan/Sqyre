//! Window actions: FocusWindow.

use crate::error::{ExecError, Result};
use crate::run::Executor;
use sqyre_domain::ActionId;

pub(crate) fn execute_focus_window(
    exec: &mut Executor<'_>,
    action_id: ActionId,
    process_path: &str,
    window_title: &str,
) -> Result<()> {
    let path = process_path.trim();
    let title = window_title.trim();
    if path.is_empty() {
        return Err(ExecError::Message(
            "focus window: no executable path set".into(),
        ));
    }
    if title.is_empty() {
        return Err(ExecError::Message(
            "focus window: no window title set".into(),
        ));
    }
    let focuser = exec
        .deps
        .window_focuser
        .ok_or_else(|| ExecError::Message("focus window: window focuser not configured".into()))?;
    focuser
        .focus(path, title)
        .map_err(|e| ExecError::Message(format!("focus window {title:?} ({path}): {e}")))?;
    exec.log(action_id, format!("Focus Window: {title} ({path})"));
    Ok(())
}
