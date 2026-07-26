//! Full data-directory backup / restore as zip archives under `{sqyre_dir}/backups/`.
//!
//! Native only — WASM has no filesystem to archive.

#![cfg(not(target_arch = "wasm32"))]

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;

use thiserror::Error;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::{sqyre_dir, Database};

const BACKUPS_SUBDIR: &str = "backups";
const BACKUP_PREFIX: &str = "sqyre-backup-";
const BACKUP_SUFFIX: &str = ".zip";

/// Max on-disk size of a backup `.zip` before restore refuses to open it.
const MAX_ARCHIVE_FILE_BYTES: usize = 256 * 1024 * 1024;
/// Max number of zip entries (files + directory markers) in one archive.
const MAX_ARCHIVE_ENTRIES: usize = 10_000;
/// Max total uncompressed payload when creating or extracting a backup.
const MAX_EXPANDED_BYTES: usize = 512 * 1024 * 1024;

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

/// Serializes backup create/restore so concurrent callers cannot interleave
/// writes to the same scratch and archive directories.
static BACKUP_OPS_LOCK: Mutex<()> = Mutex::new(());

/// Unique suffix for restore scratch directories (`pid` + subsecond time).
fn unique_scratch_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}-{nanos}", process::id())
}

fn reject_if_archive_too_large(bytes: u64) -> Result<()> {
    if bytes as usize > MAX_ARCHIVE_FILE_BYTES {
        return Err(BackupError::Message(format!(
            "backup archive too large ({} bytes; max {MAX_ARCHIVE_FILE_BYTES})",
            bytes
        )));
    }
    Ok(())
}

fn reject_if_too_many_entries(count: usize) -> Result<()> {
    if count > MAX_ARCHIVE_ENTRIES {
        return Err(BackupError::Message(format!(
            "backup has too many entries ({count}; max {MAX_ARCHIVE_ENTRIES})"
        )));
    }
    Ok(())
}

fn reject_if_expanded_too_large(total: usize) -> Result<()> {
    if total > MAX_EXPANDED_BYTES {
        return Err(BackupError::Message(format!(
            "backup expanded size exceeds limit ({total} bytes; max {MAX_EXPANDED_BYTES})"
        )));
    }
    Ok(())
}

fn copy_with_expanded_limit<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    expanded_total: &mut usize,
) -> Result<()> {
    let mut buf = [0u8; 8192];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        *expanded_total = expanded_total.saturating_add(n);
        reject_if_expanded_too_large(*expanded_total)?;
        writer.write_all(&buf[..n])?;
    }
    Ok(())
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
            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file() {
                let rel = path
                    .strip_prefix(root)
                    .map_err(|e| BackupError::Message(e.to_string()))?
                    .to_path_buf();
                out.push(rel);
            }
        }
    }
    out.sort();
    reject_if_too_many_entries(out.len())?;
    let mut expanded_total = 0usize;
    for rel in &out {
        let len = fs::metadata(root.join(rel))?.len() as usize;
        expanded_total = expanded_total.saturating_add(len);
        reject_if_expanded_too_large(expanded_total)?;
    }
    Ok(out)
}

