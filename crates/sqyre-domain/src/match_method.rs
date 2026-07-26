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
}
