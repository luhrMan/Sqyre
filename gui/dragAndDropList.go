package gui

// import (
// 	"fyne.io/fyne/v2"
// 	"fyne.io/fyne/v2/widget"
// )

// type CustomList struct {
// 	widget.List
// 	items      []string
// 	dragging   bool
// 	draggedIdx int
// }

// func (l *CustomList) DragEnd() {
// 	l.dragging = false
// 	l.draggedIdx = -1
// 	l.Refresh()
// }

// func (l *CustomList) Tapped(e *fyne.PointEvent) {
// 	idx := l.ChildIndex(widget.At(e.Position))

// 	if idx >= 0 && idx < len(l.items) {
// 		l.dragging = true
// 		l.draggedIdx = idx
// 		l.Refresh()
// 	}
// }

// func (l *CustomList) Dropped(target fyne.DropTarget, data fyne.Draggable) {
// 	if l.draggedIdx >= 0 && l.draggedIdx < len(l.items) {
// 		newIdx := l.ChildIndex(target)
// 		if newIdx >= 0 && newIdx < len(l.items) {
// 			// Rearrange items
// 			item := l.items[l.draggedIdx]
// 			l.items = append(l.items[:l.draggedIdx], l.items[l.draggedIdx+1:]...)
// 			l.items = append(l.items[:newIdx], append([]string{item}, l.items[newIdx:]...)...)
// 		}
// 	}

// 	l.DragEnd()
// }

// func NewCustomList(items []string) *CustomList {
// 	list := &CustomList{
// 		items: items,
// 	}

// 	for _, item := range items {
// 		list.Add(widget.NewLabel(item))
// 	}

// 	list.SetOnTapped(list.Tapped)
// 	list.
// 		list.SetDropReceiver(widget.DropTarget(list))
// 	list.SetOnDragEnd(list.DragEnd)

// 	return list
// }
