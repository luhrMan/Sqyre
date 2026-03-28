#!/bin/bash
# Entrypoint for Sqyre Windows cross-compile image.
#
# Forces MinGW GCC/G++ as C/C++ compilers, overriding any zig-based compiler
# that fyne-cross may inject via Docker's -e CC=... flags.
#
# The zig-wrapper.sh handles the deeper case where fyne-cross sets CC="zig cc"
# inside the shell command itself. This entrypoint is the first line of defense.

set -e

export CC=x86_64-w64-mingw32-gcc-posix
export CXX=x86_64-w64-mingw32-g++-posix
export CGO_ENABLED=1

# Prepend host MinGW C++ include dir to CGO_CXXFLAGS if available.
# This ensures <mutex>, <thread>, etc. resolve from the host's libstdc++
# rather than any mismatched MSYS2 sysroot headers.
if [ -f /etc/mingw-cxx-include-dir ]; then
    MINGW_CXX_INC=$(cat /etc/mingw-cxx-include-dir)
    if [ -n "$MINGW_CXX_INC" ] && [ -d "$MINGW_CXX_INC" ]; then
        export CGO_CXXFLAGS="-isystem ${MINGW_CXX_INC} ${CGO_CXXFLAGS}"
    fi
fi

exec "$@"
