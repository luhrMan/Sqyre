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
- Linker: `mingw-lld-link` (MinGW g++ driver + `rust-lld` via `-fuse-ld=lld`) — bookworm GNU ld segfaults on Rust 1.92 COFF
- Shipped `sqyre.exe` statically links MinGW `libstdc++` / `libgcc` / `winpthread` (no sidecar MinGW DLLs such as `libstdc++-6.dll`); OS DLLs like `msvcrt.dll` / `KERNEL32.dll` remain dynamic. The linker wrapper appends `-l:libstdc++.a` / `-l:libpthread.a` / `-lgcc` after rustc's `-nodefaultlibs` objects, with `CXXSTDLIB_x86_64_pc_windows_gnu=static=stdc++` for `link-cplusplus`.

First `make windows` on Linux builds the image (slow once). Later runs reuse it and cache crates under `.cache/cargo` (or `.cargo-home` when Make exports that). **Rebuild the image** after Dockerfile or `mingw-lld-link.sh` changes (`docker build -f scripts/windows/Dockerfile -t sqyre-windows-cross:latest scripts/windows`).

| Env | Role |
|-----|------|
| `SQYRE_WINDOWS_IMAGE` | Override image tag |
| `SQYRE_WINDOWS_FORCE_NATIVE=1` | Native Windows only (fail on Linux) |
| `CARGO_FLAGS` | Extra cargo args |

Output: `bin/sqyre.exe`.
