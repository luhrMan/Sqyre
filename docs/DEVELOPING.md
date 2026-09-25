# Developing Sqyre

## Dev container (recommended)

Open the repository in the dev container (`.devcontainer/`). It includes Rust 1.94, clang, Tesseract/Leptonica, X11 link deps, AppImage packaging tools (`appimage-builder`, squashfs-tools), **flatpak** + **flatpak-builder**, **Trunk** + `wasm32-unknown-unknown` (for `make wasm`), and the **Docker CLI** (host daemon via socket) so `make windows`, AppImage Docker fallbacks, and Flatpak Docker fallbacks work inside the container. Native Flatpak builds still need host user namespaces / `bwrap`; otherwise use the privileged Flatpak builder image via Docker.

Nested `docker run -v` mounts use the host path via `LOCAL_WORKSPACE_FOLDER` (`${localWorkspaceFolder}`). Rebuild the container after pulling that change so the env var is set.

If Cursor reports **“container is not running”** during attach, stale containers are usually the cause — remove them (`docker ps -a` → `docker rm -f <id>`) and **Rebuild Container**. Large `target/` trees are excluded from file watchers (see `.devcontainer/devcontainer.json`). Prefer `make clean-sweep` to keep `target/` (and `target-dhat/` when present) under ~20 GiB; use `cargo clean` only if you need a full cold rebuild.

### Host permissions / SELinux / git

On Fedora, Bazzite, and other SELinux-enforcing hosts, a plain bind mount of the repo often yields **Permission denied** on files and broken git (“dubious ownership”) inside the container. The devcontainer sets `--security-opt=label=disable`, remaps the `vscode` user to the host UID (`updateRemoteUserUID`), marks `/workspace` as a git `safe.directory`, and re-owns writable caches (`target/`, `.cache/`, cargo home) on each attach. Nested `make windows` / AppImage / Flatpak Docker binds use the `:z` SELinux label.

After pulling these changes, **Rebuild Container** (not just reopen). If `target/` or `.cache/` were created as root on another machine, delete them on the host or let `post-start.sh` chown them once they are unwritable.

From the repo root:

```bash
make            # ./bin/sqyre (debug)
make release    # fmt + check, then ./bin/sqyre (release)
make run        # cargo run -p sqyre-app
make check      # fmt --check + clippy (-D warnings) + cargo deny
make machete    # unused crate deps
make test       # cargo nextest (falls back to cargo test)
make bench      # criterion: match, vision (no Tesseract), serialize (not in CI)
make wasm-check # cargo check -p sqyre-app --target wasm32-unknown-unknown --no-default-features
make coverage   # llvm-cov HTML + lcov under target/coverage/
make coverage-floors  # line-% gates for pure crates (needs cargo-llvm-cov)
make clean-sweep # cargo-sweep target/ (+ target-dhat/) down to 20GB (SWEEP_MAXSIZE=…)
make docs-media # regenerate docs/images screenshots
make appimage   # fmt + check, then bin/*.AppImage (Linux)
make flatpak    # fmt + check, then bin/com.sqyre.app.flatpak (Linux; Docker fallback if needed)
make windows    # fmt + check, then bin/sqyre.exe (Docker MinGW cross / native on Windows)
make macos      # fmt + check, then bin/sqyre (macOS host)
make wasm       # fmt + check, then bin/wasm/ GUI-only browser editor (Trunk)
make tessdata   # download eng.traineddata into assets/tessdata/
make release-bundle  # portable bin/sqyre-bundle/ (dist/LTO shipping; no check gate — see scripts/linux/packaging/PACKAGING.md)
make dev            # fast prototype of release-bundle → bin/sqyre-dev/ (--release thin LTO)
make release-bundle-dhat  # same + dhat-heap → bin/sqyre-bundle-dhat/ (leak hunts; not for shipping)
```

Release builds (`make release`, `make release-bundle`, `make release-bundle-dhat`, `make dev`) need **≥4 GiB** container RAM on a cold `target/`; if rustc is SIGKILL'd, raise Docker memory or set `CARGO_BUILD_JOBS=1`. With a warm `target/` cache, `make release-bundle` / `make dev` are much faster and lighter than before (they no longer run clippy/deny first). Prefer `make dev` for day-to-day bundled iteration; `[profile.release]` uses thin LTO (default codegen-units), while `release-bundle` uses `[profile.dist]` (`codegen-units = 1`). Do not set `RUSTFLAGS='-C target-cpu=native'` for AppImage/Windows shipping artifacts. `release-bundle-dhat` uses a separate `target-dhat/` so it does not overwrite the normal release binary.

Run `make help` for the full target list. Workspace layout: [RUST.md](./RUST.md).

