//! Leptess-backed OCR engine (optional `ocr` feature).

use crate::ocr_boxes::{parse_tsv_word_boxes, text_from_ocr_boxes, OcrWordBox};
use parking_lot::Mutex;
use sqyre_match::ImageBuf;

/// Recognized page text plus word boxes in image coordinates.
#[derive(Debug, Clone, Default)]
pub struct OcrRecognition {
    pub text: String,
    pub words: Vec<OcrWordBox>,
}

fn recognize_with(api: &mut leptess::tesseract::TessApi, img: &ImageBuf) -> Result<OcrRecognition, String> {
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
    let tsv = api
        .get_tsv_text(0)
        .map_err(|e| format!("OCR tsv: {e}"))?;
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

    /// Resolve tessdata: `SQYRE_TESSDATA`, then common system paths, then error.
    pub fn from_env_or_system() -> Result<Self, String> {
        if let Ok(p) = std::env::var("SQYRE_TESSDATA") {
            let eng = std::path::Path::new(&p).join("eng.traineddata");
            if eng.is_file() {
                return Self::new(p);
            }
        }
        for candidate in [
            "/usr/share/tesseract-ocr/4.00/tessdata",
            "/usr/share/tesseract-ocr/5/tessdata",
            "/usr/share/tessdata",
            "/usr/local/share/tessdata",
        ] {
            let eng = std::path::Path::new(candidate).join("eng.traineddata");
            if eng.is_file() {
                return Self::new(candidate);
            }
        }
        // Workspace `assets/tessdata` when developing (path from build.rs).
        let repo = std::path::Path::new(env!("SQYRE_WORKSPACE_ROOT")).join("assets/tessdata");
        if repo.join("eng.traineddata").is_file() {
            return Self::new(repo.to_string_lossy());
        }
        Err(
            "OCR: eng.traineddata not found (set SQYRE_TESSDATA or install tesseract-ocr-eng)"
                .into(),
        )
    }

    pub fn recognize(&self, img: &ImageBuf) -> Result<OcrRecognition, String> {
        let mut api = self.engine.lock();
        recognize_with(&mut api, img)
    }
}
