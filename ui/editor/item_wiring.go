package editor

import (
	"strconv"

	"Sqyre/internal/models"
	"Sqyre/internal/services"
	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2/widget"
)

func setItemsWidgets(i models.Item) {
	it := shell().EditorTabs.ItemsTab.Widgets

	it["Name"].(*widget.Entry).SetText(i.Name)
	it["Cols"].(*widget.Entry).SetText(strconv.Itoa(i.GridSize[0]))
	it["Rows"].(*widget.Entry).SetText(strconv.Itoa(i.GridSize[1]))
	it["StackMax"].(*widget.Entry).SetText(strconv.Itoa(i.StackMax))

	updateMaskDisplay(i.Mask)

	// Update tags display
	updateTagsDisplay(&i)

	// Update IconVariantEditor with selected item
	if editor, ok := it["iconVariantEditor"].(*custom_widgets.IconVariantEditor); ok {
		programName := shell().EditorTabs.ItemsTab.ProgramSelector.Selected
		iconService := services.IconVariantServiceInstance()
		baseName := iconService.GetBaseItemName(i.Name)

		// Set variant change callback - only refresh when variants actually change
		editor.SetOnVariantChange(func() {
			// Only refresh the specific program's accordion item, not all items
			RefreshProgramAccordionItem(programName)
		})

		// Update both program and item at once to avoid double refresh
		editor.SetProgramAndItem(programName, baseName)
	}
	shell().RefreshEditorActionBar()
}
