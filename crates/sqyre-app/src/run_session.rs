//! Macro execution lifecycle and observability.

use crate::action_logs_ui::LogsWindowState;
use crate::app_backends::RunState;
use sqyre_domain::ActionId;
#[cfg(any(feature = "native-runtime", target_os = "windows"))]
use sqyre_hotkeys::ContinueWaitBridge;
use sqyre_hotkeys::MacroHotkeyBridge;
use sqyre_ports::{SharedActionLog, SharedClickFx, SharedHighlighter, SharedRuntimeVars};

pub(crate) struct RunSession {
    pub(crate) state: RunState,
    pub(crate) action_log: SharedActionLog,
    pub(crate) runtime_vars: SharedRuntimeVars,
    pub(crate) highlighter: SharedHighlighter,
    /// Press positions queued by the executor for the desktop click flash.
    pub(crate) click_fx: SharedClickFx,
    pub(crate) logs_window: Option<ActionId>,
    pub(crate) logs_state: LogsWindowState,
    #[cfg(any(feature = "native-runtime", target_os = "windows"))]
    pub(crate) continue_wait: ContinueWaitBridge,
    pub(crate) macro_hotkeys: MacroHotkeyBridge,
}
