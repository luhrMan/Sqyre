//go:build !android

package ui

import "fyne.io/fyne/v2"

// androidPermissionsSection returns nil on non-Android (no permissions card).
func androidPermissionsSection() fyne.CanvasObject {
	return nil
}
