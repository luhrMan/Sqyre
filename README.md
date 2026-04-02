# What is it

Sqyre is a Macro Builder, written using GO, with a few notable libraries:

- Fyne (GUI)
- Robotgo (Automation)
- Gosseract aka Tesseract (OCR)
- GoCV aka OpenCV (Computer Vision)

# Main Screen
<img width="2562" height="1362" alt="Screenshot from 2026-01-13 13-09-30" src="https://github.com/user-attachments/assets/53acf1a0-bc89-43d9-a7ab-856b46c3be63" />

# ImageSearch in action
![sqyre-imagesearch](https://github.com/user-attachments/assets/1a0fc8f4-06bb-4667-bb49-b1c4b2d5b508)

# BUILD INSTRUCTIONS

**Recommended:** Open this project in the **dev container** (e.g. in VS Code/Cursor: *Dev Containers: Reopen in Container*). All commands below are intended to be run from a terminal **inside the dev container**.

---

## Linux

```bash
make linux
```

### AppImage

```bash
make appimage
```

## Windows

```bash
make windows
```
<details>
  <summary><strong>GoCV Mat profiling (leak detection)</strong></summary>

Build with **matprofile** to track gocv `Mat` allocations and find leaks (unclosed Mats). Logs and a pprof HTTP server are enabled.

**Build with matprofile:**

| Platform | Command |
|----------|---------|
| **Linux** | `go build -tags "gocv_specific_modules,matprofile" -o sqyre ./cmd/sqyre` |
| **Windows** (from dev container) | `./.devcontainer/builds/windows/build-matprofile.sh` |

**What you get:** Logs (including Mat profile on exit) go to **`~/.sqyre/sqyre.log`** (Windows: `%USERPROFILE%\.sqyre\sqyre.log`). The pprof server starts on 127.0.0.1:6060 (or 6061–6065 if 6060 is in use); the exact URL is printed in the log. Open it in a browser and use the **gocv.io/x/gocv.Mat** profile for leak stack traces.

**Optional:** Set **`SQYRE_PPROF=0`** to disable the pprof server, or **`SQYRE_PPROF=127.0.0.1:9090`** to use a specific port. If the browser cannot connect, allow Sqyre in Windows Firewall or use a different port via `SQYRE_PPROF`.
</details>



---
