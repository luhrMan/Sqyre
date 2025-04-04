package ui

import (
	"Squire/internal/assets"
	"Squire/internal/config"
	"Squire/internal/programs"
	"Squire/internal/programs/actions"
	"Squire/ui/custom_widgets"
	"errors"
	"log"
	"sort"

	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
)

// action settings
var (
	macroList          []string
	macroName          string
	macroHotkey        []string
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
	itemsBoolList      = assets.Items.GetItemsMapAsBool()
	imageSearchTargets []string
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
			// widget.NewLabel("------------------------------------------------------------------------------------"),
			container.NewGridWithColumns(2, container.NewBorder(nil, nil, nil, container.NewHBox(widget.NewLabel("ms")), u.at.boundTimeEntry), u.at.boundTimeSlider),
		)

		moveSettings = container.NewBorder(
			container.NewVBox(
				container.NewGridWithColumns(2,
					container.NewBorder(nil, nil, container.NewHBox(widget.NewLabel("X:")), nil, u.at.boundMoveXEntry), u.at.boundMoveXSlider,
					container.NewBorder(nil, nil, container.NewHBox(widget.NewLabel("Y:")), nil, u.at.boundMoveYEntry), u.at.boundMoveYSlider,
					container.NewHBox(layout.NewSpacer(), widget.NewLabel("Spot:")), u.at.boundSpotSelect,
				),
			), nil, nil, nil) //, mouseMoveDisplayContainer)
		clickSettings = container.NewVBox(
			container.NewHBox(layout.NewSpacer(), widget.NewLabel("left"), u.at.boundButtonToggle, widget.NewLabel("right"), layout.NewSpacer()),
		)

		keySettings = container.NewVBox(
			container.NewHBox(layout.NewSpacer(), u.at.boundKeySelect, widget.NewLabel("up"), u.at.boundStateToggle, widget.NewLabel("down"), layout.NewSpacer()),
		)

		loopSettings = container.NewVBox(
			container.NewGridWithColumns(2, container.NewHBox(layout.NewSpacer(), widget.NewLabel("name:")), u.at.boundLoopNameEntry),
			container.NewGridWithColumns(2, container.NewHBox(layout.NewSpacer(), widget.NewLabel("loops:"), u.at.boundCountLabel), u.at.boundCountSlider),
		)

		imageSearchSettings = container.NewBorder(
			container.NewVBox(
				container.NewGridWithColumns(2, container.NewHBox(widget.NewLabel("name:")), u.at.boundImageSearchNameEntry),
				container.NewGridWithColumns(2, container.NewHBox(widget.NewLabel("search area:")), u.at.boundImageSearchAreaSelect),
				container.NewGridWithColumns(3, container.NewHBox(widget.NewLabel("screen split cols:")), u.at.boundXSplitSlider, u.at.boundXSplitEntry),
			), nil, nil, nil,
			// container.NewBorder(nil, nil, nil, nil,),
			//			u.st.boundImageSearchTargetsTree,
			u.createItemsCheckTree(),
		)

		ocrSettings = container.NewBorder(
			container.NewGridWithColumns(1,
				container.NewBorder(nil, nil, container.NewHBox(widget.NewLabel("Text Target:")), nil, u.at.boundOCRTargetEntry),
				container.NewBorder(nil, nil, container.NewHBox(widget.NewLabel("Search Area:")), nil, u.at.boundOCRSearchAreaSelect),
			), nil, nil, nil)
	)

	u.at.Append(container.NewTabItem("Wait", waitSettings))
	u.at.Append(container.NewTabItem("Move", moveSettings))
	u.at.Append(container.NewTabItem("Click", clickSettings))
	u.at.Append(container.NewTabItem("Key", keySettings))
	u.at.Append(container.NewTabItem("Loop", loopSettings))
	u.at.Append(container.NewTabItem("Image", imageSearchSettings))
	u.at.Append(container.NewTabItem("OCR", ocrSettings))
}

