package completionentry

import "fyne.io/fyne/v2"

// IsNavListFocused reports whether keyboard focus is on the completion popup list.
func IsNavListFocused(f fyne.Focusable) bool {
	_, ok := f.(*navigableList)
	return ok
}
