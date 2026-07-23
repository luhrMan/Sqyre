//! Full data-directory backup / restore as zip archives under `{sqyre_dir}/backups/`.
//!
//! Native only — WASM has no filesystem to archive.

#![cfg(not(target_arch = "wasm32"))]

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use thiserror::Error;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::sqyre_dir;

const BACKUPS_SUBDIR: &str = "backups";
const BACKUP_PREFIX: &str = "sqyre-backup-";
const BACKUP_SUFFIX: &str = ".zip";

/// Files / dirs under the data directory that are never included in a backup.
const SKIP_NAMES: &[&str] = &[
    BACKUPS_SUBDIR,
    "sqyre.lock",
    "crash.log",
    "diag.log",
    "last_site.txt",
];

#[derive(Debug, Error)]
pub enum BackupError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, BackupError>;

/// `{sqyre_dir}/backups`.
pub fn backups_dir() -> PathBuf {
    sqyre_dir().join(BACKUPS_SUBDIR)
}

/// Whether `name` looks like a Sqyre-managed backup archive.
fn is_backup_filename(name: &str) -> bool {
    name.starts_with(BACKUP_PREFIX) && name.ends_with(BACKUP_SUFFIX)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Format `YYYYMMDD-HHMMSS` in UTC from a unix timestamp (no external time crate).
fn format_timestamp(secs: u64) -> String {
    // Civil calendar from unix days (Howard Hinnant algorithm).
    let days = (secs / 86_400) as i64;
    let tod = secs % 86_400;
    let hour = tod / 3600;
    let min = (tod % 3600) / 60;
    let sec = tod % 60;

    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}{m:02}{d:02}-{hour:02}{min:02}{sec:02}")
}

fn should_skip_entry(name: &str) -> bool {
    SKIP_NAMES.contains(&name)
}

/// Walk `root`, collecting relative file paths to include in the archive.
fn collect_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            // Only skip top-level names under the data dir.
            if dir == root && should_skip_entry(&name) {
                continue;
            }
            let meta = entry.metadata()?;
            if meta.is_dir() {
                stack.push(path);
            } else if meta.is_file() {
                let rel = path
                    .strip_prefix(root)
                    .map_err(|e| BackupError::Message(e.to_string()))?
                    .to_path_buf();
                out.push(rel);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Create a zip of the data directory; returns the path of the new archive.
///
/// Skips `backups/`, lock, and diagnostic logs. Builds via temp file + rename.
pub fn create_backup() -> Result<PathBuf> {
    let root = sqyre_dir();
    if !root.exists() {
        return Err(BackupError::Message(format!(
            "data directory does not exist: {}",
            root.display()
        )));
    }

    let dest_dir = backups_dir();
    fs::create_dir_all(&dest_dir)?;

    let stamp = format_timestamp(unix_now());
    let final_name = format!("{BACKUP_PREFIX}{stamp}{BACKUP_SUFFIX}");
    let final_path = dest_dir.join(&final_name);
    let tmp_path = dest_dir.join(format!("{final_name}.tmp"));

    let files = collect_files(&root)?;
    let write = || -> Result<()> {
        let file = File::create(&tmp_path)?;
        let mut zip = ZipWriter::new(file);
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        let mut buf = Vec::new();
        for rel in &files {
            let abs = root.join(rel);
            let mut src = File::open(&abs)?;
            buf.clear();
            src.read_to_end(&mut buf)?;
            // Zip paths use forward slashes.
            let name = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            zip.start_file(name, opts)?;
            zip.write_all(&buf)?;
        }
        zip.finish()?;
        Ok(())
    };

    if let Err(e) = write() {
        let _ = fs::remove_file(&tmp_path);
        return Err(e);
    }
    if let Err(e) = fs::rename(&tmp_path, &final_path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(e.into());
    }
    Ok(final_path)
}

/// List managed backup archives, newest first (by filename).
pub fn list_backups() -> Result<Vec<PathBuf>> {
    let dir = backups_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if is_backup_filename(name) {
            paths.push(path);
        }
    }
    paths.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
    Ok(paths)
}

