---
name: search-timing-consistency
description: >-
  Keep Sqyre Image Search, OCR, and Find Pixel extremely consistent and
  efficient — shared wait/retry shell, cache reuse, and stable timing steps.
  Use when editing search, match, OCR, find-pixel, wait-until-found, search
  cache, PreparedTemplate, SearchPrep, log_timing, capture_search_buf, or
  detection branches; when optimizing search latency; or when search kinds
  diverge in behavior or timings.
---

# Search timing consistency

Prioritize **identical control flow across detection kinds** and **no wasted work per attempt**. Searches are a hot path; inconsistency or redundant capture/match work is a product bug.

## Scope

| Layer | Path | Role |
|-------|------|------|
| Shared shell | `sqyre-executor/src/search/common.rs` | resolve → capture → wait/repeat → branch |
| Image | `…/search/image.rs` | template match (+ caches) |
| OCR | `…/search/ocr.rs` | preprocess + recognize |
| Pixel | `…/search/pixel.rs` | color/peak scan |
| Match engine | `sqyre-match` | `PreparedTemplate`, `SearchPrep`, FFT/direct |
| Cache | `sqyre-vision/src/search_cache.rs` | blurred template / mask / prepared |

Detection kinds: **Image Search**, **OCR**, **Find Pixel**. Treat them as one product surface.

## Consistency (non-negotiable)

1. **Route through `run_detection_shell`** — do not invent a private wait/repeat loop in one kind.
2. **Capture via `capture_search_buf`** — same miss policy: resolve/capture failure → log + empty/miss so wait can retry; do not abort the macro.
3. **Wait timeout → one final search** — after `wait_until_found` / `wait_while_found` times out, re-run `try_once` once (shell already does this). Keep that contract.
4. **Same defaults** — `DetectionCtx` wait/repeat intervals default to `100` ms unless the action config overrides via `WaitTilFoundConfig`.
5. **Backoff only in `retry_until`** — interval doubles up to `min(interval*5, 2000)`. Repeat loops use a fixed interval. Do not copy backoff into one kind only.
6. **`fresh` semantics** — Image Search always fresh-captures. OCR / Find Pixel: first attempt may crop cache (`fresh=false`); wait/repeat recaptures use `fresh=true`. Do not invert this without updating all kinds + tests.
7. **Interruptibility** — sleeps go through `interruptible_sleep`; check `stop_flag` / `check_stopped` on long match work. Never block stop on a search.
8. **Hit application** — outputs, highlights, and branch children go through `apply_detection_hits` / shared helpers. No kind-specific coordinate side effects.

When adding a search feature: implement once in `common.rs` (or shared match/cache APIs), then wire all three kinds. Divergent behavior needs an explicit comment **and** tests in `search/tests.rs` for each kind affected.

## Efficiency (hot path)

Every wait/repeat poll is a full attempt. Minimize work **per attempt** and **across attempts**.

| Do | Don't |
|----|-------|
| Reuse `get_cached_blurred_template` / `get_cached_image_mask` / `get_cached_prepared_template` | Reload/blur/prepare the same icon every poll |
| Build one `SearchPrep` per search frame (`prepare_search`) and pass it into all variant matches | Re-derive search prep per template variant |
| Match with `*_with_prepared` / preblurred APIs | Call raw `match_template` in the executor hot path |
| Parallelize independent variant jobs (`rayon`) when already the pattern | Serialize variant matches without a measured reason |
| `clear_search_cache()` when a macro run finishes (`app_run`) | Grow unbounded process cache across runs |
| Keep pipeline image clones behind `log_images_enabled()` | Clone full frames for logs on every attempt |
| Prefer smaller search areas / early exits on stop | Full-desktop search + ignore `stop_flag` mid-parallel |

`sqyre-match` performance budgets in unit tests are load-bearing — do not weaken them to hide regressions; fix the path (FFT vs direct, packing, prep reuse).

## Timing instrumentation

Use `exec.log_timing(action_id, step, elapsed)` / `timed_step` — never `println!` / `tracing` / ad-hoc diag spam on the search hot path.

**Stable step names** (keep strings identical so logs compare across runs):

| Kind | Steps |
|------|--------|
| Shared capture helper | `capture` (inside `capture_search_buf`) |
| Image | `capture+preprocess`, `match` |
| OCR | `preprocess`, `recognize` |
| Pixel | `scan` |
| Action wrapper | `dispatch`, `post-delay`, `total` (from `run.rs`) |

- Time the **meaningful phases**, not every micro-helper.
- Per-variant `match_ms` in image search is fine for detailed logs; the aggregate step remains `match`.
- Do not rename steps casually — that breaks log comparison and user muscle memory.

When investigating slow searches: read action-log timing lines first (see `debug-from-logs`), then profile the phase that dominates (`capture` vs `match` vs `recognize`).

## Change checklist

Before landing search-related edits:

- [ ] All affected kinds still share wait/repeat/timeout/final-attempt behavior
- [ ] No new per-attempt disk load / blur / prepare that the cache should own
- [ ] `SearchPrep` / `PreparedTemplate` still reused where variants share a frame
- [ ] Timing step names unchanged (or all call sites + docs updated together)
- [ ] `search/tests.rs` covers the behavior (wait retry, timeout final search, cache hit) when control flow or caching changes
- [ ] Stop/interrupt still responsive during match loops

## Anti-patterns

- Forking wait logic into `image.rs` / `ocr.rs` / `pixel.rs`
- Treating capture failure as hard `Err` that skips wait-until-found
- Skipping the post-timeout final search
- Re-blurring templates every poll
- Logging every frame/attempt with `diag::note` on the hot path
- Kind-specific interval/backoff “tuning” that makes one search feel different
- Adding a second cache beside `search_cache.rs`
