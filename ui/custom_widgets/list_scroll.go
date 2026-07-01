package custom_widgets

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/widget"
)

// RefreshListPreservingScroll reloads list content without resetting scroll position.
// When content height shrinks (e.g. after delete), the offset is re-applied on the next
// frame so Fyne can clamp it after layout.
func RefreshListPreservingScroll(list *widget.List) {
	if list == nil {
		return
	}
	offset := list.GetScrollOffset()
	list.Refresh()
	restoreListScrollOffset(list, offset)
}

func restoreListScrollOffset(list *widget.List, offset float32) {
	list.ScrollToOffset(offset)
	fyne.Do(func() {
		list.ScrollToOffset(offset)
	})
}

// RefreshGridWrapPreservingScroll reloads grid-wrap content without resetting scroll position.
func RefreshGridWrapPreservingScroll(grid *widget.GridWrap) {
	if grid == nil {
		return
	}
	offset := grid.GetScrollOffset()
	grid.Refresh()
	grid.ScrollToOffset(offset)
	fyne.Do(func() {
		grid.ScrollToOffset(offset)
	})
}
