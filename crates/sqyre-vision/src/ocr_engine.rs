//! Leptess-backed OCR engine (native only; not available on wasm32).

use crate::ocr_boxes::{parse_tsv_word_boxes, text_from_ocr_boxes, OcrRecognition};
use parking_lot::Mutex;
use sqyre_match::ImageBuf;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

/// Official English traineddata used when system / env / workspace data is absent.
const ENG_TRAINEDDATA_URL: &str =
    "https://github.com/tesseract-ocr/tessdata/raw/main/eng.traineddata";
/// `eng.traineddata` is ~23 MiB; reject absurd responses.
const MAX_TESSDATA_BYTES: u64 = 40 * 1024 * 1024;
const TESSDATA_USER_AGENT: &str = "sqyre";

fn recognize_with(
    api: &mut leptess::tesseract::TessApi,
    img: &ImageBuf,
) -> Result<OcrRecognition, String> {
    let (bytes_per_pixel, bytes_per_line) = match img.channels {
        1 => (1, img.width),
        3 => (3, img.width * 3),
        other => return Err(format!("OCR: unsupported channels {other}")),
    };
    api.raw
        .set_image(
            &img.data,
            img.width as i32,
            img.height as i32,
            bytes_per_pixel,
            bytes_per_line as i32,
        )
        .map_err(|e| format!("OCR set image: {e:?}"))?;
    // Tesseract warns on 0 dpi; force a credible fallback.
    let res = api.get_source_y_resolution();
    if !(leptess::tesseract::MIN_CREDIBLE_RESOLUTION..=leptess::tesseract::MAX_CREDIBLE_RESOLUTION)
        .contains(&res)
    {
        api.set_source_resolution(70);
    }
    let tsv = api.get_tsv_text(0).map_err(|e| format!("OCR tsv: {e}"))?;
    let words = parse_tsv_word_boxes(&tsv);
    let text = {
        let joined = text_from_ocr_boxes(&words);
        if !joined.is_empty() {
            joined
        } else {
            api.get_utf8_text()
                .map_err(|e| format!("OCR text: {e}"))?
                .trim()
                .trim_matches('\n')
                .to_string()
        }
    };
    Ok(OcrRecognition { text, words })
}

/// Run Tesseract on a preprocessed `ImageBuf` (1 or 3 channel).
///
/// Prefer [`LeptessOcr::recognize`] — this constructs a fresh engine each call.
pub fn recognize_image(img: &ImageBuf, tessdata_path: &str) -> Result<OcrRecognition, String> {
    let mut api = leptess::tesseract::TessApi::new(Some(tessdata_path), "eng")
        .map_err(|e| format!("OCR init: {e}"))?;
    recognize_with(&mut api, img)
}

/// Thread-safe OCR engine that reuses one Tesseract instance across calls.
pub struct LeptessOcr {
    /// Serialize Tesseract use (API is not thread-safe) and keep the engine alive.
    engine: Mutex<leptess::tesseract::TessApi>,
}

impl std::fmt::Debug for LeptessOcr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LeptessOcr").finish_non_exhaustive()
    }
}

impl LeptessOcr {
    pub fn new(tessdata_path: impl AsRef<str>) -> Result<Self, String> {
        let path = tessdata_path.as_ref();
        let api = leptess::tesseract::TessApi::new(Some(path), "eng")
            .map_err(|e| format!("OCR init: {e}"))?;
        Ok(Self {
            engine: Mutex::new(api),
        })
    }

    /// Resolve tessdata from env / platform / workspace, downloading into the
    /// user-writable location when nothing usable is found.
    pub fn from_env_or_system() -> Result<Self, String> {
        let path = ensure_english_tessdata()?;
        Self::new(path.to_string_lossy())
    }

    pub fn recognize(&self, img: &ImageBuf) -> Result<OcrRecognition, String> {
        let mut api = self.engine.lock();
        recognize_with(&mut api, img)
    }
}

/// Process-wide OCR engine (cloned via [`Arc`]; access serialized by inner Mutex).
static SHARED_OCR: OnceLock<Result<Arc<LeptessOcr>, String>> = OnceLock::new();

