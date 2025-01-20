package main

import (
	"Squire/internal"
	"Squire/internal/actions"
	"Squire/internal/gui/custom_widgets"
	"Squire/internal/structs"
	"Squire/internal/utils"
	"fmt"
	"fyne.io/fyne/v2/canvas"
	"golang.org/x/image/colornames"
	"log"
	"os"
	"slices"
	"strconv"
	"strings"

	"fyne.io/fyne/v2/data/binding"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/layout"
	_ "fyne.io/x/fyne/widget"
	xwidget "fyne.io/x/fyne/widget"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	widget "fyne.io/fyne/v2/widget"
	"github.com/go-vgo/robotgo"
)

type ui struct {
	win fyne.Window

	mm  map[string]*macro
	sel *xwidget.CompletionEntry

	dt *container.DocTabs
	st *settingsTabs
}

type settingsTabs struct {
	tabs                  *container.AppTabs
	boundGlobalDelay      binding.Int
	boundGlobalDelayEntry *widget.Entry
	waitTab
	moveTab
	keyTab
	loopTab
	imageSearchTab
	ocrTab
}

type waitTab struct {
	boundTime binding.Int

	boundTimeSlider *widget.Slider
	boundTimeEntry  *widget.Entry
}

type moveTab struct {
	boundMoveX binding.Int
	boundMoveY binding.Int
	boundSpot  binding.String

	boundMoveXSlider *widget.Slider
	boundMoveYSlider *widget.Slider
	boundMoveXEntry  *widget.Entry
	boundMoveYEntry  *widget.Entry
	boundSpotSelect  *widget.Select
}

type keyTab struct {
	boundButton binding.Bool
	boundKey    binding.String
	boundState  binding.Bool

	boundButtonToggle *custom_widgets.Toggle
	boundKeySelect    *widget.Select
	boundStateToggle  *custom_widgets.Toggle
}

type loopTab struct {
	boundLoopName binding.String
	boundCount    binding.Int

	boundLoopNameEntry *widget.Entry
	boundCountSlider   *widget.Slider
	boundCountLabel    *widget.Label
}

type imageSearchTab struct {
	boundImageSearchName binding.String
	boundImageSearchArea binding.String
	boundXSplit          binding.Int
	boundYSplit          binding.Int

	boundImageSearchNameEntry  *widget.Entry
	boundImageSearchAreaSelect *widget.Select
	boundXSplitSlider          *widget.Slider
	boundXSplitEntry           *widget.Entry
}

type ocrTab struct {
	boundOCRTarget    binding.String
	boundOCRSearchBox binding.String

	boundOCRTargetEntry     *widget.Entry
	boundOCRSearchBoxSelect *widget.Select
}

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
	imageSearchTargets = internal.Items.GetItemsMapAsBool()
	ocrTarget          string
	ocrSearchBox       string
)

func (u *ui) LoadMainContent() *fyne.Container {
	internal.CreateItemMaps()
	u.createDocTabs()
	u.addMacroDocTab("Currency Testing")
	u.dt.SelectIndex(0)
	u.createSelect()
	u.dt.OnClosed = func(ti *container.TabItem) {
		delete(u.mm, ti.Text)
	}
	u.actionSettingsTabs()
	u.win.SetMainMenu(u.createMainMenu())

	macroLayout := container.NewBorder(
		container.NewGridWithColumns(2,
			container.NewHBox(
				u.createMacroToolbar(),
				layout.NewSpacer(),
				widget.NewLabel("Macro Name:"),
			),
			container.NewBorder(nil, nil, nil, widget.NewButtonWithIcon("", theme.LoginIcon(), func() { u.addMacroDocTab(u.sel.Text) }), u.sel),
		),
		nil,
		widget.NewSeparator(),
		nil,
		u.dt,
	)
	mainLayout := container.NewBorder(nil, nil, u.st.tabs, nil, macroLayout)

	return mainLayout
}

func (u *ui) addMacroDocTab(name string) {
	if _, ok := u.mm[name]; ok {
		return
	}
	m := &macro{}
	m.createTree()
	err := m.loadTreeFromJsonFile(name + ".json")
	if err != nil {
		dialog.ShowError(err, u.win)
		return
	}
	u.mm[name] = m

	t := container.NewTabItem(name, m.tree)
	u.dt.Append(t)
	u.dt.Select(t)
	u.updateTreeOnselect()
}

