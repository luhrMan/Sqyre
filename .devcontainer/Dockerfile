# Dev Container: build Sqyre for Linux. Run the built binary on your host or a Linux VM.
FROM ubuntu:22.04

ARG DEBIAN_FRONTEND=noninteractive

# Go (use full version e.g. 1.24.13; go1.24 is not a valid tarball)
ARG GO_VERSION=1.24.13
RUN apt-get update && apt-get install -y --no-install-recommends curl ca-certificates && \
    curl -sSL "https://go.dev/dl/go${GO_VERSION}.linux-amd64.tar.gz" -o /tmp/go.tgz && \
    tar -C /usr/local -xzf /tmp/go.tgz && rm /tmp/go.tgz && \
    ln -sf /usr/local/go/bin/go /usr/local/bin/go && \
    ln -sf /usr/local/go/bin/gofmt /usr/local/bin/gofmt

# Build deps: C compiler, pkg-config, OpenGL/X11 (Fyne, glfw, robotgo, gohook), Tesseract/Leptonica
RUN apt-get update && apt-get install -y --no-install-recommends \
    git \
    build-essential \
    pkg-config \
    cmake \
    # OpenGL / Fyne
    libgl1-mesa-dev \
    libglvnd-dev \
    libglfw3-dev \
    libxkbcommon-dev \
    libxkbcommon-x11-dev \
    # X11
    libx11-dev \
    libx11-xcb-dev \
    libxext-dev \
    libxtst-dev \
    libxcursor-dev \
    libxrandr-dev \
    libxinerama-dev \
    libxxf86vm-dev \
    libxt-dev \
    # Tesseract, Leptonica
    tesseract-ocr \
    libtesseract-dev \
    libleptonica-dev \
    # OpenCV build deps (image format libs)
    libjpeg-dev \
    libpng-dev \
    libtiff-dev \
    libwebp-dev \
    libopenjp2-7-dev \
    && rm -rf /var/lib/apt/lists/*

# Build OpenCV 4.6.0 from source (Ubuntu 22.04 only ships 4.5.4; gocv v0.43 needs >=4.6)
ARG OPENCV_VERSION=4.6.0
RUN curl -sSL "https://github.com/opencv/opencv/archive/${OPENCV_VERSION}.tar.gz" -o /tmp/opencv.tgz && \
    tar -xzf /tmp/opencv.tgz -C /tmp && rm /tmp/opencv.tgz && \
    mkdir /tmp/opencv-build && cd /tmp/opencv-build && \
    cmake /tmp/opencv-${OPENCV_VERSION} \
      -DCMAKE_BUILD_TYPE=Release \
      -DCMAKE_INSTALL_PREFIX=/usr/local \
      -DBUILD_SHARED_LIBS=ON \
      -DOPENCV_GENERATE_PKGCONFIG=ON \
      -DBUILD_LIST=core,imgproc,imgcodecs,videoio,highgui,video,calib3d,features2d,objdetect,dnn,photo \
      -DBUILD_TESTS=OFF \
      -DBUILD_PERF_TESTS=OFF \
      -DBUILD_EXAMPLES=OFF \
      -DBUILD_opencv_apps=OFF \
      -DWITH_JPEG=ON \
      -DWITH_PNG=ON \
      -DWITH_TIFF=ON \
      -DWITH_WEBP=ON \
      -DWITH_OPENJPEG=ON \
      -DWITH_GTK=OFF \
      -DWITH_QT=OFF \
    && make -j"$(nproc)" && make install && ldconfig \
    && rm -rf /tmp/opencv-${OPENCV_VERSION} /tmp/opencv-build

ENV CGO_ENABLED=1
ENV PATH="/usr/local/go/bin:${PATH}"
ENV GOPATH=/go

# AppImage builder: deps + pip install (for packaging/AppImageBuilder.yml)
RUN apt-get update && apt-get install -y --no-install-recommends \
    binutils \
    coreutils \
    desktop-file-utils \
    fakeroot \
    fuse \
    gtk-update-icon-cache \
    libgdk-pixbuf2.0-dev \
    patchelf \
    python3-pip \
    python3-setuptools \
    squashfs-tools \
    strace \
    util-linux \
    zsync \
    && pip3 install appimage-builder \
    && rm -rf /var/lib/apt/lists/*

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

# fyne-cross for cross-compiling Fyne apps (e.g. Windows from Linux)
RUN go install github.com/fyne-io/fyne-cross@latest
ENV PATH="${PATH}:/go/bin"

# Default: build for current (Linux). Binary ends up in workspace for copy-out.
WORKDIR /workspace
