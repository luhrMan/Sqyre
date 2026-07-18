//! Action type icon glyphs for the macro tree.

use crate::{Action, ActionKind};

pub fn action_icon_glyph(action: &Action) -> &'static str {
    match &action.kind {
        ActionKind::Click { state, .. } => {
            if *state {
                "⬇"
            } else {
                "⬆"
            }
        }
        ActionKind::Move { .. } => "➔",
        ActionKind::Key { state, .. } => {
            if *state {
                "⬇"
            } else {
                "⬆"
            }
        }
        ActionKind::Type { .. } => "⌨",
        ActionKind::Wait { .. } => "⏱",
        ActionKind::Pause { .. } => "⏸",
        ActionKind::FocusWindow { .. } => "👁",
        ActionKind::RunMacro { .. } => "▶",
        ActionKind::Conditional { .. } => "?",
        ActionKind::Loop { .. } | ActionKind::While { .. } => "↻",
        ActionKind::Break => "⏹",
        ActionKind::Continue => "⏭",
        ActionKind::SetVariable { .. } => "x",
        ActionKind::SaveVariable { .. } => "💾",
        ActionKind::ForEachRow { .. } => "☰",
        ActionKind::Ocr { .. } => "🔤",
        ActionKind::ImageSearch { .. } => "🔍",
        ActionKind::FindPixel { .. } => "🎨",
        ActionKind::NavigateSelect(_) => "⌖",
        ActionKind::NavigateKey { .. } => "⎇",
    }
}

/// True when the pill value looks like a `${name}` / `{name}` var ref.
pub fn looks_like_var_ref(text: &str) -> bool {
    let t = text.trim();
    (t.starts_with("${") && t.ends_with('}'))
        || (t.starts_with('{') && t.ends_with('}') && !t.starts_with("${"))
}
