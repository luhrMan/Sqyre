//! YAML syntax colouring and the line-number gutter for the builder editor.

use std::ops::Range;
use std::sync::Arc;

use eframe::egui::{self, text::LayoutJob, Color32, FontId, Galley, TextFormat};
use sqyre_ui_model::nested_var_ref_color;

use crate::theme::{contrast_fg, ok_fg, rgba, syntax_literal, PRIMARY, SPACE_4, SPACE_8};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tok {
    Text,
    Punct,
    Key,
    Str,
    Literal,
    Comment,
    /// `${name}` variable reference.
    Var,
    /// `Program~Entity` catalog reference.
    Ref,
}

/// Theme colours for each token, resolved once per frame.
struct Palette {
    font: FontId,
    text: Color32,
    weak: Color32,
    var_bg: Color32,
}

impl Palette {
    fn new(ui: &egui::Ui) -> Self {
        let v = ui.visuals();
        Self {
            font: egui::TextStyle::Monospace.resolve(ui.style()),
            text: v.text_color(),
            weak: v.weak_text_color(),
            // Same chip fill as variable pills elsewhere in the app.
            var_bg: rgba(nested_var_ref_color(v.dark_mode)),
        }
    }

    fn format(&self, tok: Tok) -> TextFormat {
        let mut f = TextFormat::simple(self.font.clone(), self.text);
        match tok {
            Tok::Text => {}
            Tok::Punct | Tok::Comment => f.color = self.weak,
            Tok::Key => f.color = PRIMARY,
            Tok::Str => f.color = ok_fg(),
            Tok::Literal => f.color = syntax_literal(),
            Tok::Var => {
                f.background = self.var_bg;
                f.color = contrast_fg(self.var_bg);
            }
            Tok::Ref => f.italics = true,
        }
        f
    }
}

/// Styled byte spans covering `line` (no trailing `\n`).
fn line_spans(line: &str) -> Vec<(Range<usize>, Tok)> {
    let mut out = Vec::new();
    let mut i = line.len() - line.trim_start_matches(' ').len();
    if i > 0 {
        out.push((0..i, Tok::Text));
    }
    while line[i..].starts_with("- ") || &line[i..] == "-" {
        let end = (i + 2).min(line.len());
        out.push((i..end, Tok::Punct));
        i = end;
    }
    let rest = &line[i..];
    if let Some(colon) = key_end(rest) {
        out.push((i..i + colon, Tok::Key));
        out.push((i + colon..i + colon + 1, Tok::Punct));
        i += colon + 1;
    }
    value_spans(line, i, &mut out);
    out
}

/// Byte offset of the `:` ending a plain mapping key at the start of `s`.
fn key_end(s: &str) -> Option<usize> {
    if s.starts_with(['#', '"', '\'']) {
        return None;
    }
    let colon = s.find(':')?;
    let key = &s[..colon];
    let after = &s[colon + 1..];
    (!key.is_empty() && !key.contains(' ') && (after.is_empty() || after.starts_with(' ')))
        .then_some(colon)
}

/// Spans for a scalar value starting at `from`, including any trailing ` # comment`.
fn value_spans(line: &str, from: usize, out: &mut Vec<(Range<usize>, Tok)>) {
    let rest = &line[from..];
    let lead = rest.len() - rest.trim_start_matches(' ').len();
    if lead > 0 {
        out.push((from..from + lead, Tok::Text));
    }
    let start = from + lead;
    let body = &line[start..];
    if body.is_empty() {
        return;
    }
    if body.starts_with('#') {
        out.push((start..line.len(), Tok::Comment));
        return;
    }
    let (scalar_end, quoted) = match body.chars().next() {
        Some(q @ ('"' | '\'')) => {
            let close = body[1..].find(q).map_or(body.len(), |c| c + 2);
            (start + close, true)
        }
        _ => (start + body.find(" #").unwrap_or(body.len()), false),
    };
    let scalar = &line[start..scalar_end];
    let tok = if quoted {
        Tok::Str
    } else {
        classify(scalar.trim_end())
    };
    push_with_vars(start, scalar, tok, out);
    if scalar_end < line.len() {
        let tail = &line[scalar_end..];
        let gap = tail.len() - tail.trim_start_matches(' ').len();
        if gap > 0 {
            out.push((scalar_end..scalar_end + gap, Tok::Text));
        }
        if scalar_end + gap < line.len() {
            out.push((scalar_end + gap..line.len(), Tok::Comment));
        }
    }
}

fn classify(scalar: &str) -> Tok {
    match scalar {
        "true" | "false" | "null" | "~" => Tok::Literal,
        "[]" | "{}" => Tok::Punct,
        s if s.parse::<f64>().is_ok() => Tok::Literal,
        s if s.contains(sqyre_domain::PROGRAM_DELIMITER) => Tok::Ref,
        _ => Tok::Text,
    }
}

