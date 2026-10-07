//! File / folder pickers. [`request`] opens one; the app applies the result from
//! [`take_picked`] on a later frame, so every platform shares one completion path.
//!
//! Desktop: `rfd` blocks inside [`request`]. On Linux, `rfd` uses the XDG portal
//! (`ashpd` / `zbus`) and blocks with `pollster`. Keep `ksni` (and anything else
//! using zbus) on the `async-io` backend so nothing enables `zbus`'s `tokio`
//! feature — otherwise sync portal calls panic with "no reactor running".
//!
//! Android: the shell opens the Storage Access Framework picker and copies the
//! document into app cache. Blocking the egui thread would deadlock activity
//! lifecycle callbacks, so the answer arrives asynchronously. SAF folders are
//! `content://` trees that `std::fs` cannot open, so folder picks are desktop-only.
//!
//! WASM: sync `FileDialog` is unavailable and files have no paths; `wasm_io` handles
//! YAML import / export instead.

use parking_lot::Mutex;
use std::path::{Path, PathBuf};

/// What a pick is for; decides the filter and where the result goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    target_arch = "wasm32",
    expect(
        dead_code,
        reason = "backup / data-folder picks are only requested by desktop Settings"
    )
)]
pub(crate) enum PickPurpose {
    /// PNG for a new icon variant of the selected item.
    IconVariant,
    /// Raster image for the selected mask.
    MaskImage,
    /// Backup zip to restore.
    RestoreBackup,
    /// Folder for the `.sqyre` data directory.
    SqyreLocation,
    /// Flatpak: host `~/.sqyre` (or its parent) through the portal.
    HostSqyreLocation,
}

impl PickPurpose {
    fn kind(self) -> PickKind {
        match self {
            Self::IconVariant => PickKind::Png,
            Self::MaskImage => PickKind::Image,
            Self::RestoreBackup => PickKind::Zip,
            Self::SqyreLocation | Self::HostSqyreLocation => PickKind::Folder,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PickKind {
    Png,
    Image,
    Zip,
    Folder,
}

impl PickKind {
    #[cfg_attr(
        all(not(target_os = "android"), not(test)),
        expect(dead_code, reason = "MIME filters are only used by the Android picker")
    )]
    fn mime_types(self) -> &'static [&'static str] {
        match self {
            Self::Png => &["image/png"],
            Self::Image => &["image/png", "image/jpeg", "image/bmp"],
            Self::Zip => &["application/zip"],
            Self::Folder => &[],
        }
    }
}

/// A finished pick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Picked {
    pub(crate) purpose: PickPurpose,
    pub(crate) path: PathBuf,
}

#[derive(Debug, Default)]
struct PickState {
    /// Purpose of the pick still open (Android) or just answered (desktop).
    open: Option<PickPurpose>,
    /// Desktop answer waiting for [`take_picked`].
    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    done: Option<PathBuf>,
}

static PICK: Mutex<PickState> = Mutex::new(PickState {
    open: None,
    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    done: None,
});

/// Whether this build can pick files for `purpose` at all (hide the control otherwise).
pub(crate) fn supported(purpose: PickPurpose) -> bool {
    match purpose.kind() {
        PickKind::Folder => !cfg!(any(target_arch = "wasm32", target_os = "android")),
        PickKind::Png | PickKind::Image | PickKind::Zip => !cfg!(target_arch = "wasm32"),
    }
}

/// Open a picker. `start` is the initial directory where the platform honours one.
/// A new request replaces one that has not finished.
pub(crate) fn request(purpose: PickPurpose, title: &str, start: Option<&Path>) {
    if !supported(purpose) {
        return;
    }
    let mut state = PICK.lock();
    state.open = Some(purpose);
    platform::open(&mut state, purpose, title, start);
}

/// Whether a pick is still open; the app keeps repainting so it notices the answer.
pub(crate) fn pending() -> bool {
    platform::pending(&PICK.lock())
}

