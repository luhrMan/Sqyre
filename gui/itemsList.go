package gui

import (
	"Dark-And-Darker/structs"
	"log"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

func createItemsCheckBoxes() *widget.Accordion {
	accordionItems := widget.NewAccordion()
	for category, items := range *structs.GetItemsMap() {
		box := container.NewVBox()
		scroll := container.NewVScroll(box)
		for _, item := range items {
			checkBoxWithIcon := container.NewHBox()
			func(itemName string) {
				checkBox := widget.NewCheck(itemName, func(checked bool) {})
				checkBox.OnChanged = func(checked bool) {
					if checked {
						log.Println(itemName)
						selectedItemsMap[itemName] = true // Add selected item to the map
					} else {
						delete(selectedItemsMap, itemName) // Remove unselected item from the map
					}
					log.Println(selectedItemsMap)
				}
				resource, err := fyne.LoadResourceFromPath("./images/icons/" + itemName + ".png")
				if err != nil {
					log.Println(err)
					checkBoxWithIcon.Add(widget.NewIcon(theme.BrokenImageIcon()))
				} else {
					icon := widget.NewIcon(resource)
					checkBoxWithIcon.Add(icon)
				}
				checkBoxWithIcon.Add(checkBox)
				box.Add(checkBoxWithIcon)
			}(item.Name)
		}
		accordionItems.Append(widget.NewAccordionItem(category, scroll))
	}
	accordionItems.MultiOpen = true
	return accordionItems
}
