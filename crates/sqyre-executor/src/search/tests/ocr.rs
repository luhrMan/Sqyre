use super::*;

#[test]
fn ocr_empty_target_always_matches() {
    assert!(ocr_target_matched("", ""));
    assert!(ocr_target_matched("", "anything"));
    assert!(ocr_target_matched("Hi", "say Hi there"));
    assert!(!ocr_target_matched("Hi", "hello"));
}

#[test]
fn ocr_writes_text_and_target_coords() {
    use crate::test_support::FixedOcrEngine;
    use sqyre_vision::OcrRecognition;
    use sqyre_vision::OcrWordBox;

    let img = RgbaImage::from_pixel(20, 10, Rgba([255, 255, 255, 255]));
    let mut backend = RecordingBackend::default();
    let mut capturer = RecordingCapturer {
        next: Some(img),
        bounds: full_desktop(),
        ..Default::default()
    };
    let resolver = SEARCH_FIXED_AREA;
    let ocr = FixedOcrEngine {
        result: OcrRecognition {
            text: "Hello Submit Button".into(),
            words: vec![
                OcrWordBox {
                    word: "Hello".into(),
                    left: 0,
                    top: 0,
                    right: 40,
                    bottom: 20,
                },
                OcrWordBox {
                    word: "Submit".into(),
                    left: 50,
                    top: 0,
                    right: 110,
                    bottom: 20,
                },
                OcrWordBox {
                    word: "Button".into(),
                    left: 120,
                    top: 0,
                    right: 180,
                    bottom: 20,
                },
            ],
        },
        ..Default::default()
    };
    let log = SharedActionLog::new();
    let ocr_id = ActionId::new();

    let mut macro_ = quiet_macro(vec![Action {
        id: ocr_id,
        kind: ActionKind::Ocr {
            name: "read".into(),
            target: "Submit".into(),
            search_area: CoordinateRef("prog~box".into()),
            output_variable: "ocrText".into(),
            blur: 1,
            min_threshold: 0,
            resize: 1.0,
            grayscale: true,
            threshold_otsu: false,
            threshold_invert: false,
            detection: sqyre_domain::DetectionBranch {
                coords: CoordinateOutputs {
                    output_x_variable: "foundX".into(),
                    output_y_variable: "foundY".into(),
                },
                ..Default::default()
            },
        },
    }]);

    execute_macro_with(
        &mut macro_,
        ExecDeps::new(&mut backend)
            .capturer(&mut capturer)
            .resolver(&resolver)
            .ocr(&ocr)
            .logger(&log),
    )
    .unwrap();

    assert_eq!(
        macro_.variables.get("ocrText").map(|v| v.as_display()),
        Some("Hello Submit Button".into())
    );
    // SEARCH_FIXED_AREA resolves to (100,200)-(110,210); box center (80,10) + origin
    assert_eq!(
        macro_.variables.get("foundX").map(|v| v.as_display()),
        Some("180".into())
    );
    assert_eq!(
        macro_.variables.get("foundY").map(|v| v.as_display()),
        Some("210".into())
    );
    let entries = log.entries_for(ocr_id);
    let image_labels: Vec<_> = entries
        .iter()
        .filter_map(|e| match e {
            crate::ActionLogEntry::Image(img) => Some(img.label.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        image_labels.iter().any(|l| l.contains("Capture")),
        "expected capture image: {image_labels:?}"
    );
    assert!(
        image_labels
            .iter()
            .any(|l| l.contains("Ready for OCR") || l.contains("Grayscale")),
        "expected preprocess step images: {image_labels:?}"
    );
    assert!(
        image_labels.iter().any(|l| l.contains("word boxes")),
        "expected OCR word-box overlay: {image_labels:?}"
    );
    let lines = lines_for(&entries);
    assert!(
        lines.iter().any(|l| l.contains("OCR full text")),
        "expected full OCR text log: {lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("word[") && l.contains("Hello")),
        "expected per-word OCR detail: {lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("word[") && l.contains("Submit")),
        "expected Submit word in OCR detail: {lines:?}"
    );
}

#[test]
fn ocr_runs_branch_when_target_found() {
    use crate::test_support::FixedOcrEngine;
    use sqyre_vision::OcrRecognition;
    use sqyre_vision::OcrWordBox;

    let mut backend = RecordingBackend::default();
    let mut capturer = capturer_queue(vec![
        RgbaImage::from_pixel(20, 10, Rgba([255, 255, 255, 255])),
        RgbaImage::from_pixel(20, 10, Rgba([254, 255, 255, 255])),
    ]);
    let resolver = SEARCH_FIXED_AREA;
    let ocr = FixedOcrEngine {
        result: OcrRecognition {
            text: "Hello Submit Button".into(),
            words: vec![OcrWordBox {
                word: "Submit".into(),
                left: 50,
                top: 0,
                right: 110,
                bottom: 20,
            }],
        },
        ..Default::default()
    };

    let mut macro_ = quiet_macro(vec![ocr_action(
        ActionId::new(),
        "Submit",
        "ocrText",
        detection_branch(Some(19), None, Default::default(), Default::default()),
    )]);

    run_search_ocr(&mut macro_, &mut backend, &mut capturer, &resolver, &ocr);
    assert_slept(&backend, 19);
    assert_eq!(
        macro_.variables.get("ocrText").map(|v| v.as_display()),
        Some("Hello Submit Button".into())
    );
}

#[test]
fn ocr_no_find_runs_branch_when_flag_set() {
    use crate::test_support::FixedOcrEngine;
    use sqyre_vision::OcrRecognition;

    let mut backend = RecordingBackend::default();
    let mut capturer = capturer_queue(vec![
        RgbaImage::from_pixel(20, 10, Rgba([255, 255, 255, 255])),
        RgbaImage::from_pixel(20, 10, Rgba([254, 255, 255, 255])),
    ]);
    let resolver = SEARCH_FIXED_AREA;
    let ocr = FixedOcrEngine {
        result: OcrRecognition {
            text: "Hello World".into(),
            words: vec![],
        },
        ..Default::default()
    };

    let mut macro_ = Macro::new("t", 0, vec![]);
    macro_.keyboard_delay = 0;
    macro_.mouse_delay = 0;
    macro_.variables.set("foundX", ScalarValue::Int(1));
    macro_.variables.set("foundY", ScalarValue::Int(2));
    macro_
        .variables
        .set("ocrText", ScalarValue::String("stale".into()));
    macro_.root = root_loop(vec![ocr_action(
        ActionId::new(),
        "will-not-match-zzz",
        "ocrText",
        detection_branch(
            None,
            Some(17),
            Default::default(),
            coords_xy("foundX", "foundY"),
        ),
    )]);

    run_search_ocr(&mut macro_, &mut backend, &mut capturer, &resolver, &ocr);
    assert_slept(&backend, 17);
    assert!(macro_.variables.get("ocrText").is_none());
    assert!(macro_.variables.get("foundX").is_none());
    assert!(macro_.variables.get("foundY").is_none());
}

#[test]
fn ocr_skips_branch_when_target_missing() {
    use crate::test_support::FixedOcrEngine;
    use sqyre_vision::OcrRecognition;

    let mut backend = RecordingBackend::default();
    let mut capturer = capturer_queue(vec![
        RgbaImage::from_pixel(20, 10, Rgba([255, 255, 255, 255])),
        RgbaImage::from_pixel(20, 10, Rgba([254, 255, 255, 255])),
    ]);
    let resolver = SEARCH_FIXED_AREA;
    let ocr = FixedOcrEngine {
        result: OcrRecognition {
            text: "Hello World".into(),
            words: vec![],
        },
        ..Default::default()
    };

    let mut macro_ = quiet_macro(vec![ocr_action(
        ActionId::new(),
        "Submit",
        "ocrText",
        detection_branch(Some(19), None, Default::default(), Default::default()),
    )]);

    run_search_ocr(&mut macro_, &mut backend, &mut capturer, &resolver, &ocr);
    assert_did_not_sleep(&backend, 19);
    assert!(macro_.variables.get("ocrText").is_none());
}

#[test]
fn ocr_wait_until_found_retries_then_succeeds() {
    use crate::test_support::QueuedOcrEngine;
    use sqyre_vision::OcrRecognition;

    let mut backend = RecordingBackend::default();
    let mut capturer = capturer_queue(vec![
        RgbaImage::from_pixel(20, 10, Rgba([255, 255, 255, 255])),
        RgbaImage::from_pixel(20, 10, Rgba([254, 255, 255, 255])),
    ]);
    let resolver = SEARCH_FIXED_AREA;
    let ocr = QueuedOcrEngine {
        queue: std::sync::Mutex::new(vec![
            OcrRecognition {
                text: "noise".into(),
                words: vec![],
            },
            OcrRecognition {
                text: "Submit now".into(),
                words: vec![],
            },
        ]),
        ..Default::default()
    };

    let mut macro_ = quiet_macro(vec![ocr_action(
        ActionId::new(),
        "Submit",
        "ocrText",
        detection_branch(
            Some(11),
            None,
            wait_until_found(5, 1),
            coords_xy("foundX", "foundY"),
        ),
    )]);

    run_search_ocr(&mut macro_, &mut backend, &mut capturer, &resolver, &ocr);

    assert!(
        ocr.log.lock().unwrap().len() >= 2,
        "expected OCR retries: {:?}",
        ocr.log.lock().unwrap()
    );
    assert_eq!(
        macro_.variables.get("ocrText").map(|v| v.as_display()),
        Some("Submit now".into())
    );
    assert_slept(&backend, 11);
}

#[test]
fn ocr_repeat_while_found_then_stops_on_miss() {
    use crate::test_support::QueuedOcrEngine;
    use sqyre_vision::OcrRecognition;

    let mut backend = RecordingBackend::default();
    let mut capturer = capturer_queue(vec![
        RgbaImage::from_pixel(20, 10, Rgba([255, 255, 255, 255])),
        RgbaImage::from_pixel(20, 10, Rgba([254, 255, 255, 255])),
    ]);
    let resolver = SEARCH_FIXED_AREA;
    let ocr = QueuedOcrEngine {
        queue: std::sync::Mutex::new(vec![
            OcrRecognition {
                text: "Keep going".into(),
                words: vec![],
            },
            OcrRecognition {
                text: "gone".into(),
                words: vec![],
            },
        ]),
        ..Default::default()
    };

    let mut macro_ = quiet_macro(vec![ocr_action(
        ActionId::new(),
        "Keep",
        "ocrText",
        detection_branch(Some(8), None, while_found(1, 5), Default::default()),
    )]);

    run_search_ocr(&mut macro_, &mut backend, &mut capturer, &resolver, &ocr);

    assert_eq!(
        count_sleeps(&backend, 8),
        1,
        "expected one OCR while-found iteration: {:?}",
        backend.log
    );
}

#[test]
fn ocr_runs_children_per_occurrence() {
    use crate::test_support::FixedOcrEngine;
    use sqyre_vision::OcrRecognition;
    use sqyre_vision::OcrWordBox;

    let mut backend = RecordingBackend::default();
    let mut capturer = capturer_queue(vec![
        RgbaImage::from_pixel(20, 10, Rgba([255, 255, 255, 255])),
        RgbaImage::from_pixel(20, 10, Rgba([254, 255, 255, 255])),
    ]);
    let resolver = SEARCH_FIXED_AREA;
    let ocr = FixedOcrEngine {
        result: OcrRecognition {
            text: "Gold Silver Gold".into(),
            words: vec![
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
            ],
        },
        ..Default::default()
    };

    let mut macro_ = quiet_macro(vec![ocr_action(
        ActionId::new(),
        "Gold",
        "ocrText",
        detection_branch(Some(5), None, Default::default(), Default::default()),
    )]);

    run_search_ocr(&mut macro_, &mut backend, &mut capturer, &resolver, &ocr);
    assert_eq!(
        count_sleeps(&backend, 5),
        2,
        "expected one child run per Gold occurrence: {:?}",
        backend.log
    );
}
