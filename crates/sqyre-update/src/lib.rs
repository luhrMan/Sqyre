//! Check GitHub Releases for a newer Sqyre build and self-replace the running binary.
//!
//! Supported install shapes: Linux raw binary, Linux AppImage (`$APPIMAGE`), Windows `.exe`.
//! macOS compiles but returns [`UpdateError::Unsupported`].

mod version;

use serde::Deserialize;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;
use version::{is_dev_sentinel, parse_release_version, version_newer};

const USER_AGENT: &str = "Sqyre-Updater";
const API_LATEST: &str = "https://api.github.com/repos/luhrMan/Squire/releases/latest";

/// Result of comparing the running build against the latest GitHub release.
#[derive(Debug, Clone)]
pub enum UpdateStatus {
    UpToDate,
    Available {
        version: String,
        asset: ReleaseAsset,
    },
}

/// A downloadable release asset selected for this install shape.
#[derive(Debug, Clone)]
pub struct ReleaseAsset {
    pub name: String,
    pub url: String,
    pub size: u64,
}

/// Extracted update file ready to replace the running install.
#[derive(Debug)]
pub struct StagedUpdate {
    /// Path to the extracted binary / AppImage on disk (temp).
    pub staged_path: PathBuf,
    /// Absolute path that will be replaced.
    pub target_path: PathBuf,
}

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("auto-update is not supported on this platform")]
    Unsupported,
    #[error("dev builds (version {0}) do not auto-update")]
    DevBuild(String),
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("failed to parse GitHub release JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("no matching release asset for this install")]
    NoMatchingAsset,
    #[error("invalid release version: {0}")]
    BadVersion(String),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("zip error: {0}")]
    Zip(String),
    #[error("could not locate current executable: {0}")]
    CurrentExe(String),
}

#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
    assets: Vec<GhAsset>,
}

#[derive(Debug, Deserialize)]
struct GhAsset {
    name: String,
    size: u64,
    browser_download_url: String,
}

/// Which install shape we are updating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // variants are selected per target OS; all used in tests
enum InstallKind {
    LinuxBinary,
    LinuxAppImage,
    WindowsExe,
}

impl InstallKind {
    fn detect() -> Result<Self, UpdateError> {
        #[cfg(target_os = "windows")]
        {
            Ok(Self::WindowsExe)
        }
        #[cfg(target_os = "linux")]
        {
            if std::env::var_os("APPIMAGE").is_some() {
                Ok(Self::LinuxAppImage)
            } else {
                Ok(Self::LinuxBinary)
            }
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            Err(UpdateError::Unsupported)
        }
    }

    fn asset_suffix(self) -> &'static str {
        match self {
            Self::LinuxBinary => "-linux-amd64.zip",
            Self::LinuxAppImage => ".AppImage.zip",
            Self::WindowsExe => "-windows-amd64.zip",
        }
    }

    fn pick_asset(self, assets: &[GhAsset]) -> Option<&GhAsset> {
        let suffix = self.asset_suffix();
        assets.iter().find(|a| a.name.ends_with(suffix))
    }
}

/// Absolute path of the file that should be replaced for this install.
fn update_target_path(kind: InstallKind) -> Result<PathBuf, UpdateError> {
    match kind {
        InstallKind::LinuxAppImage => {
            let path = std::env::var_os("APPIMAGE")
                .map(PathBuf::from)
                .ok_or_else(|| UpdateError::CurrentExe("APPIMAGE unset".into()))?;
            Ok(path)
        }
        InstallKind::LinuxBinary | InstallKind::WindowsExe => {
            std::env::current_exe().map_err(|e| UpdateError::CurrentExe(e.to_string()))
        }
    }
}

/// Query GitHub for the latest release and compare against `current` (embedded `SQYRE_VERSION`).
pub fn check_latest(current: &str) -> Result<UpdateStatus, UpdateError> {
    if is_dev_sentinel(current) {
        return Err(UpdateError::DevBuild(current.to_string()));
    }
    let kind = InstallKind::detect()?;
    let _ = update_target_path(kind)?;

    let current_v =
        parse_release_version(current).ok_or_else(|| UpdateError::BadVersion(current.into()))?;

    let body = http_get_string(API_LATEST)?;
    let release: GhRelease = serde_json::from_str(&body)?;
    let remote_v = parse_release_version(&release.tag_name)
        .ok_or_else(|| UpdateError::BadVersion(release.tag_name.clone()))?;

    if !version_newer(&remote_v, &current_v) {
        return Ok(UpdateStatus::UpToDate);
    }

    let asset = kind
        .pick_asset(&release.assets)
        .ok_or(UpdateError::NoMatchingAsset)?;

    Ok(UpdateStatus::Available {
        version: strip_v(&release.tag_name).to_string(),
        asset: ReleaseAsset {
            name: asset.name.clone(),
            url: asset.browser_download_url.clone(),
            size: asset.size,
        },
    })
}

