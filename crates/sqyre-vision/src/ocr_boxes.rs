//! Word-box target location.

use sqyre_domain::OcrMatchMode;

/// One OCR word and its axis-aligned box in image coordinates (inclusive min, exclusive max).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcrWordBox {
    pub word: String,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// Recognized page text plus word boxes in image coordinates.
///
/// Platform-agnostic (unlike [`crate::OcrsEngine`], which is native-only) so it
/// can be the shared OCR result type on every target, wasm included.
#[derive(Debug, Clone, Default)]
pub struct OcrRecognition {
    pub text: String,
    pub words: Vec<OcrWordBox>,
}

/// How a Text target is matched against recognized words.
#[derive(Debug, Clone, Copy)]
pub struct OcrMatch<'a> {
    /// Target text — a regex pattern in [`OcrMatchMode::Regex`].
    pub target: &'a str,
    pub mode: OcrMatchMode,
    /// `true` compares letter-for-letter; `false` folds both sides to lowercase.
    pub case_sensitive: bool,
}

/// A match query that could not run — only [`OcrMatchMode::Regex`] can fail.
#[derive(Debug, thiserror::Error)]
#[error("invalid OCR regex target: {0}")]
pub struct OcrMatchError(#[from] pub regex::Error);

/// Join non-empty word boxes with spaces.
pub fn text_from_ocr_boxes(boxes: &[OcrWordBox]) -> String {
    let mut parts = Vec::new();
    for b in boxes {
        let w = b.word.trim();
        if !w.is_empty() {
            parts.push(w);
        }
    }
    parts.join(" ")
}

/// One phrase occurrence: its union box plus the words OCR saw inside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetOccurrence {
    /// Union rect `(left, top, right, bottom)` in image pixels.
    pub rect: (i32, i32, i32, i32),
    /// Space-joined matched words — the text found inside the box.
    pub text: String,
}

/// One hit per phrase occurrence matching `m.target` under `m.mode`: contiguous
/// word boxes whose tokens relate to the target words per the mode (multi-word
/// targets stay one hit when they form one contiguous sequence; regex mode
/// unions the boxes its match span overlaps).
pub fn find_target_occurrences(
    boxes: &[OcrWordBox],
    m: &OcrMatch<'_>,
) -> Result<Vec<TargetOccurrence>, OcrMatchError> {
    let target = m.target.trim();
    if target.is_empty() {
        return Ok(Vec::new());
    }
    let words: Vec<&OcrWordBox> = boxes.iter().filter(|b| !b.word.trim().is_empty()).collect();
    if m.mode == OcrMatchMode::Regex {
        return regex_occurrences(&words, target, m.case_sensitive);
    }

    let full_target = fold(target, m.case_sensitive);
    let target_words: Vec<String> = full_target.split_whitespace().map(str::to_string).collect();
    if target_words.is_empty() {
        return Ok(Vec::new());
    }
    let tokens: Vec<String> = words
        .iter()
        .map(|w| fold(w.word.trim(), m.case_sensitive))
        .collect();

    let mut out = Vec::new();
    let mut i = 0;
    while i < words.len() {
        if let Some(span) = match_phrase_at(&tokens, i, &target_words, &full_target, m.mode) {
            out.push(TargetOccurrence {
                rect: union_box(&words[i..i + span]),
                text: words[i..i + span]
                    .iter()
                    .map(|b| b.word.trim())
                    .collect::<Vec<_>>()
                    .join(" "),
            });
            i += span;
        } else {
            i += 1;
        }
    }
    Ok(out)
}

