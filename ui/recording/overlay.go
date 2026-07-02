package recording

import (
	"image"
	"image/color"
	"image/draw"
	"log"
	"sync"
	"time"

	"Sqyre/internal/capture"
	"Sqyre/internal/services"
	"Sqyre/ui/desktopview"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/widget"
)

// recordingMouseLayer sits on top of the overlay stack and receives mouse clicks
// via Fyne (overlay window has focus) instead of a global low-level hook.
type recordingMouseLayer struct {
	widget.BaseWidget
	onMouseDown func(*desktop.MouseEvent)
}

func newRecordingMouseLayer(onMouseDown func(*desktop.MouseEvent)) *recordingMouseLayer {
	r := &recordingMouseLayer{onMouseDown: onMouseDown}
	r.ExtendBaseWidget(r)
	return r
}

func (r *recordingMouseLayer) MouseDown(ev *desktop.MouseEvent) {
	if r.onMouseDown != nil {
		r.onMouseDown(ev)
	}
}

func (r *recordingMouseLayer) MouseUp(*desktop.MouseEvent) {}

func (r *recordingMouseLayer) CreateRenderer() fyne.WidgetRenderer {
	rect := canvas.NewRectangle(color.Transparent)
	return widget.NewSimpleRenderer(rect)
}

type selectionRectLayout struct {
	leftX, topY, rightX, bottomY float32
}

func (s *selectionRectLayout) Layout(objects []fyne.CanvasObject, size fyne.Size) {
	if len(objects) < 1 {
		return
	}
	w := s.rightX - s.leftX
	h := s.bottomY - s.topY
	if w <= 0 || h <= 0 {
		objects[0].Resize(fyne.NewSize(0, 0))
		objects[0].Move(fyne.NewPos(0, 0))
		return
	}
	objects[0].Move(fyne.NewPos(s.leftX, s.topY))
	objects[0].Resize(fyne.NewSize(w, h))
}

func (s *selectionRectLayout) MinSize(objects []fyne.CanvasObject) fyne.Size {
	return fyne.NewSize(0, 0)
}

type fyneDesktopOverlay struct {
	win           fyne.Window
	displayIndex  int
	desktopBounds image.Rectangle
	bgImage       *canvas.Image
	widthPx       int
	heightPx      int
	selLayout     *selectionRectLayout
	selRefresher  interface{ Refresh() }
	stopPosition  chan struct{}
}

func logOverlayWindowDiagnostics(overlays []*fyneDesktopOverlay) {
	if !capture.OverlayDiagnosticsEnabled() {
		return
	}
	for _, o := range overlays {
		if o == nil || o.win == nil {
			continue
		}
		sz := o.win.Canvas().Size()
		log.Printf(
			"overlay window diag: display=%d bounds=%v monitor_px=%dx%d canvas_size=%.1fx%.1f scale=%.3f",
			o.displayIndex, o.desktopBounds, o.widthPx, o.heightPx, sz.Width, sz.Height, desktopview.CanvasScale(o.win),
		)
	}
}

func composeOverlayImage(monitors []capture.MonitorPlan, captures map[int]image.Image) (image.Image, image.Rectangle, bool) {
	var union image.Rectangle
	have := false
	for _, mon := range monitors {
		if img, ok := captures[mon.DisplayIndex]; ok && img != nil {
			union = union.Union(mon.DesktopBounds)
			have = true
		}
	}
	if !have || union.Empty() || union.Dx() <= 0 || union.Dy() <= 0 {
		return nil, image.Rectangle{}, false
	}
	out := image.NewRGBA(image.Rect(0, 0, union.Dx(), union.Dy()))
	for _, mon := range monitors {
		img, ok := captures[mon.DisplayIndex]
		if !ok || img == nil {
			continue
		}
		dstMin := image.Pt(mon.DesktopBounds.Min.X-union.Min.X, mon.DesktopBounds.Min.Y-union.Min.Y)
		dst := image.Rectangle{Min: dstMin, Max: dstMin.Add(img.Bounds().Size())}
		draw.Draw(out, dst, img, img.Bounds().Min, draw.Src)
	}
	return out, union, true
}

