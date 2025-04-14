package ui

import (
	"Squire/internal/assets"
	"log"
	"strconv"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

var boolMap = map[string]bool{}
var bind binding.ExternalBoolTree

func (at *actionTabs) DcreateItemsCheckTree() *widget.Tree {
	log.Println("Creating Items Check Tree")
	var (
		treeStructure = assets.Items.GetItemsMapAsStringsMap()
		icons         = *assets.BytesToFyneIcons()
	)
	bind = binding.BindBoolTree(&treeStructure, &boolMap)
	// bmBind := binding.BindBoolTree(&map[string][]string{"": {}}, &boolMap)

	for c, category := range treeStructure { //get all items from map and add to boolmap
		for _, i := range category {
			boolMap[i] = false
		}
		boolMap[c] = false
	}
	bind.Reload()
	// bind.AddListener(binding.NewDataListener(func() {
	// 	targets, _ := at.imageSearch.boundImageSearchTargets.Get()
	// 	log.Println("before change", targets)

	// 	// targets := []string{}
	// 	for s, b := range boolMap {
	// 		if b {
	// 			if !slices.Contains(targets, s) {
	// 				at.imageSearch.boundImageSearchTargets.Append(s)
	// 			}
	// 		} else {
	// 			at.imageSearch.boundImageSearchTargets.Remove(s)
	// 		}
	// 	}
	// 	targets, _ = at.imageSearch.boundImageSearchTargets.Get()

	// 	log.Println("after change", targets)

	// // }))
	// bind.AddListener(binding.NewDataListener(func() {
	// 	targets, _ := at.imageSearch.boundImageSearchTargets.Get()
	// 	_, v, _ := bind.Get()
	// 	// targets := []string{}
	// 	for s, b := range v {
	// 		if b {
	// 			if !slices.Contains(targets, s) {
	// 				at.imageSearch.boundImageSearchTargets.Append(s)
	// 			}
	// 		} else {
	// 			at.imageSearch.boundImageSearchTargets.Remove(s)
	// 		}
	// 	}
	// }))
	setCategory := func(uid string, b bool) {
		// defer bind.Reload()
		categories, bm, _ := bind.Get()
		if b {
			for _, item := range categories[uid] {
				// bind.SetValue(item, true)
				boolMap[item] = true
			}
			return
		}
		for _, item := range categories[uid] {
			if !bm[item] {
				return
			}
		}
		for _, item := range categories[uid] {
			// bind.SetValue(item, false)
			boolMap[item] = false
		}
	}

	twd := widget.NewTreeWithData(
		bind,
		func(b bool) fyne.CanvasObject {
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
		},
		func(di binding.DataItem, b bool, co fyne.CanvasObject) {},
	)
	twd.UpdateNode = func(uid widget.TreeNodeID, branch bool, co fyne.CanvasObject) {
		c := co.(*fyne.Container)
		wc := c.Objects[1].(*widget.Check)
		di, _ := bind.GetItem(uid)

		wc.Bind(di.(binding.Bool))
		di.AddListener(binding.NewDataListener(func() {
			_, v, _ := bind.Get()
			targets := []string{}
			for s, b := range v {
				if b {
					targets = append(targets, s)
				}
			}

			at.imageSearch.boundImageSearchTargets.Set(targets)

			// for s, b := range v {
			// 	if b {
			// 		if !slices.Contains(targets, s) {
			// 			at.imageSearch.boundImageSearchTargets.Append(s)
			// 		}
			// 	} else {
			// 		at.imageSearch.boundImageSearchTargets.Remove(s)
			// 	}
			// }
		}))
		defer bind.Reload()
		if branch {
			wc.OnChanged = func(b bool) {
				defer twd.Refresh()
				setCategory(uid, b)
			}
			var counter int
			for _, item := range treeStructure[uid] {
				if boolMap[item] {
					counter++
				}
			}
			wc.SetText(uid + ": " + strconv.Itoa(counter) + " / " + strconv.Itoa(len(treeStructure[uid])))
			if counter == len(treeStructure[uid]) {
				// bind.SetValue(uid, true)
				boolMap[uid] = true

			} else {
				boolMap[uid] = false

				// bind.SetValue(uid, false)
			}
			return
		} else {
			wc.Disable()
			wc.SetText(uid)
			wi := c.Objects[0].(*widget.Icon)
			path := uid + ".png"
			if icons[path] == nil {
				wi.SetResource(theme.BrokenImageIcon())
				return
			}
			wi.SetResource(icons[path])

			// wc.OnChanged = func(b bool) {
			// 	if b {
			// 		// bind.SetValue(uid, true)
			// 		wc.SetChecked(true)

			// 	} else {
			// 		// bind.SetValue(uid, false)
			// 		wc.SetChecked(false)

			// 	}
			// 	twd.Refresh()
			// }
		}

	}
	twd.OnSelected = func(uid widget.TreeNodeID) {
		defer twd.Refresh()
		defer twd.Unselect(uid)

		if twd.IsBranch(uid) {
			ok, _ := bind.GetValue(uid)
			setCategory(uid, !ok)
			return
		}

		if ok, _ := bind.GetValue(uid); ok {
			// bind.SetValue(uid, false)
			boolMap[uid] = false
			// bmBind.Reload()

		} else {
			// bind.SetValue(uid, true)
			boolMap[uid] = true

		}
	}
	twd.HideSeparators = true

	// bind.AddListener(binding.NewDataListener(func() {
	// 	targets, _ := at.imageSearch.boundImageSearchTargets.Get()
	// 	_, v, _ := bind.Get()
	// 	// targets := []string{}
	// 	for s, b := range v {
	// 		if b {
	// 			if !slices.Contains(targets, s) {
	// 				at.imageSearch.boundImageSearchTargets.Append(s)
	// 			}
	// 		} else {
	// 			at.imageSearch.boundImageSearchTargets.Remove(s)
	// 		}
	// 	}
	// }))

	return twd
}

// categories, bm, _ := bind.Get()
// if b {
// 	for _, item := range categories[uid] {
// 		bind.SetValue(item, true)
// 	}
// 	return
// }
// for _, item := range categories[uid] {
// 	if !bm[item] {
// 		return
// 	}
// }
// for _, item := range categories[uid] {
// 	bind.SetValue(item, false)
// }

// categories, bm, _ := bind.Get()
// if ok, _ := bind.GetValue(uid); !ok {
// 	for _, item := range categories[uid] {
// 		bind.SetValue(item, true)
// 	}
// 	return
// }
// for _, item := range categories[uid] {
// 	if !bm[item] {
// 		return
// 	}
// }
// for _, item := range categories[uid] {
// 	bind.SetValue(item, false)
// }
// return
