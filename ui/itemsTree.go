package ui

import (
	"Squire/internal/assets"
	"log"
	"slices"
	"strconv"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

func (u *Ui) createItemsCheckTree() *widget.Tree {
	log.Println("Creating Items Check Tree")
	var (
		icons       = *assets.BytesToFyneIcons()
		itemsStrMap = assets.Items.GetItemsMapAsStringsMap()
		categories  = make([]string, 0, len(itemsStrMap))
		tree        = &widget.Tree{}
	)

	for category := range itemsStrMap {
		categories = append(categories, category)
	}
	log.Println("items map", itemsStrMap)

	updateLists := func(item string, b bool) {
		log.Println("updating image search targets...")
		log.Println("Before update:", imageSearchTargets)

		t, err := u.st.boundImageSearchTargets.Get()
		if err != nil {
			log.Println(err)
			return
		}
		itemsBoolList[item] = b
		if b {
			if !slices.Contains(t, item) {
				u.st.boundImageSearchTargets.Append(item)
			}
		} else {
			u.st.boundImageSearchTargets.Remove(item)
		}
		log.Println("After update:", imageSearchTargets)
	}

	setAllItemsInCategory := func(category string, b bool) bool {
		flip := true
		defer tree.Refresh()
		if b {
			for _, item := range itemsStrMap[category] {
				if !itemsBoolList[item] {
					flip = false
				}
				updateLists(item, true)
			}
			log.Printf("Selected category: %v", category)
			return flip
		}
		for _, item := range itemsStrMap[category] {
			updateLists(item, false)
		}
		log.Printf("Unselected category: %v", category)
		return false
	}

	tree.ChildUIDs = func(id widget.TreeNodeID) []widget.TreeNodeID {
		if id == "" {
			return categories
		}
		if is, exists := itemsStrMap[id]; exists {
			return is
		}
		return nil
	}
	tree.IsBranch = func(id widget.TreeNodeID) bool {
		return id == "" || itemsStrMap[id] != nil
	}
	tree.CreateNode = func(b bool) fyne.CanvasObject {
		if b {
			return container.NewHBox(
				widget.NewLabel(""),
				widget.NewCheck("placeholder", func(b bool) {}),
			)
		} else {
			return container.NewGridWrap(fyne.NewSquareSize(40),
				widget.NewIcon(theme.BrokenImageIcon()),
				widget.NewCheck("placeholder", func(b bool) {}),
				layout.NewSpacer(),
			)
		}
	}
	tree.UpdateNode = func(id widget.TreeNodeID, b bool, o fyne.CanvasObject) {
		c := o.(*fyne.Container)
		wc := c.Objects[1].(*widget.Check)

		if b {
			wc.OnChanged = func(b bool) {
				setAllItemsInCategory(id, b)
			}
			var counter int
			for _, item := range itemsStrMap[id] {
				if itemsBoolList[item] {
					counter++
				}
			}
			wc.SetText(id + ": " + strconv.Itoa(counter) + " / " + strconv.Itoa(len(itemsStrMap[id])))
			if counter == len(itemsStrMap[id]) {
				wc.Checked = true
				wc.Refresh()
			} else {
				wc.Checked = false
				wc.Refresh()
			}
			return
		}
		wi := c.Objects[0].(*widget.Icon)
		wc.OnChanged = func(b bool) {
			if b {
				updateLists(id, true)
			} else {
				updateLists(id, false)
			}
		}
		wc.SetText(id)
		wc.SetChecked(itemsBoolList[id])

		path := id + ".png"
		if icons[path] == nil {
			wi.SetResource(theme.BrokenImageIcon())
			return
		}
		wi.SetResource(icons[path])
	}

	tree.HideSeparators = true
	tree.OnSelected = func(id widget.TreeNodeID) {
		tree.Unselect(id)

		if tree.IsBranch(id) {
			if ok := setAllItemsInCategory(id, true); ok {
				setAllItemsInCategory(id, false)
			}
		} else {
			if itemsBoolList[id] {
				updateLists(id, false)
			} else {
				updateLists(id, true)
			}
		}
		tree.Refresh()
	}
	return tree
}
