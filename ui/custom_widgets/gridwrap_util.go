package custom_widgets

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
)

// FindGridWrap returns the first GridWrap in obj's widget tree, or nil.
func FindGridWrap(obj fyne.CanvasObject) *widget.GridWrap {
	if obj == nil {
		return nil
	}
	if gw, ok := obj.(*widget.GridWrap); ok {
		return gw
	}
	if c, ok := obj.(*fyne.Container); ok {
		for _, child := range c.Objects {
			if gw := FindGridWrap(child); gw != nil {
				return gw
			}
		}
	}
	return nil
}

// RefreshGridWraps walks obj's tree and refreshes every GridWrap found.
func RefreshGridWraps(obj fyne.CanvasObject) {
	if obj == nil {
		return
	}
	if gw, ok := obj.(*widget.GridWrap); ok {
		gw.Refresh()
		return
	}
	if c, ok := obj.(*fyne.Container); ok {
		for _, child := range c.Objects {
			RefreshGridWraps(child)
		}
	}
}
