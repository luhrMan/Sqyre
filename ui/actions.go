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
	"github.com/go-vgo/robotgo"
)

// action settings
var (
	macroName          string
	selectedTreeItem   = ".1"
	time               int
	globalDelay        = 30
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
	imageSearchTargets = assets.Items.GetItemsMapAsBool()
	ocrName            string
	ocrTarget          string
	ocrSearchBox       string
)

func (u *Ui) actionSettingsTabs() {
	u.bindVariables()
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
	var (
		waitSettings = container.NewVBox(
			container.NewGridWithColumns(2, container.NewHBox(widget.NewLabel("Global Delay"), u.st.boundGlobalDelayEntry, layout.NewSpacer(), widget.NewLabel("ms"))),
			widget.NewLabel("------------------------------------------------------------------------------------"),
			container.NewGridWithColumns(2, container.NewBorder(nil, nil, nil, container.NewHBox(widget.NewLabel("ms")), u.st.boundTimeEntry), u.st.boundTimeSlider),
		)

		moveSettings = container.NewBorder(
			container.NewVBox(
				container.NewGridWithColumns(2,
					container.NewBorder(nil, nil, container.NewHBox(widget.NewLabel("X:")), nil, u.st.boundMoveXEntry), u.st.boundMoveXSlider,
					container.NewBorder(nil, nil, container.NewHBox(widget.NewLabel("Y:")), nil, u.st.boundMoveYEntry), u.st.boundMoveYSlider,
					container.NewHBox(layout.NewSpacer(), widget.NewLabel("Spot:")), u.st.boundSpotSelect,
				),
			), nil, nil, nil) //, mouseMoveDisplayContainer)
		clickSettings = container.NewVBox(
			container.NewHBox(layout.NewSpacer(), widget.NewLabel("left"), u.st.boundButtonToggle, widget.NewLabel("right"), layout.NewSpacer()),
		)

		keySettings = container.NewVBox(
			container.NewHBox(layout.NewSpacer(), u.st.boundKeySelect, widget.NewLabel("up"), u.st.boundStateToggle, widget.NewLabel("down"), layout.NewSpacer()),
		)

		loopSettings = container.NewVBox(
			container.NewGridWithColumns(2, container.NewHBox(layout.NewSpacer(), widget.NewLabel("name:")), u.st.boundLoopNameEntry),
			container.NewGridWithColumns(2, container.NewHBox(layout.NewSpacer(), widget.NewLabel("loops:"), u.st.boundCountLabel), u.st.boundCountSlider),
		)

		imageSearchSettings = container.NewBorder(
			container.NewVBox(
				container.NewGridWithColumns(2, container.NewHBox(widget.NewLabel("name:")), u.st.boundImageSearchNameEntry),
				container.NewGridWithColumns(2, container.NewHBox(widget.NewLabel("search area:")), u.st.boundImageSearchAreaSelect),
				container.NewGridWithColumns(3, container.NewHBox(widget.NewLabel("screen split cols:")), u.st.boundXSplitSlider, u.st.boundXSplitEntry),
			), nil, nil, nil,
			//			u.st.boundImageSearchTargetsTree,
			u.createItemsCheckTree(),
		)

		ocrSettings = container.NewBorder(
			container.NewGridWithColumns(1,
				container.NewBorder(nil, nil, container.NewHBox(widget.NewLabel("Text Target:")), nil, u.st.boundOCRTargetEntry),
				container.NewBorder(nil, nil, container.NewHBox(widget.NewLabel("Search Area:")), nil, u.st.boundOCRSearchBoxSelect),
			), nil, nil, nil)
	)

	u.st.tabs.Append(container.NewTabItem("Wait", waitSettings))
	u.st.tabs.Append(container.NewTabItem("Move", moveSettings))
	u.st.tabs.Append(container.NewTabItem("Click", clickSettings))
	u.st.tabs.Append(container.NewTabItem("Key", keySettings))
	u.st.tabs.Append(container.NewTabItem("Loop", loopSettings))
	u.st.tabs.Append(container.NewTabItem("Image", imageSearchSettings))
	u.st.tabs.Append(container.NewTabItem("OCR", ocrSettings))
}

