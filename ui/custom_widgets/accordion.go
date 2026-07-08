package custom_widgets

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

// AccordionWithHeaderWidgets embeds Fyne's Accordion and adds an optional right-side widget
// per item, rendered on the same row as the accordion header (same X axis).
type AccordionWithHeaderWidgets struct {
	widget.Accordion
	headerWidgets []fyne.CanvasObject

	// openStateRefreshIndices, when non-empty, tells Refresh to update only those rows'
	// open/closed visuals instead of rebuilding every header (expand/collapse path).
	openStateRefreshIndices []int

	// renderFullSyncs and renderIncrementalToggles count renderer work for diagnostics/tests.
	renderFullSyncs          int
	renderIncrementalToggles int
}

// NewAccordionWithHeaderWidgets creates an accordion that supports an optional widget at the
// right end of each item's header row. Use AppendWithHeader to add items with a header widget.
//
// The embedded Accordion must NOT be copied from widget.NewAccordion(): that sets BaseWidget.impl
// to a separate *Accordion on the heap. ExtendBaseWidget would then no-op (impl already set), so
// Refresh/Open would update an off-tree widget and the visible accordion would never repaint.
func NewAccordionWithHeaderWidgets() *AccordionWithHeaderWidgets {
	a := &AccordionWithHeaderWidgets{}
	a.ExtendBaseWidget(a)
	return a
}

// Append appends an item with no right-side header widget (editor Items tab). Prefer AppendWithHeader when
// using a header control; this shadows the embedded Accordion.Append so Refresh runs on this widget.
func (a *AccordionWithHeaderWidgets) Append(item *widget.AccordionItem) {
	a.AppendWithHeader(item, nil)
}

// AppendWithHeader appends an accordion item and an optional widget shown at the right end of its header row.
// headerWidget may be nil to show no widget.
// Do not call the embedded Accordion.Append: it Refresh()es with the inner Accordion receiver, so the
// AccordionWithHeaderWidgets renderer (header row + headerWidgets) would not reliably update—same bug
// as Points/Search Areas accordions avoiding a stale UI is fixed by refreshing this outer widget.
func (a *AccordionWithHeaderWidgets) AppendWithHeader(item *widget.AccordionItem, headerWidget fyne.CanvasObject) {
	a.Items = append(a.Items, item)
	a.headerWidgets = append(a.headerWidgets, headerWidget)
	a.Refresh()
}

// RemoveAll removes all items and header widgets so the accordion can be repopulated (e.g. when filter changes).
func (a *AccordionWithHeaderWidgets) RemoveAll() {
	a.Items = a.Items[:0]
	a.headerWidgets = a.headerWidgets[:0]
	a.Refresh()
}

// Close collapses the item at the given index.
func (a *AccordionWithHeaderWidgets) Close(index int) {
	if index < 0 || index >= len(a.Items) {
		return
	}
	if !a.Items[index].Open {
		return
	}
	a.Items[index].Open = false
	a.refreshOpenState([]int{index})
}

// Open expands the item at the given index. GridWrap detail is refreshed once after layout so
// column count matches the available width.
func (a *AccordionWithHeaderWidgets) Open(index int) {
	if index < 0 || index >= len(a.Items) {
		return
	}
	affected := []int{index}
	for i, ai := range a.Items {
		if i == index {
			ai.Open = true
		} else if !a.MultiOpen && ai.Open {
			ai.Open = false
			affected = append(affected, i)
		}
	}
	a.refreshOpenState(affected)
	a.scheduleGridWrapRefreshAfterOpen(index)
}

// SetItems replaces all accordion items and optional header widgets in one refresh.
// headers may be nil when no row has a header widget.
func (a *AccordionWithHeaderWidgets) SetItems(items []*widget.AccordionItem, headers []fyne.CanvasObject) {
	a.Items = items
	if headers == nil {
		headers = make([]fyne.CanvasObject, len(items))
	}
	a.headerWidgets = headers
	a.Refresh()
}

// AppendItem appends an accordion item and optional header widget in one refresh.
func (a *AccordionWithHeaderWidgets) AppendItem(item *widget.AccordionItem, header fyne.CanvasObject) {
	a.Items = append(a.Items, item)
	a.headerWidgets = append(a.headerWidgets, header)
	a.Refresh()
}

// RemoveItemAt removes the accordion item at index.
func (a *AccordionWithHeaderWidgets) RemoveItemAt(index int) {
	if index < 0 || index >= len(a.Items) {
		return
	}
	a.Items = append(a.Items[:index], a.Items[index+1:]...)
	if index < len(a.headerWidgets) {
		a.headerWidgets = append(a.headerWidgets[:index], a.headerWidgets[index+1:]...)
	} else if len(a.headerWidgets) > len(a.Items) {
		a.headerWidgets = a.headerWidgets[:len(a.Items)]
	}
	a.Refresh()
}

