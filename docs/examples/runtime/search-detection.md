# Search detection (Image Search, OCR, Find Pixel)

## Story

Authors get one wait/branch story across detection kinds; searches stay region-first and skip wasted match work when the frame did not change.

## Rules

- Image Search, OCR, and Find Pixel share one wait / timeout / final-try / stop / hit-apply control flow (`run_detection_shell`).
- Capture or resolve failure is a soft miss (log + retry), not a hard abort of the macro.
- After a wait timeout, the shell runs one final search attempt.
- Author wait surface is **Once** | **Until appears** | **Until disappears**; repeat-as-loop is advanced (same shell).
- Blank detection actions default search area to `General~Monitor 1 Center Square` — a centered square covering 25% of monitor 1's area.
- After input that may change the screen, the next search may request a fresh frame (`mark_capture_dirty`); otherwise nested/clean captures may reuse cache. Do not fork per-kind “always fresh” stacks.
- Unchanged capture frame-token + rect reuses the last detection result (skip match/OCR/scan).
- Do not add a second wait/search architecture beside `run_detection_shell`; keep kinds as thin adapters.
- Identical **control flow** across kinds — not identical latency (OCR stays heavier).

## Examples

- the one where a blank Image Search / OCR / Find Pixel is created → search area is `General~Monitor 1 Center Square`  
  `Test: crates/sqyre-domain/src/blank.rs::blank_detection_kinds_seed_move_to_match`
- the one where General is seeded for 1920×1080 → `Monitor 1 Center Square` is the centered 720×720 (25% area) rect  
  `Test: crates/sqyre-persist/src/programs/seed_general.rs::seeds_single_monitor_geometry`
- the one where center-square side is derived from monitor size → area is exactly one quarter of `w·h`  
  `Test: crates/sqyre-persist/src/programs/seed_general.rs::center_quarter_square_covers_quarter_area`
- the one where wait/repeat polls see the same frame token + rect → prior hits are reused without re-matching  
  `Test: crates/sqyre-executor/src/search/common.rs::frame_memo_tests::reuses_result_when_token_and_rect_match`
- the one where Image Search wait-until arms after a miss → the shared shell logs wait intent once  
  `Test: crates/sqyre-executor/src/search/tests/image.rs::image_search_wait_until_found_retries_then_succeeds`
- the one where image-search edit opens → Wait mode is on the common path (not under tip Advanced)  
  `Test: crates/sqyre-app/tests/ui_interaction.rs::image_search_edit_shows_output_and_wait_without_advanced`

## Questions

_(none)_

## Out of scope

- Match / OCR / pixel algorithm internals and bench budgets (see skill)
- Template / SearchPrep cache eviction policy details (skill + vision tests)
- Portal / Wayland capture backend specifics

## Implementation note (not acceptance criteria)

Shared shell: `sqyre-executor/src/search/common.rs` (including the wait-intent log when until/while arms — kinds must not fork pre-shell copies). Efficiency how-to and measurement: [`.cursor/skills/search-timing-consistency/SKILL.md`](../../../.cursor/skills/search-timing-consistency/SKILL.md). Seeded geometry: `ensure_general_program` / `DEFAULT_SEARCH_AREA`.
