//go:build js

package ui

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/data/binding"
)

func saveNativeWindowBounds(w fyne.Window, prefs fyne.Preferences) {}

func onCloseWithoutTray() {}

func runMousePositionReadout() {
	locX, locY := 0, 0
	blocX, blocY := binding.BindInt(&locX), binding.BindInt(&locY)
	boundLocXLabel.Bind(binding.IntToString(blocX))
	boundLocYLabel.Bind(binding.IntToString(blocY))
}