func newOverlayWindow(
	app fyne.App,
	captureImg image.Image,
	desktopBounds image.Rectangle,
	displayIndex int,
	withSelectionRect bool,
	onMouseDown func(*desktop.MouseEvent),
) *fyneDesktopOverlay {
	win := app.NewWindow("")
	win.SetPadded(false)
	win.SetFixedSize(true)
	widthPx := desktopBounds.Dx()
	heightPx := desktopBounds.Dy()
	win.Resize(fyne.NewSize(
		desktopview.PhysicalToCanvas(win, widthPx),
		desktopview.PhysicalToCanvas(win, heightPx),
	))
	bgImage := desktopview.NewOverlaySnapshotImage(captureImg, win, widthPx, heightPx)

	var selLayout *selectionRectLayout
	var selRefresher interface{ Refresh() }
	var selectionLayer fyne.CanvasObject
	if withSelectionRect {
		selLayout = &selectionRectLayout{}
		selRect := canvas.NewRectangle(color.Transparent)
		selRect.StrokeColor = color.NRGBA{R: 255, G: 200, B: 0, A: 255}
		selRect.StrokeWidth = 2
		selContainer := container.New(selLayout, selRect)
		selectionLayer = selContainer
		selRefresher = selContainer
	} else {
		selectionLayer = layout.NewSpacer()
	}
	var stack fyne.CanvasObject
	if onMouseDown != nil {
		stack = container.NewMax(bgImage, selectionLayer, newRecordingMouseLayer(onMouseDown))
	} else {
		stack = container.NewMax(bgImage, selectionLayer)
	}
	win.SetContent(stack)
	return &fyneDesktopOverlay{
		win:           win,
		displayIndex:  displayIndex,
		desktopBounds: desktopBounds,
		bgImage:       bgImage,
		widthPx:       widthPx,
		heightPx:      heightPx,
		selLayout:     selLayout,
		selRefresher:  selRefresher,
		stopPosition:  make(chan struct{}),
	}
}

func absoluteRectToOverlayLocal(leftX, topY, rightX, bottomY int, bounds image.Rectangle) (left, top, right, bottom float32, ok bool) {
	local, ok := desktopview.ClipAbsoluteRectToVirtualLocal(leftX, topY, rightX, bottomY, bounds)
	if !ok {
		return 0, 0, 0, 0, false
	}
	return float32(local.Min.X), float32(local.Min.Y), float32(local.Max.X), float32(local.Max.Y), true
}

func mapOverlayPxToCanvas(o *fyneDesktopOverlay, pxX, pxY float32) (float32, float32) {
	if o == nil || o.win == nil || o.widthPx <= 0 || o.heightPx <= 0 {
		return pxX, pxY
	}
	sz := o.win.Canvas().Size()
	if sz.Width <= 0 || sz.Height <= 0 {
		return pxX, pxY
	}
	return pxX * (sz.Width / float32(o.widthPx)), pxY * (sz.Height / float32(o.heightPx))
}

func (o *fyneDesktopOverlay) resizeBackground() {
	if o.bgImage == nil {
		return
	}
	desktopview.ResizeOverlaySnapshot(o.bgImage, o.win, o.widthPx, o.heightPx)
}

func (o *fyneDesktopOverlay) reposition() {
	positionFyneOverlayWindow(o.win, o.desktopBounds)
	o.resizeBackground()
}

// startPositionLoop keeps the overlay pinned to desktopBounds. Fyne/GLFW may
// reset window geometry after Show(); RunNative positioning is also queued
// asynchronously, so we retry until the overlay is dismissed.
func (o *fyneDesktopOverlay) startPositionLoop() {
	services.GoSafe(func() {
		delays := []time.Duration{0, 50, 100, 200, 400, 800, 1200}
		for _, d := range delays {
			select {
			case <-o.stopPosition:
				return
			case <-time.After(d):
				fyne.Do(o.reposition)
			}
		}
		// Stop after initial retries to avoid continuous native reconfiguration
		// that can disrupt compositor rendering on some multi-monitor setups.
	})
}

