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

Analyze **every commit** in scope before treating the work as done. Do this even when the user did not ask for a review.

Do not skip because the change is small, tests pass, or the commit message looks fine.

## When

| Situation | What to analyze |
|-----------|-----------------|
| User asked to commit, or you are about to create a commit | Working tree + index (`git diff`, `git diff --cached`) |
| Reviewing one commit | That commit (`git show <sha>`) |
| Branch, PR, or range | **Each commit** separately (`git log` then `git show` per sha) — not only the squash/`...` diff |
| You just finished implementing | Uncommitted diff, same as a pre-commit review |

If several commits are in scope, review them one by one. A clean merge diff can hide a messy intermediate commit.

## Required loop

1. Collect the exact patch (commands below). Read the full diff, not a summary.
2. Score it against **smells**, **refactor opportunities**, and **best practices** (this file + project rules).
3. Act:
   - **Your uncommitted work:** fix Block and Should issues in the hunks you introduced, then commit if still asked.
   - **Existing commits:** report findings. Do not rewrite history unless the user asked.
4. State a verdict. Silence is not a pass.

Pre-existing problems outside the commit's hunks: mention only if they block correctness; do not expand scope to clean the file.

## Collect the patch

```bash
git status --short
git diff
git diff --cached
```

Single commit:

```bash
git show --format=fuller --stat -p <sha>
```

Range (then show each sha):

```bash
git log --oneline <base>..HEAD
```

Ignore lockfile-only noise unless the lockfile change is inconsistent with the manifest.

## Analyze

Judge the **new** code and the **diff as a design choice** (what was left duplicated, what API leaked, what was papered over).

### Code smells

Flag when present in the patch:

- Duplication you could extract without a speculative rewrite
- Functions/types that gained a new responsibility this commit
- Boolean/`Option` soup, stringly modes, or magic values that should be an enum/newtype
- Dead code, unused params, commented-out remnants, leftover debug prints
- `unwrap` / `expect` outside tests or a documented unreachable path
- Swallowed errors (`let _ =`, empty `catch`, `ok()` with no reason)
- Compatibility shims, aliases, dual code paths, or “legacy” fallbacks (project forbids these)
- Broad `#[allow(…)]` / Clippy silences instead of a real fix
- Platform APIs behind `if cfg!(…)` instead of `#[cfg(…)]`
- New public names that encode an OS (`X11Capturer`) instead of `OsCapturer`-style

### Refactoring opportunities

Propose only changes that belong in **this** commit (or a follow-up the user would want now):

- Extract a function when the new logic is copy-pasted or the function is now doing two jobs
- Move logic to the crate that already owns that layer (`sqyre-domain` vs `sqyre-app`, capture vs UI)
- Replace one-off helpers with `From` / `TryFrom` / iterators / `?` when that is the local style
- Delete the old path entirely when behavior is replaced (no deprecation wrappers)
- Tighten visibility; stop leaking internals that this commit made public by accident

Skip drive-by refactors of untouched code. Skip rewrites that change behavior without tests.

### Best practices

Apply the workspace rules to the patch; do not re-litigate files the commit did not touch.

- **Rust:** `idiomatic-rust`, `rust-api-design`, `rust-clippy-safety` (`.cursor/rules/`)
- **Platform:** `#[cfg]` modules, OS imports scoped inside those blocks, one public capture/focus surface
- **Product:** no second language/stack; no dual codecs; no MSI/DMG/Flatpak unless asked
- **Unsafe:** every `unsafe` block has a `// SAFETY:` invariant; no unsafe to dodge the borrow checker
- **Errors:** library crates use `thiserror` types, not `String`/`anyhow`
- **Logging:** native paths use `sqyre_capture` diag macros / `crate::log::warn`, not `println!` / `tracing` / a new log crate
- **Tests:** behavior changes include or update tests when the crate already tests that layer; do not add tests that need a display in headless CI for capture/input bugs
- **Commit shape:** one purpose; generated junk and secrets stay out; message says *why*

## Verdict and output

**Block** — must fix before a new commit (or before calling a historical commit good): correctness, safety, shims, `cfg!` platform APIs, unexplained `unsafe`, swallowed errors, secrets.

**Should** — you introduced a smell or missed a local refactor; fix in this change when the patch is still yours.

**Note** — optional, pre-existing, or follow-up.

Clean commit:

```
Commit quality <sha or uncommitted>: pass
```

Issues:

```
Commit quality <sha or uncommitted>: fix before commit | issues found

Block:
- path:line — smell — why it matters

Should:
- path:line — refactor/practice — concrete change

Note:
- path:line — optional
```

Cite `path:line` from the diff. Recommend a specific edit, not a generic “consider cleaning this up.”

## Do not

- Approve a commit you have not diffed
- Treat Clippy/`cargo test` green as a smell review
- Nitpick naming or formatting that matches surrounding code
- Demand a rewrite unrelated to the hunks
- Block a commit solely on pre-existing issues you did not introduce
- Amend or force-push to “fix” historical commits unless the user asked and git safety rules allow it
