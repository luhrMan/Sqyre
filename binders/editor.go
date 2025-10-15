package binders

import (
	"Squire/ui"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

func SetEditorTabs() {
	SetAccordionPointsLists(
		ui.GetUi().EditorUi.EditorTabs.
			PointsTab.Content.(*container.Split).Leading.(*fyne.Container).Objects[0].(*widget.Accordion),
	)

}
