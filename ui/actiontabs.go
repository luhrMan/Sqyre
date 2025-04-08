package ui

import (
	"Squire/internal/assets"
	"Squire/internal/config"
	"Squire/internal/programs"
	"Squire/internal/programs/actions"
	"Squire/ui/custom_widgets"
	"log"

	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/widget"
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
	wait struct {
		boundTime binding.Int

		boundTimeSlider *widget.Slider
		boundTimeEntry  *widget.Entry
	}
	move struct {
		boundMoveX binding.Int
		boundMoveY binding.Int
		boundSpot  binding.String

		boundMoveXSlider *widget.Slider
		boundMoveYSlider *widget.Slider
		boundMoveXEntry  *widget.Entry
		boundMoveYEntry  *widget.Entry
		boundSpotSelect  *widget.Select
	}
	click struct {
		boundButton binding.Bool

		boundButtonToggle *custom_widgets.Toggle
	}
	key struct {
		boundKey   binding.String
		boundState binding.Bool

		boundKeySelect   *widget.Select
		boundStateToggle *custom_widgets.Toggle
	}
	loop struct {
		boundLoopName binding.String
		boundCount    binding.Int

		boundLoopNameEntry *widget.Entry
		boundCountSlider   *widget.Slider
		boundCountLabel    *widget.Label
	}
	imageSearch struct {
		boundImageSearchName    binding.String
		boundImageSearchArea    binding.String
		boundImageSearchTargets binding.StringList
		boundXSplit             binding.Int
		boundYSplit             binding.Int

		boundImageSearchNameEntry  *widget.Entry
		boundImageSearchAreaSelect *widget.Select
		boundXSplitSlider          *widget.Slider
		boundXSplitEntry           *widget.Entry
	}
	ocr struct {
		boundOCRTarget     binding.String
		boundOCRSearchArea binding.String

		boundOCRTargetEntry      *widget.Entry
		boundOCRSearchAreaSelect *widget.Select
	}
}

// action settings
var (
	macroList          []string
	macroName          string
	macroHotkey        []string
	selectedTreeItem   = ".1"
	time               int
	globalDelay        = 0
	moveX              int
	moveY              int
	spot               string
	button             bool
	key                string
	state              bool
	loopName           string
	count              int = 1
	imageSearchName    string
	searchArea         string
	xSplit             int
	ySplit             int
	itemsBoolList      = assets.Items.GetItemsMapAsBool()
	imageSearchTargets []string
	ocrName            string
	ocrTarget          string
	ocrSearchBox       string
)

func (u *Ui) constructActionSettingsTabs() {
	u.at.constructWaitTab()
	u.at.constructMoveTab()
	u.at.constructClickTab()
	u.at.constructKeyTab()
	u.at.constructLoopTab()
	u.at.constructImageSearchTab()
	u.at.constructOcrTab()
}

func (at *actionTabs) constructWaitTab() {
	at.wait.boundTime = binding.BindInt(&time)
	at.wait.boundTimeEntry = widget.NewEntryWithData(binding.IntToString(at.wait.boundTime))
	at.wait.boundTimeSlider = widget.NewSliderWithData(0.0, 250.0, binding.IntToFloat(at.wait.boundTime))
	at.wait.boundTime.AddListener(binding.NewDataListener(func() {
		t, err := ui.mui.mtabs.GetTabTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.Wait); ok {
			n.Time = time
			t.Tree.Refresh()
		}
	}))
	waitSettings :=
		container.NewVBox(
			container.NewGridWithColumns(
				2,
				container.NewBorder(
					nil, nil, nil,
					container.NewHBox(widget.NewLabel("ms")), at.wait.boundTimeEntry,
				),
				at.wait.boundTimeSlider),
		)
	at.Append(container.NewTabItem("Wait", waitSettings))

}

