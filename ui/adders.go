package ui

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

func addItemWindow() {
	w := fyne.CurrentApp().NewWindow("Add Item")
	w.SetContent(
		container.NewGridWithRows(
			4,
			container.NewVBox(
				widget.NewLabel("Name: "), widget.NewEntry(),
			),
			container.NewVBox(
				widget.NewLabel("Max Stacksize: "), widget.NewEntry(),
			),
			container.NewVBox(
				widget.NewLabel("Grid Size: "), widget.NewLabel("Width: "), widget.NewEntry(), widget.NewLabel("Height: "), widget.NewEntry(),
			),
			container.NewVBox(
				widget.NewLabel("Merchant"), widget.NewEntry(),
			),
		),
	)
	w.Show()
}

func addPointWindow() {

}

func addSearchArewWindow() {

}