/// Create a zip of the data directory; returns the path of the new archive.
///
/// Skips `backups/`, lock, and diagnostic logs. Builds via temp file + rename.
pub fn create_backup() -> Result<PathBuf> {
    let _guard = BACKUP_OPS_LOCK.lock();
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
        let mut expanded_total = 0usize;
        for rel in &files {
            let abs = root.join(rel);
            let mut src = File::open(&abs)?;
            buf.clear();
            src.read_to_end(&mut buf)?;
            expanded_total = expanded_total.saturating_add(buf.len());
            reject_if_expanded_too_large(expanded_total)?;
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
///
/// Staging + snapshot commit: the zip is fully extracted to a temporary staging
/// directory first; on success, the live data tree (except `backups/` and restore
/// scratch dirs) is moved aside and replaced. On any failure before commit, the
/// live directory is left unchanged. After a successful commit the previous
/// snapshot is deleted.
pub fn restore_backup(zip_path: &Path) -> Result<()> {
    let _guard = BACKUP_OPS_LOCK.lock();
    if !zip_path.is_file() {
        return Err(BackupError::Message(format!(
            "backup file not found: {}",
            zip_path.display()
        )));
    }
    reject_if_archive_too_large(fs::metadata(zip_path)?.len())?;
    let dest = sqyre_dir();
    fs::create_dir_all(&dest)?;

    let tag = unique_scratch_suffix();
    let staging = dest.join(format!(".restore-staging-{tag}"));
    let prev = dest.join(format!(".restore-prev-{tag}"));
    // Clean leftovers from interrupted runs.
    let _ = fs::remove_dir_all(&staging);
    let _ = fs::remove_dir_all(&prev);

    let extract = || -> Result<()> {
        fs::create_dir_all(&staging)?;
        let file = File::open(zip_path)?;
        let mut archive = ZipArchive::new(file)?;
        reject_if_too_many_entries(archive.len())?;
        let mut expanded_total = 0usize;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let name = entry.name().to_string();
            if name.ends_with('/') {
                let dir = safe_extract_path(&staging, name.trim_end_matches('/'))?;
                fs::create_dir_all(dir)?;
                continue;
            }
            let out_path = safe_extract_path(&staging, &name)?;
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out = File::create(&out_path)?;
            copy_with_expanded_limit(&mut entry, &mut out, &mut expanded_total)?;
        }
        let db_path = staging.join("db.yaml");
        if !db_path.is_file() {
            return Err(BackupError::Message(
                "backup is missing db.yaml (refusing restore)".into(),
            ));
        }
        let db_text = fs::read_to_string(&db_path)?;
        Database::from_yaml_with_warnings(&db_text)
            .map_err(|e| BackupError::Message(format!("restored db.yaml is invalid: {e}")))?;
        Ok(())
    };

    if let Err(e) = extract() {
        let _ = fs::remove_dir_all(&staging);
        return Err(e);
    }

    // Move live entries aside (except backups/ and restore scratch).
    fs::create_dir_all(&prev)?;
    for entry in fs::read_dir(&dest)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str == BACKUPS_SUBDIR
            || name_str.starts_with(".restore-staging-")
            || name_str.starts_with(".restore-prev-")
        {
            continue;
        }
        let from = entry.path();
        let to = prev.join(&name);
        fs::rename(&from, &to).map_err(|e| {
            // Best-effort rollback of partial move.
            let _ = rollback_prev(&prev, &dest);
            let _ = fs::remove_dir_all(&staging);
            BackupError::Message(format!(
                "failed to snapshot {} before restore: {e}",
                from.display()
            ))
        })?;
    }

    // Commit staging into live dir.
    for entry in fs::read_dir(&staging)? {
        let entry = entry?;
        let name = entry.file_name();
        let from = entry.path();
        let to = dest.join(&name);
        if let Err(e) = fs::rename(&from, &to) {
            let _ = rollback_prev(&prev, &dest);
            let _ = fs::remove_dir_all(&staging);
            return Err(BackupError::Message(format!(
                "failed to commit restore entry {}: {e}",
                name.to_string_lossy()
            )));
        }
    }

    let _ = fs::remove_dir_all(&staging);
    let _ = fs::remove_dir_all(&prev);
    Ok(())
}