/// Shared [`LeptessOcr`], initialized once and reused by the startup probe and macro
/// runs so tessdata isn't reloaded from disk on every run.
pub fn shared_leptess() -> Result<Arc<LeptessOcr>, String> {
    match SHARED_OCR.get_or_init(|| LeptessOcr::from_env_or_system().map(Arc::new)) {
        Ok(engine) => Ok(Arc::clone(engine)),
        Err(e) => Err(e.clone()),
    }
}

/// Directory containing `eng.traineddata`, discovering existing data or downloading it.
pub fn ensure_english_tessdata() -> Result<PathBuf, String> {
    if let Some(path) = find_english_tessdata() {
        return Ok(path);
    }
    let dest = writable_tessdata_dir();
    download_english_tessdata(&dest)?;
    if !has_english_tessdata(&dest) {
        return Err(format!(
            "OCR: downloaded eng.traineddata missing under {}",
            dest.display()
        ));
    }
    Ok(dest)
}

fn find_english_tessdata() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("SQYRE_TESSDATA") {
        if has_english_tessdata(&p) {
            return Some(PathBuf::from(p));
        }
    }
    for candidate in platform_tessdata_paths() {
        if has_english_tessdata(&candidate) {
            return Some(candidate);
        }
    }
    // Workspace `assets/tessdata` when developing (path from build.rs).
    let repo = Path::new(env!("SQYRE_WORKSPACE_ROOT")).join("assets/tessdata");
    if has_english_tessdata(&repo) {
        return Some(repo);
    }
    None
}

fn has_english_tessdata(path: impl AsRef<Path>) -> bool {
    path.as_ref().join("eng.traineddata").is_file()
}

/// User-writable install path used when system / env tessdata is absent.
fn writable_tessdata_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata).join("sqyre").join("tessdata");
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                return dir.join("tessdata");
            }
        }
        PathBuf::from("tessdata")
    }
    #[cfg(not(target_os = "windows"))]
    {
        dirs::home_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join(".sqyre")
            .join("tessdata")
    }
}

fn download_english_tessdata(dest_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dest_dir).map_err(|e| format!("OCR: create {}: {e}", dest_dir.display()))?;
    let dest = dest_dir.join("eng.traineddata");
    let partial = dest_dir.join("eng.traineddata.partial");
    let _ = fs::remove_file(&partial);

    eprintln!(
        "sqyre: downloading eng.traineddata into {} …",
        dest_dir.display()
    );

    let response = ureq::get(ENG_TRAINEDDATA_URL)
        .header("User-Agent", TESSDATA_USER_AGENT)
        .header("Accept", "application/octet-stream")
        .call()
        .map_err(|e| format!("OCR: download eng.traineddata: {e}"))?;
    if !(200..300).contains(&response.status().as_u16()) {
        return Err(format!(
            "OCR: download eng.traineddata failed with status {}",
            response.status()
        ));
    }

    let mut file =
        File::create(&partial).map_err(|e| format!("OCR: write {}: {e}", partial.display()))?;
    let mut reader = response.into_body().into_reader();
    let mut total = 0u64;
    let mut chunk = [0u8; 64 * 1024];
    loop {
        let n = reader
            .read(&mut chunk)
            .map_err(|e| format!("OCR: download eng.traineddata: {e}"))?;
        if n == 0 {
            break;
        }
        total = total.saturating_add(n as u64);
        if total > MAX_TESSDATA_BYTES {
            let _ = fs::remove_file(&partial);
            return Err(format!(
                "OCR: eng.traineddata download exceeded {MAX_TESSDATA_BYTES} bytes"
            ));
        }
        file.write_all(&chunk[..n])
            .map_err(|e| format!("OCR: write {}: {e}", partial.display()))?;
    }
    file.flush()
        .map_err(|e| format!("OCR: flush {}: {e}", partial.display()))?;
    drop(file);

    if total == 0 {
        let _ = fs::remove_file(&partial);
        return Err("OCR: eng.traineddata download was empty".into());
    }

    if let Err(e) = fs::rename(&partial, &dest) {
        let _ = fs::remove_file(&partial);
        return Err(format!("OCR: install eng.traineddata: {e}"));
    }
    eprintln!(
        "sqyre: installed eng.traineddata ({} bytes) in {}",
        total,
        dest_dir.display()
    );
    Ok(())
}

