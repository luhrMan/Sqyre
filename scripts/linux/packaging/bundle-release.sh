#!/usr/bin/env bash
# Build a portable Linux release directory with Tesseract/Leptonica bundled.
#
# Output layout (under bin/sqyre-bundle/):
#   sqyre          — release binary ($ORIGIN/lib rpath)
#   lib/           — libtesseract, libleptonica, and transitive .so deps
#   tessdata/      — eng.traineddata
#
# Run from repo root: make release-bundle
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/repo-root.sh
. "$SCRIPT_DIR/../../lib/repo-root.sh"

have_cmd() { command -v "$1" >/dev/null 2>&1; }

need_cmd() {
  if ! have_cmd "$1"; then
    echo "ERROR: $1 not found (required for release-bundle)." >&2
    exit 1
  fi
}

need_cmd cargo
need_cmd patchelf
need_cmd ldd
need_cmd readlink
need_cmd file

TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
BUNDLE_DIR="$REPO_ROOT/bin/sqyre-bundle"
LIB_DIR="$BUNDLE_DIR/lib"
TESS_DIR="$BUNDLE_DIR/tessdata"
BINARY_SRC="$TARGET_DIR/release/sqyre"
BINARY_DST="$BUNDLE_DIR/sqyre"

# Prefer workspace-local rustup/cargo when present.
if [ -z "${CARGO_HOME:-}" ] && [ -x "$REPO_ROOT/.cargo-home/bin/cargo" ]; then
  export CARGO_HOME="$REPO_ROOT/.cargo-home"
  export PATH="$CARGO_HOME/bin:$PATH"
fi
if [ -z "${RUSTUP_HOME:-}" ] && [ -d "$REPO_ROOT/.rustup-home" ]; then
  export RUSTUP_HOME="$REPO_ROOT/.rustup-home"
fi

SQYRE_APP_FEATURES="--features portal-capture"

echo "Building release binary…"
(
  cd "$REPO_ROOT"
  cargo build -p sqyre-app --release $SQYRE_APP_FEATURES ${CARGO_FLAGS:-}
)

if [ ! -x "$BINARY_SRC" ]; then
  echo "ERROR: missing $BINARY_SRC after cargo build" >&2
  exit 1
fi

rm -rf "$BUNDLE_DIR"
mkdir -p "$LIB_DIR" "$TESS_DIR"

cp -f "$BINARY_SRC" "$BINARY_DST"
chmod 755 "$BINARY_DST"

# Core glibc / dynamic linker — leave to the host.
is_system_lib() {
  case "$(basename "$1")" in
    ld-linux*.so.*|linux-vdso.so.*|libc.so.*|libm.so.*|libpthread.so.*|libdl.so.*|librt.so.*|libresolv.so.*|libutil.so.*)
      return 0
      ;;
  esac
  return 1
}

# PipeWire / ALSA must match the host plugin tree (portal capture + cue audio).
is_host_audio_stack_lib() {
  case "$(basename "$1")" in
    libpipewire*.so.*|libspa-*.so.*|libasound*.so.*)
      return 0
      ;;
  esac
  return 1
}

# Copy a shared library and any same-directory version symlinks (soname chain).
copy_lib_family() {
  local resolved="$1"
  [ -e "$resolved" ] || return 0
  resolved="$(readlink -f "$resolved")"
  local libdir
  libdir="$(dirname "$resolved")"
  local base="${resolved##*/}"
  # Strip trailing .so.* to find the family prefix (libfoo.so).
  local prefix="${base%%.so*}.so"

  shopt -s nullglob
  for candidate in "$libdir/$prefix"*; do
    local name="${candidate##*/}"
    [ -e "$LIB_DIR/$name" ] && continue
    cp -a "$candidate" "$LIB_DIR/"
  done
  shopt -u nullglob
}

declare -A SEEN=()

add_deps_from() {
  local obj="$1"
  [ -f "$obj" ] || return 0

  local dep resolved name
  while IFS= read -r dep; do
    [ -n "$dep" ] || continue
    [ -f "$dep" ] || continue
    is_system_lib "$dep" && continue
    is_host_audio_stack_lib "$dep" && continue
    resolved="$(readlink -f "$dep")"
    name="$(basename "$dep")"
    [ -n "${SEEN[$name]:-}" ] && continue
    SEEN[$name]=1
    copy_lib_family "$resolved"
    add_deps_from "$resolved"
  done < <(ldd "$obj" 2>/dev/null | awk '/=> \// { print $3; next } /^\// { print $1 }')
}

