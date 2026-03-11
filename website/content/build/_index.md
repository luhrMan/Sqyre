---
title: "Build"
date: 2025-03-09
---

Build Sqyre from source. The recommended way is to use the **dev container** (e.g. in VS Code/Cursor: *Dev Containers: Reopen in Container*).

## Linux (dev container)

```bash
go build -o sqyre ./cmd/sqyre
./sqyre
```

Logs: `~/.sqyre/sqyre.log`.

For **Flatpak** or **AppImage**, see the repo: [.devcontainer/builds/linux/packaging/PACKAGING.md](https://github.com/your-org/sqyre/blob/main/.devcontainer/builds/linux/packaging/PACKAGING.md).

## Linux (bare system)

Install dependencies (Debian/Ubuntu):

```bash
sudo apt install -y \
  build-essential pkg-config cmake golang-go \
  tesseract-ocr libtesseract-dev libleptonica-dev \
  libgl1-mesa-dev libglvnd-dev libglfw3-dev \
  libxkbcommon-dev libxkbcommon-x11-dev \
  libx11-dev libx11-xcb-dev libxext-dev libxtst-dev \
  libxcursor-dev libxrandr-dev libxinerama-dev \
  libxxf86vm-dev libxt-dev \
  libjpeg-dev libpng-dev libtiff-dev libwebp-dev libopenjp2-7-dev
```

Install OpenCV (≥ 4.6) — see the repo Dockerfile for reference. Then:

```bash
go build -o sqyre ./cmd/sqyre
```

## Windows

From inside the dev container:

```bash
bash .devcontainer/builds/windows/build.sh
```

Output: `.devcontainer/builds/windows/output/Sqyre.exe`

For native Windows (MSYS2), see the main [README](https://github.com/your-org/sqyre) in the repository.
