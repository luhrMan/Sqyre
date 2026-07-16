//! Leptess-backed OCR engine (optional `ocr` feature).

use crate::ocr_boxes::{parse_tsv_word_boxes, text_from_ocr_boxes, OcrWordBox};
use image::{ImageBuffer, Luma, Rgb};
use sqyre_match::ImageBuf;
use std::io::Cursor;
use std::sync::Mutex;

/// Recognized page text plus word boxes in image coordinates.
#[derive(Debug, Clone, Default)]
pub struct OcrRecognition {
    pub text: String,
    pub words: Vec<OcrWordBox>,
}

fn recognize_with(lt: &mut leptess::LepTess, img: &ImageBuf) -> Result<OcrRecognition, String> {
    let png = encode_png(img)?;
    lt.set_image_from_mem(&png)
        .map_err(|e| format!("OCR set image: {e}"))?;
    lt.set_fallback_source_resolution(70);
    let tsv = lt
        .get_tsv_text(0)
        .map_err(|e| format!("OCR tsv: {e}"))?;
    let words = parse_tsv_word_boxes(&tsv);
    let text = {
        let joined = text_from_ocr_boxes(&words);
        if !joined.is_empty() {
            joined
        } else {
            lt.get_utf8_text()
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
    let mut lt = leptess::LepTess::new(Some(tessdata_path), "eng")
        .map_err(|e| format!("OCR init: {e}"))?;
    recognize_with(&mut lt, img)
}

fn encode_png(img: &ImageBuf) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    let mut cursor = Cursor::new(&mut buf);
    match img.channels {
        1 => {
            let gray: ImageBuffer<Luma<u8>, _> =
                ImageBuffer::from_raw(img.width as u32, img.height as u32, img.data.clone())
                    .ok_or_else(|| "OCR encode: invalid gray buffer".to_string())?;
            gray.write_to(&mut cursor, image::ImageFormat::Png)
                .map_err(|e| format!("OCR encode png: {e}"))?;
        }
        3 => {
            let rgb: ImageBuffer<Rgb<u8>, _> =
                ImageBuffer::from_raw(img.width as u32, img.height as u32, img.data.clone())
                    .ok_or_else(|| "OCR encode: invalid rgb buffer".to_string())?;
            rgb.write_to(&mut cursor, image::ImageFormat::Png)
                .map_err(|e| format!("OCR encode png: {e}"))?;
        }
        other => return Err(format!("OCR encode: unsupported channels {other}")),
    }
    Ok(buf)
}

/// Thread-safe OCR engine that reuses one Tesseract instance across calls.
pub struct LeptessOcr {
    /// Serialize Tesseract use (API is not thread-safe) and keep the engine alive.
    engine: Mutex<leptess::LepTess>,
}

impl std::fmt::Debug for LeptessOcr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LeptessOcr").finish_non_exhaustive()
    }
}

impl LeptessOcr {
    pub fn new(tessdata_path: impl AsRef<str>) -> Result<Self, String> {
        let path = tessdata_path.as_ref();
        let lt = leptess::LepTess::new(Some(path), "eng")
            .map_err(|e| format!("OCR init: {e}"))?;
        Ok(Self {
            engine: Mutex::new(lt),
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
        let mut lt = self
            .engine
            .lock()
            .map_err(|_| "OCR engine lock poisoned".to_string())?;
        recognize_with(&mut lt, img)
    }
}
