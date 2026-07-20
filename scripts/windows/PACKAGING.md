# Windows packaging / cross-build

Sqyre ships a bare `sqyre.exe` (no MSI). Build it with:

```bash
make windows          # Docker MinGW cross from Linux/macOS; native on Windows
```

## Cross image

[`Dockerfile`](./Dockerfile) (based on the former Go fyne-cross Windows image) produces `sqyre-windows-cross:latest` with:

- Debian bookworm + MinGW-w64 posix
- Rust `1.92` + `x86_64-pc-windows-gnu`
- Static zlib / libpng / libjpeg-turbo / leptonica / tesseract under `/usr/local/mingw64-static`
- Linker: `mingw-lld-link` (MinGW g++ driver + `rust-lld` via `-fuse-ld=lld`) — bookworm GNU ld segfaults on Rust 1.92 COFF

First `make windows` on Linux builds the image (slow once). Later runs reuse it and cache crates under `.cache/cargo` (or `.cargo-home` when Make exports that).

| Env | Role |
|-----|------|
| `SQYRE_WINDOWS_IMAGE` | Override image tag |
| `SQYRE_WINDOWS_FORCE_NATIVE=1` | Native Windows only (fail on Linux) |
| `CARGO_FLAGS` | Extra cargo args |

Output: `bin/sqyre.exe`.
