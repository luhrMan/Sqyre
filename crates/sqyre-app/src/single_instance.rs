//! Exclusive process lock so only one native Sqyre can run per user.
//!
//! Lock path is fixed under the XDG config dir (not the relocatable data dir),
//! so a second launch cannot slip through after `sqyre_dir` moves.
//! WASM has no process lock.

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use fs2::FileExt;
    use std::fs::{self, File};
    use std::io;

    /// Holds the open lock file for the process lifetime.
    pub struct InstanceLock {
        _file: File,
    }

    fn lock_path() -> std::path::PathBuf {
        sqyre_persist::sqyre_config_dir().join("sqyre.lock")
    }

    fn is_lock_contention(err: &io::Error) -> bool {
        match err.kind() {
            io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut => true,
            _ => err.raw_os_error() == Some(33),
        }
    }

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
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::{try_acquire, InstanceLock};

#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::io;

    #[derive(Debug, Default)]
    pub struct InstanceLock;

    pub fn try_acquire() -> io::Result<Option<InstanceLock>> {
        Ok(Some(InstanceLock))
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm::{try_acquire, InstanceLock};
