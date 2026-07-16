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

/// Center of the union of boxes whose words match `target`.
/// Returns `None` when no matching word box is found.
pub fn find_target_in_boxes(boxes: &[OcrWordBox], target: &str) -> Option<(i32, i32)> {
    let target = target.trim();
    if target.is_empty() {
        return None;
    }
    let target_lower = target.to_lowercase();
    let target_words: Vec<&str> = target_lower.split_whitespace().collect();
    let mut matching: Vec<(i32, i32, i32, i32)> = Vec::new();
    for b in boxes {
        let word = b.word.trim();
        if word.is_empty() {
            continue;
        }
        let word_lower = word.to_lowercase();
        let mut matched =
            target_lower.contains(&word_lower) || word_lower.contains(&target_lower);
        if !matched && !target_words.is_empty() {
            for tw in &target_words {
                if word_lower.contains(tw) || tw.contains(&word_lower) {
                    matched = true;
                    break;
                }
            }
        }
        if matched {
            matching.push((b.left, b.top, b.right, b.bottom));
        }
    }
    if matching.is_empty() {
        return None;
    }
    let (mut min_x, mut min_y, mut max_x, mut max_y) = matching[0];
    for &(l, t, r, b) in matching.iter().skip(1) {
        min_x = min_x.min(l);
        min_y = min_y.min(t);
        max_x = max_x.max(r);
        max_y = max_y.max(b);
    }
    Some(((min_x + max_x) / 2, (min_y + max_y) / 2))
}

/// Parse Tesseract TSV (level 5 = word) into word boxes.
pub fn parse_tsv_word_boxes(tsv: &str) -> Vec<OcrWordBox> {
    let mut out = Vec::new();
    for (i, line) in tsv.lines().enumerate() {
        if i == 0 && line.starts_with("level") {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 12 {
            continue;
        }
        let Ok(level) = cols[0].parse::<i32>() else {
            continue;
        };
        // RIL_WORD == 5
        if level != 5 {
            continue;
        }
        let Ok(left) = cols[6].parse::<i32>() else {
            continue;
        };
        let Ok(top) = cols[7].parse::<i32>() else {
            continue;
        };
        let Ok(width) = cols[8].parse::<i32>() else {
            continue;
        };
        let Ok(height) = cols[9].parse::<i32>() else {
            continue;
        };
        let word = cols[11..].join("\t");
        let word = word.trim();
        if word.is_empty() {
            continue;
        }
        out.push(OcrWordBox {
            word: word.to_string(),
            left,
            top,
            right: left + width,
            bottom: top + height,
        });
    }
    out
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

    #[test]
    fn parse_tsv_words() {
        let tsv = "\
level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext
5\t1\t1\t1\t1\t1\t10\t20\t30\t15\t90\tHi
5\t1\t1\t1\t1\t2\t50\t20\t40\t15\t88\tThere
";
        let boxes = parse_tsv_word_boxes(tsv);
        assert_eq!(boxes.len(), 2);
        assert_eq!(boxes[0].word, "Hi");
        assert_eq!(boxes[0].right, 40);
        assert_eq!(text_from_ocr_boxes(&boxes), "Hi There");
    }
}
