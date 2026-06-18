package models

import "fmt"

// ImageSearchBuiltinVars are set at runtime inside Image Search sub-actions.
var ImageSearchBuiltinVars = []string{
	"StackMax", "Cols", "Rows", "ItemName", "ImagePixelWidth", "ImagePixelHeight",
}

// MonitorBuiltinVarNames returns monitor1Width, monitor1Height, ... for each display (1-based).
func MonitorBuiltinVarNames(numMonitors int) []string {
	if numMonitors < 1 {
		numMonitors = 1
	}
	names := make([]string, 0, numMonitors*2)
	for i := 1; i <= numMonitors; i++ {
		names = append(names,
			fmt.Sprintf("monitor%dWidth", i),
			fmt.Sprintf("monitor%dHeight", i),
		)
	}
	return names
}
