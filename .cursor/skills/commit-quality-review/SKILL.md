---
name: commit-quality-review
description: >-
  Analyze every git commit and staged/unstaged diff for code smells, refactoring
  opportunities, and project best-practice violations. Use when committing,
  creating a commit, reviewing commits, drafting or updating a PR, inspecting
  git diffs, finishing a code change that will be committed, or when the user
  mentions commit quality, code smells, or refactoring.
---

# Commit quality review

Review **every commit** in scope before treating work as done, even unasked and even for small changes. Green Clippy/tests are not a review.

## Scope

| Situation | Patch |
|-----------|-------|
| About to commit / just implemented | `git status --short`, `git diff`, `git diff --cached` |
| One commit | `git show --format=fuller --stat -p <sha>` |
| Branch / PR / range | `git log --oneline <base>..HEAD`, then `git show` **each** sha (a clean squash can hide a bad intermediate) |

Read the full diff. Ignore lockfile-only noise unless inconsistent with the manifest.

## What to check (in the hunks only)

Apply the rules rather than restating them: `rust-style`, `cross-platform`, `no-backwards-compatibility`, `git-identity`, and for UI `ui-usability` / `egui-window-size-ratchet`, for search `search-timing-consistency`.

Beyond the rules, flag:

- Duplication that can be extracted without speculation
- A function/type that gained a second responsibility
- Magic values / stringly modes that should be an enum or newtype
- Dead code, unused params, commented-out remnants, debug prints
- Logic in the wrong crate layer (domain vs app, capture vs UI)
- Visibility widened by accident
- Behavior change without a test when the crate already tests that layer (no display-dependent tests in headless CI; see `testing`)
- Commit shape: one purpose, no generated junk or secrets, message says *why*
- `Cargo.lock` changed without regenerated Flatpak `cargo-sources.json`

Skip drive-by refactors of untouched code and naming/format nits that match surroundings.

## Act

- **Your uncommitted work:** fix Block and Should items, then commit if asked.
- **Existing commits:** report only; do not rewrite history unless asked (except `git-identity` author/trailer fixes on unpushed commits).

## Verdict

- **Block** — correctness, safety, shims, `cfg!` platform APIs, unexplained `unsafe`, swallowed errors, secrets, attribution, weakened test gates (lowered coverage floor, loosened `perf_budget_secs`, new `#[ignore]`/nextest retries, deleted assertions without replacement).
- **Should** — smell or missed local refactor you introduced.
- **Note** — optional / pre-existing / follow-up.

```
Commit quality <sha|uncommitted>: pass
```

```
Commit quality <sha|uncommitted>: issues found
Block:
- path:line — issue — concrete fix
Should:
- path:line — issue — concrete fix
Note:
- path:line — optional
```

Silence is not a pass. Never approve an undiffed commit or block solely on pre-existing issues.
