package macro

import (
	"sort"

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
			return container.NewHBox(widget.NewLabel(""), widget.NewLabel(""))
		},
		func(id widget.ListItemID, obj fyne.CanvasObject) {
			if id < 0 || id >= len(names) {
				return
			}
			row := obj.(*fyne.Container)
			nameLbl := row.Objects[0].(*widget.Label)
			valLbl := row.Objects[1].(*widget.Label)
			name := names[id]
			nameLbl.SetText(name)
			vals := services.GetRuntimeVariables()
			valLbl.SetText(vals[name])
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
