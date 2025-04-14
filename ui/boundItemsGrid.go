package ui

import (
	"Squire/internal/assets"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/widget"
)

func (at *actionTabs) FcreateItemsCheckTree() *widget.GridWrap {
	var (
		icons = *assets.BytesToFyneIcons()
		g     *widget.GridWrap
		data  = binding.NewStringList()
	)
	data.Set([]string{"Healing Potion", "Bandage"})
	// selected := []string{}

	g = widget.NewGridWrapWithData(
		data,
		func() fyne.CanvasObject {
			return container.NewGridWrap(fyne.NewSquareSize(40),
				widget.NewIcon(nil),
				// widget.NewButtonWithIcon("", theme.BrokenImageIcon(), func() {}),
			)
		},
		func(di binding.DataItem, co fyne.CanvasObject) {
			// c := co.(*fyne.Container) //.Objects[0].(*fyne.Container)
			// wb := c.Objects[0].(*widget.Icon)
			bstr := di.(binding.String)
			uid, _ := bstr.Get()
			path := uid + ".png"
			if icons[path] == nil {
				return
			}
			// wb.SetIcon(icons[path])
			// wb.OnTapped = func() {
			// 	if wb.Text == "" {
			// 		selected = append(selected, uid)
			// 		wb.Importance = widget.SuccessImportance
			// 		wb.SetText(" ")
			// 	} else {
			// 		i := slices.Index(selected, uid)
			// 		if i != -1 {
			// 			selected = slices.Delete(selected, i, i+1)
			// 			wb.Importance = widget.MediumImportance
			// 			wb.SetText("")
			// 		}
			// 	}
			// 	log.Println(selected)
			// }
		},
	)
	return g
}
