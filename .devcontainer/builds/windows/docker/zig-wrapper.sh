#!/bin/bash
# Wrapper that intercepts "zig cc" / "zig c++" and redirects to MinGW GCC.
#
# fyne-cross sets CC="zig cc -target x86_64-windows-gnu" inside its build
# scripts. We must intercept this because MSYS2 libraries (OpenCV, Tesseract,
# Leptonica) are built with GCC/libstdc++. Linking with zig/clang (libc++)
# causes undefined symbol errors at link time.
#
# This wrapper sits at /usr/local/bin/zig (the original zig is moved to
# /usr/local/bin/zig.real). When cgo invokes "zig cc <args>", we translate
# to "x86_64-w64-mingw32-gcc <args>" (filtering zig-specific flags).

subcmd="${1:-}"
shift 2>/dev/null || true

case "$subcmd" in
    cc)  compiler=x86_64-w64-mingw32-gcc-posix ;;
    c++) compiler=x86_64-w64-mingw32-g++-posix ;;
    *)
        # For non cc/c++ subcommands, fall through to real zig if available
        [[ -x /usr/local/bin/zig.real ]] && exec /usr/local/bin/zig.real "$subcmd" "$@"
        echo "zig wrapper: unknown command '$subcmd'" >&2
        exit 1
        ;;
esac

# Filter out zig-specific flags that MinGW GCC doesn't understand
args=()
while (( $# )); do
    case "$1" in
        -target)  shift ;;          # skip -target and its <triple> argument
        -fno-sanitize=*) ;;         # zig-specific, skip
        *)        args+=("$1") ;;
    esac
    shift
done

exec "$compiler" "${args[@]}"
