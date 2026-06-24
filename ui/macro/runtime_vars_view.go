package macro

import (
	"sort"

	"Sqyre/internal/models/actions"
	"Sqyre/internal/services"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

func buildRuntimeVariablesView() (*widget.List, func()) {
	var names []string
	varList := widget.NewList(
		func() int { return len(names) },
		func() fyne.CanvasObject {
			return container.NewHBox(
				actions.NewDisplayPill("", "setvariable"),
				actions.NewDisplayPill("", "setvariable"),
			)
		},
		func(id widget.ListItemID, obj fyne.CanvasObject) {
			if id < 0 || id >= len(names) {
				return
			}
			row := obj.(*fyne.Container)
			if len(row.Objects) < 2 {
				return
			}
			name := names[id]
			vals := services.GetRuntimeVariables()
			row.Objects[0] = actions.NewDisplayPill("Name: "+name, "setvariable")
			row.Objects[1] = actions.NewDisplayPill("Value: "+vals[name], "setvariable")
			row.Refresh()
		},
	)
	refresh := func() {
		vals := services.GetRuntimeVariables()
		names = names[:0]
		for n := range vals {
			names = append(names, n)
		}
		sort.Strings(names)
		varList.Refresh()
	}
	refresh()
	return varList, refresh
}