func (u *ui) createSelect() {
	var macroList []string

	getMacroList := func() []string {
		var list []string
		files, err := os.ReadDir("./internal/saved-macros")
		if err != nil {
			log.Fatal(err)
		}
		for _, f := range files {
			list = append(list, strings.TrimSuffix(f.Name(), ".json"))
		}
		return list
	}

	macroList = getMacroList()
	u.sel = xwidget.NewCompletionEntry(macroList)
	u.sel.ActionItem = widget.NewButtonWithIcon("", theme.ViewRefreshIcon(), func() { macroList = getMacroList() })
	u.sel.OnSubmitted = func(s string) { u.addMacroDocTab(s) }
	u.sel.OnChanged = func(s string) {
		var matches []string
		userPrefix := strings.ToLower(s)
		for _, listStr := range macroList {
			if len(listStr) < len(s) {
				continue
			}
			listPrefix := strings.ToLower(listStr[:len(s)])
			if userPrefix == listPrefix {
				matches = append(matches, listStr)
			}
		}
		u.sel.SetOptions(matches)
		u.sel.ShowCompletion()
	}
}
func (u *ui) bindVariables() {
	// ct.boundMacroName = binding.BindString(&macroName)
	u.st.boundGlobalDelay = binding.BindInt(&globalDelay)
	u.st.boundGlobalDelay.AddListener(binding.NewDataListener(func() { robotgo.MouseSleep = globalDelay; robotgo.KeySleep = globalDelay }))
	u.st.boundGlobalDelayEntry = widget.NewEntryWithData(binding.IntToString(u.st.boundGlobalDelay))
	u.st.boundTime = binding.BindInt(&time)
	u.st.boundTimeEntry = widget.NewEntryWithData(binding.IntToString(u.st.boundTime))
	u.st.boundTimeSlider = widget.NewSliderWithData(0.0, 250.0, binding.IntToFloat(u.st.boundTime))
	u.st.boundTime.AddListener(binding.NewDataListener(func() {
		if n, ok := u.getCurrentTabMacro().findNode(u.getCurrentTabMacro().root, selectedTreeItem).(*actions.Wait); ok {
			n.Time = time
			u.getCurrentTabMacro().tree.Refresh()
		}
	}))
	u.st.boundMoveX = binding.BindInt(&moveX)
	u.st.boundMoveY = binding.BindInt(&moveY)
	u.st.boundMoveXSlider = widget.NewSliderWithData(-1.0, float64(utils.MonitorWidth), binding.IntToFloat(u.st.boundMoveX))
	u.st.boundMoveYSlider = widget.NewSliderWithData(-1.0, float64(utils.MonitorHeight), binding.IntToFloat(u.st.boundMoveY))
	u.st.boundMoveXEntry = widget.NewEntryWithData(binding.IntToString(u.st.boundMoveX))
	u.st.boundMoveYEntry = widget.NewEntryWithData(binding.IntToString(u.st.boundMoveY))
	u.st.boundSpot = binding.BindString(&spot)
	u.st.boundSpotSelect = widget.NewSelect(*structs.GetSpotMapKeys(*structs.GetSpotMap()), func(s string) {
		u.st.boundSpot.Set(s)
		u.st.boundMoveX.Set(structs.GetSpot(s).X)
		u.st.boundMoveY.Set(structs.GetSpot(s).Y)
	})
	u.st.boundMoveX.AddListener(binding.NewDataListener(func() {
		if n, ok := u.getCurrentTabMacro().findNode(u.getCurrentTabMacro().root, selectedTreeItem).(*actions.Move); ok {
			n.X = moveX
			u.getCurrentTabMacro().tree.Refresh()
		}
	}))
	u.st.boundMoveY.AddListener(binding.NewDataListener(func() {
		if n, ok := u.getCurrentTabMacro().findNode(u.getCurrentTabMacro().root, selectedTreeItem).(*actions.Move); ok {
			n.Y = moveY
			u.getCurrentTabMacro().tree.Refresh()
		}
	}))
	u.st.boundButton = binding.BindBool(&button)
	u.st.boundButtonToggle = custom_widgets.NewToggleWithData(u.st.boundButton)
	u.st.boundButton.AddListener(binding.NewDataListener(func() {
		if n, ok := u.getCurrentTabMacro().findNode(u.getCurrentTabMacro().root, selectedTreeItem).(*actions.Click); ok {
			if button {
				n.Button = "right"
			} else {
				n.Button = "left"
			}
			u.getCurrentTabMacro().tree.Refresh()
		}
	}))
	u.st.boundKey = binding.BindString(&key)
	u.st.boundKeySelect = widget.NewSelect([]string{"ctrl", "alt", "shift"}, func(s string) { u.st.boundKey.Set(s) })
	u.st.boundState = binding.BindBool(&state)
	u.st.boundStateToggle = custom_widgets.NewToggleWithData(u.st.boundState)
	u.st.boundKey.AddListener(binding.NewDataListener(func() {
		if n, ok := u.getCurrentTabMacro().findNode(u.getCurrentTabMacro().root, selectedTreeItem).(*actions.Key); ok {
			n.Key = key
			u.getCurrentTabMacro().tree.Refresh()
		}
	}))
	u.st.boundState.AddListener(binding.NewDataListener(func() {
		if n, ok := u.getCurrentTabMacro().findNode(u.getCurrentTabMacro().root, selectedTreeItem).(*actions.Key); ok {
			if state {
				n.State = "Up"
			} else {
				n.State = "Down"
			}
			u.getCurrentTabMacro().tree.Refresh()
		}
	}))
	u.st.boundLoopName = binding.BindString(&loopName)
	u.st.boundCount = binding.BindInt(&count)
	u.st.boundLoopNameEntry = widget.NewEntryWithData(u.st.boundLoopName)
	u.st.boundCountSlider = widget.NewSliderWithData(1, 10, binding.IntToFloat(u.st.boundCount))
	u.st.boundCountLabel = widget.NewLabelWithData(binding.IntToString(u.st.boundCount))
	u.st.boundLoopName.AddListener(binding.NewDataListener(func() {
		if n, ok := u.getCurrentTabMacro().findNode(u.getCurrentTabMacro().root, selectedTreeItem).(*actions.Loop); ok {
			n.Name = loopName
			u.getCurrentTabMacro().tree.Refresh()
		}
	}))
	u.st.boundCount.AddListener(binding.NewDataListener(func() {
		if n, ok := u.getCurrentTabMacro().findNode(u.getCurrentTabMacro().root, selectedTreeItem).(*actions.Loop); ok {
			n.Count = count
			u.getCurrentTabMacro().tree.Refresh()
		}
	}))
	u.st.boundImageSearchName = binding.BindString(&imageSearchName)
	u.st.boundImageSearchArea = binding.BindString(&searchArea)
	u.st.boundXSplit = binding.BindInt(&xSplit)
	u.st.boundYSplit = binding.BindInt(&ySplit)
	u.st.boundImageSearchNameEntry = widget.NewEntryWithData(u.st.boundImageSearchName)
	u.st.boundImageSearchAreaSelect = widget.NewSelect(*structs.GetSearchBoxMapKeys(*structs.GetSearchBoxMap()), func(s string) { u.st.boundImageSearchArea.Set(s) })

	u.st.boundXSplitSlider = widget.NewSliderWithData(0, 50, binding.IntToFloat(u.st.boundXSplit))
	u.st.boundXSplitEntry = widget.NewEntryWithData(binding.IntToString(u.st.boundXSplit))
	u.st.boundImageSearchName.AddListener(binding.NewDataListener(func() {
		if n, ok := u.getCurrentTabMacro().findNode(u.getCurrentTabMacro().root, selectedTreeItem).(*actions.ImageSearch); ok {
			n.Name = imageSearchName
			u.getCurrentTabMacro().tree.Refresh()
		}
	}))
	u.st.boundImageSearchArea.AddListener(binding.NewDataListener(func() {
		if n, ok := u.getCurrentTabMacro().findNode(u.getCurrentTabMacro().root, selectedTreeItem).(*actions.ImageSearch); ok {
			n.SearchBox = *structs.GetSearchBox(searchArea)
			u.getCurrentTabMacro().tree.Refresh()
		}
	}))
	u.st.boundOCRSearchBox = binding.BindString(&ocrSearchBox)
	u.st.boundOCRTarget = binding.BindString(&ocrTarget)
	u.st.boundOCRSearchBoxSelect = widget.NewSelect(*structs.GetSearchBoxMapKeys(*structs.GetSearchBoxMap()), func(s string) { u.st.boundOCRSearchBox.Set(s) })
	u.st.boundOCRTargetEntry = widget.NewEntryWithData(u.st.boundOCRTarget)

}

