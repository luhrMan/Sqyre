package custom_widgets

import "fyne.io/fyne/v2"

type varNameEntryRendererWrap struct {
	inner fyne.WidgetRenderer
	entry *VarNameEntry
}

func (r *varNameEntryRendererWrap) Destroy() {
	r.inner.Destroy()
}

func (r *varNameEntryRendererWrap) Layout(size fyne.Size) {
	r.inner.Layout(size)
}

func (r *varNameEntryRendererWrap) MinSize() fyne.Size {
	return r.inner.MinSize()
}

func (r *varNameEntryRendererWrap) Objects() []fyne.CanvasObject {
	return r.inner.Objects()
}

func (r *varNameEntryRendererWrap) Refresh() {
	r.inner.Refresh()
}
