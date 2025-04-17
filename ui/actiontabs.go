package ui

import (
	"Squire/internal/assets"
	"Squire/internal/config"
	"Squire/internal/programs"
	"Squire/internal/programs/actions"
	"Squire/internal/programs/coordinates"
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

	wait struct {
		boundTimeSlider *widget.Slider
		boundTimeEntry  *widget.Entry
	}

	move struct {
		boundMoveXSlider *widget.Slider
		boundMoveYSlider *widget.Slider
		boundMoveXEntry  *widget.Entry
		boundMoveYEntry  *widget.Entry
		// boundPointTree   *widget.Tree
		boundPointList *widget.List
		// boundSpotSelect  *widget.Select
	}
	click struct {
		boundButtonToggle *custom_widgets.Toggle
	}
	key struct {
		boundKeySelect   *widget.Select
		boundStateToggle *custom_widgets.Toggle
	}
	loop struct {
		boundLoopNameEntry *widget.Entry
		boundCountSlider   *widget.Slider
		boundCountLabel    *widget.Label
	}
	imageSearch struct {
		boundTargetsGridSearchBar  *widget.Entry
		boundTargetsGrid           *widget.GridWrap
		boundImageSearchNameEntry  *widget.Entry
		boundImageSearchAreaSelect *widget.Select
		// boundXSplitSlider          *widget.Slider
		// boundXSplitEntry           *widget.Entry
	}
	ocr struct {
		// boundOCRTarget     binding.String
		// boundOCRSearchArea binding.String

		boundOCRTargetEntry      *widget.Entry
		boundOCRSearchAreaSelect *widget.Select
	}
}

// action settings
var (
	macroList        []string
	macroName        string
	macroHotkey      []string
	selectedTreeItem = ""
	globalDelay      = 0
	button           bool
	key              string
	state            bool
)

func (u *Ui) constructActionSettingsTabs() {
	u.at.boundAdvancedAction = binding.BindStruct(&actions.AdvancedAction{})
	u.at.boundSearchArea = binding.BindStruct(&coordinates.SearchArea{})

	u.at.constructWaitTab()
	u.at.constructMoveTab()
	u.at.constructClickTab()
	u.at.constructKeyTab()
	u.at.constructLoopTab()
	u.at.constructImageSearchTab()
	u.at.constructOcrTab()
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
		tree, err := GetUi().mui.mtabs.GetTabTree()
		if err != nil {
			return
		}
		tree.Refresh()
	})
	ats := GetUi().at
	switch node := a.(type) {
	case *actions.Wait:
		ats.boundWait = binding.BindStruct(node)

		t, _ := ats.boundWait.GetItem("Time")
		ats.wait.boundTimeEntry.Bind(binding.IntToString(t.(binding.Int)))
		ats.wait.boundTimeSlider.Bind(binding.IntToFloat(t.(binding.Int)))

		t.AddListener(dl)
	case *actions.Move:
		ats.boundMove = binding.BindStruct(node)
		ats.boundPoint = binding.BindStruct(&node.Point)
		x, _ := ats.boundPoint.GetItem("X")
		y, _ := ats.boundPoint.GetItem("Y")

		ats.move.boundMoveXSlider.Bind(binding.IntToFloat(x.(binding.Int)))
		ats.move.boundMoveYSlider.Bind(binding.IntToFloat(y.(binding.Int)))
		ats.move.boundMoveXEntry.Bind(binding.IntToString(x.(binding.Int)))
		ats.move.boundMoveYEntry.Bind(binding.IntToString(y.(binding.Int)))
		// ats.move.boundSpotSelect.Bind())

		x.AddListener(dl)
		y.AddListener(dl)
	case *actions.Click:
		ats.boundClick = binding.BindStruct(node)
		b, _ := ats.boundClick.GetItem("Button")
		// v, _ := b.(binding.String).Get()
		// if v == actions.LeftOrRight(false) {
		// 	ats.click.boundButtonToggle.SetToggled(false)
		// } else {
		// 	ats.click.boundButtonToggle.SetToggled(false)
		// }
		ats.click.boundButtonToggle.Bind(binding.StringToBool(b.(binding.String)))
		b.AddListener(dl)
	case *actions.Key:
		ats.boundKey = binding.BindStruct(node)
		k, _ := ats.boundKey.GetItem("Key")
		s, _ := ats.boundKey.GetItem("State")

		// v, _ := s.(binding.String).Get()
		// if v == actions.UpOrDown(false) {
		// 	ats.key.boundStateToggle.SetToggled(false)
		// } else {
		// 	ats.key.boundStateToggle.SetToggled(false)
		// }

		ats.key.boundKeySelect.Bind(k.(binding.String))
		ats.key.boundStateToggle.Bind(binding.StringToBool(s.(binding.String)))

		k.AddListener(dl)
		s.AddListener(dl)

	case *actions.Loop:
		ats.boundLoop = binding.BindStruct(node)
		c, _ := ats.boundLoop.GetItem("Count")
		n, _ := ats.boundAdvancedAction.GetItem("Name")
		ats.loop.boundLoopNameEntry.Bind(n.(binding.String))
		ats.loop.boundCountLabel.Bind(binding.IntToString(c.(binding.Int)))
		ats.loop.boundCountSlider.Bind(binding.IntToFloat(c.(binding.Int)))
		c.AddListener(dl)
		n.AddListener(dl)
	case *actions.ImageSearch:
		ats.boundImageSearch = binding.BindStruct(node)
		ats.boundAdvancedAction = binding.BindStruct(node.AdvancedAction)
		ats.boundSearchArea = binding.BindStruct(&node.SearchArea)

		n, _ := ats.boundAdvancedAction.GetItem("Name")
		ats.imageSearch.boundImageSearchNameEntry.Bind(n.(binding.String))
		sa, _ := ats.boundSearchArea.GetItem("Name")
		ats.imageSearch.boundImageSearchAreaSelect.Bind(sa.(binding.String))

		ats.boundImageSearch.SetValue("Targets", slices.Clone(node.Targets))
		t, _ := ats.boundImageSearch.GetItem("Targets")

		t.AddListener(dl)
		n.AddListener(dl)
		sa.AddListener(dl)
	case *actions.Ocr:

	}
}

