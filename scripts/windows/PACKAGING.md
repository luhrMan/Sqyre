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

| Path | Role |
|------|------|
| `.cache/cargo/` | Crate registry/git (or `.cargo-home` when Make exports that) |
| `target/x86_64-pc-windows-gnu/` | Incremental Cargo artifacts for the Windows triple |
| `.cache/sccache-windows/` | sccache rustc outputs (survives `cargo clean`) |

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
| `CARGO_INCREMENTAL` | Default `1` when sccache is off; must stay unset with sccache |
| `CARGO_FLAGS` | Extra cargo args |
| `SCCACHE_DIR` | Host cache dir under the repo (default `.cache/sccache-windows`) |

Output: `bin/sqyre.exe`.
