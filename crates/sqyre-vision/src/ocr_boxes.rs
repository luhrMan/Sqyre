//! Word-box target location.

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

/// Center of the first phrase occurrence matching `target`.
/// Returns `None` when no matching occurrence is found.
pub fn find_target_in_boxes(boxes: &[OcrWordBox], target: &str) -> Option<(i32, i32)> {
    find_target_occurrences(boxes, target).into_iter().next()
}

/// One hit per phrase occurrence: contiguous word boxes matching `target`, each at the
/// union-center of that occurrence. Multi-word targets like `"Submit Button"` stay one
/// hit when they form one contiguous sequence.
pub fn find_target_occurrences(boxes: &[OcrWordBox], target: &str) -> Vec<(i32, i32)> {
    let target = target.trim();
    if target.is_empty() {
        return Vec::new();
    }
    let target_lower = target.to_lowercase();
    let target_words: Vec<&str> = target_lower.split_whitespace().collect();
    if target_words.is_empty() {
        return Vec::new();
    }

    let words: Vec<&OcrWordBox> = boxes.iter().filter(|b| !b.word.trim().is_empty()).collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < words.len() {
        if let Some(span) = match_phrase_at(&words, i, &target_words, &target_lower) {
            out.push(union_center(&words[i..i + span]));
            i += span;
        } else {
            i += 1;
        }
    }
    out
}

/// Try to match `target_words` starting at `start`. Returns the number of boxes consumed.
fn match_phrase_at(
    words: &[&OcrWordBox],
    start: usize,
    target_words: &[&str],
    full_target_lower: &str,
) -> Option<usize> {
    let n = target_words.len();
    if start >= words.len() || n == 0 {
        return None;
    }

    // Contiguous N boxes matching N target words in order.
    if start + n <= words.len() && phrase_boxes_match(&words[start..start + n], target_words) {
        return Some(n);
    }

    let first = words[start].word.trim().to_lowercase();
    if first.is_empty() {
        return None;
    }

    // One OCR token holding the whole multi-word phrase.
    if n > 1 && (first == full_target_lower || first.contains(full_target_lower)) {
        return Some(1);
    }

    // Single-word target: fuzzy match this box.
    if n == 1 && token_matches_word(&first, target_words[0]) {
        return Some(1);
    }

    None
}

fn phrase_boxes_match(boxes: &[&OcrWordBox], target_words: &[&str]) -> bool {
    if boxes.len() != target_words.len() {
        return false;
    }
    boxes
        .iter()
        .zip(target_words.iter())
        .all(|(b, tw)| token_matches_word(&b.word.trim().to_lowercase(), tw))
}

fn token_matches_word(token: &str, target_word: &str) -> bool {
    !token.is_empty()
        && !target_word.is_empty()
        && (token == target_word || token.contains(target_word) || target_word.contains(token))
}

fn union_center(boxes: &[&OcrWordBox]) -> (i32, i32) {
    let (mut min_x, mut min_y, mut max_x, mut max_y) =
        (boxes[0].left, boxes[0].top, boxes[0].right, boxes[0].bottom);
    for b in boxes.iter().skip(1) {
        min_x = min_x.min(b.left);
        min_y = min_y.min(b.top);
        max_x = max_x.max(b.right);
        max_y = max_y.max(b.bottom);
    }
    ((min_x + max_x) / 2, (min_y + max_y) / 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_target_union_center() {
        let boxes = vec![
            OcrWordBox {
                word: "Submit".into(),
                left: 10,
                top: 10,
                right: 50,
                bottom: 30,
            },
            OcrWordBox {
                word: "Button".into(),
                left: 55,
                top: 10,
                right: 100,
                bottom: 30,
            },
        ];
        let (cx, cy) = find_target_in_boxes(&boxes, "Submit Button").unwrap();
        assert_eq!((cx, cy), (55, 20));
    }

    #[test]
    fn find_target_occurrences_two_labels() {
        let boxes = vec![
            OcrWordBox {
                word: "Gold".into(),
                left: 0,
                top: 0,
                right: 10,
                bottom: 10,
            },
            OcrWordBox {
                word: "Silver".into(),
                left: 20,
                top: 0,
                right: 30,
                bottom: 10,
            },
            OcrWordBox {
                word: "Gold".into(),
                left: 0,
                top: 40,
                right: 10,
                bottom: 50,
            },
        ];
        let hits = find_target_occurrences(&boxes, "Gold");
        assert_eq!(hits, vec![(5, 5), (5, 45)]);
    }

    #[test]
    fn find_target_occurrences_two_phrases() {
        let boxes = vec![
            OcrWordBox {
                word: "Submit".into(),
                left: 0,
                top: 0,
                right: 10,
                bottom: 10,
            },
            OcrWordBox {
                word: "Button".into(),
                left: 12,
                top: 0,
                right: 22,
                bottom: 10,
            },
            OcrWordBox {
                word: "Submit".into(),
                left: 0,
                top: 30,
                right: 10,
                bottom: 40,
            },
            OcrWordBox {
                word: "Button".into(),
                left: 12,
                top: 30,
                right: 22,
                bottom: 40,
            },
        ];
        let hits = find_target_occurrences(&boxes, "Submit Button");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0], (11, 5));
        assert_eq!(hits[1], (11, 35));
    }

    #[test]
    fn text_from_boxes_joins() {
        let boxes = vec![
            OcrWordBox {
                word: "Hello".into(),
                left: 0,
                top: 0,
                right: 1,
                bottom: 1,
            },
            OcrWordBox {
                word: "  ".into(),
                left: 0,
                top: 0,
                right: 1,
                bottom: 1,
            },
            OcrWordBox {
                word: "World".into(),
                left: 0,
                top: 0,
                right: 1,
                bottom: 1,
            },
        ];
        assert_eq!(text_from_ocr_boxes(&boxes), "Hello World");
    }
}