/// Free-text equivalent of [`find_target_occurrences`]: does `text` satisfy the
/// same match options? Used when word boxes miss or are absent. Empty target
/// always matches, so wait-until-found never arms on a blank target.
pub fn ocr_text_matches(text: &str, m: &OcrMatch<'_>) -> bool {
    let target = m.target.trim();
    if target.is_empty() {
        return true;
    }
    if m.mode == OcrMatchMode::Regex {
        return regex::RegexBuilder::new(target)
            .case_insensitive(!m.case_sensitive)
            .build()
            .is_ok_and(|re| re.is_match(text));
    }
    let full_target = fold(target, m.case_sensitive);
    let target_words: Vec<String> = full_target.split_whitespace().map(str::to_string).collect();
    if target_words.is_empty() {
        return true;
    }
    let tokens: Vec<String> = text
        .split_whitespace()
        .map(|t| fold(t, m.case_sensitive))
        .collect();
    (0..tokens.len())
        .any(|i| match_phrase_at(&tokens, i, &target_words, &full_target, m.mode).is_some())
}

/// Match each regex hit against the space-joined word texts, then union the
/// boxes the hit's byte span overlaps — that is how regex hits get coordinates.
fn regex_occurrences(
    words: &[&OcrWordBox],
    pattern: &str,
    case_sensitive: bool,
) -> Result<Vec<TargetOccurrence>, OcrMatchError> {
    let re = regex::RegexBuilder::new(pattern)
        .case_insensitive(!case_sensitive)
        .build()?;
    let mut joined = String::new();
    let mut spans = Vec::with_capacity(words.len());
    for w in words {
        if !joined.is_empty() {
            joined.push(' ');
        }
        let start = joined.len();
        joined.push_str(w.word.trim());
        spans.push((start, joined.len()));
    }
    let mut out = Vec::new();
    for m in re.find_iter(&joined) {
        let covered: Vec<&OcrWordBox> = words
            .iter()
            .zip(spans.iter())
            .filter(|(_, &(s, e))| s < m.end() && e > m.start())
            .map(|(w, _)| *w)
            .collect();
        // Zero-width or whitespace-only hits own no boxes.
        if covered.is_empty() {
            continue;
        }
        out.push(TargetOccurrence {
            rect: union_box(&covered),
            text: m.as_str().to_string(),
        });
    }
    Ok(out)
}

fn fold(s: &str, case_sensitive: bool) -> String {
    if case_sensitive {
        s.to_string()
    } else {
        s.to_lowercase()
    }
}

/// Try to match `target_words` starting at token index `start`.
/// Returns the number of tokens consumed.
fn match_phrase_at(
    tokens: &[String],
    start: usize,
    target_words: &[String],
    full_target: &str,
    mode: OcrMatchMode,
) -> Option<usize> {
    let n = target_words.len();
    if start >= tokens.len() || n == 0 {
        return None;
    }

    // Contiguous N tokens matching N target words in order.
    if start + n <= tokens.len()
        && phrase_tokens_match(&tokens[start..start + n], target_words, mode)
    {
        return Some(n);
    }

    let first = &tokens[start];
    if first.is_empty() {
        return None;
    }

    // One OCR token holding the whole multi-word phrase.
    if n > 1 && token_matches_word(first, full_target, mode) {
        return Some(1);
    }

    // Single-word target: match this token per the mode.
    if n == 1 && token_matches_word(first, &target_words[0], mode) {
        return Some(1);
    }

    None
}

fn phrase_tokens_match(tokens: &[String], target_words: &[String], mode: OcrMatchMode) -> bool {
    tokens.len() == target_words.len()
        && tokens
            .iter()
            .zip(target_words.iter())
            .all(|(t, tw)| token_matches_word(t, tw, mode))
}

fn token_matches_word(token: &str, target_word: &str, mode: OcrMatchMode) -> bool {
    if token.is_empty() || target_word.is_empty() {
        return false;
    }
    match mode {
        OcrMatchMode::Exact => token == target_word,
        OcrMatchMode::Contains => token.contains(target_word),
        OcrMatchMode::Fragment => substantial_fragment(token, target_word),
        // Regex mode never reaches per-word matching — see find_target_occurrences.
        OcrMatchMode::Regex => false,
    }
}