Build caches (all gitignored):

| Path | Role |
|------|------|
| `target/` | Incremental compile artifacts (host + docker bind-mount; Windows under `target/x86_64-pc-windows-gnu/`). Cap with `make clean-sweep` (~20 GB). |
| `target-dhat/` | Separate release artifacts for `make release-bundle-dhat` (avoids clobbering normal `target/release`). Also swept by `make clean-sweep`. |
| `.cargo-home/` | Optional workspace-local cargo/rustup install |
| `.cache/cargo/` | Cargo registry/git cache used by CI and docker AppImage / Windows builds |
| `.cache/sccache-linux/` | sccache rustc cache for Linux CI (`test` / `build-linux` / `build-wasm`) |
| `.cache/sccache-windows/` | sccache rustc cache for `make windows` (Linux/CI bind mount) |
| Docker volumes `sqyre-windows-*` | Windows cross cargo/target/sccache when the repo is on a Docker Desktop Windows path |
| Dev container volume `sqyre-cargo-home` | Persistent `/home/vscode/.cargo` in the container |

`make appimage` via Docker reuses `CARGO_HOME` when Make exports `.cargo-home`, otherwise `.cache/cargo`. `make windows` defaults to `CARGO_INCREMENTAL=1`; on Docker Desktop it stores cargo caches in Linux volumes (bind-mounted `target/` on a Windows host path is very slow). CI uses `SQYRE_WINDOWS_SCCACHE=1` instead. See [`scripts/windows/PACKAGING.md`](../scripts/windows/PACKAGING.md).

---

## Make targets

| Target | Output |
|--------|--------|
| `all` / `sqyre` | `bin/sqyre` (debug) — **default** |
| `release` | `bin/sqyre` (release) |
| `check-fmt` | `cargo fmt --all -- --check` |
| `fmt` | `cargo fmt --all` (write) |
| `clippy` | `cargo clippy --workspace --all-targets` (`-D warnings`) |
| `deny` | `cargo deny check` (licenses / advisories / bans / sources) |
| `machete` | `cargo machete` (unused dependencies) |
| `check` | `check-fmt` + `clippy` + `deny` (CI quality gates) |
| `test` | `cargo nextest run --workspace` (falls back to `cargo test`) |
| `smoke` | Debug `bin/sqyre --version` (no display) |
| `bench` | Criterion benches for `sqyre-match`, `sqyre-vision`, `sqyre-serialize` (local only; not CI) |
| `wasm-check` | `cargo check` of the GUI-only WASM editor (no Trunk) |
| `coverage` | llvm-cov nextest → HTML + `lcov.info` + `summary.json` under `target/coverage/` (no % gate) |
| `coverage-floors` | Line-coverage floors for pure crates (`sqyre-domain`, `sqyre-varref`, `path_confine`, `migrate`, `sqyre-serialize`, `sqyre-validate`, `sqyre-persist`, `sqyre-executor`; see `scripts/coverage-floors.json`) |
| `clean-sweep` | `cargo-sweep --maxsize 20GB` on `target/` (and `target-dhat/` if present); override with `SWEEP_MAXSIZE=` |
| `run` | `cargo run -p sqyre-app` |
| `docs-media` | Regenerate `docs/images/` screenshots |
| `appimage` | `bin/Sqyre-*.AppImage` |
| `flatpak` | `bin/com.sqyre.app.flatpak` |
| `release-bundle` | `bin/sqyre-bundle/` (portable Linux + Tesseract; dist/LTO; no check gate) |
| `dev` | `bin/sqyre-dev/` (same layout; `--release` / no LTO; fast prototyping) |
| `release-bundle-dhat` | `bin/sqyre-bundle-dhat/` (same + `dhat-heap`; local leak hunts only) |
| `windows` | `bin/sqyre.exe` (Docker MinGW cross on Linux; native on Windows) |
| `macos` | `bin/sqyre` (release; macOS host only) |
| `wasm` | GUI-only browser editor → `bin/wasm/` (Trunk; no Run/capture/OCR) |
| `tessdata` | Tesseract trained data via `scripts/download-tessdata.sh` |

Set `CARGO_FLAGS` for extra cargo args. Set `RELEASE_VERSION` (or write a `VERSION` file) before `make appimage` / `make flatpak` / `make release` / `make windows` to stamp package names and embed `SQYRE_VERSION` in the binary for auto-update checks (Flatpak disables in-app self-replace — use `flatpak update`). Local builds without either default to `0.0.0-dev` (update checks disabled).

### OCR data (`eng.traineddata`)

On native startup Sqyre resolves Tesseract English data, then **downloads** it if nothing usable is found:

