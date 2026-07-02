package actiondialog

import (
	"Sqyre/internal/models/actions"
	"image/color"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	ttwidget "github.com/dweymouth/fyne-tooltip/widget"
)

const (
	dialogEdgeGapFraction = float32(0.15)
	dialogMinW            = float32(200)
	dialogMinH            = float32(200)

	standardFormMinW = float32(560)
	wideFormMinW     = float32(720)

	scrollAreaMinH     = float32(400)
	listAreaMinH       = float32(240)
	splitPanelMinW     = float32(400)
	wideSplitPanelMinW = float32(520)
	accordionPanelMinH = float32(400)

	forEachRowSourceEntryMinHeight   = float32(120)
	forEachRowSourcesScrollMinHeight = float32(320)
	forEachRowSourceFieldMinW        = float32(520)
)

func dialogMaxSize(parent fyne.Size) fyne.Size {
	usable := 1 - 2*dialogEdgeGapFraction
	return fyne.NewSize(parent.Width*usable, parent.Height*usable)
}

func isComplexAction(action actions.ActionInterface) bool {
	switch action.(type) {
	case *actions.ImageSearch, *actions.Move, *actions.Conditional,
		*actions.Ocr, *actions.ForEachRow, *actions.FindPixel,
		*actions.FocusWindow, *actions.Calculate:
		return true
	default:
		return false
	}
}

func enforceDialogMin(size fyne.Size) fyne.Size {
	w, h := size.Width, size.Height
	if w < dialogMinW {
		w = dialogMinW
	}
	if h < dialogMinH {
		h = dialogMinH
	}
	return fyne.NewSize(w, h)
}

// actionDialogSize picks the popup size for an action dialog on the given parent window.
// Complex actions fill the parent minus a 15% gap on each edge; simple actions shrink to content.
func actionDialogSize(parent fyne.Size, action actions.ActionInterface, want fyne.Size) fyne.Size {
	maxSize := dialogMaxSize(parent)
	if isComplexAction(action) {
		return enforceDialogMin(maxSize)
	}
	w, h := want.Width, want.Height
	if w > maxSize.Width {
		w = maxSize.Width
	}
	if h > maxSize.Height {
		h = maxSize.Height
	}
	return enforceDialogMin(fyne.NewSize(w, h))
}

func scrollWithMin(obj fyne.CanvasObject, min fyne.Size) *container.Scroll {
	s := container.NewScroll(obj)
	s.SetMinSize(min)
	return s
}

// withMinSize ensures content reports at least min as its MinSize so entries and splits expand.
func withMinSize(content fyne.CanvasObject, min fyne.Size) fyne.CanvasObject {
	if min.Width <= 0 && min.Height <= 0 {
		return content
	}
	spacer := canvas.NewRectangle(color.Transparent)
	spacer.SetMinSize(min)
	return container.NewStack(spacer, content)
}

// minContentSizeFor returns the minimum content area for each action dialog type.
// Width drives form field expansion; height is a floor for scroll/split layouts (0 = intrinsic).
func minContentSizeFor(action actions.ActionInterface) fyne.Size {
	switch action.(type) {
	case *actions.ImageSearch:
		return fyne.NewSize(960, 720)
	case *actions.Move:
		return fyne.NewSize(640, 480)
	case *actions.Conditional:
		return fyne.NewSize(wideFormMinW, 520)
	case *actions.Ocr:
		return fyne.NewSize(680, 560)
	case *actions.ForEachRow:
		return fyne.NewSize(wideFormMinW, 620)
	case *actions.FindPixel:
		return fyne.NewSize(760, 500)
	case *actions.FocusWindow:
		return fyne.NewSize(520, 480)
	case *actions.Calculate:
		return fyne.NewSize(640, 360)
	case *actions.Wait:
		return fyne.NewSize(500, 0)
	case *actions.Loop, *actions.SetVariable, *actions.SaveVariable:
		return fyne.NewSize(600, 0)
	case *actions.Click, *actions.Key:
		return fyne.NewSize(360, 0)
	case *actions.Type, *actions.RunMacro:
		return fyne.NewSize(400, 0)
	case *actions.Break, *actions.Continue:
		return fyne.NewSize(400, 0)
	default:
		return fyne.NewSize(standardFormMinW, 0)
	}
}

func wrapActionDialogContent(action actions.ActionInterface, content fyne.CanvasObject) fyne.CanvasObject {
	return withMinSize(content, minContentSizeFor(action))
}

// buildActionDialogPanel assembles title, action fields, footer, and bordered chrome.
func buildActionDialogPanel(title string, content, footer fyne.CanvasObject) fyne.CanvasObject {
	titleLabel := ttwidget.NewLabel(title)
	titleLabel.TextStyle = fyne.TextStyle{Bold: true}
	dialogContent := container.NewBorder(
		container.NewPadded(titleLabel),
		footer,
		nil,
		nil,
		content,
	)

	th := fyne.CurrentApp().Settings().Theme()
	v := fyne.CurrentApp().Settings().ThemeVariant()
	panelBg := canvas.NewRectangle(th.Color(theme.ColorNameOverlayBackground, v))
	panelBg.CornerRadius = theme.InputRadiusSize()
	border := canvas.NewRectangle(color.Transparent)
	border.StrokeColor = th.Color(theme.ColorNamePrimary, v)
	border.StrokeWidth = 1
	border.CornerRadius = theme.InputRadiusSize()
	innerPadded := container.NewPadded(container.NewPadded(container.NewPadded(container.NewPadded(dialogContent))))
	return container.NewStack(panelBg, border, innerPadded)
}
