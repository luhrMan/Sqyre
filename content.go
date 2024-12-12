package main

import (
	"Squire/internal"
	"Squire/internal/actions"
	"Squire/internal/gui/custom_widgets"
	"Squire/internal/structs"
	"Squire/internal/utils"
	"fmt"
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
	tabs *container.AppTabs

	boundTime            binding.Int
	boundMoveX           binding.Int
	boundMoveY           binding.Int
	boundSpot            binding.String
	boundButton          binding.Bool
	boundKey             binding.String
	boundState           binding.Bool
	boundLoopName        binding.String
	boundCount           binding.Float
	boundImageSearchName binding.String
	boundSearchArea      binding.String
}

// action settings
var (
	macroName        string
	globalDelay      int = 25
	selectedTreeItem     = ".1"

	//BASICS
	//wait
	time int
	//move
	moveX int
	moveY int
	spot  string
	//click
	button bool
	//key
	key   string
	state bool

	//ADVANCED
	//loop
	loopName string
	count    float64 = 1
	//image search
	imageSearchName    string
	searchArea         string
	imageSearchTargets = internal.Items.GetItemsMapAsBool()
	//ocr
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

	// searchAreaSelector.SetSelected(searchAreaSelector.Options[0])

	//        boundMacroNameEntry := widget.NewEntryWithData(ct.boundMacroName)

	// boundGlobalDelayEntry := widget.NewEntryWithData(binding.IntToString(ct.boundGlobalDelay))

	macroLayout := container.NewBorder(
		container.NewGridWithColumns(2,
			container.NewHBox(
				u.createMacroToolbar(),
				// widget.NewLabel("Global Delay:"),
				// boundGlobalDelayEntry,
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
	// ct.boundGlobalDelay = binding.BindInt(&globalDelay)

	u.st.boundTime = binding.BindInt(&time)
	u.st.boundMoveX = binding.BindInt(&moveX)
	u.st.boundMoveY = binding.BindInt(&moveY)
	u.st.boundSpot = binding.BindString(&spot)
	u.st.boundButton = binding.BindBool(&button)
	u.st.boundKey = binding.BindString(&key)
	u.st.boundState = binding.BindBool(&state)
	u.st.boundLoopName = binding.BindString(&loopName)
	u.st.boundCount = binding.BindFloat(&count)
	u.st.boundImageSearchName = binding.BindString(&imageSearchName)
	u.st.boundSearchArea = binding.BindString(&searchArea)
}

func (u *ui) createDocTabs() {
	u.dt = container.NewDocTabs()
}

// WIDGET LOCATIONS ARE HARD CODED IN THE TREE ONSELECTED CALLBACK. CAREFUL WITH CHANGES HERE
func (u *ui) actionSettingsTabs() {
	u.bindVariables()
	var (
		//BASICS
		//wait
		boundTimeSlider = widget.NewSliderWithData(0.0, 250.0, binding.IntToFloat(u.st.boundTime))
		boundTimeLabel  = widget.NewLabelWithData(binding.FloatToStringWithFormat(binding.IntToFloat(u.st.boundTime), "%0.0f"))
		//move
		// boundSpotSelect  = widget.NewSelect(*structs.GetSpotMapKeys(*structs.GetSpotMap()), func(s string) { boundSpot.Set(s) })
		boundMoveXSlider = widget.NewSliderWithData(-1.0, float64(utils.MonitorWidth), binding.IntToFloat(u.st.boundMoveX))
		boundMoveYSlider = widget.NewSliderWithData(-1.0, float64(utils.MonitorHeight), binding.IntToFloat(u.st.boundMoveY))
		boundMoveXLabel  = widget.NewLabelWithData(binding.FloatToStringWithFormat(binding.IntToFloat(u.st.boundMoveX), "%0.0f"))
		boundMoveYLabel  = widget.NewLabelWithData(binding.FloatToStringWithFormat(binding.IntToFloat(u.st.boundMoveY), "%0.0f"))
		//click
		boundButtonToggle = custom_widgets.NewToggleWithData(u.st.boundButton)
		//key
		boundKeySelect   = widget.NewSelect([]string{"ctrl", "alt", "shift"}, func(s string) { u.st.boundKey.Set(s) })
		boundStateToggle = custom_widgets.NewToggleWithData(u.st.boundState)

		//ADVANCED
		//loop
		boundLoopNameEntry = widget.NewEntryWithData(u.st.boundLoopName)
		boundCountSlider   = widget.NewSliderWithData(1, 10, u.st.boundCount)
		boundCountLabel    = widget.NewLabelWithData(binding.FloatToStringWithFormat(u.st.boundCount, "%0.0f"))
		//image search
		boundImageSearchNameEntry = widget.NewEntryWithData(u.st.boundImageSearchName)
		boundSearchAreaSelect     = widget.NewSelect(*structs.GetSearchBoxMapKeys(*structs.GetSearchBoxMap()), func(s string) { u.st.boundSearchArea.Set(s) })

		waitSettings = container.NewVBox(
			widget.NewLabel("------------------------------------------------------------------------------------"),
			container.NewGridWithColumns(2, container.NewHBox(layout.NewSpacer(), boundTimeLabel, widget.NewLabel("ms")), boundTimeSlider),
		)
		moveSettings = container.NewVBox(container.NewGridWithColumns(2,
			container.NewHBox(layout.NewSpacer(), widget.NewLabel("X:"), boundMoveXLabel), boundMoveXSlider,
			container.NewHBox(layout.NewSpacer(), widget.NewLabel("Y:"), boundMoveYLabel), boundMoveYSlider),
		)
		clickSettings = container.NewVBox(
			container.NewHBox(layout.NewSpacer(), widget.NewLabel("left"), boundButtonToggle, widget.NewLabel("right"), layout.NewSpacer()),
		)
		keySettings = container.NewVBox(
			container.NewHBox(layout.NewSpacer(), boundKeySelect, widget.NewLabel("down"), boundStateToggle, widget.NewLabel("up"), layout.NewSpacer()))
		loopSettings = container.NewVBox(
			container.NewGridWithColumns(2, container.NewHBox(layout.NewSpacer(), widget.NewLabel("name:")), boundLoopNameEntry),
			container.NewGridWithColumns(2, container.NewHBox(layout.NewSpacer(), widget.NewLabel("loops:"), boundCountLabel), boundCountSlider),
		)
		imageSearchSettings = container.NewBorder(
			container.NewVBox(
				container.NewGridWithColumns(2, container.NewHBox(layout.NewSpacer(), widget.NewLabel("name:")), boundImageSearchNameEntry),
				container.NewGridWithColumns(2, container.NewHBox(layout.NewSpacer(), widget.NewLabel("search area:")), boundSearchAreaSelect),
			), nil, nil, nil,
			u.createItemsCheckTree(),
		)

		ocrSettings = container.NewHBox(
			layout.NewSpacer(), layout.NewSpacer())
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
			case *actions.Wait:
				node.Time = time
			case *actions.Move:
				node.X = moveX
				node.Y = moveY
			case *actions.Click:
				if !button {
					node.Button = "left"
				} else {
					node.Button = "right"
				}
			case *actions.Key:
				node.Key = key
				if !state {
					node.State = "down"
				} else {
					node.State = "up"
				}
			case *actions.Loop:
				node.Name = loopName
				node.Count = int(count)
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
		w, h := robotgo.GetScreenSize()
		log.Println("Monitor 1 size")
		log.Println(robotgo.GetDisplayBounds(0))
		log.Println("Monitor 2 size")
		log.Println(robotgo.GetDisplayBounds(1))
		dialog.ShowInformation("Computer Information",
			"Total Screen Size: w "+strconv.Itoa(w)+" h "+strconv.Itoa(h)+"\n"+
				"Monitor 1 Size: "+"\n"+
				"Monitor 2 Size: ",
			u.win)
	})

	return fyne.NewMainMenu(fyne.NewMenu("Settings", computerInfo), actionMenu)
}