/// Download `asset` zip, extract the single payload file to a durable temp path.
pub fn download_and_stage(asset: &ReleaseAsset) -> Result<StagedUpdate, UpdateError> {
    let kind = InstallKind::detect()?;
    let target_path = update_target_path(kind)?;

    let tmp_dir = tempfile::tempdir()?;
    let zip_path = tmp_dir.path().join(&asset.name);
    http_download(&asset.url, &zip_path)?;

    let extracted = extract_single_file(&zip_path, tmp_dir.path())?;
    let durable = std::env::temp_dir().join(format!(
        "sqyre-update-{}-{}",
        std::process::id(),
        extracted
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("payload")
    ));
    if durable.exists() {
        let _ = fs::remove_file(&durable);
    }
    if fs::rename(&extracted, &durable).is_err() {
        fs::copy(&extracted, &durable)?;
        let _ = fs::remove_file(&extracted);
    }
    drop(tmp_dir);

    Ok(StagedUpdate {
        staged_path: durable,
        target_path,
    })
}

/// Atomically replace the running install with `staged`.
pub fn apply(staged: StagedUpdate) -> Result<(), UpdateError> {
    #[cfg(target_os = "windows")]
    {
        apply_windows(&staged.staged_path, &staged.target_path)
    }
    #[cfg(unix)]
    {
        apply_unix(&staged.staged_path, &staged.target_path)
    }
    #[cfg(not(any(target_os = "windows", unix)))]
    {
        let _ = staged;
        Err(UpdateError::Unsupported)
    }
}

/// Remove leftover `*.old` next to the executable (Windows self-replace residue).
pub fn cleanup_stale_update() {
    #[cfg(target_os = "windows")]
    {
        if let Ok(exe) = std::env::current_exe() {
            let mut old_os = exe.as_os_str().to_owned();
            old_os.push(".old");
            let _ = fs::remove_file(PathBuf::from(old_os));
        }
    }
}

/// Re-exec (Unix) or spawn + exit (Windows) the current binary. Does not return on success.
///
/// Caller must drop the single-instance lock first.
pub fn restart() -> Result<(), UpdateError> {
    let exe = std::env::current_exe().map_err(|e| UpdateError::CurrentExe(e.to_string()))?;
    // AppImage: re-exec the AppImage path, not the squashfs-mounted current_exe.
    let launch = if let Ok(appimage) = std::env::var("APPIMAGE") {
        PathBuf::from(appimage)
    } else {
        exe
    };
    let args: Vec<String> = std::env::args().skip(1).collect();

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = Command::new(&launch).args(&args).exec();
        Err(UpdateError::Io(err))
    }
    #[cfg(target_os = "windows")]
    {
        Command::new(&launch)
            .args(&args)
            .spawn()
            .map_err(UpdateError::Io)?;
        std::process::exit(0);
    }
    #[cfg(not(any(unix, target_os = "windows")))]
    {
        let _ = (launch, args);
        Err(UpdateError::Unsupported)
    }
}

fn strip_v(tag: &str) -> &str {
    version::strip_v(tag)
}

fn http_get_string(url: &str) -> Result<String, UpdateError> {
    let response = ureq::get(url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| UpdateError::Http(e.to_string()))?;
    if !(200..300).contains(&response.status().as_u16()) {
        return Err(UpdateError::Http(format!(
            "status {} fetching {url}",
            response.status()
        )));
    }
    response
        .into_body()
        .read_to_string()
        .map_err(|e| UpdateError::Http(e.to_string()))
}

