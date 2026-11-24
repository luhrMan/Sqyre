//! Portal session eis_input helpers.

use super::*;

/// Whether EIS is ready to inject pointer/keyboard.
pub fn portal_input_ready() -> bool {
    EIS_READY.load(Ordering::SeqCst)
}

/// Block until the EIS thread resumes a device, fails, or the wait budget is spent.
///
/// Capture becomes ready on the first PipeWire frame. COSMIC often delivers that
/// frame while the EIS handshake is still waiting for a device to resume; sampling
/// [`portal_input_ready`] at that instant reports a grant with no backend.
pub fn portal_input_wait() -> Result<(), AutomationError> {
    let deadline = Instant::now() + EIS_BECOME_READY_LIMIT;
    loop {
        let mut snap = eis_snapshot();
        if snap.connecting && !snap.ready && snap.failure.is_none() && Instant::now() >= deadline {
            EIS_WAIT_EXHAUSTED.store(true, Ordering::SeqCst);
            snap.exhausted = true;
        }
        match eis_wait_decision(&snap) {
            EisWaitDecision::Ready => {
                EIS_WAIT_EXHAUSTED.store(false, Ordering::SeqCst);
                return Ok(());
            }
            EisWaitDecision::Failed(err) => return Err(AutomationError::Backend(err)),
            EisWaitDecision::Poll => thread::sleep(Duration::from_millis(20)),
        }
    }
}

pub(super) fn eis_snapshot() -> EisWaitSnapshot {
    EisWaitSnapshot {
        ready: portal_input_ready(),
        connecting: EIS_CONNECTING.load(Ordering::SeqCst),
        exhausted: EIS_WAIT_EXHAUSTED.load(Ordering::SeqCst),
        failure: EIS_FAILURE.lock().clone(),
        granted: super::grants::portal_remote_desktop_granted(),
    }
}

pub(super) struct EisWaitSnapshot {
    pub(super) ready: bool,
    pub(super) connecting: bool,
    pub(super) exhausted: bool,
    pub(super) failure: Option<String>,
    pub(super) granted: bool,
}

pub(super) enum EisWaitDecision {
    Ready,
    Poll,
    Failed(String),
}

pub(super) fn eis_wait_decision(snap: &EisWaitSnapshot) -> EisWaitDecision {
    if snap.ready {
        return EisWaitDecision::Ready;
    }
    if let Some(err) = &snap.failure {
        return EisWaitDecision::Failed(err.clone());
    }
    if snap.connecting && !snap.exhausted {
        return EisWaitDecision::Poll;
    }
    if snap.granted {
        EisWaitDecision::Failed("Remote Desktop granted but EIS is not ready".into())
    } else {
        EisWaitDecision::Failed(
            "desktop control not granted (enable Allow Remote Interaction, then Share)".into(),
        )
    }
}

pub(super) fn note_eis_failure(err: String) {
    cap_log("INPUT", "fail", &err);
    *EIS_FAILURE.lock() = Some(err);
    EIS_CONNECTING.store(false, Ordering::SeqCst);
}

pub(super) fn clear_eis_status() {
    EIS_CONNECTING.store(false, Ordering::SeqCst);
    EIS_WAIT_EXHAUSTED.store(false, Ordering::SeqCst);
    *EIS_FAILURE.lock() = None;
}

/// Compositor cursor from ScreenCast `CursorMode::Metadata`, in desktop pixels.
/// Works over native Wayland surfaces; XQueryPointer does not.
pub fn portal_cursor_position() -> Option<(i32, i32)> {
    *PORTAL_CURSOR.lock()
}

pub fn portal_input_move(x: i32, y: i32) -> Result<(), AutomationError> {
    let (reply, rx) = mpsc::channel();
    send_eis(EisCmd::Move { x, y, reply })?;
    recv_eis(rx)
}

/// Last absolute pointer sent over EIS this session, if any.
pub fn portal_input_last_pos() -> Option<(i32, i32)> {
    *LAST_ABS.lock()
}

pub fn portal_input_click(button: &str, down: bool) -> Result<(), AutomationError> {
    let code = match button {
        "right" => BTN_RIGHT,
        "center" | "middle" => BTN_MIDDLE,
        _ => BTN_LEFT,
    };
    let (reseat, reseat_src) = click_reseat();
    let pos = reseat
        .map(|(x, y)| format!("{x},{y}"))
        .unwrap_or_else(|| "none".into());
    let (reply, rx) = mpsc::channel();
    send_eis(EisCmd::Click {
        button: code,
        down,
        reseat,
        reply,
    })?;
    let r = recv_eis(rx);
    let edge = if down { "down" } else { "up" };
    match &r {
        Ok(()) => {
            note_stage_mutation();
            cap_log(
                "INPUT",
                "ok",
                &format!(
                    "click={edge} button={button} pos={pos} reseat={} src={reseat_src}",
                    if reseat.is_some() { "yes" } else { "no" }
                ),
            );
        }
        Err(e) => {
            cap_log(
                "INPUT",
                "fail",
                &format!("click={edge} button={button} pos={pos} err={e}"),
            );
        }
    }
    r
}

