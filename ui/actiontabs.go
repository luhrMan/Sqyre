package ui

import (
	"Squire/internal/assets"
	"Squire/internal/programs"
	"Squire/internal/programs/actions"
	"Squire/internal/programs/coordinates"
	"Squire/internal/programs/items"
	"Squire/ui/custom_widgets"
	"fmt"
	"image/color"
	"log"
	"slices"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	"github.com/lithammer/fuzzysearch/fuzzy"
)

const (
	waittab = iota
	movetab
	clicktab
	keytab
	looptab
	imagesearchtab
	ocrtab
)

type actionTabs struct {
	*container.AppTabs
	boundBaseAction     binding.Struct
	boundAdvancedAction binding.Struct

	boundWait  binding.Struct
	boundKey   binding.Struct
	boundMove  binding.Struct
	boundClick binding.Struct

	boundLoop        binding.Struct
	boundImageSearch binding.Struct
	boundOcr         binding.Struct

	boundSearchArea binding.Struct
	boundPoint      binding.Struct

	boundTimeSlider *widget.Slider
	boundTimeEntry  *widget.Entry

	boundMoveXSlider *widget.Slider
	boundMoveYSlider *widget.Slider
	boundMoveXEntry  *widget.Entry
	boundMoveYEntry  *widget.Entry
	// boundPointTree   *widget.Tree
	boundPointList *widget.List
	// boundSpotSelect  *widget.Select
	boundButtonToggle *custom_widgets.Toggle
	boundKeySelect    *widget.Select
	boundStateToggle  *custom_widgets.Toggle

	boundLoopNameEntry *widget.Entry
	boundCountSlider   *widget.Slider
	boundCountLabel    *widget.Label

	boundTargetsGridSearchBar *widget.Entry
	boundTargetsGrid          *widget.GridWrap
	boundImageSearchNameEntry *widget.Entry
	// boundImageSearchAreaSelect *widget.Select
	boundImageSearchSearchAreaStringList binding.ExternalStringList
	boundImageSearchAreaList             *widget.List
	// boundXSplitSlider          *widget.Slider
	// boundXSplitEntry           *widget.Entry
	// boundOCRTarget     binding.String
	// boundOCRSearchArea binding.String

	boundOCRTargetEntry      *widget.Entry
	boundOCRSearchAreaSelect *widget.Select
}

func (at *actionTabs) constructActionSettingsTabs() {
	at.boundAdvancedAction = binding.BindStruct(&actions.AdvancedAction{})
	at.boundSearchArea = binding.BindStruct(&coordinates.SearchArea{})

	at.constructWaitTab()
	at.constructMoveTab()
	at.constructClickTab()
	at.constructKeyTab()
	at.constructLoopTab()
	at.constructImageSearchTab()
	at.constructOcrTab()
}

func unbindAll() {
	bindAction(&actions.Wait{})
	bindAction(&actions.Move{})
	bindAction(&actions.Click{})
	bindAction(&actions.Key{})

	bindAction(&actions.Loop{AdvancedAction: &actions.AdvancedAction{}})
	bindAction(&actions.ImageSearch{AdvancedAction: &actions.AdvancedAction{}, SearchArea: coordinates.SearchArea{}})
	bindAction(&actions.Ocr{AdvancedAction: &actions.AdvancedAction{}, SearchArea: coordinates.SearchArea{}})
}