func (at *actionTabs) constructWaitTab() {
	at.wait.boundTimeEntry = widget.NewEntryWithData(binding.NewString())
	at.wait.boundTimeSlider = widget.NewSliderWithData(0.0, 1000.0, binding.NewFloat())
	waitSettings :=
		widget.NewForm(
			widget.NewFormItem("ms", container.NewGridWithColumns(2,
				at.wait.boundTimeEntry, at.wait.boundTimeSlider,
			)),
		)
	at.Append(container.NewTabItem("Wait", waitSettings))
}

func (at *actionTabs) constructMoveTab() {
	at.move.boundMoveXSlider = widget.NewSliderWithData(-1.0, float64(config.MonitorWidth), binding.NewFloat())
	at.move.boundMoveYSlider = widget.NewSliderWithData(-1.0, float64(config.MonitorHeight), binding.NewFloat())
	at.move.boundMoveXEntry = widget.NewEntryWithData(binding.NewString())
	at.move.boundMoveYEntry = widget.NewEntryWithData(binding.NewString())
	// at.move.boundPointList = widget.NewListWithData(
	// 	binding.NewStringList(),
	// 	func() fyne.CanvasObject {
	// 		return widget.NewLabel("")
	// 	},
	// 	func(di binding.DataItem, co fyne.CanvasObject) {},
	// )
	// at.move.boundPointList.OnSelected = func(id widget.ListItemID) {

	// }
	// at.move.boundPointTree = widget.NewTreeWithData(
	// )

	moveSettings :=
		container.NewBorder(
			widget.NewForm(
				widget.NewFormItem("X:", container.NewGridWithColumns(2,
					at.move.boundMoveXEntry, at.move.boundMoveXSlider,
				)),
				widget.NewFormItem("Y:", container.NewGridWithColumns(2,
					at.move.boundMoveYEntry, at.move.boundMoveYSlider,
				)),
			),
			nil, nil, nil,
			// at.move.boundPointList,
			// mouseMoveDisplayContainer
		)
	at.Append(container.NewTabItem("Move", moveSettings))
}

func (at *actionTabs) constructClickTab() {
	at.click.boundButtonToggle = custom_widgets.NewToggleWithData(binding.NewBool())

	clickSettings :=
		container.NewVBox(
			container.NewHBox(
				layout.NewSpacer(),
				widget.NewLabel("left"),
				at.click.boundButtonToggle,
				widget.NewLabel("right"),
				layout.NewSpacer(),
			),
		)
	at.Append(container.NewTabItem("Click", clickSettings))
}

func (at *actionTabs) constructKeyTab() {
	at.key.boundKeySelect = widget.NewSelectWithData([]string{"ctrl", "alt", "shift"}, binding.NewString()) //func(s string) { at.key.boundKey.Set(s) })
	at.key.boundStateToggle = custom_widgets.NewToggleWithData(binding.NewBool())

	keySettings :=
		container.NewVBox(
			container.NewHBox(
				layout.NewSpacer(),
				at.key.boundKeySelect,
				widget.NewLabel("up"),
				at.key.boundStateToggle,
				widget.NewLabel("down"),
				layout.NewSpacer(),
			),
		)
	at.Append(container.NewTabItem("Key", keySettings))
}