fn http_download(url: &str, dest: &Path) -> Result<(), UpdateError> {
    let response = ureq::get(url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/octet-stream")
        .call()
        .map_err(|e| UpdateError::Http(e.to_string()))?;
    if !(200..300).contains(&response.status().as_u16()) {
        return Err(UpdateError::Http(format!(
            "status {} downloading {url}",
            response.status()
        )));
    }
    let mut file = File::create(dest)?;
    let mut reader = response.into_body().into_reader();
    io::copy(&mut reader, &mut file)?;
    file.flush()?;
    Ok(())
}

fn extract_single_file(zip_path: &Path, out_dir: &Path) -> Result<PathBuf, UpdateError> {
    let file = File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| UpdateError::Zip(e.to_string()))?;
    let mut chosen = None;
    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|e| UpdateError::Zip(e.to_string()))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry
            .enclosed_name()
            .map(|p| p.to_path_buf())
            .ok_or_else(|| UpdateError::Zip("unsafe zip path".into()))?;
        chosen = Some((i, name));
        break;
    }
    let (idx, name) = chosen.ok_or_else(|| UpdateError::Zip("empty zip".into()))?;
    let out_name = name
        .file_name()
        .map(|n| n.to_owned())
        .unwrap_or_else(|| name.as_os_str().to_owned());
    let out_path = out_dir.join(out_name);
    {
        let mut entry = archive
            .by_index(idx)
            .map_err(|e| UpdateError::Zip(e.to_string()))?;
        let mut out = File::create(&out_path)?;
        io::copy(&mut entry, &mut out)?;
        out.flush()?;
    }
    Ok(out_path)
}

#[cfg(unix)]
fn apply_unix(staged: &Path, target: &Path) -> Result<(), UpdateError> {
    use std::os::unix::fs::PermissionsExt;

    let parent = target
        .parent()
        .ok_or_else(|| UpdateError::Io(io::Error::other("target has no parent")))?;
    let new_path = parent.join(format!(
        "{}.new",
        target
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("sqyre")
    ));
    fs::copy(staged, &new_path)?;
    let mut perms = fs::metadata(&new_path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&new_path, perms)?;
    fs::rename(&new_path, target)?;
    let _ = fs::remove_file(staged);
    Ok(())
}

#[cfg(target_os = "windows")]
fn apply_windows(staged: &Path, target: &Path) -> Result<(), UpdateError> {
    let mut old_os = target.as_os_str().to_owned();
    old_os.push(".old");
    let old_path = PathBuf::from(old_os);
    let _ = fs::remove_file(&old_path);
    fs::rename(target, &old_path)?;
    if let Err(_e) = fs::rename(staged, target) {
        if let Err(copy_err) = fs::copy(staged, target) {
            let _ = fs::rename(&old_path, target);
            return Err(UpdateError::Io(copy_err));
        }
        let _ = fs::remove_file(staged);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_kind_suffixes() {
        assert_eq!(InstallKind::LinuxBinary.asset_suffix(), "-linux-amd64.zip");
        assert_eq!(InstallKind::LinuxAppImage.asset_suffix(), ".AppImage.zip");
        assert_eq!(InstallKind::WindowsExe.asset_suffix(), "-windows-amd64.zip");
    }

    #[test]
    fn pick_asset_by_suffix() {
        let assets = vec![
            GhAsset {
                name: "sqyre-v2026.07.23-linux-amd64.zip".into(),
                size: 1,
                browser_download_url: "https://example/linux".into(),
            },
            GhAsset {
                name: "Sqyre-v2026.07.23-x86_64.AppImage.zip".into(),
                size: 2,
                browser_download_url: "https://example/appimage".into(),
            },
            GhAsset {
                name: "sqyre-v2026.07.23-windows-amd64.zip".into(),
                size: 3,
                browser_download_url: "https://example/win".into(),
            },
        ];
        assert_eq!(
            InstallKind::LinuxBinary
                .pick_asset(&assets)
                .map(|a| a.name.as_str()),
            Some("sqyre-v2026.07.23-linux-amd64.zip")
        );
        assert_eq!(
            InstallKind::LinuxAppImage
                .pick_asset(&assets)
                .map(|a| a.name.as_str()),
            Some("Sqyre-v2026.07.23-x86_64.AppImage.zip")
        );
        assert_eq!(
            InstallKind::WindowsExe
                .pick_asset(&assets)
                .map(|a| a.name.as_str()),
            Some("sqyre-v2026.07.23-windows-amd64.zip")
        );
    }

    #[test]
    fn extract_single_file_from_zip() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("payload.zip");
        {
            let file = File::create(&zip_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let opts = zip::write::SimpleFileOptions::default();
            zip.start_file("sqyre-v1-linux-amd64", opts).unwrap();
            zip.write_all(b"#!/bin/sqyre\n").unwrap();
            zip.finish().unwrap();
        }
        let out = extract_single_file(&zip_path, dir.path()).unwrap();
        assert_eq!(fs::read(&out).unwrap(), b"#!/bin/sqyre\n");
        assert_eq!(
            out.file_name().and_then(|n| n.to_str()),
            Some("sqyre-v1-linux-amd64")
        );
    }
}
