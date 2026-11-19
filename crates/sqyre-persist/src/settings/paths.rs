//! Data-directory pointer and file-manager helpers for settings.

use std::fs;
use std::path::{Path, PathBuf};

use crate::{PersistError, Result};

/// Settings YAML path under the active Sqyre data directory.
pub fn settings_path() -> PathBuf {
    crate::sqyre_dir().join(crate::SETTINGS_FILE)
}

/// XDG pointer that records a relocated data directory (one path per line).
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn data_dir_pointer_path() -> PathBuf {
    crate::sqyre_config_dir().join("data_dir")
}

/// Apply a relocated data-dir pointer before loading settings from `.sqyre`.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn apply_data_dir_pointer() {
    let path = data_dir_pointer_path();
    let Ok(text) = fs::read_to_string(&path) else {
        return;
    };
    let dir = text.lines().next().unwrap_or("").trim();
    if dir.is_empty() {
        let _ = fs::remove_file(&path);
        crate::set_sqyre_dir_override(None);
        return;
    }
    crate::set_sqyre_dir_override(Some(PathBuf::from(dir)));
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn write_data_dir_pointer(sqyre_dir: &str) -> Result<()> {
    let path = data_dir_pointer_path();
    let dir = sqyre_dir.trim();
    if dir.is_empty() {
        if path.exists() {
            fs::remove_file(&path)?;
        }
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    crate::atomic_write(&path, format!("{dir}\n"))?;
    Ok(())
}

/// Open `path` in the platform file manager.
pub fn open_path_in_file_manager(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    if path.as_os_str().is_empty() {
        return Err(PersistError::Message("open folder: empty path".into()));
    }
    if !path.exists() {
        return Err(PersistError::Message(format!(
            "open folder {}: path does not exist",
            path.display()
        )));
    }
    let status = {
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("explorer").arg(path).status()
        }
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open").arg(path).status()
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            std::process::Command::new("xdg-open").arg(path).status()
        }
    };
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(PersistError::Message(format!(
            "open folder {}: exited {}",
            path.display(),
            s
        ))),
        Err(e) => Err(PersistError::Io(e)),
    }
}

/// Ensure the Sqyre data directory exists and open it in the file manager.
pub fn open_sqyre_dir() -> Result<()> {
    crate::initialize_directories()?;
    open_path_in_file_manager(crate::sqyre_dir())
}

/// Move `src` to `dst`. Falls back to recursive copy on cross-device rename.
/// `dst` must not already exist.
pub fn move_dir(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    if src == dst {
        return Ok(());
    }
    if dst.exists() {
        return Err(PersistError::Message(format!(
            "move {} to {}: destination already exists",
            src.display(),
            dst.display()
        )));
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(e) if is_cross_device(&e) => {
            copy_tree(src, dst)?;
            fs::remove_dir_all(src)?;
            Ok(())
        }
        Err(e) => Err(PersistError::Io(e)),
    }
}

fn is_cross_device(err: &std::io::Error) -> bool {
    // Linux EXDEV = 18; Windows ERROR_NOT_SAME_DEVICE = 17.
    matches!(err.raw_os_error(), Some(17) | Some(18))
}

fn copy_tree(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let ft = entry.file_type()?;
        if ft.is_dir() {
            copy_tree(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn move_dir_relocates_nested_tree() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("from");
        let dst = dir.path().join("to");
        fs::create_dir_all(src.join("nested")).unwrap();
        fs::write(src.join("nested/file.txt"), b"ok").unwrap();
        move_dir(&src, &dst).unwrap();
        assert!(!src.exists());
        assert_eq!(
            fs::read_to_string(dst.join("nested/file.txt")).unwrap(),
            "ok"
        );
    }

    #[test]
    fn move_dir_rejects_existing_destination() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("from");
        let dst = dir.path().join("to");
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&dst).unwrap();
        assert!(move_dir(&src, &dst).is_err());
    }

    #[test]
    fn settings_path_joins_settings_file() {
        let path = settings_path();
        assert!(path.ends_with(crate::SETTINGS_FILE));
    }
}