func (at *actionTabs) constructLoopTab() {
	at.loop.boundLoopNameEntry = widget.NewEntryWithData(binding.NewString())
	at.loop.boundCountSlider = widget.NewSliderWithData(1, 10, binding.IntToFloat(binding.NewInt()))
	at.loop.boundCountLabel = widget.NewLabelWithData(binding.NewString())

	loopSettings :=
		widget.NewForm(
			widget.NewFormItem("Name:", at.loop.boundLoopNameEntry),
			widget.NewFormItem("Loops:", container.NewBorder(
				nil, nil, at.loop.boundCountLabel, nil, at.loop.boundCountSlider,
			)),
		)

	at.Append(container.NewTabItem("Loop", loopSettings))

}

func (at *actionTabs) constructImageSearchTab() {
	is := actions.ImageSearch{AdvancedAction: &actions.AdvancedAction{}}

	at.imageSearch.boundImageSearchNameEntry = widget.NewEntryWithData(binding.NewString())
	at.imageSearch.boundImageSearchAreaSelect = widget.NewSelectWithData(
		programs.CurrentProgramAndScreenSizeCoordinates().GetSearchAreasAsStringSlice(),
		binding.NewString(),
	)
	bindAction(&is)

	var (
		icons       = *assets.BytesToFyneIcons()
		itemsStrMap = assets.Items.GetItemsMapAsStringsMap()
		allItems    = []string{}
		searchList  = []string{}
		bSearchList binding.ExternalStringList
	)
	bSearchList = binding.BindStringList(&searchList)

	for _, items := range itemsStrMap {
		allItems = append(allItems, items...)
	}
	searchList = slices.Clone(allItems)
	bSearchList.Reload()

	at.imageSearch.boundTargetsGridSearchBar = widget.NewEntry()
	at.imageSearch.boundTargetsGridSearchBar.PlaceHolder = "Search here"

	at.imageSearch.boundTargetsGridSearchBar.OnChanged = func(s string) {
		defer bSearchList.Reload()
		defer at.imageSearch.boundTargetsGrid.ScrollToTop()

		if s == "" {
			searchList = slices.Clone(allItems)
			return
		}
		searchList = []string{}
		for _, i := range allItems {
			if fuzzy.MatchFold(s, i) {
				searchList = append(searchList, i)
			}
		}
	}

	at.imageSearch.boundTargetsGrid = widget.NewGridWrapWithData(
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
					widget.NewLabel(""),
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
			label := stack.Objects[1].(*widget.Label)
			label.Bind(item)
			icon := stack.Objects[2].(*fyne.Container).Objects[0].(*canvas.Image)

			ist, _ := at.boundImageSearch.GetValue("Targets")
			t := ist.([]string)

			if slices.Contains(t, name) {
				rect.FillColor = color.RGBA{R: 0, G: 128, B: 0, A: 128}
			} else {
				rect.FillColor = color.RGBA{}
			}

			label.Hidden = true

			path := name + ".png"
			if icons[path] != nil {
				icon.Resource = icons[path]
			} else {
				icon.Resource = theme.BrokenImageIcon()
			}
			o.Refresh()
		},
	)
	at.imageSearch.boundTargetsGrid.OnSelected = func(id widget.GridWrapItemID) {
		defer at.imageSearch.boundTargetsGrid.UnselectAll()
		defer at.imageSearch.boundTargetsGrid.RefreshItem(id)
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

	// at.imageSearch.boundXSplit = binding.BindInt(&xSplit)
	// at.imageSearch.boundYSplit = binding.BindInt(&ySplit)

	// at.imageSearch.boundXSplitSlider = widget.NewSliderWithData(0, 50, binding.IntToFloat(at.imageSearch.boundXSplit))
	// at.imageSearch.boundXSplitEntry = widget.NewEntryWithData(binding.IntToString(at.imageSearch.boundXSplit))

	imageSearchSettings :=
		container.NewBorder(
			widget.NewForm(
				widget.NewFormItem("Name:", at.imageSearch.boundImageSearchNameEntry),
				widget.NewFormItem("Search Area:", at.imageSearch.boundImageSearchAreaSelect),
				widget.NewFormItem("Items:", at.imageSearch.boundTargetsGridSearchBar),
			),
			nil, nil, nil,
			container.NewScroll(
				at.imageSearch.boundTargetsGrid,
			),
		)
	at.Append(container.NewTabItem("Image", imageSearchSettings))

}

func (at *actionTabs) constructOcrTab() {
	at.ocr.boundOCRSearchAreaSelect = widget.NewSelectWithData(programs.CurrentProgramAndScreenSizeCoordinates().GetSearchAreasAsStringSlice(), binding.NewString())
	at.ocr.boundOCRTargetEntry = widget.NewEntryWithData(binding.NewString())

	ocrSettings :=
		widget.NewForm(
			widget.NewFormItem("Text Target:", at.ocr.boundOCRTargetEntry),
			widget.NewFormItem("Search Area:", at.ocr.boundOCRSearchAreaSelect),
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
