//! Template-match method enum shared by the wire format and the matching engine.

use serde::{Deserialize, Serialize};

/// OpenCV `cv::TemplateMatchModes` (methods 0–5).
///
/// The single template-match-method enum shared by the domain action model
/// (wire format) and the matching engine itself.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchMethod {
    Sqdiff = 0,
    SqdiffNormed = 1,
    Ccorr = 2,
    CcorrNormed = 3,
    Ccoeff = 4,
    #[default]
    CcoeffNormed = 5,
}

impl MatchMethod {
    pub const ALL: [Self; 6] = [
        Self::Sqdiff,
        Self::SqdiffNormed,
        Self::Ccorr,
        Self::CcorrNormed,
        Self::Ccoeff,
        Self::CcoeffNormed,
    ];

    /// Beginner-friendly combo order (recommended first). Wire/`ALL` stay OpenCV order.
    pub const UI_ORDER: [Self; 6] = [
        Self::CcoeffNormed,
        Self::Ccoeff,
        Self::CcorrNormed,
        Self::Ccorr,
        Self::SqdiffNormed,
        Self::Sqdiff,
    ];

    /// OpenCV-style name for wire/debug/engine surfaces.
    pub fn label(self) -> &'static str {
        match self {
            Self::Sqdiff => "SQDIFF",
            Self::SqdiffNormed => "SQDIFF_NORMED",
            Self::Ccorr => "CCORR",
            Self::CcorrNormed => "CCORR_NORMED",
            Self::Ccoeff => "CCOEFF",
            Self::CcoeffNormed => "CCOEFF_NORMED",
        }
    }

    /// Everyday name for UI combo boxes and beginner-facing labels.
    pub fn ui_label(self) -> &'static str {
        match self {
            Self::CcoeffNormed => "Default (recommended)",
            Self::Ccoeff => "Default (raw score)",
            Self::CcorrNormed => "Correlation",
            Self::Ccorr => "Correlation (raw)",
            Self::SqdiffNormed => "Difference (lower is better)",
            Self::Sqdiff => "Difference raw (lower is better)",
        }
    }

    /// Short hover hint for the method combo (optional per-option detail).
    pub fn ui_hint(self) -> &'static str {
        match self {
            Self::CcoeffNormed => {
                "Best starting point. Scores stay near 0–1; higher means a closer match."
            }
            Self::Ccoeff => "Same idea as Default, but scores are not scaled to 0–1.",
            Self::CcorrNormed => "How well the image lines up with the template (0–1 scale).",
            Self::Ccorr => "Same as Correlation, with raw (unscaled) scores.",
            Self::SqdiffNormed => {
                "How different the pixels are (0–1). Lower scores are better; 0 is a perfect match."
            }
            Self::Sqdiff => "Same as Difference, with raw (unscaled) scores. Lower is better.",
        }
    }

    /// `false` for `SQDIFF` / `SQDIFF_NORMED` (lower score is better).
    #[inline]
    pub fn higher_is_better(self) -> bool {
        !matches!(self, Self::Sqdiff | Self::SqdiffNormed)
    }

    #[inline]
    pub fn is_normed(self) -> bool {
        matches!(
            self,
            Self::SqdiffNormed | Self::CcorrNormed | Self::CcoeffNormed
        )
    }

    /// `CCOEFF` / `CCOEFF_NORMED` — the mean-subtracting family.
    #[inline]
    pub fn is_ccoeff_family(self) -> bool {
        matches!(self, Self::Ccoeff | Self::CcoeffNormed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqdiff_family_prefers_lower_scores() {
        assert!(!MatchMethod::Sqdiff.higher_is_better());
        assert!(!MatchMethod::SqdiffNormed.higher_is_better());
        assert!(MatchMethod::CcoeffNormed.higher_is_better());
    }

    #[test]
    fn serde_uses_snake_case_wire_names() {
        let yaml = serde_yaml::to_string(&MatchMethod::CcoeffNormed).unwrap();
        assert_eq!(yaml.trim(), "ccoeff_normed");
        let back: MatchMethod = serde_yaml::from_str("sqdiff_normed").unwrap();
        assert_eq!(back, MatchMethod::SqdiffNormed);
    }

    #[test]
    fn label_keeps_opencv_names() {
        assert_eq!(MatchMethod::CcoeffNormed.label(), "CCOEFF_NORMED");
        assert_eq!(MatchMethod::Sqdiff.label(), "SQDIFF");
    }

    #[test]
    fn ui_label_uses_everyday_names() {
        assert_eq!(
            MatchMethod::CcoeffNormed.ui_label(),
            "Default (recommended)"
        );
        assert_eq!(MatchMethod::Ccoeff.ui_label(), "Default (raw score)");
        assert_eq!(MatchMethod::CcorrNormed.ui_label(), "Correlation");
        assert_eq!(MatchMethod::Ccorr.ui_label(), "Correlation (raw)");
        assert_eq!(
            MatchMethod::SqdiffNormed.ui_label(),
            "Difference (lower is better)"
        );
        assert_eq!(
            MatchMethod::Sqdiff.ui_label(),
            "Difference raw (lower is better)"
        );
        for m in MatchMethod::ALL {
            assert!(!m.ui_hint().is_empty());
        }
    }
}