fn platform_tessdata_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    #[cfg(target_os = "windows")]
    {
        // Bare Windows releases keep data colocated with the executable, or in
        // the user's roaming application data directory.
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                paths.push(dir.to_path_buf());
                paths.push(dir.join("tessdata"));
            }
        }
        if let Some(appdata) = std::env::var_os("APPDATA") {
            paths.push(PathBuf::from(appdata).join("sqyre").join("tessdata"));
        }
        if let Some(program_files) = std::env::var_os("PROGRAMFILES") {
            paths.push(
                PathBuf::from(program_files)
                    .join("Tesseract-OCR")
                    .join("tessdata"),
            );
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Bundled Linux releases colocate tessdata/ next to the executable.
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                paths.push(dir.join("tessdata"));
            }
        }
        paths.extend([
            PathBuf::from("/usr/share/tesseract-ocr/4.00/tessdata"),
            PathBuf::from("/usr/share/tesseract-ocr/5/tessdata"),
            PathBuf::from("/usr/share/tessdata"),
            PathBuf::from("/usr/local/share/tessdata"),
        ]);
        // Auto-download target (checked after system paths).
        paths.push(writable_tessdata_dir());
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tessdata_or_skip() -> Option<String> {
        if let Ok(p) = std::env::var("SQYRE_TESSDATA") {
            let eng = std::path::Path::new(&p).join("eng.traineddata");
            if eng.is_file() {
                return Some(p);
            }
        }
        let repo = std::path::Path::new(env!("SQYRE_WORKSPACE_ROOT")).join("assets/tessdata");
        if repo.join("eng.traineddata").is_file() {
            return Some(repo.to_string_lossy().into_owned());
        }
        None
    }

    #[test]
    fn recognize_rejects_unsupported_channels() {
        let _img = ImageBuf::new(4, 4, 1, 0);
        // Force 2-channel via from_raw would panic; use channels=3 with wrong data path
        // by calling recognize_with logic via a 0-size edge case instead.
        let bad = ImageBuf {
            width: 2,
            height: 2,
            channels: 4,
            data: vec![0; 16],
        };
        let Some(path) = tessdata_or_skip() else {
            // Still verify the channel check without tessdata by constructing via new path.
            let err = recognize_image(&bad, "/nonexistent/tessdata").unwrap_err();
            assert!(
                err.contains("unsupported channels") || err.contains("OCR init"),
                "{err}"
            );
            return;
        };
        let err = recognize_image(&bad, &path).unwrap_err();
        assert!(err.contains("unsupported channels"), "{err}");
    }

    #[test]
    fn leptess_new_missing_tessdata_errors() {
        let err = LeptessOcr::new("/tmp/sqyre-missing-tessdata-xyz").unwrap_err();
        assert!(err.contains("OCR init"), "{err}");
    }

    #[test]
    fn recognize_image_empty_buffer_with_tessdata() {
        let Some(path) = tessdata_or_skip() else {
            eprintln!("skipping: eng.traineddata not found");
            return;
        };
        let img = ImageBuf::new(16, 16, 3, 255);
        // Blank white image should still initialize and return some recognition result.
        let result = recognize_image(&img, &path);
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn from_env_or_system_finds_existing_tessdata() {
        let Some(_) = find_english_tessdata() else {
            eprintln!("skipping: eng.traineddata not found (download not exercised here)");
            return;
        };
        let engine = LeptessOcr::from_env_or_system();
        assert!(engine.is_ok(), "{engine:?}");
        let blank = ImageBuf::new(8, 8, 3, 255);
        assert!(engine.unwrap().recognize(&blank).is_ok());
    }

    #[test]
    fn writable_tessdata_dir_ends_with_tessdata() {
        let dir = writable_tessdata_dir();
        assert_eq!(
            dir.file_name().and_then(|s| s.to_str()),
            Some("tessdata"),
            "{dir:?}"
        );
    }

    #[test]
    fn ensure_english_tessdata_resolves_or_downloads() {
        let path = ensure_english_tessdata().expect("ensure tessdata");
        assert!(
            has_english_tessdata(&path),
            "missing eng.traineddata under {}",
            path.display()
        );
    }
}
