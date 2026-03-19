# To reuse this image as base for Android OpenCV build (faster): tag it, e.g. sqyre-dev:latest.
# Build targets:
#   devcontainer (default) – Linux build only; no Android NDK.
#   with-android            – same + Android NDK. Set in devcontainer.json: "build": { "target": "with-android" }
FROM ubuntu:22.04 AS devcontainer

ARG DEBIAN_FRONTEND=noninteractive
ARG GO_VERSION=1.26.0
ARG OPENCV_VERSION=4.13.0

# -----------------------------------------------------------------------------
# Stage: toolchain downloads (curl, unzip). Rarely changes.
# -----------------------------------------------------------------------------
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl ca-certificates unzip \
    && rm -rf /var/lib/apt/lists/*

# -----------------------------------------------------------------------------
# Stage: Go
# -----------------------------------------------------------------------------
RUN curl -sSL "https://go.dev/dl/go${GO_VERSION}.linux-amd64.tar.gz" -o /tmp/go.tgz && \
    tar -C /usr/local -xzf /tmp/go.tgz && rm /tmp/go.tgz && \
    ln -sf /usr/local/go/bin/go /usr/local/bin/go && \
    ln -sf /usr/local/go/bin/gofmt /usr/local/bin/gofmt

ENV CGO_ENABLED=1
ENV PATH="/usr/local/go/bin:${PATH}"
ENV GOPATH=/go

# -----------------------------------------------------------------------------
# Stage: build deps for OpenCV + CGO (Fyne, X11, Tesseract, image libs).
# Add packages here only if needed to build OpenCV or the Go app's C deps.
# -----------------------------------------------------------------------------
RUN apt-get update && apt-get install -y --no-install-recommends \
    git build-essential pkg-config cmake ninja-build \
    libgl1-mesa-dev libglvnd-dev libglfw3-dev \
    libxkbcommon-dev libxkbcommon-x11-dev \
    libx11-dev libx11-xcb-dev libxext-dev libxtst-dev libxcursor-dev \
    libxrandr-dev libxinerama-dev libxxf86vm-dev libxt-dev \
    tesseract-ocr libtesseract-dev libleptonica-dev \
    libjpeg-dev libpng-dev libtiff-dev libwebp-dev libopenjp2-7-dev libgif-dev \
    && rm -rf /var/lib/apt/lists/*

# -----------------------------------------------------------------------------
# Stage: fyne-cross (cross-compile Fyne). Pinned for cache; bump when upgrading.
# -----------------------------------------------------------------------------
RUN go install github.com/fyne-io/fyne-cross@v1.6.1
ENV PATH="${PATH}:/go/bin"

# -----------------------------------------------------------------------------
# Stage: OpenCV from source (slow). Script lives under builds/opencv/linux/.
# Only rebuilds when OPENCV_VERSION or build-deps change.
# -----------------------------------------------------------------------------
COPY .devcontainer/builds/opencv/linux/build-opencv-linux.sh /tmp/build-opencv-linux.sh
RUN chmod +x /tmp/build-opencv-linux.sh && OPENCV_VERSION=${OPENCV_VERSION} OPENCV_ROOT=/opt/opencv/linux /tmp/build-opencv-linux.sh
# Point pkg-config and runtime linker at OpenCV under its build folder
ENV PKG_CONFIG_PATH="/opt/opencv/linux/install/lib/pkgconfig:${PKG_CONFIG_PATH}"
ENV LD_LIBRARY_PATH="/opt/opencv/linux/install/lib:${LD_LIBRARY_PATH}"

# -----------------------------------------------------------------------------
# Stage: AppImage / extras. ADD NEW APT PACKAGES HERE so OpenCV is not rebuilt.
# -----------------------------------------------------------------------------
RUN apt-get update && apt-get install -y --no-install-recommends \
    binutils coreutils desktop-file-utils fakeroot fuse gtk-update-icon-cache \
    libarchive13 libgcc-s1 libgdk-pixbuf2.0-dev libstdc++6 libxcb-xkb1 \
    patchelf zlib1g python3-pip python3-setuptools squashfs-tools strace util-linux zsync \
    && rm -rf /var/lib/apt/lists/*

RUN pip3 install --no-cache-dir appimage-builder

# Nix (for `nix develop` in .devcontainer/builds/linux/nix)
# Installer expects /nix and group nixbld; create them (we run as root, no sudo).
# RUN apt-get update && apt-get install -y --no-install-recommends xz-utils && \
#     mkdir -m 0755 /nix && chown root:root /nix && \
#     groupadd --system nixbld || true && \
#     useradd --system --no-create-home -G nixbld nixbuilder && \
#     curl -L https://nixos.org/nix/install | sh -s -- --no-daemon && \
#     mkdir -p /root/.config/nix && echo 'experimental-features = nix-command flakes' >> /root/.config/nix/nix.conf && \
#     rm -rf /var/lib/apt/lists/*
# ENV PATH="/root/.nix-profile/bin:${PATH}"

WORKDIR /workspace

# -----------------------------------------------------------------------------
# Stage: with-android – devcontainer + Android NDK (optional; use build target).
# -----------------------------------------------------------------------------
FROM devcontainer AS with-android

ARG NDK_REV=r25c
RUN mkdir -p /opt && cd /opt && \
    curl -sSL "https://dl.google.com/android/repository/android-ndk-${NDK_REV}-linux.zip" -o ndk.zip && \
    unzip -q ndk.zip && mv android-ndk-* android-ndk && rm ndk.zip
ENV ANDROID_NDK=/opt/android-ndk
