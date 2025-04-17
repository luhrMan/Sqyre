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

func createItemsStringGrid() *widget.Accordion {
	log.Println("Creating Items Check Tree")
	var (
		icons       = *assets.BytesToFyneIcons()
		itemsStrMap = assets.Items.GetItemsMapAsStringsMap()
		ac          = widget.NewAccordion()
	)

	getCategoryCount := func(c string) int {
		var counter int
		for _, i := range itemsStrMap[c] {
			if slices.Contains(imageSearchTargets, i) {
				counter++
			}
		}
		return counter
	}

	for category := range itemsStrMap {
		ait := category + ": " + strconv.Itoa(getCategoryCount(category)) + " / " + strconv.Itoa(len(itemsStrMap[category]))
		ai := widget.NewAccordionItem(ait, nil)
		gw := widget.NewGridWrap(
			func() int { return len(itemsStrMap[category]) },
			func() fyne.CanvasObject {
				rect := canvas.NewRectangle(color.RGBA{})
				rect.SetMinSize(fyne.NewSquareSize(45))
				rect.CornerRadius = 5

				icon := canvas.NewImageFromResource(theme.BrokenImageIcon())
				icon.SetMinSize(fyne.NewSquareSize(40))
				icon.FillMode = canvas.ImageFillOriginal

				stack :=
					container.NewStack(
						rect,
						widget.NewLabel(""),
						container.NewPadded(
							icon,
						),
					)
				return stack
			},
			func(gwii widget.GridWrapItemID, o fyne.CanvasObject) {
				stack := o.(*fyne.Container)
				rect := stack.Objects[0].(*canvas.Rectangle)
				label := stack.Objects[1].(*widget.Label)
				icon := stack.Objects[2].(*fyne.Container).Objects[0].(*canvas.Image)
				item := itemsStrMap[category][gwii]

				if slices.Contains(imageSearchTargets, item) {
					rect.FillColor = color.RGBA{R: 0, G: 128, B: 0, A: 128}
				} else {
					rect.FillColor = color.RGBA{}
				}

				label.Hidden = true
				label.SetText(item)

				path := item + ".png"
				if icons[path] != nil {
					icon.Resource = icons[path]
				}
				ai.Title = category + ": " + strconv.Itoa(getCategoryCount(category)) + " / " + strconv.Itoa(len(itemsStrMap[category]))

				o.Refresh()
			},
		)
		gw.OnSelected = func(uid widget.GridWrapItemID) {
			defer gw.UnselectAll()
			defer boundImageSearchTargets.Reload()
			defer gw.RefreshItem(uid)
			// if gw.IsBranch(uid) {
			// 	flip := true
			// 	//if one item is missing from category, add the rest
			// 	for _, c := range itemsStrMap[uid] {
			// 		if !slices.Contains(imageSearchTargets, c) {
			// 			imageSearchTargets = append(imageSearchTargets, c)
			// 			flip = false
			// 		}
			// 	}Col
			// 	if !flip {
			// 		return
			// 	}
			// 	//if one item is missing from category, don't flip
			// 	for _, item := range itemsStrMap[uid] {
			// 		if !slices.Contains(imageSearchTargets, item) {
			// 			return
			// 		}
			// 	}
			// 	//for all items in category, delete them
			// 	for _, item := range itemsStrMap[uid] {
			// 		i := slices.Index(imageSearchTargets, item)
			// 		if i != -1 {
			// 			imageSearchTargets = slices.Delete(imageSearchTargets, i, i+1)
			// 		}
			// 	}
			// 	return
			// }
			item := itemsStrMap[category][uid]
			if !slices.Contains(imageSearchTargets, item) {
				imageSearchTargets = append(imageSearchTargets, item)
			} else {
				i := slices.Index(imageSearchTargets, item)
				if i != -1 {
					imageSearchTargets = slices.Delete(imageSearchTargets, i, i+1)
				}
			}
			ai.Title = category + ": " + strconv.Itoa(getCategoryCount(category)) + " / " + strconv.Itoa(len(itemsStrMap[category]))

			ac.Refresh()
		}

		r := canvas.NewRectangle(color.RGBA{R: 255, G: 255, B: 255, A: 25})
		//I could probably do some column math here to determine the amount of rows and multiply by that, but this works for now
		r.SetMinSize(fyne.NewSize(150, float32(len(itemsStrMap[category])*8)))
		r.CornerRadius = 5
		s := container.NewStack(r, gw)
		ai.Detail = s
		ac.Append(ai) //widget.NewAccordionItem(category, s))
	}

	ac.MultiOpen = true
	// at.imageSearch.targetsAccordionGrids = ac
	return ac
}