func bindAction(a actions.ActionInterface) {
	dl := binding.NewDataListener(func() {
		mt := ui.mui.mtabs.selectedTab()
		fyne.Do(func() { mt.RefreshItem(selectedTreeItem) })
	})
	ats := GetUi().at
	switch node := a.(type) {
	case *actions.Wait:
		ats.boundWait = binding.BindStruct(node)
		t, _ := ats.boundWait.GetItem("Time")

		ats.boundTimeEntry.Bind(binding.IntToString(t.(binding.Int)))
		ats.boundTimeSlider.Bind(binding.IntToFloat(t.(binding.Int)))

		t.AddListener(dl)
	case *actions.Move:
		ats.boundMove = binding.BindStruct(node)
		ats.boundPoint = binding.BindStruct(&node.Point)
		x, _ := ats.boundPoint.GetItem("X")
		y, _ := ats.boundPoint.GetItem("Y")

		ats.boundMoveXSlider.Bind(binding.IntToFloat(x.(binding.Int)))
		ats.boundMoveYSlider.Bind(binding.IntToFloat(y.(binding.Int)))
		ats.boundMoveXEntry.Bind(binding.IntToString(x.(binding.Int)))
		ats.boundMoveYEntry.Bind(binding.IntToString(y.(binding.Int)))
		// ats.boundSpotSelect.Bind())

		x.AddListener(dl)
		y.AddListener(dl)
	case *actions.Click:
		ats.boundClick = binding.BindStruct(node)
		b, _ := ats.boundClick.GetItem("Button")

		ats.boundButtonToggle.Bind(custom_widgets.CustomStringToBool(b.(binding.String), "click", dl))

		b.AddListener(dl)
	case *actions.Key:
		ats.boundKey = binding.BindStruct(node)
		k, _ := ats.boundKey.GetItem("Key")
		s, _ := ats.boundKey.GetItem("State")

		ats.boundKeySelect.Bind(k.(binding.String))
		ats.boundStateToggle.Bind(custom_widgets.CustomStringToBool(s.(binding.String), "key", dl))

		k.AddListener(dl)
		s.AddListener(dl)

	case *actions.Loop:
		ats.boundLoop = binding.BindStruct(node)
		ats.boundAdvancedAction = binding.BindStruct(node.AdvancedAction)
		c, _ := ats.boundLoop.GetItem("Count")
		n, _ := ats.boundAdvancedAction.GetItem("Name")

		ats.boundLoopNameEntry.Bind(n.(binding.String))
		ats.boundCountLabel.Bind(binding.IntToString(c.(binding.Int)))
		ats.boundCountSlider.Bind(binding.IntToFloat(c.(binding.Int)))

		c.AddListener(dl)
		n.AddListener(dl)
	case *actions.ImageSearch:
		ats.boundImageSearch = binding.BindStruct(node)
		ats.boundAdvancedAction = binding.BindStruct(node.AdvancedAction)
		ats.boundSearchArea = binding.BindStruct(&node.SearchArea)

		n, _ := ats.boundAdvancedAction.GetItem("Name")
		sa, _ := ats.boundSearchArea.GetItem("Name")
		t, _ := ats.boundImageSearch.GetItem("Targets")

		ats.boundImageSearchNameEntry.Bind(n.(binding.String))
		// ats.boundImageSearchAreaList.Select(slices.Index(programs.CurrentProgramAndScreenSizeCoordinates().GetSearchAreasAsStringSlice(), node.SearchArea.Name))
		v, _ := ats.boundImageSearchSearchAreaStringList.Get()
		for i, s := range v {
			if s == node.SearchArea.Name {
				ats.boundImageSearchAreaList.Select(i)
			}
		}
		// ats.boundImageSearchAreaSelect.Bind(sa.(binding.String))
		ats.boundImageSearch.SetValue("Targets", slices.Clone(node.Targets))

		t.AddListener(dl)
		n.AddListener(dl)
		sa.AddListener(dl)
	case *actions.Ocr:
		ats.boundOcr = binding.BindStruct(node)
		ats.boundAdvancedAction = binding.BindStruct(node.AdvancedAction)
		ats.boundSearchArea = binding.BindStruct(&node.SearchArea)

		t, _ := ats.boundOcr.GetItem("Target")
		n, _ := ats.boundAdvancedAction.GetItem("Name")
		sa, _ := ats.boundSearchArea.GetItem("Name")

		t.AddListener(dl)
		n.AddListener(dl)
		sa.AddListener(dl)
	}
}

func (at *actionTabs) constructWaitTab() {
	gdfi := widget.NewFormItem("delay", GetUi().mui.mtabs.boundGlobalDelayEntry)
	gdfi.HintText = "keyboard & mouse global delay (ms)"

	waitSettings :=
		widget.NewForm(
			widget.NewFormItem("ms", container.NewGridWithColumns(2,
				at.boundTimeEntry, at.boundTimeSlider,
			)),
			gdfi,
		)
	at.Append(container.NewTabItem("Wait", waitSettings))
}

func (at *actionTabs) constructMoveTab() {
	// at.boundPointList = widget.NewListWithData(
	// 	binding.NewStringList(),
	// 	func() fyne.CanvasObject {
	// 		return widget.NewLabel("")
	// 	},
	// 	func(di binding.DataItem, co fyne.CanvasObject) {},
	// )
	// at.boundPointList.OnSelected = func(id widget.ListItemID) {

	// }
	// at.boundPointTree = widget.NewTreeWithData(
	// )

	moveSettings :=
		container.NewBorder(
			widget.NewForm(
				widget.NewFormItem("X:", container.NewGridWithColumns(2,
					at.boundMoveXEntry, at.boundMoveXSlider,
				)),
				widget.NewFormItem("Y:", container.NewGridWithColumns(2,
					at.boundMoveYEntry, at.boundMoveYSlider,
				)),
			),
			nil, nil, nil,
			// at.boundPointList,
			// mouseMoveDisplayContainer
		)
	at.Append(container.NewTabItem("Move", moveSettings))
}

func (at *actionTabs) constructClickTab() {
	clickSettings :=
		container.NewVBox(
			container.NewHBox(
				layout.NewSpacer(),
				widget.NewLabel("left"),
				at.boundButtonToggle,
				widget.NewLabel("right"),
				layout.NewSpacer(),
			),
		)
	at.Append(container.NewTabItem("Click", clickSettings))
}

func (at *actionTabs) constructKeyTab() {
	keySettings :=
		container.NewVBox(
			container.NewHBox(
				layout.NewSpacer(),
				at.boundKeySelect,
				widget.NewLabel("up"),
				at.boundStateToggle,
				widget.NewLabel("down"),
				layout.NewSpacer(),
			),
		)
	at.Append(container.NewTabItem("Key", keySettings))
}

