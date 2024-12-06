package main

import (
        "Dark-And-Darker/internal"
        "Dark-And-Darker/internal/gui/custom_widgets"
        "Dark-And-Darker/internal/utils"
        "fyne.io/fyne/v2/data/binding"
        "fyne.io/fyne/v2/layout"

        "Dark-And-Darker/internal/structs"
        "log"

        "fyne.io/fyne/v2"
        "fyne.io/fyne/v2/container"
        "fyne.io/fyne/v2/theme"
        "fyne.io/fyne/v2/widget"
        "github.com/go-vgo/robotgo"
)

type ui struct {
        win fyne.Window

        mt *macroTree
        st *settingsTabs
}

type settingsTabs struct {
        tabs *container.AppTabs

        wait                 *fyne.Container
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

//action settings
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
        log.Println("Screen Size")
        log.Println(robotgo.GetScreenSize())
        log.Println("Monitor 1 size")
        log.Println(robotgo.GetDisplayBounds(0))
        log.Println("Monitor 2 size")
        log.Println(robotgo.GetDisplayBounds(1))
        internal.CreateItemMaps()
        u.mt.createTree()
        u.updateTreeOnselect()
        u.actionSettingsTabs()

        // searchAreaSelector.SetSelected(searchAreaSelector.Options[0])
        settingsLayout := container.NewBorder(nil, u.createUpdateButton(), nil, nil, u.st.tabs)
        boundMacroNameEntry := widget.NewEntryWithData(u.mt.boundMacroName)
        boundGlobalDelayEntry := widget.NewEntryWithData(binding.IntToString(u.mt.boundGlobalDelay))

        macroLayout := container.NewBorder(
                container.NewGridWithColumns(3,
                        u.mt.createMacroToolbar(),

                        container.NewHBox(
                                widget.NewLabel("Global Delay:"),
                                boundGlobalDelayEntry,
                                layout.NewSpacer(),
                                widget.NewLabel("Macro Name:"),
                        ),
                        boundMacroNameEntry,
                ),
                u.mt.macroSelector(),
                widget.NewSeparator(),
                nil,
                u.mt.tree,
        )
        u.mt.macroSelector().OnChanged = func(s string) {
                boundMacroNameEntry.Text = s
        }
        mainLayout := container.NewBorder(nil, nil, settingsLayout, nil, macroLayout)

        u.mt.loadTreeFromJsonFile("Currency Testing.json")
        return mainLayout
}

func (u *ui) bindVariables() {
        u.mt.boundMacroName = binding.BindString(&macroName)
        u.mt.boundGlobalDelay = binding.BindInt(&globalDelay)

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

//WIDGET LOCATIONS ARE HARD CODED IN THE TREE ONSELECTED CALLBACK. CAREFUL WITH CHANGES HERE
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
                        widget.NewLabel("-----------------------------------------------------------------------------------------------------------"),
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

func (m *macroTree) createMacroToolbar() *widget.Toolbar {
        tb := widget.NewToolbar(
                widget.NewToolbarAction(theme.DocumentSaveIcon(), func() {
                        err := m.saveTreeToJsonFile(macroName)
                        log.Printf("createSaveSettings(): %v", err)
                }),
                widget.NewToolbarSpacer(),
                widget.NewToolbarSeparator(),
                widget.NewToolbarAction(theme.MoveDownIcon(), func() {
                        m.moveNodeDown(selectedTreeItem)
                }),
                widget.NewToolbarAction(theme.MoveUpIcon(), func() {
                        m.moveNodeUp(selectedTreeItem)
                }),
                widget.NewToolbarAction(theme.MediaPlayIcon(), func() {
                        m.executeActionTree()
                }),
                widget.NewToolbarSeparator(),
                widget.NewToolbarAction(nil, func() {}),
                widget.NewToolbarAction(nil, func() {}),

        )
        return tb
}

func (u *ui) createActionMenu() *fyne.Menu {

        basicActionsSubMenu := fyne.NewMenuItem("Basic Actions", nil)
        basicActionsSubMenu.ChildMenu = fyne.NewMenu("")
        advancedActionsSubMenu := fyne.NewMenuItem("Advanced Actions", nil)
        advancedActionsSubMenu.ChildMenu = fyne.NewMenu("")

        waitActionMenuItem := fyne.NewMenuItem("Wait", func() { u.addActionToTree(&structs.WaitAction{}) })
        mouseMoveActionMenuItem := fyne.NewMenuItem("Mouse Move", func() { u.addActionToTree(&structs.MoveAction{}) })
        clickActionMenuItem := fyne.NewMenuItem("Click", func() { u.addActionToTree(&structs.ClickAction{}) })
        keyActionMenuItem := fyne.NewMenuItem("Key", func() { u.addActionToTree(&structs.KeyAction{}) })

        loopActionMenuItem := fyne.NewMenuItem("Loop", func() { u.addActionToTree(&structs.LoopAction{}) })
        imageSearchActionMenuItem := fyne.NewMenuItem("Image Search", func() { u.addActionToTree(&structs.ImageSearchAction{}) })
        ocrActionMenuItem := fyne.NewMenuItem("OCR", func() { u.addActionToTree(&structs.OcrAction{}) })

        ocrActionMenuItem.Icon, _ = fyne.LoadResourceFromPath("./internal/resources/images/Squire.png")

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

        return fyne.NewMenu("Add Action", basicActionsSubMenu, advancedActionsSubMenu)
}
