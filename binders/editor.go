package binders

import (
	"Squire/internal/models/items"
	"Squire/internal/models/repositories"
	"Squire/ui"
	"log"
	"strconv"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

func SetEditorUi() {
	setEditorTabs()
	setButtons()
	ui.GetUi().EditorUi.ProgramSelector.SetOptions(repositories.ProgramRepo().GetAllAsStringSlice())
}

func setEditorTabs() {
	setAccordionPointsLists(
		ui.GetUi().EditorUi.EditorTabs.
			PointsTab.Content.(*container.Split).Leading.(*fyne.Container).Objects[0].(*widget.Accordion),
	)
	setAccordionSearchAreasLists(
		ui.GetUi().EditorUi.EditorTabs.
			SearchAreasTab.Content.(*container.Split).Leading.(*fyne.Container).Objects[0].(*widget.Accordion),
	)
	setAccordionItemsLists(
		ui.GetUi().EditorUi.EditorTabs.
			ItemsTab.Content.(*container.Split).Leading.(*fyne.Container).Objects[0].(*widget.Accordion),
	)
}

func setButtons() {
	ui.GetUi().EditorUi.AddButton.OnTapped = func() {
		program := ui.GetUi().EditorUi.ProgramSelector.Text

		switch ui.GetUi().EditorUi.EditorTabs.Selected().Text {
		case "Items":
			x, _ := strconv.Atoi(ui.GetUi().EditorTabs.ItemsTab.BindableWidgets["GridSizeX"].(*widget.Entry).Text)
			y, _ := strconv.Atoi(ui.GetUi().EditorTabs.ItemsTab.BindableWidgets["GridSizeY"].(*widget.Entry).Text)
			sm, _ := strconv.Atoi(ui.GetUi().EditorTabs.ItemsTab.BindableWidgets["StackMax"].(*widget.Entry).Text)
			i := &items.Item{
				Name:     ui.GetUi().EditorTabs.ItemsTab.BindableWidgets["Name"].(*widget.Entry).Text,
				GridSize: [2]int{x, y},
				Tags:     []string{},
				StackMax: sm,
				Merchant: "",
			}
			repositories.ProgramRepo().Get(program).AddItem(i)
			log.Println(repositories.ProgramRepo().Get(program).GetItem(i.Name))
			repositories.ProgramRepo().SetAll()
		case "Points":

		case "Search Areas":

		}
	}
	ui.GetUi().EditorUi.RemoveButton.OnTapped = func() {
		switch ui.GetUi().EditorUi.EditorTabs.Selected().Text {
		case "Items":

		case "Points":

		case "Search Areas":

		}
	}
}