func (at *actionTabs) constructMoveTab() {
	at.move.boundMoveX = binding.BindInt(&moveX)
	at.move.boundMoveY = binding.BindInt(&moveY)
	at.move.boundMoveXSlider = widget.NewSliderWithData(-1.0, float64(config.MonitorWidth), binding.IntToFloat(at.move.boundMoveX))
	at.move.boundMoveYSlider = widget.NewSliderWithData(-1.0, float64(config.MonitorHeight), binding.IntToFloat(at.move.boundMoveY))
	at.move.boundMoveXEntry = widget.NewEntryWithData(binding.IntToString(at.move.boundMoveX))
	at.move.boundMoveYEntry = widget.NewEntryWithData(binding.IntToString(at.move.boundMoveY))
	at.move.boundSpot = binding.BindString(&spot)
	at.move.boundSpotSelect = widget.NewSelect(programs.CurrentProgramAndScreenSizeCoordinates().GetPointsAsStringSlice(), func(s string) {
		at.move.boundSpot.Set(s)
		at.move.boundMoveX.Set(programs.CurrentProgramAndScreenSizeCoordinates().GetPoint(s).X)
		at.move.boundMoveY.Set(programs.CurrentProgramAndScreenSizeCoordinates().GetPoint(s).Y)
	})
	at.move.boundMoveX.AddListener(binding.NewDataListener(func() {
		t, err := ui.mui.mtabs.GetTabTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.Move); ok {
			n.X = moveX
			t.Tree.Refresh()
		}
	}))
	at.move.boundMoveY.AddListener(binding.NewDataListener(func() {
		t, err := ui.mui.mtabs.GetTabTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.Move); ok {
			n.Y = moveY
			t.Tree.Refresh()
		}
	}))

	moveSettings :=
		container.NewBorder(
			container.NewVBox(
				container.NewGridWithColumns(
					2,
					container.NewBorder(
						nil, nil,
						container.NewHBox(widget.NewLabel("X:")),
						nil, at.move.boundMoveXEntry),
					at.move.boundMoveXSlider,
					container.NewBorder(
						nil, nil,
						container.NewHBox(widget.NewLabel("Y:")),
						nil, at.move.boundMoveYEntry),
					at.move.boundMoveYSlider,
					container.NewHBox(layout.NewSpacer(), widget.NewLabel("Spot:")),
					at.move.boundSpotSelect,
				),
			),
			nil, nil, nil,
		) //, mouseMoveDisplayContainer)
	at.Append(container.NewTabItem("Move", moveSettings))
}

func (at *actionTabs) constructClickTab() {
	at.click.boundButton = binding.BindBool(&button)
	at.click.boundButtonToggle = custom_widgets.NewToggleWithData(at.click.boundButton)
	at.click.boundButton.AddListener(binding.NewDataListener(func() {
		t, err := ui.mui.mtabs.GetTabTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.Click); ok {
			n.Button = actions.LeftOrRight(button)
			t.Tree.Refresh()
		}
	}))
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
	at.key.boundKey = binding.BindString(&key)
	at.key.boundKeySelect = widget.NewSelect([]string{"ctrl", "alt", "shift"}, func(s string) { at.key.boundKey.Set(s) })
	at.key.boundState = binding.BindBool(&state)
	at.key.boundStateToggle = custom_widgets.NewToggleWithData(at.key.boundState)
	at.key.boundKey.AddListener(binding.NewDataListener(func() {
		t, err := ui.mui.mtabs.GetTabTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.Key); ok {
			n.Key = key
			t.Tree.Refresh()
		}
	}))
	at.key.boundState.AddListener(binding.NewDataListener(func() {
		t, err := ui.mui.mtabs.GetTabTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.Key); ok {
			n.State = actions.UpOrDown(state)
			t.Tree.Refresh()
		}
	}))
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
	at.loop.boundLoopName = binding.BindString(&loopName)
	at.loop.boundCount = binding.BindInt(&count)
	at.loop.boundLoopNameEntry = widget.NewEntryWithData(at.loop.boundLoopName)
	at.loop.boundCountSlider = widget.NewSliderWithData(1, 10, binding.IntToFloat(at.loop.boundCount))
	at.loop.boundCountLabel = widget.NewLabelWithData(binding.IntToString(at.loop.boundCount))
	at.loop.boundLoopName.AddListener(binding.NewDataListener(func() {
		t, err := ui.mui.mtabs.GetTabTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.Loop); ok {
			n.Name = loopName
			t.Tree.Refresh()
		}
	}))
	at.loop.boundCount.AddListener(binding.NewDataListener(func() {
		t, err := ui.mui.mtabs.GetTabTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.Loop); ok {
			n.Count = count
			t.Tree.Refresh()
		}
	}))

	loopSettings :=
		container.NewVBox(
			container.NewGridWithColumns(
				2,
				container.NewHBox(
					layout.NewSpacer(),
					widget.NewLabel("name:"),
				),
				at.loop.boundLoopNameEntry,
			),
			container.NewGridWithColumns(
				2,
				container.NewHBox(
					layout.NewSpacer(),
					widget.NewLabel("loops:"),
					at.loop.boundCountLabel),
				at.loop.boundCountSlider,
			),
		)
	at.Append(container.NewTabItem("Loop", loopSettings))

}

