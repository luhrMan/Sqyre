package ui

import (
	"Squire/ui/custom_widgets"

	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/layout"
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
	BoundWait  binding.Struct
	BoundMove  binding.Struct
	BoundPoint binding.Struct
	BoundClick binding.Struct
	BoundKey   binding.Struct

	BoundLoop          binding.Struct
	BoundLoopAA        binding.Struct
	BoundImageSearch   binding.Struct
	BoundImageSearchAA binding.Struct
	BoundImageSearchSA binding.Struct
	BoundOcr           binding.Struct
	BoundOcrAA         binding.Struct
	BoundOcrSA         binding.Struct

	BoundTimeSlider *widget.Slider
	BoundTimeEntry  *widget.Entry

	PointsAccordion *widget.Accordion

	BoundButtonToggle *custom_widgets.Toggle

	BoundKeySelect   *widget.Select
	BoundStateToggle *custom_widgets.Toggle

	BoundLoopNameEntry *widget.Entry
	BoundCountSlider   *widget.Slider
	BoundCountLabel    *widget.Label

	BoundImageSearchNameEntry     *widget.Entry
	BoundImageSearchColSplitEntry *widget.Entry
	BoundImageSearchRowSplitEntry *widget.Entry
	ImageSearchSAAccordion        *widget.Accordion
	ImageSearchItemsAccordion     *widget.Accordion

	// boundXSplitSlider          *widget.Slider
	// boundXSplitEntry           *widget.Entry

	boundOCRTargetEntry *widget.Entry
	OCRSAAccordion      *widget.Accordion
}

func newActionTabs() *ActionTabs {
	return &ActionTabs{
		AppTabs: &container.AppTabs{},

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

		BoundImageSearchNameEntry:     widget.NewEntryWithData(binding.NewString()),
		BoundImageSearchColSplitEntry: widget.NewEntryWithData(binding.NewString()),
		BoundImageSearchRowSplitEntry: widget.NewEntryWithData(binding.NewString()),

		ImageSearchSAAccordion:    widget.NewAccordion(),
		ImageSearchItemsAccordion: widget.NewAccordion(),

		boundOCRTargetEntry: widget.NewEntryWithData(binding.NewString()),
		OCRSAAccordion:      widget.NewAccordion(),
	}
}

func (u *Ui) constructActionTabs() *ActionTabs {
	u.ActionTabs.constructWaitTab()
	u.ActionTabs.constructMoveTab()
	u.ActionTabs.constructClickTab()
	u.ActionTabs.constructKeyTab()
	u.ActionTabs.constructLoopTab()
	u.ActionTabs.constructImageSearchTab()
	u.ActionTabs.constructOcrTab()
	return u.ActionTabs
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
	moveSettings :=
		container.NewBorder(
			nil, nil, nil, nil,
			at.PointsAccordion,
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

	// var (
	// 	icons       = *assets.BytesToFyneIcons()
	// 	searchList  = slices.Clone(items.AllItems("category"))
	// 	bSearchList binding.ExternalStringList
	// )
	// bSearchList = binding.BindStringList(&searchList)

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

					widget.NewFormItem("Cols: ",
						container.NewGridWithColumns(3,
							at.BoundImageSearchColSplitEntry,
							widget.NewLabel("Rows:"),
							at.BoundImageSearchRowSplitEntry,
						),
					),
					// widget.NewFormItem("Rows:", at.BoundImageSearchRowSplitEntry),
				),
				nil, nil, nil,
				widget.NewAccordion(
					widget.NewAccordionItem("Search Areas",
						container.NewBorder(
							nil, nil, nil, nil,
							at.ImageSearchSAAccordion,
						),
					),
					widget.NewAccordionItem("Items",
						container.NewBorder(
							nil, nil, nil, nil,
							at.ImageSearchItemsAccordion,
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
			widget.NewFormItem("Search Area:", at.OCRSAAccordion),
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
