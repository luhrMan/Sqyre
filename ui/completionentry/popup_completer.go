package completionentry

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
)

// PopupCompleter drives a navigable completion popup for a host widget that may
// be wrapped inside a renderer (so the popup anchors to the host object rather
// than an embedded entry). Selecting an option calls OnSelected instead of
// replacing the entry text, which lets callers insert partial completions such
// as ${variable} references.
type PopupCompleter struct {
	// Host is used for canvas lookup and popup positioning. It should be the
	// widget that is actually placed on the canvas.
	Host fyne.CanvasObject
	// Entry receives keystrokes forwarded from the navigable list.
	Entry *widget.Entry
	// OnSelected is called with the chosen option.
	OnSelected func(string)

	popupMenu     *widget.PopUp
	navigableList *navigableList
	itemHeight    float32
	visible       bool
}

// Show displays the completion popup with the given options.
func (p *PopupCompleter) Show(options []string) {
	p.ShowLabels(options, options)
}

// ShowLabels displays options with separate display labels. OnSelected receives the option value.
func (p *PopupCompleter) ShowLabels(options, labels []string) {
	if len(options) == 0 || p.Host == nil || p.Entry == nil {
		p.Hide()
		return
	}
	if len(labels) != len(options) {
		labels = options
	}
	holder := fyne.CurrentApp().Driver().CanvasForObject(p.Host)
	if holder == nil {
		return
	}

	selected := func(s string) {
		if p.OnSelected != nil {
			p.OnSelected(s)
		}
		p.Hide()
	}

	if p.navigableList == nil {
		p.navigableList = newNavigableList(options, p.Entry, selected, p.Hide, nil, nil)
	} else {
		p.navigableList.SetOptions(options)
	}
	p.navigableList.SetLabels(labels)

	if p.popupMenu == nil {
		p.popupMenu = widget.NewPopUp(p.navigableList, holder)
	}
	geo := p.popupGeometry()
	p.popupMenu.Resize(geo.size)
	p.popupMenu.ShowAtPosition(geo.pos)
	holder.Focus(p.navigableList)
	p.visible = true
}

// Hide hides the completion popup and returns focus to the host widget.
func (p *PopupCompleter) Hide() {
	p.hide(true)
}

// HideWithoutRefocus hides the completion popup without moving keyboard focus.
func (p *PopupCompleter) HideWithoutRefocus() {
	p.hide(false)
}

func (p *PopupCompleter) hide(refocus bool) {
	if p.popupMenu != nil {
		p.popupMenu.Hide()
	}
	if !p.visible {
		return
	}
	p.visible = false
	if p.navigableList != nil {
		p.navigableList.selected = -1
	}
	if !refocus {
		return
	}
	if holder := fyne.CurrentApp().Driver().CanvasForObject(p.Host); holder != nil {
		if f, ok := p.Host.(fyne.Focusable); ok {
			holder.Focus(f)
		}
	}
}

// Visible reports whether the completion popup is currently shown.
func (p *PopupCompleter) Visible() bool {
	return p.visible
}

func (p *PopupCompleter) popupGeometry() popupGeometry {
	count := 0
	if p.navigableList != nil {
		count = len(p.navigableList.items)
	}
	measureItemHeight := func() float32 {
		if p.navigableList == nil {
			return 0
		}
		p.itemHeight = p.navigableList.CreateItem().MinSize().Height
		return p.itemHeight
	}
	return popupGeometryFor(p.Host, count, p.itemHeight, measureItemHeight)
}
