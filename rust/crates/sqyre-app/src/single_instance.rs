//! Exclusive process lock (matches Go `internal/app` `sqyre.lock`).

use fs2::FileExt;
use std::fs::{self, File};
use std::io;
use std::path::PathBuf;

/// Holds the open lock file for the process lifetime.
/// Dropping (or exiting) releases the advisory lock.
pub struct InstanceLock {
    _file: File,
}

fn lock_path() -> PathBuf {
    sqyre_persist::sqyre_dir().join("sqyre.lock")
}

fn is_lock_contention(err: &io::Error) -> bool {
    match err.kind() {
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut => true,
        // Windows LockFileEx fail-immediate → ERROR_LOCK_VIOLATION (33)
        _ => err.raw_os_error() == Some(33),
    }
}

/// Try to become the sole Sqyre instance. Returns `Ok(None)` if another
/// instance already holds the lock.
pub fn try_acquire() -> io::Result<Option<InstanceLock>> {
    let path = lock_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = File::create(&path)?;
    match file.try_lock_exclusive() {
        Ok(()) => Ok(Some(InstanceLock { _file: file })),
        Err(e) if is_lock_contention(&e) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Drop `previous` then lock under the current [`sqyre_persist::sqyre_dir`].
/// Used after a settings data-location change so the lock follows the active tree.
pub fn reacquire(previous: Option<InstanceLock>) -> io::Result<Option<InstanceLock>> {
    drop(previous);
    try_acquire()
}