| Platform | Auto-download location |
|----------|------------------------|
| Windows | `%APPDATA%\sqyre\tessdata\eng.traineddata` |
| Linux / macOS | `~/.sqyre/tessdata/eng.traineddata` |

Discovery order (earlier entries win): `SQYRE_TESSDATA`, platform system paths (and beside `sqyre.exe` on Windows), workspace `assets/tessdata` (dev), then the auto-download path above.

`make tessdata` still fills `assets/tessdata/` for packaging / CI. A failed download is a non-fatal warning in the app and on stderr; OCR actions stay unavailable until data can be found.

### WASM editor (`make wasm`)

Browser-only macro editor (import/export `db.yaml`). Does not run automation. The **dev container** already has Trunk and the `wasm32-unknown-unknown` target — rebuild the container after pulling those Dockerfile changes, then:

```bash
make wasm-check    # cargo check wasm32 editor (no Trunk; also run on Linux CI)
make wasm          # → bin/wasm/index.html  (deployable; use this, not trunk serve's dist)
cd crates/sqyre-app && env -u NO_COLOR trunk serve   # local preview + reload only
```

Serve the release output with any static file server (`python3 -m http.server` from `bin/wasm/`, etc.). Do **not** copy `dist/` from a running `trunk serve` — that injects an unreplaced autoreload WebSocket stub and floods the console.
On a bare host (no container), install once:

```bash
rustup target add wasm32-unknown-unknown
cargo install --locked trunk
```

Uses `--no-default-features` (no global hotkey hooks). Native `make` / `make release` are unchanged.

### CI and GitHub Releases

Push/PR to `main` runs Linux quality checks, an instrumented coverage/test pass (`make coverage` + floors), `make smoke` / `sqyre --version`, and `make wasm-check`, plus a Windows job that tests OS-agnostic crates (`sqyre-domain`, `sqyre-varref`, `sqyre-serialize`, `sqyre-validate`, `sqyre-persist`) — **not** a GitHub Release. Capture, hotkeys, and GPU UI tests stay Linux-only. Criterion benches (`make bench`) are local-only.

Releases come from [`.github/workflows/main.yml`](../.github/workflows/main.yml) on **schedule** or **manual dispatch** only:

| Trigger | When |
|---------|------|
| Cron | Daily at **23:00 UTC** (`0 23 * * *`) |
| Manual | Actions → **Build and Release** → Run workflow, or `gh workflow run "Build and Release" --ref main` |

The `version` job sets `should_release=true` only when there is no prior `v*` tag, or `main` has changed since the latest `v*` tag **excluding** `docs/**` and `*.md`. Docs-only / markdown-only changes do not publish. If nothing releasable changed, release jobs are skipped.

**Tag shape:** `vYYYY.MM.DD` (UTC date). If that tag already exists, CI uses `vYYYY.MM.DD.HHMM`.

**Artifacts:** Linux binary + AppImage + Flatpak (`com.sqyre.app.flatpak`), Windows `.exe` (MinGW cross via [`scripts/windows/`](../scripts/windows/PACKAGING.md)), and the WASM editor zip (`make wasm`). `make macos` stays native; MSI/DMG packaging is not shipped yet.

Shipped Linux AppImage/Windows builds embed `SQYRE_VERSION` so the in-app updater can compare against GitHub Releases (local `0.0.0-dev` builds skip update checks). Flatpak installs use `flatpak update` instead of in-app self-replace.

**Signed updates:** Releases must publish `SHA256SUMS` and `SHA256SUMS.sig` (Ed25519 over the exact `SHA256SUMS` bytes). The client verifies the signature with the public key in `crates/sqyre-update/update_pubkey.hex` before trusting hashes.

To configure signing (maintainer, once):

1. Generate a keypair (32-byte seed as hex), e.g. with Python `nacl` / `cryptography`, or any Ed25519 tool that can emit raw keys.
2. Write the **public** key as 64 lowercase hex characters into `crates/sqyre-update/update_pubkey.hex` and commit it.
3. Store the **private** seed hex as GitHub Actions secret `SQYRE_UPDATE_SIGNING_KEY` (never commit it). CI runs `sign_update_sums` during the release job.

Until `update_pubkey.hex` is configured (not `UNCONFIGURED`), auto-update checks fail closed on signature setup.

CI caches (shared where possible):

