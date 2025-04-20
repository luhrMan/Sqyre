package ui

import (
	"Squire/internal/programs/items"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/widget"
)

func addItemWindow() {
	w := fyne.CurrentApp().NewWindow("Add Item")
	i := &items.Item{}
	bi := binding.BindStruct(i)
	n, _ := bi.GetItem("Name")
	gs, _ := bi.GetItem("GridSize")
	// sm, _ := bi.GetItem("StackMax")
	m, _ := bi.GetItem("Merchant")
	form := widget.NewForm(
		widget.NewFormItem("Name:", widget.NewEntryWithData(n.(binding.String))),
		widget.NewFormItem("Max Stacksize:", widget.NewEntryWithData(binding.IntToString(gs.(binding.Int)))),
		widget.NewFormItem("Grid Size:", container.NewHBox(widget.NewLabel("Width: "), widget.NewEntry(), widget.NewLabel("Height: "), widget.NewEntry())),
		widget.NewFormItem("Merchant:", widget.NewEntryWithData(m.(binding.String))),
	)
	nv, _ := bi.GetValue("Name")

	form.OnSubmit = func() {
		GetUi().p.Items[nv.(string)] = *i
	}
	w.SetContent(form)
	w.Show()
}

func addPointWindow() {

}

func addSearchArewWindow() {

}
