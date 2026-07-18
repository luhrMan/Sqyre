# Sqyre Windows cross-compile image (Linux → x86_64-pc-windows-gnu).
# Static MinGW zlib/libpng/libjpeg/leptonica/tesseract — adapted from the former
# Go fyne-cross Dockerfile.windows-amd64 (debian bookworm + cmake staticbuild).
FROM debian:bookworm-slim

ARG DEBIAN_FRONTEND=noninteractive
ARG RUST_VERSION=1.92.0
ARG MINGW_PREFIX=/usr/local/mingw64-static

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl git \
    build-essential cmake \
    gcc-mingw-w64-x86-64 g++-mingw-w64-x86-64 \
    gcc-mingw-w64-x86-64-posix g++-mingw-w64-x86-64-posix \
    pkg-config \
    clang libclang-dev \
    lld \
    && rm -rf /var/lib/apt/lists/*

ENV MINGW_HOST=x86_64-w64-mingw32 \
    MINGW_PREFIX=${MINGW_PREFIX}

COPY build-mingw-deps.sh /tmp/build-mingw-deps.sh
RUN sed -i 's/\r$//' /tmp/build-mingw-deps.sh \
 && chmod +x /tmp/build-mingw-deps.sh \
 && bash /tmp/build-mingw-deps.sh "${MINGW_PREFIX}" \
 && rm -f /tmp/build-mingw-deps.sh

# Unversioned names for -lleptonica / -ltesseract (same as Go image).
RUN set -eux \
 && cd "${MINGW_PREFIX}/lib" \
 && if [ ! -e libleptonica.a ]; then \
      f=$(ls libleptonica-*.a liblept.a 2>/dev/null | head -1); \
      [ -n "$f" ] && ln -sf "$f" libleptonica.a && ln -sf "$f" liblept.a; \
    fi \
 && if [ ! -e libtesseract.a ]; then \
      f=$(ls libtesseract[0-9]*.a 2>/dev/null | head -1); \
      [ -n "$f" ] && ln -sf "$f" libtesseract.a; \
    fi \
 && ls -la liblept* libtesseract* libpng* libz* libjpeg* 2>/dev/null || true

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH="/usr/local/cargo/bin:${PATH}"
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
        --default-toolchain "${RUST_VERSION}" \
        --profile minimal \
        --target x86_64-pc-windows-gnu \
    && chmod -R a+rwX /usr/local/rustup /usr/local/cargo \
    && rustc --version && rustup target list --installed

COPY mingw-lld-link.sh /usr/local/bin/mingw-lld-link
RUN sed -i 's/\r$//' /usr/local/bin/mingw-lld-link && chmod 755 /usr/local/bin/mingw-lld-link \
 && mkdir -p /usr/local/libexec/mingw-rust-lld \
 && ln -sfn /usr/local/cargo/bin/rust-lld /usr/local/libexec/mingw-rust-lld/ld.lld \
 && ln -sfn /usr/local/cargo/bin/rust-lld /usr/local/libexec/mingw-rust-lld/x86_64-w64-mingw32-ld.lld \
 && ln -sfn /usr/local/cargo/bin/rust-lld /usr/local/libexec/mingw-rust-lld/ld \
 && chmod -R a+rX /usr/local/libexec/mingw-rust-lld

# rust-lld wrapper + MinGW lib paths (LLVM 14 system lld rejects Rust 1.92 COFF).
ENV CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=mingw-lld-link \
    CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc-posix \
    CXX_x86_64_pc_windows_gnu=x86_64-w64-mingw32-g++-posix \
    AR_x86_64_pc_windows_gnu=x86_64-w64-mingw32-ar \
    PKG_CONFIG_ALLOW_CROSS=1 \
    PKG_CONFIG_PATH=${MINGW_PREFIX}/lib/pkgconfig \
    PKG_CONFIG_LIBDIR=${MINGW_PREFIX}/lib/pkgconfig \
    BINDGEN_EXTRA_CLANG_ARGS_x86_64_pc_windows_gnu=--sysroot=/usr/x86_64-w64-mingw32 \
    RUSTFLAGS="-L${MINGW_PREFIX}/lib -C link-arg=-lpng16 -C link-arg=-ljpeg -C link-arg=-lz -C link-arg=-lstdc++ -C link-arg=-lm -C link-arg=-lmsvcrt"

WORKDIR /workspace
