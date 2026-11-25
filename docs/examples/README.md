# Example Maps

Behavioral source of truth for Sqyre features (UI and runtime). Agents and humans use these maps instead of long scenario prose in `.cursor/rules`.

## Card legend

| Section | Meaning |
|---------|---------|
| **Story** | One-line goal for the theme |
| **Rules** | Accepted constraints / acceptance criteria |
| **Examples** | Concrete *the one where…* scenarios; each cites a `Test:` |
| **Questions** | Unresolved decisions (block or batch-ask before coding) |
| **Out of scope** | Explicit non-goals |

Rules may stand without examples when the constraint is obvious. Every **Example** must cite a real test `fn` that enforces it. New greens require a new or updated test in the same change.

## When to use

- New or changed UI / feature work: open or update the relevant map under [`ui/`](ui/) or [`runtime/`](runtime/) **before** coding.
- Underspecified requests: treat missing rules/examples as **Questions**; batch-ask (see `ask-on-ambiguity`).
- Implementation: helpers and thin `.mdc` rules point here; skills teach how (tests, benches), not a second copy of acceptance criteria.
- Usability index: [`ui/usability.md`](ui/usability.md) — pick a story map, don’t treat the index as acceptance criteria.

## UI maps

### Interaction / layout

| Theme | Map |
|-------|-----|
| Nested parent popups / menus | [ui/nested-popups.md](ui/nested-popups.md) |
| Hover tip ratchet + hug content | [ui/tooltip-ratchet.md](ui/tooltip-ratchet.md) |
| Elided labels disclose on hover | [ui/elided-tooltips.md](ui/elided-tooltips.md) |
| Live top surface owns keys | [ui/focus-layers.md](ui/focus-layers.md) |
| Recording capture owns keys | [ui/recording-input.md](ui/recording-input.md) |
| Search-first surfaces focus filter on open | [ui/search-on-open.md](ui/search-on-open.md) |
| Dialogs do not runaway-grow | [ui/dialog-layout.md](ui/dialog-layout.md) |
| Cell selection strokes stay inside | [ui/cell-strokes.md](ui/cell-strokes.md) |
| Layout chrome write-back | [ui/persisted-ui.md](ui/persisted-ui.md) |

### Product UX

| Theme | Map |
|-------|-----|
| Usability index (links only) | [ui/usability.md](ui/usability.md) |
| Lists sort / filter / empty | [ui/list-search-sort.md](ui/list-search-sort.md) |
| Progressive disclosure (Advanced) | [ui/progressive-disclosure.md](ui/progressive-disclosure.md) |
| Icon chrome + AccessKit names | [ui/icon-chrome.md](ui/icon-chrome.md) |
| Preference settings save / nav / search | [ui/settings-preferences.md](ui/settings-preferences.md) |
| Variable-ref fields, autocomplete, pills | [ui/var-ref-fields.md](ui/var-ref-fields.md) |
| Destructive remove confirms + Delete-key hygiene | [ui/destructive-confirm.md](ui/destructive-confirm.md) |
| Overwrite / migrate / restore confirms | [ui/choice-confirms.md](ui/choice-confirms.md) |

## Runtime maps

| Theme | Map |
|-------|-----|
| Image Search / OCR / Find Pixel wait + region defaults | [runtime/search-detection.md](runtime/search-detection.md) |

## Discovery note

Maps encode confirmed Q&A plus current tested contracts. Amend when product intent differs; leave open items under **Questions**.
