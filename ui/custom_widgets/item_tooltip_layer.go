package custom_widgets

import (
	"errors"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

// ItemTooltipLayer holds pass-through item tooltips in the window content stack (not canvas
// overlays), so clicks reach widgets underneath.
type ItemTooltipLayer struct {
	Container fyne.Container
	overlays  map[fyne.CanvasObject]*ItemTooltipLayer
}

var itemTooltipLayers = make(map[fyne.Canvas]*ItemTooltipLayer)

// AddWindowItemTooltipLayer stacks an item-tooltip layer above windowContent. Call when setting
// window content, after fynetooltip.AddWindowToolTipLayer if both are used.
func AddWindowItemTooltipLayer(windowContent fyne.CanvasObject, canvas fyne.Canvas) fyne.CanvasObject {
	layer := &ItemTooltipLayer{}
	itemTooltipLayers[canvas] = layer
	return container.NewStack(windowContent, &layer.Container)
}

// AddPopUpItemTooltipLayer adds an item-tooltip layer to a pop-up (e.g. action dialog).
// Call after the pop-up is created with content and after fynetooltip.AddPopUpToolTipLayer.
func AddPopUpItemTooltipLayer(pop *widget.PopUp) {
	root := itemTooltipLayers[pop.Canvas]
	if root == nil {
		fyne.LogError("item tooltip layer", errors.New("no item tooltip layer for parent canvas"))
		return
	}
	layer := &ItemTooltipLayer{}
	if root.overlays == nil {
		root.overlays = make(map[fyne.CanvasObject]*ItemTooltipLayer)
	}
	root.overlays[pop] = layer
	pop.Content = container.NewStack(pop.Content, &layer.Container)
}

func findItemTooltipLayer(canvas fyne.Canvas, overlay fyne.CanvasObject) *ItemTooltipLayer {
	root := itemTooltipLayers[canvas]
	if root == nil {
		return nil
	}
	if overlay != nil {
		if nested := root.overlays[overlay]; nested != nil {
			return nested
		}
	}
	return root
}

func itemTooltipLayerOrigin(layer *ItemTooltipLayer, overlay fyne.CanvasObject) fyne.Position {
	if pop, ok := overlay.(*widget.PopUp); ok && pop != nil {
		return pop.Content.Position()
	}
	return fyne.CurrentApp().Driver().AbsolutePositionForObject(&layer.Container)
}
