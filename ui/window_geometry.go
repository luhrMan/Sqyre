package ui

import (
	"Sqyre/internal/config"
	"Sqyre/internal/screen"

	"fyne.io/fyne/v2"
)

const windowScreenMargin = 32

// availableScreenSize returns the primary display size for window clamping.
func availableScreenSize() fyne.Size {
	if config.IsUITestMode() {
		return fyne.NewSize(1920, 1080)
	}
	w, h := screen.GetScreenSize()
	return fyne.NewSize(float32(w), float32(h))
}

// clampWindowSizeToScreen fits size within screen, keeping a small margin.
func clampWindowSizeToScreen(size, screen fyne.Size) fyne.Size {
	maxW := screen.Width - windowScreenMargin
	maxH := screen.Height - windowScreenMargin
	if maxW < 1 {
		maxW = size.Width
	}
	if maxH < 1 {
		maxH = size.Height
	}

	minW := float32(400)
	minH := float32(300)
	if maxW < minW {
		minW = maxW
	}
	if maxH < minH {
		minH = maxH
	}

	w, h := size.Width, size.Height
	if w > maxW {
		w = maxW
	}
	if h > maxH {
		h = maxH
	}
	if w < minW {
		w = minW
	}
	if h < minH {
		h = minH
	}
	return fyne.NewSize(w, h)
}

// clampWindowSize fits a window size within the available screen, keeping a small margin.
func clampWindowSize(size fyne.Size) fyne.Size {
	return clampWindowSizeToScreen(size, availableScreenSize())
}

// clampWindowToScreen resizes the window when it exceeds the visible screen area.
func clampWindowToScreen(w fyne.Window) {
	if w == nil {
		return
	}
	size := clampWindowSize(w.Canvas().Size())
	if size != w.Canvas().Size() {
		w.Resize(size)
	}
}
