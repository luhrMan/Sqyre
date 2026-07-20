#!/bin/sh
# MinGW g++ driver + rust-lld (as ld.lld). Bookworm GNU ld segfaults on Rust 1.92 COFF.
set -e
BDIR="${MINGW_LLD_BDIR:-/usr/local/libexec/mingw-rust-lld}"
exec x86_64-w64-mingw32-g++-posix -B"$BDIR" -fuse-ld=lld "$@"