func (u *ui) createDocTabs() {
	u.dt = container.NewDocTabs()
}

func (u *ui) actionSettingsTabs() {
	u.bindVariables()
	//	screen := robotgo.CaptureScreen(0, 0, 2560, 1440)
	//	defer robotgo.FreeBitmap(screen)
	//		mouseMoveDisplay := canvas.NewImageFromImage(robotgo.ToImage(screen))

	mouseMoveDisplayImage := canvas.NewImageFromFile("./internal/resources/images/full-screen.png")
	mouseMoveDisplayImage.FillMode = canvas.ImageFillStretch
	vLine := canvas.NewLine(colornames.Red)
	hLine := canvas.NewLine(colornames.Red)
	vLine.StrokeWidth = 2
	hLine.StrokeWidth = 2
	mouseMoveDisplayContainer := container.NewBorder(nil, nil, nil, nil, mouseMoveDisplayImage, vLine, hLine)
	//	vLine.Position1 = mouseMoveDisplayContainer.Position()
	x, _ := u.st.boundMoveX.Get()
	vLine.Position1.X = float32(x)
	vLine.Position1.Y = 0
	vLine.Position2.X = float32(x)
	vLine.Position2.Y = mouseMoveDisplayImage.Size().Height
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
			), nil, nil, nil, mouseMoveDisplayContainer)
		clickSettings = container.NewVBox(
			container.NewHBox(layout.NewSpacer(), widget.NewLabel("left"), u.st.boundButtonToggle, widget.NewLabel("right"), layout.NewSpacer()),
		)
		keySettings = container.NewVBox(
			container.NewHBox(layout.NewSpacer(), u.st.boundKeySelect, widget.NewLabel("down"), u.st.boundStateToggle, widget.NewLabel("up"), layout.NewSpacer()))
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