// RefreshHeaderWidgets redraws optional right-side header controls without rebuilding accordion rows.
func (a *AccordionWithHeaderWidgets) RefreshHeaderWidgets() {
	for _, hw := range a.headerWidgets {
		if hw != nil {
			hw.Refresh()
		}
	}
}

// RenderStats returns how often the accordion renderer ran a full header rebuild
// (updateObjects) vs an incremental open/close update (syncRowOpenState only).
func (a *AccordionWithHeaderWidgets) RenderStats() (fullSyncs, incrementalToggles int) {
	return a.renderFullSyncs, a.renderIncrementalToggles
}

// ResetRenderStats clears RenderStats counters.
func (a *AccordionWithHeaderWidgets) ResetRenderStats() {
	a.renderFullSyncs = 0
	a.renderIncrementalToggles = 0
}

func (a *AccordionWithHeaderWidgets) UpdateHeaderAt(index int, header fyne.CanvasObject) {
	for len(a.headerWidgets) < len(a.Items) {
		a.headerWidgets = append(a.headerWidgets, nil)
	}
	if index < 0 || index >= len(a.Items) {
		return
	}
	a.headerWidgets[index] = header
	a.Refresh()
}

func (a *AccordionWithHeaderWidgets) refreshOpenState(indices []int) {
	a.openStateRefreshIndices = indices
	a.Refresh()
	a.openStateRefreshIndices = nil
}

func (a *AccordionWithHeaderWidgets) scheduleGridWrapRefreshAfterOpen(index int) {
	if index < 0 || index >= len(a.Items) || !a.Items[index].Open {
		return
	}
	// Defer until after accordion layout so GridWrap has a real width for column count.
	fyne.Do(func() {
		if index >= len(a.Items) || !a.Items[index].Open {
			return
		}
		if gw := FindGridWrap(a.Items[index].Detail); gw != nil {
			RefreshGridWrapPreservingScroll(gw)
		}
	})
}

// CreateRenderer returns a renderer that lays out headers (title + optional right widget) and details like the standard accordion.
func (a *AccordionWithHeaderWidgets) CreateRenderer() fyne.WidgetRenderer {
	r := &accordionWithHeaderRenderer{acc: a}
	r.updateObjects()
	return r
}

type accordionWithHeaderRenderer struct {
	acc          *AccordionWithHeaderWidgets
	headers      []*widget.Button
	headerRows   []fyne.CanvasObject
	headerRowHW  []fyne.CanvasObject // cached right-side widget per row; avoids rebuilding border rows
	dividers     []fyne.CanvasObject
	objects      []fyne.CanvasObject
	minSizeCache fyne.Size
}

func (r *accordionWithHeaderRenderer) Objects() []fyne.CanvasObject {
	return r.objects
}

func (r *accordionWithHeaderRenderer) Destroy() {}

func (r *accordionWithHeaderRenderer) Layout(size fyne.Size) {
	th := r.acc.Theme()
	pad := th.Size(theme.SizeNamePadding)
	sep := th.Size(theme.SizeNameSeparatorThickness)
	divOff := (pad + sep) / 2
	x := float32(0)
	y := float32(0)
	hasOpen := 0

	for i, ai := range r.acc.Items {
		row := r.headerRows[i]
		minH := row.MinSize().Height
		y += minH
		if ai.Open {
			y += pad + ai.Detail.MinSize().Height
			hasOpen++
		}
		if i < len(r.acc.Items)-1 {
			y += pad
		}
	}

	extra := float32(0)
	if hasOpen > 0 {
		extra = (size.Height - y) / float32(hasOpen)
		if extra < 0 {
			extra = 0
		}
	}
	y = 0
	for i, ai := range r.acc.Items {
		if i > 0 {
			div := r.dividers[i-1]
			div.Move(fyne.NewPos(x, y-divOff))
			div.Resize(fyne.NewSize(size.Width, sep))
		}

		row := r.headerRows[i]
		row.Move(fyne.NewPos(x, y))
		minH := row.MinSize().Height
		row.Resize(fyne.NewSize(size.Width, minH))
		y += minH

		if ai.Open {
			y += pad
			d := ai.Detail
			d.Move(fyne.NewPos(x, y))
			openSize := ai.Detail.MinSize().Height + extra
			d.Resize(fyne.NewSize(size.Width, openSize))
			y += openSize
		}
		if i < len(r.acc.Items)-1 {
			y += pad
		}
	}
}

