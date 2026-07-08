package macro

import (
	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2"
)

const tooltipGeometryEpsilon float32 = 0.5

func (h *actionDisplayTooltipHover) pointerInTreeActionSpace(pos fyne.Position) bool {
	return h.pointerInRowKeepAlive(pos)
}

func pointInCachedRect(abs, origin fyne.Position, size fyne.Size) bool {
	return abs.X >= origin.X && abs.Y >= origin.Y &&
		abs.X < origin.X+size.Width && abs.Y < origin.Y+size.Height
}

func (h *actionDisplayTooltipHover) refreshKeepAliveGeometry() {
	driver := fyne.CurrentApp().Driver()
	h.cachedSelfOrigin = driver.AbsolutePositionForObject(h)
	h.cachedSelfSize = h.Size()
	if h.keepAliveArea != nil && h.keepAliveArea.Visible() {
		h.cachedKeepAliveOrigin = driver.AbsolutePositionForObject(h.keepAliveArea)
		h.cachedKeepAliveSize = h.keepAliveArea.Size()
	}
	h.keepAliveGeometryOK = true
}

func (h *actionDisplayTooltipHover) clearKeepAliveGeometryCache() {
	h.keepAliveGeometryOK = false
}

func (h *actionDisplayTooltipHover) refreshTooltipPanelGeometry(origin, relPos fyne.Position, size fyne.Size) {
	h.cachedPanelOrigin = origin.Add(relPos)
	h.cachedPanelSize = size
	h.tooltipPanelGeometryOK = true
}

func (h *actionDisplayTooltipHover) clearTooltipPanelGeometryCache() {
	h.tooltipPanelGeometryOK = false
}

func (h *actionDisplayTooltipHover) pointerInTooltipPanel(pos fyne.Position) bool {
	if h.tooltipPanel == nil || !h.tooltipPanelGeometryOK {
		return false
	}
	return pointInCachedRect(pos, h.cachedPanelOrigin, h.cachedPanelSize)
}

func (h *actionDisplayTooltipHover) pointerInRowKeepAlive(pos fyne.Position) bool {
	if h.pointerInKeepAliveExclude(pos) {
		return false
	}
	if !h.keepAliveGeometryOK {
		h.refreshKeepAliveGeometry()
	}
	if h.keepAliveArea != nil && h.keepAliveArea.Visible() {
		if pointInCachedRect(pos, h.cachedKeepAliveOrigin, h.cachedKeepAliveSize) {
			return true
		}
	}
	return pointInCachedRect(pos, h.cachedSelfOrigin, h.cachedSelfSize)
}

func (h *actionDisplayTooltipHover) pointerInKeepAliveExclude(pos fyne.Position) bool {
	if h.keepAliveExclude == nil || !h.keepAliveExclude.Visible() {
		return false
	}
	driver := fyne.CurrentApp().Driver()
	origin := driver.AbsolutePositionForObject(h.keepAliveExclude)
	size := h.keepAliveExclude.Size()
	return pointInCachedRect(pos, origin, size)
}

func (h *actionDisplayTooltipHover) tooltipPosition(c fyne.Canvas, origin fyne.Position) (fyne.Size, fyne.Position) {
	size := h.tooltipPanel.measureLayoutSize(c)
	if h.tooltipPinned() && h.tooltipPanelGeometryOK {
		return size, actionDisplayTooltipPositionClamped(c, h.tooltipPanel.Position(), size)
	}
	mousePos := h.absoluteMousePos.Subtract(origin)
	return size, actionDisplayTooltipPosition(c, mousePos, size)
}

func fyneSizesClose(a, b fyne.Size) bool {
	return abs32(a.Width-b.Width) <= tooltipGeometryEpsilon &&
		abs32(a.Height-b.Height) <= tooltipGeometryEpsilon
}

func fynePositionsClose(a, b fyne.Position) bool {
	return abs32(a.X-b.X) <= tooltipGeometryEpsilon &&
		abs32(a.Y-b.Y) <= tooltipGeometryEpsilon
}

func abs32(v float32) float32 {
	if v < 0 {
		return -v
	}
	return v
}

func actionDisplayTooltipSizeAndPosition(panel *actionDisplayTooltipPanel, c fyne.Canvas, mousePos fyne.Position) (fyne.Size, fyne.Position) {
	size := panel.measureLayoutSize(c)
	return size, actionDisplayTooltipPosition(c, mousePos, size)
}

func actionDisplayTooltipPosition(c fyne.Canvas, mousePos fyne.Position, size fyne.Size) fyne.Position {
	canvasSize := c.Size()
	edgeMarginX := canvasSize.Width * custom_widgets.TooltipEdgeMarginFraction
	edgeMarginY := canvasSize.Height * custom_widgets.TooltipEdgeMarginFraction

	pos := mousePos
	if rightEdge := pos.X + size.Width; rightEdge > canvasSize.Width-edgeMarginX {
		pos.X -= rightEdge - canvasSize.Width + edgeMarginX
	}
	if pos.X < edgeMarginX {
		pos.X = edgeMarginX
	}
	const belowMouseDist = 16
	const aboveMouseDist = 8
	if bottomEdge := pos.Y + size.Height + belowMouseDist; bottomEdge > canvasSize.Height-edgeMarginY {
		pos.Y -= size.Height + aboveMouseDist
	} else {
		pos.Y += belowMouseDist
	}
	if pos.Y < edgeMarginY {
		pos.Y = edgeMarginY
	}
	return pos
}

// actionDisplayTooltipPositionClamped keeps an existing tooltip anchor and only nudges it
// when a larger size would clip past the canvas edge (e.g. growing into edit mode).
func actionDisplayTooltipPositionClamped(c fyne.Canvas, pos fyne.Position, size fyne.Size) fyne.Position {
	canvasSize := c.Size()
	edgeMarginX := canvasSize.Width * custom_widgets.TooltipEdgeMarginFraction
	edgeMarginY := canvasSize.Height * custom_widgets.TooltipEdgeMarginFraction

	if rightEdge := pos.X + size.Width; rightEdge > canvasSize.Width-edgeMarginX {
		pos.X -= rightEdge - canvasSize.Width + edgeMarginX
	}
	if pos.X < edgeMarginX {
		pos.X = edgeMarginX
	}
	if bottomEdge := pos.Y + size.Height; bottomEdge > canvasSize.Height-edgeMarginY {
		pos.Y -= bottomEdge - (canvasSize.Height - edgeMarginY)
	}
	if pos.Y < edgeMarginY {
		pos.Y = edgeMarginY
	}
	return pos
}