func (u *Ui) bindVariables() {
	boundLocX = binding.BindInt(&locX)
	boundLocY = binding.BindInt(&locY)
	boundLocXLabel = widget.NewLabelWithData(binding.IntToString(boundLocX))
	boundLocYLabel = widget.NewLabelWithData(binding.IntToString(boundLocY))

	boundMacro := binding.NewUntyped()
	boundMacro.Set(u.p.Macros[0])
	u.ms.boundMacroList = binding.BindStringList(&macroList)
	for _, m := range u.p.Macros {
		u.ms.boundMacroList.Append(m.Name)
	}
	u.ms.boundMacroList.AddListener(binding.NewDataListener(func() {
		ml, err := u.ms.boundMacroList.Get()
		if err != nil {
			log.Println(err)
			return
		}
		sort.Strings(ml)
	}))
	u.ms.boundMacroName = binding.BindString(&macroName)
	u.ms.boundMacroNameEntry = widget.NewEntryWithData(u.ms.boundMacroName)
	u.ms.boundMacroNameEntry.OnSubmitted = func(string) {
		t, err := u.GetMacroTabMacroTree()
		if err != nil {
			log.Println(err)
			return
		}
		for _, m := range u.p.Macros {
			if m.Name == macroName {
				dialog.ShowError(errors.New("macro name already exists"), u.win)
				return
			}
		}
		delete(u.mtMap, t.Macro.Name)
		u.ms.boundMacroList.Remove(t.Macro.Name)
		u.SetMacroTreeMapKeyValue(macroName, t)
		// u.mtMap[macroName] = t
		t.Macro.Name = macroName
		u.dt.Selected().Text = macroName
		u.ms.boundMacroList.Append(macroName)

		u.dt.Refresh()
	}
	macroHotkey = []string{"1", "2", "3"}
	u.ms.boundMacroHotkey = binding.BindStringList(&macroHotkey)
	u.ms.macroHotkeySelect1 = &widget.Select{Options: []string{"ctrl"}}
	u.ms.macroHotkeySelect2 = &widget.Select{Options: []string{"", "shift"}}
	u.ms.macroHotkeySelect3 = &widget.Select{Options: []string{"1", "2", "3", "4", "5"}}

	u.ms.macroHotkeySelect1.SetSelectedIndex(0)

	u.ms.boundGlobalDelay = binding.BindInt(&globalDelay)
	u.ms.boundGlobalDelayEntry = widget.NewEntryWithData(binding.IntToString(u.ms.boundGlobalDelay))
	u.ms.boundGlobalDelay.AddListener(binding.NewDataListener(func() {
		t, err := u.GetMacroTabMacroTree()
		if err != nil {
			log.Println(err)
			return
		}

		t.Macro.GlobalDelay = globalDelay
		robotgo.MouseSleep = globalDelay
		robotgo.KeySleep = globalDelay
	}))
	u.at.boundTime = binding.BindInt(&time)
	u.at.boundTimeEntry = widget.NewEntryWithData(binding.IntToString(u.at.boundTime))
	u.at.boundTimeSlider = widget.NewSliderWithData(0.0, 250.0, binding.IntToFloat(u.at.boundTime))
	u.at.boundTime.AddListener(binding.NewDataListener(func() {
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
	u.at.boundMoveX = binding.BindInt(&moveX)
	u.at.boundMoveY = binding.BindInt(&moveY)
	u.at.boundMoveXSlider = widget.NewSliderWithData(-1.0, float64(config.MonitorWidth), binding.IntToFloat(u.at.boundMoveX))
	u.at.boundMoveYSlider = widget.NewSliderWithData(-1.0, float64(config.MonitorHeight), binding.IntToFloat(u.at.boundMoveY))
	u.at.boundMoveXEntry = widget.NewEntryWithData(binding.IntToString(u.at.boundMoveX))
	u.at.boundMoveYEntry = widget.NewEntryWithData(binding.IntToString(u.at.boundMoveY))
	u.at.boundSpot = binding.BindString(&spot)
	u.at.boundSpotSelect = widget.NewSelect(programs.GetPrograms().GetProgram(config.DarkAndDarker).Coordinates["2560x1440"].GetPointsAsStringSlice(), func(s string) {
		u.at.boundSpot.Set(s)
		u.at.boundMoveX.Set(programs.CurrentProgram().Coordinates["2560x1440"].GetPoint(s).X)
		u.at.boundMoveY.Set(programs.CurrentProgram().Coordinates["2560x1440"].GetPoint(s).Y)
	})
	u.at.boundMoveX.AddListener(binding.NewDataListener(func() {
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
	u.at.boundMoveY.AddListener(binding.NewDataListener(func() {
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
	u.at.boundButton = binding.BindBool(&button)
	u.at.boundButtonToggle = custom_widgets.NewToggleWithData(u.at.boundButton)
	u.at.boundButton.AddListener(binding.NewDataListener(func() {
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
	u.at.boundKey = binding.BindString(&key)
	u.at.boundKeySelect = widget.NewSelect([]string{"ctrl", "alt", "shift"}, func(s string) { u.at.boundKey.Set(s) })
	u.at.boundState = binding.BindBool(&state)
	u.at.boundStateToggle = custom_widgets.NewToggleWithData(u.at.boundState)
	u.at.boundKey.AddListener(binding.NewDataListener(func() {
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
	u.at.boundState.AddListener(binding.NewDataListener(func() {
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
	u.at.boundLoopName = binding.BindString(&loopName)
	u.at.boundCount = binding.BindInt(&count)
	u.at.boundLoopNameEntry = widget.NewEntryWithData(u.at.boundLoopName)
	u.at.boundCountSlider = widget.NewSliderWithData(1, 10, binding.IntToFloat(u.at.boundCount))
	u.at.boundCountLabel = widget.NewLabelWithData(binding.IntToString(u.at.boundCount))
	u.at.boundLoopName.AddListener(binding.NewDataListener(func() {
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
	u.at.boundCount.AddListener(binding.NewDataListener(func() {
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
	u.at.boundImageSearchName = binding.BindString(&imageSearchName)
	u.at.boundImageSearchArea = binding.BindString(&searchArea)
	u.at.boundImageSearchTargets = binding.BindStringList(&imageSearchTargets)
	u.at.boundImageSearchTargets.AddListener(binding.NewDataListener(func() {
		t, err := u.GetMacroTabMacroTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.ImageSearch); ok {
			n.Targets = imageSearchTargets
			t.Tree.Refresh()
		}
	}))
	u.at.boundXSplit = binding.BindInt(&xSplit)
	u.at.boundYSplit = binding.BindInt(&ySplit)
	u.at.boundImageSearchNameEntry = widget.NewEntryWithData(u.at.boundImageSearchName)
	u.at.boundImageSearchAreaSelect = widget.NewSelect(programs.CurrentProgramAndScreenSizeCoordinates().GetSearchAreasAsStringSlice(),
		func(s string) { u.at.boundImageSearchArea.Set(s) })

	u.at.boundXSplitSlider = widget.NewSliderWithData(0, 50, binding.IntToFloat(u.at.boundXSplit))
	u.at.boundXSplitEntry = widget.NewEntryWithData(binding.IntToString(u.at.boundXSplit))
	u.at.boundImageSearchName.AddListener(binding.NewDataListener(func() {
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
	u.at.boundImageSearchArea.AddListener(binding.NewDataListener(func() {
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
	u.at.boundOCRSearchArea = binding.BindString(&ocrSearchBox)
	u.at.boundOCRTarget = binding.BindString(&ocrTarget)
	u.at.boundOCRSearchAreaSelect = widget.NewSelect(programs.CurrentProgramAndScreenSizeCoordinates().GetSearchAreasAsStringSlice(), func(s string) { u.at.boundOCRSearchArea.Set(s) })
	u.at.boundOCRTargetEntry = widget.NewEntryWithData(u.at.boundOCRTarget)
	u.at.boundOCRSearchArea.AddListener(binding.NewDataListener(func() {
		t, err := u.GetMacroTabMacroTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.Ocr); ok {
			n.SearchArea = programs.CurrentProgramAndScreenSizeCoordinates().GetSearchArea(searchArea)
			t.Tree.Refresh()
		}
	}))
	u.at.boundOCRTarget.AddListener(binding.NewDataListener(func() {
		t, err := u.GetMacroTabMacroTree()
		if err != nil {
			log.Println(err)
			return
		}

		if n, ok := t.Macro.Root.GetAction(selectedTreeItem).(*actions.Ocr); ok {
			n.Target = ocrTarget
			t.Tree.Refresh()
		}
	}))

}
