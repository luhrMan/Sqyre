//go:build js

package ui

import (
	"Sqyre/internal/config"
	"Sqyre/internal/screen"
	"fmt"
)

func computerInfoText() string {
	vb := screen.VirtualBounds()
	return fmt.Sprintf("Browser demo (logical desktop)\nVirtual bounds: %dx%d at (%d,%d)\nConfig resolution key: %s\n",
		vb.Dx(), vb.Dy(), vb.Min.X, vb.Min.Y, config.MainMonitorSizeString)
}
