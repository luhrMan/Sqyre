//! Portal session grants helpers.

use super::*;

/// Gate the grant on at least one PipeWire stream, then mark the session granted.
pub(super) fn grant_screencast_streams(
    streams: &[ashpd::desktop::screencast::Stream],
) -> Result<(), CaptureError> {
    if streams.is_empty() {
        return Err(CaptureError::Message(
            "portal ScreenCast returned no streams".into(),
        ));
    }
    SCREENCAST_SESSION_GRANTED.store(true, Ordering::SeqCst);
    Ok(())
}

/// Portal Start succeeded this process, or a restore token is on disk from a prior grant.
pub fn portal_screencast_granted() -> bool {
    SCREENCAST_SESSION_GRANTED.load(Ordering::SeqCst)
        || super::tokens::read_restore_token_at(&super::tokens::restore_token_path()).is_some()
        || super::tokens::read_restore_token_at(&super::tokens::legacy_screencast_token_path())
            .is_some()
}

/// Remote Desktop devices were granted on the live combined portal session.
pub fn portal_remote_desktop_granted() -> bool {
    REMOTE_DESKTOP_GRANTED.load(Ordering::SeqCst)
}

/// Drop the current ScreenCast session and show the share picker again (ignores restore token).
pub fn request_portal_screencast_picker() {
    FORCE_SCREENCAST_PICKER.store(true, Ordering::SeqCst);
    forget_live_portal_grant();
    super::tokens::clear_restore_tokens();
    crate::linux::reset_shared_capturer();
    cap_log("PORTAL", "picker", "requested");
    let _ = thread::Builder::new()
        .name("sqyre-portal-picker".into())
        .spawn(|| {
            if let Err(e) = crate::linux::shared_capturer() {
                cap_log("PORTAL", "picker", &format!("reopen failed: {e}"));
            }
        });
}

/// Forget persistent ScreenCast / Remote Desktop grants and drop the live session.
///
/// Unlike [`request_portal_screencast_picker`], this does not reopen capture.
pub fn revoke_portal_grants() {
    let mut tokens = super::tokens::restore_tokens_on_disk();
    forget_live_portal_grant();
    super::tokens::clear_restore_tokens();
    crate::linux::reset_shared_capturer();
    // An in-flight `open()` may rewrite a token after the first clear; pick it up.
    forget_live_portal_grant();
    tokens.extend(super::tokens::restore_tokens_on_disk());
    tokens.sort();
    tokens.dedup();
    super::tokens::clear_restore_tokens();
    super::tokens::wipe_portal_permission_store(&tokens);
    forget_live_portal_grant();
    cap_log("PORTAL", "revoke", &format!("tokens={}", tokens.len()));
}

pub(super) fn forget_live_portal_grant() {
    SCREENCAST_SESSION_GRANTED.store(false, Ordering::SeqCst);
    stop_eis_thread();
}

pub(super) fn take_force_screencast_picker() -> bool {
    FORCE_SCREENCAST_PICKER.swap(false, Ordering::SeqCst)
}
