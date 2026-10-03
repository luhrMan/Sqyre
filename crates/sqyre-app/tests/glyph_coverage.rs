//! Every non-ASCII char in UI string/char literals must have a glyph in the
//! installed fonts; otherwise egui draws a hollow "missing glyph" box.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use skrifa::MetadataProvider;

use sqyre_app::SettingsUi;

const UI_CRATES: &[&str] = &[
    "sqyre-app",
    "sqyre-overlay",
    "sqyre-ui-model",
    "sqyre-ui-theme",
];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Non-ASCII chars inside string and char literals (comments skipped).
fn literal_chars(src: &str) -> Vec<(usize, char)> {
    let s: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut line = 1;
    let mut i = 0;
    let at = |i: usize| s.get(i).copied().unwrap_or('\0');
    while i < s.len() {
        let c = s[i];
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c == '/' && at(i + 1) == '/' {
            while i < s.len() && s[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && at(i + 1) == '*' {
            i += 2;
            while i < s.len() && !(s[i] == '*' && at(i + 1) == '/') {
                line += usize::from(s[i] == '\n');
                i += 1;
            }
            i += 2;
        } else if c == 'r'
            && (at(i + 1) == '"' || at(i + 1) == '#')
            && !is_ident(at(i.wrapping_sub(1)))
        {
            let mut j = i + 1;
            let mut hashes = 0;
            while at(j) == '#' {
                hashes += 1;
                j += 1;
            }
            if at(j) != '"' {
                i += 1;
                continue;
            }
            j += 1;
            loop {
                if j >= s.len() {
                    break;
                }
                if s[j] == '"' && (0..hashes).all(|k| at(j + 1 + k) == '#') {
                    j += 1 + hashes;
                    break;
                }
                line += usize::from(s[j] == '\n');
                if !s[j].is_ascii() {
                    out.push((line, s[j]));
                }
                j += 1;
            }
            i = j;
        } else if c == '"' {
            i += 1;
            while i < s.len() && s[i] != '"' {
                if s[i] == '\\' {
                    i += 1;
                }
                line += usize::from(at(i) == '\n');
                if !at(i).is_ascii() {
                    out.push((line, s[i]));
                }
                i += 1;
            }
            i += 1;
        } else if c == '\'' && at(i + 1) != '\\' && at(i + 2) == '\'' {
            if !at(i + 1).is_ascii() {
                out.push((line, s[i + 1]));
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    out
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[test]
fn ui_literals_have_glyphs() {
    let ctx = egui::Context::default();
    SettingsUi::install_fonts(&ctx);
    ctx.run_ui(egui::RawInput::default(), |_| {})
        .textures_delta
        .clear();

    let crates_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for name in UI_CRATES {
        rust_files(&crates_dir.join(name).join("src"), &mut files);
    }
    assert!(!files.is_empty(), "no UI sources found");

    // `Fonts::has_glyph` reports false negatives for chars owned by the
    // replacement-glyph face, so check each face's charmap directly.
    let defs = ctx.fonts(|f| f.definitions().clone());
    let faces: Vec<_> = defs.families[&egui::FontFamily::Proportional]
        .iter()
        .map(|name| defs.font_data[name].clone())
        .collect();
    let has_glyph = |c: char| {
        faces.iter().any(|data| {
            skrifa::FontRef::from_index(&data.font, data.index)
                .is_ok_and(|font| font.charmap().map(c).is_some())
        })
    };

    let mut missing: BTreeMap<char, Vec<String>> = BTreeMap::new();
    for file in &files {
        let src = std::fs::read_to_string(file).expect("read source");
        for (line, c) in literal_chars(&src) {
            if c.is_whitespace() || matches!(c, '\u{fe0f}' | '\u{200d}') {
                continue;
            }
            if !has_glyph(c) {
                let rel = file.strip_prefix(&crates_dir).unwrap_or(file);
                missing
                    .entry(c)
                    .or_default()
                    .push(format!("{}:{line}", rel.display()));
            }
        }
    }

    let report: Vec<String> = missing
        .iter()
        .map(|(c, sites)| format!("{c:?} U+{:04X}: {}", u32::from(*c), sites.join(", ")))
        .collect();
    assert!(
        missing.is_empty(),
        "glyphs missing from installed fonts:\n{}",
        report.join("\n")
    );
}
