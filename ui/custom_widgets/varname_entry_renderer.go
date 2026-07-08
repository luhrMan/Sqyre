package custom_widgets

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
)

type varNameEntryRendererWrap struct {
	inner   fyne.WidgetRenderer
	overlay *variableNameOverlay
	entry   *VarNameEntry
}

func (r *varNameEntryRendererWrap) Destroy() {
	r.inner.Destroy()
}

func (r *varNameEntryRendererWrap) Layout(size fyne.Size) {
	r.inner.Layout(size)
	r.layoutOverlay()
}

func (r *varNameEntryRendererWrap) layoutOverlay() {
	content := r.textContentObject()
	if content == nil {
		return
	}
	pos := content.Position()
	area := content.Size()
	host := r.overlay.object(r.entry)
	host.Resize(area)
	host.Move(pos)
}

func (r *varNameEntryRendererWrap) MinSize() fyne.Size {
	return r.inner.MinSize()
}

func (r *varNameEntryRendererWrap) Objects() []fyne.CanvasObject {
	objs := r.inner.Objects()
	objs = append(objs, r.overlay.object(r.entry))
	return objs
}

func (r *varNameEntryRendererWrap) Refresh() {
	r.entry.syncPillDisplay()
	r.inner.Refresh()
	show := r.entry.hideTextForPills
	r.setTextContentVisible(!show)
	r.overlay.sync(r.entry.Text, show, r.entry.knownVariables(), r.entry.pillOverlayBorderless)
	r.layoutOverlay()
}

func (r *varNameEntryRendererWrap) textContentObject() fyne.CanvasObject {
	for _, obj := range r.inner.Objects() {
		if scroll, ok := obj.(*container.Scroll); ok {
			return scroll
		}
	}
	objs := r.inner.Objects()
	if len(objs) >= 3 {
		return objs[2]
	}
	return nil
}

func (r *varNameEntryRendererWrap) setTextContentVisible(visible bool) {
	content := r.textContentObject()
	if content == nil {
		return
	}
	if visible {
		content.Show()
	} else {
		content.Hide()
	}
}
