//! Window actions: FocusWindow.

use crate::error::{ExecError, Result};
use crate::run::Executor;
use sqyre_domain::ActionId;
use sqyre_ports::FocusedWindow;

pub(crate) fn execute_focus_window(
    exec: &mut Executor<'_>,
    action_id: ActionId,
    process_path: &str,
    window_title: &str,
    previous: bool,
) -> Result<()> {
    let target = if previous {
        exec.previous_focus.clone().ok_or_else(|| {
            ExecError::Message(
                "focus window: no previous window yet; run a Focus Window step for a specific window first"
                    .into(),
            )
        })?
    } else {
        named_target(process_path, window_title)?
    };
    let focuser = exec
        .deps
        .window_focuser
        .ok_or_else(|| ExecError::Message("focus window: window focuser not configured".into()))?;
    let before = match focuser.focused() {
        Ok(w) => w,
        Err(e) => {
            exec.log(
                action_id,
                format!("Focus Window: could not read the focused window ({e}); previous window not updated"),
            );
            None
        }
    };
    focuser.focus(&target.process_path, &target.window_title)?;
    if let Some(before) = before.filter(|w| *w != target) {
        exec.previous_focus = Some(before);
    }
    let FocusedWindow {
        process_path: path,
        window_title: title,
    } = &target;
    let kind = if previous { " previous" } else { "" };
    exec.log(action_id, format!("Focus Window{kind}: {title} ({path})"));
    Ok(())
}

fn named_target(process_path: &str, window_title: &str) -> Result<FocusedWindow> {
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
    Ok(FocusedWindow {
        process_path: path.to_string(),
        window_title: title.to_string(),
    })
}
