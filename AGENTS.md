# Agent guide

Sqyre is a Rust Cargo workspace (egui desktop automation app + WASM editor); the repo may still be named Squire. Full workflow: [`docs/DEVELOPING.md`](docs/DEVELOPING.md). Do not add a second implementation language or dual-build stack.

## Layout

- `crates/sqyre-app` — GUI binary → `./bin/sqyre`; brand icons in `crates/sqyre-app/assets/`
- Libs: `sqyre-domain`, `-ui-model`, `-ui-theme`, `-serialize`, `-persist`, `-validate`, `-varref`, `-match`, `-vision`, `-ports`, `-capture`, `-executor`, `-hotkeys`, `-input`, `-overlay`, `-update`
- Android: `sqyre-android` (only JNI crate) + Kotlin shell in `android/` → `make android`; see [`docs/ANDROID.md`](docs/ANDROID.md)
- Tools: `sqyre-probe` (capability probe → `./bin/sqyre-probe`), `sqyre-bench-compare` (local bench harness)
- tessdata: `assets/tessdata/` (fetched by `make tessdata`, not committed)

## Build

- `make` (debug) / `make release` → `./bin/sqyre`; `make dev` = release opts without LTO; `make help` lists all targets.
- Release artifacts: `make appimage` / `flatpak` / `windows` / `wasm` / `android` (devcontainer must keep these working).
- Linux `make` builds add `--features portal-capture` (Wayland); plain `cargo build/clippy -p sqyre-app` skips that code.
- Toolchain pinned in `rust-toolchain.toml`; edition 2021. New shared deps go in `[workspace.dependencies]` (`default-features = false` when defaults are heavy) and must pass `cargo deny` and `make machete`. Features unify workspace-wide — native-only deps via target `cfg`.

## Essentials

- Done means `make fmt && make check && make test` pass on the tip (plus `make wasm-check` when touching `sqyre-app` or crates it uses).
- No backwards compatibility: change APIs and call sites directly; no shims, aliases, or dual code paths.
- Platform code uses `#[cfg(...)]` modules, never platform APIs inside `if cfg!(...)`. One OS-neutral public surface (`OsCapturer`, `OsWindowFocuser`).
- Library errors use `thiserror`; logging uses `sqyre_capture` diag / executor logs, not `println!` / `tracing`.
- Headless CI: tests must not need a display, portal, or `/dev/input`.
- Git: author/committer is the repo owner; no AI/tool attribution, trailers, or co-authors in commits, branches, or PRs.

## Detailed guidance

Rules live in [`.cursor/rules/`](.cursor/rules/), skills in [`.cursor/skills/`](.cursor/skills/):

| Topic | File |
|-------|------|
| Ask before consequential guesses | `ask-on-ambiguity.mdc` |
| No compat shims | `no-backwards-compatibility.mdc` |
| Git identity (local, gitignored) | `git-identity.mdc` |
| Platforms, `#[cfg]`, features, FFI | `cross-platform.mdc` |
| Rust style, errors, unsafe, logging | `rust-style.mdc` |
| Quality gate | `verify-new-code.mdc` |
| Tests, coverage floors, perf budgets | `testing.mdc` |
| WASM editor constraints | `wasm-editor.mdc` |
| egui dialog sizing | `egui-window-size-ratchet.mdc` |
| UI usability/consistency | `ui-usability.mdc` |
| Devcontainer release parity | `devcontainer-release-parity.mdc` |
| Debugging from logs | `skills/debug-from-logs` |
| Wayland/desktop parity, probe | `skills/linux-desktop-parity` |
| Search/OCR/pixel hot path | `skills/search-timing-consistency` |
| Commit review | `skills/commit-quality-review` |