pub fn portal_input_scroll(up: bool) -> Result<(), AutomationError> {
    let (reply, rx) = mpsc::channel();
    send_eis(EisCmd::Scroll { up, reply })?;
    let r = recv_eis(rx);
    if r.is_ok() {
        note_stage_mutation();
    }
    r
}

pub fn portal_input_key(evdev: u32, down: bool) -> Result<(), AutomationError> {
    let (reply, rx) = mpsc::channel();
    send_eis(EisCmd::Key { evdev, down, reply })?;
    let r = recv_eis(rx);
    if r.is_ok() {
        note_stage_mutation();
    }
    r
}

pub(super) fn send_eis(cmd: EisCmd) -> Result<(), AutomationError> {
    let tx = INPUT_TX.lock().clone().ok_or_else(|| {
        AutomationError::Backend(
            "desktop control not granted (enable Allow Remote Interaction, then Share)".into(),
        )
    })?;
    tx.send(cmd)
        .map_err(|_| AutomationError::Backend("portal input thread exited".into()))
}

pub(super) fn recv_eis(rx: Receiver<Result<(), AutomationError>>) -> Result<(), AutomationError> {
    rx.recv()
        .map_err(|_| AutomationError::Backend("portal input reply dropped".into()))?
}

pub(super) fn dispatch_eis(eis: &mut EisInput, cmd: EisCmd) {
    match cmd {
        EisCmd::Move { x, y, reply } => {
            let result = eis.move_to(x, y);
            if result.is_ok() {
                *LAST_ABS.lock() = Some((x, y));
                *LAST_ABS_AT.lock() = Some(Instant::now());
            }
            let _ = reply.send(result);
        }
        EisCmd::Click {
            button,
            down,
            reseat,
            reply,
        } => {
            let _ = reply.send(eis.click(button, down, reseat));
        }
        EisCmd::Scroll { up, reply } => {
            let _ = reply.send(eis.scroll(up));
        }
        EisCmd::Key { evdev, down, reply } => {
            let _ = reply.send(eis.key(evdev, down));
        }
    }
}

pub(super) fn publish_eis_ready(eis: &EisInput) {
    if EIS_SHUTDOWN.load(Ordering::SeqCst) || !eis.has_usable_device() {
        return;
    }
    if EIS_READY.swap(true, Ordering::SeqCst) {
        return;
    }
    EIS_CONNECTING.store(false, Ordering::SeqCst);
    EIS_WAIT_EXHAUSTED.store(false, Ordering::SeqCst);
    *EIS_FAILURE.lock() = None;
    cap_log("INPUT", "ok", "backend=eis");
}

/// Where a click should re-frame the pointer.
///
/// A bare click uses the live cursor. The last Move is used only while
/// ScreenCast metadata can still be behind that warp, so Move-then-Click
/// still lands on the target. A stale warp is never a click target: without
/// live metadata the button applies at the seat pointer, which is the real
/// cursor — warping to an old Move would click the wrong spot.
pub(super) fn click_reseat() -> (Option<(i32, i32)>, &'static str) {
    let sent = *LAST_ABS.lock();
    let warp_fresh = LAST_ABS_AT
        .lock()
        .is_some_and(|t| t.elapsed() <= RESEAT_TRUST_WARP);
    let (pos, src) = reseat_target(sent, portal_cursor_position(), warp_fresh);
    if let Some(pos) = pos {
        if sent != Some(pos) {
            // Button-up must not warp back to an older Move. Do not refresh
            // LAST_ABS_AT: that window is only for a warp ScreenCast has not
            // echoed yet.
            *LAST_ABS.lock() = Some(pos);
        }
    }
    (pos, src)
}

pub(super) fn reseat_target(
    sent: Option<(i32, i32)>,
    live: Option<(i32, i32)>,
    warp_fresh: bool,
) -> (Option<(i32, i32)>, &'static str) {
    match (sent, live) {
        (Some(sent), Some(live)) if sent != live && warp_fresh => (Some(sent), "warp"),
        (_, Some(live)) => (Some(live), "cursor"),
        (Some(sent), None) if warp_fresh => (Some(sent), "warp"),
        _ => (None, "none"),
    }
}