func (u *Ui) bindVariables() {

	// u.sel.boundMacroName = binding.BindString(&macroName)
	u.st.boundGlobalDelay = binding.BindInt(&globalDelay)
	u.st.boundGlobalDelayEntry = widget.NewEntryWithData(binding.IntToString(u.st.boundGlobalDelay))
	u.st.boundGlobalDelay.AddListener(binding.NewDataListener(func() {
		t, err := u.GetMacroTabMacroTree()
		if err != nil {
			log.Println(err)
			return
		}

		t.Macro.GlobalDelay = globalDelay
		robotgo.MouseSleep = globalDelay
		robotgo.KeySleep = globalDelay
	}))
	u.st.boundTime = binding.BindInt(&time)
	u.st.boundTimeEntry = widget.NewEntryWithData(binding.IntToString(u.st.boundTime))
	u.st.boundTimeSlider = widget.NewSliderWithData(0.0, 250.0, binding.IntToFloat(u.st.boundTime))
	u.st.boundTime.AddListener(binding.NewDataListener(func() {
		t, err := u.GetMacroTabMacroTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.Wait); ok {
			n.Time = time
			t.Tree.Refresh()
		}
	}))
	u.st.boundMoveX = binding.BindInt(&moveX)
	u.st.boundMoveY = binding.BindInt(&moveY)
	u.st.boundMoveXSlider = widget.NewSliderWithData(-1.0, float64(config.MonitorWidth), binding.IntToFloat(u.st.boundMoveX))
	u.st.boundMoveYSlider = widget.NewSliderWithData(-1.0, float64(config.MonitorHeight), binding.IntToFloat(u.st.boundMoveY))
	u.st.boundMoveXEntry = widget.NewEntryWithData(binding.IntToString(u.st.boundMoveX))
	u.st.boundMoveYEntry = widget.NewEntryWithData(binding.IntToString(u.st.boundMoveY))
	u.st.boundSpot = binding.BindString(&spot)
	u.st.boundSpotSelect = widget.NewSelect(programs.GetPrograms().GetProgram(config.DarkAndDarker).Coordinates["2560x1440"].GetPointsAsStringSlice(), func(s string) {
		u.st.boundSpot.Set(s)
		u.st.boundMoveX.Set(programs.CurrentProgram().Coordinates["2560x1440"].GetPoint(s).X)
		u.st.boundMoveY.Set(programs.CurrentProgram().Coordinates["2560x1440"].GetPoint(s).Y)
	})
	u.st.boundMoveX.AddListener(binding.NewDataListener(func() {
		t, err := u.GetMacroTabMacroTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.Move); ok {
			n.X = moveX
			t.Tree.Refresh()
		}
	}))
	u.st.boundMoveY.AddListener(binding.NewDataListener(func() {
		t, err := u.GetMacroTabMacroTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.Move); ok {
			n.Y = moveY
			t.Tree.Refresh()
		}
	}))
	u.st.boundButton = binding.BindBool(&button)
	u.st.boundButtonToggle = custom_widgets.NewToggleWithData(u.st.boundButton)
	u.st.boundButton.AddListener(binding.NewDataListener(func() {
		t, err := u.GetMacroTabMacroTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.Click); ok {
			n.Button = actions.LeftOrRight(button)
			t.Tree.Refresh()
		}
	}))
	u.st.boundKey = binding.BindString(&key)
	u.st.boundKeySelect = widget.NewSelect([]string{"ctrl", "alt", "shift"}, func(s string) { u.st.boundKey.Set(s) })
	u.st.boundState = binding.BindBool(&state)
	u.st.boundStateToggle = custom_widgets.NewToggleWithData(u.st.boundState)
	u.st.boundKey.AddListener(binding.NewDataListener(func() {
		t, err := u.GetMacroTabMacroTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.Key); ok {
			n.Key = key
			t.Tree.Refresh()
		}
	}))
	u.st.boundState.AddListener(binding.NewDataListener(func() {
		t, err := u.GetMacroTabMacroTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.Key); ok {
			n.State = actions.UpOrDown(state)
			t.Tree.Refresh()
		}
	}))
	u.st.boundLoopName = binding.BindString(&loopName)
	u.st.boundCount = binding.BindInt(&count)
	u.st.boundLoopNameEntry = widget.NewEntryWithData(u.st.boundLoopName)
	u.st.boundCountSlider = widget.NewSliderWithData(1, 10, binding.IntToFloat(u.st.boundCount))
	u.st.boundCountLabel = widget.NewLabelWithData(binding.IntToString(u.st.boundCount))
	u.st.boundLoopName.AddListener(binding.NewDataListener(func() {
		t, err := u.GetMacroTabMacroTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.Loop); ok {
			n.Name = loopName
			t.Tree.Refresh()
		}
	}))
	u.st.boundCount.AddListener(binding.NewDataListener(func() {
		t, err := u.GetMacroTabMacroTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.Loop); ok {
			n.Count = count
			t.Tree.Refresh()
		}
	}))
	u.st.boundImageSearchName = binding.BindString(&imageSearchName)
	u.st.boundImageSearchArea = binding.BindString(&searchArea)
	u.st.boundXSplit = binding.BindInt(&xSplit)
	u.st.boundYSplit = binding.BindInt(&ySplit)
	u.st.boundImageSearchNameEntry = widget.NewEntryWithData(u.st.boundImageSearchName)
	u.st.boundImageSearchAreaSelect = widget.NewSelect(programs.CurrentProgramAndScreenSizeCoordinates().GetSearchAreasAsStringSlice(), func(s string) { u.st.boundImageSearchArea.Set(s) })

	u.st.boundXSplitSlider = widget.NewSliderWithData(0, 50, binding.IntToFloat(u.st.boundXSplit))
	u.st.boundXSplitEntry = widget.NewEntryWithData(binding.IntToString(u.st.boundXSplit))
	u.st.boundImageSearchName.AddListener(binding.NewDataListener(func() {
		t, err := u.GetMacroTabMacroTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.ImageSearch); ok {
			n.Name = imageSearchName
			t.Tree.Refresh()
		}
	}))
	u.st.boundImageSearchArea.AddListener(binding.NewDataListener(func() {
		t, err := u.GetMacroTabMacroTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.ImageSearch); ok {
			n.SearchArea = programs.CurrentProgramAndScreenSizeCoordinates().GetSearchArea(searchArea)
			t.Tree.Refresh()
		}
	}))
	u.st.boundOCRSearchBox = binding.BindString(&ocrSearchBox)
	u.st.boundOCRTarget = binding.BindString(&ocrTarget)
	u.st.boundOCRSearchBoxSelect = widget.NewSelect(programs.CurrentProgramAndScreenSizeCoordinates().GetSearchAreasAsStringSlice(), func(s string) { u.st.boundOCRSearchBox.Set(s) })
	u.st.boundOCRTargetEntry = widget.NewEntryWithData(u.st.boundOCRTarget)

}
