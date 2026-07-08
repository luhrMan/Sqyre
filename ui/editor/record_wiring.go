package editor

import (
	"Sqyre/internal/models"
	"Sqyre/internal/panicsafe"
	"Sqyre/internal/screen"
	"Sqyre/ui/custom_widgets"
	"strconv"
	"sync"
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/widget"
)

func wirePointRecordButton(w map[string]fyne.CanvasObject, onRecorded func(x, y int)) {
	recordButton, ok := w["recordButton"].(*widget.Button)
	if !ok {
		return
	}
	recordButton.OnTapped = func() {
		var dismissOverlay func()
		dismissOverlay = activeWire.ShowRecordingOverlay(
			nil,
			func(ev *desktop.MouseEvent) {
				switch ev.Button {
				case desktop.MouseButtonPrimary:
					x, y := screen.Location()
					custom_widgets.SetEntryText(w["X"], strconv.Itoa(x))
					custom_widgets.SetEntryText(w["Y"], strconv.Itoa(y))
					dismissOverlay()
					if onRecorded != nil {
						onRecorded(x, y)
					}
				default:
					dismissOverlay()
				}
			},
		)
	}
}

func wireSearchAreaRecordButton(w map[string]fyne.CanvasObject, onRecorded func(leftX, topY, rightX, bottomY int)) {
	saRecordButton, ok := w["recordButton"].(*widget.Button)
	if !ok {
		return
	}
	saRecordButton.OnTapped = func() {
		stopPoll := make(chan struct{})
		var stopOnce sync.Once
		stopPolling := func() { stopOnce.Do(func() { close(stopPoll) }) }

		var mu sync.Mutex
		leftX, topY := 0, 0
		firstClickDone := false

		var dismissOverlay func()
		var setSelectionRect func(leftX, topY, rightX, bottomY int)
		dismissOverlay, setSelectionRect = activeWire.ShowSearchAreaRecordingOverlay(
			func() { stopPolling() },
			func(ev *desktop.MouseEvent) {
				if ev.Button != desktop.MouseButtonPrimary {
					dismissOverlay()
					return
				}
				adjX, adjY := screen.Location()
				mu.Lock()
				if !firstClickDone {
					leftX, topY = adjX, adjY
					firstClickDone = true
					mu.Unlock()
					return
				}
				rightX, bottomY := adjX, adjY
				lx, ty := leftX, topY
				mu.Unlock()
				if lx > rightX {
					lx, rightX = rightX, lx
				}
				if ty > bottomY {
					ty, bottomY = bottomY, ty
				}
				leftX, topY = lx, ty
				stopPolling()
				custom_widgets.SetEntryText(w["LeftX"], strconv.Itoa(leftX))
				custom_widgets.SetEntryText(w["TopY"], strconv.Itoa(topY))
				custom_widgets.SetEntryText(w["RightX"], strconv.Itoa(rightX))
				custom_widgets.SetEntryText(w["BottomY"], strconv.Itoa(bottomY))
				dismissOverlay()
				if onRecorded != nil {
					onRecorded(leftX, topY, rightX, bottomY)
				}
			},
		)

		panicsafe.GoSafe(func() {
			for {
				select {
				case <-stopPoll:
					return
				default:
					mu.Lock()
					done := firstClickDone
					lx, ty := leftX, topY
					mu.Unlock()
					if !done {
						setSelectionRect(0, 0, 0, 0)
					} else {
						x, y := screen.Location()
						rx, by := x, y
						if lx > rx {
							lx, rx = rx, lx
						}
						if ty > by {
							ty, by = by, ty
						}
						setSelectionRect(lx, ty, rx, by)
					}
				}
				select {
				case <-stopPoll:
					return
				case <-time.After(50 * time.Millisecond):
				}
			}
		})
	}
}

func pointFromWidgets(w map[string]fyne.CanvasObject) *models.Point {
	n := w["Name"].(*widget.Entry).Text
	xText := custom_widgets.EntryText(w["X"])
	yText := custom_widgets.EntryText(w["Y"])
	return &models.Point{
		Name: n,
		X:    parseIntOrString(xText),
		Y:    parseIntOrString(yText),
	}
}

func searchAreaFromWidgets(w map[string]fyne.CanvasObject) *models.SearchArea {
	n := w["Name"].(*widget.Entry).Text
	return &models.SearchArea{
		Name:    n,
		LeftX:   parseIntOrString(custom_widgets.EntryText(w["LeftX"])),
		TopY:    parseIntOrString(custom_widgets.EntryText(w["TopY"])),
		RightX:  parseIntOrString(custom_widgets.EntryText(w["RightX"])),
		BottomY: parseIntOrString(custom_widgets.EntryText(w["BottomY"])),
	}
}

func wirePointPreviewRefresh(panel *editorPreviewPanel, refreshBtn *widget.Button, w map[string]fyne.CanvasObject) {
	if refreshBtn == nil {
		return
	}
	refreshBtn.OnTapped = func() {
		safeUpdatePointPreviewPanel(panel, pointFromWidgets(w))
	}
}

func wireSearchAreaPreviewRefresh(panel *editorPreviewPanel, refreshBtn *widget.Button, w map[string]fyne.CanvasObject) {
	if refreshBtn == nil {
		return
	}
	refreshBtn.OnTapped = func() {
		safeUpdateSearchAreaPreviewPanel(panel, searchAreaFromWidgets(w))
	}
}
