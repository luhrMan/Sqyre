//! ocrs-backed OCR engine (native only; not available on wasm32).
//!
//! The detection + recognition models are embedded into the binary at compile
//! time (`include_bytes!` from `assets/ocr-models/` — fetch with
//! `make ocr-models`). `SQYRE_OCR_MODELS=<dir>` overrides with a directory of
//! model files, for testing different weights without a rebuild.

use crate::ocr_boxes::{text_from_ocr_boxes, OcrRecognition, OcrWordBox};
use ocrs::{ImageSource, OcrEngineParams, TextItem};
use rten::Model;
use sqyre_match::ImageBuf;
use sqyre_ports::PortError;
use std::path::Path;
use std::sync::{Arc, OnceLock};

const DETECTION_FILE: &str = "text-detection.onnx";
const RECOGNITION_FILE: &str = "text-recognition.onnx";

static DETECTION_MODEL: &[u8] = include_bytes!(concat!(
    env!("SQYRE_WORKSPACE_ROOT"),
    "/assets/ocr-models/text-detection.onnx"
));
static RECOGNITION_MODEL: &[u8] = include_bytes!(concat!(
    env!("SQYRE_WORKSPACE_ROOT"),
    "/assets/ocr-models/text-recognition.onnx"
));

fn recognize_with(engine: &ocrs::OcrEngine, img: &ImageBuf) -> Result<OcrRecognition, PortError> {
    let src = ImageSource::from_bytes(&img.data, (img.width as u32, img.height as u32))
        .map_err(|e| PortError::invalid(format!("OCR input: {e}")))?;
    let input = engine
        .prepare_input(src)
        .map_err(|e| PortError::Message(format!("OCR prepare: {e}")))?;
    let word_rects = engine
        .detect_words(&input)
        .map_err(|e| PortError::Message(format!("OCR detect: {e}")))?;
    let line_rects = engine.find_text_lines(&input, &word_rects);
    let lines = engine
        .recognize_text(&input, &line_rects)
        .map_err(|e| PortError::Message(format!("OCR recognize: {e}")))?;

    let mut words = Vec::new();
    let mut line_texts = Vec::new();
    for line in lines.iter().flatten() {
        line_texts.push(line.to_string());
        for word in line.words() {
            let rect = word.bounding_rect();
            words.push(OcrWordBox {
                word: word.to_string(),
                left: rect.left(),
                top: rect.top(),
                right: rect.right(),
                bottom: rect.bottom(),
            });
        }
    }
    let mut text = text_from_ocr_boxes(&words);
    if text.is_empty() {
        text = line_texts.join("\n");
    }
    Ok(OcrRecognition { text, words })
}

/// Run ocrs on a preprocessed `ImageBuf` (1, 3 or 4 channel).
///
/// Prefer [`OcrsEngine::recognize`] — this constructs a fresh engine each call.
#[cfg(test)]
fn recognize_embedded(img: &ImageBuf) -> Result<OcrRecognition, PortError> {
    let engine = OcrsEngine::new()?;
    recognize_with(&engine.engine, img)
}

/// OCR engine that reuses loaded ocrs models across calls.
///
/// `ocrs::OcrEngine` is `Send + Sync` and parallelizes internally (rayon), so
/// no outer lock is needed.
pub struct OcrsEngine {
    engine: ocrs::OcrEngine,
}

impl std::fmt::Debug for OcrsEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OcrsEngine").finish_non_exhaustive()
    }
}

impl OcrsEngine {
    /// Load the embedded detection + recognition models.
    pub fn new() -> Result<Self, PortError> {
        Self::from_models(|| {
            Ok((
                Model::load_static_slice(DETECTION_MODEL)?,
                Model::load_static_slice(RECOGNITION_MODEL)?,
            ))
        })
    }

