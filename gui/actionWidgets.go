package gui

import (
	"Dark-And-Darker/custom_widgets"
	"Dark-And-Darker/structs"
	"Dark-And-Darker/utils"
	"fmt"
	"log"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

var (
	//BASICS
	//wait
	time            float64
	boundTime       = binding.BindFloat(&time)
	boundTimeSlider = widget.NewSliderWithData(0.0, 250.0, boundTime)
	boundTimeLabel  = widget.NewLabelWithData(binding.FloatToStringWithFormat(boundTime, "%0.0f"))
	//move
	moveX float64
	moveY float64
	// spot             structs.Spot
	// boundSpot        = binding.BindString(&spot.Name)
	boundMoveX = binding.BindFloat(&moveX)
	boundMoveY = binding.BindFloat(&moveY)
	// boundSpotSelect  = widget.NewSelect(*structs.GetSpotMapKeys(*structs.GetSpotMap()), func(s string) { boundSpot.Set(s) })
	boundMoveXSlider = widget.NewSliderWithData(0.0, float64(utils.MonitorWidth), boundMoveX)
	boundMoveYSlider = widget.NewSliderWithData(0.0, float64(utils.MonitorHeight), boundMoveY)
	boundMoveXLabel  = widget.NewLabelWithData(binding.FloatToStringWithFormat(boundMoveX, "%0.0f"))
	boundMoveYLabel  = widget.NewLabelWithData(binding.FloatToStringWithFormat(boundMoveY, "%0.0f"))
	//click
	button            bool
	boundButton       = binding.BindBool(&button)
	boundButtonToggle = custom_widgets.NewToggleWithData(boundButton)
	//key
	key              string
	state            bool
	boundKey         = binding.BindString(&key)
	boundState       = binding.BindBool(&state)
	boundKeySelect   = widget.NewSelect([]string{"ctrl", "alt", "shift"}, func(s string) { boundKey.Set(s) })
	boundStateToggle = custom_widgets.NewToggleWithData(boundState)

	//ADVANCED
	advancedActionName           string
	searchArea                   string
	boundAdvancedActionName      = binding.BindString(&advancedActionName)
	boundSearchArea              = binding.BindString(&searchArea)
	boundAdvancedActionNameEntry = widget.NewEntryWithData(boundAdvancedActionName)
	boundSearchAreaSelect        = widget.NewSelect(*structs.GetSearchBoxMapKeys(*structs.GetSearchBoxMap()), func(s string) { boundSearchArea.Set(s) })

	//loop
	count            float64 = 1
	boundCount               = binding.BindFloat(&count)
	boundCountSlider         = widget.NewSliderWithData(1, 10, boundCount)
	boundCountLabel          = widget.NewLabelWithData(binding.FloatToStringWithFormat(boundCount, "%0.0f"))
	//image search
	targets []string

	// selectedItemsMap       = make(map[string]any)
	boundSelectedItemsMap = binding.BindUntypedMap(&selectedItemsMap)

	boundTargets = binding.BindStringList(&targets)

	//ocr
)

func createUpdateButton() *widget.Button {
	return widget.NewButton("Update", func() {
		node := findNode(root, selectedTreeItem)
		if selectedTreeItem == "" {
			log.Println("No node selected")
			return
		}
		og := node.String()
		switch node := node.(type) {
		case *structs.WaitAction:
			t, _ := boundTime.Get()
			node.Time = int(t)
		case *structs.MouseMoveAction:
			x, _ := boundMoveX.Get()
			y, _ := boundMoveY.Get()
			node.X = int(x)
			node.Y = int(y)
		case *structs.ClickAction:
			b, _ := boundButton.Get()
			if !b {
				node.Button = "left"
			} else {
				node.Button = "right"
			}
		case *structs.KeyAction:
			k, _ := boundKey.Get()
			s, _ := boundState.Get()
			node.Key = k
			if !s {
				node.State = "down"
			} else {
				node.State = "up"
			}
		case *structs.LoopAction:
			n, _ := boundAdvancedActionName.Get()
			c, _ := boundCount.Get()
			node.Name = n
			node.Count = int(c)
		case *structs.ImageSearchAction:
			n, _ := boundAdvancedActionName.Get()
			s, _ := boundSearchArea.Get()
			t := boundSelectedItemsMap.Keys()
			node.Name = n
			node.SearchBox = *structs.GetSearchBox(s)
			node.Targets = t
		}

		fmt.Printf("Updated node: %+v from '%v' to '%v' \n", node.GetUID(), og, node)

		tree.Refresh()
	})
}

func createItemsCheckBoxes() *widget.Accordion {
	// var boundTargetsCheck []widget.Check
	var (
		accordionItems = widget.NewAccordion()
	)
	accordionItems.MultiOpen = true
	for category, items := range *structs.GetItemsMap() {
		var (
			box           = container.NewVBox()
			scroll        = container.NewVScroll(box)
			categoryCheck = widget.NewCheck("select all", func(checked bool) {
				switch checked {
				case true:
					for _, item := range items {
						boundSelectedItemsMap.SetValue(item.Name, true)
					}
				case false:
					for _, item := range items {
						boundSelectedItemsMap.Delete(item.Name)
					}
				}
				log.Println(selectedItemsMap)
			})
		)
		accordionItems.Append(widget.NewAccordionItem(category, scroll))
		box.Add(categoryCheck)
		for _, item := range items {
			var (
				itemName                = item.Name
				HBoxWithCheckBoxAndIcon = container.NewHBox()
				itemCheckBox            = widget.NewCheck(itemName, func(checked bool) {
					switch checked {
					case true:
						boundSelectedItemsMap.SetValue(itemName, true)
					case false:
						boundSelectedItemsMap.Delete(itemName)
					}
					log.Println(selectedItemsMap)
				})
				// itemBool                bool
				// boundItemBool           = binding.BindBool(&itemBool)
				// boundItemCheck          = widget.NewCheckWithData(itemName, boundItemBool)
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

	// for category, items := range *structs.GetItemsMap() {
	// 	var (
	// 		box              = container.NewVBox()
	// 		scroll           = container.NewVScroll(box)
	// 		categoryCheckbox = widget.NewCheck("select all", func(checked bool) {
	// 			switch checked {
	// 			case true:
	// 				for _, item := range items {
	// 					selectedItemsMap[item.Name] = true
	// 				}
	// 			case false:
	// 				for _, item := range items {
	// 					delete(selectedItemsMap, item.Name)
	// 				}
	// 			}
	// 			log.Println(selectedItemsMap)
	// 		})
	// 	)
	// 	accordionItems.Append(widget.NewAccordionItem(category, scroll))
	// 	box.Add(categoryCheckbox)
	// 	for _, item := range items {
	// 		var (
	// 			itemName                = item.Name
	// 			HBoxWithCheckBoxAndIcon = container.NewHBox()
	// 			itemCheckBox            = widget.NewCheck(itemName, func(checked bool) {
	// 				switch checked {
	// 				case true:
	// 					selectedItemsMap[itemName] = true // Add selected item to the map
	// 				case false:
	// 					delete(selectedItemsMap, itemName) // Remove unselected item from the map
	// 				}
	// 				log.Println(selectedItemsMap)
	// 			})
	// 			resource, imageLoadErr = fyne.LoadResourceFromPath("./images/icons/" + itemName + ".png")
	// 		)
	// 		utils.HandleError(
	// 			imageLoadErr,
	// 			func() {
	// 				HBoxWithCheckBoxAndIcon.Add(widget.NewIcon(theme.BrokenImageIcon()))
	// 			},
	// 			func() {
	// 				icon := widget.NewIcon(resource)
	// 				HBoxWithCheckBoxAndIcon.Add(icon)
	// 			})
	// 		HBoxWithCheckBoxAndIcon.Add(itemCheckBox)
	// 		box.Add(HBoxWithCheckBoxAndIcon)
	// 	}
	// }
	return accordionItems
}

// func selectedItems() []string {
// 	var items []string
// 	for s := range selectedItemsMap {
// 		items = append(items, s)
// 	}
// 	return items
// }

// ***************************************************************************************Move

// spotSelector := &widget.Select{Options: *structs.GetSpotMapKeys(*structs.GetSpotMap())}
// spotSelector.OnChanged = func(s string) {
// 	//structs.GetSpot(spotSelector.Selected)
// 	log.Println(*structs.GetSpot(s))
// 	mouseMoveXEntry.SetText(strconv.FormatInt(int64(structs.GetSpot(s).X), 10))
// 	mouseMoveYEntry.SetText(strconv.FormatInt(int64(structs.GetSpot(s).Y), 10))
// }
// spotSelector.SetSelected(spotSelector.Options[0])

// ***************************************************************************************Conditional
// func createConditionalSettings() *fyne.Container {
// 	addConditonalActionButton := &widget.Button{
// 		Text: utils.GetEmoji("Conditional") + "Add Conditional",
// 		OnTapped: func() {
// 			selectedNode := findNode(root, selectedTreeItem)
// 			if selectedNode == nil {
// 				selectedNode = root
// 			}
// 			ca := &structs.ConditionalAction{
// 				AdvancedAction: structs.AdvancedAction{
// 					BaseAction: structs.NewBaseAction(),
// 					Name:       advancedActionNameEntry.Text,
// 				},
// 				Condition: func(i interface{}) bool { return true },
// 			}
// 			if s, ok := selectedNode.(structs.AdvancedActionInterface); ok {
// 				s.AddSubAction(ca)
// 			} else {
// 				selectedNode.GetParent().AddSubAction(ca)
// 			}
// 			updateTree(&tree, root)
// 		},
// 		IconPlacement: widget.ButtonIconPlacement(widget.ButtonAlignTrailing),
// 		Icon:          theme.NavigateNextIcon(),
// 		Importance:    widget.HighImportance,
// 	}
// 	return container.NewGridWithColumns(4,
// 		layout.NewSpacer(),
// 		layout.NewSpacer(),
// 		layout.NewSpacer(),
// 		addConditonalActionButton,
// 	)
// }
