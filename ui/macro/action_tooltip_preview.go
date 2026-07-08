package macro

import (
	"context"

	"Sqyre/internal/vision"
	"Sqyre/ui/actiondisplay"
	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

func (h *actionDisplayTooltipHover) clearPreviewCache() {
	h.previewCacheReady = false
	h.previewCache = custom_widgets.PreviewTooltipResult{}
	h.previewCacheErr = nil
}

func (h *actionDisplayTooltipHover) applyPreviewCache(panel *actionDisplayTooltipPanel) {
	if !h.previewCacheReady || panel == nil {
		return
	}
	if h.previewCacheErr != nil {
		panel.setPreviewError(h.previewCacheErr.Error())
		return
	}
	panel.setPreviewImage(h.previewCache.Image, h.previewCache.Caption)
}

func (h *actionDisplayTooltipHover) previewCaptureInFlight() bool {
	return h.captureCtx != nil
}

func (h *actionDisplayTooltipHover) beginPreviewCapture() {
	if actionTooltipEditPinnedByOther(h) {
		return
	}
	if h.tooltipPanel == nil || !h.isTooltipMounted() {
		h.showTooltipPanel()
	}
	if h.tooltipPanel == nil {
		return
	}
	h.startPreviewCapture()
}

func (h *actionDisplayTooltipHover) startPreviewCapture() {
	h.capturePreview(false, true)
}

func (h *actionDisplayTooltipHover) capturePreview(force, reposition bool) {
	if h.previewLoader == nil || h.tooltipPanel == nil {
		return
	}
	if !force {
		if h.previewCaptureInFlight() {
			return
		}
		if h.previewCacheReady {
			h.applyPreviewCache(h.tooltipPanel)
			if reposition {
				h.repositionTooltip()
			}
			return
		}
	} else {
		h.clearPreviewCache()
		h.invalidatePreviewVisionCache()
	}

	panel := h.tooltipPanel
	c := h.hoverCanvas()
	if c == nil {
		return
	}

	h.cancelCapture()
	panel.setPreviewLoading()
	panel.Refresh()
	if reposition {
		h.repositionTooltip()
	}

	load := h.previewLoader
	ctx, cancel := context.WithCancel(context.Background())
	h.captureCtx = ctx
	h.captureCancel = cancel
	go func() {
		if !custom_widgets.AcquirePreviewCaptureSlot(ctx) {
			return
		}
		defer custom_widgets.ReleasePreviewCaptureSlot()
		if ctx.Err() != nil {
			return
		}
		result, err := load()
		if ctx.Err() != nil {
			return
		}
		fyne.Do(func() {
			if ctx.Err() != nil || h.captureCtx != ctx || h.tooltipPanel != panel {
				return
			}
			h.previewCache = result
			h.previewCacheErr = err
			h.previewCacheReady = true
			if !h.shouldKeepViewTooltip() {
				return
			}
			h.captureCancel = nil
			h.captureCtx = nil
			if err != nil {
				panel.setPreviewError(err.Error())
			} else {
				panel.setPreviewImage(result.Image, result.Caption)
			}
			panel.Refresh()
			if reposition {
				h.repositionTooltip()
			}
		})
	}()
}

func (h *actionDisplayTooltipHover) reloadPreview() {
	if h.previewLoader == nil || h.tooltipPanel == nil {
		return
	}
	h.capturePreview(true, false)
}

func (h *actionDisplayTooltipHover) invalidatePreviewVisionCache() {
	if h.tooltipPanel != nil && h.tooltipPanel.editing && h.tooltipPanel.editForm != nil {
		if ref := h.tooltipPanel.editForm.stagedCoordRef; !ref.IsEmpty() {
			vision.InvalidatePreviewTooltipCacheEntity(ref.Name())
			return
		}
	}
	if ref, ok := coordinateRefForPreview(h.node); ok && !ref.IsEmpty() {
		vision.InvalidatePreviewTooltipCacheEntity(ref.Name())
	}
}

func buildPreviewRefreshOverlay(owner *actionDisplayTooltipHover) fyne.CanvasObject {
	if owner == nil || owner.previewLoader == nil {
		return nil
	}
	refreshBtn := actiondisplay.NewPillIconButton(theme.ViewRefreshIcon(), func() {
		owner.reloadPreview()
	})
	return custom_widgets.StackTopRight(actiondisplay.PillChrome(refreshBtn, owner.actionType))
}
