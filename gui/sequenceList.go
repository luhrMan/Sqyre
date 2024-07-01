package gui

import (
	"Dark-And-Darker/actions"
	"strconv"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

var macro actions.Macro

var sequenceList = widget.NewList(
	func() int { return len(macro.Sequences) },
	func() fyne.CanvasObject { return container.NewHBox(widget.NewLabel("Text")) },
	func(lii widget.ListItemID, co fyne.CanvasObject) {
		co.(*fyne.Container).Objects[0].(*widget.Label).SetText(macro.Sequences[lii].Name + " x" + strconv.FormatInt(int64(macro.Sequences[lii].Loops), 10))
		// for i, action := range macro.Sequences[lii].Actions {
		// 	co.(*fyne.Container).Objects[i].(*widget.Label).SetText(action.PrintParams())
		// }
	},
)