fn rollback_prev(prev: &Path, dest: &Path) -> Result<()> {
    if !prev.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(prev)? {
        let entry = entry?;
        let name = entry.file_name();
        let from = entry.path();
        let to = dest.join(&name);
        if to.exists() {
            if to.is_dir() {
                let _ = fs::remove_dir_all(&to);
            } else {
                let _ = fs::remove_file(&to);
            }
        }
        fs::rename(&from, &to)?;
    }
    let _ = fs::remove_dir_all(prev);
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
    fn restore_rejects_archive_without_db_yaml_and_keeps_live_data() -> Result<()> {
        let tmp = tempfile::tempdir().map_err(BackupError::from)?;
        let data = tmp.path().join(".sqyre");
        fs::create_dir_all(&data)?;
        fs::write(data.join("db.yaml"), "macros: {}\nprograms: {}\n")?;
        let original = fs::read_to_string(data.join("db.yaml")).unwrap();

        let bad_zip = tmp.path().join("bad.zip");
        {
            let file = File::create(&bad_zip)?;
            let mut zip = ZipWriter::new(file);
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
            zip.start_file("settings.yaml", opts)?;
            zip.write_all(b"backup_enabled: false\n")?;
            zip.finish()?;
        }

        crate::with_sqyre_dir_override(data.clone(), || -> Result<()> {
            let err = restore_backup(&bad_zip).unwrap_err();
            assert!(err.to_string().contains("missing db.yaml"), "got {err}");
            assert_eq!(fs::read_to_string(data.join("db.yaml")).unwrap(), original);
            Ok(())
        })?;
        Ok(())
    }

    #[test]
    fn safe_extract_rejects_traversal() {
        let dest = Path::new("/tmp/dest");
        assert!(safe_extract_path(dest, "../etc/passwd").is_err());
        assert!(safe_extract_path(dest, "/etc/passwd").is_err());
        assert!(safe_extract_path(dest, "images/icons/a.png").is_ok());
    }

    #[test]
    fn restore_limits_reject_oversized_archive_and_too_many_entries() {
        assert!(reject_if_archive_too_large(MAX_ARCHIVE_FILE_BYTES as u64).is_ok());
        assert!(reject_if_archive_too_large(MAX_ARCHIVE_FILE_BYTES as u64 + 1).is_err());

        assert!(reject_if_too_many_entries(MAX_ARCHIVE_ENTRIES).is_ok());
        assert!(reject_if_too_many_entries(MAX_ARCHIVE_ENTRIES + 1).is_err());

        assert!(reject_if_expanded_too_large(MAX_EXPANDED_BYTES).is_ok());
        assert!(reject_if_expanded_too_large(MAX_EXPANDED_BYTES + 1).is_err());
    }

    #[test]
    fn restore_rejects_oversized_archive_and_keeps_live_data() -> Result<()> {
        let tmp = tempfile::tempdir().map_err(BackupError::from)?;
        let data = tmp.path().join(".sqyre");
        fs::create_dir_all(&data)?;
        fs::write(data.join("db.yaml"), "macros: {}\nprograms: {}\n")?;
        let original = fs::read_to_string(data.join("db.yaml")).unwrap();

        let big_zip = tmp.path().join("big.zip");
        fs::write(&big_zip, vec![0u8; MAX_ARCHIVE_FILE_BYTES + 1])?;

        crate::with_sqyre_dir_override(data.clone(), || -> Result<()> {
            let err = restore_backup(&big_zip).unwrap_err();
            assert!(err.to_string().contains("too large"), "got {err}");
            assert_eq!(fs::read_to_string(data.join("db.yaml")).unwrap(), original);
            Ok(())
        })?;
        Ok(())
    }

    #[test]
    fn restore_rejects_invalid_db_yaml_and_keeps_live_data() -> Result<()> {
        let tmp = tempfile::tempdir().map_err(BackupError::from)?;
        let data = tmp.path().join(".sqyre");
        fs::create_dir_all(&data)?;
        fs::write(data.join("db.yaml"), "macros: {}\nprograms: {}\n")?;
        let original = fs::read_to_string(data.join("db.yaml")).unwrap();

        let bad_zip = tmp.path().join("bad-db.zip");
        {
            let file = File::create(&bad_zip)?;
            let mut zip = ZipWriter::new(file);
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
            zip.start_file("db.yaml", opts)?;
            zip.write_all(b"macros: [unterminated")?;
            zip.finish()?;
        }

        crate::with_sqyre_dir_override(data.clone(), || -> Result<()> {
            let err = restore_backup(&bad_zip).unwrap_err();
            assert!(err.to_string().contains("invalid"), "got {err}");
            assert_eq!(fs::read_to_string(data.join("db.yaml")).unwrap(), original);
            Ok(())
        })?;
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn create_backup_skips_symlink_files() -> Result<()> {
        use std::os::unix::fs::symlink;

        let tmp = tempfile::tempdir().map_err(BackupError::from)?;
        let data = tmp.path().join(".sqyre");
        fs::create_dir_all(data.join("images"))?;
        fs::write(data.join("db.yaml"), "macros: {}\nprograms: {}\n")?;
        fs::write(data.join("images/real.png"), b"png")?;
        symlink(data.join("images/real.png"), data.join("images/link.png"))?;

        crate::with_sqyre_dir_override(data.clone(), || -> Result<()> {
            let path = create_backup()?;
            let archive = ZipArchive::new(File::open(&path)?)?;
            let names: Vec<_> = archive.file_names().map(str::to_owned).collect();
            assert!(names.iter().any(|n| n == "images/real.png"));
            assert!(!names.iter().any(|n| n == "images/link.png"));
            Ok(())
        })?;
        Ok(())
    }
}
