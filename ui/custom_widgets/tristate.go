package custom_widgets

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
)

// TriStateSelectAll shows a checkbox in one of three states: empty (none selected),
// half (some selected), or full (all selected). Clicking it triggers onTapped (e.g. select all / deselect all).
// getState is called on Refresh() to update the display; 0 = empty, 1 = half, 2 = full.
type TriStateSelectAll struct {
	widget.BaseWidget
	getState func() int
	onTapped func()
	check    *widget.Check
}

func (t *TriStateSelectAll) applyState() {
	if t.getState == nil {
		return
	}
	s := t.getState()
	t.check.Checked = (s == 2)
	t.check.Partial = (s == 1)
}

// NewTriStateSelectAll creates a tri-state checkbox. getState returns 0 (empty), 1 (half), or 2 (full).
// onTapped is called when the user clicks (e.g. to toggle select all / deselect all).
func NewTriStateSelectAll(getState func() int, onTapped func()) *TriStateSelectAll {
	t := &TriStateSelectAll{getState: getState, onTapped: onTapped}
	t.check = widget.NewCheck("", func(bool) {
		if t.onTapped != nil {
			t.onTapped()
		}
		// The underlying Check updates its own state on tap; immediately re-apply our tri-state
		// derived from getState so the UI doesn't drift.
		t.applyState()
		t.check.Refresh()
	})
	t.ExtendBaseWidget(t)
	t.applyState()
	return t
}

// CreateRenderer returns a renderer that displays the internal check.
func (t *TriStateSelectAll) CreateRenderer() fyne.WidgetRenderer {
	return &triStateRenderer{tri: t, check: t.check}
}

// Refresh updates the checkbox state from getState (do not call BaseWidget.Refresh to avoid recursion).
func (t *TriStateSelectAll) Refresh() {
	t.applyState()
	t.check.Refresh()
}

type triStateRenderer struct {
	tri   *TriStateSelectAll
	check *widget.Check
}

func (r *triStateRenderer) Objects() []fyne.CanvasObject {
	return []fyne.CanvasObject{r.check}
}

func (r *triStateRenderer) Layout(size fyne.Size) {
	r.check.Resize(size)
	r.check.Move(fyne.NewPos(0, 0))
}

func (r *triStateRenderer) MinSize() fyne.Size {
	return r.check.MinSize()
}

func (r *triStateRenderer) Refresh() {
	r.tri.applyState()
	r.check.Refresh()
}

func (r *triStateRenderer) Destroy() {}
