# Rust Sqyre workspace (egui / PureCV / Tesseract) + AppImage / Flatpak packaging tools.
# Also used as the Linux AppImage Docker build image (make appimage fallback / CI).
# Docker CLI comes from the docker-outside-of-docker feature (host daemon via socket)
# so `make windows`, nested AppImage builds, and Flatpak Docker fallbacks work inside
# the container. Flatpak builds need user namespaces / bwrap (host Docker privileged
# image when the container itself cannot run flatpak-builder natively).
# Trunk + wasm32-unknown-unknown support `make wasm` (GUI-only browser editor).
# Build and test in-container; run the GUI binary on the host (needs X11).
FROM ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive
ARG RUST_VERSION=1.95.0

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
    libpipewire-0.3-dev libspa-0.2-dev \
    libfontconfig1-dev libasound2-dev \
    # Headless wgpu (egui_kittest docs screenshots)
    mesa-vulkan-drivers libvulkan1 \
    # rustautogui / X11 capture & input
    libxtst-dev libxrandr-dev libxinerama-dev libxi-dev libxfixes-dev \
    # rdev unstable_grab (Wayland evdev hotkeys)
    libevdev-dev \
    # AppImage packaging
    python3 python3-pip python3-setuptools \
    squashfs-tools patchelf zsync file desktop-file-utils \
    libgtk-3-bin librsvg2-bin \
    # Flatpak packaging (native builds need bwrap/user namespaces on the host)
    flatpak flatpak-builder ostree \
    sudo \
    && pip3 install --no-cache-dir --break-system-packages 'appimage-builder==1.1.0' \
        'aiohttp>=3.9.5,<4' 'tomlkit>=0.13.3,<1' \
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

# Host-mounted SCCACHE_DIR reuses rustc outputs across clean target/ trees and CI runners.
# Wrapper unsets CARGO_INCREMENTAL*: sccache rejects those vars when present at all.
ARG SCCACHE_VERSION=0.16.0
RUN set -eux \
 && curl -fsSL "https://github.com/mozilla/sccache/releases/download/v${SCCACHE_VERSION}/sccache-v${SCCACHE_VERSION}-x86_64-unknown-linux-musl.tar.gz" \
    | tar -xz -C /usr/local/bin --strip-components=1 \
      "sccache-v${SCCACHE_VERSION}-x86_64-unknown-linux-musl/sccache" \
 && chmod 755 /usr/local/bin/sccache \
 && printf '%s\n' \
      '#!/bin/sh' \
      'unset CARGO_INCREMENTAL CARGO_BUILD_INCREMENTAL' \
      'exec /usr/local/bin/sccache "$@"' \
      > /usr/local/bin/sccache-rustc-wrapper \
 && chmod 755 /usr/local/bin/sccache-rustc-wrapper \
 && sccache --version

# Android APK (`make android` / `make android-check`): JDK 17, SDK, NDK, Gradle, cargo-ndk.
ARG ANDROID_CMDLINE_TOOLS=11076708
ARG ANDROID_PLATFORM=35
ARG ANDROID_BUILD_TOOLS=35.0.0
ARG ANDROID_NDK_VERSION=27.2.12479018
ARG GRADLE_VERSION=8.10.2
ENV ANDROID_HOME=/opt/android-sdk \
    ANDROID_NDK_HOME=/opt/android-sdk/ndk/${ANDROID_NDK_VERSION} \
    JAVA_HOME=/usr/lib/jvm/java-17-openjdk-amd64 \
    PATH="/opt/gradle/bin:/opt/android-sdk/cmdline-tools/latest/bin:/opt/android-sdk/platform-tools:${PATH}"
RUN set -eux \
 && apt-get update && apt-get install -y --no-install-recommends openjdk-17-jdk-headless unzip \
 && rm -rf /var/lib/apt/lists/* \
 && mkdir -p "${ANDROID_HOME}/cmdline-tools" \
 && curl -fsSL -o /tmp/cmdline-tools.zip \
      "https://dl.google.com/android/repository/commandlinetools-linux-${ANDROID_CMDLINE_TOOLS}_latest.zip" \
 && unzip -q /tmp/cmdline-tools.zip -d "${ANDROID_HOME}/cmdline-tools" \
 && mv "${ANDROID_HOME}/cmdline-tools/cmdline-tools" "${ANDROID_HOME}/cmdline-tools/latest" \
 && rm /tmp/cmdline-tools.zip \
 && yes | sdkmanager --licenses >/dev/null \
 && sdkmanager --install \
      "platform-tools" \
      "platforms;android-${ANDROID_PLATFORM}" \
      "build-tools;${ANDROID_BUILD_TOOLS}" \
      "ndk;${ANDROID_NDK_VERSION}" \
 && curl -fsSL -o /tmp/gradle.zip \
      "https://services.gradle.org/distributions/gradle-${GRADLE_VERSION}-bin.zip" \
 && unzip -q /tmp/gradle.zip -d /opt \
 && mv "/opt/gradle-${GRADLE_VERSION}" /opt/gradle \
 && rm /tmp/gradle.zip \
 && chmod -R a+rwX "${ANDROID_HOME}" \
 && rustup target add aarch64-linux-android x86_64-linux-android \
 && cargo install cargo-ndk --locked \
 && chmod -R a+rX /usr/local/rustup /usr/local/cargo \
 && java -version && gradle --version && cargo ndk --version

ARG USERNAME=vscode
ARG USER_UID=1000
ARG USER_GID=1000
RUN groupadd --gid "${USER_GID}" "${USERNAME}" \
    || groupmod -n "${USERNAME}" "$(getent group "${USER_GID}" | cut -d: -f1)" \
    && useradd -s /bin/bash --uid "${USER_UID}" --gid "${USER_GID}" -m "${USERNAME}" \
    || usermod -l "${USERNAME}" -d "/home/${USERNAME}" -m "$(getent passwd "${USER_UID}" | cut -d: -f1)" \
    && usermod -g "${USER_GID}" "${USERNAME}" \
    && echo "${USERNAME} ALL=(root) NOPASSWD:ALL" > "/etc/sudoers.d/${USERNAME}" \
    && chmod 0440 "/etc/sudoers.d/${USERNAME}"

USER ${USERNAME}
WORKDIR /workspace
ENV CARGO_HOME=/home/${USERNAME}/.cargo \
    RUSTUP_HOME=/usr/local/rustup \
    PATH="/usr/local/cargo/bin:${PATH}"
