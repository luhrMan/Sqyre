package editor

import (
	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2/widget"
)

// refreshProgramListUI updates the Programs tab list without touching entity accordions.
func refreshProgramListUI() {
	et := shell().EditorTabs
	if programList, ok := et.ProgramsTab.Widgets["list"].(*widget.List); ok {
		setProgramList(programList)
		custom_widgets.RefreshListPreservingScroll(programList)
	}
}

// resyncEntityAccordions refreshes Points/Search Areas/Masks/AutoPic accordions using
// incremental sync (reuses per-program list widgets).
func resyncEntityAccordions() {
	et := shell().EditorTabs
	if acc, ok := et.PointsTab.Widgets["Accordion"].(*custom_widgets.AccordionWithHeaderWidgets); ok {
		syncProgramEntityAccordion(acc, pointsAccordionConfig())
	}
	if acc, ok := et.SearchAreasTab.Widgets["Accordion"].(*custom_widgets.AccordionWithHeaderWidgets); ok {
		syncProgramEntityAccordion(acc, searchAreasAccordionConfig())
	}
	if acc, ok := et.MasksTab.Widgets["Accordion"].(*custom_widgets.AccordionWithHeaderWidgets); ok {
		syncProgramEntityAccordion(acc, masksAccordionConfig())
	}
	if acc, ok := et.AutoPicTab.Widgets["Accordion"].(*custom_widgets.AccordionWithHeaderWidgets); ok {
		setAccordionAutoPicSearchAreasLists(acc)
	}
}

// refreshAllProgramRelatedUI refreshes program list and resyncs editor accordions without
// recreating entity list widgets. Items accordion uses per-program row sync.
func refreshAllProgramRelatedUI() {
	refreshProgramListUI()
	resyncEntityAccordions()
	resyncItemsAccordion()
	InvalidateProgramTagsCache("")
}
