package custom_widgets

import (
	"fyne.io/fyne/v2"
)

type varEntryRendererWrap struct {
	inner   fyne.WidgetRenderer
	overlay *variableRefOverlay
	entry   *VarEntry
}

func (r *varEntryRendererWrap) Destroy() {
	r.inner.Destroy()
}

func (r *varEntryRendererWrap) Layout(size fyne.Size) {
	x := size.Width

	if r.entry.insert != nil {
		insertWidth := r.entry.insert.MinSize().Width
		x -= insertWidth
		r.entry.insert.Resize(fyne.NewSize(insertWidth, size.Height))
		r.entry.insert.Move(fyne.NewPos(x, 0))
	}
	if r.entry.feedbackIcon != nil && !r.entry.feedbackIcon.Hidden {
		iconWidth := r.entry.feedbackIcon.MinSize().Width
		x -= iconWidth
		r.entry.feedbackIcon.Resize(fyne.NewSize(iconWidth, size.Height))
		r.entry.feedbackIcon.Move(fyne.NewPos(x, 0))
	}

	r.inner.Layout(fyne.NewSize(x, size.Height))
	r.layoutOverlay()
}

func (r *varEntryRendererWrap) trailingWidth() float32 {
	var w float32
	if r.entry.feedbackIcon != nil && !r.entry.feedbackIcon.Hidden {
		w += r.entry.feedbackIcon.MinSize().Width
	}
	if r.entry.insert != nil {
		w += r.entry.insert.MinSize().Width
	}
	return w
}

func (r *varEntryRendererWrap) layoutOverlay() {
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

func (r *varEntryRendererWrap) MinSize() fyne.Size {
	min := r.inner.MinSize()
	min.Width += r.trailingWidth()
	return min
}

func (r *varEntryRendererWrap) Objects() []fyne.CanvasObject {
	objs := r.inner.Objects()
	objs = append(objs, r.overlay.object(r.entry))
	if r.entry.feedbackIcon != nil {
		objs = append(objs, r.entry.feedbackIcon)
	}
	if r.entry.insert != nil {
		objs = append(objs, r.entry.insert)
	}
	return objs
}

func (r *varEntryRendererWrap) Refresh() {
	r.entry.UpdateInsertButton()
	r.entry.syncPillDisplay()
	r.inner.Refresh()
	show := r.entry.hideTextForPills
	r.setTextContentVisible(!show)
	r.overlay.sync(r.entry.Text, r.entry.MultiLine, r.entry.TextStyle, show)
	r.layoutOverlay()
}

// textContentObject returns the entry scroll or content canvas object (index 2).
func (r *varEntryRendererWrap) textContentObject() fyne.CanvasObject {
	objs := r.inner.Objects()
	if len(objs) < 3 {
		return nil
	}
	return objs[2]
}

func (r *varEntryRendererWrap) setTextContentVisible(visible bool) {
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
