//go:build !js

package macro

import "github.com/go-vgo/robotgo"

func applyMacroGlobalDelays(ms int) {
	robotgo.MouseSleep = ms
	robotgo.KeySleep = ms
}