func (at *actionTabs) constructLoopTab() {
	loopSettings :=
		widget.NewForm(
			widget.NewFormItem("Name:", at.boundLoopNameEntry),
			widget.NewFormItem("Loops:", container.NewBorder(
				nil, nil, at.boundCountLabel, nil, at.boundCountSlider,
			)),
		)
	at.Append(container.NewTabItem("Loop", loopSettings))
}

func (at *actionTabs) constructImageSearchTab() {
	var (
		saSearchList = slices.Clone(programs.CurrentProgramAndScreenSizeCoordinates().GetSearchAreasAsStringSlice())
		// boundImageSearchSearchAreaStringList binding.ExternalStringList
	)
	at.boundImageSearchSearchAreaStringList = binding.BindStringList(&saSearchList)

	at.boundImageSearchAreaList = widget.NewListWithData(
		at.boundImageSearchSearchAreaStringList,
		func() fyne.CanvasObject { return widget.NewLabel("template") },
		func(di binding.DataItem, co fyne.CanvasObject) {
			bsa := di.(binding.String)
			label := co.(*widget.Label)
			v, _ := bsa.Get()
			sa := programs.CurrentProgramAndScreenSizeCoordinates().GetSearchArea(v)
			label.SetText(fmt.Sprintf("%v: %d, %d | %d, %d)", sa.Name, sa.LeftX, sa.TopY, sa.RightX, sa.BottomY))
			label.Refresh()
		},
	)
	at.boundImageSearchAreaList.OnSelected = func(lii widget.ListItemID) {
		v, _ := at.boundImageSearchSearchAreaStringList.GetValue(lii)
		at.boundImageSearch.SetValue("SearchArea", programs.CurrentProgramAndScreenSizeCoordinates().SearchAreas[v])
		at.boundImageSearch.Reload()
		GetUi().mui.mtabs.selectedTab().Refresh()
	}
	log.Println(items.AllItems("category"))
	// log.Println(items.AllItems("none"))
	var (
		icons       = *assets.BytesToFyneIcons()
		searchList  = slices.Clone(items.AllItems("category"))
		bSearchList binding.ExternalStringList
	)
	bSearchList = binding.BindStringList(&searchList)

	at.boundTargetsGridSearchBar = widget.NewEntry()
	at.boundTargetsGridSearchBar.PlaceHolder = "Search here"
	at.boundTargetsGridSearchBar.ActionItem = widget.NewButtonWithIcon("", theme.RadioButtonIcon(), func() {
		searchList = slices.Clone(items.AllItems("category"))
		at.boundImageSearch.SetValue("Targets", []string{})
		at.boundTargetsGrid.Refresh()
		bSearchList.Reload()
	})

	at.boundTargetsGridSearchBar.OnChanged = func(s string) {
		defer bSearchList.Reload()
		defer at.boundTargetsGrid.ScrollToTop()
		defer at.boundTargetsGrid.Refresh()

		if s == "" {
			searchList = slices.Clone(items.AllItems("category"))
			return
		}
		searchList = []string{}
		for _, i := range items.AllItems("category") {
			if fuzzy.MatchFold(s, i) || fuzzy.MatchFold(s, items.ItemsMap()[strings.ToLower(i)].Category) {
				searchList = append(searchList, i)
			}
		}
	}

	at.boundTargetsGrid = widget.NewGridWrapWithData(
		bSearchList,
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
					container.NewPadded(
						icon,
					),
				)
			return stack
		},
		func(di binding.DataItem, o fyne.CanvasObject) {
			item := di.(binding.String)
			name, _ := item.Get()

			stack := o.(*fyne.Container)
			rect := stack.Objects[0].(*canvas.Rectangle)
			icon := stack.Objects[1].(*fyne.Container).Objects[0].(*canvas.Image)

			ist, _ := at.boundImageSearch.GetValue("Targets")
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
	at.boundTargetsGrid.OnSelected = func(id widget.GridWrapItemID) {
		defer at.boundTargetsGrid.UnselectAll()
		defer at.boundTargetsGrid.RefreshItem(id)
		ist, _ := at.boundImageSearch.GetValue("Targets")
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
		at.boundImageSearch.SetValue("Targets", t)
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
					widget.NewFormItem("Name:", at.boundImageSearchNameEntry),
				),
				nil, nil, nil,
				widget.NewAccordion(
					widget.NewAccordionItem("Search Areas", at.boundImageSearchAreaList),
					widget.NewAccordionItem("Items", container.NewScroll(
						container.NewBorder(
							at.boundTargetsGridSearchBar, nil, nil, nil,
							at.boundTargetsGrid,
						),
					)),
				),
			),
		)
	at.Append(container.NewTabItem("Image", imageSearchSettings))

}

func (at *actionTabs) constructOcrTab() {
	at.boundOCRSearchAreaSelect = widget.NewSelectWithData(programs.CurrentProgramAndScreenSizeCoordinates().GetSearchAreasAsStringSlice(), binding.NewString())
	at.boundOCRTargetEntry = widget.NewEntryWithData(binding.NewString())

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
