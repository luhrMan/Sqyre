package gui

import (
	"Dark-And-Darker/structs"
	"Dark-And-Darker/utils"
	"log"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

func createItemsCheckBoxes() *widget.Accordion {
	var (
		accordionItems = widget.NewAccordion()
	)
	accordionItems.MultiOpen = true
	for category, items := range *structs.GetItemsMap() {
		var (
			box              = container.NewVBox()
			scroll           = container.NewVScroll(box)
			categoryCheckbox = widget.NewCheck("select all", func(checked bool) {
				switch checked {
				case true:
					for _, item := range items {
						selectedItemsMap[item.Name] = true
					}
				case false:
					for _, item := range items {
						delete(selectedItemsMap, item.Name)
					}
				}
				log.Println(selectedItemsMap)
			})
		)
		accordionItems.Append(widget.NewAccordionItem(category, scroll))
		box.Add(categoryCheckbox)
		for _, item := range items {
			var (
				itemName                = item.Name
				HBoxWithCheckBoxAndIcon = container.NewHBox()
				itemCheckBox            = widget.NewCheck(itemName, func(checked bool) {
					switch checked {
					case true:
						selectedItemsMap[itemName] = true // Add selected item to the map
					case false:
						delete(selectedItemsMap, itemName) // Remove unselected item from the map
					}
					log.Println(selectedItemsMap)
				})
				resource, imageLoadErr = fyne.LoadResourceFromPath("./images/icons/" + itemName + ".png")
			)
			utils.HandleError(
				imageLoadErr,
				func() {
					HBoxWithCheckBoxAndIcon.Add(widget.NewIcon(theme.BrokenImageIcon()))
				},
				func() {
					icon := widget.NewIcon(resource)
					HBoxWithCheckBoxAndIcon.Add(icon)
				})
			HBoxWithCheckBoxAndIcon.Add(itemCheckBox)
			box.Add(HBoxWithCheckBoxAndIcon)
		}
	}
	return accordionItems
}

func selectedItems() []string {
	var items []string
	for s := range selectedItemsMap {
		items = append(items, s)
	}
	return items
}
