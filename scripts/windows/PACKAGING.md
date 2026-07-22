# Windows packaging / cross-build

Sqyre ships a bare `sqyre.exe` (no MSI). Build it with:

```bash
make windows          # Docker MinGW cross from Linux/macOS; native on Windows
```

## Cross image

[`Dockerfile`](./Dockerfile) (based on the former Go fyne-cross Windows image) produces `sqyre-windows-cross:latest` with:

- Debian bookworm + MinGW-w64 posix
- Rust `1.92` + `x86_64-pc-windows-gnu` (plus `rustfmt`/`clippy` to match `rust-toolchain.toml`)
- Static zlib / libpng / libjpeg-turbo / leptonica / tesseract under `/usr/local/mingw64-static`
- `sccache` available in-image (`sccache-rustc-wrapper`); enabled in CI via `SQYRE_WINDOWS_SCCACHE=1`
- Linker: `mingw-lld-link` (MinGW g++ driver + `rust-lld` via `-fuse-ld=lld`) — bookworm GNU ld segfaults on Rust 1.92 COFF
- Shipped `sqyre.exe` statically links MinGW `libstdc++` / `libgcc` / `winpthread` (no sidecar MinGW DLLs such as `libstdc++-6.dll`); OS DLLs like `msvcrt.dll` / `KERNEL32.dll` remain dynamic. The linker wrapper appends `-l:libstdc++.a` / `-l:libpthread.a` / `-lgcc` after rustc's `-nodefaultlibs` objects, with `CXXSTDLIB_x86_64_pc_windows_gnu=static=stdc++` for `link-cplusplus`.

### Image reuse

1. Reuse a local `sqyre-windows-cross:latest` that already includes `sccache`.
2. Else try `docker pull` of `SQYRE_WINDOWS_REGISTRY_IMAGE`, or `ghcr.io/<owner>/<repo>-windows-cross:latest` (from `GITHUB_REPOSITORY` / `origin`).
3. Else `docker build` (slow once: MinGW Tesseract).

CI pushes the runnable image to GHCR on main. **Rebuild the local image** after Dockerfile or `mingw-lld-link.sh` changes (`docker build -f scripts/windows/Dockerfile -t sqyre-windows-cross:latest scripts/windows`), or delete the tag so the next `make windows` pulls/rebuilds.

### Compile caches

| Path / volume | Role |
|---------------|------|
| `.cache/cargo/` | Crate registry/git (Linux/CI bind mount; or `.cargo-home` when Make exports that) |
| `target/x86_64-pc-windows-gnu/` | Incremental Cargo artifacts (Linux/CI bind mount) |
| `.cache/sccache-windows/` | sccache rustc outputs when sccache is enabled (Linux/CI) |
| Docker volumes `sqyre-windows-{target,cargo,sccache}` | Same caches on **Docker Desktop** (Windows host path) — bind mounts there make incremental rebuilds very slow |

On Docker Desktop, the first build after this change fills the Linux volumes (cold once); later incremental runs should be much faster. Override with `SQYRE_WINDOWS_BIND_CACHE=1` to force workspace bind mounts, or set `SQYRE_WINDOWS_TARGET_VOLUME` / `SQYRE_WINDOWS_CARGO_VOLUME` / `SQYRE_WINDOWS_SCCACHE_VOLUME`.

Release builds default to **`CARGO_INCREMENTAL=1`** (best for warm `target/` locally). CI sets `SQYRE_WINDOWS_SCCACHE=1` instead (sccache rejects `CARGO_INCREMENTAL` when that variable is set at all, even to `0`):

```bash
make windows                       # incremental (default)
SQYRE_WINDOWS_SCCACHE=1 make windows   # sccache (CI / cold caches)
```

| Env | Role |
|-----|------|
| `SQYRE_WINDOWS_IMAGE` | Override local image tag |
| `SQYRE_WINDOWS_REGISTRY_IMAGE` | Override GHCR (or other) pull ref |
| `SQYRE_WINDOWS_SKIP_PULL=1` | Never pull; build locally if needed |
| `SQYRE_WINDOWS_FORCE_NATIVE=1` | Native Windows only (fail on Linux) |
| `SQYRE_WINDOWS_SCCACHE=1` | Enable sccache (disables incremental) |
| `SQYRE_WINDOWS_BIND_CACHE=1` | Force bind-mount caches even on Windows Docker paths |
| `SQYRE_WINDOWS_TARGET_VOLUME` | Docker volume name for target (default `sqyre-windows-target`) |
| `SQYRE_WINDOWS_CARGO_VOLUME` | Docker volume name for cargo home (default `sqyre-windows-cargo`) |
| `SQYRE_WINDOWS_SCCACHE_VOLUME` | Docker volume name for sccache (default `sqyre-windows-sccache`) |
| `CARGO_INCREMENTAL` | Default `1` when sccache is off; must stay unset with sccache |
| `CARGO_FLAGS` | Extra cargo args |
| `SCCACHE_DIR` | Host cache dir under the repo (default `.cache/sccache-windows`; unused when cache volumes are on) |

Output: `bin/sqyre.exe`.
