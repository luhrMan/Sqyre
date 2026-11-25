# Example Maps

Behavioral source of truth for Sqyre features — especially UI. Agents and humans use these maps instead of long scenario prose in `.cursor/rules`.

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

- New or changed UI / feature work: open or update the relevant map under [`ui/`](ui/) **before** coding.
- Underspecified requests: treat missing rules/examples as **Questions**; batch-ask (see `ask-on-ambiguity`).
- Implementation: helpers and thin `.mdc` rules point here; the [`egui-interaction-testing`](../../.cursor/skills/egui-interaction-testing/SKILL.md) skill is how to write the tests, not a second copy of the criteria.

## UI maps

| Theme | Map |
|-------|-----|
| Nested parent popups / menus | [ui/nested-popups.md](ui/nested-popups.md) |
| Hover tip / Area ratchet | [ui/tooltip-ratchet.md](ui/tooltip-ratchet.md) |
| Elided labels disclose on hover | [ui/elided-tooltips.md](ui/elided-tooltips.md) |
| Live top surface owns keys | [ui/focus-layers.md](ui/focus-layers.md) |
| Search-first surfaces focus filter on open | [ui/search-on-open.md](ui/search-on-open.md) |
| Layout chrome write-back | [ui/persisted-ui.md](ui/persisted-ui.md) |
| Preference settings (save / nav / search / understand) | [ui/settings-preferences.md](ui/settings-preferences.md) |
| Dialog / layout sizing | [ui/dialog-layout.md](ui/dialog-layout.md) |
| Lists / search / icons / disclosure | [ui/usability.md](ui/usability.md) |

## Discovery note

Initial maps encode the **current tested contracts** in `sqyre-app` (helpers + headless tests). Amend via conversation when product intent differs; leave open items under **Questions**.