func (at *actionTabs) constructImageSearchTab() {
	at.imageSearch.boundImageSearchName = binding.BindString(&imageSearchName)
	at.imageSearch.boundImageSearchArea = binding.BindString(&searchArea)
	at.imageSearch.boundImageSearchTargets = binding.BindStringList(&imageSearchTargets)
	at.imageSearch.boundImageSearchTargets.AddListener(binding.NewDataListener(func() {
		t, err := ui.mui.mtabs.GetTabTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.ImageSearch); ok {
			n.Targets = imageSearchTargets
			t.Tree.Refresh()
		}
	}))
	at.imageSearch.boundXSplit = binding.BindInt(&xSplit)
	at.imageSearch.boundYSplit = binding.BindInt(&ySplit)
	at.imageSearch.boundImageSearchNameEntry = widget.NewEntryWithData(at.imageSearch.boundImageSearchName)
	at.imageSearch.boundImageSearchAreaSelect = widget.NewSelect(programs.CurrentProgramAndScreenSizeCoordinates().GetSearchAreasAsStringSlice(),
		func(s string) { at.imageSearch.boundImageSearchArea.Set(s) })

	at.imageSearch.boundXSplitSlider = widget.NewSliderWithData(0, 50, binding.IntToFloat(at.imageSearch.boundXSplit))
	at.imageSearch.boundXSplitEntry = widget.NewEntryWithData(binding.IntToString(at.imageSearch.boundXSplit))
	at.imageSearch.boundImageSearchName.AddListener(binding.NewDataListener(func() {
		t, err := ui.mui.mtabs.GetTabTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.ImageSearch); ok {
			n.Name = imageSearchName
			t.Tree.Refresh()
		}
	}))
	at.imageSearch.boundImageSearchArea.AddListener(binding.NewDataListener(func() {
		t, err := ui.mui.mtabs.GetTabTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.ImageSearch); ok {
			n.SearchArea = programs.CurrentProgramAndScreenSizeCoordinates().GetSearchArea(searchArea)
			t.Tree.Refresh()
		}
	}))

	imageSearchSettings :=
		container.NewBorder(
			container.NewVBox(
				container.NewGridWithColumns(
					2,
					container.NewHBox(
						widget.NewLabel("name:"),
					),
					at.imageSearch.boundImageSearchNameEntry,
				),
				container.NewGridWithColumns(
					2,
					container.NewHBox(
						widget.NewLabel("search area:"),
					),
					at.imageSearch.boundImageSearchAreaSelect,
				),
				container.NewGridWithColumns(
					3,
					container.NewHBox(
						widget.NewLabel("screen split cols:"),
					),
					at.imageSearch.boundXSplitSlider,
					at.imageSearch.boundXSplitEntry,
				),
			),
			nil, nil, nil,
			// container.NewBorder(nil, nil, nil, nil,),
			//			u.st.boundImageSearchTargetsTree,
			at.createItemsCheckTree(),
		)
	at.Append(container.NewTabItem("Image", imageSearchSettings))

}

func (at *actionTabs) constructOcrTab() {
	at.ocr.boundOCRSearchArea = binding.BindString(&ocrSearchBox)
	at.ocr.boundOCRTarget = binding.BindString(&ocrTarget)
	at.ocr.boundOCRSearchAreaSelect = widget.NewSelect(programs.CurrentProgramAndScreenSizeCoordinates().GetSearchAreasAsStringSlice(), func(s string) { at.ocr.boundOCRSearchArea.Set(s) })
	at.ocr.boundOCRTargetEntry = widget.NewEntryWithData(at.ocr.boundOCRTarget)
	at.ocr.boundOCRSearchArea.AddListener(binding.NewDataListener(func() {
		t, err := ui.mui.mtabs.GetTabTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.Ocr); ok {
			n.SearchArea = programs.CurrentProgramAndScreenSizeCoordinates().GetSearchArea(searchArea)
			t.Tree.Refresh()
		}
	}))
	at.ocr.boundOCRTarget.AddListener(binding.NewDataListener(func() {
		t, err := ui.mui.mtabs.GetTabTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.Ocr); ok {
			n.Target = ocrTarget
			t.Tree.Refresh()
		}
	}))

	ocrSettings :=
		container.NewBorder(
			container.NewGridWithColumns(
				1,
				container.NewBorder(
					nil, nil,
					container.NewHBox(
						widget.NewLabel("Text Target:"),
					),
					nil,
					at.ocr.boundOCRTargetEntry,
				),
				container.NewBorder(
					nil, nil,
					container.NewHBox(
						widget.NewLabel("Search Area:"),
					),
					nil,
					at.ocr.boundOCRSearchAreaSelect,
				),
			),
			nil, nil, nil,
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