/// Take the finished pick, if any. Cancelled picks yield `None`.
pub(crate) fn take_picked() -> Option<Picked> {
    let mut state = PICK.lock();
    let path = platform::take(&mut state)?;
    let purpose = state.open.take()?;
    path.map(|path| Picked { purpose, path })
}

#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
mod platform {
    use super::{PickKind, PickPurpose, PickState};
    use std::path::{Path, PathBuf};

    pub(super) fn open(
        state: &mut PickState,
        purpose: PickPurpose,
        title: &str,
        start: Option<&Path>,
    ) {
        let mut dialog = rfd::FileDialog::new().set_title(title);
        if let Some(start) = start {
            dialog = dialog.set_directory(start);
        }
        let picked = match purpose.kind() {
            PickKind::Png => dialog.add_filter("PNG", &["png"]).pick_file(),
            PickKind::Image => dialog
                .add_filter("Images", &["png", "jpg", "jpeg", "bmp"])
                .pick_file(),
            PickKind::Zip => dialog.add_filter("Zip archive", &["zip"]).pick_file(),
            PickKind::Folder => dialog.pick_folder(),
        };
        state.done = picked;
        if state.done.is_none() {
            state.open = None;
        }
    }

    pub(super) fn pending(_state: &PickState) -> bool {
        false
    }

    pub(super) fn take(state: &mut PickState) -> Option<Option<PathBuf>> {
        state.done.take().map(Some)
    }
}

#[cfg(target_os = "android")]
mod platform {
    use super::{PickPurpose, PickState};
    use std::path::{Path, PathBuf};

    pub(super) fn open(
        state: &mut PickState,
        purpose: PickPurpose,
        _title: &str,
        _start: Option<&Path>,
    ) {
        let id = sqyre_android::picks().begin();
        if let Err(e) = sqyre_android::bridge::pick_document(id, purpose.kind().mime_types()) {
            state.open = None;
            crate::log::warn(format!("file picker: {e}"));
        }
    }

    pub(super) fn pending(_state: &PickState) -> bool {
        sqyre_android::picks().is_pending()
    }

    pub(super) fn take(_state: &mut PickState) -> Option<Option<PathBuf>> {
        sqyre_android::picks().take()
    }
}

#[cfg(target_arch = "wasm32")]
mod platform {
    use super::{PickPurpose, PickState};
    use std::path::{Path, PathBuf};

    pub(super) fn open(
        state: &mut PickState,
        _purpose: PickPurpose,
        _title: &str,
        _start: Option<&Path>,
    ) {
        state.open = None;
    }

    pub(super) fn pending(_state: &PickState) -> bool {
        false
    }

    pub(super) fn take(_state: &mut PickState) -> Option<Option<PathBuf>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn purposes_map_to_filters() {
        assert_eq!(PickPurpose::IconVariant.kind(), PickKind::Png);
        assert_eq!(PickPurpose::MaskImage.kind(), PickKind::Image);
        assert_eq!(PickPurpose::RestoreBackup.kind(), PickKind::Zip);
        assert_eq!(PickPurpose::SqyreLocation.kind(), PickKind::Folder);
        assert_eq!(PickPurpose::HostSqyreLocation.kind(), PickKind::Folder);
        assert_eq!(PickKind::Png.mime_types(), ["image/png"]);
        assert!(PickKind::Image.mime_types().contains(&"image/jpeg"));
        assert_eq!(PickKind::Zip.mime_types(), ["application/zip"]);
    }

    #[test]
    fn desktop_supports_every_purpose() {
        for purpose in [
            PickPurpose::IconVariant,
            PickPurpose::MaskImage,
            PickPurpose::RestoreBackup,
            PickPurpose::SqyreLocation,
            PickPurpose::HostSqyreLocation,
        ] {
            assert!(supported(purpose), "{purpose:?}");
        }
    }

    #[test]
    fn nothing_picked_without_a_request() {
        assert!(!pending());
        assert_eq!(take_picked(), None);
    }
}