func (u *ui) createMacroToolbar() *widget.Toolbar {
	tb := widget.NewToolbar(
		widget.NewToolbarAction(theme.ContentAddIcon(), func() {
			switch u.st.tabs.Selected().Text {
			case "Wait":
				u.getCurrentTabMacro().addActionToTree(&actions.Wait{})
			case "Move":
				u.getCurrentTabMacro().addActionToTree(&actions.Move{})
			case "Click":
				u.getCurrentTabMacro().addActionToTree(&actions.Click{})
			case "Key":
				u.getCurrentTabMacro().addActionToTree(&actions.Key{})
			case "Loop":
				u.getCurrentTabMacro().addActionToTree(&actions.Loop{})
			case "Image":
				u.getCurrentTabMacro().addActionToTree(&actions.ImageSearch{})
			case "OCR":
				u.getCurrentTabMacro().addActionToTree(&actions.Ocr{})
			}
		}),
		widget.NewToolbarAction(theme.ViewRefreshIcon(), func() {
			node := u.getCurrentTabMacro().findNode(u.getCurrentTabMacro().root, selectedTreeItem)
			if selectedTreeItem == "" {
				log.Println("No node selected")
				return
			}
			og := node.String()
			switch node := node.(type) {
			//			case *actions.Wait:
			//				node.Time = time
			//			case *actions.Move:
			//				node.X = moveX
			//				node.Y = moveY
			//			case *actions.Click:
			//				if !button {
			//					node.Button = "left"
			//				} else {
			//					node.Button = "right"
			//				}
			//			case *actions.Key:
			//				node.Key = key
			//				if !state {
			//					node.State = "down"
			//				} else {
			//					node.State = "up"
			//				}
			//			case *actions.Loop:
			//				node.Name = loopName
			//				node.Count = count
			case *actions.ImageSearch:
				var t []string
				for i, item := range imageSearchTargets {
					if item {
						t = append(t, i)
					}
				}
				node.Name = imageSearchName
				node.SearchBox = *structs.GetSearchBox(searchArea)
				node.Targets = t
			}

			fmt.Printf("Updated node: %+v from '%v' to '%v' \n", node.GetUID(), og, node)

			u.getCurrentTabMacro().tree.Refresh()
		}),
		widget.NewToolbarSpacer(),
		widget.NewToolbarSeparator(),
		widget.NewToolbarAction(theme.RadioButtonIcon(), func() {
			u.getCurrentTabMacro().tree.UnselectAll()
			selectedTreeItem = ""
		}),
		widget.NewToolbarAction(theme.MoveDownIcon(), func() {
			u.getCurrentTabMacro().moveNodeDown(selectedTreeItem)
		}),
		widget.NewToolbarAction(theme.MoveUpIcon(), func() {
			u.getCurrentTabMacro().moveNodeUp(selectedTreeItem)
		}),
		widget.NewToolbarSeparator(),
		widget.NewToolbarSpacer(),
		widget.NewToolbarAction(theme.MediaPlayIcon(), func() {
			robotgo.ActiveName("Dark and Darker")
			u.getCurrentTabMacro().executeActionTree()
		}),
		widget.NewToolbarAction(theme.DocumentSaveIcon(), func() {
			save := func() {
				err := u.getCurrentTabMacro().saveTreeToJsonFile(u.sel.Text)
				if err != nil {
					dialog.ShowError(err, u.win)
					log.Printf("saveTreeToJsonFile(): %v", err)
				} else {
					dialog.ShowInformation("File Saved Successfully", u.sel.Text+".json"+"\nPlease refresh the list.", u.win)
				}
			}
			if slices.Contains(u.sel.Options, u.sel.Text) {
				dialog.ShowConfirm("Overwrite existing file", "Overwrite "+u.sel.Text+"?", func(b bool) {
					if !b {
						return
					}
					save()
				}, u.win)
			} else {
				save()
			}
		}),
	)
	return tb
}