| Cache | Shared by | Notes |
|-------|-----------|--------|
| Cargo registry (`.cache/cargo`) | All Linux jobs | One key on `Cargo.lock`; warmed once per run via `cargo-registry` |
| Cargo `target/` | Per triple | `linux` / `windows-gnu` / `wasm32` / macOS — not cross-shareable |
| Linux sccache (`.cache/sccache-linux`) | `test`, `build-linux`, `build-wasm` | rustc outputs across clippy / coverage / release / wasm |
| Linux build image | `test`, `build-linux`, `build-wasm` | Content-hash tag on GHCR + Buildx layers; built once per run |
| Windows cross image | `build-windows` (+ local `make windows`) | Content-hash tag on GHCR + Buildx layers |
| Windows sccache | `build-windows` | Separate from Cargo target and Linux sccache |
| tessdata / Homebrew | Across runs | Stable keys |

Runnable images are tagged `ghcr.io/<owner>/<repo>-linux-build:<dockerfile-hash>` and `…-windows-cross:<scripts-hash>` (also `:latest`).

---

## Native dependencies

| Resource | Purpose |
|----------|---------|
| [.devcontainer/Dockerfile](../.devcontainer/Dockerfile) | Rust + Tesseract + AppImage/Flatpak tools + Trunk/wasm32 |
| [.devcontainer/devcontainer.json](../.devcontainer/devcontainer.json) | Docker-outside-of-Docker (CLI + host socket) for `make windows` / packaging fallbacks |
| [scripts/windows/Dockerfile](../scripts/windows/Dockerfile) | MinGW cross image for `make windows` on Linux |
| [crates/sqyre-app/assets/icons/](../crates/sqyre-app/assets/icons/) | Brand icons (embedded SVG) |
| [assets/tessdata/](../assets/tessdata/) | Optional local `eng.traineddata` fallback |

OCR uses system tessdata when available, or `SQYRE_TESSDATA` / `assets/tessdata` when developing. If none are found, startup downloads `eng.traineddata` into the user data path (see above).

---

## Manual setup (without dev container)

Prefer the container when possible. Needs **Rust ≥ 1.92**, clang, Tesseract/Leptonica, and X11 libs (`libx11-dev`, `libxtst-dev`, …). See [RUST.md](./RUST.md).

```bash
make            # or: cargo build -p sqyre-app
./bin/sqyre
```

For AppImage on the host, also install `appimage-builder`, `patchelf`, and `squashfs-tools`. For Flatpak, install `flatpak`, `flatpak-builder`, and `ostree` (and ensure user namespaces / bwrap work), or use the Docker fallback in `scripts/linux/packaging/flatpak/build-flatpak.sh`.

---

## Tests

```bash
make test
# or: cargo test
```

Headless CI uses Null* backends / stub hotkeys where hooks are unavailable.

### Coverage

`make coverage` instruments the full workspace (nextest when available), runs tests once, and writes HTML + LCOV + JSON under `target/coverage/` with **no** percentage gate. Requires `cargo-llvm-cov` and `llvm-tools-preview` (preinstalled in the dev container). CI uses this instead of a separate `make test` so the suite is not compiled twice. Normal local `make test` does **not** need these tools.

`make coverage-floors` fails when line coverage drops below the floors in [`scripts/coverage-floors.json`](../scripts/coverage-floors.json):

| Target | Scope | Default floor |
|--------|-------|---------------|
| `sqyre-domain` | whole crate | 83% |
| `sqyre-varref` | whole crate | 93% |
| `path_confine` | `sqyre-executor` path confinement | 90% |
| `migrate` | `sqyre-persist` db.yaml migration | 83% |
| `sqyre-serialize` | whole crate | 93% |
| `sqyre-validate` | whole crate | 84% |
| `sqyre-persist` | whole crate | 77% |
| `sqyre-executor` | whole crate | 85% |

OS-specific crates (`sqyre-capture`, etc.) are intentionally **not** gated. By default floors run a smaller instrumented pass over pure crates only. After `make coverage`, pass `COVERAGE_REPORT_JSON=target/coverage/summary.json` to reuse that report (no second instrumented build) — CI does this. Set `COVERAGE_FLOORS=1` when running `make coverage` to run the floor check automatically after the report.

### README screenshots

In-memory egui goldens live under `docs/images/` (test: `cargo test -p sqyre-app --test docs_screenshots`):

| Asset | Surface |
|-------|---------|
| `main-window.png` | Macro tree + Macros sidebar |
| `add-action-picker.png` | Add Action catalog |
| `data-editor.png` | Data Editor → Coordinates / Points |
| `settings.png` | Settings → Appearance (sidebar layout) |
| `command-palette.png` | Ctrl+K command palette |

```bash
make docs-media
# or: SQYRE_UPDATE_SCREENSHOTS=1 ./scripts/generate-docs-media.sh
```

Needs wgpu (lavapipe in the dev container / CI image).

---

## Packaging

See [scripts/linux/packaging/PACKAGING.md](../scripts/linux/packaging/PACKAGING.md) for AppImage and Flatpak builds.
