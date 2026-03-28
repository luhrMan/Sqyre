#!/bin/bash
# Recursively find all DLL dependencies of a Windows PE executable
# and copy them from the MSYS2 sysroot. Skips Windows system DLLs.
#
# Usage: collect-dlls.sh <exe_path> <output_dir> [dll_search_dir]
# Runs inside the fyne-cross-windows Docker image where objdump is available.
set -e

EXE="$1"
OUTDIR="$2"
DLL_DIR="${3:-/usr/local/mingw64/bin}"
VISITED=$(mktemp)

if [ -z "$EXE" ] || [ -z "$OUTDIR" ]; then
    echo "Usage: $0 <exe_path> <output_dir> [dll_search_dir]" >&2
    exit 1
fi

mkdir -p "$OUTDIR"

# Windows system DLLs — always present on the target machine, never bundle.
is_system_dll() {
    case "$(echo "$1" | tr '[:upper:]' '[:lower:]')" in
        kernel32.dll|user32.dll|gdi32.dll|advapi32.dll|shell32.dll|\
        ole32.dll|oleaut32.dll|msvcrt.dll|ws2_32.dll|winmm.dll|\
        comctl32.dll|comdlg32.dll|shlwapi.dll|dwmapi.dll|uxtheme.dll|\
        opengl32.dll|imm32.dll|dnsapi.dll|iphlpapi.dll|wldap32.dll|\
        crypt32.dll|bcrypt.dll|secur32.dll|ncrypt.dll|setupapi.dll|\
        cfgmgr32.dll|version.dll|psapi.dll|mswsock.dll|ntdll.dll|\
        rpcrt4.dll|userenv.dll|wsock32.dll|normaliz.dll|d3d11.dll|\
        dxgi.dll|d3d9.dll|winspool.drv|powrprof.dll|dbghelp.dll|\
        d3dcompiler_47.dll|mfplat.dll|mf.dll|mfreadwrite.dll|\
        propsys.dll|avrt.dll|credui.dll|netapi32.dll|wtsapi32.dll|\
        hid.dll|winhttp.dll|cabinet.dll|wintrust.dll|msi.dll|\
        oleacc.dll|usp10.dll|msimg32.dll|comdlg32.dll)
            return 0 ;;
        api-ms-win-*|ext-ms-win-*)
            return 0 ;;
        *)
            return 1 ;;
    esac
}

# Recursively scan a PE binary for DLL imports and copy non-system ones.
scan_deps() {
    local target="$1"
    local deps
    deps=$(x86_64-w64-mingw32-objdump -p "$target" 2>/dev/null \
        | grep "DLL Name:" | awk '{print $3}') || true

    for dll in $deps; do
        local lower
        lower=$(echo "$dll" | tr '[:upper:]' '[:lower:]')

        is_system_dll "$lower" && continue
        grep -qxF "$lower" "$VISITED" 2>/dev/null && continue

        # Try exact name first, then lowercase
        local dll_path=""
        [ -f "$DLL_DIR/$dll" ]   && dll_path="$DLL_DIR/$dll"
        [ -z "$dll_path" ] && [ -f "$DLL_DIR/$lower" ] && dll_path="$DLL_DIR/$lower"
        [ -n "$dll_path" ] || continue

        echo "$lower" >> "$VISITED"
        cp "$dll_path" "$OUTDIR/"
        echo "  $dll"

        # Recurse into this DLL's own dependencies
        scan_deps "$dll_path"
    done
}

echo "Collecting DLL dependencies for $(basename "$EXE")..."
scan_deps "$EXE"

COUNT=$(wc -l < "$VISITED")
echo "Collected $COUNT DLLs into $OUTDIR/"
rm -f "$VISITED"
