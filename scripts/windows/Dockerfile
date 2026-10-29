# Sqyre Windows cross-compile image (Linux → x86_64-pc-windows-gnu).
# The OCR stack is pure Rust (ocrs/rten); the MinGW toolchain only provides the
# C toolchain for deps like ring and the static C++ runtime link.
FROM debian:trixie-slim

ARG DEBIAN_FRONTEND=noninteractive
ARG RUST_VERSION=1.96.1

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl git \
    build-essential \
    gcc-mingw-w64-x86-64 g++-mingw-w64-x86-64 \
    gcc-mingw-w64-x86-64-posix g++-mingw-w64-x86-64-posix \
    && rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH="/usr/local/cargo/bin:${PATH}"
# Components match repo rust-toolchain.toml so cargo does not try to download them at build time.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
        --default-toolchain "${RUST_VERSION}" \
        --profile minimal \
        --target x86_64-pc-windows-gnu \
        --component rustfmt,clippy \
    && chmod -R a+rwX /usr/local/rustup /usr/local/cargo \
    && rustc --version && rustup target list --installed \
    && rustup component list --installed

# Host-mounted SCCACHE_DIR reuses rustc outputs across clean target/ trees and CI runners.
ARG SCCACHE_VERSION=0.16.0
RUN set -eux \
 && curl -fsSL "https://github.com/mozilla/sccache/releases/download/v${SCCACHE_VERSION}/sccache-v${SCCACHE_VERSION}-x86_64-unknown-linux-musl.tar.gz" \
    | tar -xz -C /usr/local/bin --strip-components=1 \
      "sccache-v${SCCACHE_VERSION}-x86_64-unknown-linux-musl/sccache" \
 && chmod 755 /usr/local/bin/sccache \
 && sccache --version

# Cargo may inject CARGO_INCREMENTAL=0 into the wrapper env; sccache rejects any value.
COPY sccache-rustc-wrapper.sh /usr/local/bin/sccache-rustc-wrapper
RUN sed -i 's/\r$//' /usr/local/bin/sccache-rustc-wrapper \
 && chmod 755 /usr/local/bin/sccache-rustc-wrapper

COPY mingw-lld-link.sh /usr/local/bin/mingw-lld-link
RUN sed -i 's/\r$//' /usr/local/bin/mingw-lld-link && chmod 755 /usr/local/bin/mingw-lld-link \
 && mkdir -p /usr/local/libexec/mingw-rust-lld \
 && ln -sfn /usr/local/cargo/bin/rust-lld /usr/local/libexec/mingw-rust-lld/ld.lld \
 && ln -sfn /usr/local/cargo/bin/rust-lld /usr/local/libexec/mingw-rust-lld/x86_64-w64-mingw32-ld.lld \
 && ln -sfn /usr/local/cargo/bin/rust-lld /usr/local/libexec/mingw-rust-lld/ld \
 && chmod -R a+rX /usr/local/libexec/mingw-rust-lld

# Stable path to MinGW libstdc++.a so RUSTFLAGS can -L it (static=stdc++ must find the .a).
RUN set -eux \
 && LIBSTDCPP_DIR="$(dirname "$(x86_64-w64-mingw32-g++-posix -print-file-name=libstdc++.a)")" \
 && test -f "${LIBSTDCPP_DIR}/libstdc++.a" \
 && ln -sfn "${LIBSTDCPP_DIR}" /usr/local/mingw-libstdcpp \
 && ls -la /usr/local/mingw-libstdcpp/libstdc++.a /usr/local/mingw-libstdcpp/libgcc*.a 2>/dev/null || true

# rust-lld wrapper + MinGW lib paths (LLVM 14 system lld rejects Rust ≥1.92 COFF).
# Static MinGW runtimes are appended by mingw-lld-link (libstdc++.a / libpthread.a / libgcc)
# so sqyre.exe needs no MinGW sidecar DLLs.
ENV CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=mingw-lld-link \
    CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc-posix \
    CXX_x86_64_pc_windows_gnu=x86_64-w64-mingw32-g++-posix \
    AR_x86_64_pc_windows_gnu=x86_64-w64-mingw32-ar \
    CXXSTDLIB_x86_64_pc_windows_gnu=static=stdc++ \
    RUSTFLAGS="-L/usr/local/mingw-libstdcpp -C link-arg=-lm -C link-arg=-lmsvcrt" \
    SCCACHE_DIR=/workspace/.cache/sccache-windows \
    SCCACHE_CACHE_SIZE=10G

WORKDIR /workspace