func (r *accordionWithHeaderRenderer) MinSize() fyne.Size {
	if !r.minSizeCache.IsZero() {
		return r.minSizeCache
	}
	if len(r.headerRows) < len(r.acc.Items) {
		r.updateObjects()
	}
	th := r.acc.Theme()
	pad := th.Size(theme.SizeNamePadding)
	size := fyne.Size{}
	for i, ai := range r.acc.Items {
		if i > 0 {
			size.Height += pad
		}
		min := r.headerRows[i].MinSize()
		size.Width = fyne.Max(size.Width, min.Width)
		size.Height += min.Height
		min = ai.Detail.MinSize()
		size.Width = fyne.Max(size.Width, min.Width)
		if ai.Open {
			size.Height += min.Height + pad
		}
	}
	r.minSizeCache = size
	return size
}

func (r *accordionWithHeaderRenderer) Refresh() {
	r.minSizeCache = fyne.Size{}
	if indices := r.acc.openStateRefreshIndices; len(indices) > 0 {
		r.acc.renderIncrementalToggles++
		for _, i := range indices {
			r.syncRowOpenState(i)
		}
	} else {
		r.acc.renderFullSyncs++
		r.updateObjects()
	}
	r.Layout(r.acc.Size())
	canvas.Refresh(r.acc)
}

func (r *accordionWithHeaderRenderer) syncRowOpenState(i int) {
	if i < 0 || i >= len(r.acc.Items) {
		return
	}
	if i >= len(r.headers) {
		r.acc.renderFullSyncs++
		r.updateObjects()
		return
	}
	ai := r.acc.Items[i]
	h := r.headers[i]
	th := r.acc.Theme()
	if ai.Open {
		h.Icon = th.Icon(theme.IconNameArrowDropUp)
		ai.Detail.Show()
	} else {
		h.Icon = th.Icon(theme.IconNameArrowDropDown)
		ai.Detail.Hide()
	}
	h.Refresh()
}

func (r *accordionWithHeaderRenderer) updateObjects() {
	th := r.acc.Theme()
	items := r.acc.Items
	n := len(items)

	// Ensure we have enough headers and header rows
	for len(r.headers) < n {
		r.headers = append(r.headers, &widget.Button{})
		r.headerRows = append(r.headerRows, nil)
		r.headerRowHW = append(r.headerRowHW, nil)
	}
	r.headers = r.headers[:n]
	r.headerRows = r.headerRows[:n]
	r.headerRowHW = r.headerRowHW[:n]

	for i, ai := range items {
		h := r.headers[i]
		h.Alignment = widget.ButtonAlignLeading
		h.IconPlacement = widget.ButtonIconLeadingText
		h.Hidden = false
		h.Importance = widget.LowImportance
		h.Text = ai.Title
		idx := i
		h.OnTapped = func() {
			if r.acc.Items[idx].Open {
				r.acc.Close(idx)
			} else {
				r.acc.Open(idx)
			}
		}
		if ai.Open {
			h.Icon = th.Icon(theme.IconNameArrowDropUp)
			ai.Detail.Show()
		} else {
			h.Icon = th.Icon(theme.IconNameArrowDropDown)
			ai.Detail.Hide()
		}
		h.Refresh()

		var hw fyne.CanvasObject
		if i < len(r.acc.headerWidgets) {
			hw = r.acc.headerWidgets[i]
		}
		if hw != nil {
			if r.headerRowHW[i] != hw {
				r.headerRowHW[i] = hw
				r.headerRows[i] = container.NewBorder(nil, nil, nil, hw, h)
			}
		} else {
			r.headerRowHW[i] = nil
			r.headerRows[i] = h
		}
	}

	for i := n; i < len(r.headers); i++ {
		r.headers[i].Hide()
	}

	// Dividers (reuse existing, create more only if needed). n==0 => n-1 is -1; [:n-1] panics.
	for len(r.dividers) < n-1 {
		r.dividers = append(r.dividers, widget.NewSeparator())
	}
	if n > 0 {
		r.dividers = r.dividers[:n-1]
	} else {
		r.dividers = r.dividers[:0]
	}

	objects := make([]fyne.CanvasObject, 0, n*2+len(r.dividers))
	for _, row := range r.headerRows {
		objects = append(objects, row)
	}
	for _, ai := range r.acc.Items {
		objects = append(objects, ai.Detail)
	}
	for _, d := range r.dividers {
		objects = append(objects, d)
	}
	r.objects = objects
}