    /// Load models from a directory containing `text-detection.onnx` and
    /// `text-recognition.onnx` (`SQYRE_OCR_MODELS` override).
    fn from_dir(dir: impl AsRef<Path>) -> Result<Self, PortError> {
        let dir = dir.as_ref().to_path_buf();
        Self::from_models(|| {
            Ok((
                Model::load_file(dir.join(DETECTION_FILE))?,
                Model::load_file(dir.join(RECOGNITION_FILE))?,
            ))
        })
        .map_err(|e| PortError::Message(format!("OCR load {}: {e}", dir.display())))
    }

    fn from_models(
        load: impl FnOnce() -> Result<(Model, Model), rten::LoadError>,
    ) -> Result<Self, PortError> {
        let (detection, recognition) =
            load().map_err(|e| PortError::Message(format!("OCR load: {e}")))?;
        let engine = ocrs::OcrEngine::new(OcrEngineParams {
            detection_model: Some(detection),
            recognition_model: Some(recognition),
            ..Default::default()
        })
        .map_err(|e| PortError::Message(format!("OCR init: {e}")))?;
        Ok(Self { engine })
    }

    /// Embedded models, or the `SQYRE_OCR_MODELS` directory when set. The
    /// override is authoritative — a missing or incomplete dir is an error,
    /// not a silent fallback to the embedded pair.
    pub fn from_env_or_system() -> Result<Self, PortError> {
        Self::from_override_dir(
            std::env::var("SQYRE_OCR_MODELS")
                .ok()
                .filter(|d| !d.trim().is_empty())
                .as_deref()
                .map(Path::new),
        )
    }

    /// `Some(dir)` loads from `dir` (errors are authoritative); `None` uses
    /// the embedded models.
    fn from_override_dir(dir: Option<&Path>) -> Result<Self, PortError> {
        match dir {
            Some(dir) => Self::from_dir(dir),
            None => Self::new(),
        }
    }

    pub fn recognize(&self, img: &ImageBuf) -> Result<OcrRecognition, PortError> {
        recognize_with(&self.engine, img)
    }
}

/// Process-wide OCR engine (cloned via [`Arc`]).
static SHARED_OCR: OnceLock<Result<Arc<OcrsEngine>, PortError>> = OnceLock::new();

/// Shared [`OcrsEngine`], initialized once and reused by the startup probe and macro
/// runs so models aren't reloaded on every run.
pub fn shared_ocrs() -> Result<Arc<OcrsEngine>, PortError> {
    match SHARED_OCR.get_or_init(|| OcrsEngine::from_env_or_system().map(Arc::new)) {
        Ok(engine) => Ok(Arc::clone(engine)),
        Err(e) => Err(e.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognize_rejects_bad_input() {
        let bad = ImageBuf {
            width: 0,
            height: 2,
            channels: 1,
            data: Vec::new(),
        };
        let engine = OcrsEngine::new().expect("engine");
        let err = engine.recognize(&bad).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("OCR input"), "{msg}");
    }

    #[test]
    fn recognize_blank_image() {
        let img = ImageBuf::new(16, 16, 3, 255);
        assert!(recognize_embedded(&img).is_ok());
    }

    #[test]
    fn from_dir_missing_models_errors() {
        let err = OcrsEngine::from_dir("/tmp/sqyre-missing-ocr-models-xyz").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("OCR load"), "{msg}");
    }

    #[test]
    fn override_dir_is_attempted_and_errors() {
        // A set SQYRE_OCR_MODELS is authoritative: an empty dir must be
        // attempted (and fail) rather than silently using the embedded pair.
        let dir = tempfile::tempdir().unwrap();
        let err = OcrsEngine::from_override_dir(Some(dir.path())).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("OCR load"), "{msg}");
        assert!(msg.contains(&dir.path().display().to_string()), "{msg}");
    }

    #[test]
    fn no_override_uses_embedded() {
        assert!(OcrsEngine::from_override_dir(None).is_ok());
    }
}