func (u *ui) getCurrentTabMacro() *macro {
	return u.mm[u.dt.Selected().Text]
}

func (u *ui) createMainMenu() *fyne.MainMenu {
	basicActionsSubMenu := fyne.NewMenuItem("Basic Actions", nil)
	basicActionsSubMenu.ChildMenu = fyne.NewMenu("")
	advancedActionsSubMenu := fyne.NewMenuItem("Advanced Actions", nil)
	advancedActionsSubMenu.ChildMenu = fyne.NewMenu("")

	waitActionMenuItem := fyne.NewMenuItem("Wait", func() { u.getCurrentTabMacro().addActionToTree(&actions.Wait{}) })
	mouseMoveActionMenuItem := fyne.NewMenuItem("Mouse Move", func() { u.getCurrentTabMacro().addActionToTree(&actions.Move{}) })
	clickActionMenuItem := fyne.NewMenuItem("Click", func() { u.getCurrentTabMacro().addActionToTree(&actions.Click{}) })
	keyActionMenuItem := fyne.NewMenuItem("Key", func() { u.getCurrentTabMacro().addActionToTree(&actions.Key{}) })

	loopActionMenuItem := fyne.NewMenuItem("Loop", func() { u.getCurrentTabMacro().addActionToTree(&actions.Loop{}) })
	imageSearchActionMenuItem := fyne.NewMenuItem("Image Search", func() { u.getCurrentTabMacro().addActionToTree(&actions.ImageSearch{}) })
	ocrActionMenuItem := fyne.NewMenuItem("OCR", func() { u.getCurrentTabMacro().addActionToTree(&actions.Ocr{}) })

	// ocrActionMenuItem.Icon, _ = fyne.LoadResourceFromPath("./internal/resources/images/Squire.png")

	basicActionsSubMenu.ChildMenu.Items = append(basicActionsSubMenu.ChildMenu.Items,
		waitActionMenuItem,
		mouseMoveActionMenuItem,
		clickActionMenuItem,
		keyActionMenuItem,
	)

	advancedActionsSubMenu.ChildMenu.Items = append(advancedActionsSubMenu.ChildMenu.Items,
		loopActionMenuItem,
		imageSearchActionMenuItem,
		ocrActionMenuItem,
	)

	actionMenu := fyne.NewMenu("Add Action", basicActionsSubMenu, advancedActionsSubMenu)

	computerInfo := fyne.NewMenuItem("Computer info", func() {
		var str string
		w, h := robotgo.GetScreenSize()
		str = str + "Total Screen Size: " + strconv.Itoa(w) + "x" + strconv.Itoa(h) + "\n"
		for d := range robotgo.DisplaysNum() {
			_, _, mh, mw := robotgo.GetDisplayBounds(d)
			str = str + "Monitor " + strconv.Itoa(d+1) + " Size: " + strconv.Itoa(mh) + "x" + strconv.Itoa(mw) + "\n"
		}
		dialog.ShowInformation("Computer Information", str, u.win)
	})

	return fyne.NewMainMenu(fyne.NewMenu("Settings", computerInfo), actionMenu)
}
