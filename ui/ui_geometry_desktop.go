//go:build !js

package ui

import (
	"Sqyre/internal/config"
	"Sqyre/internal/services"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/data/binding"
	"github.com/go-vgo/robotgo"
)

func saveNativeWindowBounds(w fyne.Window, prefs fyne.Preferences) {
	pid := robotgo.GetPid()
	x, y, width, height := robotgo.GetBounds(pid)
	if width > 0 && height > 0 {
		prefs.SetInt(config.PrefWindowX, x)
		prefs.SetInt(config.PrefWindowY, y)
		prefs.SetInt(config.PrefWindowWidth, width)
		prefs.SetInt(config.PrefWindowHeight, height)
	}
}

func onCloseWithoutTray() {
	services.LogMatProfile()
}

func runMousePositionReadout() {
	locX, locY := robotgo.Location()
	blocX, blocY := binding.BindInt(&locX), binding.BindInt(&locY)
	boundLocXLabel.Bind(binding.IntToString(blocX))
	boundLocYLabel.Bind(binding.IntToString(blocY))
	services.GoSafe(func() {
		for {
			robotgo.MilliSleep(100)
			newLocX, newLocY := robotgo.Location()
			if locX == newLocX && locY == newLocY {
				continue
			}
			locX, locY = robotgo.Location()
			blocX.Reload()
			blocY.Reload()
		}
	})
}
