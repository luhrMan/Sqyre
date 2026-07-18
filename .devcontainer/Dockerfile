# Rust Sqyre workspace (egui / PureCV / Tesseract) + AppImage packaging tools.
# Also used as the Linux AppImage Docker build image (make appimage fallback / CI).
# Docker CLI comes from the docker-outside-of-docker feature (host daemon via socket)
# so `make windows` and nested AppImage builds work inside the container.
# Trunk + wasm32-unknown-unknown support `make wasm` (GUI-only browser editor).
# Build and test in-container; run the GUI binary on the host (needs X11).
FROM ubuntu:22.04

ARG DEBIAN_FRONTEND=noninteractive
ARG RUST_VERSION=1.92.0

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl ca-certificates git \
    build-essential pkg-config \
    # leptess / bindgen
    clang libclang-dev \
    tesseract-ocr tesseract-ocr-eng libtesseract-dev libleptonica-dev \
    # egui / eframe / winit
    libx11-dev libxkbcommon-dev libxkbcommon-x11-dev \
    libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
    libwayland-dev libegl1-mesa-dev libdbus-1-dev \
    libfontconfig1-dev libasound2-dev \
    # Headless wgpu (egui_kittest docs screenshots)
    mesa-vulkan-drivers libvulkan1 \
    # rustautogui / X11 capture & input
    libxtst-dev libxrandr-dev libxinerama-dev libxi-dev \
    # AppImage packaging
    python3 python3-pip python3-setuptools \
    squashfs-tools patchelf zsync file desktop-file-utils \
    libgtk-3-bin \
    sudo \
    && pip3 install --no-cache-dir 'appimage-builder==1.1.0' \
    && rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH="/usr/local/cargo/bin:${PATH}"
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
        --default-toolchain "${RUST_VERSION}" \
        --profile default \
        --component rustfmt,clippy,rust-analyzer,rust-src,llvm-tools-preview \
    && rustup target add wasm32-unknown-unknown \
    && chmod -R a+rX /usr/local/rustup /usr/local/cargo \
    && rustc --version && cargo --version \
    && cargo install cargo-nextest cargo-deny cargo-llvm-cov cargo-machete trunk --locked \
    && chmod -R a+rX /usr/local/cargo \
    && cargo nextest --version && cargo deny --version && cargo llvm-cov --version \
    && cargo machete --version && trunk --version \
    && rustup target list --installed | grep -q wasm32-unknown-unknown

ARG USERNAME=vscode
ARG USER_UID=1000
ARG USER_GID=1000
RUN groupadd --gid "${USER_GID}" "${USERNAME}" \
    && useradd --uid "${USER_UID}" --gid "${USER_GID}" -m -s /bin/bash "${USERNAME}" \
    && echo "${USERNAME} ALL=(root) NOPASSWD:ALL" > "/etc/sudoers.d/${USERNAME}" \
    && chmod 0440 "/etc/sudoers.d/${USERNAME}"

USER ${USERNAME}
WORKDIR /workspace
ENV CARGO_HOME=/home/${USERNAME}/.cargo \
    RUSTUP_HOME=/usr/local/rustup \
    PATH="/usr/local/cargo/bin:${PATH}"
