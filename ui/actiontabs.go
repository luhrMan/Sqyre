package ui

import (
	"Squire/internal/assets"
	"Squire/internal/programs/actions"
	"Squire/internal/programs/coordinates"
	"Squire/internal/programs/items"
	"Squire/ui/custom_widgets"
	"image/color"
	"slices"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

const (
	WaitTab = iota
	MoveTab
	ClickTab
	KeyTab
	LoopTab
	ImageSearchTab
	OcrTab
)

type ActionTabs struct {
	*container.AppTabs
	BoundBaseAction     binding.Struct
	BoundAdvancedAction binding.Struct

	BoundWait  binding.Struct
	BoundKey   binding.Struct
	BoundMove  binding.Struct
	BoundClick binding.Struct

	BoundLoop        binding.Struct
	BoundImageSearch binding.Struct
	BoundOcr         binding.Struct

	BoundSearchArea binding.Struct
	BoundPoint      binding.Struct

	BoundTimeSlider *widget.Slider
	BoundTimeEntry  *widget.Entry

	// BoundMoveXSlider *widget.Slider
	// BoundMoveYSlider *widget.Slider
	// BoundMoveXEntry  *widget.Entry
	// BoundMoveYEntry  *widget.Entry
	// boundMovePointSearchBar  *widget.Entry
	// boundMovePointStringList binding.ExternalStringList
	PointsAccordion *widget.Accordion
	// BoundPointList  *widget.List
	// PointTree                *widget.Tree

	BoundButtonToggle *custom_widgets.Toggle

	BoundKeySelect   *widget.Select
	BoundStateToggle *custom_widgets.Toggle

	BoundLoopNameEntry *widget.Entry
	BoundCountSlider   *widget.Slider
	BoundCountLabel    *widget.Label

	BoundTargetsGridSearchBar *widget.Entry
	BoundTargetsGrid          *widget.GridWrap
	BoundImageSearchNameEntry *widget.Entry
	// BoundImageSearchAreaSearchBar        *widget.Entry
	// BoundImageSearchSearchAreaStringList binding.ExternalStringList
	// BoundImageSearchAreaList             *widget.List
	SAAccordion *widget.Accordion

	// boundXSplitSlider          *widget.Slider
	// boundXSplitEntry           *widget.Entry
	// boundOCRTarget     binding.String
	// boundOCRSearchArea binding.String

	boundOCRTargetEntry      *widget.Entry
	boundOCRSearchAreaSelect *widget.Select
}

func newActionTabs() *ActionTabs {
	return &ActionTabs{
		AppTabs: &container.AppTabs{},

		BoundBaseAction:     binding.BindStruct(actions.BaseAction{}),
		BoundAdvancedAction: binding.BindStruct(actions.AdvancedAction{}),

		BoundWait:  binding.BindStruct(actions.NewWait(0)),
		BoundKey:   binding.BindStruct(actions.NewKey("ctrl", "down")),
		BoundMove:  binding.BindStruct(actions.NewMove(coordinates.Point{"blank", 0, 0})),
		BoundClick: binding.BindStruct(actions.NewClick("left")),

		BoundLoop:        binding.BindStruct(actions.NewLoop(1, "blank", []actions.ActionInterface{})),
		BoundImageSearch: binding.BindStruct(actions.NewImageSearch("blank", []actions.ActionInterface{}, []string{}, coordinates.SearchArea{})),
		BoundOcr:         binding.BindStruct(actions.NewOcr("blank", []actions.ActionInterface{}, "blank", coordinates.SearchArea{})),

		BoundSearchArea: binding.BindStruct(coordinates.SearchArea{}),
		BoundPoint:      binding.BindStruct(coordinates.Point{Name: "", X: 0, Y: 0}),

		BoundTimeSlider: widget.NewSliderWithData(0.0, 1000.0, binding.NewFloat()),
		BoundTimeEntry:  &widget.Entry{},
		// BoundMoveXSlider:  widget.NewSliderWithData(-1.0, float64(config.MonitorWidth), binding.NewFloat()),
		// BoundMoveYSlider:  widget.NewSliderWithData(-1.0, float64(config.MonitorHeight), binding.NewFloat()),
		// BoundMoveXEntry:   widget.NewEntryWithData(binding.NewString()),
		// BoundMoveYEntry:   widget.NewEntryWithData(binding.NewString()),
		PointsAccordion:   widget.NewAccordion(),
		BoundButtonToggle: custom_widgets.NewToggleWithData(binding.NewBool()),
		BoundKeySelect:    widget.NewSelectWithData([]string{"ctrl", "alt", "shift"}, binding.NewString()),
		BoundStateToggle:  custom_widgets.NewToggleWithData(binding.NewBool()),

		BoundLoopNameEntry: widget.NewEntryWithData(binding.NewString()),
		BoundCountSlider:   widget.NewSliderWithData(1, 10, binding.IntToFloat(binding.NewInt())),
		BoundCountLabel:    widget.NewLabelWithData(binding.NewString()),

		BoundTargetsGridSearchBar: &widget.Entry{},
		BoundTargetsGrid:          &widget.GridWrap{},
		SAAccordion:               widget.NewAccordion(),

		BoundImageSearchNameEntry: widget.NewEntryWithData(binding.NewString()),
		// BoundImageSearchAreaList:             &widget.List{},
		// BoundImageSearchSearchAreaStringList: binding.BindStringList(&[]string{}),
		boundOCRTargetEntry:      &widget.Entry{},
		boundOCRSearchAreaSelect: &widget.Select{},
	}
}

func (u *Ui) constructActionTabs() {
	u.ActionTabs.BoundAdvancedAction = binding.BindStruct(&actions.AdvancedAction{})
	u.ActionTabs.BoundSearchArea = binding.BindStruct(&coordinates.SearchArea{})

	u.ActionTabs.constructWaitTab()
	u.ActionTabs.constructMoveTab()
	u.ActionTabs.constructClickTab()
	u.ActionTabs.constructKeyTab()
	u.ActionTabs.constructLoopTab()
	u.ActionTabs.constructImageSearchTab()
	u.ActionTabs.constructOcrTab()
}

func (at *ActionTabs) constructWaitTab() {
	gdfi := widget.NewFormItem("delay", GetUi().Mui.MTabs.BoundGlobalDelayEntry)
	gdfi.HintText = "keyboard & mouse global delay (ms)"

	waitSettings :=
		widget.NewForm(
			widget.NewFormItem("ms", container.NewGridWithColumns(2,
				at.BoundTimeEntry, at.BoundTimeSlider,
			)),
			gdfi,
		)
	at.Append(container.NewTabItem("Wait", waitSettings))
}
func (at *ActionTabs) constructMoveTab() {
	//change point to custom if changed from selected option in point list
	// at.boundMoveXEntry.OnChanged = func(s string) {
	// 	i, e := strconv.Atoi(s)
	// 	if e != nil {
	// 		log.Println(e)
	// 		return
	// 	}
	// 	n, _ := at.boundPoint.GetValue("Name")
	// 	p := programs.CurrentProgramAndScreenSizeCoordinates().GetPoint(strings.ToLower(n.(string)))
	// 	if p.X != i {
	// 		at.boundPoint.SetValue("Name", "custom")
	// 		at.boundPointList.UnselectAll()
	// 	}
	// }
	// at.boundMoveYEntry.OnChanged = func(s string) {
	// 	i, e := strconv.Atoi(s)
	// 	if e != nil {
	// 		log.Println(e)
	// 		return
	// 	}
	// 	n, _ := at.boundPoint.GetValue("Name")
	// 	p := programs.CurrentProgramAndScreenSizeCoordinates().GetPoint(strings.ToLower(n.(string)))
	// 	if p.Y != i {
	// 		at.boundPoint.SetValue("Name", "custom")
	// 		at.boundPointList.UnselectAll()
	// 	}
	// }
	moveSettings :=
		container.NewBorder(
			// widget.NewForm(
			// 	widget.NewFormItem("X:", container.NewGridWithColumns(2,
			// 		at.BoundMoveXEntry, at.BoundMoveXSlider,
			// 	)),
			// 	widget.NewFormItem("Y:", container.NewGridWithColumns(2,
			// 		at.BoundMoveYEntry, at.BoundMoveYSlider,
			// 	)),
			// ),
			nil, nil, nil, nil,
			at.PointsAccordion,
			// mouseMoveDisplayContainer
		)
	at.Append(container.NewTabItem("Move", moveSettings))
}

func (at *ActionTabs) constructClickTab() {
	clickSettings :=
		container.NewVBox(
			container.NewHBox(
				layout.NewSpacer(),
				widget.NewLabel("left"),
				at.BoundButtonToggle,
				widget.NewLabel("right"),
				layout.NewSpacer(),
			),
		)
	at.Append(container.NewTabItem("Click", clickSettings))
}

func (at *ActionTabs) constructKeyTab() {
	keySettings :=
		container.NewVBox(
			container.NewHBox(
				layout.NewSpacer(),
				at.BoundKeySelect,
				widget.NewLabel("up"),
				at.BoundStateToggle,
				widget.NewLabel("down"),
				layout.NewSpacer(),
			),
		)
	at.Append(container.NewTabItem("Key", keySettings))
}

func (at *ActionTabs) constructLoopTab() {
	loopSettings :=
		widget.NewForm(
			widget.NewFormItem("Name:", at.BoundLoopNameEntry),
			widget.NewFormItem("Loops:", container.NewBorder(
				nil, nil, at.BoundCountLabel, nil, at.BoundCountSlider,
			)),
		)
	at.Append(container.NewTabItem("Loop", loopSettings))
}

func (at *ActionTabs) constructImageSearchTab() {
	// at.BoundImageSearchNameEntry.OnChanged = func(s string) { at.BoundAdvancedAction.SetValue("Name", s) }

	// var saSearchList = slices.Clone(programs.CurrentProgramAndScreenSizeCoordinates().GetSearchAreasAsStringSlice())
	// at.boundImageSearchSearchAreaStringList = binding.BindStringList(&saSearchList)
	// at.boundImageSearchAreaSearchBar = &widget.Entry{
	// 	PlaceHolder: "Search here",
	// 	OnChanged: func(s string) {
	// 		defaultList := programs.CurrentProgramAndScreenSizeCoordinates().GetSearchAreasAsStringSlice()
	// 		defer at.boundImageSearchSearchAreaStringList.Reload()
	// 		defer at.boundImageSearchAreaList.ScrollToTop()
	// 		defer at.boundImageSearchAreaList.Refresh()

	// 		if s == "" {
	// 			saSearchList = defaultList
	// 			return
	// 		}
	// 		saSearchList = []string{}
	// 		for _, i := range defaultList {
	// 			if fuzzy.MatchFold(s, i) {
	// 				saSearchList = append(saSearchList, i)
	// 			}
	// 		}
	// 	},
	// }

	// at.BoundImageSearchAreaList = widget.NewListWithData(
	// 	at.BoundImageSearchSearchAreaStringList,
	// 	func() fyne.CanvasObject { return widget.NewLabel("template") },
	// 	func(di binding.DataItem, co fyne.CanvasObject) {
	// 		bsa := di.(binding.String)
	// 		label := co.(*widget.Label)
	// 		v, _ := bsa.Get()
	// 		sa := programs.CurrentProgramAndScreenSizeCoordinates().GetSearchArea(v)
	// 		label.SetText(fmt.Sprintf("%v: %d, %d | %d, %d", sa.Name, sa.LeftX, sa.TopY, sa.RightX, sa.BottomY))
	// 		label.Refresh()
	// 	},
	// )
	// at.BoundImageSearchAreaList.OnSelected = func(lii widget.ListItemID) {
	// 	v, _ := at.BoundImageSearchSearchAreaStringList.GetValue(lii)
	// 	at.BoundImageSearch.SetValue("SearchArea", programs.CurrentProgramAndScreenSizeCoordinates().SearchAreas[v])
	// 	at.BoundSearchArea.SetValue("Name", v)
	// 	at.BoundImageSearch.Reload()
	// 	GetUi().Mui.MTabs.SelectedTab().Refresh()
	// }

	var (
		icons       = *assets.BytesToFyneIcons()
		searchList  = slices.Clone(items.AllItems("category"))
		bSearchList binding.ExternalStringList
	)
	bSearchList = binding.BindStringList(&searchList)

	// at.boundTargetsGridSearchBar = &widget.Entry{
	// 	PlaceHolder: "Search here",
	// 	ActionItem: widget.NewButtonWithIcon("", theme.RadioButtonIcon(), func() {
	// 		searchList = slices.Clone(items.AllItems("category"))
	// 		at.boundImageSearch.SetValue("Targets", []string{})
	// 		at.boundTargetsGridSearchBar.Text = ""
	// 		at.boundTargetsGridSearchBar.Refresh()
	// 		at.boundTargetsGrid.Refresh()
	// 		bSearchList.Reload()
	// 	}),
	// 	OnChanged: func(s string) {
	// 		defer bSearchList.Reload()
	// 		defer at.boundTargetsGrid.ScrollToTop()
	// 		defer at.boundTargetsGrid.Refresh()

	// 		if s == "" {
	// 			searchList = slices.Clone(items.AllItems("category"))
	// 			return
	// 		}
	// 		searchList = []string{}
	// 		for _, i := range items.AllItems("category") {
	// 			if fuzzy.MatchFold(s, i) || fuzzy.MatchFold(s, items.ItemsMap()[strings.ToLower(i)].Category) {
	// 				searchList = append(searchList, i)
	// 			}
	// 		}
	// 	},
	// }

	at.BoundTargetsGrid = widget.NewGridWrapWithData(
		bSearchList,
		func() fyne.CanvasObject {
			rect := canvas.NewRectangle(color.RGBA{})
			rect.SetMinSize(fyne.NewSquareSize(45))
			rect.CornerRadius = 5

			icon := canvas.NewImageFromResource(theme.BrokenImageIcon())
			icon.SetMinSize(fyne.NewSquareSize(40))
			icon.FillMode = canvas.ImageFillOriginal

			stack := container.NewStack(rect, container.NewPadded(icon))
			return stack
		},
		func(di binding.DataItem, o fyne.CanvasObject) {
			item := di.(binding.String)
			name, _ := item.Get()

			stack := o.(*fyne.Container)
			rect := stack.Objects[0].(*canvas.Rectangle)
			icon := stack.Objects[1].(*fyne.Container).Objects[0].(*canvas.Image)

			ist, _ := at.BoundImageSearch.GetValue("Targets")
			t := ist.([]string)

			if slices.Contains(t, name) {
				rect.FillColor = color.RGBA{R: 0, G: 128, B: 0, A: 128}
			} else {
				rect.FillColor = color.RGBA{}
			}

			path := name + ".png"
			if icons[path] != nil {
				icon.Resource = icons[path]
			} else {
				icon.Resource = theme.BrokenImageIcon()
			}
			o.Refresh()
		},
	)
	at.BoundTargetsGrid.OnSelected = func(id widget.GridWrapItemID) {
		defer at.BoundTargetsGrid.UnselectAll()
		defer at.BoundTargetsGrid.RefreshItem(id)
		ist, _ := at.BoundImageSearch.GetValue("Targets")
		t := ist.([]string)

		item := searchList[id]
		if !slices.Contains(t, item) {
			t = append(t, item)
		} else {
			i := slices.Index(t, item)
			if i != -1 {
				t = slices.Delete(t, i, i+1)
			}
		}
		at.BoundImageSearch.SetValue("Targets", t)
	}

	// at.boundXSplit = binding.BindInt(&xSplit)
	// at.boundYSplit = binding.BindInt(&ySplit)

	// at.boundXSplitSlider = widget.NewSliderWithData(0, 50, binding.IntToFloat(at.boundXSplit))
	// at.boundXSplitEntry = widget.NewEntryWithData(binding.IntToString(at.boundXSplit))
	// safi := widget.NewFormItem("Search Area:", widget.NewAccordion(widget.NewAccordionItem("Search Areas", at.boundImageSearchAreaList)))
	// safi.HintText = "rightX, topY, leftX, bottomY"
	imageSearchSettings :=
		container.NewScroll(
			container.NewBorder(
				widget.NewForm(
					widget.NewFormItem("Name:", at.BoundImageSearchNameEntry),
				),
				nil, nil, nil,
				widget.NewAccordion(
					widget.NewAccordionItem("Search Areas",
						container.NewBorder(
							nil, nil, nil, nil,
							at.SAAccordion,
						),
					),
					widget.NewAccordionItem("Items",
						container.NewBorder(
							at.BoundTargetsGridSearchBar, nil, nil, nil,
							at.BoundTargetsGrid,
						),
					),
				),
			),
		)
	at.Append(container.NewTabItem("Image", imageSearchSettings))

}

func (at *ActionTabs) constructOcrTab() {
	// 	at.boundOCRSearchAreaSelect = widget.NewSelectWithData(programs.CurrentProgramAndScreenSizeCoordinates().GetSearchAreasAsStringSlice(), binding.NewString())
	// 	at.boundOCRTargetEntry = widget.NewEntryWithData(binding.NewString())

	ocrSettings :=
		widget.NewForm(
			widget.NewFormItem("Text Target:", at.boundOCRTargetEntry),
			widget.NewFormItem("Search Area:", at.boundOCRSearchAreaSelect),
		)
	at.Append(container.NewTabItem("OCR", ocrSettings))

}

//	screen := robotgo.CaptureScreen(0, 0, 2560, 1440)
//	defer robotgo.FreeBitmap(screen)
//		mouseMoveDisplay := canvas.NewImageFromImage(robotgo.ToImage(screen))

// mouseMoveDisplayImage := canvas.NewImageFromFile("./internal/resources/images/full-screen.png")
// mouseMoveDisplayImage.FillMode = canvas.ImageFillStretch
// vLine := canvas.NewLine(colornames.Red)
// hLine := canvas.NewLine(colornames.Red)
// vLine.StrokeWidth = 2
// hLine.StrokeWidth = 2
// mouseMoveDisplayContainer := container.NewBorder(nil, nil, nil, nil, mouseMoveDisplayImage, vLine, hLine)
//	vLine.Position1 = mouseMoveDisplayContainer.Position()
// x, _ := u.st.boundMoveX.Get()
// vLine.Position1.X = float32(x)
// vLine.Position1.Y = 0
// vLine.Position2.X = float32(x)
// vLine.Position2.Y = mouseMoveDisplayImage.Size().Height
//	vLine.Position1.Y /= 2
//	hLine.Position1.X /= 2
//	hLine.Position1.Y /= 2
//	vLine.Position2.X /= 2