/// Delete oldest managed backups beyond `max_keep`.
pub fn prune_backups(max_keep: usize) -> Result<()> {
    if max_keep == 0 {
        return Ok(());
    }
    let paths = list_backups()?;
    if paths.len() <= max_keep {
        return Ok(());
    }
    for path in paths.into_iter().skip(max_keep) {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// Reject zip entries that would escape the destination via `..` or absolute paths.
fn safe_extract_path(dest: &Path, name: &str) -> Result<PathBuf> {
    let rel = Path::new(name);
    if rel.is_absolute() {
        return Err(BackupError::Message(format!(
            "backup entry has absolute path: {name}"
        )));
    }
    for c in rel.components() {
        match c {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(BackupError::Message(format!(
                    "backup entry has unsafe path: {name}"
                )));
            }
        }
    }
    Ok(dest.join(rel))
}

/// Extract a backup zip into the current data directory.
pub fn restore_backup(zip_path: &Path) -> Result<()> {
    if !zip_path.is_file() {
        return Err(BackupError::Message(format!(
            "backup file not found: {}",
            zip_path.display()
        )));
    }
    let dest = sqyre_dir();
    fs::create_dir_all(&dest)?;

    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        // Skip directory-only entries (trailing slash).
        if name.ends_with('/') {
            let dir = safe_extract_path(&dest, name.trim_end_matches('/'))?;
            fs::create_dir_all(dir)?;
            continue;
        }
        let out_path = safe_extract_path(&dest, &name)?;
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = File::create(&out_path)?;
        io::copy(&mut entry, &mut out)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_timestamp_epoch() {
        assert_eq!(format_timestamp(0), "19700101-000000");
        // 2024-03-15 12:34:56 UTC
        assert_eq!(format_timestamp(1_710_506_096), "20240315-123456");
    }

    #[test]
    fn create_list_prune_restore_roundtrip() -> Result<()> {
        let tmp = tempfile::tempdir().map_err(BackupError::from)?;
        let data = tmp.path().join(".sqyre");
        fs::create_dir_all(data.join("images/icons"))?;
        fs::write(data.join("db.yaml"), "macros: {}\nprograms: {}\n")?;
        fs::write(data.join("settings.yaml"), "backup_enabled: false\n")?;
        fs::write(data.join("images/icons/a.png"), b"png")?;
        fs::write(data.join("crash.log"), "ignore")?;

        crate::with_sqyre_dir_override(data.clone(), || -> Result<()> {
            let path = create_backup()?;
            assert!(path.exists());
            assert!(is_backup_filename(
                path.file_name().and_then(|n| n.to_str()).unwrap()
            ));

            // Extra managed archives to exercise prune (filenames sort newest-first).
            let backups = backups_dir();
            fs::write(backups.join("sqyre-backup-20000101-000000.zip"), b"old")?;
            fs::write(backups.join("sqyre-backup-20990101-000000.zip"), b"new")?;
            fs::write(backups.join("notes.txt"), b"keep")?;
            prune_backups(2)?;
            let listed = list_backups()?;
            assert_eq!(listed.len(), 2);
            assert!(backups.join("notes.txt").exists());
            assert!(!backups.join("sqyre-backup-20000101-000000.zip").exists());

            // Wipe user data and restore the real archive.
            fs::remove_file(data.join("db.yaml"))?;
            fs::remove_file(data.join("images/icons/a.png"))?;
            restore_backup(&path)?;
            assert_eq!(
                fs::read_to_string(data.join("db.yaml")).unwrap(),
                "macros: {}\nprograms: {}\n"
            );
            assert_eq!(fs::read(data.join("images/icons/a.png")).unwrap(), b"png");
            assert!(!ZipArchive::new(File::open(&path)?)?
                .file_names()
                .any(|n| n == "crash.log"));
            Ok(())
        })
    }

    #[test]
    fn safe_extract_rejects_traversal() {
        let dest = Path::new("/tmp/dest");
        assert!(safe_extract_path(dest, "../etc/passwd").is_err());
        assert!(safe_extract_path(dest, "/etc/passwd").is_err());
        assert!(safe_extract_path(dest, "images/icons/a.png").is_ok());
    }
}