/// Push `text` (starting at byte `at`) as `tok`, splitting out `${var}` references.
fn push_with_vars(at: usize, text: &str, tok: Tok, out: &mut Vec<(Range<usize>, Tok)>) {
    let mut i = 0;
    while let Some(open) = text[i..].find("${") {
        let open = i + open;
        let Some(close) = text[open..].find('}').map(|c| open + c + 1) else {
            break;
        };
        if open > i {
            out.push((at + i..at + open, tok));
        }
        out.push((at + open..at + close, Tok::Var));
        i = close;
    }
    if i < text.len() {
        out.push((at + i..at + text.len(), tok));
    }
}

/// Syntax-coloured layout for the whole document.
fn layout_job(text: &str, palette: &Palette) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let content = line.strip_suffix('\n').unwrap_or(line);
        for (range, tok) in line_spans(content) {
            job.append(
                &text[offset + range.start..offset + range.end],
                0.0,
                palette.format(tok),
            );
        }
        if content.len() < line.len() {
            job.append("\n", 0.0, palette.format(Tok::Text));
        }
        offset += line.len();
    }
    job
}

/// [`egui::TextEdit::layouter`] that colours YAML with the app theme.
pub(super) fn yaml_layouter(
    ui: &egui::Ui,
) -> impl FnMut(&egui::Ui, &dyn egui::TextBuffer, f32) -> Arc<Galley> {
    let palette = Palette::new(ui);
    move |ui, buf, wrap_width| {
        let mut job = layout_job(buf.as_str(), &palette);
        job.wrap.max_width = wrap_width;
        ui.fonts_mut(|f| f.layout_job(job))
    }
}

/// Left text margin that fits line numbers for `text`.
pub(super) fn gutter_width(ui: &egui::Ui, text: &str) -> f32 {
    let digits = (text.matches('\n').count() + 1).to_string().len().max(2);
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let digit_w = ui.fonts_mut(|f| f.glyph_width(&font, '0'));
    digit_w * digits as f32 + SPACE_8 + SPACE_4
}

/// Paint line numbers into the editor's left margin (`gutter` wide);
/// `caret_line` (0-based) is drawn at full strength.
pub(super) fn paint_gutter(
    ui: &egui::Ui,
    output: &egui::text_edit::TextEditOutput,
    gutter: f32,
    caret_line: Option<usize>,
) {
    let rect = output.response.rect;
    let painter = ui.painter_at(rect);
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let visuals = ui.visuals();
    let line_x = rect.left() + gutter - SPACE_4;
    painter.vline(
        line_x,
        rect.y_range(),
        visuals.widgets.noninteractive.bg_stroke,
    );

    let mut line = 0;
    let mut starts_line = true;
    for row in &output.galley.rows {
        if starts_line {
            let y = output.galley_pos.y + row.pos.y;
            let color = if Some(line) == caret_line {
                visuals.text_color()
            } else {
                visuals.weak_text_color()
            };
            painter.text(
                egui::pos2(line_x - SPACE_4, y),
                egui::Align2::RIGHT_TOP,
                (line + 1).to_string(),
                font.clone(),
                color,
            );
            line += 1;
        }
        starts_line = row.ends_with_newline;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(line: &str) -> Vec<(&str, Tok)> {
        line_spans(line)
            .into_iter()
            .map(|(r, t)| (&line[r], t))
            .collect()
    }

    #[test]
    fn spans_cover_line_exactly() {
        for line in [
            "  - type: imagesearch  # find it",
            "    targets:",
            "    - Shop~Potion",
            "  text: \"hi ${name}\" # c",
            "# only a comment",
            "-",
            "",
        ] {
            let joined: String = toks(line).into_iter().map(|(s, _)| s).collect();
            assert_eq!(joined, line);
        }
    }

    #[test]
    fn keys_values_and_refs() {
        assert_eq!(
            toks("  - count: 5"),
            [
                ("  ", Tok::Text),
                ("- ", Tok::Punct),
                ("count", Tok::Key),
                (":", Tok::Punct),
                (" ", Tok::Text),
                ("5", Tok::Literal),
            ]
        );
        assert_eq!(toks("- Shop~Potion")[1], ("Shop~Potion", Tok::Ref));
        assert_eq!(toks("a: true")[3], ("true", Tok::Literal));
    }

    #[test]
    fn variables_and_comments() {
        let t = toks("text: go ${x} now # note");
        assert!(t.contains(&("${x}", Tok::Var)));
        assert_eq!(t.last(), Some(&("# note", Tok::Comment)));
        let q = toks("text: \"${a}b\"");
        assert!(q.contains(&("${a}", Tok::Var)));
        assert!(q.contains(&("b\"", Tok::Str)));
    }

    #[test]
    fn windows_paths_are_not_keys() {
        assert_eq!(toks("C:\\games\\x.exe")[0].1, Tok::Text);
        assert_eq!(toks("path: C:\\x")[0], ("path", Tok::Key));
    }
}