/// Substring tolerance for [`OcrMatchMode::Fragment`]: the shorter side must be
/// at least 3 chars and cover two-thirds of the longer side so stray glyph
/// boxes ("e" inside "network") can't match.
fn substantial_fragment(a: &str, b: &str) -> bool {
    let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    long.contains(short) && short.len() >= 3 && short.len() * 3 >= long.len() * 2
}

fn union_box(boxes: &[&OcrWordBox]) -> (i32, i32, i32, i32) {
    let (mut min_x, mut min_y, mut max_x, mut max_y) =
        (boxes[0].left, boxes[0].top, boxes[0].right, boxes[0].bottom);
    for b in boxes.iter().skip(1) {
        min_x = min_x.min(b.left);
        min_y = min_y.min(b.top);
        max_x = max_x.max(b.right);
        max_y = max_y.max(b.bottom);
    }
    (min_x, min_y, max_x, max_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bx(word: &str, l: i32, t: i32, r: i32, b: i32) -> OcrWordBox {
        OcrWordBox {
            word: word.into(),
            left: l,
            top: t,
            right: r,
            bottom: b,
        }
    }

    fn opts(target: &str, mode: OcrMatchMode) -> OcrMatch<'_> {
        OcrMatch {
            target,
            mode,
            case_sensitive: false,
        }
    }

    #[test]
    fn find_target_occurrences_two_labels() {
        let boxes = vec![
            bx("Gold", 0, 0, 10, 10),
            bx("Silver", 20, 0, 30, 10),
            bx("Gold", 0, 40, 10, 50),
        ];
        let hits = find_target_occurrences(&boxes, &opts("Gold", OcrMatchMode::Contains)).unwrap();
        let rects: Vec<_> = hits.iter().map(|o| o.rect).collect();
        assert_eq!(rects, vec![(0, 0, 10, 10), (0, 40, 10, 50)]);
        assert_eq!(hits[0].text, "Gold");
    }

    #[test]
    fn find_target_occurrences_two_phrases() {
        let boxes = vec![
            bx("Submit", 0, 0, 10, 10),
            bx("Button", 12, 0, 22, 10),
            bx("Submit", 0, 30, 10, 40),
            bx("Button", 12, 30, 22, 40),
        ];
        let hits = find_target_occurrences(&boxes, &opts("Submit Button", OcrMatchMode::Contains))
            .unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].rect, (0, 0, 22, 10));
        assert_eq!(hits[0].text, "Submit Button");
        assert_eq!(hits[1].rect, (0, 30, 22, 40));
        assert_eq!(hits[1].text, "Submit Button");
    }

    #[test]
    fn exact_rejects_partial_words() {
        let boxes = vec![
            bx("networks", 0, 0, 40, 10),
            bx("networ", 0, 20, 30, 30),
            bx("Network", 0, 40, 50, 50),
        ];
        let hits = find_target_occurrences(&boxes, &opts("network", OcrMatchMode::Exact)).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].rect, (0, 40, 50, 50));
    }

    #[test]
    fn contains_accepts_superstring_not_fragment() {
        let boxes = vec![bx("networks", 0, 0, 40, 10), bx("networ", 0, 20, 30, 30)];
        let hits =
            find_target_occurrences(&boxes, &opts("network", OcrMatchMode::Contains)).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].rect, (0, 0, 40, 10));
    }

    #[test]
    fn fragment_tolerates_clipped_reads_not_glyphs() {
        let boxes = vec![
            bx("E", 0, 0, 5, 10),
            bx("O", 20, 0, 25, 10),
            bx("networ", 40, 0, 70, 10),
        ];
        let hits =
            find_target_occurrences(&boxes, &opts("Network", OcrMatchMode::Fragment)).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].text, "networ");
        assert_eq!(hits[0].rect, (40, 0, 70, 10));
    }

    #[test]
    fn glyph_junk_never_matches_any_mode() {
        let boxes = vec![
            bx("E", 0, 0, 5, 10),
            bx("O", 20, 0, 25, 10),
            bx("Network", 40, 0, 90, 10),
        ];
        for mode in [
            OcrMatchMode::Exact,
            OcrMatchMode::Contains,
            OcrMatchMode::Fragment,
        ] {
            let hits = find_target_occurrences(&boxes, &opts("Network", mode)).unwrap();
            assert_eq!(hits.len(), 1, "mode {mode:?}");
            assert_eq!(hits[0].rect, (40, 0, 90, 10));
        }
    }

    #[test]
    fn case_sensitive_requires_exact_case() {
        let boxes = vec![bx("network", 0, 0, 40, 10), bx("Network", 0, 20, 50, 30)];
        let m = OcrMatch {
            target: "Network",
            mode: OcrMatchMode::Exact,
            case_sensitive: true,
        };
        let hits = find_target_occurrences(&boxes, &m).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].rect, (0, 20, 50, 30));
    }

    #[test]
    fn regex_matches_pattern_over_joined_words() {
        let boxes = vec![
            bx("Order", 0, 0, 30, 10),
            bx("#1284", 35, 0, 75, 10),
            bx("shipped", 80, 0, 140, 10),
        ];
        let hits = find_target_occurrences(&boxes, &opts(r"#\d+", OcrMatchMode::Regex)).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].rect, (35, 0, 75, 10));
        assert_eq!(hits[0].text, "#1284");
    }

    #[test]
    fn regex_spanning_words_unions_boxes() {
        let boxes = vec![
            bx("flight", 0, 0, 30, 10),
            bx("SQ", 35, 0, 55, 10),
            bx("123", 60, 0, 90, 10),
        ];
        let hits =
            find_target_occurrences(&boxes, &opts(r"SQ\s?\d+", OcrMatchMode::Regex)).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].rect, (35, 0, 90, 10));
    }

    #[test]
    fn invalid_regex_is_an_error() {
        let boxes = vec![bx("anything", 0, 0, 10, 10)];
        assert!(find_target_occurrences(&boxes, &opts("(", OcrMatchMode::Regex)).is_err());
    }

    #[test]
    fn text_matches_follow_the_same_modes() {
        let exact = opts("network", OcrMatchMode::Exact);
        let contains = opts("network", OcrMatchMode::Contains);
        let fragment = opts("network", OcrMatchMode::Fragment);
        assert!(!ocr_text_matches("networks", &exact));
        assert!(ocr_text_matches("networks", &contains));
        assert!(ocr_text_matches("networks", &fragment));
        assert!(ocr_text_matches("networ", &fragment));
        assert!(!ocr_text_matches("networ", &contains));
        assert!(ocr_text_matches(
            "say Hi there",
            &opts("Hi", OcrMatchMode::Exact)
        ));
        assert!(!ocr_text_matches(
            "hello",
            &opts("Hi", OcrMatchMode::Contains)
        ));
    }

    #[test]
    fn text_matches_empty_target_always() {
        assert!(ocr_text_matches("", &opts("", OcrMatchMode::Exact)));
        assert!(ocr_text_matches(
            "anything",
            &opts("", OcrMatchMode::Contains)
        ));
    }

    #[test]
    fn text_matches_regex() {
        assert!(ocr_text_matches(
            "order #1284 shipped",
            &opts(r"#\d+", OcrMatchMode::Regex)
        ));
        assert!(!ocr_text_matches(
            "order shipped",
            &opts(r"#\d+", OcrMatchMode::Regex)
        ));
    }

    #[test]
    fn text_from_boxes_joins() {
        let boxes = vec![
            bx("Hello", 0, 0, 1, 1),
            bx("  ", 0, 0, 1, 1),
            bx("World", 0, 0, 1, 1),
        ];
        assert_eq!(text_from_ocr_boxes(&boxes), "Hello World");
    }
}
