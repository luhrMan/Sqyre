package services

import (
	"Sqyre/internal/models"
	"Sqyre/internal/screen"
	"fmt"
)

// ApplyMonitorBuiltinVariables sets monitor1Width, monitor1Height, ... from current display bounds.
func ApplyMonitorBuiltinVariables(m *models.Macro) {
	if m == nil {
		return
	}
	n := screen.NumDisplays()
	for i := range n {
		b := screen.DisplayBoundsAbs(i)
		monitorNum := i + 1
		setMacroVariable(m, fmt.Sprintf("monitor%dWidth", monitorNum), b.Dx())
		setMacroVariable(m, fmt.Sprintf("monitor%dHeight", monitorNum), b.Dy())
	}
}