func showFullScreenOverlay(withSelectionRect bool, onClosed func(), onMouseDown func(*desktop.MouseEvent)) (dismiss func(), setSelectionRect func(leftX, topY, rightX, bottomY int)) {
	app := fyne.CurrentApp()
	if app == nil {
		return func() {}, func(int, int, int, int) {}
	}

	plan, planErr := capture.SessionPlanForOverlay()
	if planErr != nil {
		log.Printf("overlay: capture probe failed: %v", planErr)
		return func() {}, func(int, int, int, int) {}
	}
	session, sessionErr := capture.NewSession(plan)
	if sessionErr != nil {
		log.Printf("overlay: capture session init failed: %v", sessionErr)
		return func() {}, func(int, int, int, int) {}
	}

	hiddenAppWindows := hideAppWindowsDuringRecording(app)

	var (
		lifecycleMu      sync.Mutex
		overlays         []*fyneDesktopOverlay
		pendingTimer     *time.Timer
		pendingCancelled bool
		dismissOnce      sync.Once
		closedOnce       sync.Once
		restoreOnce      sync.Once
		setSelectionFn   func(leftX, topY, rightX, bottomY int)
	)

	restoreHidden := func() {
		restoreOnce.Do(func() {
			showAppWindows(hiddenAppWindows)
		})
	}

	cancelPending := func() {
		lifecycleMu.Lock()
		defer lifecycleMu.Unlock()
		pendingCancelled = true
		if pendingTimer != nil {
			pendingTimer.Stop()
		}
	}

	pendingStillActive := func() bool {
		lifecycleMu.Lock()
		defer lifecycleMu.Unlock()
		return !pendingCancelled
	}

	dismiss = func() {
		dismissOnce.Do(func() {
			cancelPending()
			lifecycleMu.Lock()
			ovs := overlays
			lifecycleMu.Unlock()
			for _, o := range ovs {
				close(o.stopPosition)
				o.win.Close()
			}
			restoreHidden()
		})
	}

	setSelectionRect = func(leftX, topY, rightX, bottomY int) {
		if setSelectionFn != nil {
			setSelectionFn(leftX, topY, rightX, bottomY)
		}
	}

	presentOverlay := func() {
		if !pendingStillActive() {
			restoreHidden()
			return
		}

		monitorCaptures := make(map[int]image.Image, len(plan.Monitors))
		diag := capture.OverlayDiagnosticsEnabled()
		for _, mon := range plan.Monitors {
			captureImg, source, err := capture.OverlayMonitorImage(plan, session, mon.DisplayIndex)
			if diag {
				log.Printf("overlay source diag: display=%d source=%s", mon.DisplayIndex, source)
			}
			if err != nil {
				log.Printf("overlay: capture failed display=%d backend=%s: %v", mon.DisplayIndex, plan.Backend, err)
				continue
			}
			if captureImg == nil {
				continue
			}
			monitorCaptures[mon.DisplayIndex] = captureImg
		}
		if diag {
			log.Printf("overlay source diag: backend=%s per_monitor=%v", plan.Backend, !useVirtualDesktopOverlay())
		}

		var built []*fyneDesktopOverlay
		if useVirtualDesktopOverlay() {
			composedImg, unionBounds, ok := composeOverlayImage(plan.Monitors, monitorCaptures)
			if !ok {
				restoreHidden()
				log.Printf("overlay: no overlay windows built")
				return
			}
			built = append(built, newOverlayWindow(app, composedImg, unionBounds, 0, withSelectionRect, onMouseDown))
		} else {
			for _, mon := range plan.Monitors {
				img := monitorCaptures[mon.DisplayIndex]
				if img == nil {
					continue
				}
				built = append(built, newOverlayWindow(app, img, mon.DesktopBounds, mon.DisplayIndex, withSelectionRect, onMouseDown))
			}
			if len(built) == 0 {
				restoreHidden()
				log.Printf("overlay: no overlay windows built")
				return
			}
		}

		if !pendingStillActive() {
			for _, o := range built {
				close(o.stopPosition)
				o.win.Close()
			}
			restoreHidden()
			return
		}

		for _, overlay := range built {
			overlay.reposition()
		}
		for _, overlay := range built {
			overlay.win.Show()
			overlay.reposition()
			overlay.startPositionLoop()
		}

		lifecycleMu.Lock()
		overlays = built
		lifecycleMu.Unlock()

		for i, overlay := range built {
			if onClosed != nil {
				overlay.win.SetOnClosed(func() {
					restoreHidden()
					closedOnce.Do(onClosed)
				})
			} else {
				overlay.win.SetOnClosed(restoreHidden)
			}
			overlay.win.Canvas().SetOnTypedKey(func(e *fyne.KeyEvent) {
				if e.Name == fyne.KeyEscape {
					dismiss()
				}
			})
			if i == 0 {
				overlay.win.RequestFocus()
			}
		}

		logOverlayWindowDiagnostics(built)
		if capture.OverlayDiagnosticsEnabled() {
			services.GoSafe(func() {
				time.Sleep(250 * time.Millisecond)
				fyne.Do(func() { logOverlayWindowDiagnostics(built) })
			})
		}

		if withSelectionRect {
			setSelectionFn = func(leftX, topY, rightX, bottomY int) {
				fyne.Do(func() {
					for _, o := range built {
						if o.selLayout == nil || o.selRefresher == nil {
							continue
						}
						l, t, r, b, ok := absoluteRectToOverlayLocal(leftX, topY, rightX, bottomY, o.desktopBounds)
						if ok {
							o.selLayout.leftX, o.selLayout.topY = mapOverlayPxToCanvas(o, l, t)
							o.selLayout.rightX, o.selLayout.bottomY = mapOverlayPxToCanvas(o, r, b)
						} else {
							o.selLayout.leftX, o.selLayout.topY, o.selLayout.rightX, o.selLayout.bottomY = 0, 0, 0, 0
						}
						o.selRefresher.Refresh()
					}
				})
			}
		}
	}

	lifecycleMu.Lock()
	pendingTimer = scheduleAfterHidingApp(hiddenAppWindows, presentOverlay)
	lifecycleMu.Unlock()

	return dismiss, setSelectionRect
}

func ShowRecordingOverlay(onClosed func(), onMouseDown func(*desktop.MouseEvent)) func() {
	dismiss, _ := showFullScreenOverlay(false, onClosed, onMouseDown)
	return dismiss
}

func ShowSearchAreaRecordingOverlay(onClosed func(), onMouseDown func(*desktop.MouseEvent)) (dismiss func(), setSelectionRect func(leftX, topY, rightX, bottomY int)) {
	return showFullScreenOverlay(true, onClosed, onMouseDown)
}
