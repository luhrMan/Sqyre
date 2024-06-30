package gui

import (
	"Dark-And-Darker/actions"
	"Dark-And-Darker/structs"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

var actionsArr []actions.Action

var actionsList = widget.NewList(
	func() int { return len(actionsArr) },
	func() fyne.CanvasObject { return container.NewHBox(widget.NewLabel("Text")) },
	func(lii widget.ListItemID, co fyne.CanvasObject) {
		co.(*fyne.Container).Objects[0].(*widget.Label).SetText(actionsArr[lii].PrintParams())
	},
)

var mouseMoveSelector = widget.NewSelect([]string{"Stash Tab", "Play Tab"}, func(s string) {})
var mouseMoveSettingsForm = widget.Form{
	Items: []*widget.FormItem{
		{Text: "Mouse Move to", Widget: mouseMoveSelector},
	},
	OnSubmit: func() {
		action := actions.MouseMove{
			//Place:       goToSelector.Selected,
			Coordinates: structs.GetSpot(mouseMoveSelector.Selected), //[2]int{structs.GetSpot(goToSelector.Selected).X, structs.GetSpot(goToSelector.Selected).Y},
		}
		actionsArr = append(actionsArr, action)
		actionsList.Refresh()
	},
}

var clickSelector = widget.NewSelect([]string{"Left", "Right"}, func(s string) {})
var clickAmountSlider = widget.NewSlider(0, 50)

// var holdKeysCheckGroup = widget.NewCheckGroup([]string{"Alt", "Shift", "Ctrl"}, func(s []string) {})
var clickSettingsForm = widget.Form{
	Items: []*widget.FormItem{
		{Text: "Amount", Widget: clickAmountSlider},
		{Text: "Button", Widget: clickSelector},
		//{Text: "Hold Keys Down", Widget: holdKeysCheckGroup},
	},
	OnSubmit: func() {
		action := actions.Click{
			Amount: int(clickAmountSlider.Value),
			Button: clickSelector.Selected,
			//KeysHeldDown: holdKeysCheckGroup.Selected,
		}
		actionsArr = append(actionsArr, action)
		actionsList.Refresh()
	},
}

var starterCheck = widget.NewCheck("Starter", func(b bool) {})
var repeatAmountSlider = widget.NewSlider(0, 50)
var repeaterSettingsForm = widget.Form{
	Items: []*widget.FormItem{
		{Text: "Amount", Widget: repeatAmountSlider},
		{Text: "Starter", Widget: starterCheck},
		//{Text: "Hold Keys Down", Widget: holdKeysCheckGroup},
	},
	OnSubmit: func() {
		action := actions.Repeater{
			Amount:  int(repeatAmountSlider.Value),
			Starter: starterCheck.Checked,
			//KeysHeldDown: holdKeysCheckGroup.Selected,
		}
		actionsArr = append(actionsArr, action)
		actionsList.Refresh()
	},
}
var actionsTypeList = []string{
	"🖱️ Mouse Move",
	"🖱️ Click",
	"🔍 Search",
	"📝 OCR",
	"🔁 Repeater",
}
var actionSelector = widget.NewSelect(actionsTypeList, func(s string) {
	mouseMoveSettingsForm.Hide()
	clickSettingsForm.Hide()
	//searchSettingsForm.Hide()
	//ocrSettingsForm.Hide()
	repeaterSettingsForm.Hide()
	switch s {
	case actionsTypeList[0]:
		mouseMoveSettingsForm.Show()
	case actionsTypeList[1]:
		clickSettingsForm.Show()
	//case actionsTypeList[2]:
	//searchSettingsForm.Show()
	//case actionsTypeList[3]:
	//ocrSettingsForm.Show()
	case actionsTypeList[4]:
		repeaterSettingsForm.Show()
	}
})
