package archive

import (
	"Squire/internal/assets"
	"image/color"
	"log"
	"slices"
	"strconv"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

var imageSearchTargets = []string{}

func createItemsStringTree() *widget.Tree {
	log.Println("Creating Items Check Tree")
	var (
		icons       = *assets.BytesToFyneIcons()
		itemsStrMap = assets.Items.GetItemsMapAsStringsMap()
		categories  = make([]string, 0, len(itemsStrMap))
	)

	for category := range itemsStrMap {
		categories = append(categories, category)
	}
	twd := widget.NewTree(
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
			return container.NewStack(
				canvas.NewRectangle(color.RGBA{}),
				container.NewGridWithColumns(
					2,
					widget.NewLabel(""),
					widget.NewIcon(nil),
				),
			)
		},
		func(id widget.TreeNodeID, b bool, o fyne.CanvasObject) {
			layout := o.(*fyne.Container)
			grid := layout.Objects[1].(*fyne.Container)
			label := grid.Objects[0].(*widget.Label)
			layout.Objects[0].(*canvas.Rectangle).FillColor = color.RGBA{}

			if b {
				var counter int
				for _, item := range itemsStrMap[id] {
					if slices.Contains(imageSearchTargets, item) {
						counter++
					}
				}
				label.SetText(id + ": " + strconv.Itoa(counter) + " / " + strconv.Itoa(len(itemsStrMap[id])))
				if counter == len(itemsStrMap[id]) {
					layout.Objects[0].(*canvas.Rectangle).FillColor =
						color.RGBA{R: 0, G: 128, B: 0, A: 128}
				}
				return
			}

			label.SetText(id)
			if slices.Contains(imageSearchTargets, id) {
				layout.Objects[0].(*canvas.Rectangle).FillColor =
					color.RGBA{R: 0, G: 128, B: 0, A: 128}
			}

			layout.Objects[0].(*canvas.Rectangle).Refresh()

			icon := grid.Objects[1].(*widget.Icon)
			path := id + ".png"
			if icons[path] == nil {
				icon.SetResource(theme.BrokenImageIcon())
				return
			}
			icon.SetResource(icons[path])
		},
	)

	twd.OnSelected = func(uid widget.TreeNodeID) {
		log.Println("selected:", uid)
		defer twd.UnselectAll()
		// defer at.imageSearch.boundImageSearchTargets.Reload()
		defer twd.RefreshItem(uid)
		if twd.IsBranch(uid) {
			flip := true
			//if one item is missing from category, add the rest
			for _, c := range itemsStrMap[uid] {
				if !slices.Contains(imageSearchTargets, c) {
					imageSearchTargets = append(imageSearchTargets, c)
					flip = false
				}
			}
			if !flip {
				return
			}
			//if one item is missing from category, don't flip
			for _, item := range itemsStrMap[uid] {
				if !slices.Contains(imageSearchTargets, item) {
					return
				}
			}
			//for all items in category, delete them
			for _, item := range itemsStrMap[uid] {
				i := slices.Index(imageSearchTargets, item)
				if i != -1 {
					imageSearchTargets = slices.Delete(imageSearchTargets, i, i+1)
				}
			}
			return
		}
		if !slices.Contains(imageSearchTargets, uid) {
			imageSearchTargets = append(imageSearchTargets, uid)
		} else {
			i := slices.Index(imageSearchTargets, uid)
			if i != -1 {
				imageSearchTargets = slices.Delete(imageSearchTargets, i, i+1)
			}
		}
	}
	twd.HideSeparators = true
	// at.imageSearch.targetsTree = twd
	return twd
}
