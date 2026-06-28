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
	x := size.Width
	if r.entry.insert != nil {
		insertWidth := r.entry.insert.MinSize().Width
		x -= insertWidth
		r.entry.insert.Resize(fyne.NewSize(insertWidth, size.Height))
		r.entry.insert.Move(fyne.NewPos(x, 0))
	}
	r.inner.Layout(fyne.NewSize(x, size.Height))
}

func (r *varNameEntryRendererWrap) MinSize() fyne.Size {
	min := r.inner.MinSize()
	if r.entry.insert != nil {
		min.Width += r.entry.insert.MinSize().Width
	}
	return min
}

func (r *varNameEntryRendererWrap) Objects() []fyne.CanvasObject {
	objs := r.inner.Objects()
	if r.entry.insert != nil {
		objs = append(objs, r.entry.insert)
	}
	return objs
}

func (r *varNameEntryRendererWrap) Refresh() {
	r.entry.UpdateInsertButton()
	r.inner.Refresh()
}
