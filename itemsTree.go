package main

import (
	"Dark-And-Darker/internal"
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	"log"
	"strconv"
)

func (u *ui) createItemsCheckTree() *widget.Tree {
	log.Println("Creating Items Check Tree")
	var (
		icons       = *internal.BytesToFyneIcons()
		itemsStrMap = internal.Items.GetItemsMapAsStringsMap()
		categories  = make([]string, 0, len(itemsStrMap))
	)

	for category := range itemsStrMap {
		categories = append(categories, category)
	}

	setAllItemsInCategory := func(category string, b bool) bool {
		flip := true
		if b {
			for _, item := range itemsStrMap[category] {
				if imageSearchTargets[item] == false {
					flip = false
				}
				imageSearchTargets[item] = true
			}
			log.Printf("Selected category: %v", category)
			return flip
		}
		for _, item := range itemsStrMap[category] {
			imageSearchTargets[item] = false
		}
		log.Printf("Unselected category: %v", category)
		return false
	}

	tree := widget.NewTree(
		func(id widget.TreeNodeID) []widget.TreeNodeID {
			if id == "" {
				return categories
			}
			if is, exists := itemsStrMap[id]; exists {
				return is
			}
			return nil
		},
		func(id widget.TreeNodeID) bool {
			return id == "" || itemsStrMap[id] != nil
		},
		func(b bool) fyne.CanvasObject {
			if b {
				return container.NewGridWithRows(1,
					widget.NewLabel(""),
					widget.NewCheck("placeholder", func(b bool) {}),
					layout.NewSpacer(),

				)
			} else {
				return container.NewGridWrap(fyne.NewSquareSize(40),
					widget.NewIcon(theme.BrokenImageIcon()),
					widget.NewCheck("placeholder", func(b bool) {}),
					layout.NewSpacer(),
				)
			}
		},
		func(id widget.TreeNodeID, b bool, o fyne.CanvasObject) {
			c := o.(*fyne.Container)
			wc := c.Objects[1].(*widget.Check)

			if b {
				wc.OnChanged = func(b bool) {
					setAllItemsInCategory(id, b)
				}
				var counter int
				for _, item := range itemsStrMap[id] {
					if imageSearchTargets[item] {
						counter++
					}
				}
				wc.SetText(id + ": " + strconv.Itoa(counter) + " / " + strconv.Itoa(len(itemsStrMap[id])))
				return
			}
			wi := c.Objects[0].(*widget.Icon)
			wc.OnChanged = func(b bool) {
				if b {
					imageSearchTargets[id] = true
				} else {
					imageSearchTargets[id] = false
				}
				return
			}
			wc.SetText(id)
			wc.SetChecked(imageSearchTargets[id])

			path := id + ".png"
			if icons[path] == nil {
				wi.SetResource(theme.BrokenImageIcon())
				return
			}
			wi.SetResource(icons[path])
		},
	)
	tree.HideSeparators = true
	tree.OnSelected = func(id widget.TreeNodeID) {
		tree.Unselect(id)

		if tree.IsBranch(id) {
			if ok := setAllItemsInCategory(id, true); ok {
				setAllItemsInCategory(id, false)
			}
		} else {
			if imageSearchTargets[id] {
				imageSearchTargets[id] = false
			} else {
				imageSearchTargets[id] = true
			}
		}
		tree.Refresh()
	}
	return tree
}
