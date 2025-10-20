package binders

import (
	"Squire/ui"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

func SetEditorTabs() {
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