add_deps_from "$BINARY_DST"

# Explicit OCR stack (ldd may not walk indirect deps on all distros).
for pattern in libtesseract libleptonica liblept; do
  for libdir in /usr/lib64 /usr/lib/x86_64-linux-gnu /usr/lib; do
    shopt -s nullglob
    for lib in "$libdir/$pattern.so"*; do
      [ -f "$lib" ] || [ -L "$lib" ] || continue
      copy_lib_family "$(readlink -f "$lib")"
      add_deps_from "$(readlink -f "$lib")"
    done
    shopt -u nullglob
  done
done

# Image/archive deps Tesseract often pulls (same list as AppImage recipe).
for dep in libjpeg libpng libtiff libjbig libwebp libdeflate libgif libarchive libbz2 libzstd liblz4 libcurl libnghttp2 libssh2 libpsl libidn2 libunistring libbrotlidec libbrotlicommon libgomp; do
  for libdir in /usr/lib64 /usr/lib/x86_64-linux-gnu /usr/lib; do
    shopt -s nullglob
    for lib in "$libdir/$dep.so"*; do
      [ -f "$lib" ] || [ -L "$lib" ] || continue
      copy_lib_family "$(readlink -f "$lib")"
    done
    shopt -u nullglob
  done
done

patchelf --force-rpath --set-rpath '$ORIGIN/lib' "$BINARY_DST"

# Bundle English tessdata.
eng=""
for d in "$REPO_ROOT/assets/tessdata" /usr/share/tesseract-ocr/*/tessdata /usr/share/tessdata /usr/local/share/tessdata; do
  if [ -f "$d/eng.traineddata" ]; then
    eng="$d/eng.traineddata"
    break
  fi
done
if [ -z "$eng" ]; then
  echo "eng.traineddata not found; downloading…"
  "$REPO_ROOT/scripts/download-tessdata.sh"
  eng="$REPO_ROOT/assets/tessdata/eng.traineddata"
fi
if [ ! -f "$eng" ]; then
  echo "ERROR: eng.traineddata not found; run: make tessdata" >&2
  exit 1
fi
cp -f "$eng" "$TESS_DIR/"

# Transitive deps (e.g. libtesseract → liblept) do not inherit the
# executable RUNPATH; each bundled .so needs $ORIGIN so its NEEDED chain
# resolves inside lib/.
is_elf_shared() {
  file -b "$1" 2>/dev/null | grep -qE 'ELF .*shared object'
}

while IFS= read -r -d '' elf; do
  is_elf_shared "$elf" || continue
  patchelf --force-rpath --set-rpath '$ORIGIN' "$elf"
done < <(find "$LIB_DIR" -maxdepth 1 -type f -print0)

# Smoke-check: bundled libs satisfy NEEDED entries on the binary and bundled .so files.
missing=""
check_needed() {
  local elf="$1"
  local needed
  while IFS= read -r needed; do
    [ -n "$needed" ] || continue
    is_system_lib "$needed" && continue
    is_host_audio_stack_lib "$needed" && continue
    case "$needed" in
      libstdc++.so.*|libgcc_s.so.*|libgomp.so.*) continue ;;
    esac
    if [ ! -e "$LIB_DIR/$needed" ]; then
      missing="${missing:+$missing, }$needed (from $(basename "$elf"))"
    fi
  done < <(patchelf --print-needed "$elf" 2>/dev/null || true)
}

check_needed "$BINARY_DST"
while IFS= read -r -d '' elf; do
  is_elf_shared "$elf" || continue
  check_needed "$elf"
done < <(find "$LIB_DIR" -maxdepth 1 -type f -print0)
if [ -n "$missing" ]; then
  echo "ERROR: bundle missing libraries: $missing" >&2
  exit 1
fi

echo "Bundled release: $BUNDLE_DIR"
echo "  sqyre     $(du -h "$BINARY_DST" | cut -f1)"
echo "  lib/      $(find "$LIB_DIR" -maxdepth 1 -type f -o -type l | wc -l) shared objects"
echo "  tessdata/ eng.traineddata ($(du -h "$TESS_DIR/eng.traineddata" | cut -f1))"
echo ""
echo "Run: $BUNDLE_DIR/sqyre"
