# Dev Container: build Sqyre for Linux. Run the built binary on your host or a Linux VM.
FROM ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive

# Go (use full version e.g. 1.24.13; go1.24 is not a valid tarball)
ARG GO_VERSION=1.24.13
RUN apt-get update && apt-get install -y --no-install-recommends curl ca-certificates && \
    curl -sSL "https://go.dev/dl/go${GO_VERSION}.linux-amd64.tar.gz" -o /tmp/go.tgz && \
    tar -C /usr/local -xzf /tmp/go.tgz && rm /tmp/go.tgz && \
    ln -sf /usr/local/go/bin/go /usr/local/bin/go && \
    ln -sf /usr/local/go/bin/gofmt /usr/local/bin/gofmt

# Build deps: C compiler, pkg-config, OpenGL/X11 (Fyne, glfw, robotgo, gohook), OpenCV, Tesseract/Leptonica
RUN apt-get update && apt-get install -y --no-install-recommends \
    git \
    build-essential \
    pkg-config \
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
    # OpenCV, Tesseract, Leptonica
    libopencv-dev \
    tesseract-ocr \
    libtesseract-dev \
    libleptonica-dev \
    && rm -rf /var/lib/apt/lists/*

ENV CGO_ENABLED=1
ENV PATH="/usr/local/go/bin:${PATH}"
ENV GOPATH=/go

# fyne-cross for cross-compiling Fyne apps (e.g. Windows from Linux)
RUN go install github.com/fyne-io/fyne-cross@latest
ENV PATH="${PATH}:/go/bin"

# Default: build for current (Linux). Binary ends up in workspace for copy-out.
WORKDIR /workspace
