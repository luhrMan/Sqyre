#!/bin/sh
# MinGW g++ driver + rust-lld (as ld.lld). Bookworm GNU ld segfaults on Rust 1.92 COFF.
#
# rustc passes -nodefaultlibs on windows-gnu. After rustc's objects/libs, force-absorb
# MinGW C++/GCC/pthread into the binary (no libstdc++-6 / libgcc_s / libwinpthread DLLs):
#   1) static libstdc++.a (by filename — avoids libstdc++.dll.a)
#   2) static libpthread.a + libgcc (nanosleep/clock_gettime/__emutls_get_address)
#   3) dynamic mingwex + kernel32 (Win32 imports for those archives)
set -e
BDIR="${MINGW_LLD_BDIR:-/usr/local/libexec/mingw-rust-lld}"
LIBSTDCPP_DIR="${MINGW_LIBSTDCPP_DIR:-/usr/local/mingw-libstdcpp}"
exec x86_64-w64-mingw32-g++-posix -B"$BDIR" -fuse-ld=lld -static-libgcc -static-libstdc++ "$@" \
  -L"$LIBSTDCPP_DIR" \
  -Wl,-Bstatic \
  -l:libstdc++.a -l:libpthread.a -lgcc -lgcc_eh \
  -Wl,-Bdynamic \
  -lmingwex -lmoldname -lmingw32 -lm -lmsvcrt \
  -lkernel32 -luser32 -ladvapi32 -lshell32
