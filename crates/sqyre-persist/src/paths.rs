//! Relocatable Sqyre data-directory paths and overrides.
//!
//! Leaf module so backup/settings/database need not import through `lib.rs`.

use crate::error::Result;
use parking_lot::{Mutex, RwLock};

use std::fs;
use std::path::PathBuf;

const SQYRE_DIR: &str = sqyre_ports::APP_DIR_NAME;
/// `db.yaml` database file name within [`sqyre_dir`].
pub const DB_FILE: &str = "db.yaml";
/// `settings.yaml` file name within [`sqyre_dir`].
pub const SETTINGS_FILE: &str = "settings.yaml";
/// Reject absurdly large `db.yaml` bodies before parsing.
pub const MAX_DB_YAML_BYTES: usize = 32 * 1024 * 1024;
/// Soft cap on macro count in one database.
pub const MAX_MACROS: usize = 2_000;

static DIR_OVERRIDE: RwLock<Option<PathBuf>> = RwLock::new(None);
static BACKUP_DIR_OVERRIDE: RwLock<Option<PathBuf>> = RwLock::new(None);

static HOME_DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// Serializes tests that mutate [`set_sqyre_dir_override`].
static DIR_OVERRIDE_TEST_LOCK: Mutex<()> = Mutex::new(());

/// Override the Sqyre data directory (empty clears → `~/.sqyre`).
pub fn set_sqyre_dir_override(path: Option<PathBuf>) {
    *DIR_OVERRIDE.write() = path;
}

/// Override the backups directory (`None` → `{sqyre_dir}/backups`).
pub fn set_backup_dir_override(path: Option<PathBuf>) {
    *BACKUP_DIR_OVERRIDE.write() = path;
}

/// Active backups-directory override, if any.
pub fn backup_dir_override() -> Option<PathBuf> {
    BACKUP_DIR_OVERRIDE.read().clone()
}

/// Replace the user home for Sqyre paths (Android app-private storage has no `$HOME`).
///
/// Call once at startup, before any path lookup.
pub fn set_home_dir(path: PathBuf) {
    // A recreated Android activity re-enters startup in the same process with the same path.
    let _ = HOME_DIR.set(path);
}

fn home_dir() -> PathBuf {
    HOME_DIR
        .get()
        .cloned()
        .or_else(dirs::home_dir)
        .unwrap_or_else(std::env::temp_dir)
}

/// Per-user config directory for Sqyre (`~/.config/sqyre` on Linux).
///
/// Holds the data-dir pointer and the single-instance lock — paths that must
/// not move when the relocatable data directory changes.
pub fn sqyre_config_dir() -> PathBuf {
    let base = match HOME_DIR.get() {
        Some(home) => home.join(".config"),
        None => dirs::config_dir().unwrap_or_else(|| home_dir().join(".config")),
    };
    base.join(sqyre_ports::APP_ID)
}

/// Run `f` with a temporary Sqyre dir override, serialized against other override users.
pub fn with_sqyre_dir_override<R>(path: PathBuf, f: impl FnOnce() -> R) -> R {
    with_sqyre_dirs_override(path, None, f)
}

/// Run `f` with temporary data + optional backup dir overrides under
/// [`DIR_OVERRIDE_TEST_LOCK`]. Clears both overrides when `f` returns.
pub fn with_sqyre_dirs_override<R>(
    data: PathBuf,
    backup: Option<PathBuf>,
    f: impl FnOnce() -> R,
) -> R {
    let _guard = DIR_OVERRIDE_TEST_LOCK.lock();
    set_sqyre_dir_override(Some(data));
    set_backup_dir_override(backup);
    let result = f();
    set_backup_dir_override(None);
    set_sqyre_dir_override(None);
    result
}

pub fn sqyre_dir() -> PathBuf {
    if let Some(p) = DIR_OVERRIDE.read().clone() {
        return p;
    }

    {
        home_dir().join(SQYRE_DIR)
    }
}

pub fn db_path() -> PathBuf {
    sqyre_dir().join(DB_FILE)
}

pub fn variables_path() -> PathBuf {
    sqyre_dir().join("variables")
}

pub fn images_path() -> PathBuf {
    sqyre_dir().join("images")
}

pub fn screen_cap_path() -> PathBuf {
    images_path().join("ScreenCap")
}

pub fn screen_cap_trash_path() -> PathBuf {
    screen_cap_path().join("trash")
}

pub fn initialize_directories() -> Result<()> {
    {
        for p in [
            sqyre_dir().join("images/icons"),
            sqyre_dir().join("images/process"),
            sqyre_dir().join("images/ScreenCap"),
            screen_cap_trash_path(),
            sqyre_dir().join("images/Collections"),
            sqyre_dir().join("images/masks"),
            variables_path(),
        ] {
            fs::create_dir_all(&p)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn dirs_override_sets_data_and_backup_then_clears() {
        let data = tempdir().unwrap();
        let backup = tempdir().unwrap();
        with_sqyre_dirs_override(
            data.path().to_path_buf(),
            Some(backup.path().to_path_buf()),
            || {
                assert_eq!(sqyre_dir(), data.path());
                assert_eq!(backup_dir_override().as_deref(), Some(backup.path()));
            },
        );
        assert!(backup_dir_override().is_none());
        assert_ne!(sqyre_dir(), data.path());
    }
}
