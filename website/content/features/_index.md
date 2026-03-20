---
title: "Features"
date: 2025-03-09
---

Sqyre is built with a small set of powerful libraries and exposes them through a tree-based macro editor.

## Tech stack

- **Fyne** — Cross-platform GUI
- **Robotgo** — Mouse and keyboard automation
- **Gosseract (Tesseract)** — OCR
- **GoCV (OpenCV)** — Computer vision

## Action tree

The editor uses a Fyne `widget.Tree`:

- **Root** — One loop action containing your macro
- **Branch (advanced)**
  - **Loop** — Repeat a sequence
  - **Image Search** — Find an image on screen and act on it
  - **OCR** — Read text from the screen
- **Leaf**
  - **Click** — Click at cursor position
  - **Move** — Move mouse to coordinates
  - **Key** — Set key state up/down
  - **Wait** — Pause (milliseconds)

Combine these to automate workflows without writing code.
